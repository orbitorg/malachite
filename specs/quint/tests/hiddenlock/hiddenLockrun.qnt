// -*- mode: Bluespec; -*-

module hiddenLockrun {

import TendermintDSL.* from "../../specs/TendermintDSL"
export TendermintDSL.*


const nut : Address
const otherSet: Set[Address]
val others = otherSet.fold(List(), (s, x) => s.append(x)) 

/*
- there need to be faulty processes
- there needs to be a correct node under test "nut"
- there are other processes that together with the faulty ones are 2f+1
- the others include the proposer.
- the others are successful until sending precommit value
    - they set locked value and valid value
- then faulty precommit nil are delivered to correct process

- the precommits from others are delivered to correct processes so that timeoutprecommit is started
- correct processes go to the next round
- there the proposer
    - uses the valid value stored to send proposal for valid value
    - correct processes take this to execute line 28 and prevote for this value
*/

run hiddenlockrun = 
    val nextProposer = proposer(validatorSet, 0, 1)
    nondet value = oneOf(Values)
    nondet otherValue = oneOf(Values.exclude(Set(value)))
    init

    // some assertions that are needed to make the run a run... and then everyony takes a step
    // to get started
    .then(all {
        // others should be at most 2/3.
        // if this assertion fails the set need to be set differently
        assert(3 * size(otherSet) <= 2 * size(validators)),  
        assert(3 * size(otherSet.union(Faulty)) > 2 * size(validators)), 
        // the following is required for fromPrevoteToPrecommit to have the right number of steps. 
        // We should better understand this to simplify test writing
        assert(2 * size(Faulty) + 1 == size(Correct)), 
        //assert(not(Faulty.union(otherSet).contains(nextProposer))), 
        //assert(Faulty.contains(proposer(validatorSet, 0, 0))),
        assert(not(nut == nextProposer) and otherSet.contains(nextProposer)),
        ListTakeAStep(correctList)
    })

    // proposer is faulty and equivocates
    .then(ListDeliverProposal(others, mkProposal(proposer(validatorSet, 0, 0), 0, 0, value, -1)))
    .then(deliverProposal(nut, mkProposal(proposer(validatorSet, 0, 0), 0, 0, otherValue, -1)))
    .then(ListTakeAStep(correctList))
    .then(ListTakeAStep(correctList))

    // All correct processes sent prevotes, but nut prevoted for the other value
    .expect(Correct.forall(p => voteBuffer.get(p).size() == Correct.size()))
    .expect(otherSet.forall(p => Correct.forall(q => voteBuffer.get(q).contains(mkVote(Prevote, p, 0, 0, Val(value))))))
    .expect(Correct.forall(q => voteBuffer.get(q).contains(mkVote(Prevote, nut, 0, 0, Val(otherValue)))))

    // the faulty nodes equivocate and prevote nil to nut and the value to the others
    .then(ProcessDeliverAllVotes(Prevote, nut, faultyList, validatorSet, 0, 0, Nil))
    .then(ListDeliverAllVotes(Prevote, faultyList, others, validatorSet, 0, 0, Val(value)))

     // then we deliver the remaining votes to the others
    .then(ListDeliverAllVotes(Prevote, others, correctList, validatorSet, 0, 0, Val(value)))

    // and after some steps the others have locked the value, but nut didn't
    .then(Correct.size().reps(_ => ListTakeAStep(correctList)))
    .then(2.reps(_ => ListTakeAStep(correctList)))
    .expect(all {
        otherSet.forall(p => system.get(p).es.cs.lockedValue == Val(value)),
        system.get(nut).es.cs.lockedValue == Nil
        })

    // now all enough precommits are passed around so that we can start a timeout and start 
    // the next round...
    .then(ListDeliverAllVotes(Precommit, others, correctList, validatorSet, 0, 0, Val(value))) 
    .then(ListDeliverAllVotes(Precommit, List(nut), correctList, validatorSet, 0, 0, Nil))
    .then(Correct.size().reps(_ => ListTakeAStep(correctList)))
    .then(3.reps(_ => ListTakeAStep(correctList)))

    // ... where a correct node is the proposer 
    .then(ListDeliverSomeProposal(correctList))

    // now we deliver also the other prevote of the faulty nodes to nut
    .then(ListDeliverAllVotes(Prevote, faultyList, List(nut), validatorSet, 0, 0, Val(value)))

    // now some messages are digested
    .then(1.reps(_ => ListTakeAStep(correctList)))
    .then(2.reps(_ => valStep(nut)))

    // and we have the liveness situation where the others have prevoted for the value ...
    .expect(otherSet.forall(p => Correct.forall(q => voteBuffer.get(q).contains(mkVote(Prevote, p, 0, 1, Val(value))))))

    // ... but nut is prevoting Nil because of the lock that is hidden form it
    .expect(Correct.forall(q => voteBuffer.get(q).contains(mkVote(Prevote, nut, 0, 1, Nil))))

    // nut has collected evidence 
    .expect(Faulty.forall(f => system.get(nut).es.bk.evidence.contains(Set( mkVote(Prevote, f, 0, 0, Nil), 
                                                                            mkVote(Prevote, f, 0, 0, Val(value))))))

    // TODO we should not get the commit in and get out of the situation
}
