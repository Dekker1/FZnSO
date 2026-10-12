"""Array constraints.

Each case drives an edge of the constraint rather than its middle: the first and
the last position, a position one past each end, an index set that does not
start at one, a one-element and an empty array, duplicate values that let
several positions answer the same way, and both polarities of a reified form.

Every position argument is read in the caller's numbering, which starts at the
`offset` that travels with the array, so the predicates below convert with
`index - offset` and nothing assumes one.
"""

from __future__ import annotations

import math

from fznso_conform.case import Case, Decision


def _at(xs, offset, index):
    """The element at a caller-numbered position, or `None` when out of range."""
    k = index - offset
    return xs[k] if 0 <= k < len(xs) else None


def _nd_at(xs, offsets, sizes, indices):
    """The element of a row-major array at one position per dimension.

    `None` when a position leaves its own dimension, which the flat index on its
    own would let a later dimension make up for.
    """
    if any(i < o or i >= o + s for i, o, s in zip(indices, offsets, sizes)):
        return None
    flat = sum(
        (i - o) * math.prod(sizes[d + 1 :])
        for d, (i, o) in enumerate(zip(indices, offsets))
    )
    return xs[flat]


def _if_then_else(conditions, values, result):
    for condition, value in zip(conditions, values):
        if condition:
            return result == value
    return True


def _arg_max(xs, offset, index):
    k = index - offset
    return (
        0 <= k < len(xs) and xs[k] == max(xs) and all(x < xs[k] for x in xs[:k])
    )


def _arg_min(xs, offset, index):
    k = index - offset
    return (
        0 <= k < len(xs) and xs[k] == min(xs) and all(x > xs[k] for x in xs[:k])
    )


def _arg_val(xs, offset, value, index):
    k = index - offset
    return 0 <= k < len(xs) and xs[k] == value and all(x != value for x in xs[:k])


def _write(xs, offset, index, value):
    """`xs` with one position overwritten, unchanged when the position is outside."""
    return [value if j == index - offset else x for j, x in enumerate(xs)]


def _writes(xs, offset, indices, values, result):
    """Every write lands, and an untouched position keeps the array's value.

    A position outside the array has nowhere to write to, so the constraint
    cannot hold; `int_array_writes_sequential` ignores such a write instead.
    """
    positions = [i - offset for i in indices]
    if any(not 0 <= p < len(xs) for p in positions):
        return False
    if any(result[p] != v for p, v in zip(positions, values)):
        return False
    return all(result[j] == xs[j] for j in range(len(xs)) if j not in positions)


def _writes_sequential(xs, offset, indices, values):
    out = list(xs)
    for index, value in zip(indices, values):
        k = index - offset
        if 0 <= k < len(xs):
            out[k] = value
    return out


def _arg_sort(xs, offset, permutation):
    positions = [p - offset for p in permutation]
    if sorted(positions) != list(range(len(xs))):
        return False
    keys = [(xs[p], p) for p in positions]
    return all(a < b for a, b in zip(keys, keys[1:]))


CASES = (
    Case(
        ident="int_array_element",
        why="the first and last position, and one past each end",
        decisions=(Decision("i", range(0, 5)), Decision("v", [5, 7, 9])),
        constraint="fzn_int_array_element([5, 7, 9], 1, i, v)",
        holds=lambda a: a["v"] == _at([5, 7, 9], 1, a["i"]),
    ),
    Case(
        ident="int_array_element",
        why="an index set numbered from minus one",
        decisions=(Decision("i", range(-2, 3)), Decision("v", [5, 7, 9])),
        constraint="fzn_int_array_element([5, 7, 9], -1, i, v)",
        holds=lambda a: a["v"] == _at([5, 7, 9], -1, a["i"]),
    ),
    Case(
        ident="int_array_element",
        why="a one-element array",
        decisions=(Decision("i", range(0, 3)), Decision("v", range(3, 6))),
        constraint="fzn_int_array_element([4], 1, i, v)",
        holds=lambda a: a["v"] == _at([4], 1, a["i"]),
    ),
    Case(
        ident="int_array_element",
        why="an empty array has no element to select",
        decisions=(Decision("i", range(0, 3)), Decision("v", range(1, 3))),
        constraint="fzn_int_array_element(empty, 1, i, v)",
        holds=lambda a: a["v"] == _at([], 1, a["i"]),
        params="array [1..0] of var int: empty = [];",
    ),
    Case(
        ident="int_array_element",
        why="a variable position into an array of decisions, with duplicate values",
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            Decision("i", range(0, 5)),
            Decision("v", range(1, 3)),
        ),
        constraint="fzn_int_array_element([x1, x2, x2], 1, i, v)",
        holds=lambda a: a["v"] == _at([a["x1"], a["x2"], a["x2"]], 1, a["i"]),
    ),
    Case(
        ident="bool_array_element",
        why="an index set numbered from two, and one past each end",
        decisions=(Decision("i", range(1, 6)), Decision("v", [False, True], kind="bool")),
        constraint="fzn_bool_array_element([true, false, true], 2, i, v)",
        holds=lambda a: a["v"] == _at([True, False, True], 2, a["i"]),
    ),
    Case(
        ident="bool_array_element",
        why="a variable position into an array of decisions",
        decisions=(
            Decision("b1", [False, True], kind="bool"),
            Decision("b2", [False, True], kind="bool"),
            Decision("i", range(0, 4)),
            Decision("v", [False, True], kind="bool"),
        ),
        constraint="fzn_bool_array_element([b1, b2], 1, i, v)",
        holds=lambda a: a["v"] == _at([a["b1"], a["b2"]], 1, a["i"]),
    ),
    Case(
        ident="int_array_element_nd",
        why="two dimensions, and a position past the end of the faster one",
        # The faster dimension runs 1..3, so `j = 4` keeps the flat index inside
        # the array; each position has to stay inside its own dimension anyway.
        decisions=(
            Decision("i", range(0, 4)),
            Decision("j", range(0, 5)),
            Decision("v", [11, 12, 13, 21, 22, 23]),
        ),
        constraint="fzn_int_array_element_nd([11, 12, 13, 21, 22, 23], [1, 1], [2, 3], [i, j], v)",
        holds=lambda a: a["v"]
        == _nd_at([11, 12, 13, 21, 22, 23], [1, 1], [2, 3], [a["i"], a["j"]]),
    ),
    Case(
        ident="int_array_element_nd",
        why="offsets that start neither at one nor at the same number",
        decisions=(
            Decision("i", range(-1, 3)),
            Decision("j", range(-2, 2)),
            Decision("v", range(1, 5)),
        ),
        constraint="fzn_int_array_element_nd([1, 2, 3, 4], [0, -1], [2, 2], [i, j], v)",
        holds=lambda a: a["v"] == _nd_at([1, 2, 3, 4], [0, -1], [2, 2], [a["i"], a["j"]]),
    ),
    Case(
        ident="int_array_element_nd",
        why="one dimension numbered from three, with duplicate values",
        decisions=(Decision("i", range(2, 8)), Decision("v", range(7, 10))),
        constraint="fzn_int_array_element_nd([7, 8, 8, 9], [3], [4], [i], v)",
        holds=lambda a: a["v"] == _nd_at([7, 8, 8, 9], [3], [4], [a["i"]]),
    ),
    Case(
        ident="bool_array_element_nd",
        why="two dimensions, and a position one past the end of each",
        decisions=(
            Decision("i", range(0, 3)),
            Decision("j", range(0, 3)),
            Decision("v", [False, True], kind="bool"),
        ),
        constraint=(
            "fzn_bool_array_element_nd([true, false, false, true], [1, 1], [2, 2], [i, j], v)"
        ),
        holds=lambda a: a["v"]
        == _nd_at([True, False, False, True], [1, 1], [2, 2], [a["i"], a["j"]]),
    ),
    Case(
        ident="set_of_int_array_element_nd",
        why="two dimensions of sets, and a position past the slower dimension",
        decisions=(
            Decision("i", range(0, 3)),
            Decision("j", range(1, 3)),
            Decision("v", [set(), {1}, {2}, {1, 2}], kind="set of int"),
        ),
        constraint=(
            "fzn_set_of_int_array_element_nd([{}, {1}, {2}, {1, 2}], [1, 1], [2, 2], [i, j], v)"
        ),
        holds=lambda a: set(a["v"])
        == _nd_at([set(), {1}, {2}, {1, 2}], [1, 1], [2, 2], [a["i"], a["j"]]),
    ),
    Case(
        ident="set_of_int_array_element",
        why="duplicate sets, and a position one past each end",
        decisions=(
            Decision("i", range(0, 5)),
            Decision("v", [set(), {1}, {2}, {1, 2}], kind="set of int"),
        ),
        constraint="fzn_set_of_int_array_element([{1}, {1}, {1, 2}], 1, i, v)",
        holds=lambda a: set(a["v"]) == _at([{1}, {1}, {1, 2}], 1, a["i"]),
    ),
    Case(
        ident="set_of_int_array_element",
        why="an index set numbered from zero, over an array of decisions",
        decisions=(
            Decision("s1", [set(), {1}, {2}, {1, 2}], kind="set of int"),
            Decision("i", range(-1, 2)),
            Decision("v", [set(), {1}, {2}, {1, 2}], kind="set of int"),
        ),
        constraint="fzn_set_of_int_array_element([s1, {2}], 0, i, v)",
        holds=lambda a: set(a["v"]) == _at([set(a["s1"]), {2}], 0, a["i"]),
    ),
    Case(
        ident="int_if_then_else",
        why="no condition holds, which leaves the result free",
        decisions=(
            Decision("c1", [False, True], kind="bool"),
            Decision("c2", [False, True], kind="bool"),
            Decision("r", range(0, 4)),
        ),
        constraint="fzn_int_if_then_else([c1, c2], [1, 2], r)",
        holds=lambda a: _if_then_else([a["c1"], a["c2"]], [1, 2], a["r"]),
    ),
    Case(
        ident="int_if_then_else",
        why="the first of three conditions that holds wins",
        decisions=(
            Decision("c1", [False, True], kind="bool"),
            Decision("c2", [False, True], kind="bool"),
            Decision("c3", [False, True], kind="bool"),
            Decision("r", range(1, 4)),
        ),
        constraint="fzn_int_if_then_else([c1, c2, c3], [3, 1, 2], r)",
        holds=lambda a: _if_then_else([a["c1"], a["c2"], a["c3"]], [3, 1, 2], a["r"]),
    ),
    Case(
        ident="int_if_then_else",
        why="an else branch is a final condition fixed to true, over decision values",
        decisions=(
            Decision("c", [False, True], kind="bool"),
            Decision("v1", range(1, 3)),
            Decision("v2", range(1, 3)),
            Decision("r", range(1, 3)),
        ),
        constraint="fzn_int_if_then_else([c, true], [v1, v2], r)",
        holds=lambda a: _if_then_else([a["c"], True], [a["v1"], a["v2"]], a["r"]),
    ),
    Case(
        ident="int_if_then_else",
        why="an empty condition list constrains nothing",
        decisions=(Decision("r", range(1, 4)),),
        constraint="fzn_int_if_then_else(no_conditions, no_values, r)",
        holds=lambda a: _if_then_else([], [], a["r"]),
        params=(
            "array [1..0] of var bool: no_conditions = [];\n"
            "array [1..0] of var int: no_values = [];"
        ),
    ),
    Case(
        ident="bool_if_then_else",
        why="one condition, which either selects a decision value or leaves the result free",
        decisions=(
            Decision("c", [False, True], kind="bool"),
            Decision("v", [False, True], kind="bool"),
            Decision("r", [False, True], kind="bool"),
        ),
        constraint="fzn_bool_if_then_else([c], [v], r)",
        holds=lambda a: _if_then_else([a["c"]], [a["v"]], a["r"]),
    ),
    Case(
        ident="bool_if_then_else",
        why="the first of two conditions that holds wins",
        decisions=(
            Decision("c1", [False, True], kind="bool"),
            Decision("c2", [False, True], kind="bool"),
            Decision("r", [False, True], kind="bool"),
        ),
        constraint="fzn_bool_if_then_else([c1, c2], [true, false], r)",
        holds=lambda a: _if_then_else([a["c1"], a["c2"]], [True, False], a["r"]),
    ),
    Case(
        ident="set_of_int_if_then_else",
        why="an else branch, over sets",
        decisions=(
            Decision("c", [False, True], kind="bool"),
            Decision("r", [set(), {1}, {2}, {1, 2}], kind="set of int"),
        ),
        constraint="fzn_set_of_int_if_then_else([c, true], [{1}, {1, 2}], r)",
        holds=lambda a: _if_then_else([a["c"], True], [{1}, {1, 2}], set(a["r"])),
    ),
    Case(
        ident="set_of_int_if_then_else",
        why="no condition holds, which leaves the set free",
        decisions=(
            Decision("c", [False, True], kind="bool"),
            Decision("r", [set(), {1}], kind="set of int"),
        ),
        constraint="fzn_set_of_int_if_then_else([c], [{1}], r)",
        holds=lambda a: _if_then_else([a["c"]], [{1}], set(a["r"])),
    ),
    Case(
        ident="int_array_maximum",
        why="duplicates at the maximum, over an array of decisions",
        decisions=(
            Decision("x1", range(1, 4)),
            Decision("x2", range(1, 4)),
            Decision("m", range(0, 5)),
        ),
        constraint="fzn_int_array_maximum([x1, x2, x2], m)",
        holds=lambda a: a["m"] == max(a["x1"], a["x2"], a["x2"]),
    ),
    Case(
        ident="int_array_maximum",
        why="a value domain that cuts off the top of the array's range",
        decisions=(Decision("x", range(-2, 3)), Decision("m", range(-1, 2))),
        constraint="fzn_int_array_maximum([x, -1], m)",
        holds=lambda a: a["m"] == max(a["x"], -1),
    ),
    Case(
        ident="int_array_minimum",
        why="a one-element array",
        decisions=(Decision("x", range(1, 4)), Decision("m", range(0, 5))),
        constraint="fzn_int_array_minimum([x], m)",
        holds=lambda a: a["m"] == a["x"],
    ),
    Case(
        ident="int_array_minimum",
        why="a value domain that cuts off the bottom of the array's range",
        decisions=(Decision("x", range(-2, 3)), Decision("m", range(-1, 2))),
        constraint="fzn_int_array_minimum([x, 1], m)",
        holds=lambda a: a["m"] == min(a["x"], 1),
    ),
    Case(
        ident="int_array_arg_max",
        why="duplicates at the maximum resolve to the first, and a position one past each end",
        decisions=(Decision("i", range(0, 6)),),
        constraint="fzn_int_array_arg_max([3, 5, 5, 1], 1, i)",
        holds=lambda a: _arg_max([3, 5, 5, 1], 1, a["i"]),
    ),
    Case(
        ident="int_array_arg_max",
        why="an array of decisions numbered from two",
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            Decision("i", range(1, 5)),
        ),
        constraint="fzn_int_array_arg_max([x1, x2], 2, i)",
        holds=lambda a: _arg_max([a["x1"], a["x2"]], 2, a["i"]),
    ),
    Case(
        ident="int_array_arg_max",
        why="a one-element array numbered from five",
        decisions=(Decision("x", range(1, 3)), Decision("i", range(4, 7))),
        constraint="fzn_int_array_arg_max([x], 5, i)",
        holds=lambda a: _arg_max([a["x"]], 5, a["i"]),
    ),
    Case(
        ident="int_array_arg_min",
        why="duplicates at the minimum, in an index set numbered from minus one",
        decisions=(Decision("i", range(-2, 4)),),
        constraint="fzn_int_array_arg_min([2, -1, -1, 4], -1, i)",
        holds=lambda a: _arg_min([2, -1, -1, 4], -1, a["i"]),
    ),
    Case(
        ident="int_array_arg_min",
        why="an array of decisions, and a position one past each end",
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            Decision("i", range(0, 4)),
        ),
        constraint="fzn_int_array_arg_min([x1, x2], 1, i)",
        holds=lambda a: _arg_min([a["x1"], a["x2"]], 1, a["i"]),
    ),
    Case(
        ident="bool_array_arg_max",
        why="the first element that holds, and an array where none does, which gives the first position",
        decisions=(
            Decision("b1", [False, True], kind="bool"),
            Decision("b2", [False, True], kind="bool"),
            Decision("i", range(0, 4)),
        ),
        constraint="fzn_bool_array_arg_max([b1, b2], 1, i)",
        holds=lambda a: _arg_max([a["b1"], a["b2"]], 1, a["i"]),
    ),
    Case(
        ident="bool_array_arg_max",
        why="a one-element array numbered from zero",
        decisions=(Decision("b", [False, True], kind="bool"), Decision("i", range(-1, 2))),
        constraint="fzn_bool_array_arg_max([b], 0, i)",
        holds=lambda a: _arg_max([a["b"]], 0, a["i"]),
    ),
    Case(
        ident="bool_array_arg_min",
        why="the first element that does not hold, numbered from three",
        decisions=(
            Decision("b1", [False, True], kind="bool"),
            Decision("b2", [False, True], kind="bool"),
            Decision("i", range(2, 6)),
        ),
        constraint="fzn_bool_array_arg_min([b1, b2], 3, i)",
        holds=lambda a: _arg_min([a["b1"], a["b2"]], 3, a["i"]),
    ),
    Case(
        ident="bool_array_arg_min",
        why="several elements do not hold, and the first of them wins",
        decisions=(Decision("i", range(0, 5)),),
        constraint="fzn_bool_array_arg_min([true, false, false], 1, i)",
        holds=lambda a: _arg_min([True, False, False], 1, a["i"]),
    ),
    Case(
        ident="int_array_arg_val",
        why="the first of two positions holding the value, and values the array lacks",
        decisions=(Decision("v", range(4, 9)), Decision("i", range(0, 5))),
        constraint="fzn_int_array_arg_val([4, 7, 7], 1, v, i)",
        holds=lambda a: _arg_val([4, 7, 7], 1, a["v"], a["i"]),
    ),
    Case(
        ident="int_array_arg_val",
        why="an array of decisions numbered from minus two",
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            Decision("v", range(1, 3)),
            Decision("i", range(-3, 1)),
        ),
        constraint="fzn_int_array_arg_val([x1, x2], -2, v, i)",
        holds=lambda a: _arg_val([a["x1"], a["x2"]], -2, a["v"], a["i"]),
    ),
    Case(
        ident="bool_array_arg_val",
        why="the first position matching the value, for both polarities",
        decisions=(
            Decision("b1", [False, True], kind="bool"),
            Decision("b2", [False, True], kind="bool"),
            Decision("v", [False, True], kind="bool"),
            Decision("i", range(0, 4)),
        ),
        constraint="fzn_bool_array_arg_val([b1, b2], 1, v, i)",
        holds=lambda a: _arg_val([a["b1"], a["b2"]], 1, a["v"], a["i"]),
    ),
    Case(
        ident="bool_array_arg_val",
        why="duplicates in a fixed array numbered from two",
        decisions=(
            Decision("v", [False, True], kind="bool"),
            Decision("i", range(1, 6)),
        ),
        constraint="fzn_bool_array_arg_val([true, true, false], 2, v, i)",
        holds=lambda a: _arg_val([True, True, False], 2, a["v"], a["i"]),
    ),
    Case(
        ident="int_array_member",
        why="duplicate values, and values the array does not hold",
        decisions=(Decision("v", range(1, 7)),),
        constraint="fzn_int_array_member([2, 2, 5], v)",
        holds=lambda a: a["v"] in (2, 2, 5),
    ),
    Case(
        ident="int_array_member",
        why="an empty array holds nothing",
        decisions=(Decision("v", range(1, 4)),),
        constraint="fzn_int_array_member(empty, v)",
        holds=lambda a: a["v"] in (),
        params="array [1..0] of var int: empty = [];",
    ),
    Case(
        ident="int_array_member",
        why="a one-element array of decisions forces the value",
        decisions=(Decision("x", range(1, 3)), Decision("v", range(1, 4))),
        constraint="fzn_int_array_member([x], v)",
        holds=lambda a: a["v"] == a["x"],
    ),
    Case(
        ident="int_array_member_reif",
        why="both polarities of the reifying variable",
        decisions=(
            Decision("x", range(1, 3)),
            Decision("v", range(1, 4)),
            Decision("b", [False, True], kind="bool"),
        ),
        constraint="fzn_int_array_member_reif([x, 2], v, b)",
        holds=lambda a: a["b"] == (a["v"] in (a["x"], 2)),
    ),
    Case(
        ident="int_array_member",
        why="half-reified, so a false antecedent constrains nothing",
        # The registry names `int_array_member_imp`, but no library ships that
        # predicate, so the implication is written out and the compiler
        # half-reifies the call itself.
        decisions=(
            Decision("x", range(1, 3)),
            Decision("v", range(1, 4)),
            Decision("b", [False, True], kind="bool"),
        ),
        constraint="b -> fzn_int_array_member([x, 2], v)",
        holds=lambda a: not a["b"] or a["v"] in (a["x"], 2),
    ),
    Case(
        ident="bool_array_member",
        why="a one-element array forces the value",
        decisions=(
            Decision("b", [False, True], kind="bool"),
            Decision("v", [False, True], kind="bool"),
        ),
        constraint="fzn_bool_array_member([b], v)",
        holds=lambda a: a["v"] == a["b"],
    ),
    Case(
        ident="bool_array_member",
        why="an array of decisions, where both elements may answer the same way",
        decisions=(
            Decision("b1", [False, True], kind="bool"),
            Decision("b2", [False, True], kind="bool"),
            Decision("v", [False, True], kind="bool"),
        ),
        constraint="fzn_bool_array_member([b1, b2], v)",
        holds=lambda a: a["v"] in (a["b1"], a["b2"]),
    ),
    Case(
        ident="bool_array_member_reif",
        why="both polarities of the reifying variable",
        decisions=(
            Decision("b1", [False, True], kind="bool"),
            Decision("b2", [False, True], kind="bool"),
            Decision("v", [False, True], kind="bool"),
            Decision("r", [False, True], kind="bool"),
        ),
        constraint="fzn_bool_array_member_reif([b1, b2], v, r)",
        holds=lambda a: a["r"] == (a["v"] in (a["b1"], a["b2"])),
    ),
    Case(
        ident="bool_array_member",
        why="half-reified, so a false antecedent constrains nothing",
        decisions=(
            Decision("b", [False, True], kind="bool"),
            Decision("v", [False, True], kind="bool"),
            Decision("r", [False, True], kind="bool"),
        ),
        constraint="r -> fzn_bool_array_member([b], v)",
        holds=lambda a: not a["r"] or a["v"] == a["b"],
    ),
    Case(
        ident="set_of_int_array_member",
        why="duplicate sets, and sets the array does not hold",
        decisions=(Decision("v", [set(), {1}, {2}, {1, 2}], kind="set of int"),),
        constraint="fzn_set_of_int_array_member([{1}, {1}], v)",
        holds=lambda a: set(a["v"]) == {1},
    ),
    Case(
        ident="set_of_int_array_member",
        why="an array of decisions, where the value may match either element",
        decisions=(
            Decision("s1", [set(), {1}, {2}, {1, 2}], kind="set of int"),
            Decision("v", [set(), {1}, {2}, {1, 2}], kind="set of int"),
        ),
        constraint="fzn_set_of_int_array_member([s1, {2}], v)",
        holds=lambda a: set(a["v"]) in (set(a["s1"]), {2}),
    ),
    Case(
        ident="set_of_int_array_member_reif",
        why="both polarities of the reifying variable",
        decisions=(
            Decision("v", [set(), {1}, {2}, {1, 2}], kind="set of int"),
            Decision("r", [False, True], kind="bool"),
        ),
        constraint="fzn_set_of_int_array_member_reif([{1}, {1, 2}], v, r)",
        holds=lambda a: a["r"] == (set(a["v"]) in ({1}, {1, 2})),
    ),
    Case(
        ident="set_of_int_array_member",
        why="half-reified, so a false antecedent constrains nothing",
        decisions=(
            Decision("v", [set(), {1}, {2}, {1, 2}], kind="set of int"),
            Decision("r", [False, True], kind="bool"),
        ),
        constraint="r -> fzn_set_of_int_array_member([{1}], v)",
        holds=lambda a: not a["r"] or set(a["v"]) == {1},
    ),
    Case(
        ident="int_array_write",
        why="a position one past each end leaves the array unchanged",
        decisions=(
            Decision("i", range(0, 4)),
            Decision("v", range(1, 3)),
            Decision("r1", range(1, 3)),
            Decision("r2", range(1, 3)),
        ),
        constraint="fzn_int_array_write([1, 2], 1, i, v, [r1, r2])",
        holds=lambda a: [a["r1"], a["r2"]] == _write([1, 2], 1, a["i"], a["v"]),
    ),
    Case(
        ident="int_array_write",
        why="an index set numbered from three",
        decisions=(
            Decision("i", range(2, 6)),
            Decision("v", range(1, 3)),
            Decision("r1", range(1, 3)),
            Decision("r2", range(1, 3)),
        ),
        constraint="fzn_int_array_write([1, 2], 3, i, v, [r1, r2])",
        holds=lambda a: [a["r1"], a["r2"]] == _write([1, 2], 3, a["i"], a["v"]),
    ),
    Case(
        ident="int_array_write",
        why="a one-element array of decisions",
        decisions=(
            Decision("x", range(1, 3)),
            Decision("i", range(0, 3)),
            Decision("v", range(1, 3)),
            Decision("r1", range(1, 3)),
        ),
        constraint="fzn_int_array_write([x], 1, i, v, [r1])",
        holds=lambda a: [a["r1"]] == _write([a["x"]], 1, a["i"], a["v"]),
    ),
    Case(
        ident="int_array_writes",
        why="two writes to one position must agree",
        decisions=(
            Decision("i1", range(1, 3)),
            Decision("i2", range(1, 3)),
            Decision("r1", range(1, 3)),
            Decision("r2", range(1, 3)),
        ),
        constraint="fzn_int_array_writes([1, 2], 1, [i1, i2], [1, 2], [r1, r2])",
        holds=lambda a: _writes(
            [1, 2], 1, [a["i1"], a["i2"]], [1, 2], [a["r1"], a["r2"]]
        ),
    ),
    Case(
        ident="int_array_writes",
        why="a position one past each end has nowhere to write, numbered from zero",
        decisions=(
            Decision("i", range(-1, 3)),
            Decision("r1", range(1, 3)),
            Decision("r2", range(1, 3)),
        ),
        constraint="fzn_int_array_writes([1, 2], 0, [i], [2], [r1, r2])",
        holds=lambda a: _writes([1, 2], 0, [a["i"]], [2], [a["r1"], a["r2"]]),
    ),
    Case(
        ident="int_array_writes",
        why="no writes at all leaves the array unchanged",
        decisions=(Decision("r1", range(1, 3)), Decision("r2", range(1, 3))),
        constraint="fzn_int_array_writes([1, 2], 1, no_index, no_value, [r1, r2])",
        holds=lambda a: _writes([1, 2], 1, [], [], [a["r1"], a["r2"]]),
        params=(
            "array [1..0] of var int: no_index = [];\n"
            "array [1..0] of var int: no_value = [];"
        ),
    ),
    Case(
        ident="int_array_writes_reif",
        why="both polarities of the reifying variable",
        decisions=(
            Decision("i", range(1, 3)),
            Decision("r1", range(1, 3)),
            Decision("r2", range(1, 3)),
            Decision("b", [False, True], kind="bool"),
        ),
        constraint="fzn_int_array_writes_reif([1, 2], 1, [i], [2], [r1, r2], b)",
        holds=lambda a: a["b"]
        == _writes([1, 2], 1, [a["i"]], [2], [a["r1"], a["r2"]]),
    ),
    Case(
        ident="int_array_writes",
        why="half-reified, so a false antecedent constrains nothing",
        decisions=(
            Decision("i", range(1, 3)),
            Decision("r1", range(1, 3)),
            Decision("r2", range(1, 3)),
            Decision("b", [False, True], kind="bool"),
        ),
        constraint="b -> fzn_int_array_writes([1, 2], 1, [i], [2], [r1, r2])",
        holds=lambda a: not a["b"]
        or _writes([1, 2], 1, [a["i"]], [2], [a["r1"], a["r2"]]),
    ),
    Case(
        ident="int_array_writes_sequential",
        why="the later of two writes to one position wins",
        decisions=(
            Decision("i1", range(1, 3)),
            Decision("i2", range(1, 3)),
            Decision("r1", range(1, 3)),
            Decision("r2", range(1, 3)),
        ),
        constraint="fzn_int_array_writes_sequential([1, 2], 1, [i1, i2], [1, 2], [r1, r2])",
        holds=lambda a: [a["r1"], a["r2"]]
        == _writes_sequential([1, 2], 1, [a["i1"], a["i2"]], [1, 2]),
    ),
    Case(
        ident="int_array_writes_sequential",
        why="a position one past each end is ignored, numbered from two",
        decisions=(
            Decision("i", range(1, 5)),
            Decision("r1", range(1, 3)),
            Decision("r2", range(1, 3)),
        ),
        constraint="fzn_int_array_writes_sequential([1, 2], 2, [i], [1], [r1, r2])",
        holds=lambda a: [a["r1"], a["r2"]]
        == _writes_sequential([1, 2], 2, [a["i"]], [1]),
    ),
    Case(
        ident="int_array_sort",
        why="duplicates, over an array of decisions",
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            Decision("y1", range(1, 3)),
            Decision("y2", range(1, 3)),
        ),
        constraint="fzn_int_array_sort([x1, x2], [y1, y2])",
        holds=lambda a: [a["y1"], a["y2"]] == sorted([a["x1"], a["x2"]]),
    ),
    Case(
        ident="int_array_sort",
        why="a one-element array",
        decisions=(Decision("x", range(1, 4)), Decision("y", range(1, 4))),
        constraint="fzn_int_array_sort([x], [y])",
        holds=lambda a: a["y"] == a["x"],
    ),
    Case(
        ident="int_array_sort",
        why="every copy of a duplicated value has to survive",
        decisions=(
            Decision("y1", range(1, 3)),
            Decision("y2", range(1, 3)),
            Decision("y3", range(1, 3)),
        ),
        constraint="fzn_int_array_sort([2, 1, 2], [y1, y2, y3])",
        holds=lambda a: [a["y1"], a["y2"], a["y3"]] == sorted([2, 1, 2]),
    ),
    Case(
        ident="int_array_arg_sort",
        why="duplicate values resolve to the lower original position",
        decisions=(
            Decision("p1", range(1, 4)),
            Decision("p2", range(1, 4)),
            Decision("p3", range(1, 4)),
        ),
        constraint="fzn_int_array_arg_sort([2, 1, 2], 1, [p1, p2, p3])",
        holds=lambda a: _arg_sort([2, 1, 2], 1, [a["p1"], a["p2"], a["p3"]]),
    ),
    Case(
        ident="int_array_arg_sort",
        why="positions numbered from zero, and one past the end",
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            Decision("p1", range(0, 3)),
            Decision("p2", range(0, 3)),
        ),
        constraint="fzn_int_array_arg_sort([x1, x2], 0, [p1, p2])",
        holds=lambda a: _arg_sort([a["x1"], a["x2"]], 0, [a["p1"], a["p2"]]),
    ),
    Case(
        ident="int_array_arg_sort",
        why="a one-element array numbered from four",
        decisions=(Decision("x", range(1, 3)), Decision("p1", range(3, 6))),
        constraint="fzn_int_array_arg_sort([x], 4, [p1])",
        holds=lambda a: _arg_sort([a["x"]], 4, [a["p1"]]),
    ),
)
