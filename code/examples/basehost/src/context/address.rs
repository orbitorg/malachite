use std::fmt;

use malachite_common::{Address, Context};

use crate::context::BaseContext;

/// The simplest representation of an address in the network.
/// Implements [`Address`].
#[derive(Clone, Eq, Ord, PartialEq, PartialOrd, Debug)]
pub struct BaseAddress(pub String);

impl BaseAddress {
    pub fn new(address: String) -> <BaseContext as Context>::Address {
        Self(address)
    }

    // Convenience function to move from a String to a u32
    // TODO: Refactor away when we find a better way to correlate
    //     the states, metrics, and inboxes of peers.
    pub fn as_position(&self) -> usize {
        self.0.parse().unwrap()
    }
}

impl Address for BaseAddress {}

impl fmt::Display for BaseAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "base addr {}", self.0)
    }
}
