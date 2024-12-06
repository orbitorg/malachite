use rand::prelude::StdRng;
use rand::SeedableRng;

use malachite_common::{
    Context, NilOrVal, PublicKey, Round, Signature, SignedMessage, SignedProposal, ValueId,
    VoteType,
};
use malachite_test::{Ed25519, PrivateKey};

use address::BaseAddress;
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

#[derive(Clone)]
pub struct BaseContext {
    private_key: PrivateKey,
}

impl BaseContext {
    pub fn new() -> BaseContext {
        // The context is shared across all peers
        // This is unusual, because each peer would normally have its own
        // private/public key pair, but our application is unusual in this way
        let mut rng = StdRng::seed_from_u64(0x42);
        let sk = PrivateKey::generate(&mut rng);

        Self { private_key: sk }
    }

    pub fn public_key(&self) -> PublicKey<BaseContext> {
        self.private_key.public_key()
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

    // Todo: Can we reduce the # of assumptions on the signing scheme &
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
        // Keep it simple, the proposer is always the same peer
        validator_set
            .peers
            .get(0)
            .expect("no peer found in the validator set")
    }

    fn sign_vote(&self, vote: Self::Vote) -> SignedMessage<Self, Self::Vote> {
        use signature::Signer;
        let signature = self.private_key.sign(&vote.to_bytes());
        SignedMessage::new(vote, signature)
    }

    // Todo: It is a problem that the application needs to provide this.
    //      It seems like I was able to get away without implementing it.
    //      The same goes for the proposal.
    fn verify_signed_vote(
        &self,
        vote: &Self::Vote,
        signature: &Signature<Self>,
        public_key: &PublicKey<Self>,
    ) -> bool {
        todo!()
    }

    fn sign_proposal(&self, proposal: BaseProposal) -> SignedMessage<Self, BaseProposal> {
        use signature::Signer;
        let signature = self.private_key.sign(&proposal.to_bytes());
        SignedProposal::new(proposal, signature)
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
        BaseProposal {
            height,
            value,
            proposer: address,
            round,
        }
    }

    fn new_prevote(
        height: Self::Height,
        round: Round,
        value_id: NilOrVal<ValueId<Self>>,
        address: Self::Address,
    ) -> Self::Vote {
        BaseVote {
            vote_type: VoteType::Prevote,
            height,
            value_id,
            round,
            voter: address,
            // TODO: A bit strange there is option to put extension into Prevotes
            //  clarify.
            extension: None,
        }
    }

    fn new_precommit(
        height: Self::Height,
        round: Round,
        value_id: NilOrVal<ValueId<Self>>,
        address: Self::Address,
    ) -> Self::Vote {
        BaseVote {
            vote_type: VoteType::Precommit,
            height,
            value_id,
            round,
            voter: address,
            extension: None,
        }
    }
}
