// Validator set updates.
//
// .load valset.qnt valset
//
// quint run --invariant "invariants" valset.qnt 
// 

module valset {

	import order.* from "./order"

	// Only the registration index is relevant
	type Registration = int

	// The last registration applied
	type ValidatorSet = Registration

	type Block = {
		height: int,
		registrations: Set[Registration],
		unstaged: Set[Registration],
		valset: ValidatorSet,
	}

	// Epochs for L2 blockchain
	val L2epochLength = 3

	// epoch to which a height belongs
	pure def epoch(height: int) : int = {
		height / L2epochLength
	}

	// epochStart is the first height of an epoch
	pure def epochStart(epoch: int) : int = {
		epoch * L2epochLength
	}

	// committedRegistrations returns all registration added to every block
	// from the provided blockchain.
	pure def committedRegistrations(blockchain: List[Block]) : Set[Registration] = {
		blockchain.foldl(Set(), (registrations, block) =>
			registrations.union(block.registrations))
	}

	// nextValidatorSet attemtps to extend the previous validator set with
	// a set of registrations
	pure def nextValidatorSet(previous: ValidatorSet, registrations: Set[Registration]) : ValidatorSet = {
		// TODO: orderedPrefixFrom should start from first + 1,
		val extendedRegistrations = orderedPrefixFrom(previous + 1, registrations)
		// TODO: use a max helper
		if (extendedRegistrations != Set())
			max(extendedRegistrations)
		else
			previous
	}

	// newBlock produces a new block form the previous block and new registrations
	pure def newBlock(previous: Block, registrations: Set[Registration]) : Block = {
		val height = previous.height + 1
		val unstaged = previous.unstaged.union(registrations)
		val valset =
			if (height == epochStart(epoch(height))) // new epoch
				nextValidatorSet(previous.valset, previous.unstaged)
			else
				previous.valset
		{
			height: height,
			registrations: registrations,
			unstaged: unstaged,
			valset: valset,
		}
	}

	//
	// HELPERS
	//

	def lastL2 = L2[length(L2) - 1]

	//
	// INVARIANTS
	//

	// heightsInvariant: heights increase monotonically (trivial)
	def heightsInvariant = 
		val prevH = if (length(L2) > 1) L2[length(L2) - 2].height else -1
		lastL2.height == prevH + 1

	// regsIntegrityInvariant: committed registrations are valid and come
	// from the state variable regs (trivial)
	def regsIntegrityInvariant =
		val commRegs = committedRegistrations(L2)
		commRegs.filter(x => x <= 0) == Set() and commRegs.subseteq(regs)

	// regsUnstagedInvariant: committed invariants become unstaged
	def regsUnstagedInvariant =
		val commRegs = committedRegistrations(L2)
		lastL2.unstaged == commRegs

	// valsetInvariant: validator set for a block of epoch E is produced
	// from all registrations committed up to the previous epoch E-1
	def valsetInvariant =
		val currentEpoch = epoch(lastL2.height)
		val commRegs = committedRegistrations(L2.slice(0, epochStart(currentEpoch)))
		lastL2.valset == nextValidatorSet(0, commRegs)

	// invariants is an alias for all evaluated invariants
	def invariants =
		heightsInvariant and
		regsIntegrityInvariant and
		regsUnstagedInvariant and
		valsetInvariant

	//
	// STATE MACHINE
	//

	var regs : Set[Registration]
	var L2 : List[Block]

	// bootstrap
	action init = all {
		regs' = Set(),
		L2' = List({
			height: 0,
			registrations: Set(),
			unstaged: Set(),
			valset: 0,
		}),
	}

	// add registration
	action addRegistration = all {
		// Registration 0 is not allowed
		val r = size(regs) + 1
		regs' = regs.union(Set(r)),
		L2' = L2,
	}

	// add block
	action addBlock = all {
		// FIXME: limit the size of new registrations?
		nondet registrations = regs.powerset().oneOf()
		L2' = L2.append(newBlock(lastL2, registrations)),
		regs' = regs,
	}

	// any state transition
	action step = any {
		addRegistration,
		addBlock,
	}

}

// vim: syntax=quint
