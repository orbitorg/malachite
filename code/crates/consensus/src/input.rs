use bytes::Bytes;
use derive_where::derive_where;

use malachite_common::{
    Certificate, Context, Extension, Round, SignedProposal, SignedVote, Timeout,
};

use crate::types::ProposedValue;

/// Inputs to be handled by the consensus process.
#[derive_where(Clone, Debug, PartialEq, Eq)]
pub enum Input<Ctx>
where
    Ctx: Context,
{
    /// Start a new height with the given validator set
    StartHeight(Ctx::Height, Ctx::ValidatorSet),

    /// Process a vote
    Vote(SignedVote<Ctx>),

    /// Process a proposal
    Proposal(SignedProposal<Ctx>),

    /// Propose a value
    ProposeValue(Ctx::Height, Round, Ctx::Value, Option<Extension>),

    /// A timeout has elapsed
    TimeoutElapsed(Timeout),

    /// The value corresponding to a proposal has been received
    /// Note: All peers -- except the proposer -- have to provide this input
    ///     after `Input::Proposal` for consensus state machine to progress.
    ReceivedProposedValue(ProposedValue<Ctx>),

    /// A block received via BlockSync
    ReceivedSyncedBlock(SignedProposal<Ctx>, Certificate<Ctx>, Bytes),
}
