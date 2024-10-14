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
use tendermint::{proposal, tx, TendermintKey, Time};

use tendermint_proto::abci::{RequestFinalizeBlock, RequestProcessProposal, ResponseFinalizeBlock};
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
use std::str;
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

fn get_tx_bytes(all_parts: Vec<Arc<ProposalPart>>) -> Vec<Bytes> {
    let transactions: Vec<_> = all_parts
        .iter()
        .map(|p| p.as_transactions().clone())
        .into_iter()
        .collect();

    let all_transactions: Vec<malachite_abci_p2p_types::Transaction> = transactions
        .iter()
        .flatten()
        .cloned()
        .map(|x| x.clone().into_vec())
        .flatten()
        .collect();

    let tx_bytes: Vec<Bytes> = all_transactions
        .into_iter()
        .map(|x: malachite_abci_p2p_types::Transaction| Bytes::from(x.as_bytes().to_vec()))
        .collect();
    return tx_bytes;
}

fn process_finalize_block_response(response: ResponseFinalizeBlock) -> Bytes {
    // TODO Here is the processing and storing of events, tx responses etc.
    // The number of returned tx_results is in Comet matched against the number of transactions
    // in the proposal and this throws an error if it does not match.
    return response.app_hash;
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
        let kvstore_socket = std::env::var("KVSTORE_SOCKET").unwrap();
        print!("{}", kvstore_socket);

        // INIT CHAIN

        // INFO  to fill out the height and info the app sends to the consensus engine to sync up on
        // current state
        let state = HostState {
            height: Height::new(0),
            round: Round::Nil,
            proposer: None,
            part_store: PartStore::default(),
            part_streams_map: PartStreamsMap::default(),
            next_stream_id: StreamId::default(),
            abci_client: AbciClient::connect(kvstore_socket).await?,
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
                debug!(%height, %round, "Building new proposal...");

                let next_validators_hash = Bytes::from(
                    self.params.initial_validator_set.get_keys()[0] // Todo: which validator are you looking from exactly?
                        .as_bytes()
                        .to_vec(),
                );

                // **** PREPARE PROPOSAL
                let proposer_address =
                    Bytes::from(state.proposer.clone().unwrap().as_bytes().to_vec());
                let prep_proposal = RequestPrepareProposal {
                    max_tx_bytes: 10,
                    txs: Vec::new(),
                    height: height.as_u64() as i64,
                    local_last_commit: None, // TODO THIS NEEDS TO BE ADDED IN THE DATA STRUCTURES OF THE PROPOSAL
                    misbehavior: Vec::new(), // TODO THIS NEEDS TO BE ADDED IN THE DATA STRUCTURES OF THE PROPOSAL
                    time: None,
                    next_validators_hash,
                    proposer_address,
                };
                let a_req = Request {
                    value: Some(
                        tendermint_proto::v0_38::abci::request::Value::PrepareProposal(
                            prep_proposal,
                        ),
                    ),
                };
                let txs_value = state.abci_client.request_with_flush(a_req).await?.value;
                let x = txs_value.unwrap();

                let tx_array: Vec<bytes::Bytes> = match x {
                    tendermint_proto::v0_38::abci::response::Value::PrepareProposal(prep) => {
                        prep.txs
                    }

                    tendermint_proto::v0_38::abci::response::Value::Exception(exp) => {
                        todo!("Exception")
                    }
                    _ => {
                        todo!("should not happen;");
                    }
                };

                // This should be removed, just for debugging purposes, prints the transactions
                print!("\nTransactions retrieved");
                for tx in &tx_array {
                    let tx_vec = tx.to_vec();
                    let tx_string = str::from_utf8(&tx_vec).unwrap();
                    print!("\n{}", tx_string);
                }

                let txes = tx_array.into_iter().map(Transaction::new).collect();

                // ***** END PREPARE PROPOSAL
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

                let next_validators_hash = Bytes::from(
                    self.params.initial_validator_set.get_keys()[0] // Todo: which validator are you looking from exactly?
                        .as_bytes()
                        .to_vec(),
                );
                let proposer_address =
                    Bytes::from(state.proposer.clone().unwrap().as_bytes().to_vec());
                // TODO: Build the block from proposal parts and commits and store it

                // Update metrics
                let block_size: usize = all_parts.iter().map(|p| p.size_bytes()).sum();
                let tx_count: usize = all_parts
                    .iter()
                    .map(|p: &Arc<ProposalPart>| p.tx_count())
                    .sum();

                self.metrics.block_tx_count.observe(tx_count as f64);
                self.metrics.block_size_bytes.observe(block_size as f64);
                self.metrics.finalized_txes.inc_by(tx_count as u64);

                let tx_bytes = get_tx_bytes(all_parts);

                // ***** PROCESS PROPOSAL
                // Notify ABCI App of the decision
                let process_proposal = RequestProcessProposal {
                    txs: tx_bytes.clone(),
                    height: height.as_u64() as i64,
                    proposed_last_commit: None, // TODO THIS NEEDS TO BE ADDED IN THE DATA STRUCTURES OF THE PROPOSAL
                    misbehavior: Vec::new(), // TODO THIS NEEDS TO BE ADDED IN THE DATA STRUCTURES OF THE PROPOSAL
                    hash: Bytes::new(),
                    time: None,
                    next_validators_hash: next_validators_hash.clone(),
                    proposer_address: proposer_address.clone(),
                };
                let a_req = Request {
                    value: Some(
                        tendermint_proto::v0_38::abci::request::Value::ProcessProposal(
                            process_proposal,
                        ),
                    ),
                };
                let response_process_proposal = state
                    .abci_client
                    .request_with_flush(a_req)
                    .await?
                    .value
                    .unwrap(); // Causing panic if empty?

                let status = match response_process_proposal {
                    tendermint_proto::v0_38::abci::response::Value::ProcessProposal(proc) => {
                        proc.status
                    }

                    _ => {
                        panic!("{:?}", response_process_proposal);
                    }
                };

                info!("Proposal has been accepted if status is 1: {status}");

                // *** END PROCESS PROPOSAL

                // *** FINALIZE BLOCK
                let finalize_block_req = RequestFinalizeBlock {
                    txs: tx_bytes,
                    decided_last_commit: None, // TODO THIS NEEDS TO BE ADDED IN THE DATA STRUCTURES OF THE PROPOSAL
                    misbehavior: Vec::new(), // TODO THIS NEEDS TO BE ADDED IN THE DATA STRUCTURES OF THE PROPOSAL
                    hash: Bytes::from(block_hash.as_bytes().to_vec()),
                    height: height.as_u64() as i64,
                    time: None,
                    next_validators_hash,
                    proposer_address,
                };

                let a_req = Request {
                    value: Some(
                        tendermint_proto::v0_38::abci::request::Value::FinalizeBlock(
                            finalize_block_req,
                        ),
                    ),
                };

                let response_finalize_block = state
                    .abci_client
                    .request_with_flush(a_req)
                    .await?
                    .value
                    .unwrap(); // Causing panic if empty?

                let resp = match response_finalize_block {
                    tendermint_proto::v0_38::abci::response::Value::FinalizeBlock(resp) => resp,
                    _ => {
                        panic!("{:?}", response_finalize_block);
                    }
                };

                // Here is where the finalize block events are received. For consensus the important part is the app_hash
                let app_hash = process_finalize_block_response(resp);

                // **** END FINALIZE BLOCK

                // **** COMMIT
                let commit_request = abci::RequestCommit {};
                let a_req = Request {
                    value: Some(tendermint_proto::v0_38::abci::request::Value::Commit(
                        commit_request,
                    )),
                };

                let response_commit = state
                    .abci_client
                    .request_with_flush(a_req)
                    .await?
                    .value
                    .unwrap();

                let retain_height = match response_commit {
                    tendermint_proto::v0_38::abci::response::Value::Commit(retain_height) => {
                        retain_height
                    }
                    _ => {
                        panic!("{:?}", response_commit);
                    }
                };

                // TODO Prune block and state store based on the retain height returned here

                // **** END COMMIT

                // Prune the PartStore of all parts for heights lower than `state.height`
                state.part_store.prune(state.height); // This is cleaning only internal actor state

                // Start the next height
                consensus.cast(ConsensusMsg::StartHeight(state.height.increment()))?;

                Ok(())
            }
        }
    }
}
