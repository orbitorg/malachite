/// A network is a set of peers, comprising an instance of
/// a Malachite-based decentralized system
use rand::seq::SliceRandom;
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::Duration;
use tracing::{debug, info, span, trace, Level};

use malachite_common::{Height, Round, SignedMessage, Timeout, TimeoutStep, Validity, ValueOrigin};
use malachite_consensus::{ConsensusMsg, Effect, Error, Input, Params, ProposedValue, Resume, SignedConsensusMsg, State, ValuePayload, ValueToPropose};
use malachite_metrics::Metrics;
use ValueOrigin::Consensus;
use crate::common;
use crate::context::address::BaseAddress;
use crate::context::height::BaseHeight;
use crate::context::peer_set::BasePeerSet;
use crate::context::BaseContext;
use crate::context::value::BaseValue;
use crate::decision::Decision;

// The delay between each consecutive step
pub const STEP_DELAY: Duration = Duration::from_millis(200);

pub struct Network {
    // Params of each peer
    // Todo: Same as for the state vector, revisit this decision
    // Todo: Unclear if we need to store this separately for each
    //  peer, because the `state` variable also has the params
    params: Vec<Params<BaseContext>>,

    metrics: Vec<Metrics>,

    inboxes: HashMap<String, VecDeque<Input<BaseContext>>>,

    tx_decision: Sender<Decision>,
}

impl Network {
    pub fn new(size: u32) -> (Network, Vec<State<BaseContext>>, Receiver<Decision>) {
        assert!(size >= 4);
        assert!(size < 10);

        let mut states = vec![];
        let mut params = vec![];

        // Construct the set of peers that comprise the network
        let ctx = BaseContext::new();
        let val_set = BasePeerSet::start_new(size, ctx.public_key());

        // Construct the consensus states and params for each peer
        for i in 0..size {
            let id_addr = i.to_string();
            let p = Params {
                start_height: BaseHeight::default(),
                initial_validator_set: val_set.clone(),
                address: BaseAddress::new(id_addr.clone()),
                // Note: The library provides a type and implementation
                // for threshold params which we're re-using.
                threshold_params: Default::default(),
                // Todo: This can be tricky, must be documented properly
                value_payload: ValuePayload::ProposalOnly,
            };

            // The params at this specific peer
            params.push(p.clone());

            // The state at this specific peer
            let s = State::new(ctx.clone(), p);
            states.push(s);
        }

        // Channels on which send/receive the decisions
        let (tx, rx) = mpsc::channel();

        (
            Network {
                params,
                metrics: vec![], // Initialize during bootstrap
                inboxes: HashMap::new(),
                tx_decision: tx,
            },
            states,
            rx,
        )
    }

    // Orchestrate the execution of this network across all peers
    pub fn run(&mut self, states: &mut Vec<State<BaseContext>>) {
        // Todo: Potentially introduce an intermediate abstraction
        //     layer to handle timeouts

        debug!("bootstrapping network");
        self.bootstrap_network(states);
        info!("bootstrap done");

        // Busy loop to orchestrate among peers
        loop {
            // Pick a random peer and do 1 step
            self.step_arbitrary_peer(states);

            // Simulate network and execution delays
            thread::sleep(STEP_DELAY);
        }
    }

    // Sends a [`Input::Start`] to each peer
    fn bootstrap_network(&mut self, states: &mut Vec<State<BaseContext>>) {
        let input = self.input_start_height(BaseHeight::default());

        for (position, peer_state) in states.iter_mut().enumerate() {
            let peer_params = self
                .params
                .get(position)
                .expect("could not identify peer at next position")
                .clone();

            // Todo: Correlation states <-> params <-> metrics is very fragile
            //     major refactor needed
            let metrics = common::new_metrics();

            // Initialize the inbox
            self.inboxes.insert(position.to_string(), VecDeque::new());

            // Kick off consensus at this peer
            self.process_peer(input.clone(), &peer_params, &metrics, peer_state)
                .expect("unknown error during step_peer");

            // Save the metrics for later use
            self.metrics.push(metrics);
        }
    }

    // Wrapper over `process!` macro to work around the confusion
    // in return types due to the loop { } inside that macro.
    fn process_peer(
        &mut self,
        input: Input<BaseContext>,
        peer_params: &Params<BaseContext>,
        metrics: &Metrics,
        peer_state: &mut State<BaseContext>,
    ) -> Result<(), Error<BaseContext>> {
        malachite_consensus::process!(
            input: input,
            state: peer_state,
            metrics: metrics,
            with: effect =>
                self.handle_effect(peer_params, effect)
        )
    }

    fn step_arbitrary_peer(&mut self, states: &mut Vec<State<BaseContext>>) {
        // Todo: Should not use clone here
        let ps = self
            .params
            .choose(&mut rand::thread_rng())
            .expect("the network has no peers")
            .clone();

        let state = states.get_mut(ps.address.as_position()).unwrap();
        // Todo: Fix the clone
        let metrics = self.metrics.get(ps.address.as_position()).unwrap().clone();

        self.step_peer(ps.address.0.clone(), &ps, &metrics, state);
    }

    fn step_peer(
        &mut self,
        position: String,
        ps: &Params<BaseContext>,
        metrics: &Metrics,
        peer_state: &mut State<BaseContext>,
    ) {
        // Select the inbox of this peer, consume a message, and process it
        // as input for taking the next step
        let ix = self
            .inboxes
            .get_mut(&position)
            .expect("inbox for peer not found");
        if let Some(msg) = ix.pop_front() {
            self.process_peer(msg, ps, metrics, peer_state)
                .expect("unknown error during step_peer");
        } else {
            trace!("empty inbox @ {}", position);
        }
    }

    fn handle_effect(
        &mut self,
        peer_params: &Params<BaseContext>,
        effect: Effect<BaseContext>,
    ) -> Result<Resume<BaseContext>, String> {
        let peer_id = peer_params.address.0.to_owned();

        let span = span!(Level::INFO, "handle_effect", peer_id);
        let _enter = span.enter();

        match effect {
            Effect::ResetTimeouts => {
                trace!("ResetTimeouts");

                Ok(Resume::Continue)
            }
            Effect::CancelAllTimeouts => {
                trace!("CancelAllTimeouts");

                Ok(Resume::Continue)
            }
            Effect::CancelTimeout(_) => {
                trace!("CancelTimeout");

                Ok(Resume::Continue)
            }
            Effect::ScheduleTimeout(t) => {
                trace!("ScheduleTimeout {}", t);

                // Special case to handle: If it's a timeout for commit step
                let Timeout { round: _, step } = t;
                if step == TimeoutStep::Commit {
                    debug!("Triggering TimeoutElapsed for Commit");
                    // We handle this timeout instantly: Signal that the timeout elapsed
                    // This will prompt consensus to provide the effect `Decide`
                    let ix = self.inboxes.get_mut(&peer_id).expect("inbox not found");
                    ix.push_back(Input::TimeoutElapsed(t));
                }

                Ok(Resume::Continue)
            }
            Effect::StartRound(_, _, _) => {
                trace!("StartRound");

                // Nothing in particular to keep track of

                Ok(Resume::Continue)
            }
            Effect::Broadcast(v) => {
                info!("Broadcast {}", pretty_broadcast(&v));

                // Push the signed consensus message into the inbox of all peers
                // This is all that broadcast entails
                for (_, ix) in self.inboxes.iter_mut() {
                    // Todo: Any way to avoid clones below?
                    match v {
                        SignedConsensusMsg::Vote(ref sv) => {
                            //  FIXME: Not needed to broadcast to self
                            ix.push_back(Input::Vote(sv.clone()));
                        }
                        SignedConsensusMsg::Proposal(ref sp) => {
                            ix.push_back(Input::Proposal(sp.clone()));
                            // Normally, this input would be triggered by a separate message, not
                            // by the `Input::Proposal` message.
                            // But we short-circuit here, and instead of using `ProposalPart` we
                            // directly trigger the input.
                            // Todo: Not sure this is right, double check w/ RR & AZ
                            // Todo: This was not intuitive to find (source of bug/confusion)
                            ix.push_back(Input::ProposedValue(ProposedValue {
                                height: sp.height,
                                round: sp.round,
                                valid_round: Round::Nil,
                                validator_address: sp.proposer.clone(),
                                value: sp.value,
                                validity: Validity::Valid,
                                extension: None,
                            }, Consensus));
                        }
                    }
                }

                Ok(Resume::Continue)
            }
            Effect::GetValue(h, r, _) => {
                trace!("GetValue");

                // Control passes to the application here.
                // The app creates a value and provides it as input to Malachite
                // in the form of a `ProposeValue` variant.
                // Register this input in the inbox of the current validator.
                let ix = self.inboxes.get_mut(&peer_id).expect("inbox not found");
                let value = 786 + h.0;
                let input_value = ValueToPropose {
                    height: h,
                    round: r,
                    valid_round: Round::Nil,
                    value: BaseValue(value),
                    extension: None,
                };
                ix.push_back(Input::Propose(
                    input_value
                ));

                Ok(Resume::Continue)
            }
            Effect::GetValidatorSet(h) => {
                info!("GetValidatorSet({}); providing the default", h);

                let val_set = self
                    .params
                    .get(0)
                    .expect("no params found")
                    .initial_validator_set
                    .clone();

                // Todo: Clarify why is this call needed??
                //  The app already provides the validator set in `StartHeight`.
                Ok(Resume::ValidatorSet(h, Some(val_set)))
            }
            Effect::VerifySignature(m, _) => {
                trace!("VerifySignature {}", pretty_verify_signature(m));

                // We should implement this for performance reasons and to reflect realistic
                // conditions.
                // Though in practice, it does not make any difference given the
                // simulated conditions of the local testnet.
                // Todo: signature verification for later

                Ok(Resume::SignatureValidity(true))
            }
            Effect::Decide {
                certificate
            } => {
                // Let the top-level application know about the decision
                self.tx_decision
                    .send(Decision {
                        peer: BaseAddress(peer_id.clone()),
                        value_id: certificate.value_id,
                        height: certificate.height,
                    })
                    .expect("unable to send a decision");

                // Proceed to the next height
                let ix = self.inboxes.get_mut(&peer_id).expect("inbox not found");
                // Todo: Reuse `start_new_height` somehow, needs refactoring
                let val_set = self
                    .params
                    .get(0)
                    .expect("no params found")
                    .initial_validator_set
                    .clone();

                // Register the input in the inbox of this peer
                ix.push_back(Input::StartHeight(certificate.height.increment(), val_set));

                Ok(Resume::Continue)
            }
            Effect::RestreamValue(_, _, _, _, _) => {
                panic!("unimplemented arm Effect::RestreamValue in match effect")
            }
            Effect::PersistMessage(_) => {
                // No support for crash-recovery
                Ok(Resume::Continue)
            }
            Effect::PersistTimeout(_) => {
                // No support for crash-recovery
                Ok(Resume::Continue)
            }
        }
    }

    // Convenience function
    fn input_start_height(&self, height: BaseHeight) -> Input<BaseContext> {
        // The starting validator set
        let val_set = self
            .params
            .get(0)
            .expect("no params found")
            .initial_validator_set
            .clone();

        Input::StartHeight(height, val_set)
    }
}

fn pretty_broadcast(v: &SignedConsensusMsg<BaseContext>) -> String {
    match v {
        SignedConsensusMsg::Vote(ref sv) => sv.to_string(),
        SignedConsensusMsg::Proposal(ref sp) => sp.to_string(),
    }
}

fn pretty_verify_signature(m: SignedMessage<BaseContext, ConsensusMsg<BaseContext>>) -> String {
    match m.message {
        ConsensusMsg::Vote(v) => v.to_string(),
        ConsensusMsg::Proposal(p) => p.to_string(),
    }
}
