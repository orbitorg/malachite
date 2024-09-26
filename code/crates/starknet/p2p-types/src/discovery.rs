use multiaddr::{Multiaddr, PeerId};

use malachite_proto::Error as ProtoError;
use malachite_proto::Protobuf;
use malachite_starknet_p2p_proto as proto;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capability {
    pub protocol: String,
    pub capability: Vec<u8>,
}

impl Protobuf for Capability {
    type Proto = proto::Capability;

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn from_proto(proto: Self::Proto) -> Result<Self, ProtoError> {
        Ok(Self {
            protocol: proto.protocol,
            capability: proto.capability,
        })
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn to_proto(&self) -> Result<Self::Proto, ProtoError> {
        Ok(proto::Capability {
            protocol: self.protocol.clone(),
            capability: self.capability.clone(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pow {
    /// One of the accepted block hashes in the underlying layer (ethereum in starknet).
    /// Accepted is currently the current last or one before it.
    pub block_hash: Vec<u8>,
    /// A salt such that keccak(salt||blockHash||id) is below posDifficulty
    pub salt: Vec<u8>,
}

impl Protobuf for Pow {
    type Proto = proto::Pow;

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn from_proto(proto: Self::Proto) -> Result<Self, ProtoError> {
        Ok(Self {
            block_hash: proto.block_hash,
            salt: proto.salt,
        })
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn to_proto(&self) -> Result<Self::Proto, ProtoError> {
        Ok(proto::Pow {
            block_hash: self.block_hash.clone(),
            salt: self.salt.clone(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub id: PeerId,
    pub addresses: Vec<Multiaddr>,
    pub capabilities: Vec<Capability>,
    pub pow: Pow,
}

impl Protobuf for Node {
    type Proto = proto::Node;

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn from_proto(proto: Self::Proto) -> Result<Self, ProtoError> {
        let id = proto
            .id
            .ok_or_else(|| ProtoError::missing_field::<Self::Proto>("id"))?;

        let pow = proto
            .pow
            .ok_or_else(|| ProtoError::missing_field::<Self::Proto>("pow"))?;

        Ok(Self {
            id: PeerId::from_bytes(&id.id).map_err(|e| ProtoError::Other(e.to_string()))?,
            addresses: proto
                .addresses
                .into_iter()
                .map(|address| Multiaddr::try_from(address.value))
                .collect::<Result<_, _>>()
                .map_err(|e| ProtoError::Other(e.to_string()))?,
            capabilities: proto
                .capabilities
                .into_iter()
                .map(Capability::from_proto)
                .collect::<Result<_, _>>()?,
            pow: Pow::from_proto(pow)?,
        })
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn to_proto(&self) -> Result<Self::Proto, ProtoError> {
        Ok(proto::Node {
            id: Some(proto::PeerId {
                id: self.id.to_bytes(),
            }),
            addresses: self
                .addresses
                .iter()
                .map(|address| proto::MultiAddress {
                    value: address.to_vec(),
                })
                .collect(),
            capabilities: self
                .capabilities
                .iter()
                .map(Protobuf::to_proto)
                .collect::<Result<_, _>>()?,
            pow: Some(self.pow.to_proto()?),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodesRequest {
    /// This can be used to request for peer information when only its id is known.
    /// The number of ids is limited (TBD) we might know only of an id when
    /// getting a message through a relayer from a new peer.
    pub ids: Vec<PeerId>,
}

impl Protobuf for NodesRequest {
    type Proto = proto::NodesRequest;

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn from_proto(proto: Self::Proto) -> Result<Self, ProtoError> {
        Ok(Self {
            ids: proto
                .ids
                .into_iter()
                .map(|id| PeerId::from_bytes(&id.id).map_err(|e| ProtoError::Other(e.to_string())))
                .collect::<Result<_, _>>()?,
        })
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn to_proto(&self) -> Result<Self::Proto, ProtoError> {
        Ok(proto::NodesRequest {
            ids: self
                .ids
                .iter()
                .map(|id| proto::PeerId { id: id.to_bytes() })
                .collect(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodesResponse {
    pub nodes: Vec<Node>,
}

impl Protobuf for NodesResponse {
    type Proto = proto::NodesResponse;

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn from_proto(proto: Self::Proto) -> Result<Self, ProtoError> {
        Ok(Self {
            nodes: proto
                .nodes
                .into_iter()
                .map(Node::from_proto)
                .collect::<Result<_, _>>()?,
        })
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    fn to_proto(&self) -> Result<Self::Proto, ProtoError> {
        Ok(proto::NodesResponse {
            nodes: self
                .nodes
                .iter()
                .map(Protobuf::to_proto)
                .collect::<Result<_, _>>()?,
        })
    }
}
