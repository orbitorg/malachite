use bytes::Bytes;
use core::fmt;
use serde::{Deserialize, Serialize};

use malachite_abci_p2p_proto as p2p_proto;
use malachite_proto::{Error as ProtoError, Protobuf};

use crate::PublicKey;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Address(PublicKey);

impl Address {
    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn new(bytes: [u8; 32]) -> Self {
        Self::from_public_key(PublicKey::from_bytes(bytes))
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn from_public_key(public_key: PublicKey) -> Self {
        Self(public_key)
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn as_bytes(&self) -> [u8; 32] {
        self.0.as_bytes()
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    pub fn to_bytes(&self) -> Bytes {
        Bytes::copy_from_slice(self.0.as_bytes().as_ref())
    }
}

impl fmt::Display for Address {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for Address {
    #[cfg_attr(coverage_nightly, coverage(off))]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address({})", self)
    }
}

impl malachite_common::Address for Address {}

impl Protobuf for Address {
    type Proto = p2p_proto::Address;

    fn from_proto(proto: Self::Proto) -> Result<Self, ProtoError> {
        if proto.elements.len() != 32 {
            return Err(ProtoError::Other(format!(
                "Invalid address length: expected 32, got {}",
                proto.elements.len()
            )));
        }

        let mut bytes = [0; 32];
        bytes.copy_from_slice(&proto.elements);
        Ok(Address::new(bytes))
    }

    fn to_proto(&self) -> Result<Self::Proto, ProtoError> {
        Ok(p2p_proto::Address {
            elements: self.0.as_bytes().to_vec(),
        })
    }
}