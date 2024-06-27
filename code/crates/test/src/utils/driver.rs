use malachite_common::{NilOrVal, Round, Timeout, Validity};
use malachite_driver::{Input, Output};
use malachite_round::state::{RoundValue, State, Step};

use crate::{Address, Height, Proposal, TestContext, Value, Vote};

pub fn new_round_input(round: Round, proposer: Address) -> Input<TestContext> {
    Input::NewRound(Height::new(1), round, proposer)
}

pub fn new_round_output(round: Round) -> Output<TestContext> {
    Output::NewRound(Height::new(1), round)
}

pub fn proposal_output(
    round: Round,
    value: Value,
    locked_round: Round,
    address: Address,
) -> Output<TestContext> {
    let proposal = Proposal::new(Height::new(1), round, value, locked_round, address);
    Output::Propose(proposal)
}

pub fn proposal_input(
    round: Round,
    value: Value,
    locked_round: Round,
    validity: Validity,
    address: Address,
) -> Input<TestContext> {
    let proposal = Proposal::new(Height::new(1), round, value, locked_round, address);
    Input::Proposal(proposal, validity)
}

pub fn prevote_output(round: Round, addr: &Address) -> Output<TestContext> {
    let value = Value::new(9999);

    Output::Vote(Vote::new_prevote(
        Height::new(1),
        round,
        NilOrVal::Val(value.id()),
        *addr,
    ))
}

pub fn prevote_nil_output(round: Round, addr: &Address) -> Output<TestContext> {
    Output::Vote(Vote::new_prevote(
        Height::new(1),
        round,
        NilOrVal::Nil,
        *addr,
    ))
}

pub fn prevote_input(addr: &Address) -> Input<TestContext> {
    let value = Value::new(9999);

    Input::Vote(Vote::new_prevote(
        Height::new(1),
        Round::new(0),
        NilOrVal::Val(value.id()),
        *addr,
    ))
}

pub fn prevote_nil_input(addr: &Address) -> Input<TestContext> {
    Input::Vote(Vote::new_prevote(
        Height::new(1),
        Round::new(0),
        NilOrVal::Nil,
        *addr,
    ))
}

pub fn prevote_input_at(round: Round, addr: &Address) -> Input<TestContext> {
    let value = Value::new(9999);

    Input::Vote(Vote::new_prevote(
        Height::new(1),
        round,
        NilOrVal::Val(value.id()),
        *addr,
    ))
}

pub fn precommit_output(round: Round, value: Value, addr: &Address) -> Output<TestContext> {
    Output::Vote(Vote::new_precommit(
        Height::new(1),
        round,
        NilOrVal::Val(value.id()),
        *addr,
    ))
}

pub fn precommit_nil_output(round: Round, addr: &Address) -> Output<TestContext> {
    Output::Vote(Vote::new_precommit(
        Height::new(1),
        round,
        NilOrVal::Nil,
        *addr,
    ))
}

pub fn precommit_input(round: Round, value: Value, addr: &Address) -> Input<TestContext> {
    Input::Vote(Vote::new_precommit(
        Height::new(1),
        round,
        NilOrVal::Val(value.id()),
        *addr,
    ))
}

pub fn decide_output(round: Round, value: Value) -> Output<TestContext> {
    Output::Decide(round, value)
}

pub fn start_propose_timer_output(round: Round) -> Output<TestContext> {
    Output::ScheduleTimeout(Timeout::propose(round))
}

pub fn timeout_propose_input(round: Round) -> Input<TestContext> {
    Input::TimeoutElapsed(Timeout::propose(round))
}

pub fn start_prevote_timer_output(round: Round) -> Output<TestContext> {
    Output::ScheduleTimeout(Timeout::prevote(round))
}

pub fn timeout_prevote_input(round: Round) -> Input<TestContext> {
    Input::TimeoutElapsed(Timeout::prevote(round))
}

pub fn start_precommit_timer_output(round: Round) -> Output<TestContext> {
    Output::ScheduleTimeout(Timeout::precommit(round))
}

pub fn timeout_precommit_input(round: Round) -> Input<TestContext> {
    Input::TimeoutElapsed(Timeout::precommit(round))
}

pub fn propose_state(round: Round) -> State<TestContext> {
    State {
        height: Height::new(1),
        round,
        step: Step::Propose,
        locked: None,
        valid: None,
        decision: None,
    }
}

pub fn propose_state_with_proposal_and_valid(
    state_round: Round,
    valid_round: Round,
    proposal: Proposal,
) -> State<TestContext> {
    State {
        height: Height::new(1),
        round: state_round,
        step: Step::Propose,
        valid: Some(RoundValue {
            value: proposal.value,
            round: valid_round,
        }),
        locked: None,
        decision: None,
    }
}

pub fn propose_state_with_proposal_and_locked_and_valid(
    round: Round,
    proposal: Proposal,
) -> State<TestContext> {
    State {
        height: Height::new(1),
        round,
        step: Step::Propose,
        valid: Some(RoundValue {
            value: proposal.value,
            round: Round::new(0),
        }),
        locked: Some(RoundValue {
            value: proposal.value,
            round: Round::new(0),
        }),
        decision: None,
    }
}

pub fn prevote_state(round: Round) -> State<TestContext> {
    State {
        height: Height::new(1),
        round,
        step: Step::Prevote,
        locked: None,
        valid: None,
        decision: None,
    }
}

pub fn prevote_state_with_proposal_and_valid(
    state_round: Round,
    valid_round: Round,
    proposal: Proposal,
) -> State<TestContext> {
    State {
        height: Height::new(1),
        round: state_round,
        step: Step::Prevote,
        valid: Some(RoundValue {
            value: proposal.value,
            round: valid_round,
        }),
        locked: None,
        decision: None,
    }
}

pub fn prevote_state_with_proposal_and_locked_and_valid(
    round: Round,
    proposal: Proposal,
) -> State<TestContext> {
    State {
        height: Height::new(1),
        round,
        step: Step::Prevote,
        valid: Some(RoundValue {
            value: proposal.value,
            round: Round::new(0),
        }),
        locked: Some(RoundValue {
            value: proposal.value,
            round: Round::new(0),
        }),
        decision: None,
    }
}

pub fn precommit_state_with_proposal_and_locked_and_valid(
    round: Round,
    proposal: Proposal,
) -> State<TestContext> {
    State {
        height: Height::new(1),
        round,
        step: Step::Precommit,
        valid: Some(RoundValue {
            value: proposal.value,
            round: Round::new(0),
        }),
        locked: Some(RoundValue {
            value: proposal.value,
            round: Round::new(0),
        }),
        decision: None,
    }
}

pub fn precommit_state(round: Round) -> State<TestContext> {
    State {
        height: Height::new(1),
        round,
        step: Step::Precommit,
        locked: None,
        valid: None,
        decision: None,
    }
}

pub fn precommit_state_with_proposal_and_valid(
    state_round: Round,
    valid_round: Round,
    proposal: Proposal,
) -> State<TestContext> {
    State {
        height: Height::new(1),
        round: state_round,
        step: Step::Precommit,
        valid: Some(RoundValue {
            value: proposal.value,
            round: valid_round,
        }),
        locked: None,
        decision: None,
    }
}

pub fn new_round(round: Round) -> State<TestContext> {
    State {
        height: Height::new(1),
        round,
        step: Step::Unstarted,
        valid: None,
        locked: None,
        decision: None,
    }
}

pub fn new_round_with_proposal_and_valid(round: Round, proposal: Proposal) -> State<TestContext> {
    State {
        height: Height::new(1),
        round,
        step: Step::Unstarted,
        valid: Some(RoundValue {
            value: proposal.value,
            round: Round::new(0),
        }),
        locked: None,
        decision: None,
    }
}

pub fn new_round_with_proposal_and_locked_and_valid(
    round: Round,
    proposal: Proposal,
) -> State<TestContext> {
    State {
        height: Height::new(1),
        round,
        step: Step::Unstarted,
        valid: Some(RoundValue {
            value: proposal.value,
            round: Round::new(0),
        }),
        locked: Some(RoundValue {
            value: proposal.value,
            round: Round::new(0),
        }),
        decision: None,
    }
}

pub fn decided_state(round: Round, value: Value) -> State<TestContext> {
    State {
        height: Height::new(1),
        round,
        step: Step::Commit,
        valid: None,
        locked: None,
        decision: Some(value),
    }
}

pub fn decided_state_with_proposal_and_locked_and_valid(
    round: Round,
    proposal: Proposal,
) -> State<TestContext> {
    State {
        height: Height::new(1),
        round,
        step: Step::Commit,
        valid: Some(RoundValue {
            value: proposal.value,
            round: Round::new(0),
        }),
        locked: Some(RoundValue {
            value: proposal.value,
            round: Round::new(0),
        }),
        decision: Some(proposal.value),
    }
}