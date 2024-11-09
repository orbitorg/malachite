/// A network is a set of peers, comprising an instance of
/// a Malachite-based decentralized system

use std::sync::mpsc::Sender;
use std::thread;
use rand::seq::SliceRandom;
use std::time::Duration;

use crate::context::address::BaseAddress;
use crate::context::height::BaseHeight;
use crate::context::peer_set::BasePeerSet;
use crate::context::BaseContext;
use malachite_common::Context;
use malachite_consensus::Input::StartHeight;
use malachite_consensus::{Effect, Params, State};
use malachite_metrics::{Metrics, SharedRegistry};

#[allow(dead_code)]
pub struct Network<Ctx: Context> {
    // The set of all peers
    // Remains static throughout the lifetime
    peers: BasePeerSet,

    // The state of each peer
    // Todo: Rethink this: the vector-based solution seems awkward
    state: Vec<State<Ctx>>,

    // Params of each peer
    // Todo: Same as for the state vector, revisit this decision
    // Todo: Unclear if we need to store this separately for each
    //  peer, because the `state` variable also has the params
    params: Vec<Params<Ctx>>,
}

impl<Ctx> Network<Ctx>
where
    Ctx: Context,
{
    pub fn new(size: u32) -> Network<BaseContext> {
        let mut state = vec![];
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
            state.push(s);
        }

        Network {
            peers: val_set,
            state,
            params,
        }
    }

    // Orchestrate the execution of this network across all peers
    pub fn run(&mut self, tx: Sender<BaseHeight>) {
        // Todo: Potentially introduce an intermediate abstraction
        //     layer to handle timeouts

        let _metrics = self.bootstrap_network();

        // Busy loop to orchestrate among peers
        loop {
            // Pick a random peer and do 1 step
            self.step_peer();

            // Send the decisions to the caller
            tx.send(BaseHeight::new(1)).unwrap();
            thread::sleep(Duration::from_secs(1));
        }
    }

    // Sends a simple `Start` to each peer
    fn bootstrap_network(&self) -> Result<(), Metrics> {

        let registry = SharedRegistry::global();
        let metrics = Metrics::register(registry);

        // The starting validator set
        let val_set: <Ctx as Context>::ValidatorSet = self
            .params
            .get(0)
            .expect("no params found")
            .initial_validator_set
            .clone();
        let height: <Ctx as Context>::Height = BaseHeight::default();

        let input = StartHeight(height, val_set);

        for peer_state in self.state.iter() {
            let mut pstate = peer_state.clone();

            // Kick off consensus at this peer
            malachite_consensus::process!(
                input: input,
                state: &mut pstate,
                metrics: &metrics,
                with: effect =>
                    self.handle_effect(effect)
            )
        }
    }

    fn step_peer(&mut self) {
        let _peer_state = self
            .state
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

    fn handle_effect(&self, _effect: Effect<Ctx>) -> Result<(), Metrics> {
        Ok(todo!())
    }

    // TODO refactor into this method
    // fn get_val_set(&self) -> {
    //
    // }
}
