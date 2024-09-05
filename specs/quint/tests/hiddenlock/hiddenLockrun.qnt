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
- there are other processes that together with "nut"" are 2f+1
- there is a faulty proposer in the first round who equivocates.
- the others are successful until sending precommit value
    - they set locked value and valid value
- "nut" doesn't see the lock (the faulty proposer prevotes nil to nut)
- everyone goes to the next round

- now there is a correct proposer, but it fails because "nut" doesn't have enough
  prevotes to execute line 28. 
  
TODO: From then on we are stuck forever. Thus the idea is to communicate Polkas


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

    // ... but nut is prevoting Nil because of the lock that is hidden from it
    .expect(Correct.forall(q => voteBuffer.get(q).contains(mkVote(Prevote, nut, 0, 1, Nil))))

    // nut has collected evidence 
    .expect(Faulty.forall(f => system.get(nut).es.bk.evidence.contains(Set( mkVote(Prevote, f, 0, 0, Nil), 
                                                                            mkVote(Prevote, f, 0, 0, Val(value))))))

    // nut now gets the certificate for round 0, but it is too late for this round.
    .then(deliverSomeCertificate(nut))

    // let's move on to the next round
    .then(ListDeliverAllVotes(Prevote, others, correctList, validatorSet, 0, 1, Val(value))) 
    .then(ListDeliverAllVotes(Prevote, List(nut), correctList, validatorSet, 0, 1, Nil))   
    .then(Correct.size().reps(_ => ListTakeAStep(correctList)))
    .then(3.reps(_ => ListTakeAStep(correctList)))
    .then(ListDeliverAllVotes(Precommit, correctList, correctList, validatorSet, 0, 1, Nil))
    .then(Correct.size().reps(_ => ListTakeAStep(correctList)))
    .then(valStep(nut))
    .then(everyoneReceivesProposalVR (correctList, validatorList, validatorSet, 0, 2, value, 0))

    // now we are good again. All correct nodes prevote for the value in round 2 ...
    .expect(Correct.forall(p => Correct.forall(q => voteBuffer.get(q).contains(mkVote(Prevote, p, 0, 2, Val(value))))))

}
