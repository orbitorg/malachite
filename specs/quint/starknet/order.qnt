// Helpers for computing totally-ordered prefixes.
//
// .load order.qnt order
//

module order {

	pure def max(s: Set[int]) : int = {
		s.fold(0, (s, x) => if (x > s) x else s)
	}

	pure def min(s: Set[int]) : int = {
		s.fold(0, (s, x) => if (x < s or s == 0) x else s)
	}

	pure def minMax(s: Set[int]) : (int, int) = {
		val a = s.fold({min: 0, max: 0}, (s, x) =>
			if (x < s.min or s.min == 0)
				if (x > s.max)
					{min: x, max: x}
				else {min: x, max: s.max}
			else  // x >= s.min
				if (x > s.max)
					{min: s.min, max: x}
				else
					s)
		(a.min, a.max)
	}

	pure def totalOrdered(s: Set[int]) : bool = {
		val a = minMax(s)
		range(a._1, a._2).foldl(true, (out, x) => if (out == true and x.in(s)) true else false)
	}

	pure def orderedPrefix(s: Set[int]) : Set[int] = {
		val a = minMax(s)
		range(a._1, a._2+1).foldl((Set(), true), (out, x) =>
			if (out._2 == true and x.in(s))
				 (out._1.union(Set(x)), true)
			else // not totally ordered, stop increasing prefix
				(out._1, false)
		)._1
	}

	pure def orderedPrefixFrom(first: int, s: Set[int]) : Set[int] = {
		if (first.in(s))
			orderedPrefix(s.filter(x => x >= first))
		else
			Set()
	}

	// Tests

	run minTest = all {
		min(Set()) == 0,
		min(Set(1)) == 1,
		min(Set(1, 2)) == 1,
		min(Set(2, 1)) == 1,
		min(Set(2, 5, 7, 1)) == 1,
		min(Set(7, 5, 7, 1)) == 1,
		min(Set(1, 5, 7, 1)) == 1,
	}

	run maxTest = all {
		max(Set()) == 0,
		max(Set(1)) == 1,
		max(Set(1, 2)) == 2,
		max(Set(2, 1)) == 2,
		max(Set(2, 5, 7, 1)) == 7,
		max(Set(7, 5, 7, 1)) == 7,
		max(Set(1, 5, 7, 1)) == 7,
	}

	run minMaxTest = all {
		minMax(Set()) == (0, 0),
		minMax(Set(1)) == (1, 1),
		minMax(Set(1, 2)) == (1, 2),
		minMax(Set(2, 1)) == (1, 2),
		minMax(Set(2, 5, 7, 1)) == (1, 7),
		minMax(Set(7, 5, 7, 1)) == (1, 7),
		minMax(Set(1, 5, 7, 1)) == (1, 7),
	}

	run totalOrderedTest = all {
		totalOrdered(Set()) == true,
		totalOrdered(Set(1)) == true,
		totalOrdered(Set(1, 2)) == true,
		totalOrdered(Set(2, 1)) == true,
		totalOrdered(Set(1, 3)) == false,
		totalOrdered(Set(3, 1)) == false,
		totalOrdered(Set(1, 2, 3, 4, 6)) == false,
	}

	run orderedPrefixTest = all {
		orderedPrefix(Set()) == Set(),
		orderedPrefix(Set(1)) == Set(1),
		orderedPrefix(Set(7)) == Set(7),
		orderedPrefix(Set(1, 7)) == Set(1),
		orderedPrefix(Set(7, 1)) == Set(1),
		orderedPrefix(Set(7, 1, 2)) == Set(1, 2),
		orderedPrefix(Set(7, 1, 2, 4)) == Set(1, 2),
		orderedPrefix(Set(3, 1, 2, 4)) == Set(1, 2, 3, 4),
		orderedPrefix(Set(3, 2, 4)) == Set(2, 3, 4),
		orderedPrefix(Set(1, 2, 4)) == Set(1, 2),
		orderedPrefix(Set(3, 1, 2)) == Set(1, 2, 3),
	}

}

// vim: syntax=quint
