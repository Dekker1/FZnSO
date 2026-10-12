"""Arithmetic constraints.

Each case drives the edge of the constraint rather than its middle: a bound
that is exactly met, a coefficient of zero, a negative dividend, an empty
array. The Python predicate beside it is the authority.

A reified form is called by its own name, `fzn_<ident>_reif`. A half-reified
one cannot be: `fznso_constraints/` deliberately holds no `fzn_<ident>_imp.mzn`,
so the name exists only for a solver that declares it. Those cases ask for the
implication instead, `r -> fzn_<ident>(..)`, which the compiler lowers onto
`<ident>_imp` where the solver has it and onto `<ident>_reif` where it does not.
Either way a false `r` must leave the constraint unconstrained.

The float constraints are absent. A `var float` decision stands for an
interval, not for the finite set of values the harness enumerates, and Huub
declares no float decision at all.
"""

from __future__ import annotations

from fznso_conform.case import Case, Decision

#: The domain of a Boolean decision, in the order the harness enumerates it.
BOOL = (False, True)

CASES = (
    Case(
        ident="int_lin_le",
        why="the bound is met exactly, and a coefficient is negative",
        decisions=(Decision("x", range(0, 4)), Decision("y", range(0, 4))),
        constraint="fzn_int_lin_le([2, -1], [x, y], 3)",
        holds=lambda a: 2 * a["x"] - a["y"] <= 3,
    ),
    Case(
        ident="int_lin_le",
        why="a zero coefficient leaves its decision free",
        decisions=(Decision("x", range(0, 3)), Decision("y", range(0, 3))),
        constraint="fzn_int_lin_le([0, 1], [x, y], 1)",
        holds=lambda a: a["y"] <= 1,
    ),
    Case(
        ident="int_lin_eq",
        why="the only solutions are at the ends of both domains",
        decisions=(Decision("x", range(-2, 3)), Decision("y", range(-2, 3))),
        constraint="fzn_int_lin_eq([1, 1], [x, y], -4)",
        holds=lambda a: a["x"] + a["y"] == -4,
    ),
    Case(
        ident="int_lin_ne",
        why="every assignment but one is a solution",
        decisions=(Decision("x", range(0, 3)), Decision("y", range(0, 3))),
        constraint="fzn_int_lin_ne([1, 1], [x, y], 2)",
        holds=lambda a: a["x"] + a["y"] != 2,
    ),
    Case(
        ident="int_abs",
        why="zero and both signs, including the asymmetric end of the domain",
        decisions=(Decision("x", range(-3, 3)), Decision("y", range(0, 4))),
        constraint="fzn_int_abs(x, y)",
        holds=lambda a: abs(a["x"]) == a["y"],
    ),
    Case(
        ident="int_times",
        why="a zero factor, and a product that overflows the product's domain",
        decisions=(
            Decision("x", range(-2, 3)),
            Decision("y", range(-2, 3)),
            Decision("z", range(-4, 5)),
        ),
        constraint="fzn_int_times(x, y, z)",
        holds=lambda a: a["x"] * a["y"] == a["z"],
    ),
    Case(
        ident="int_div",
        why="truncation towards zero, for all four sign combinations",
        decisions=(
            Decision("x", [-7, -6, 6, 7]),
            Decision("y", [-2, 2, 3]),
            Decision("z", range(-4, 5)),
        ),
        constraint="fzn_int_div(x, y, z)",
        holds=lambda a: a["y"] != 0 and int(a["x"] / a["y"]) == a["z"],
    ),
    Case(
        ident="int_mod",
        why="the remainder takes the sign of the dividend",
        decisions=(
            Decision("x", [-7, -6, 6, 7]),
            Decision("y", [-3, -2, 2, 3]),
            Decision("z", range(-3, 4)),
        ),
        constraint="fzn_int_mod(x, y, z)",
        holds=lambda a: a["y"] != 0 and a["x"] - int(a["x"] / a["y"]) * a["y"] == a["z"],
    ),
    Case(
        ident="int_lin_eq_reif",
        why="a false r must force the two apart, a true one together",
        decisions=(
            Decision("x", range(-1, 2)),
            Decision("y", range(-1, 2)),
            Decision("r", BOOL, "bool"),
        ),
        constraint="fzn_int_lin_eq_reif([1, -1], [x, y], 0, r)",
        holds=lambda a: (a["x"] - a["y"] == 0) == a["r"],
    ),
    Case(
        ident="int_lin_eq",
        why="the implied form: a false antecedent leaves the sum free",
        decisions=(
            Decision("x", range(0, 3)),
            Decision("y", range(0, 3)),
            Decision("r", BOOL, "bool"),
        ),
        constraint="r -> fzn_int_lin_eq([1, 1], [x, y], 2)",
        holds=lambda a: not a["r"] or a["x"] + a["y"] == 2,
    ),
    Case(
        ident="int_lin_le_reif",
        why="r holds where the bound is met exactly and fails where it is missed by one",
        decisions=(
            Decision("x", range(0, 3)),
            Decision("y", range(0, 3)),
            Decision("r", BOOL, "bool"),
        ),
        constraint="fzn_int_lin_le_reif([2, 1], [x, y], 2, r)",
        holds=lambda a: (2 * a["x"] + a["y"] <= 2) == a["r"],
    ),
    Case(
        ident="int_lin_le",
        why="the implied form: a false antecedent leaves a one-element sum unbounded",
        decisions=(Decision("x", range(0, 4)), Decision("r", BOOL, "bool")),
        constraint="r -> fzn_int_lin_le([1], [x], 1)",
        holds=lambda a: not a["r"] or a["x"] <= 1,
    ),
    Case(
        ident="int_lin_ne_reif",
        why="a zero coefficient, and a false r forces the sum onto the total",
        decisions=(
            Decision("x", range(0, 3)),
            Decision("y", range(0, 3)),
            Decision("r", BOOL, "bool"),
        ),
        constraint="fzn_int_lin_ne_reif([1, 0], [x, y], 1, r)",
        holds=lambda a: (a["x"] != 1) == a["r"],
    ),
    Case(
        ident="int_lin_ne",
        why="the implied form: a false antecedent allows the forbidden total",
        decisions=(Decision("x", range(0, 3)), Decision("r", BOOL, "bool")),
        constraint="r -> fzn_int_lin_ne([1], [x], 1)",
        holds=lambda a: not a["r"] or a["x"] != 1,
    ),
    # The exponent is a parameter, not a decision: Gecode declares `int_pow`
    # only for a fixed one, and a decision there reaches the body, which aborts.
    Case(
        ident="int_pow",
        why="a zero exponent is one, even for a base of zero",
        decisions=(Decision("a", [-2, 0, 2]), Decision("c", range(0, 3))),
        constraint="fzn_int_pow(a, 0, c)",
        holds=lambda a: a["c"] == 1,
    ),
    Case(
        ident="int_pow",
        why="an odd exponent keeps the sign of the base",
        decisions=(Decision("a", range(-2, 3)), Decision("c", [-8, -1, 0, 1, 8])),
        constraint="fzn_int_pow(a, 3, c)",
        holds=lambda a: a["a"] ** 3 == a["c"],
    ),
    Case(
        ident="int_pow",
        why="a negative exponent is the reciprocal, truncated towards zero",
        decisions=(Decision("a", [-2, -1, 1, 2]), Decision("c", range(-1, 2))),
        constraint="fzn_int_pow(a, -1, c)",
        holds=lambda a: int(1 / a["a"]) == a["c"],
    ),
    Case(
        ident="bool_clause",
        why="the implication form, one literal in each list",
        decisions=(Decision("a", BOOL, "bool"), Decision("b", BOOL, "bool")),
        constraint="fzn_bool_clause([b], [a])",
        holds=lambda a: a["b"] or not a["a"],
    ),
    Case(
        ident="bool_clause",
        why="an empty positive list leaves only the negated literals",
        decisions=(Decision("a", BOOL, "bool"), Decision("b", BOOL, "bool")),
        constraint="fzn_bool_clause([], [a, b])",
        holds=lambda a: not a["a"] or not a["b"],
    ),
    Case(
        ident="bool_clause_reif",
        why="a false r must fail every literal in the clause",
        decisions=(
            Decision("a", BOOL, "bool"),
            Decision("b", BOOL, "bool"),
            Decision("r", BOOL, "bool"),
        ),
        constraint="fzn_bool_clause_reif([a], [b], r)",
        holds=lambda a: (a["a"] or not a["b"]) == a["r"],
    ),
    Case(
        ident="bool_clause",
        why="the implied form: a false antecedent leaves the clause free",
        decisions=(
            Decision("a", BOOL, "bool"),
            Decision("b", BOOL, "bool"),
            Decision("r", BOOL, "bool"),
        ),
        constraint="r -> fzn_bool_clause([a], [b])",
        holds=lambda a: not a["r"] or a["a"] or not a["b"],
    ),
    Case(
        ident="bool_array_and",
        why="an empty array conjoins to true",
        decisions=(Decision("r", BOOL, "bool"),),
        constraint="fzn_bool_array_and([], r)",
        holds=lambda a: a["r"],
    ),
    Case(
        ident="bool_array_and",
        why="a single element, which the result repeats",
        decisions=(Decision("a", BOOL, "bool"), Decision("r", BOOL, "bool")),
        constraint="fzn_bool_array_and([a], r)",
        holds=lambda a: a["r"] == a["a"],
    ),
    Case(
        ident="bool_array_and",
        why="a false result needs only one element to fail",
        decisions=(
            Decision("a", BOOL, "bool"),
            Decision("b", BOOL, "bool"),
            Decision("r", BOOL, "bool"),
        ),
        constraint="fzn_bool_array_and([a, b], r)",
        holds=lambda a: a["r"] == (a["a"] and a["b"]),
    ),
    Case(
        ident="bool_array_xor",
        why="an empty array has no element holding, so nothing is a solution",
        decisions=(Decision("a", BOOL, "bool"),),
        constraint="fzn_bool_array_xor([])",
        holds=lambda a: False,
    ),
    Case(
        ident="bool_array_xor",
        why="a single element must hold, and one outside the array stays free",
        decisions=(Decision("a", BOOL, "bool"), Decision("b", BOOL, "bool")),
        constraint="fzn_bool_array_xor([a])",
        holds=lambda a: a["a"],
    ),
    Case(
        ident="bool_array_xor",
        why="three elements, of which an odd number hold",
        decisions=(
            Decision("a", BOOL, "bool"),
            Decision("b", BOOL, "bool"),
            Decision("c", BOOL, "bool"),
        ),
        constraint="fzn_bool_array_xor([a, b, c])",
        holds=lambda a: (a["a"] + a["b"] + a["c"]) % 2 == 1,
    ),
    Case(
        ident="bool_array_xor_reif",
        why="a false r must leave an even number holding",
        decisions=(
            Decision("a", BOOL, "bool"),
            Decision("b", BOOL, "bool"),
            Decision("r", BOOL, "bool"),
        ),
        constraint="fzn_bool_array_xor_reif([a, b], r)",
        holds=lambda a: (a["a"] != a["b"]) == a["r"],
    ),
    Case(
        ident="bool_array_xor",
        why="the implied form: a false antecedent allows an even number to hold",
        decisions=(
            Decision("a", BOOL, "bool"),
            Decision("b", BOOL, "bool"),
            Decision("r", BOOL, "bool"),
        ),
        constraint="r -> fzn_bool_array_xor([a, b])",
        holds=lambda a: not a["r"] or a["a"] != a["b"],
    ),
    Case(
        ident="bool_lin_eq",
        why="a zero coefficient beside a negative one",
        decisions=(
            Decision("a", BOOL, "bool"),
            Decision("b", BOOL, "bool"),
            Decision("t", range(-1, 2)),
        ),
        constraint="fzn_bool_lin_eq([0, -1], [a, b], t)",
        holds=lambda a: a["t"] == -int(a["b"]),
    ),
    Case(
        ident="bool_lin_eq",
        why="an empty array weighs nothing, so the total is zero",
        decisions=(Decision("t", range(-1, 2)),),
        constraint="fzn_bool_lin_eq([], [], t)",
        holds=lambda a: a["t"] == 0,
    ),
    Case(
        ident="bool_lin_le",
        why="the bound is met exactly only once the negative coefficient counts",
        decisions=(Decision("a", BOOL, "bool"), Decision("b", BOOL, "bool")),
        constraint="fzn_bool_lin_le([2, -1], [a, b], 1)",
        holds=lambda a: 2 * int(a["a"]) - int(a["b"]) <= 1,
    ),
    Case(
        ident="bool_lin_le",
        why="a bound of zero, which a zero coefficient leaves its Boolean free under",
        decisions=(Decision("a", BOOL, "bool"), Decision("b", BOOL, "bool")),
        constraint="fzn_bool_lin_le([0, 1], [a, b], 0)",
        holds=lambda a: not a["b"],
    ),
    Case(
        ident="bool_lin_le_reif",
        why="a false r must force both Booleans to hold",
        decisions=(
            Decision("a", BOOL, "bool"),
            Decision("b", BOOL, "bool"),
            Decision("r", BOOL, "bool"),
        ),
        constraint="fzn_bool_lin_le_reif([1, 1], [a, b], 1, r)",
        holds=lambda a: (int(a["a"]) + int(a["b"]) <= 1) == a["r"],
    ),
    Case(
        ident="bool_lin_le",
        why="the implied form: a false antecedent lets the bound be exceeded",
        decisions=(
            Decision("a", BOOL, "bool"),
            Decision("b", BOOL, "bool"),
            Decision("r", BOOL, "bool"),
        ),
        constraint="r -> fzn_bool_lin_le([1, 1], [a, b], 1)",
        holds=lambda a: not a["r"] or int(a["a"]) + int(a["b"]) <= 1,
    ),
    Case(
        ident="bool_lin_ne",
        why="the total is a decision, so each count rules out one of its values",
        decisions=(
            Decision("a", BOOL, "bool"),
            Decision("b", BOOL, "bool"),
            Decision("t", range(0, 3)),
        ),
        constraint="fzn_bool_lin_ne([1, 1], [a, b], t)",
        holds=lambda a: int(a["a"]) + int(a["b"]) != a["t"],
    ),
    Case(
        ident="bool_lin_ne_reif",
        why="a negative coefficient, and a false r forces the count onto the total",
        decisions=(
            Decision("a", BOOL, "bool"),
            Decision("b", BOOL, "bool"),
            Decision("t", range(0, 2)),
            Decision("r", BOOL, "bool"),
        ),
        constraint="fzn_bool_lin_ne_reif([1, -1], [a, b], t, r)",
        holds=lambda a: (int(a["a"]) - int(a["b"]) != a["t"]) == a["r"],
    ),
    Case(
        ident="bool_lin_ne",
        why="the implied form: a false antecedent allows the forbidden total",
        decisions=(
            Decision("a", BOOL, "bool"),
            Decision("b", BOOL, "bool"),
            Decision("r", BOOL, "bool"),
        ),
        constraint="r -> fzn_bool_lin_ne([1, 1], [a, b], 2)",
        holds=lambda a: not a["r"] or int(a["a"]) + int(a["b"]) != 2,
    ),
    Case(
        ident="bool_to_int",
        why="zero and one, and no other value the integer's domain offers",
        decisions=(Decision("a", BOOL, "bool"), Decision("b", range(-1, 3))),
        constraint="fzn_bool_to_int(a, b)",
        holds=lambda a: a["b"] == int(a["a"]),
    ),
)
