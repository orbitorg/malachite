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

	type L2Block = {
		height: int,
		registrations: Set[Registration],
		staged: Set[Registration],
		unstaged: Set[Registration],
		valset: ValidatorSet,
	}

	// Epochs for L2 blockchain
	val L2epochLength = 3

	// epoch to which a height belongs
	pure def epoch(height: int) : int = {
		if (height > 0)
			height / L2epochLength
		else
			0
	}

	// epochStart is the first height of an epoch
	pure def epochStart(epoch: int) : int = {
		if (epoch > 0)
			epoch * L2epochLength
		else
			0
	}

	// committedRegistrations returns all registration added to every block
	// from the provided blockchain.
	pure def committedRegistrations(blockchain: List[L2Block]) : Set[Registration] = {
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

	// newL2Block produces a new block form the previous block and new registrations
	pure def newL2Block(previous: L2Block, registrations: Set[Registration]) : L2Block = {
		val height = previous.height + 1
		// TODO: clean-up unstaged, that now is historic
		val unstaged = previous.unstaged.union(registrations)
		val valset =
			if (height == epochStart(epoch(height))) // new epoch
				nextValidatorSet(previous.valset, previous.staged)
			else
				previous.valset
		val staged =
			if (height == epochStart(epoch(height))) // new epoch
				orderedPrefixFrom(valset + 1, previous.unstaged)
			else
				previous.staged
		{
			height: height,
			registrations: registrations,
			staged: staged,
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
	// from all registrations committed up to the end of epoch E-2
	def valsetInvariant =
		val currentEpoch = epoch(lastL2.height)
		val epochMinusTwoEnd = epochStart(currentEpoch - 1) - 1
		val commRegs = committedRegistrations(L2.slice(0, epochMinusTwoEnd + 1))
		lastL2.valset == nextValidatorSet(0, commRegs)

	// orderedStagedInvariant: all staged registrations are applicable,
	// namely, they follow the validator set and are in order. Moreover,
	// registrations already applied should not be still staged.
	def orderedStagedInvariant =
		orderedPrefixFrom(lastL2.valset + 1, lastL2.staged) == lastL2.staged

	// invariants is an alias for all evaluated invariants
	def invariants =
		heightsInvariant and
		regsIntegrityInvariant and
		regsUnstagedInvariant and
		valsetInvariant	and
		orderedStagedInvariant

	//
	// STATE MACHINE
	//

	var regs : Set[Registration]
	var L2 : List[L2Block]

	// bootstrap
	action init = all {
		regs' = Set(),
		L2' = List({
			height: 0,
			registrations: Set(),
			staged: Set(),
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
	action addL2Block = all {
		// FIXME: limit the size of new registrations?
		nondet registrations = regs.powerset().oneOf()
		L2' = L2.append(newL2Block(lastL2, registrations)),
		regs' = regs,
	}

	// any state transition
	action step = any {
		addRegistration,
		addL2Block,
	}

}

// vim: syntax=quint
