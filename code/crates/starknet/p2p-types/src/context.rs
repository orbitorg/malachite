use std::sync::Arc;

use malachite_core_types::{Context, NilOrVal, Round, ValidatorSet as _};

use crate::signing::EcdsaProvider;
use crate::{
    Address, BlockHash, Ecdsa, Height, PrivateKey, Proposal, ProposalPart, Validator, ValidatorSet,
    Vote,
};

mod impls;

#[derive(Clone, Debug)]
pub struct MockContext {
    ecdsa_provider: Arc<EcdsaProvider>,
}

impl MockContext {
    pub fn new(private_key: PrivateKey) -> Self {
        Self {
            ecdsa_provider: Arc::new(EcdsaProvider::new(private_key)),
        }
    }
}

impl Context for MockContext {
    type Address = Address;
    type ProposalPart = ProposalPart;
    type Height = Height;
    type Proposal = Proposal;
    type ValidatorSet = ValidatorSet;
    type Validator = Validator;
    type Value = BlockHash;
    type Vote = Vote;
    type SigningScheme = Ecdsa;
    type SigningProvider = EcdsaProvider;

    fn signing_provider(&self) -> &Self::SigningProvider {
        &self.ecdsa_provider
    }

    fn select_proposer<'a>(
        &self,
        validator_set: &'a Self::ValidatorSet,
        height: Self::Height,
        round: Round,
    ) -> &'a Self::Validator {
        assert!(validator_set.count() > 0);
        assert!(round != Round::Nil && round.as_i64() >= 0);

        let proposer_index = {
            let height = height.as_u64() as usize;
            let round = round.as_i64() as usize;

            (height - 1 + round) % validator_set.count()
        };

        validator_set
            .get_by_index(proposer_index)
            .expect("proposer_index is valid")
    }

    fn new_proposal(
        height: Height,
        round: Round,
        block_hash: BlockHash,
        pol_round: Round,
        address: Address,
    ) -> Proposal {
        Proposal::new(height, round, block_hash, pol_round, address)
    }

    fn new_prevote(
        height: Height,
        round: Round,
        value_id: NilOrVal<BlockHash>,
        address: Address,
    ) -> Vote {
        Vote::new_prevote(height, round, value_id, address)
    }

    fn new_precommit(
        height: Height,
        round: Round,
        value_id: NilOrVal<BlockHash>,
        address: Address,
    ) -> Vote {
        Vote::new_precommit(height, round, value_id, address)
    }
}
