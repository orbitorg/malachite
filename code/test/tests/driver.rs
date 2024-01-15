use malachite_test::utils::{make_validators, FixedProposer, RotateProposer};

use malachite_common::{NilOrVal, Round, Timeout, TimeoutStep};
use malachite_driver::{Driver, Error, Input, Output, Validity};
use malachite_round::state::{RoundValue, State, Step};
use malachite_test::{Height, Proposal, TestContext, ValidatorSet, Value, Vote};

pub struct TestStep {
    desc: &'static str,
    input: Option<Input<TestContext>>,
    expected_output: Option<Output<TestContext>>,
    expected_round: Round,
    new_state: State<TestContext>,
}

pub fn output_to_input(output: Output<TestContext>) -> Option<Input<TestContext>> {
    match output {
        Output::NewRound(height, round) => Some(Input::NewRound(height, round)),
        // Let's consider our own proposal to always be valid
        Output::Propose(p) => Some(Input::Proposal(p, Validity::Valid)),
        Output::Vote(v) => Some(Input::Vote(v)),
        Output::Decide(_, _) => None,
        Output::ScheduleTimeout(_) => None,
        Output::GetValueAndScheduleTimeout(_, _) => None,
    }
}

#[test]
fn driver_steps_proposer() {
    let value = Value::new(9999);

    let [(v1, sk1), (v2, _sk2), (v3, _sk3)] = make_validators([1, 2, 3]);
    let (my_sk, my_addr) = (sk1, v1.address);

    let height = Height::new(1);
    let ctx = TestContext::new(my_sk.clone());
    let sel = FixedProposer::new(my_addr);
    let vs = ValidatorSet::new(vec![v1, v2.clone(), v3.clone()]);

    let mut driver = Driver::new(ctx, height, sel, vs, my_addr, Default::default());

    let proposal = Proposal::new(Height::new(1), Round::new(0), value, Round::new(-1));

    let steps = vec![
        TestStep {
            desc: "Start round 0, we are proposer, ask for a value to propose",
            input: Some(Input::NewRound(Height::new(1), Round::new(0))),
            expected_output: Some(Output::GetValueAndScheduleTimeout(
                Round::new(0),
                Timeout::new(Round::new(0), TimeoutStep::Propose),
            )),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Propose,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "Feed a value to propose, propose that value",
            input: Some(Input::ProposeValue(Round::new(0), value)),
            expected_output: Some(Output::Propose(proposal.clone())),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Propose,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "Receive our own proposal, prevote for it (v1)",
            input: None,
            expected_output: Some(Output::Vote(Vote::new_prevote(
                Height::new(1),
                Round::new(0),
                NilOrVal::Val(value.id()),
                my_addr,
            ))),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "Receive our own prevote v1",
            input: None,
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "v2 prevotes for our proposal",
            input: Some(Input::Vote(Vote::new_prevote(
                Height::new(1),
                Round::new(0),
                NilOrVal::Val(value.id()),
                v2.address,
            ))),
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "v3 prevotes for our proposal, we get +2/3 prevotes, precommit for it (v1)",
            input: Some(Input::Vote(Vote::new_prevote(
                Height::new(1),
                Round::new(0),
                NilOrVal::Val(value.id()),
                v3.address,
            ))),
            expected_output: Some(Output::Vote(Vote::new_precommit(
                Height::new(1),
                Round::new(0),
                NilOrVal::Val(value.id()),
                my_addr,
            ))),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Precommit,
                locked: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                valid: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                decision: None,
            },
        },
        TestStep {
            desc: "v1 receives its own precommit",
            input: None,
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Precommit,
                locked: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                valid: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                decision: None,
            },
        },
        TestStep {
            desc: "v2 precommits for our proposal",
            input: Some(Input::Vote(Vote::new_precommit(
                Height::new(1),
                Round::new(0),
                NilOrVal::Val(value.id()),
                v2.address,
            ))),
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Precommit,
                locked: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                valid: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                decision: None,
            },
        },
        TestStep {
            desc: "v3 precommits for our proposal, we get +2/3 precommits, decide it (v1)",
            input: Some(Input::Vote(Vote::new_precommit(
                Height::new(1),
                Round::new(0),
                NilOrVal::Val(value.id()),
                v3.address,
            ))),
            expected_output: Some(Output::Decide(Round::new(0), value)),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Commit,
                locked: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                valid: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                decision: Some(value),
            },
        },
    ];

    run_steps(&mut driver, steps);
}

#[test]
fn driver_steps_proposer_timeout_get_value() {
    let [(v1, sk1), (v2, _sk2), (v3, _sk3)] = make_validators([1, 2, 3]);
    let (my_sk, my_addr) = (sk1, v1.address);

    let height = Height::new(1);
    let ctx = TestContext::new(my_sk.clone());
    let sel = FixedProposer::new(my_addr);
    let vs = ValidatorSet::new(vec![v1, v2.clone(), v3.clone()]);

    let mut driver = Driver::new(ctx, height, sel, vs, my_addr, Default::default());

    let steps = vec![
        TestStep {
            desc: "Start round 0, we are proposer, ask for a value to propose",
            input: Some(Input::NewRound(Height::new(1), Round::new(0))),
            expected_output: Some(Output::GetValueAndScheduleTimeout(
                Round::new(0),
                Timeout::new(Round::new(0), TimeoutStep::Propose),
            )),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Propose,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "Receive a propose timeout",
            input: Some(Input::TimeoutElapsed(Timeout::propose(Round::new(0)))),
            expected_output: Some(Output::Vote(Vote::new_prevote(
                Height::new(1),
                Round::new(0),
                NilOrVal::Nil,
                my_addr,
            ))),
            expected_round: Round::new(0),
            new_state: State {
                round: Round::new(0),
                height: Height::new(1),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
    ];

    run_steps(&mut driver, steps);
}

#[test]
fn driver_steps_not_proposer_valid() {
    let value = Value::new(9999);

    let [(v1, _sk1), (v2, sk2), (v3, _sk3)] = make_validators([1, 2, 3]);

    // Proposer is v1, so we are not the proposer
    let (my_sk, my_addr) = (sk2, v2.address);

    let height = Height::new(1);
    let ctx = TestContext::new(my_sk.clone());
    let sel = FixedProposer::new(v1.address);
    let vs = ValidatorSet::new(vec![v1.clone(), v2.clone(), v3.clone()]);

    let mut driver = Driver::new(ctx, height, sel, vs, my_addr, Default::default());

    let proposal = Proposal::new(Height::new(1), Round::new(0), value, Round::new(-1));

    let steps = vec![
        TestStep {
            desc: "Start round 0, we are not the proposer",
            input: Some(Input::NewRound(Height::new(1), Round::new(0))),
            expected_output: Some(Output::ScheduleTimeout(Timeout::propose(Round::new(0)))),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Propose,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "Receive a proposal, prevote for it (v2)",
            input: Some(Input::Proposal(proposal.clone(), Validity::Valid)),
            expected_output: Some(Output::Vote(Vote::new_prevote(
                Height::new(1),
                Round::new(0),
                NilOrVal::Val(value.id()),
                my_addr,
            ))),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "Receive our own prevote (v2)",
            input: None,
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "v1 prevotes for its own proposal",
            input: Some(Input::Vote(Vote::new_prevote(
                Height::new(1),
                Round::new(0),
                NilOrVal::Val(value.id()),
                v1.address,
            ))),
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "v3 prevotes for v1's proposal, it gets +2/3 prevotes, precommit for it (v2)",
            input: Some(Input::Vote(Vote::new_prevote(
                Height::new(1),
                Round::new(0),
                NilOrVal::Val(value.id()),
                v3.address,
            ))),
            expected_output: Some(Output::Vote(Vote::new_precommit(
                Height::new(1),
                Round::new(0),
                NilOrVal::Val(value.id()),
                my_addr,
            ))),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Precommit,
                locked: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                valid: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                decision: None,
            },
        },
        TestStep {
            desc: "we receive our own precommit",
            input: None,
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Precommit,
                locked: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                valid: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                decision: None,
            },
        },
        TestStep {
            desc: "v1 precommits its proposal",
            input: Some(Input::Vote(Vote::new_precommit(
                Height::new(1),
                Round::new(0),
                NilOrVal::Val(value.id()),
                v1.address,
            ))),
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Precommit,
                locked: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                valid: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                decision: None,
            },
        },
        TestStep {
            desc: "v3 precommits for v1's proposal, it gets +2/3 precommits, decide it",
            input: Some(Input::Vote(Vote::new_precommit(
                Height::new(1),
                Round::new(0),
                NilOrVal::Val(value.id()),
                v3.address,
            ))),
            expected_output: Some(Output::Decide(Round::new(0), value)),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Commit,
                locked: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                valid: Some(RoundValue {
                    value,
                    round: Round::new(0),
                }),
                decision: Some(value),
            },
        },
    ];

    run_steps(&mut driver, steps);
}

#[test]
fn driver_steps_not_proposer_invalid() {
    let value = Value::new(9999);

    let [(v1, _sk1), (v2, sk2), (v3, _sk3)] = make_validators([1, 2, 3]);

    // Proposer is v1, so we are not the proposer
    let (my_sk, my_addr) = (sk2, v2.address);

    let height = Height::new(1);
    let ctx = TestContext::new(my_sk.clone());
    let sel = FixedProposer::new(v1.address);
    let vs = ValidatorSet::new(vec![v1.clone(), v2.clone(), v3.clone()]);

    let mut driver = Driver::new(ctx, height, sel, vs, my_addr, Default::default());

    let proposal = Proposal::new(Height::new(1), Round::new(0), value, Round::new(-1));

    let steps = vec![
        TestStep {
            desc: "Start round 0, we are not the proposer",
            input: Some(Input::NewRound(Height::new(1), Round::new(0))),
            expected_output: Some(Output::ScheduleTimeout(Timeout::propose(Round::new(0)))),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Propose,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "Receive an invalid proposal, prevote for nil (v2)",
            input: Some(Input::Proposal(proposal.clone(), Validity::Invalid)),
            expected_output: Some(Output::Vote(
                Vote::new_prevote(Height::new(1),Round::new(0), NilOrVal::Nil, my_addr)
            )),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "Receive our own prevote (v2)",
            input: None,
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "v1 prevotes for its own proposal",
            input: Some(Input::Vote(
                Vote::new_prevote(Height::new(1), Round::new(0), NilOrVal::Val(value.id()), v1.address)
            )),
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "v3 prevotes for v1's proposal, we have polka for any, schedule prevote timeout (v2)",
            input: Some(Input::Vote(
                Vote::new_prevote(Height::new(1), Round::new(0), NilOrVal::Val(value.id()), v3.address)
            )),
            expected_output: Some(Output::ScheduleTimeout(Timeout::prevote(Round::new(0)))),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "prevote timeout elapses, we precommit for nil (v2)",
            input: Some(Input::TimeoutElapsed(Timeout::prevote(Round::new(0)))),
            expected_output: Some(Output::Vote(
                Vote::new_precommit(Height::new(1), Round::new(0), NilOrVal::Nil, my_addr)
            )),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Precommit,
                locked: None,
                valid: None,
                decision: None,
            },
        },
    ];

    run_steps(&mut driver, steps);
}

#[test]
fn driver_steps_not_proposer_other_height() {
    let value = Value::new(9999);

    let [(v1, _sk1), (v2, sk2)] = make_validators([1, 2]);

    // Proposer is v1, so we are not the proposer
    let (my_sk, my_addr) = (sk2, v2.address);

    let height = Height::new(1);
    let ctx = TestContext::new(my_sk.clone());
    let sel = FixedProposer::new(v1.address);
    let vs = ValidatorSet::new(vec![v1.clone(), v2.clone()]);

    let mut driver = Driver::new(ctx, height, sel, vs, my_addr, Default::default());

    // Proposal is for another height
    let proposal = Proposal::new(Height::new(2), Round::new(0), value, Round::new(-1));

    let steps = vec![
        TestStep {
            desc: "Start round 0, we are not the proposer",
            input: Some(Input::NewRound(Height::new(1), Round::new(0))),
            expected_output: Some(Output::ScheduleTimeout(Timeout::propose(Round::new(0)))),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Propose,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "Receive a proposal for another height, ignore it (v2)",
            input: Some(Input::Proposal(proposal.clone(), Validity::Invalid)),
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Propose,
                locked: None,
                valid: None,
                decision: None,
            },
        },
    ];

    run_steps(&mut driver, steps);
}

#[test]
fn driver_steps_not_proposer_other_round() {
    let value = Value::new(9999);

    let [(v1, _sk1), (v2, sk2)] = make_validators([1, 2]);

    // Proposer is v1, so we are not the proposer
    let (my_sk, my_addr) = (sk2, v2.address);

    let height = Height::new(1);
    let ctx = TestContext::new(my_sk.clone());
    let sel = FixedProposer::new(v1.address);
    let vs = ValidatorSet::new(vec![v1.clone(), v2.clone()]);

    let mut driver = Driver::new(ctx, height, sel, vs, my_addr, Default::default());

    // Proposal is for another round
    let proposal = Proposal::new(Height::new(1), Round::new(1), value, Round::new(-1));

    let steps = vec![
        TestStep {
            desc: "Start round 0, we are not the proposer",
            input: Some(Input::NewRound(Height::new(1), Round::new(0))),
            expected_output: Some(Output::ScheduleTimeout(Timeout::propose(Round::new(0)))),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Propose,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "Receive a proposal for another round, ignore it (v2)",
            input: Some(Input::Proposal(proposal.clone(), Validity::Invalid)),
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Propose,
                locked: None,
                valid: None,
                decision: None,
            },
        },
    ];

    run_steps(&mut driver, steps);
}

#[test]
fn driver_steps_not_proposer_timeout_multiple_rounds() {
    let value = Value::new(9999);

    let [(v1, _sk1), (v2, _sk2), (v3, sk3)] = make_validators([1, 3, 1]);

    // Proposer is v1, so we, v3, are not the proposer
    let (my_sk, my_addr) = (sk3, v3.address);

    let height = Height::new(1);
    let ctx = TestContext::new(my_sk.clone());
    let sel = FixedProposer::new(v1.address);
    let vs = ValidatorSet::new(vec![v1.clone(), v2.clone(), v3.clone()]);

    let mut driver = Driver::new(ctx, height, sel, vs, my_addr, Default::default());

    let steps = vec![
        // Start round 0, we, v3, are not the proposer
        TestStep {
            desc: "Start round 0, we, v3, are not the proposer",
            input: Some(Input::NewRound(Height::new(1), Round::new(0))),
            expected_output: Some(Output::ScheduleTimeout(Timeout::propose(Round::new(0)))),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Propose,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // Receive a propose timeout, prevote for nil (from v3)
        TestStep {
            desc: "Receive a propose timeout, prevote for nil (v3)",
            input: Some(Input::TimeoutElapsed(Timeout::propose(Round::new(0)))),
            expected_output: Some(Output::Vote(Vote::new_prevote(
                Height::new(1),
                Round::new(0),
                NilOrVal::Nil,
                my_addr,
            ))),
            expected_round: Round::new(0),
            new_state: State {
                round: Round::new(0),
                height: Height::new(1),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // Receive our own prevote v3
        TestStep {
            desc: "Receive our own prevote v3",
            input: None,
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // v1 prevotes for its own proposal
        TestStep {
            desc: "v1 prevotes for its own proposal",
            input: Some(Input::Vote(Vote::new_prevote(
                Height::new(1),
                Round::new(0),
                NilOrVal::Val(value.id()),
                v1.address,
            ))),
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // v2 prevotes for nil, we get +2/3 nil prevotes and precommit for nil
        TestStep {
            desc: "v2 prevotes for nil, we get +2/3 prevotes, precommit for nil",
            input: Some(Input::Vote(Vote::new_prevote(
                Height::new(1),
                Round::new(0),
                NilOrVal::Nil,
                v2.address,
            ))),
            expected_output: Some(Output::Vote(Vote::new_precommit(
                Height::new(1),
                Round::new(0),
                NilOrVal::Nil,
                my_addr,
            ))),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Precommit,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // v3 receives its own precommit
        TestStep {
            desc: "v3 receives its own precommit",
            input: None,
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Precommit,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // v1 precommits its proposal
        TestStep {
            desc: "v1 precommits its proposal",
            input: Some(Input::Vote(Vote::new_precommit(
                Height::new(1),
                Round::new(0),
                NilOrVal::Val(value.id()),
                v1.address,
            ))),
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Precommit,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // v2 precommits for nil
        TestStep {
            desc: "v2 precommits for nil",
            input: Some(Input::Vote(Vote::new_precommit(
                Height::new(1),
                Round::new(0),
                NilOrVal::Nil,
                v2.address,
            ))),
            expected_output: Some(Output::ScheduleTimeout(Timeout::precommit(Round::new(0)))),
            expected_round: Round::new(0),
            new_state: State {
                height: Height::new(1),
                round: Round::new(0),
                step: Step::Precommit,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // we receive a precommit timeout, start a new round
        TestStep {
            desc: "we receive a precommit timeout, start a new round",
            input: Some(Input::TimeoutElapsed(Timeout::precommit(Round::new(0)))),
            expected_output: Some(Output::NewRound(Height::new(1), Round::new(1))),
            expected_round: Round::new(1),
            new_state: State {
                height: Height::new(1),
                round: Round::new(1),
                step: Step::Unstarted,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        TestStep {
            desc: "Start round 1, we are not the proposer",
            input: Some(Input::NewRound(Height::new(1), Round::new(1))),
            expected_output: Some(Output::ScheduleTimeout(Timeout::propose(Round::new(1)))),
            expected_round: Round::new(1),
            new_state: State {
                height: Height::new(1),
                round: Round::new(1),
                step: Step::Propose,
                locked: None,
                valid: None,
                decision: None,
            },
        },
    ];

    run_steps(&mut driver, steps);
}

// No value to propose
#[test]
fn driver_steps_no_value_to_propose() {
    let [(v1, sk1), (v2, _sk2), (v3, _sk3)] = make_validators([1, 2, 3]);
    let (my_sk, my_addr) = (sk1, v1.address);

    let height = Height::new(1);
    let ctx = TestContext::new(my_sk.clone());

    // We are the proposer
    let sel = FixedProposer::new(v1.address);
    let vs = ValidatorSet::new(vec![v1.clone(), v2.clone(), v3.clone()]);

    let mut driver = Driver::new(ctx, height, sel, vs, my_addr, Default::default());

    let mut outputs = driver
        .process(Input::NewRound(Height::new(1), Round::new(0)))
        .expect("execute succeeded");

    let output = outputs.pop();

    assert_eq!(
        output,
        Some(Output::GetValueAndScheduleTimeout(
            Round::new(0),
            Timeout::propose(Round::new(0))
        ))
    );
}

#[test]
fn driver_steps_proposer_not_found() {
    let [(v1, _sk1), (v2, sk2), (v3, _sk3)] = make_validators([1, 2, 3]);

    let (my_sk, my_addr) = (sk2, v2.address);

    let height = Height::new(1);
    let ctx = TestContext::new(my_sk.clone());

    // Proposer is v1, which is not in the validator set
    let sel = FixedProposer::new(v1.address);
    let vs = ValidatorSet::new(vec![v2.clone(), v3.clone()]);

    let mut driver = Driver::new(ctx, height, sel, vs, my_addr, Default::default());

    let output = driver.process(Input::NewRound(Height::new(1), Round::new(0)));
    assert_eq!(output, Err(Error::ProposerNotFound(v1.address)));
}

#[test]
fn driver_steps_validator_not_found() {
    let value = Value::new(9999);

    let [(v1, _sk1), (v2, _sk2), (v3, sk3)] = make_validators([1, 2, 3]);

    let (my_sk, my_addr) = (sk3.clone(), v3.address);

    let height = Height::new(1);
    let ctx = TestContext::new(my_sk.clone());

    // Proposer is v1
    let sel = FixedProposer::new(v1.address);
    // We omit v2 from the validator set
    let vs = ValidatorSet::new(vec![v1.clone(), v3.clone()]);

    let mut driver = Driver::new(ctx, height, sel, vs, my_addr, Default::default());

    // Start new height
    driver
        .process(Input::NewRound(Height::new(1), Round::new(0)))
        .expect("execute succeeded");

    // v2 prevotes for some proposal, we cannot find it in the validator set => error
    let output = driver.process(Input::Vote(Vote::new_prevote(
        Height::new(1),
        Round::new(0),
        NilOrVal::Val(value.id()),
        v2.address,
    )));

    assert_eq!(output, Err(Error::ValidatorNotFound(v2.address)));
}

#[test]
fn driver_steps_skip_round_skip_threshold() {
    let value = Value::new(9999);

    let sel = RotateProposer;

    let [(v1, _sk1), (v2, _sk2), (v3, sk3)] = make_validators([1, 1, 1]);

    // Proposer is v1, so we, v3, are not the proposer
    let (my_sk, my_addr) = (sk3, v3.address);

    let ctx = TestContext::new(my_sk.clone());
    let height = Height::new(1);

    let vs = ValidatorSet::new(vec![v1.clone(), v2.clone(), v3.clone()]);
    let mut driver = Driver::new(ctx, height, sel, vs, my_addr, Default::default());

    let steps = vec![
        // Start round 0, we, v3, are not the proposer
        TestStep {
            desc: "Start round 0, we, v3, are not the proposer",
            input: Some(Input::NewRound(height, Round::new(0))),
            expected_output: Some(Output::ScheduleTimeout(Timeout::propose(Round::new(0)))),
            expected_round: Round::new(0),
            new_state: State {
                height,
                round: Round::new(0),
                step: Step::Propose,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // Receive a propose timeout, prevote for nil (from v3)
        TestStep {
            desc: "Receive a propose timeout, prevote for nil (v3)",
            input: Some(Input::TimeoutElapsed(Timeout::propose(Round::new(0)))),
            expected_output: Some(Output::Vote(Vote::new_prevote(
                height,
                Round::new(0),
                NilOrVal::Nil,
                my_addr,
            ))),
            expected_round: Round::new(0),
            new_state: State {
                height,
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // Receive our own prevote v3
        TestStep {
            desc: "Receive our own prevote v3",
            input: None,
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height,
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // v1 prevotes for its own proposal
        TestStep {
            desc: "v1 prevotes for its own proposal in round 1",
            input: Some(Input::Vote(Vote::new_prevote(
                height,
                Round::new(1),
                NilOrVal::Val(value.id()),
                v1.address,
            ))),
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height,
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // v2 prevotes for v1 proposal in round 1, expected output is to move to next round
        TestStep {
            desc: "v2 prevotes for v1 proposal, we get +1/3 messages from future round",
            input: Some(Input::Vote(Vote::new_prevote(
                height,
                Round::new(1),
                NilOrVal::Val(value.id()),
                v2.address,
            ))),
            expected_output: Some(Output::NewRound(height, Round::new(1))),
            expected_round: Round::new(1),
            new_state: State {
                height,
                round: Round::new(1),
                step: Step::Unstarted,
                locked: None,
                valid: None,
                decision: None,
            },
        },
    ];

    run_steps(&mut driver, steps);
}

#[test]
fn driver_steps_skip_round_quorum_threshold() {
    let value = Value::new(9999);

    let sel = RotateProposer;

    let [(v1, _sk1), (v2, _sk2), (v3, sk3)] = make_validators([1, 2, 1]);

    // Proposer is v1, so we, v3, are not the proposer
    let (my_sk, my_addr) = (sk3, v3.address);

    let height = Height::new(1);
    let ctx = TestContext::new(my_sk.clone());

    let vs = ValidatorSet::new(vec![v1.clone(), v2.clone(), v3.clone()]);
    let mut driver = Driver::new(ctx, height, sel, vs, my_addr, Default::default());

    let steps = vec![
        // Start round 0, we, v3, are not the proposer
        TestStep {
            desc: "Start round 0, we, v3, are not the proposer",
            input: Some(Input::NewRound(height, Round::new(0))),
            expected_output: Some(Output::ScheduleTimeout(Timeout::propose(Round::new(0)))),
            expected_round: Round::new(0),
            new_state: State {
                height,
                round: Round::new(0),
                step: Step::Propose,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // Receive a propose timeout, prevote for nil (from v3)
        TestStep {
            desc: "Receive a propose timeout, prevote for nil (v3)",
            input: Some(Input::TimeoutElapsed(Timeout::propose(Round::new(0)))),
            expected_output: Some(Output::Vote(Vote::new_prevote(
                height,
                Round::new(0),
                NilOrVal::Nil,
                my_addr,
            ))),
            expected_round: Round::new(0),
            new_state: State {
                height,
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // Receive our own prevote v3
        TestStep {
            desc: "Receive our own prevote v3",
            input: None,
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height,
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // v1 prevotes for its own proposal
        TestStep {
            desc: "v1 prevotes for its own proposal in round 1",
            input: Some(Input::Vote(Vote::new_prevote(
                height,
                Round::new(1),
                NilOrVal::Val(value.id()),
                v1.address,
            ))),
            expected_output: None,
            expected_round: Round::new(0),
            new_state: State {
                height,
                round: Round::new(0),
                step: Step::Prevote,
                locked: None,
                valid: None,
                decision: None,
            },
        },
        // v2 prevotes for v1 proposal in round 1, expected output is to move to next round
        TestStep {
            desc: "v2 prevotes for v1 proposal, we get +1/3 messages from future round",
            input: Some(Input::Vote(Vote::new_prevote(
                height,
                Round::new(1),
                NilOrVal::Val(value.id()),
                v2.address,
            ))),
            expected_output: Some(Output::NewRound(height, Round::new(1))),
            expected_round: Round::new(1),
            new_state: State {
                height,
                round: Round::new(1),
                step: Step::Unstarted,
                locked: None,
                valid: None,
                decision: None,
            },
        },
    ];

    run_steps(&mut driver, steps);
}

fn run_steps(driver: &mut Driver<TestContext>, steps: Vec<TestStep>) {
    let mut input_from_prev_output = None;

    for step in steps {
        println!("Step: {}", step.desc);

        let input = step
            .input
            .unwrap_or_else(|| input_from_prev_output.unwrap());

        let mut outputs = driver.process(input).expect("execute succeeded");
        let output = outputs.pop();

        assert_eq!(output, step.expected_output, "expected output");
        assert_eq!(driver.round(), step.expected_round, "expected round");
        assert_eq!(driver.round_state, step.new_state, "new state");

        input_from_prev_output = output.and_then(output_to_input);
    }
}