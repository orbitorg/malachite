// -*- mode: Bluespec; -*-

module line32run {

import TendermintDSL.* from "../../specs/TendermintDSL"
export TendermintDSL.*

const testedVal : Address
const slowByz : Address

// the driver does not store invalid proposals. So we need to receive prevotes first and then the proposal
// first we send the correct guy to round 1


run runToLine32invalid = 
    val nextProposer = proposer(validatorSet, 0, 1)
    val propVal = "invalid"
    init
    .then(4.reps(_ => valStep(testedVal))) // start, pending step, consume timeout
    .then(deliverVote(testedVal, mkVote(Prevote, testedVal, 0, 0, Nil)))
    .then(ProcessDeliverAllVotes(Prevote, testedVal, faultyList.select(x => x != slowByz), validatorSet, 0, 0, Val(propVal)))
    .then(faultyList.select(x => x != slowByz).length().reps(_ => valStep(testedVal)))  
    .then(ProcessDeliverAllVotes(Precommit, testedVal, faultyList, validatorSet, 0, 0, Nil))
    .then(valStep(testedVal))
    .then(faultyList.length().reps(_ => valStep(testedVal))) 
    .then(
        all{
            assert(system.get(testedVal).timeouts.contains((PrecommitTimeout, 0, 0))),
            valStep(testedVal)
        }
    )
    .then(3.reps(_ => valStep(testedVal)))
    .then(deliverVote(testedVal, mkVote(Prevote, slowByz, 0, 0, Val(propVal))))
    .then(valStep(testedVal))
    .then(deliverProposal(testedVal, mkProposal(nextProposer, 0, 1, propVal, 0))) 
    .then(valStep(testedVal))
    .then(
        all{
            assert(system.get(testedVal).es.cs.step == PrevoteStep),
            assert(voteBuffer.get(testedVal).contains({ height: 0, round: 1, srcAddress: testedVal, valueId: Nil, voteType: Prevote })),            
            unchangedAll
        }

    )

}
