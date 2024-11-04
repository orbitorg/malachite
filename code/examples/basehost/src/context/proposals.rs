use malachite_common::Round;

use crate::context::address::BaseAddress;
use crate::context::height::BaseHeight;
use crate::context::value::BaseValue;
use crate::context::BaseContext;

/// A proposal for a value
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseProposal {
    pub height: BaseHeight,
    pub value: BaseValue,
    pub proposer: BaseAddress,
    pub round: Round,
    // Todo: Clarify if needs to be exposed here or at lower levels
    //  of abstraction?
    // pub pol_round: Round,
}

impl malachite_common::Proposal<BaseContext> for BaseProposal {
    fn height(&self) -> BaseHeight {
        self.height
    }

    fn round(&self) -> Round {
        self.round
    }

    // Todo: How come value() returns a Ctx::Value
    //  instead of returning a Ctx::ValueId ?
    fn value(&self) -> &BaseValue {
        &self.value
    }

    // Todo: This seems un-necessary.
    fn take_value(self) -> BaseValue {
        self.value
    }

    // Todo: Seems like exactly the kind of stuff we can
    //  abstract in a "base" layer of primitives ?
    fn pol_round(&self) -> Round {
        unimplemented!()
    }

    fn validator_address(&self) -> &BaseAddress {
        &self.proposer
    }
}

/// A part of a proposal
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseProposalPart {
    pub height: BaseHeight,
    // No need for anything else, for the moment
}

#[allow(unused)]
impl BaseProposalPart {
    pub fn new(h: BaseHeight) -> Self {
        Self { height: h }
    }
}

impl malachite_common::ProposalPart<BaseContext> for BaseProposalPart {
    fn is_first(&self) -> bool {
        true
    }

    fn is_last(&self) -> bool {
        // Todo: Why is this needed?
        //  Maybe just needed in case of streaming proposals?
        unimplemented!()
    }
}
