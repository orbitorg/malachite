/// A network is a set of peers, comprising an instance of
/// a Malachite-based decentralized system
use std::sync::mpsc::Sender;

use malachite_common::Context;
use malachite_consensus::{Params, State};

use crate::context::address::BaseAddress;
use crate::context::height::BaseHeight;
use crate::context::peer_set::BasePeerSet;
use crate::context::BaseContext;

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

    pub fn start(&self, tx: Sender<BaseHeight>) {
        // Orchestrate the starting of consensus at every peer in the network

        // thread::spawn(move || {
        //      Todo .. the actual work
        //     });

        // malachite_consensus::process!(
        //     input: input,
        //     state: &mut state.consensus,
        //     metrics: &self.metrics,
        //     with: effect => {
        //         self.handle_effect(myself, &mut state.timers, &mut state.timeouts, effect).await
        //     }
        // )
        tx.send(BaseHeight::new(1)).unwrap()
    }
}
