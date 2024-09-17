use derive_where::derive_where;

use malachite_common::{
    Context, NilOrVal, Proposal, Round, SignedProposal, SignedVote, Validity, Value, Vote,
};

pub use libp2p_identity::PeerId;
pub use multiaddr::Multiaddr;

/// A message that can be broadcast by the gossip layer
#[derive_where(Clone, Debug, PartialEq, Eq)]
pub enum GossipMsg<Ctx: Context> {
    Vote(SignedVote<Ctx>),
    Proposal(SignedProposal<Ctx>),
}

impl<Ctx: Context> GossipMsg<Ctx> {
    pub fn msg_height(&self) -> Ctx::Height {
        match self {
            GossipMsg::Vote(msg) => msg.height(),
            GossipMsg::Proposal(msg) => msg.height(),
        }
    }
    pub fn msg_round(&self) -> Round {
        match self {
            GossipMsg::Vote(msg) => msg.round(),
            GossipMsg::Proposal(msg) => msg.round(),
        }
    }
    pub fn msg_value_id(&self) -> NilOrVal<<Ctx::Value as Value>::Id> {
        match self {
            GossipMsg::Vote(msg) => msg.value().clone(),
            GossipMsg::Proposal(msg) => NilOrVal::Val(msg.take_value().id()),
        }
    }
}

/// A message that can be sent by the consensus layer
#[derive_where(Clone, Debug, PartialEq, Eq)]
pub enum ConsensusMsg<Ctx: Context> {
    Vote(Ctx::Vote),
    Proposal(Ctx::Proposal),
}

/// A value proposed by a validator
#[derive_where(Clone, Debug, PartialEq, Eq)]
pub struct ProposedValue<Ctx: Context> {
    pub height: Ctx::Height,
    pub round: Round,
    pub validator_address: Ctx::Address,
    pub value: Ctx::Value,
    pub validity: Validity,
}
