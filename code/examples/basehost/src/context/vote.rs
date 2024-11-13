use malachite_common::{Extension, NilOrVal, Round, VoteType};
use std::fmt;

use crate::context::address::BaseAddress;
use crate::context::height::BaseHeight;
use crate::context::value::BaseValueId;
use crate::context::BaseContext;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BaseVote {
    pub vote_type: VoteType,
    pub height: BaseHeight,
    pub value_id: NilOrVal<BaseValueId>,
    pub round: Round,
    pub voter: BaseAddress,
}

impl BaseVote {
    // TODO: Similar to how we do it for `BaseProposal`, serialize only
    //  the height here as a quick prototype
    pub fn to_bytes(&self) -> [u8; size_of::<u64>()] {
        self.height.0.to_be_bytes()
    }
}

impl fmt::Display for BaseVote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} / {} / {} / {:?} / <- {}",
            self.vote_type, self.height, self.round, self.value_id, self.voter
        )
    }
}

impl malachite_common::Vote<BaseContext> for BaseVote {
    fn height(&self) -> BaseHeight {
        self.height
    }

    fn round(&self) -> Round {
        self.round
    }

    fn value(&self) -> &NilOrVal<BaseValueId> {
        &self.value_id
    }

    // Todo: Why is this needed?
    //  Candidate for deletion?
    fn take_value(self) -> NilOrVal<BaseValueId> {
        self.value_id
    }

    fn vote_type(&self) -> VoteType {
        self.vote_type
    }

    fn validator_address(&self) -> &BaseAddress {
        &self.voter
    }

    fn extension(&self) -> Option<&Extension> {
        None
    }

    fn extend(self, _extension: Extension) -> Self {
        unimplemented!()
    }
}
