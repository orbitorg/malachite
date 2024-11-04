use std::fmt;

use malachite_common::{PublicKey, Validator, VotingPower};

use crate::context::address::BaseAddress;
use crate::context::BaseContext;

/// This is the voting power of each peer.
pub const BASE_VOTING_POWER: u64 = 1;

/// The most basic definition of a peer.
/// All peers have equal voting power, [`BASE_VOTING_POWER`].
/// Implements [`Validator`] trait.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasePeer {
    pub id: BaseAddress,
}

impl BasePeer {
    pub fn new(id: String) -> BasePeer {
        BasePeer {
            id: BaseAddress::new(id),
        }
    }
}

impl fmt::Display for BasePeer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "peer {}", self.id)
    }
}

impl Validator<BaseContext> for BasePeer {
    fn address(&self) -> &BaseAddress {
        &self.id
    }

    fn public_key(&self) -> &PublicKey<BaseContext> {
        todo!()
    }

    fn voting_power(&self) -> VotingPower {
        BASE_VOTING_POWER
    }
}
