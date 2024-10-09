#![allow(unused_variables, unused_imports)]

use std::ops::Deref;
use std::ptr::null;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use bytesize::ByteSize;
use eyre::eyre;
use prost::Message;
use ractor::{async_trait, Actor, ActorProcessingErr, SpawnErr};
use sha3::Digest;
use tendermint::v0_38::abci::request::PrepareProposal;
use tendermint::v0_38::abci::response::PrepareProposal as prep_response;
use tendermint::{TendermintKey, Time};

use tendermint_proto::v0_38::abci::{
    self, Request, RequestPrepareProposal, ResponsePrepareProposal,
};
use tokio::time::Instant;
use tokio_util::codec::Encoder;
use tracing::{debug, error, info, trace};

use malachite_abci_p2p_types::Transaction;
use malachite_actors::consensus::ConsensusMsg;
use malachite_actors::gossip_consensus::{GossipConsensusMsg, GossipConsensusRef};
use malachite_actors::host::{LocallyProposedValue, ProposedValue};
use malachite_actors::util::streaming::{StreamContent, StreamId, StreamMessage};
use malachite_common::{Round, Validity};
use malachite_metrics::Metrics;

use crate::build_proposal::build_proposal_parts;
use crate::build_value::{build_value_from_part, build_value_from_parts};
use crate::client::{AbciClient, Encode};
use crate::context::AbciContext;
use crate::part_store::PartStore;
use crate::streaming::PartStreamsMap;
use crate::types::{Address, BlockHash, Height, Proposal, ProposalPart, ValidatorSet};
use malachite_gossip_mempool::BoxError;
use std::time;
pub struct HostParams {
    pub address: Address,
    pub initial_validator_set: ValidatorSet,
    pub max_block_size: ByteSize,
    pub tx_size: ByteSize,
    pub txs_per_part: usize,
    pub time_allowance_factor: f32,
    pub exec_time_per_tx: Duration,
}

pub struct AbciHost {
    params: HostParams,
    gossip_consensus: GossipConsensusRef<AbciContext>,
    metrics: Metrics,
}

pub struct HostState {
    pub height: Height,
    pub round: Round,
    pub proposer: Option<Address>,
    pub part_store: PartStore<AbciContext>,
    pub part_streams_map: PartStreamsMap,
    pub next_stream_id: StreamId,
    pub abci_client: AbciClient,
}

pub type HostRef = malachite_actors::host::HostRef<AbciContext>;
pub type HostMsg = malachite_actors::host::HostMsg<AbciContext>;

impl AbciHost {
    pub fn new(
        params: HostParams,
        gossip_consensus: GossipConsensusRef<AbciContext>,
        metrics: Metrics,
    ) -> Self {
        Self {
            params,
            gossip_consensus,
            metrics,
        }
    }

    pub async fn spawn(
        params: HostParams,
        gossip_consensus: GossipConsensusRef<AbciContext>,
        metrics: Metrics,
    ) -> Result<HostRef, SpawnErr> {
        let (actor_ref, _) =
            Actor::spawn(None, Self::new(params, gossip_consensus, metrics), ()).await?;

        Ok(actor_ref)
    }
}

#[async_trait]
impl Actor for AbciHost {
    type Arguments = ();
    type State = HostState;
    type Msg = HostMsg;

    async fn pre_start(
        &self,
        _myself: HostRef,
        args: (),
    ) -> Result<Self::State, ActorProcessingErr> {
        let state = HostState {
            height: Height::new(0),
            round: Round::Nil,
            proposer: None,
            part_store: PartStore::default(),
            part_streams_map: PartStreamsMap::default(),
            next_stream_id: StreamId::default(),
            abci_client: AbciClient::connect("/tmp/kvstoreplusplus.sock").await?,
        };

        Ok(state)
    }

    #[tracing::instrument("abci.host", skip_all)]
    async fn handle(
        &self,
        _myself: HostRef,
        msg: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            HostMsg::StartRound {
                height,
                round,
                proposer,
            } => {
                state.height = height;
                state.round = round;
                state.proposer = Some(proposer);
                Ok(())
            }

            HostMsg::GetValue {
                height,
                round,
                timeout_duration,
                address,
                reply_to,
            } => {
                let deadline = Instant::now() + timeout_duration;
                info!("XXXX1");
                debug!(%height, %round, "Building new proposal...");

                // let txes: Vec<Bytes> = todo!("Send PrepareProposal to the ABCI app");

                let prep_proposal = RequestPrepareProposal {
                    max_tx_bytes: 10,
                    txs: Vec::new(),
                    height: 1,
                    local_last_commit: None,
                    misbehavior: Vec::new(),
                    time: None,
                    next_validators_hash: Bytes::new(), // self.params.initial_validator_set,
                    proposer_address: Bytes::new(),     //to_bytes(state.proposer),
                };
                let a_req = Request {
                    value: Some(
                        tendermint_proto::v0_38::abci::request::Value::PrepareProposal(
                            prep_proposal,
                        ),
                    ),
                };
                info!("XXXX");
                let txes: tendermint_proto::v0_38::abci::ResponsePrepareProposal =
                    state.abci_client.request(a_req).await?;
                info!(".YYY");
                let x = txes.txs;

                // match x {
                //     Some(tendermint_proto::v0_38::abci::response::Value::PrepareProposal(_)) => {
                //         print!("Found response preparep roposal");
                //     }
                //     None => print!("Not found"),
                //     _ => {
                //         print!("should nt0 happen;");
                //         todo!("should not happen");
                //     }
                // };

                // let resp_alue: tendermint_proto::tendermint::v0_38::abci::ResponsePrepareProposal =
                //     Some(txes.);

                //  = abci::Response {
                //     value: Some(tendermint_proto::abci::response::Value::PrepareProposal(
                //         txes.value,
                //     )),
                // };

                let txes = x.into_iter().map(Transaction::new).collect();

                let (block_hash, parts) =
                    build_proposal_parts(height, round, &self.params, txes).await?;

                let stream_id = state.next_stream_id;
                state.next_stream_id += 1;

                let mut sequence = 0;
                for part in parts {
                    state.part_store.store(height, round, part.clone());

                    debug!(
                        %stream_id,
                        %sequence,
                        part_type = ?part.part_type(),
                        "Broadcasting proposal part"
                    );

                    let msg = StreamMessage::new(stream_id, sequence, StreamContent::Data(part));
                    sequence += 1;

                    self.gossip_consensus
                        .cast(GossipConsensusMsg::BroadcastProposalPart(msg))?;
                }

                let msg = StreamMessage::new(stream_id, sequence, StreamContent::Fin(true));

                self.gossip_consensus
                    .cast(GossipConsensusMsg::BroadcastProposalPart(msg))?;

                let parts = state.part_store.all_parts(height, round);

                if let Some(value) = build_value_from_parts(&parts, height, round) {
                    reply_to.send(LocallyProposedValue::new(
                        value.height,
                        value.round,
                        value.value,
                    ))?;
                }

                Ok(())
            }

            HostMsg::ReceivedProposalPart {
                from,
                part,
                reply_to,
            } => {
                let sequence = part.sequence;

                let Some(parts) = state.part_streams_map.insert(from, part) else {
                    return Ok(());
                };

                if parts.height < state.height {
                    trace!(
                        height = %state.height,
                        round = %state.round,
                        part.height = %parts.height,
                        part.round = %parts.round,
                        part.sequence = %sequence,
                        "Received outdated proposal part, ignoring"
                    );

                    return Ok(());
                }

                for part in parts.parts {
                    debug!(
                        part.sequence = %sequence,
                        part.height = %parts.height,
                        part.round = %parts.round,
                        part.message = ?part.part_type(),
                        "Processing proposal part"
                    );

                    if let Some(value) =
                        build_value_from_part(state, parts.height, parts.round, part).await
                    {
                        reply_to.send(value)?;
                        break;
                    }
                }

                Ok(())
            }

            HostMsg::GetValidatorSet { height, reply_to } => {
                reply_to.send(self.params.initial_validator_set.clone())?;
                Ok(())
            }

            HostMsg::Decide {
                height,
                round,
                value: block_hash,
                commits,
                consensus,
            } => {
                let all_parts = state.part_store.all_parts(height, round);

                // TODO: Build the block from proposal parts and commits and store it

                // Update metrics
                let block_size: usize = all_parts.iter().map(|p| p.size_bytes()).sum();
                let tx_count: usize = all_parts.iter().map(|p| p.tx_count()).sum();

                self.metrics.block_tx_count.observe(tx_count as f64);
                self.metrics.block_size_bytes.observe(block_size as f64);
                self.metrics.finalized_txes.inc_by(tx_count as u64);

                // Prune the PartStore of all parts for heights lower than `state.height`
                state.part_store.prune(state.height);

                // Notify ABCI App of the decision
                todo!("Notify ABCI App of the decision");

                // Start the next height
                consensus.cast(ConsensusMsg::StartHeight(state.height.increment()))?;

                Ok(())
            }
        }
    }
}
