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
}

impl Address for BaseAddress {}

impl fmt::Display for BaseAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "base addr {}", self.0)
    }
}
