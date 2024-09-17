use std::collections::{BTreeSet, VecDeque};
use std::time::Duration;

use async_trait::async_trait;
use eyre::eyre;
use libp2p::PeerId;
use ractor::{Actor, ActorCell, ActorProcessingErr, ActorRef};
use tokio::sync::mpsc;
use tokio::time::Instant;
use tracing::{debug, error, info, warn};

use malachite_common::{Context, NilOrVal, Round, Timeout, TimeoutStep, ValidatorSet, VoteType};
use malachite_consensus::{Crdt, Effect, GossipMsg, Resume};

use malachite_driver::Driver;
use malachite_metrics::Metrics;
use malachite_node::config::TimeoutConfig;
use malachite_vote::ThresholdParams;

use crate::gossip_consensus::{GossipConsensusRef, GossipEvent, Msg as GossipConsensusMsg};
use crate::host::{HostMsg, HostRef, LocallyProposedValue, ProposedValue};
use crate::util::forward::forward;
use crate::util::timers::{TimeoutElapsed, TimerScheduler};

pub struct ConsensusParams<Ctx: Context> {
    pub start_height: Ctx::Height,
    pub initial_validator_set: Ctx::ValidatorSet,
    pub address: Ctx::Address,
    pub threshold_params: ThresholdParams,
}

pub type ConsensusRef<Ctx> = ActorRef<Msg<Ctx>>;

pub type TxDecision<Ctx> = mpsc::Sender<(<Ctx as Context>::Height, Round, <Ctx as Context>::Value)>;

pub struct Consensus<Ctx>
where
    Ctx: Context,
{
    ctx: Ctx,
    params: ConsensusParams<Ctx>,
    timeout_config: TimeoutConfig,
    gossip_consensus: GossipConsensusRef<Ctx>,
    host: HostRef<Ctx>,
    metrics: Metrics,
    tx_decision: Option<TxDecision<Ctx>>,
}

pub type ConsensusMsg<Ctx> = Msg<Ctx>;

pub enum Msg<Ctx: Context> {
    /// Received an event from the gossip layer
    GossipEvent(GossipEvent<Ctx>),

    /// A timeout has elapsed
    TimeoutElapsed(TimeoutElapsed<Timeout>),

    /// The proposal builder has built a value and can be used in a new proposal consensus message
    ProposeValue(Ctx::Height, Round, Ctx::Value),

    /// Received and sssembled the full value proposed by a validator
    ReceivedProposedValue(ProposedValue<Ctx>),
}

type InnerMsg<Ctx> = malachite_consensus::Msg<Ctx>;

impl<Ctx: Context> From<TimeoutElapsed<Timeout>> for Msg<Ctx> {
    fn from(msg: TimeoutElapsed<Timeout>) -> Self {
        Msg::TimeoutElapsed(msg)
    }
}

type Timers<Ctx> = TimerScheduler<Timeout, Msg<Ctx>>;

struct Timeouts {
    config: TimeoutConfig,
}

impl Timeouts {
    pub fn new(config: TimeoutConfig) -> Self {
        Self { config }
    }

    fn reset(&mut self, config: TimeoutConfig) {
        self.config = config;
    }

    fn duration_for(&self, step: TimeoutStep) -> Duration {
        match step {
            TimeoutStep::Propose => self.config.timeout_propose,
            TimeoutStep::Prevote => self.config.timeout_prevote,
            TimeoutStep::Precommit => self.config.timeout_precommit,
            TimeoutStep::Commit => self.config.timeout_commit,
        }
    }

    fn increase_timeout(&mut self, step: TimeoutStep) {
        let c = &mut self.config;
        match step {
            TimeoutStep::Propose => c.timeout_propose += c.timeout_propose_delta,
            TimeoutStep::Prevote => c.timeout_prevote += c.timeout_prevote_delta,
            TimeoutStep::Precommit => c.timeout_precommit += c.timeout_precommit_delta,
            TimeoutStep::Commit => (),
        };
    }
}

pub struct State<Ctx: Context> {
    /// Scheduler for timers
    timers: Timers<Ctx>,

    /// Timeouts configuration
    timeouts: Timeouts,

    /// The state of the consensus state machine
    consensus: malachite_consensus::State<Ctx>,

    /// The set of peers we are connected to.
    connected_peers: BTreeSet<PeerId>,
}

impl<Ctx: Context> State<Ctx> {}

impl<Ctx> Consensus<Ctx>
where
    Ctx: Context,
{
    pub fn new(
        ctx: Ctx,
        params: ConsensusParams<Ctx>,
        timeout_config: TimeoutConfig,
        gossip_consensus: GossipConsensusRef<Ctx>,
        host: HostRef<Ctx>,
        metrics: Metrics,
        tx_decision: Option<TxDecision<Ctx>>,
    ) -> Self {
        Self {
            ctx,
            params,
            timeout_config,
            gossip_consensus,
            host,
            metrics,
            tx_decision,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn spawn(
        ctx: Ctx,
        params: ConsensusParams<Ctx>,
        timeout_config: TimeoutConfig,
        gossip_consensus: GossipConsensusRef<Ctx>,
        host: HostRef<Ctx>,
        metrics: Metrics,
        tx_decision: Option<TxDecision<Ctx>>,
        supervisor: Option<ActorCell>,
    ) -> Result<ActorRef<Msg<Ctx>>, ractor::SpawnErr> {
        let node = Self::new(
            ctx,
            params,
            timeout_config,
            gossip_consensus,
            host,
            metrics,
            tx_decision,
        );

        let (actor_ref, _) = if let Some(supervisor) = supervisor {
            Actor::spawn_linked(None, node, (), supervisor).await?
        } else {
            Actor::spawn(None, node, ()).await?
        };

        Ok(actor_ref)
    }

    async fn process_msg(
        &self,
        myself: &ActorRef<Msg<Ctx>>,
        state: &mut State<Ctx>,
        msg: InnerMsg<Ctx>,
    ) -> Result<(), ActorProcessingErr> {
        malachite_consensus::process!(
            msg: msg,
            state: &mut state.consensus,
            metrics: &self.metrics,
            with: effect => {
                self.handle_effect(myself, &mut state.timers, &mut state.timeouts, effect).await
            }
        )
    }

    async fn handle_msg(
        &self,
        myself: ActorRef<Msg<Ctx>>,
        state: &mut State<Ctx>,
        msg: Msg<Ctx>,
    ) -> Result<(), ActorProcessingErr> {
        match msg {
            Msg::ProposeValue(height, round, value) => {
                let result = self
                    .process_msg(&myself, state, InnerMsg::ProposeValue(height, round, value))
                    .await;

                if let Err(e) = result {
                    error!("Error when processing ProposeValue message: {e:?}");
                }

                Ok(())
            }

            Msg::GossipEvent(event) => {
                match event {
                    GossipEvent::Listening(addr) => {
                        info!("Listening on {addr}");
                        Ok(())
                    }

                    GossipEvent::PeerConnected(peer_id) => {
                        if !state.connected_peers.insert(peer_id) {
                            // We already saw that peer, ignoring...
                            return Ok(());
                        }

                        info!("Connected to peer {peer_id}");

                        let connected_peers = state.connected_peers.len();
                        let total_peers = state.consensus.driver.validator_set.count() - 1;

                        debug!("Connected to {connected_peers}/{total_peers} peers");

                        self.metrics.connected_peers.inc();

                        if connected_peers == total_peers {
                            info!("Enough peers ({connected_peers}) connected to start consensus");

                            let height = state.consensus.driver.height();

                            let result = self
                                .process_msg(&myself, state, InnerMsg::StartHeight(height))
                                .await;

                            if let Err(e) = result {
                                error!("Error when starting height {height}: {e:?}");
                            }
                        }

                        Ok(())
                    }

                    GossipEvent::PeerDisconnected(peer_id) => {
                        info!("Disconnected from peer {peer_id}");

                        if state.connected_peers.remove(&peer_id) {
                            self.metrics.connected_peers.dec();

                            // TODO: pause/stop consensus, if necessary
                        }

                        Ok(())
                    }

                    GossipEvent::Vote(from, vote) => {
                        if let Err(e) = self
                            .process_msg(&myself, state, InnerMsg::Vote(vote.clone()))
                            .await
                        {
                            error!(%from, "Error when processing vote: {e:?}");
                        }
                        state.consensus.crdt.store_msg(from, GossipMsg::Vote(vote));
                        Ok(())
                    }

                    GossipEvent::Proposal(from, proposal) => {
                        if let Err(e) = self
                            .process_msg(&myself, state, InnerMsg::Proposal(proposal.clone()))
                            .await
                        {
                            error!(%from, "Error when processing proposal: {e:?}");
                        }
                        state
                            .consensus
                            .crdt
                            .store_msg(from, GossipMsg::Proposal(proposal));
                        Ok(())
                    }

                    GossipEvent::ProposalPart(from, part) => {
                        self.host
                            .call_and_forward(
                                |reply_to| HostMsg::ReceivedProposalPart {
                                    from,
                                    part,
                                    reply_to,
                                },
                                &myself,
                                |value| Msg::ReceivedProposedValue(value),
                                None,
                            )
                            .map_err(|e| {
                                eyre!("Error when forwarding proposal parts to host: {e:?}")
                            })?;

                        Ok(())
                    }
                }
            }

            Msg::TimeoutElapsed(elapsed) => {
                let Some(timeout) = state.timers.intercept_timer_msg(elapsed) else {
                    // Timer was cancelled or already processed, ignore
                    return Ok(());
                };

                state.timeouts.increase_timeout(timeout.step);

                if matches!(timeout.step, TimeoutStep::Prevote | TimeoutStep::Precommit) {
                    warn!(step = ?timeout.step, "Timeout elapsed");

                    if let Some(per_round) = state
                        .consensus
                        .driver
                        .vote_keeper
                        .per_round()
                        .get(&state.consensus.driver.round())
                    {
                        warn!(
                            "Number of validators having voted: {} / {}",
                            per_round.addresses_weights().get_inner().len(),
                            state.consensus.driver.validator_set.count()
                        );
                        warn!(
                            "Total voting power of validators: {}",
                            state.consensus.driver.validator_set.total_voting_power()
                        );
                        warn!(
                            "Voting power required: {}",
                            state.consensus.driver.validator_set.total_voting_power() * 2 / 3
                        );
                        warn!(
                            "Total voting power of validators having voted: {}",
                            per_round.addresses_weights().sum()
                        );
                        warn!(
                            "Total voting power of validators having prevoted nil: {}",
                            per_round
                                .votes()
                                .get_weight(VoteType::Prevote, &NilOrVal::Nil)
                        );
                        warn!(
                            "Total voting power of validators having precommited nil: {}",
                            per_round
                                .votes()
                                .get_weight(VoteType::Precommit, &NilOrVal::Nil)
                        );
                        warn!(
                            "Total weight of prevotes: {}",
                            per_round.votes().weight_sum(VoteType::Prevote)
                        );
                        warn!(
                            "Total weight of precommits: {}",
                            per_round.votes().weight_sum(VoteType::Precommit)
                        );
                    }
                }

                let result = self
                    .process_msg(&myself, state, InnerMsg::TimeoutElapsed(timeout))
                    .await;

                if let Err(e) = result {
                    error!("Error when processing TimeoutElapsed message: {e:?}");
                }

                Ok(())
            }

            Msg::ReceivedProposedValue(block) => {
                let result = self
                    .process_msg(&myself, state, InnerMsg::ReceivedProposedValue(block))
                    .await;

                if let Err(e) = result {
                    error!("Error when processing GossipEvent message: {e:?}");
                }

                Ok(())
            }
        }
    }

    #[tracing::instrument(skip(self, myself))]
    fn get_value(
        &self,
        myself: &ActorRef<Msg<Ctx>>,
        height: Ctx::Height,
        round: Round,
        timeout_duration: Duration,
    ) -> Result<(), ActorProcessingErr> {
        // Call `GetValue` on the Host actor, and forward the reply
        // to the current actor, wrapping it in `Msg::ProposeValue`.
        self.host.call_and_forward(
            |reply| HostMsg::GetValue {
                height,
                round,
                timeout_duration,
                address: self.params.address.clone(),
                reply_to: reply,
            },
            myself,
            |proposed: LocallyProposedValue<Ctx>| {
                Msg::<Ctx>::ProposeValue(proposed.height, proposed.round, proposed.value)
            },
            None,
        )?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn get_validator_set(
        &self,
        height: Ctx::Height,
    ) -> Result<Ctx::ValidatorSet, ActorProcessingErr> {
        let validator_set = ractor::call!(self.host, |reply_to| HostMsg::GetValidatorSet {
            height,
            reply_to
        })
        .map_err(|e| eyre!("Error at height {height} when waiting for validator set: {e:?}"))?;

        Ok(validator_set)
    }

    #[tracing::instrument(skip_all)]
    async fn handle_effect(
        &self,
        myself: &ActorRef<Msg<Ctx>>,
        timers: &mut Timers<Ctx>,
        timeouts: &mut Timeouts,
        effect: Effect<Ctx>,
    ) -> Result<Resume<Ctx>, ActorProcessingErr> {
        match effect {
            Effect::ResetTimeouts => {
                timeouts.reset(self.timeout_config);
                Ok(Resume::Continue)
            }

            Effect::CancelAllTimeouts => {
                timers.cancel_all();
                Ok(Resume::Continue)
            }

            Effect::CancelTimeout(timeout) => {
                timers.cancel(&timeout);
                Ok(Resume::Continue)
            }

            Effect::ScheduleTimeout(timeout) => {
                let duration = timeouts.duration_for(timeout.step);
                timers.start_timer(timeout, duration);

                Ok(Resume::Continue)
            }

            Effect::StartRound(height, round, proposer) => {
                self.host.cast(HostMsg::StartRound {
                    height,
                    round,
                    proposer,
                })?;

                Ok(Resume::Continue)
            }

            Effect::VerifySignature(msg, pk) => {
                use malachite_consensus::ConsensusMsg as Msg;

                let start = Instant::now();

                let valid = match msg.message {
                    Msg::Vote(v) => self.ctx.verify_signed_vote(&v, &msg.signature, &pk),
                    Msg::Proposal(p) => self.ctx.verify_signed_proposal(&p, &msg.signature, &pk),
                };

                self.metrics
                    .signature_verification_time
                    .observe(start.elapsed().as_secs_f64());

                Ok(Resume::SignatureValidity(valid))
            }

            Effect::Broadcast(gossip_msg) => {
                self.gossip_consensus
                    .cast(GossipConsensusMsg::BroadcastMsg(gossip_msg))
                    .map_err(|e| eyre!("Error when broadcasting gossip message: {e:?}"))?;

                Ok(Resume::Continue)
            }

            Effect::GetValue(height, round, timeout) => {
                let timeout_duration = timeouts.duration_for(timeout.step);

                self.get_value(myself, height, round, timeout_duration)
                    .map_err(|e| eyre!("Error when asking for value to be built: {e:?}"))?;

                Ok(Resume::Continue)
            }

            Effect::GetValidatorSet(height) => {
                let validator_set = self.get_validator_set(height).await.map_err(|e| {
                    eyre!("Error when getting validator set at height {height}: {e:?}")
                })?;

                Ok(Resume::ValidatorSet(height, validator_set))
            }

            Effect::DecidedOnValue {
                height,
                round,
                value,
                commits,
            } => {
                if let Some(tx_decision) = &self.tx_decision {
                    let _ = tx_decision.send((height, round, value.clone())).await;
                }

                self.host
                    .cast(HostMsg::DecidedOnValue {
                        height,
                        round,
                        value,
                        commits,
                    })
                    .map_err(|e| eyre!("Error when sending decided value to host: {e:?}"))?;

                Ok(Resume::Continue)
            }
        }
    }
}

#[async_trait]
impl<Ctx> Actor for Consensus<Ctx>
where
    Ctx: Context,
{
    type Msg = Msg<Ctx>;
    type State = State<Ctx>;
    type Arguments = ();

    #[tracing::instrument(name = "consensus", skip_all)]
    async fn pre_start(
        &self,
        myself: ActorRef<Msg<Ctx>>,
        _args: (),
    ) -> Result<State<Ctx>, ActorProcessingErr> {
        let forward = forward(myself.clone(), Some(myself.get_cell()), Msg::GossipEvent).await?;

        self.gossip_consensus
            .cast(GossipConsensusMsg::Subscribe(forward))?;

        let driver = Driver::new(
            self.ctx.clone(),
            self.params.start_height,
            self.params.initial_validator_set.clone(),
            self.params.address.clone(),
            self.params.threshold_params,
        );

        let consensus_state = malachite_consensus::State {
            ctx: self.ctx.clone(),
            driver,
            msg_queue: VecDeque::new(),
            received_blocks: vec![],
            signed_precommits: Default::default(),
            crdt: Crdt {
                peer_state: Default::default(),
            },
        };

        Ok(State {
            timers: Timers::new(myself),
            timeouts: Timeouts::new(self.timeout_config),
            consensus: consensus_state,
            connected_peers: BTreeSet::new(),
        })
    }

    async fn post_start(
        &self,
        _myself: ActorRef<Msg<Ctx>>,
        state: &mut State<Ctx>,
    ) -> Result<(), ActorProcessingErr> {
        state.timers.cancel_all();
        Ok(())
    }

    #[tracing::instrument(
        name = "consensus",
        skip_all,
        fields(
            height = %state.consensus.driver.height(),
            round = %state.consensus.driver.round()
        )
    )]
    async fn handle(
        &self,
        myself: ActorRef<Msg<Ctx>>,
        msg: Msg<Ctx>,
        state: &mut State<Ctx>,
    ) -> Result<(), ActorProcessingErr> {
        self.handle_msg(myself, state, msg).await
    }

    #[tracing::instrument(
        name = "consensus",
        skip_all,
        fields(
            height = %state.consensus.driver.height(),
            round = %state.consensus.driver.round()
        )
    )]
    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        state: &mut State<Ctx>,
    ) -> Result<(), ActorProcessingErr> {
        info!("Stopping...");

        state.timers.cancel_all();

        Ok(())
    }
}
