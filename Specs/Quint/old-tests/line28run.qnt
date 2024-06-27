// -*- mode: Bluespec; -*-

module line28run {

import TendermintDSL.* from "./TendermintDSL"
export TendermintDSL.*

const otherSet : Set[Address_t]
val others = otherSet.fold(List(), (s, x) => s.append(x)) 

/*
- there need to be faulty processes
- there are other processes that together with the faulty ones are 2f+1
- the others include the proposer.
- the others are successful until sending precommit value
    - they set locked value and valid value
- then faulty precommit nil are delivered to correct processes
- the precommits from others are delivered to correct processes so that timeoutprecommit is started
- correct processes go to the next round
- there the proposer
    - uses the valid value stored to send proposal for valid value
    - correct processes take this to execute line 28 and prevote for this value
*/

run runToLine28 = {
    val nextProposer = Proposer (validatorSet, 0, 1)
    init

    // proposer sends proposal and correct processes receive -> send prevote value
    .then(all {
        // others should be at most 2/3.
        // if this assertion fails the set need to be set differently
        assert(3 * size(otherSet) <= 2 * size(validators)),  
        assert(3 * size(otherSet.union(Faulty)) > 2 * size(validators)),  
        everyoneReceivesProposal(correctList, validatorList, validatorSet, 0, 0, "blue")
    })
    // correct processes receive prevotes from correct processes -> send precommit    
    .then(fromPrevoteToPrecommit (correctList, correctList, validatorList, validatorSet, 0, 0, "blue"))
    // TODO assertion: valid and locked is set.
    // now the faulty nodes precommit nil
    .then(ListDeliverAllVotes("Precommit", faultyList, correctList, validatorSet, 0, 0, "nil"))
    // correct processes receive faulty precommit nil
    .then(faultyList.length().reps(_ => ListTakeAStep(correctList)))   
    // now the precommits from some correct processes are delivered, 
    // "other" processes are just enough so that timeoutPrecommit is started
    .then(ListDeliverAllVotes("Precommit", others, correctList, validatorSet, 0, 0, "blue"))
    .then(others.length().reps(_ => ListTakeAStep(correctList)))
    // TimeoutPrecommit is there an can fire and bring is to the next round.
    .then(all{
        assert(system.get(nextProposer).timeout.contains(("TimeoutPrecommit", 0, 0))),
    // proposer sends proposal and correct processes receive -> send prevote value on line 28   
        everyoneReceivesProposal(correctList, validatorList, validatorSet, 0, 0, "red")
    })
    .then(all{
        assert(voteBuffer.get(nextProposer).contains( { height: 0, id: "blue", round: 1, src: nextProposer, step: "Prevote" })),
        unchangedAll
    })
}

}