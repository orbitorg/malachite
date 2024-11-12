/// Implementation of `PeerSet` and some utility methods.
use std::cmp::PartialEq;

use crate::context::address::BaseAddress;
use crate::context::peer::BasePeer;
use crate::context::BaseContext;
use malachite_common::{ValidatorSet, VotingPower};
use malachite_test::PublicKey;

/// A minimal type capturing a set of peers.
/// Implements [`ValidatorSet`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasePeerSet {
    pub peers: Vec<BasePeer>,
}

impl BasePeerSet {
    // Start a new network with the given number of peers
    // Simplifying assumption: All peers have the same public key
    // Todo: Make this more reflective of real conditions
    pub fn start_new(size: u32, pub_key: PublicKey) -> Self {
        let mut peers = vec![];

        for i in 0..size {
            let peer = BasePeer::new(i.to_string(), pub_key);
            println!("{}: started ", peer);

            peers.push(peer);
        }

        peers.into()
    }
}

impl From<Vec<BasePeer>> for BasePeerSet {
    fn from(value: Vec<BasePeer>) -> Self {
        Self { peers: value }
    }
}

impl ValidatorSet<BaseContext> for BasePeerSet {
    fn count(&self) -> usize {
        self.peers.len()
    }

    // Note: VotingPower is a primitive we can simply re-use
    fn total_voting_power(&self) -> VotingPower {
        // Todo: Double-check if this is fishy
        self.count() as u64
    }

    fn get_by_address(&self, address: &BaseAddress) -> Option<&BasePeer> {
        self.peers.iter().find(|v| &v.id == address)
    }

    fn get_by_index(&self, index: usize) -> Option<&BasePeer> {
        self.peers.get(index)
    }
}
