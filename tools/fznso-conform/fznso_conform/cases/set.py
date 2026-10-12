"""Set constraints.

Each case drives the edge of the constraint rather than its middle: the empty
set against the full universe, two sets sharing one element against two that
are disjoint, a cardinality at its bound and either side of it, a universe that
does not start at one. A set decision's domain lists every subset of its
universe, because the solver is free to answer with any of them.

The registry gives no `_imp` file, on purpose, so an implied form is driven the
way MiniZinc builds one: `r -> fzn_<ident>(...)`, with `r` a decision, so the
case covers both a true and a false antecedent. The `_reif` file travels with
it, which is where MiniZinc looks when the solver declares the base natively.
"""

from __future__ import annotations

from collections.abc import Sequence
from itertools import combinations

from fznso_conform.case import Case, Decision

BOOLS = (True, False)


def _subsets(universe: Sequence[int]) -> tuple[frozenset[int], ...]:
    """Every subset of a universe, smallest first."""
    return tuple(
        frozenset(c) for n in range(len(universe) + 1) for c in combinations(universe, n)
    )


def _order(s: frozenset[int]) -> tuple[int, ...]:
    """The key of the standard total order on sets.

    Comparing these tuples compares sets by their smallest differing element,
    with the set that ran out of elements first the smaller of the two.
    """
    return tuple(sorted(s))


def _channels(ints: Sequence[int], sets: Sequence[frozenset[int]]) -> bool:
    """Whether element i names a set holding i, with every membership mirrored.

    Both arrays are numbered from one, which is the offset every case passes.
    """
    return all(
        (v == j + 1) == (i + 1 in s)
        for i, v in enumerate(ints)
        for j, s in enumerate(sets)
    )


THREE = _subsets((1, 2, 3))
TWO = _subsets((1, 2))
ONE = _subsets((1,))

CASES = (
    Case(
        ident="int_in",
        why="membership against the empty set and against the full universe",
        decisions=(Decision("x", range(1, 4)), Decision("s", THREE, "set of int")),
        constraint="fzn_int_in(x, s)",
        holds=lambda a: a["x"] in a["s"],
    ),
    Case(
        ident="int_in",
        why="a universe that starts at zero rather than one",
        decisions=(
            Decision("x", range(0, 3)),
            Decision("s", _subsets((0, 1, 2)), "set of int"),
        ),
        constraint="fzn_int_in(x, s)",
        holds=lambda a: a["x"] in a["s"],
    ),
    Case(
        ident="int_in_reif",
        why="a false reifying variable asserts non-membership, not nothing",
        decisions=(
            Decision("x", range(1, 3)),
            Decision("s", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_int_in_reif(x, s, r)",
        holds=lambda a: a["r"] == (a["x"] in a["s"]),
    ),
    Case(
        ident="int_in",
        why="implied form, where a false antecedent constrains nothing",
        decisions=(
            Decision("x", range(1, 3)),
            Decision("s", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_int_in(x, s)",
        holds=lambda a: not a["r"] or a["x"] in a["s"],
        extra_includes=("fzn_int_in_reif.mzn",),
    ),
    Case(
        ident="set_of_int_max_overlap",
        why="an overlap bound of zero, which forces the two sets apart",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
        ),
        constraint="fzn_set_of_int_max_overlap([s, t], 0)",
        holds=lambda a: len(a["s"] & a["t"]) <= 0,
    ),
    Case(
        ident="set_of_int_max_overlap",
        why="sharing one element is at the bound, sharing two is over it",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
        ),
        constraint="fzn_set_of_int_max_overlap([s, t], 1)",
        holds=lambda a: len(a["s"] & a["t"]) <= 1,
    ),
    Case(
        ident="set_of_int_max_overlap_reif",
        why="a false reifying variable asserts the sets do share an element",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_set_of_int_max_overlap_reif([s, t], 0, r)",
        holds=lambda a: a["r"] == (len(a["s"] & a["t"]) <= 0),
    ),
    Case(
        ident="set_of_int_max_overlap",
        why="implied form, where a false antecedent constrains nothing",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_set_of_int_max_overlap([s, t], 0)",
        holds=lambda a: not a["r"] or len(a["s"] & a["t"]) <= 0,
        extra_includes=("fzn_set_of_int_max_overlap_reif.mzn",),
    ),
    Case(
        ident="set_of_int_card",
        why="every cardinality from the empty set up to the full universe",
        decisions=(Decision("s", THREE, "set of int"), Decision("n", range(0, 4))),
        constraint="fzn_set_of_int_card(s, n)",
        holds=lambda a: a["n"] == len(a["s"]),
    ),
    Case(
        ident="set_of_int_card",
        why="a universe that starts at three, so the count is not the largest member",
        decisions=(
            Decision("s", _subsets((3, 4, 5)), "set of int"),
            Decision("n", range(0, 4)),
        ),
        constraint="fzn_set_of_int_card(s, n)",
        holds=lambda a: a["n"] == len(a["s"]),
    ),
    Case(
        ident="set_of_int_eq",
        why="equality including empty against empty, and a singleton against a singleton",
        decisions=(
            Decision("s", THREE, "set of int"),
            Decision("t", THREE, "set of int"),
        ),
        constraint="fzn_set_of_int_eq(s, t)",
        holds=lambda a: a["s"] == a["t"],
    ),
    Case(
        ident="set_of_int_eq_reif",
        why="a false reifying variable asserts inequality, not nothing",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_set_of_int_eq_reif(s, t, r)",
        holds=lambda a: a["r"] == (a["s"] == a["t"]),
    ),
    Case(
        ident="set_of_int_eq",
        why="implied form, where a false antecedent constrains nothing",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_set_of_int_eq(s, t)",
        holds=lambda a: not a["r"] or a["s"] == a["t"],
        extra_includes=("fzn_set_of_int_eq_reif.mzn",),
    ),
    Case(
        ident="set_of_int_ne",
        why="a singleton differs from the singleton beside it and from the empty set",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
        ),
        constraint="fzn_set_of_int_ne(s, t)",
        holds=lambda a: a["s"] != a["t"],
    ),
    Case(
        ident="set_of_int_ne_reif",
        why="a false reifying variable asserts equality, not nothing",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_set_of_int_ne_reif(s, t, r)",
        holds=lambda a: a["r"] == (a["s"] != a["t"]),
    ),
    Case(
        ident="set_of_int_ne",
        why="implied form, where a false antecedent constrains nothing",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_set_of_int_ne(s, t)",
        holds=lambda a: not a["r"] or a["s"] != a["t"],
        extra_includes=("fzn_set_of_int_ne_reif.mzn",),
    ),
    Case(
        ident="set_of_int_subset",
        why="the empty set is below everything, the universe only below itself",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
        ),
        constraint="fzn_set_of_int_subset(s, t)",
        holds=lambda a: a["s"] <= a["t"],
    ),
    Case(
        ident="set_of_int_subset_reif",
        why="a false reifying variable asserts a member outside the second set",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_set_of_int_subset_reif(s, t, r)",
        holds=lambda a: a["r"] == (a["s"] <= a["t"]),
    ),
    Case(
        ident="set_of_int_subset",
        why="implied form, where a false antecedent constrains nothing",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_set_of_int_subset(s, t)",
        holds=lambda a: not a["r"] or a["s"] <= a["t"],
        extra_includes=("fzn_set_of_int_subset_reif.mzn",),
    ),
    # On a two-element universe the order is {} < {1} < {1,2} < {2}: a superset
    # may be the smaller of the pair, which a subset test would get wrong.
    Case(
        ident="set_of_int_le",
        why="the total order on sets, at the smallest differing element",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
        ),
        constraint="fzn_set_of_int_le(s, t)",
        holds=lambda a: _order(a["s"]) <= _order(a["t"]),
    ),
    Case(
        ident="set_of_int_le_reif",
        why="a false reifying variable asserts the strict order the other way",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_set_of_int_le_reif(s, t, r)",
        holds=lambda a: a["r"] == (_order(a["s"]) <= _order(a["t"])),
    ),
    Case(
        ident="set_of_int_le",
        why="implied form, where a false antecedent constrains nothing",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_set_of_int_le(s, t)",
        holds=lambda a: not a["r"] or _order(a["s"]) <= _order(a["t"]),
        extra_includes=("fzn_set_of_int_le_reif.mzn",),
    ),
    Case(
        ident="set_of_int_lt",
        why="the strict order, which no set satisfies against itself",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
        ),
        constraint="fzn_set_of_int_lt(s, t)",
        holds=lambda a: _order(a["s"]) < _order(a["t"]),
    ),
    Case(
        ident="set_of_int_lt_reif",
        why="a false reifying variable asserts the non-strict order the other way",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_set_of_int_lt_reif(s, t, r)",
        holds=lambda a: a["r"] == (_order(a["s"]) < _order(a["t"])),
    ),
    Case(
        ident="set_of_int_lt",
        why="implied form, where a false antecedent constrains nothing",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_set_of_int_lt(s, t)",
        holds=lambda a: not a["r"] or _order(a["s"]) < _order(a["t"]),
        extra_includes=("fzn_set_of_int_lt_reif.mzn",),
    ),
    Case(
        ident="set_of_int_union",
        why="the empty set and the full universe as operands",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("u", TWO, "set of int"),
        ),
        constraint="fzn_set_of_int_union(s, t, u)",
        holds=lambda a: a["u"] == a["s"] | a["t"],
    ),
    Case(
        ident="set_of_int_intersect",
        why="two disjoint sets, and two sharing one element",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("u", TWO, "set of int"),
        ),
        constraint="fzn_set_of_int_intersect(s, t, u)",
        holds=lambda a: a["u"] == a["s"] & a["t"],
    ),
    Case(
        ident="set_of_int_diff",
        why="removing the full universe, and removing nothing",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("u", TWO, "set of int"),
        ),
        constraint="fzn_set_of_int_diff(s, t, u)",
        holds=lambda a: a["u"] == a["s"] - a["t"],
    ),
    Case(
        ident="set_of_int_symdiff",
        why="two equal sets give nothing, two disjoint ones give their union",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("u", TWO, "set of int"),
        ),
        constraint="fzn_set_of_int_symdiff(s, t, u)",
        holds=lambda a: a["u"] == a["s"] ^ a["t"],
    ),
    Case(
        ident="set_of_int_array_union",
        why="a two-element array whose members are the empty set and the universe",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("u", TWO, "set of int"),
        ),
        constraint="fzn_set_of_int_array_union([s, t], u)",
        holds=lambda a: a["u"] == a["s"] | a["t"],
    ),
    Case(
        ident="set_of_int_array_intersect",
        why="a one-element array, where the result is that element",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("u", TWO, "set of int"),
        ),
        constraint="fzn_set_of_int_array_intersect([s], u)",
        holds=lambda a: a["u"] == a["s"],
    ),
    Case(
        ident="set_of_int_array_intersect",
        why="two sets sharing one element, and two that are disjoint",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("u", TWO, "set of int"),
        ),
        constraint="fzn_set_of_int_array_intersect([s, t], u)",
        holds=lambda a: a["u"] == a["s"] & a["t"],
    ),
    Case(
        ident="int_array_union",
        why="the values an array takes, with both entries equal",
        decisions=(
            Decision("x", range(1, 3)),
            Decision("y", range(1, 3)),
            Decision("u", TWO, "set of int"),
        ),
        constraint="fzn_int_array_union([x, y], u)",
        holds=lambda a: a["u"] == {a["x"], a["y"]},
    ),
    Case(
        ident="set_of_int_partition",
        why="two blocks over a two-element universe, so one is empty when the other is full",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
        ),
        constraint="fzn_set_of_int_partition([s, t], 1..2)",
        holds=lambda a: a["s"] | a["t"] == {1, 2} and not (a["s"] & a["t"]),
    ),
    Case(
        ident="set_of_int_partition",
        why="a universe that starts at zero rather than one",
        decisions=(
            Decision("s", _subsets((0, 1)), "set of int"),
            Decision("t", _subsets((0, 1)), "set of int"),
        ),
        constraint="fzn_set_of_int_partition([s, t], 0..1)",
        holds=lambda a: a["s"] | a["t"] == {0, 1} and not (a["s"] & a["t"]),
    ),
    Case(
        ident="set_of_int_partition_reif",
        why="a false reifying variable allows an overlap or an uncovered element",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_set_of_int_partition_reif([s, t], 1..2, r)",
        holds=lambda a: a["r"]
        == (a["s"] | a["t"] == {1, 2} and not (a["s"] & a["t"])),
    ),
    Case(
        ident="set_of_int_partition",
        why="implied form, where a false antecedent constrains nothing",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_set_of_int_partition([s, t], 1..2)",
        holds=lambda a: not a["r"]
        or (a["s"] | a["t"] == {1, 2} and not (a["s"] & a["t"])),
        extra_includes=("fzn_set_of_int_partition_reif.mzn",),
    ),
    Case(
        ident="set_of_int_to_bool",
        why="the characteristic vector of the empty set and of the full universe",
        decisions=(
            Decision("s", TWO, "set of int"),
            Decision("b1", BOOLS, "bool"),
            Decision("b2", BOOLS, "bool"),
        ),
        constraint="fzn_set_of_int_to_bool(s, [b1, b2], 1)",
        holds=lambda a: a["b1"] == (1 in a["s"]) and a["b2"] == (2 in a["s"]),
    ),
    Case(
        ident="set_of_int_to_bool",
        why="a universe that starts at zero, so the offset is not one",
        decisions=(
            Decision("s", _subsets((0, 1)), "set of int"),
            Decision("b1", BOOLS, "bool"),
            Decision("b2", BOOLS, "bool"),
        ),
        constraint="fzn_set_of_int_to_bool(s, [b1, b2], 0)",
        holds=lambda a: a["b1"] == (0 in a["s"]) and a["b2"] == (1 in a["s"]),
    ),
    Case(
        ident="int_set_channel",
        why="each index lands in exactly one set, so the sets partition the indices",
        decisions=(
            Decision("x", range(1, 3)),
            Decision("y", range(1, 3)),
            Decision("s", TWO, "set of int"),
            Decision("t", TWO, "set of int"),
        ),
        constraint="fzn_int_set_channel([x, y], 1, [s, t], 1)",
        holds=lambda a: _channels([a["x"], a["y"]], [a["s"], a["t"]]),
    ),
    Case(
        ident="int_set_channel_reif",
        why="a false reifying variable breaks the channelling in one direction or the other",
        decisions=(
            Decision("x", range(1, 3)),
            Decision("s", ONE, "set of int"),
            Decision("t", ONE, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_int_set_channel_reif([x], 1, [s, t], 1, r)",
        holds=lambda a: a["r"] == _channels([a["x"]], [a["s"], a["t"]]),
    ),
    Case(
        ident="int_set_channel",
        why="implied form, where a false antecedent constrains nothing",
        decisions=(
            Decision("x", range(1, 3)),
            Decision("s", ONE, "set of int"),
            Decision("t", ONE, "set of int"),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_int_set_channel([x], 1, [s, t], 1)",
        holds=lambda a: not a["r"] or _channels([a["x"]], [a["s"], a["t"]]),
        extra_includes=("fzn_int_set_channel_reif.mzn",),
    ),
    Case(
        ident="int_array_roots",
        why="an empty value set gives an empty index set, a full one gives every index",
        decisions=(
            Decision("x", range(1, 3)),
            Decision("y", range(1, 3)),
            Decision("v", TWO, "set of int"),
            Decision("s", TWO, "set of int"),
        ),
        constraint="fzn_int_array_roots([x, y], 1, v, s)",
        holds=lambda a: all(
            (i in a["s"]) == (v in a["v"]) for i, v in ((1, a["x"]), (2, a["y"]))
        ),
    ),
    Case(
        ident="int_array_range",
        why="an empty index domain gives an empty image, and both entries may coincide",
        decisions=(
            Decision("x", range(1, 3)),
            Decision("y", range(1, 3)),
            Decision("d", TWO, "set of int"),
            Decision("im", TWO, "set of int"),
        ),
        constraint="fzn_int_array_range([x, y], 1, d, im)",
        holds=lambda a: a["im"] == {[a["x"], a["y"]][i - 1] for i in a["d"]},
    ),
)
