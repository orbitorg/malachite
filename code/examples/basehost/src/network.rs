use rand::seq::SliceRandom;
use std::collections::HashMap;
/// A network is a set of peers, comprising an instance of
/// a Malachite-based decentralized system
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use malachite_consensus::Input::{ProposeValue, StartHeight};
use malachite_consensus::{Effect, Error, Input, Params, Resume, State};
use malachite_metrics::Metrics;

use crate::common;
use crate::context::address::BaseAddress;
use crate::context::height::BaseHeight;
use crate::context::peer_set::BasePeerSet;
use crate::context::value::BaseValue;
use crate::context::BaseContext;

#[allow(dead_code)]
pub struct Network {
    // The set of all peers
    // Remains static throughout the lifetime
    peers: BasePeerSet,

    // Params of each peer
    // Todo: Same as for the state vector, revisit this decision
    // Todo: Unclear if we need to store this separately for each
    //  peer, because the `state` variable also has the params
    params: Vec<Params<BaseContext>>,

    metrics: Vec<Metrics>,

    inbox: HashMap<String, Vec<Input<BaseContext>>>,
}

impl Network {
    pub fn new(size: u32) -> (Network, Vec<State<BaseContext>>) {
        let mut states = vec![];
        let mut params = vec![];

        // Construct the set of peers that comprise the network
        let val_set = BasePeerSet::start_new(size);
        let ctx = BaseContext::new();

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
            };

            // The params at this specific peer
            params.push(p.clone());

            // The state at this specific peer
            let s = State::new(ctx.clone(), p);
            states.push(s);
        }

        (
            Network {
                peers: val_set,
                params,
                metrics: vec![],       // Initialize during bootstrap
                inbox: HashMap::new(), // Initialize during bootstrap
            },
            states,
        )
    }

    // Orchestrate the execution of this network across all peers
    pub fn run(&mut self, _tx: Sender<BaseHeight>, states: &mut Vec<State<BaseContext>>) {
        // Todo: Potentially introduce an intermediate abstraction
        //     layer to handle timeouts

        self.bootstrap_network(states);

        // Busy loop to orchestrate among peers
        loop {
            // Pick a random peer and do 1 step
            self.step_arbitrary_peer(states);

            // Send the decisions to the caller
            // tx.send(BaseHeight::new(1)).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    }

    // Sends a [`Input::Start`] to each peer
    fn bootstrap_network(&mut self, states: &mut Vec<State<BaseContext>>) {
        // The starting validator set
        let val_set = self
            .params
            .get(0)
            .expect("no params found")
            .initial_validator_set
            .clone();
        let height = BaseHeight::default();

        let input: Input<BaseContext> = StartHeight(height, val_set);

        let mut position = 0;
        for peer_state in states.iter_mut() {
            let peer_params = self
                .params
                .get(position)
                .expect("could not identify peer at next position")
                .clone();

            // Todo: Correlation states <-> params <-> metrics is very fragile
            //     major refactor needed
            let metrics = common::new_metrics();

            // Initialize the inbox
            self.inbox.insert(position.to_string(), vec![]);

            // Kick off consensus at this peer
            self.process_peer(input.clone(), &peer_params, &metrics, peer_state)
                .expect("unknown error during step_peer");

            // Save the metrics for later use
            self.metrics.push(metrics);

            // Prep to advance to the next peer
            position += 1;
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
        println!("selected peer {}", ps.address.0);

        let state = states.get_mut(ps.address.as_position()).unwrap();
        // Todo: Fix the clone
        let metrics = self.metrics.get(ps.address.as_position()).unwrap().clone();

        self.step_with_inbox_for_peer(ps.address.0.clone(), &ps, &metrics, state);
    }

    fn step_with_inbox_for_peer(
        &mut self,
        position: String,
        ps: &Params<BaseContext>,
        metrics: &Metrics,
        peer_state: &mut State<BaseContext>,
    ) {
        // Select the inbox of this peer, consume a message, and process it
        // as input for taking the next step
        let ix = self
            .inbox
            .get_mut(&position)
            .expect("inbox for peer not found");
        if !ix.is_empty() {
            let msg = ix.pop().expect("message not found in the inbox");

            self.process_peer(msg, ps, metrics, peer_state)
                .expect("unknown error during step_peer");
        } else {
            println!("the inbox for {} was empty", position);
        }
    }

    fn handle_effect(
        &mut self,
        peer_params: &Params<BaseContext>,
        effect: Effect<BaseContext>,
    ) -> Result<Resume<BaseContext>, String> {
        let peer_id = peer_params.address.0.to_owned();

        // Todo: Handle the actual side-effects, most of them are boilerplate
        // Todo: Use proper logging w/ scoped vars
        match effect {
            Effect::ResetTimeouts => {
                println!("\t{}** ResetTimeouts", peer_id);

                Ok(Resume::Continue)
            }
            Effect::CancelAllTimeouts => {
                println!("\t{}** CancelAllTimeouts", peer_id);

                Ok(Resume::Continue)
            }
            Effect::CancelTimeout(_) => {
                println!("\t{}** CancelTimeout", peer_id);

                Ok(Resume::Continue)
            }
            Effect::ScheduleTimeout(_) => {
                println!("\t{}** ScheduleTimeout", peer_id);

                Ok(Resume::Continue)
            }
            Effect::StartRound(_, _, _) => {
                println!("\t{}** StartRound", peer_id);

                Ok(Resume::Continue)
            }
            Effect::Broadcast(_) => {
                panic!("Broadcast not impl")
            }
            Effect::GetValue(h, r, _) => {
                println!("\t{}** GetValue", peer_id);

                // Control passes to the application here.
                // The app creates a value and provides it as input to Malachite
                // in the form of a `ProposeValue` variant.
                // Register this input in the inbox of the current validator.
                let ix = self.inbox.get_mut(&peer_id).expect("inbox not found");
                ix.push(ProposeValue(h, r, BaseValue(786), None));

                Ok(Resume::Continue)
            }
            Effect::GetValidatorSet(_) => {
                panic!("GetValidatorSet not impl")
            }
            Effect::VerifySignature(_, _) => {
                panic!("VerifySignature not impl")
            }
            Effect::Decide { .. } => {
                panic!("Decide not impl")
            }
            Effect::SyncedBlock { .. } => {
                panic!("SyncedBlock not impl")
            }
        }
    }
}
