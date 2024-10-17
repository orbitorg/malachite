use std::str;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use bytesize::ByteSize;
use eyre::eyre;
use ractor::{async_trait, Actor, ActorProcessingErr, SpawnErr};
use tendermint::Hash;
use tokio::time::Instant;
use tracing::{debug, error, info, trace};

use tendermint_proto::v0_38::abci;

use malachite_abci_p2p_types as p2p;
use malachite_actors::consensus::ConsensusMsg;
use malachite_actors::gossip_consensus::{GossipConsensusMsg, GossipConsensusRef};
use malachite_actors::host::LocallyProposedValue;
use malachite_actors::util::streaming::{StreamContent, StreamId, StreamMessage};
use malachite_common::Round;
use malachite_metrics::Metrics;

use crate::build_proposal::build_proposal_parts;
use crate::build_value::{build_value_from_part, build_value_from_parts};
use crate::client::AbciClient;
use crate::context::AbciContext;
use crate::part_store::PartStore;
use crate::streaming::PartStreamsMap;
use crate::types::{Address, Height, ProposalPart, ValidatorSet};

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
    all_parts
        .iter()
        .flat_map(|p| p.as_transactions())
        .flat_map(|x| x.to_vec())
        .map(|x| Bytes::from(x.to_bytes()))
        .collect()
}

fn process_finalize_block_response(response: abci::ResponseFinalizeBlock) -> Hash {
    // TODO: Here is the processing and storing of events, tx responses etc.
    // The number of returned tx_results is in Comet matched against the number of transactions
    // in the proposal and this throws an error if it does not match.
    Hash::from_bytes(
        tendermint::hash::Algorithm::Sha256,
        response.app_hash.as_ref(),
    )
    .unwrap()
}

#[async_trait]
impl Actor for AbciHost {
    type Arguments = ();
    type State = HostState;
    type Msg = HostMsg;

    async fn pre_start(
        &self,
        _myself: HostRef,
        _args: (),
    ) -> Result<Self::State, ActorProcessingErr> {
        let Ok(kvstore_socket) = std::env::var("KVSTORE_SOCKET") else {
            return Err(eyre!("KVSTORE_SOCKET environment variable not set").into());
        };

        info!("KV Store Socket: {kvstore_socket}");

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
                address: _,
                reply_to,
            } => {
                debug!(%height, %round, "Building new proposal...");

                let _deadline = Instant::now() + timeout_duration;

                let next_validators_hash = Bytes::from(
                    self.params.initial_validator_set.get_keys()[0] // Todo: which validator are you looking from exactly?
                        .as_bytes()
                        .to_vec(),
                );

                // **** PREPARE PROPOSAL
                let proposer_address =
                    Bytes::from(state.proposer.clone().unwrap().as_bytes().to_vec());

                let prepare_proposal = abci::RequestPrepareProposal {
                    max_tx_bytes: 10,
                    txs: Vec::new(),
                    height: height.as_u64() as i64,
                    local_last_commit: None, // TODO THIS NEEDS TO BE ADDED IN THE DATA STRUCTURES OF THE PROPOSAL
                    misbehavior: Vec::new(), // TODO THIS NEEDS TO BE ADDED IN THE DATA STRUCTURES OF THE PROPOSAL
                    time: None,
                    next_validators_hash,
                    proposer_address,
                };

                let abci_request = abci::Request {
                    value: Some(abci::request::Value::PrepareProposal(prepare_proposal)),
                };

                let response = state
                    .abci_client
                    .request_with_flush(abci_request)
                    .await?
                    .value;

                let txes: Vec<_> = match response {
                    Some(abci::response::Value::PrepareProposal(prep)) => prep.txs,

                    Some(abci::response::Value::Exception(e)) => {
                        error!("ABCI app raised an exception: {e:?}");
                        return Ok(());
                    }

                    Some(other) => {
                        error!("Received unexpected response from ABCI app: {other:?}");
                        return Ok(());
                    }

                    None => {
                        error!("No response from ABCI app");
                        return Ok(());
                    }
                };

                // This should be removed, just for debugging purposes, prints the transactions
                info!("Transactions retrieved:");

                for tx in &txes {
                    let tx_string = str::from_utf8(tx.as_ref()).unwrap();
                    info!(" - {}", tx_string);
                }

                let txes = txes.into_iter().map(p2p::Transaction::new).collect();

                // ***** END PREPARE PROPOSAL

                let (block_hash, parts) =
                    build_proposal_parts(height, round, &self.params, txes).await?;

                info!("Block Hash: {block_hash}");

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

            HostMsg::GetValidatorSet {
                height: _,
                reply_to,
            } => {
                reply_to.send(self.params.initial_validator_set.clone())?;
                Ok(())
            }

            HostMsg::Decide {
                height,
                round,
                value: block_hash,
                commits: _,
                consensus,
            } => {
                let all_parts = state.part_store.all_parts(height, round);

                let next_validators_hash = Bytes::from(
                    self.params.initial_validator_set.get_keys()[0] // Todo: which validator are you looking from exactly?
                        .as_bytes()
                        .to_vec(),
                );

                let proposer_address = state
                    .proposer
                    .as_ref()
                    .map(|p| p.to_bytes())
                    .unwrap_or_default();

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
                let process_proposal = abci::RequestProcessProposal {
                    txs: tx_bytes.clone(),
                    height: height.as_u64() as i64,
                    proposed_last_commit: None, // TODO THIS NEEDS TO BE ADDED IN THE DATA STRUCTURES OF THE PROPOSAL
                    misbehavior: Vec::new(), // TODO THIS NEEDS TO BE ADDED IN THE DATA STRUCTURES OF THE PROPOSAL
                    hash: Bytes::new(),
                    time: None,
                    next_validators_hash: next_validators_hash.clone(),
                    proposer_address: proposer_address.clone(),
                };

                let request = abci::Request {
                    value: Some(abci::request::Value::ProcessProposal(process_proposal)),
                };

                let response = state.abci_client.request_with_flush(request).await?.value;
                let status = match response {
                    Some(abci::response::Value::ProcessProposal(proc)) => proc.status,

                    Some(other) => {
                        error!("Received unexpected response from ABCI app: {other:?}");
                        return Ok(());
                    }

                    None => {
                        error!("No response from ABCI app");
                        return Ok(());
                    }
                };

                info!("Proposal has been accepted if status is 1: {status}");

                // *** END PROCESS PROPOSAL

                // *** FINALIZE BLOCK
                let finalize_block = abci::RequestFinalizeBlock {
                    txs: tx_bytes,
                    decided_last_commit: None, // TODO THIS NEEDS TO BE ADDED IN THE DATA STRUCTURES OF THE PROPOSAL
                    misbehavior: Vec::new(), // TODO THIS NEEDS TO BE ADDED IN THE DATA STRUCTURES OF THE PROPOSAL
                    hash: block_hash.to_bytes(),
                    height: height.as_u64() as i64,
                    time: None,
                    next_validators_hash,
                    proposer_address,
                };

                let request = abci::Request {
                    value: Some(abci::request::Value::FinalizeBlock(finalize_block)),
                };

                let response = state.abci_client.request_with_flush(request).await?.value;
                let response = match response {
                    Some(abci::response::Value::FinalizeBlock(resp)) => resp,
                    other => {
                        error!("Got an unexpected response from ABCI app: {other:?}");
                        return Ok(());
                    }
                };

                // Here is where the finalize block events are received. For consensus the important part is the app_hash
                let app_hash = process_finalize_block_response(response);
                info!("App Hash: {app_hash}");

                // **** END FINALIZE BLOCK

                // **** COMMIT
                let request_commit = abci::RequestCommit {};
                let request = abci::Request {
                    value: Some(abci::request::Value::Commit(request_commit)),
                };

                let response = state.abci_client.request_with_flush(request).await?.value;

                let retain_height = match response {
                    Some(abci::response::Value::Commit(response)) => response.retain_height,

                    Some(other) => {
                        error!("Received unexpected response from ABCI app: {other:?}");
                        return Ok(());
                    }

                    None => {
                        error!("No response from ABCI app");
                        return Ok(());
                    }
                };

                info!("Retain height: {retain_height}");

                // TODO: Prune block and state store based on the retain height returned here

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
