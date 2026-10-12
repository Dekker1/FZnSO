"""Scheduling and packing constraints.

Each case drives the edge of the constraint rather than its middle: a capacity
met exactly and then exceeded by one unit, a task of zero duration, a task of
zero resource use, tasks that touch end to start, a capacity of zero, a bin
numbered from something other than one. The Python predicate beside it is the
authority.

The registry gives no `_imp` file, on purpose, so an implied form is driven the
way MiniZinc builds one: `r -> fzn_<ident>(...)`, with `r` a decision, so the
case covers both a true and a false antecedent.
"""

from __future__ import annotations

from collections.abc import Sequence

from fznso_conform.case import Case, Decision

BOOLS = (True, False)


def _peak(start: Sequence[int], duration: Sequence[int], resource: Sequence[int]) -> int:
    """The largest total resource use at any one time.

    Load changes only where a task begins or ends, so those times are the whole
    of what the registry's `\\forall t` can see.
    """
    times = {t for s, d in zip(start, duration) for t in (s, s + d)}
    return max(
        (
            sum(r for s, d, r in zip(start, duration, resource) if s <= t < s + d)
            for t in times
        ),
        default=0,
    )


def _machine_loads(
    start: Sequence[int],
    duration: Sequence[int],
    resource: Sequence[int],
    machine: Sequence[int],
) -> dict[int, list[int]]:
    """Per machine, its load at every time a task on it begins or ends."""
    times = {t for s, d in zip(start, duration) for t in (s, s + d)}
    loads: dict[int, list[int]] = {}
    for m in set(machine):
        loads[m] = [
            sum(
                r
                for s, d, r, mm in zip(start, duration, resource, machine)
                if mm == m and s <= t < s + d
            )
            for t in sorted(times)
        ]
    return loads


CASES = (
    Case(
        ident="int_cumulative",
        why="the two resource needs sum to exactly the capacity, so they may overlap",
        decisions=(Decision("s1", range(0, 3)), Decision("s2", range(0, 3))),
        constraint="fzn_int_cumulative([s1, s2], [2, 2], [1, 2], 3)",
        holds=lambda a: _peak([a["s1"], a["s2"]], [2, 2], [1, 2]) <= 3,
    ),
    Case(
        ident="int_cumulative",
        why="one unit over the capacity, so the tasks may only touch end to start",
        decisions=(Decision("s1", range(0, 4)), Decision("s2", range(0, 4))),
        constraint="fzn_int_cumulative([s1, s2], [2, 2], [1, 2], 2)",
        holds=lambda a: _peak([a["s1"], a["s2"]], [2, 2], [1, 2]) <= 2,
    ),
    Case(
        ident="int_cumulative",
        why="a zero-duration task and a zero-resource task consume nothing",
        decisions=(
            Decision("s1", range(0, 3)),
            Decision("s2", range(0, 3)),
            Decision("s3", range(0, 3)),
        ),
        constraint="fzn_int_cumulative([s1, s2, s3], [0, 2, 2], [2, 0, 2], 2)",
        holds=lambda a: _peak([a["s1"], a["s2"], a["s3"]], [0, 2, 2], [2, 0, 2]) <= 2,
    ),
    Case(
        ident="int_cumulative",
        why="a capacity of zero, which only tasks consuming nothing can meet",
        decisions=(Decision("s1", range(0, 3)), Decision("s2", range(0, 3))),
        constraint="fzn_int_cumulative([s1, s2], [0, 2], [1, 0], 0)",
        holds=lambda a: _peak([a["s1"], a["s2"]], [0, 2], [1, 0]) <= 0,
    ),
    Case(
        ident="int_cumulative",
        why="the capacity is a decision, forced up to the peak load",
        decisions=(Decision("s1", range(0, 2)), Decision("cap", range(0, 4))),
        constraint="fzn_int_cumulative([s1, 0], [2, 2], [1, 2], cap)",
        holds=lambda a: _peak([a["s1"], 0], [2, 2], [1, 2]) <= a["cap"],
    ),
    Case(
        ident="int_cumulative_reif",
        why="a false reifying variable leaves the overloaded tasks free",
        decisions=(
            Decision("s1", range(0, 3)),
            Decision("s2", range(0, 3)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_int_cumulative_reif([s1, s2], [2, 2], [1, 2], 2, r)",
        holds=lambda a: a["r"] == (_peak([a["s1"], a["s2"]], [2, 2], [1, 2]) <= 2),
    ),
    Case(
        ident="int_cumulative",
        why="implied form, where a false antecedent constrains nothing",
        extra_includes=("fzn_int_cumulative_reif.mzn",),
        decisions=(
            Decision("s1", range(0, 3)),
            Decision("s2", range(0, 3)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_int_cumulative([s1, s2], [2, 2], [1, 2], 2)",
        holds=lambda a: not a["r"] or _peak([a["s1"], a["s2"]], [2, 2], [1, 2]) <= 2,
    ),
    Case(
        ident="int_cumulatives",
        why="two machines, one at capacity exactly and one with room to spare",
        decisions=(Decision("s1", range(0, 3)), Decision("m", range(1, 3))),
        constraint="fzn_int_cumulatives([s1, 0], [2, 2], [1, 1], [m, 1], [1, 2], 1, true)",
        holds=lambda a: all(
            load <= cap
            for m, cap in ((1, 1), (2, 2))
            for load in _machine_loads([a["s1"], 0], [2, 2], [1, 1], [a["m"], 1]).get(m, [0])
        ),
    ),
    Case(
        ident="int_cumulatives",
        why="machines numbered from zero, with tasks that may only touch end to start",
        decisions=(Decision("s1", range(0, 4)), Decision("s2", range(0, 4))),
        constraint="fzn_int_cumulatives([s1, s2], [2, 2], [1, 1], [0, 0], [1], 0, true)",
        holds=lambda a: all(
            load <= 1
            for load in _machine_loads([a["s1"], a["s2"]], [2, 2], [1, 1], [0, 0])[0]
        ),
    ),
    # A lower bound binds only while the machine is in use: read as the registry
    # writes it, over every time point, no positive lower bound could ever hold.
    Case(
        ident="int_cumulatives",
        why="lower bounds, which force the two tasks to run at exactly the same times",
        decisions=(Decision("s1", range(0, 3)), Decision("s2", range(0, 3))),
        constraint="fzn_int_cumulatives([s1, s2], [2, 2], [1, 1], [1, 1], [2], 1, false)",
        holds=lambda a: all(
            load == 0 or load >= 2
            for load in _machine_loads([a["s1"], a["s2"]], [2, 2], [1, 1], [1, 1])[1]
        ),
    ),
    Case(
        ident="int_cumulatives_reif",
        why="a false reifying variable leaves the overloaded machine free",
        decisions=(
            Decision("s1", range(0, 3)),
            Decision("s2", range(0, 3)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_int_cumulatives_reif([s1, s2], [2, 2], [1, 1], [1, 1], [1], 1, true, r)",
        holds=lambda a: a["r"]
        == all(
            load <= 1
            for load in _machine_loads([a["s1"], a["s2"]], [2, 2], [1, 1], [1, 1])[1]
        ),
    ),
    Case(
        ident="int_cumulatives",
        why="implied form, where a false antecedent constrains nothing",
        extra_includes=("fzn_int_cumulatives_reif.mzn",),
        decisions=(
            Decision("s1", range(0, 3)),
            Decision("s2", range(0, 3)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_int_cumulatives([s1, s2], [2, 2], [1, 1], [1, 1], [1], 1, true)",
        holds=lambda a: not a["r"]
        or all(
            load <= 1
            for load in _machine_loads([a["s1"], a["s2"]], [2, 2], [1, 1], [1, 1])[1]
        ),
    ),
    Case(
        ident="int_disjunctive",
        why="tasks that touch end to start do not overlap",
        decisions=(Decision("s1", range(0, 4)), Decision("s2", range(0, 4))),
        constraint="fzn_int_disjunctive([s1, s2], [2, 2])",
        holds=lambda a: a["s1"] + 2 <= a["s2"] or a["s2"] + 2 <= a["s1"],
    ),
    Case(
        ident="int_disjunctive",
        why="a zero-duration task may coincide with anything",
        decisions=(Decision("s1", range(0, 3)), Decision("s2", range(0, 3))),
        constraint="fzn_int_disjunctive([s1, s2], [0, 2])",
        holds=lambda a: True,
    ),
    Case(
        ident="int_disjunctive_reif",
        why="a false reifying variable lets the two tasks overlap",
        decisions=(
            Decision("s1", range(0, 3)),
            Decision("s2", range(0, 3)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_int_disjunctive_reif([s1, s2], [2, 2], r)",
        holds=lambda a: a["r"] == (a["s1"] + 2 <= a["s2"] or a["s2"] + 2 <= a["s1"]),
    ),
    Case(
        ident="int_disjunctive",
        why="implied form, where a false antecedent constrains nothing",
        extra_includes=("fzn_int_disjunctive_reif.mzn",),
        decisions=(
            Decision("s1", range(0, 3)),
            Decision("s2", range(0, 3)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_int_disjunctive([s1, s2], [2, 2])",
        holds=lambda a: not a["r"] or a["s1"] + 2 <= a["s2"] or a["s2"] + 2 <= a["s1"],
    ),
    Case(
        ident="int_disjunctive_strict",
        why="a zero-duration task must still be kept out of the other task",
        decisions=(Decision("s1", range(0, 3)), Decision("s2", range(0, 3))),
        constraint="fzn_int_disjunctive_strict([s1, s2], [0, 2])",
        holds=lambda a: a["s1"] + 0 <= a["s2"] or a["s2"] + 2 <= a["s1"],
    ),
    Case(
        ident="int_disjunctive_strict_reif",
        why="a false reifying variable lets the zero-duration task fall inside",
        decisions=(
            Decision("s1", range(0, 3)),
            Decision("s2", range(0, 3)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_int_disjunctive_strict_reif([s1, s2], [0, 2], r)",
        holds=lambda a: a["r"] == (a["s1"] + 0 <= a["s2"] or a["s2"] + 2 <= a["s1"]),
    ),
    Case(
        ident="int_disjunctive_strict",
        why="implied form, where a false antecedent constrains nothing",
        extra_includes=("fzn_int_disjunctive_strict_reif.mzn",),
        decisions=(
            Decision("s1", range(0, 3)),
            Decision("s2", range(0, 3)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_int_disjunctive_strict([s1, s2], [0, 2])",
        holds=lambda a: not a["r"] or a["s1"] + 0 <= a["s2"] or a["s2"] + 2 <= a["s1"],
    ),
    Case(
        ident="int_bin_packing",
        why="one bin filled exactly to capacity, the other overflowed by one unit",
        decisions=(Decision("b1", range(1, 3)), Decision("b2", range(1, 3))),
        constraint="fzn_int_bin_packing([b1, b2], [2, 1], [2, 1], 1)",
        holds=lambda a: all(
            sum(w for b, w in zip((a["b1"], a["b2"]), (2, 1)) if b == k) <= cap
            for k, cap in ((1, 2), (2, 1))
        ),
    ),
    Case(
        ident="int_bin_packing",
        why="bins numbered from zero, the second with a capacity of zero",
        decisions=(Decision("b1", range(0, 2)), Decision("b2", range(0, 2))),
        constraint="fzn_int_bin_packing([b1, b2], [1, 1], [2, 0], 0)",
        holds=lambda a: all(
            sum(w for b, w in zip((a["b1"], a["b2"]), (1, 1)) if b == k) <= cap
            for k, cap in ((0, 2), (1, 0))
        ),
    ),
    Case(
        ident="int_bin_packing",
        why="a bin index one past the last capacity, which the constraint rejects",
        decisions=(Decision("b1", range(1, 4)), Decision("b2", range(1, 4))),
        constraint="fzn_int_bin_packing([b1, b2], [1, 1], [2, 2], 1)",
        holds=lambda a: all(1 <= b <= 2 for b in (a["b1"], a["b2"]))
        and all(
            sum(w for b, w in zip((a["b1"], a["b2"]), (1, 1)) if b == k) <= 2
            for k in (1, 2)
        ),
    ),
    Case(
        ident="int_bin_packing_reif",
        why="a false reifying variable lets a bin overflow",
        decisions=(
            Decision("b1", range(1, 3)),
            Decision("b2", range(1, 3)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_int_bin_packing_reif([b1, b2], [2, 1], [2, 1], 1, r)",
        holds=lambda a: a["r"]
        == all(
            sum(w for b, w in zip((a["b1"], a["b2"]), (2, 1)) if b == k) <= cap
            for k, cap in ((1, 2), (2, 1))
        ),
    ),
    Case(
        ident="int_bin_packing",
        why="implied form, where a false antecedent constrains nothing",
        extra_includes=("fzn_int_bin_packing_reif.mzn",),
        decisions=(
            Decision("b1", range(1, 3)),
            Decision("b2", range(1, 3)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_int_bin_packing([b1, b2], [2, 1], [2, 1], 1)",
        holds=lambda a: not a["r"]
        or all(
            sum(w for b, w in zip((a["b1"], a["b2"]), (2, 1)) if b == k) <= cap
            for k, cap in ((1, 2), (2, 1))
        ),
    ),
    Case(
        ident="int_bin_packing_load",
        why="an item of zero weight leaves its bin's load alone",
        decisions=(
            Decision("b1", range(1, 3)),
            Decision("b2", range(1, 3)),
            Decision("l1", range(0, 2)),
            Decision("l2", range(0, 2)),
        ),
        constraint="fzn_int_bin_packing_load([b1, b2], [1, 0], [l1, l2], 1)",
        holds=lambda a: all(
            a[f"l{k}"] == sum(w for b, w in zip((a["b1"], a["b2"]), (1, 0)) if b == k)
            for k in (1, 2)
        ),
    ),
    Case(
        ident="int_bin_packing_load",
        why="loads numbered from zero, with one bin left empty",
        decisions=(
            Decision("b1", range(0, 2)),
            Decision("b2", range(0, 2)),
            Decision("l1", range(0, 3)),
            Decision("l2", range(0, 3)),
        ),
        constraint="fzn_int_bin_packing_load([b1, b2], [1, 1], [l1, l2], 0)",
        holds=lambda a: all(
            a[f"l{k + 1}"] == sum(w for b, w in zip((a["b1"], a["b2"]), (1, 1)) if b == k)
            for k in (0, 1)
        ),
    ),
    # The registry's formula names no sign condition, but the entry calls `xs` a
    # quantity, so the domain here is the non-negative one the notes imply.
    Case(
        ident="int_knapsack",
        why="an item of zero weight, and a quantity of zero",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("tw", range(0, 3)),
            Decision("tp", range(0, 5)),
        ),
        constraint="fzn_int_knapsack([2, 0], [1, 3], [x1, x2], tw, tp)",
        holds=lambda a: a["tw"] == 2 * a["x1"] and a["tp"] == a["x1"] + 3 * a["x2"],
    ),
    Case(
        ident="int_no_overlap",
        why="unit rectangles that merely touch, along an edge and at a corner",
        decisions=(Decision("x1", range(0, 3)), Decision("y1", range(0, 3))),
        constraint="fzn_int_no_overlap([x1, 0], [1, 1], [y1, 0], [1, 1])",
        holds=lambda a: (a["x1"], a["y1"]) != (0, 0),
    ),
    Case(
        ident="int_no_overlap",
        why="a rectangle of zero width, which the strict form already lets through",
        decisions=(Decision("x1", range(0, 3)), Decision("y1", range(0, 3))),
        constraint="fzn_int_no_overlap([x1, 1], [0, 1], [y1, 0], [1, 1])",
        holds=lambda a: True,
    ),
    Case(
        ident="int_no_overlap_reif",
        why="a false reifying variable lets the two rectangles coincide",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_int_no_overlap_reif([x1, 0], [1, 1], [y1, 0], [1, 1], r)",
        holds=lambda a: a["r"] == ((a["x1"], a["y1"]) != (0, 0)),
    ),
    Case(
        ident="int_no_overlap",
        why="implied form, where a false antecedent constrains nothing",
        extra_includes=("fzn_int_no_overlap_reif.mzn",),
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_int_no_overlap([x1, 0], [1, 1], [y1, 0], [1, 1])",
        holds=lambda a: not a["r"] or (a["x1"], a["y1"]) != (0, 0),
    ),
    Case(
        ident="int_no_overlap_nonstrict",
        why="a rectangle of zero height is ignored, so it may sit anywhere",
        decisions=(Decision("x1", range(0, 3)), Decision("y1", range(0, 3))),
        constraint="fzn_int_no_overlap_nonstrict([x1, 0], [1, 1], [y1, 0], [0, 1])",
        holds=lambda a: True,
    ),
    Case(
        ident="int_no_overlap_nonstrict",
        why="two rectangles of non-zero area, touching at a corner",
        decisions=(Decision("x1", range(0, 3)), Decision("y1", range(0, 3))),
        constraint="fzn_int_no_overlap_nonstrict([x1, 0], [1, 1], [y1, 0], [1, 1])",
        holds=lambda a: (a["x1"], a["y1"]) != (0, 0),
    ),
    Case(
        ident="int_no_overlap_nonstrict_reif",
        why="a false reifying variable lets the two rectangles coincide",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_int_no_overlap_nonstrict_reif([x1, 0], [1, 1], [y1, 0], [1, 1], r)",
        holds=lambda a: a["r"] == ((a["x1"], a["y1"]) != (0, 0)),
    ),
    Case(
        ident="int_no_overlap_nonstrict",
        why="implied form, where a false antecedent constrains nothing",
        extra_includes=("fzn_int_no_overlap_nonstrict_reif.mzn",),
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_int_no_overlap_nonstrict([x1, 0], [1, 1], [y1, 0], [1, 1])",
        holds=lambda a: not a["r"] or (a["x1"], a["y1"]) != (0, 0),
    ),
    Case(
        ident="int_no_overlap_nd",
        why="two unit boxes in two dimensions, touching at a corner",
        decisions=(Decision("x1", range(0, 3)), Decision("y1", range(0, 3))),
        constraint="fzn_int_no_overlap_nd(2, [x1, y1, 0, 0], [1, 1, 1, 1])",
        holds=lambda a: (a["x1"], a["y1"]) != (0, 0),
    ),
    Case(
        ident="int_no_overlap_nd",
        why="three dimensions, where one separating axis is enough",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("z1", range(0, 2)),
        ),
        constraint="fzn_int_no_overlap_nd(3, [x1, y1, z1, 0, 0, 0], [1, 1, 1, 1, 1, 1])",
        holds=lambda a: (a["x1"], a["y1"], a["z1"]) != (0, 0, 0),
    ),
    Case(
        ident="int_no_overlap_nd_reif",
        why="a false reifying variable lets the two boxes coincide",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_int_no_overlap_nd_reif(2, [x1, y1, 0, 0], [1, 1, 1, 1], r)",
        holds=lambda a: a["r"] == ((a["x1"], a["y1"]) != (0, 0)),
    ),
    Case(
        ident="int_no_overlap_nd",
        why="implied form, where a false antecedent constrains nothing",
        extra_includes=("fzn_int_no_overlap_nd_reif.mzn",),
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_int_no_overlap_nd(2, [x1, y1, 0, 0], [1, 1, 1, 1])",
        holds=lambda a: not a["r"] or (a["x1"], a["y1"]) != (0, 0),
    ),
    Case(
        ident="int_no_overlap_nd_nonstrict",
        why="a box with zero extent in one dimension is ignored",
        decisions=(Decision("x1", range(0, 3)), Decision("y1", range(0, 3))),
        constraint="fzn_int_no_overlap_nd_nonstrict(2, [x1, y1, 0, 0], [1, 0, 1, 1])",
        holds=lambda a: True,
    ),
    Case(
        ident="int_no_overlap_nd_nonstrict",
        why="two boxes of non-zero volume, touching at a corner",
        decisions=(Decision("x1", range(0, 3)), Decision("y1", range(0, 3))),
        constraint="fzn_int_no_overlap_nd_nonstrict(2, [x1, y1, 0, 0], [1, 1, 1, 1])",
        holds=lambda a: (a["x1"], a["y1"]) != (0, 0),
    ),
    Case(
        ident="int_no_overlap_nd_nonstrict_reif",
        why="a false reifying variable lets the two boxes coincide",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="fzn_int_no_overlap_nd_nonstrict_reif(2, [x1, y1, 0, 0], [1, 1, 1, 1], r)",
        holds=lambda a: a["r"] == ((a["x1"], a["y1"]) != (0, 0)),
    ),
    Case(
        ident="int_no_overlap_nd_nonstrict",
        why="implied form, where a false antecedent constrains nothing",
        extra_includes=("fzn_int_no_overlap_nd_nonstrict_reif.mzn",),
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint="r -> fzn_int_no_overlap_nd_nonstrict(2, [x1, y1, 0, 0], [1, 1, 1, 1])",
        holds=lambda a: not a["r"] or (a["x1"], a["y1"]) != (0, 0),
    ),
    # Both objects are the domino of rectangles 1 and 2, so only one of the two
    # rectangles of each clashes with the other object.
    Case(
        ident="int_no_overlap_shape",
        why="a two-rectangle shape, where only part of it clashes",
        decisions=(Decision("x1", range(0, 3)), Decision("y1", range(0, 2))),
        constraint=(
            "fzn_int_no_overlap_shape(2, [1, 1, 1, 1], [0, 0, 1, 0], [{1, 2}],"
            " [x1, y1, 0, 0], [1, 1])"
        ),
        holds=lambda a: not (
            {(a["x1"], a["y1"]), (a["x1"] + 1, a["y1"])} & {(0, 0), (1, 0)}
        ),
    ),
    Case(
        ident="int_no_overlap_shape",
        why="`kind` picks the shape, and a negative rectangle origin shifts it left",
        decisions=(Decision("x1", range(0, 3)), Decision("k", range(1, 3))),
        constraint=(
            "fzn_int_no_overlap_shape(2, [1, 1, 1, 1], [0, 0, -1, 0], [{1}, {2}],"
            " [x1, 0, 0, 0], [k, 1])"
        ),
        holds=lambda a: (a["x1"] - (a["k"] - 1), 0) != (0, 0),
    ),
    Case(
        ident="int_no_overlap_shape_reif",
        why="a false reifying variable lets the two objects coincide",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint=(
            "fzn_int_no_overlap_shape_reif(2, [1, 1], [0, 0], [{1}],"
            " [x1, y1, 0, 0], [1, 1], r)"
        ),
        holds=lambda a: a["r"] == ((a["x1"], a["y1"]) != (0, 0)),
    ),
    Case(
        ident="int_no_overlap_shape",
        why="implied form, where a false antecedent constrains nothing",
        extra_includes=("fzn_int_no_overlap_shape_reif.mzn",),
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint=(
            "r -> fzn_int_no_overlap_shape(2, [1, 1], [0, 0], [{1}],"
            " [x1, y1, 0, 0], [1, 1])"
        ),
        holds=lambda a: not a["r"] or (a["x1"], a["y1"]) != (0, 0),
    ),
    # The bounding box is one cell wider than the two unit objects together, so
    # the free object has exactly one place to go.
    Case(
        ident="int_no_overlap_shape_bounded",
        why="an object pressed against the far edge of the bounding box",
        decisions=(Decision("x1", range(0, 3)), Decision("y1", range(0, 3))),
        constraint=(
            "fzn_int_no_overlap_shape_bounded(2, [1, 1], [0, 0], [{1}],"
            " [x1, y1, 0, 0], [1, 1], [0, 0], [2, 1])"
        ),
        holds=lambda a: (a["x1"], a["y1"]) != (0, 0)
        and 0 <= a["x1"]
        and a["x1"] + 1 <= 2
        and 0 <= a["y1"]
        and a["y1"] + 1 <= 1,
    ),
    Case(
        ident="int_no_overlap_shape_bounded_reif",
        why="a false reifying variable lets an object leave the bounding box",
        decisions=(Decision("x1", range(0, 3)), Decision("r", BOOLS, "bool")),
        constraint=(
            "fzn_int_no_overlap_shape_bounded_reif(1, [1], [0], [{1}],"
            " [x1, 0], [1, 1], [0], [2], r)"
        ),
        holds=lambda a: a["r"] == (a["x1"] == 1),
    ),
    Case(
        ident="int_no_overlap_shape_bounded",
        why="implied form, where a false antecedent constrains nothing",
        extra_includes=("fzn_int_no_overlap_shape_bounded_reif.mzn",),
        decisions=(Decision("x1", range(0, 3)), Decision("r", BOOLS, "bool")),
        constraint=(
            "r -> fzn_int_no_overlap_shape_bounded(1, [1], [0], [{1}],"
            " [x1, 0], [1, 1], [0], [2])"
        ),
        holds=lambda a: not a["r"] or a["x1"] == 1,
    ),
    Case(
        ident="int_no_overlap_shape_tightest_bound",
        why="the box is squeezed onto the objects' own extent, in one dimension",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("lo", range(0, 3)),
            Decision("hi", range(1, 4)),
        ),
        constraint=(
            "fzn_int_no_overlap_shape_tightest_bound(1, [1], [0], [{1}],"
            " [x1, 0], [1, 1], [lo], [hi])"
        ),
        holds=lambda a: a["x1"] != 0
        and a["lo"] == min(a["x1"], 0)
        and a["hi"] == max(a["x1"] + 1, 1),
    ),
    Case(
        ident="int_no_overlap_shape_tightest_bound_reif",
        why="a false reifying variable leaves the bounding box loose",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("hi", range(1, 4)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint=(
            "fzn_int_no_overlap_shape_tightest_bound_reif(1, [1], [0], [{1}],"
            " [x1, 0], [1, 1], [0], [hi], r)"
        ),
        holds=lambda a: a["r"]
        == (a["x1"] != 0 and a["hi"] == max(a["x1"] + 1, 1)),
    ),
    Case(
        ident="int_no_overlap_shape_tightest_bound",
        why="implied form, where a false antecedent constrains nothing",
        extra_includes=("fzn_int_no_overlap_shape_tightest_bound_reif.mzn",),
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("hi", range(1, 4)),
            Decision("r", BOOLS, "bool"),
        ),
        constraint=(
            "r -> fzn_int_no_overlap_shape_tightest_bound(1, [1], [0], [{1}],"
            " [x1, 0], [1, 1], [0], [hi])"
        ),
        holds=lambda a: not a["r"]
        or (a["x1"] != 0 and a["hi"] == max(a["x1"] + 1, 1)),
    ),
)
