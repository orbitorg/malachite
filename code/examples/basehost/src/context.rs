use address::BaseAddress;
use malachite_common::{Context, NilOrVal, PublicKey, Round, Signature, SignedMessage, ValueId};
use malachite_test::Ed25519;

use height::BaseHeight;
use peer::BasePeer;
use peer_set::BasePeerSet;
use proposals::{BaseProposal, BaseProposalPart};
use value::BaseValue;
use vote::BaseVote;

// Type definitions needed for the context
pub mod address;
pub mod height;
pub mod peer;
pub mod peer_set;
pub mod proposals;
pub mod value;
pub mod vote;

#[allow(dead_code)]
#[derive(Clone)]
pub struct BaseContext {
    // Todo: This is never being used.
    //  It's probably necessary in all possible deployments ..
    private_key: String,
}

impl BaseContext {
    pub(crate) fn new() -> BaseContext {
        Self {
            private_key: "".into(),
        }
    }
}

#[allow(unused_variables)]
impl Context for BaseContext {
    type Address = BaseAddress;
    type Height = BaseHeight;
    type ProposalPart = BaseProposalPart;
    type Proposal = BaseProposal;
    type Validator = BasePeer;
    type ValidatorSet = BasePeerSet;
    type Value = BaseValue;
    type Vote = BaseVote;

    // Todo: Can we lower the assumptions on the signing scheme &
    //  signature checks?
    // As a shortcut, we'll use the one from malachite. This might
    // actually be the right way to go for the library.
    type SigningScheme = Ed25519;

    fn select_proposer<'a>(
        &self,
        validator_set: &'a Self::ValidatorSet,
        height: Self::Height,
        round: Round,
    ) -> &'a Self::Validator {
        todo!()
    }

    fn sign_vote(&self, vote: Self::Vote) -> SignedMessage<Self, Self::Vote> {
        todo!()
    }

    fn verify_signed_vote(
        &self,
        vote: &Self::Vote,
        signature: &Signature<Self>,
        public_key: &PublicKey<Self>,
    ) -> bool {
        todo!()
    }

    fn sign_proposal(&self, proposal: Self::Proposal) -> SignedMessage<Self, Self::Proposal> {
        todo!()
    }

    fn verify_signed_proposal(
        &self,
        proposal: &Self::Proposal,
        signature: &Signature<Self>,
        public_key: &PublicKey<Self>,
    ) -> bool {
        todo!()
    }

    fn sign_proposal_part(
        &self,
        proposal_part: Self::ProposalPart,
    ) -> SignedMessage<Self, Self::ProposalPart> {
        todo!()
    }

    fn verify_signed_proposal_part(
        &self,
        proposal_part: &Self::ProposalPart,
        signature: &Signature<Self>,
        public_key: &PublicKey<Self>,
    ) -> bool {
        todo!()
    }

    fn new_proposal(
        height: Self::Height,
        round: Round,
        value: Self::Value,
        pol_round: Round,
        address: Self::Address,
    ) -> Self::Proposal {
        todo!()
    }

    fn new_prevote(
        height: Self::Height,
        round: Round,
        value_id: NilOrVal<ValueId<Self>>,
        address: Self::Address,
    ) -> Self::Vote {
        todo!()
    }

    fn new_precommit(
        height: Self::Height,
        round: Round,
        value_id: NilOrVal<ValueId<Self>>,
        address: Self::Address,
    ) -> Self::Vote {
        todo!()
    }
}
