// Validator set updates.
//
// .load valset.qnt valset
//
// quint run --invariant "invariants" valset.qnt 
// 

module valset {

	// Only the registration index is relevant
	type Registration = int

	type Block = {
		height: int,
		registrations: Set[Registration],
	}

	// committedRegistrations returns all registration added to every block
	// from the provided blockchain.
	pure def committedRegistrations(blockchain: List[Block]) : Set[Registration] = {
		blockchain.foldl(Set(), (registrations, block) =>
			registrations.union(block.registrations))
	}

	// newBlock produces a new block form the previous block and new registrations
	pure def newBlock(previous: Block, registrations: Set[Registration]) : Block = {
		val height = previous.height + 1
		{
			height: height,
			registrations: registrations,
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

	// invariants is an alias for all evaluated invariants
	def invariants = heightsInvariant and regsIntegrityInvariant

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
