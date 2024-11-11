use rand::seq::SliceRandom;
/// A network is a set of peers, comprising an instance of
/// a Malachite-based decentralized system
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use malachite_consensus::Input::StartHeight;
use malachite_consensus::{Effect, Error, Input, Params, Resume, State};
use malachite_metrics::{Metrics, SharedRegistry};

use crate::context::address::BaseAddress;
use crate::context::height::BaseHeight;
use crate::context::peer_set::BasePeerSet;
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
            },
            states,
        )
    }

    // Orchestrate the execution of this network across all peers
    pub fn run(&mut self, tx: Sender<BaseHeight>, states: &mut Vec<State<BaseContext>>) {
        // Todo: Potentially introduce an intermediate abstraction
        //     layer to handle timeouts

        self.bootstrap_network(states);

        // Busy loop to orchestrate among peers
        loop {
            // Pick a random peer and do 1 step
            // TODO
            // self.step_peer();
            //
            // Send the decisions to the caller
            tx.send(BaseHeight::new(1)).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    }

    // Sends a simple `Start` to each peer
    fn bootstrap_network(&mut self, states: &mut Vec<State<BaseContext>>) {
        let registry = SharedRegistry::global();
        let metrics = Metrics::register(registry);

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
            // Todo: Correlation states <-> params is very fragile
            //     major refactor needed
            let peer_params = self
                .params
                .get(position)
                .expect("could not identify peer at next position")
                .clone();
            println!("using peer {}", peer_params.address.0);

            position += 1;

            // Kick off consensus at this peer
            self.step_peer(input.clone(), &peer_params, &metrics, peer_state)
                .expect("unknown error during step_peer");
        }
    }

    fn step_peer(
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

    #[allow(dead_code)]
    fn step_peer_simple(&mut self) {
        let _peer_state = self
            .params
            .choose(&mut rand::thread_rng())
            .expect("the network has no peers");

        // let input;

        // malachite_consensus::process!(
        //     input: input,
        //     state: &mut state.consensus,
        //     metrics: &self.metrics,
        //     with: effect => {
        //         self.handle_effect(myself, &mut state.timers, &mut state.timeouts, effect).await
        //     }
        // )
    }

    fn handle_effect(
        &self,
        peer_params: &Params<BaseContext>,
        effect: Effect<BaseContext>,
    ) -> Result<Resume<BaseContext>, String> {
        let peer_id = peer_params.address.0.to_owned();

        // Todo: Handle the actual side-effects
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
                panic!("CancelTimeout not impl")
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
            Effect::GetValue(_, _, _) => {
                println!("\t{}** GetValue", peer_id);

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
