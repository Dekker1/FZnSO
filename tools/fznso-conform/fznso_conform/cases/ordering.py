"""Ordering and counting constraints.

Each case drives an edge rather than the middle: a sequence that ties where the
strict form forbids it, a count sitting exactly on its bound, a value that never
occurs and one that occurs everywhere, an array of one element, an empty array,
an offset that is not one. The Python predicate beside each case is the
authority, written from the registry definition.

A reified case leaves the reifying variable free, so the harness enumerates both
`b = false`, where the constraint must not restrict anything, and `b = true`,
where it must. A half-reification shows up as the whole `b = false` half going
missing.

The float constraints are driven with integer decisions, which MiniZinc coerces
at the call. A `var float` has no finite domain to enumerate, and integral
values compare exactly.
"""

from __future__ import annotations

import itertools

from fznso_conform.case import Case, Decision


def arr(a, n, prefix="x"):
    """The decisions named `<prefix>1` .. `<prefix>n`, as an array."""
    return [a[f"{prefix}{i}"] for i in range(1, n + 1)]


def subsets(universe):
    """Every subset of a contiguous universe.

    This is the whole domain a `var set of lo..hi` takes, and the harness
    enumerates only the values a decision lists, so a set decision has to list
    all of them or the solver will rightly find more.
    """
    return tuple(
        frozenset(c)
        for k in range(len(universe) + 1)
        for c in itertools.combinations(universe, k)
    )


def rank(s):
    """A set as its sorted elements: the standard total order on sets.

    Comparing these tuples is the order the MiniZinc specification gives, a
    lexicographic order on the sorted set form, so a proper prefix is below its
    extensions: `powerset(1..2)` runs `{} < {1} < {1,2} < {2}`, and the spec's
    own example is `{} < {1,3} < {2}`.
    """
    return tuple(sorted(s))


def windows(xs, w):
    """Every window of `w` consecutive elements that fits inside the array."""
    return [xs[k : k + w] for k in range(len(xs) - w + 1)]


def cols(flat, columns):
    """The columns of a matrix passed flat, row-major, `columns` per row."""
    return [tuple(flat[j::columns]) for j in range(columns)]


def chain_lesseq(flat, columns, strict):
    order = cols(flat, columns)
    return all(
        (b > a if strict else b >= a) for a, b in zip(order, order[1:])
    )


def involution(xs, offset):
    """Whether xs is a permutation of its index range that is its own inverse."""
    indices = range(offset, offset + len(xs))
    return all(v in indices for v in xs) and all(
        xs[xs[i - offset] - offset] == i for i in indices
    )


def lex_least_under(xs, perms):
    """Whether xs is no greater than any of the listed permutations of itself.

    `perms` is flattened row-major, one permutation per row, rows as long as xs.
    """
    n = len(xs)
    rows = [perms[k : k + n] for k in range(0, len(perms), n)]
    return all(tuple(xs) <= tuple(xs[p - 1] for p in row) for row in rows)


def square_images(flat, columns):
    """Every row and column permutation of a square matrix, and of its transpose."""
    c = columns
    rows = [flat[i * c : (i + 1) * c] for i in range(c)]
    transpose = [[rows[i][j] for i in range(c)] for j in range(c)]
    return [
        tuple(m[i][j] for i in rp for j in cp)
        for m in (rows, transpose)
        for rp in itertools.permutations(range(c))
        for cp in itertools.permutations(range(c))
    ]


def precede(xs, s, t):
    return all(
        any(xs[i] == s for i in range(j)) for j in range(len(xs)) if xs[j] == t
    )


def set_precede(xs, s, t):
    return all(
        any(s in xs[i] and t not in xs[i] for i in range(j))
        for j in range(len(xs))
        if t in xs[j] and s not in xs[j]
    )


def inverse_both(f, f_offset, invf, invf_offset):
    """Whether f[i] = j is the same statement as invf[j] = i, over both ranges."""
    fi = range(f_offset, f_offset + len(f))
    ii = range(invf_offset, invf_offset + len(invf))
    return all(
        (f[i - f_offset] == j) == (invf[j - invf_offset] == i) for i in fi for j in ii
    )


def inverse_in_range(f, f_offset, invf, invf_offset):
    """The one-sided form: each array inverts the other wherever the index fits."""
    fi = range(f_offset, f_offset + len(f))
    ii = range(invf_offset, invf_offset + len(invf))
    return all(
        invf[f[i - f_offset] - invf_offset] == i for i in fi if f[i - f_offset] in ii
    ) and all(
        f[invf[j - invf_offset] - f_offset] == j
        for j in ii
        if invf[j - invf_offset] in fi
    )


def set_inverse(f, f_offset, invf, invf_offset):
    fi = range(f_offset, f_offset + len(f))
    ii = range(invf_offset, invf_offset + len(invf))
    return all(
        (j in f[i - f_offset]) == (i in invf[j - invf_offset]) for i in fi for j in ii
    )


#: An empty array holds no decision, so it arrives as a parameter.
EMPTY_INT = "array [1..0] of var int: none;\n"

BOOLS = [False, True]

CASES = (
    Case(
        ident="int_all_different",
        why="the domain holds exactly as many values as the array",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("x3", range(0, 3)),
        ),
        constraint="fzn_int_all_different([x1, x2, x3])",
        holds=lambda a: len(set(arr(a, 3))) == 3,
    ),

    Case(
        ident="int_all_different",
        why="an array of one element, which has no pair to differ",
        decisions=(Decision("x1", range(0, 3)),),
        constraint="fzn_int_all_different([x1])",
        holds=lambda a: True,
    ),

    Case(
        ident="int_all_different_reif",
        why="reified, so the false half must leave the array free",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_int_all_different_reif([x1, x2], b)",
        holds=lambda a: a["b"] == (a["x1"] != a["x2"]),
    ),

    Case(
        ident="int_all_different_except",
        why="only the excepted value may repeat",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("x3", range(0, 3)),
        ),
        params="set of int: ex = {0};\n",
        constraint="fzn_int_all_different_except([x1, x2, x3], ex)",
        holds=lambda a: all(
            x != y or x == 0 for x, y in itertools.combinations(arr(a, 3), 2)
        ),
    ),

    Case(
        ident="int_all_different_except",
        why="an empty exception set is plain all-different",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("x3", range(0, 3)),
        ),
        params="set of int: ex = {};\n",
        constraint="fzn_int_all_different_except([x1, x2, x3], ex)",
        holds=lambda a: len(set(arr(a, 3))) == 3,
    ),

    Case(
        ident="int_all_different_except_reif",
        why="reified, with the excepted value the only repeat allowed",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="set of int: ex = {0};\n",
        constraint="fzn_int_all_different_except_reif([x1, x2], ex, b)",
        holds=lambda a: a["b"] == (a["x1"] != a["x2"] or a["x1"] == 0),
    ),

    Case(
        ident="int_all_different_symmetric",
        why="a three-cycle is a permutation that is not its own inverse",
        decisions=(
            Decision("x1", range(2, 5)),
            Decision("x2", range(2, 5)),
            Decision("x3", range(2, 5)),
        ),
        params="int: off = 2;\n",
        constraint="fzn_int_all_different_symmetric([x1, x2, x3], off)",
        holds=lambda a: involution(arr(a, 3), 2),
    ),

    Case(
        ident="int_all_different_symmetric_reif",
        why="reified, over the two involutions of a pair",
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: off = 1;\n",
        constraint="fzn_int_all_different_symmetric_reif([x1, x2], off, b)",
        holds=lambda a: a["b"] == involution(arr(a, 2), 1),
    ),

    Case(
        ident="set_of_int_all_different",
        why="three sets over a two-element universe, where the empty set counts",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("s3", subsets([1, 2]), kind="set of int"),
        ),
        constraint="fzn_set_of_int_all_different([s1, s2, s3])",
        holds=lambda a: len({frozenset(s) for s in arr(a, 3, "s")}) == 3,
    ),

    Case(
        ident="set_of_int_all_different_reif",
        why="reified over two sets, which tie on one assignment in four",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_set_of_int_all_different_reif([s1, s2], b)",
        holds=lambda a: a["b"] == (frozenset(a["s1"]) != frozenset(a["s2"])),
    ),

    Case(
        ident="int_all_equal",
        why="one position out of step breaks it",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
        ),
        constraint="fzn_int_all_equal([x1, x2, x3])",
        holds=lambda a: len(set(arr(a, 3))) == 1,
    ),

    Case(
        ident="int_all_equal",
        why="an array of one element is always equal to itself",
        decisions=(Decision("x1", range(0, 3)),),
        constraint="fzn_int_all_equal([x1])",
        holds=lambda a: True,
    ),

    Case(
        ident="int_all_equal_reif",
        why="reified, so the false half must demand a difference",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_int_all_equal_reif([x1, x2], b)",
        holds=lambda a: a["b"] == (a["x1"] == a["x2"]),
    ),

    Case(
        ident="set_of_int_all_equal",
        why="sets equal as sets, the empty set included",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
        ),
        constraint="fzn_set_of_int_all_equal([s1, s2])",
        holds=lambda a: frozenset(a["s1"]) == frozenset(a["s2"]),
    ),

    Case(
        ident="set_of_int_all_equal_reif",
        why="reified over two sets",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_set_of_int_all_equal_reif([s1, s2], b)",
        holds=lambda a: a["b"] == (frozenset(a["s1"]) == frozenset(a["s2"])),
    ),

    Case(
        ident="set_of_int_all_disjoint",
        why="three sets over two values, so a shared member is easy to reach",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("s3", subsets([1, 2]), kind="set of int"),
        ),
        constraint="fzn_set_of_int_all_disjoint([s1, s2, s3])",
        holds=lambda a: all(
            not (s & t) for s, t in itertools.combinations(arr(a, 3, "s"), 2)
        ),
    ),

    Case(
        ident="set_of_int_all_disjoint_reif",
        why="reified, where two empty sets are disjoint",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_set_of_int_all_disjoint_reif([s1, s2], b)",
        holds=lambda a: a["b"] == (not (set(a["s1"]) & set(a["s2"]))),
    ),

    Case(
        ident="int_nvalue",
        why="one distinct value is all-equal, three is all-different",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("x3", range(0, 3)),
            Decision("n", range(0, 5)),
        ),
        constraint="fzn_int_nvalue([x1, x2, x3], n)",
        holds=lambda a: a["n"] == len(set(arr(a, 3))),
    ),

    Case(
        ident="int_nvalue",
        why="an empty array takes no values at all",
        decisions=(Decision("n", range(0, 3)),),
        params=EMPTY_INT,
        constraint="fzn_int_nvalue(none, n)",
        holds=lambda a: a["n"] == 0,
    ),

    Case(
        ident="int_count",
        why="the counted value may be absent, or fill the array",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
            Decision("v", range(0, 3)),
            Decision("n", range(0, 4)),
        ),
        constraint="fzn_int_count([x1, x2, x3], v, n)",
        holds=lambda a: a["n"] == sum(x == a["v"] for x in arr(a, 3)),
    ),

    Case(
        ident="int_count",
        why="nothing occurs in an empty array",
        decisions=(Decision("v", range(0, 2)), Decision("n", range(0, 3))),
        params=EMPTY_INT,
        constraint="fzn_int_count(none, v, n)",
        holds=lambda a: a["n"] == 0,
    ),

    Case(
        ident="bool_count",
        why="counting false counts the complement",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("x3", BOOLS, kind="bool"),
            Decision("v", BOOLS, kind="bool"),
            Decision("n", range(0, 4)),
        ),
        constraint="fzn_bool_count([x1, x2, x3], v, n)",
        holds=lambda a: a["n"] == sum(x == a["v"] for x in arr(a, 3)),
    ),

    Case(
        ident="set_of_int_count",
        why="the counted value is a set, and may be the empty set",
        decisions=(
            Decision("s1", subsets([1]), kind="set of int"),
            Decision("s2", subsets([1]), kind="set of int"),
            Decision("v", subsets([1]), kind="set of int"),
            Decision("n", range(0, 3)),
        ),
        constraint="fzn_set_of_int_count([s1, s2], v, n)",
        holds=lambda a: a["n"]
        == sum(frozenset(s) == frozenset(a["v"]) for s in arr(a, 2, "s")),
    ),

    Case(
        ident="int_among",
        why="membership of a set, counted where no value, some, or all belong",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("x3", range(0, 3)),
            Decision("n", range(0, 4)),
        ),
        params="set of int: vals = {1, 2};\n",
        constraint="fzn_int_among([x1, x2, x3], vals, n)",
        holds=lambda a: a["n"] == sum(x in (1, 2) for x in arr(a, 3)),
    ),

    Case(
        ident="int_among",
        why="a value set the array cannot reach counts zero",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("n", range(0, 3)),
        ),
        params="set of int: vals = {5};\n",
        constraint="fzn_int_among([x1, x2], vals, n)",
        holds=lambda a: a["n"] == 0,
    ),

    Case(
        ident="int_sliding_among",
        why="every window of two holds exactly one of the values",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
        ),
        params="set of int: vals = {1};\nint: w = 2;\n",
        constraint="fzn_int_sliding_among([x1, x2, x3], vals, w, 1, 1)",
        holds=lambda a: all(
            sum(x == 1 for x in win) == 1 for win in windows(arr(a, 3), 2)
        ),
    ),

    Case(
        ident="int_sliding_among",
        why="the window spans the whole array, so there is one window",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
        ),
        params="set of int: vals = {1};\nint: w = 3;\n",
        constraint="fzn_int_sliding_among([x1, x2, x3], vals, w, 0, 1)",
        holds=lambda a: all(
            0 <= sum(x == 1 for x in win) <= 1 for win in windows(arr(a, 3), 3)
        ),
    ),

    Case(
        ident="int_sliding_among_reif",
        why="reified, with both bounds at the same value",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="set of int: vals = {1};\nint: w = 2;\n",
        constraint="fzn_int_sliding_among_reif([x1, x2, x3], vals, w, 1, 1, b)",
        holds=lambda a: a["b"]
        == all(sum(x == 1 for x in win) == 1 for win in windows(arr(a, 3), 2)),
    ),

    Case(
        ident="bool_sliding_among",
        why="the value counted is itself a decision, so both polarities run",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("x3", BOOLS, kind="bool"),
            Decision("v", BOOLS, kind="bool"),
        ),
        params="int: w = 2;\n",
        constraint="fzn_bool_sliding_among([x1, x2, x3], v, w, 0, 1)",
        holds=lambda a: all(
            sum(x == a["v"] for x in win) <= 1 for win in windows(arr(a, 3), 2)
        ),
    ),

    Case(
        ident="bool_sliding_among_reif",
        why="reified, with the count pinned to exactly one per window",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("x3", BOOLS, kind="bool"),
            Decision("v", BOOLS, kind="bool"),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: w = 2;\n",
        constraint="fzn_bool_sliding_among_reif([x1, x2, x3], v, w, 1, 1, b)",
        holds=lambda a: a["b"]
        == all(sum(x == a["v"] for x in win) == 1 for win in windows(arr(a, 3), 2)),
    ),

    Case(
        ident="int_sliding_sum",
        why="every window of two sums to exactly the bound",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("x3", range(0, 3)),
        ),
        params="int: w = 2;\n",
        constraint="fzn_int_sliding_sum([x1, x2, x3], w, 2, 2)",
        holds=lambda a: all(sum(win) == 2 for win in windows(arr(a, 3), 2)),
    ),

    Case(
        ident="int_sliding_sum",
        why="a window of one bounds each element on its own",
        decisions=(
            Decision("x1", range(-1, 3)),
            Decision("x2", range(-1, 3)),
            Decision("x3", range(-1, 3)),
        ),
        params="int: w = 1;\n",
        constraint="fzn_int_sliding_sum([x1, x2, x3], w, 0, 1)",
        holds=lambda a: all(0 <= sum(win) <= 1 for win in windows(arr(a, 3), 1)),
    ),

    Case(
        ident="int_sliding_sum_reif",
        why="reified, with both bounds at the same window sum",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: w = 2;\n",
        constraint="fzn_int_sliding_sum_reif([x1, x2, x3], w, 1, 1, b)",
        holds=lambda a: a["b"]
        == all(sum(win) == 1 for win in windows(arr(a, 3), 2)),
    ),

    Case(
        ident="int_lex_least_under_permutation",
        why="a swap of the two entries, where a tie is still least",
        decisions=(Decision("x1", range(0, 4)), Decision("x2", range(0, 4))),
        params="array [1..4] of int: perms = [1, 2, 2, 1];\n",
        constraint="fzn_int_lex_least_under_permutation([x1, x2], perms)",
        holds=lambda a: lex_least_under(arr(a, 2), [1, 2, 2, 1]),
    ),

    # The three rotations of a triple form a group, so the registry's reading
    # (least under each listed permutation) and a pairwise one coincide.
    Case(
        ident="int_lex_least_under_permutation",
        why="least among the rotations of a triple, all-equal included",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("x3", range(0, 3)),
        ),
        params="array [1..9] of int: perms = [1, 2, 3, 2, 3, 1, 3, 1, 2];\n",
        constraint="fzn_int_lex_least_under_permutation([x1, x2, x3], perms)",
        holds=lambda a: lex_least_under(
            arr(a, 3), [1, 2, 3, 2, 3, 1, 3, 1, 2]
        ),
    ),

    Case(
        ident="int_lex_least_under_permutation_reif",
        why="reified under a single swap",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="array [1..4] of int: perms = [1, 2, 2, 1];\n",
        constraint="fzn_int_lex_least_under_permutation_reif([x1, x2], perms, b)",
        holds=lambda a: a["b"] == lex_least_under(arr(a, 2), [1, 2, 2, 1]),
    ),

    Case(
        ident="int_lex_least_under_transposition",
        why="a two by two matrix, least among its rows, columns and transpose",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
            Decision("x4", range(0, 2)),
        ),
        params="int: c = 2;\n",
        constraint="fzn_int_lex_least_under_transposition([x1, x2, x3, x4], c)",
        holds=lambda a: all(
            tuple(arr(a, 4)) <= img for img in square_images(arr(a, 4), 2)
        ),
    ),

    Case(
        ident="int_lex_least_under_transposition",
        why="a one by one matrix, whose only image is itself",
        decisions=(Decision("x1", range(0, 3)),),
        params="int: c = 1;\n",
        constraint="fzn_int_lex_least_under_transposition([x1], c)",
        holds=lambda a: True,
    ),

    Case(
        ident="int_lex_least_under_transposition_reif",
        why="reified over a two by two matrix",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
            Decision("x4", range(0, 2)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: c = 2;\n",
        constraint="fzn_int_lex_least_under_transposition_reif([x1, x2, x3, x4], c, b)",
        holds=lambda a: a["b"]
        == all(tuple(arr(a, 4)) <= img for img in square_images(arr(a, 4), 2)),
    ),

    Case(
        ident="int_global_cardinality",
        why="a covered value may occur nowhere, or everywhere",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
            Decision("c1", range(0, 4)),
            Decision("c2", range(0, 4)),
        ),
        params="array [1..2] of int: cover = [0, 1];\n",
        constraint="fzn_int_global_cardinality([x1, x2, x3], cover, [c1, c2])",
        holds=lambda a: a["c1"] == sum(x == 0 for x in arr(a, 3))
        and a["c2"] == sum(x == 1 for x in arr(a, 3)),
    ),

    Case(
        ident="int_global_cardinality",
        why="a covered value outside the domain, and an uncovered one left free",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("c1", range(0, 3)),
            Decision("c2", range(0, 3)),
        ),
        params="array [1..2] of int: cover = [0, 5];\n",
        constraint="fzn_int_global_cardinality([x1, x2], cover, [c1, c2])",
        holds=lambda a: a["c1"] == sum(x == 0 for x in arr(a, 2)) and a["c2"] == 0,
    ),

    Case(
        ident="int_global_cardinality_closed",
        why="a value outside the cover is now forbidden",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("x3", range(0, 3)),
            Decision("c1", range(0, 4)),
            Decision("c2", range(0, 4)),
        ),
        params="array [1..2] of int: cover = [0, 1];\n",
        constraint="fzn_int_global_cardinality_closed([x1, x2, x3], cover, [c1, c2])",
        holds=lambda a: all(x in (0, 1) for x in arr(a, 3))
        and a["c1"] == sum(x == 0 for x in arr(a, 3))
        and a["c2"] == sum(x == 1 for x in arr(a, 3)),
    ),

    Case(
        ident="int_global_cardinality_closed_reif",
        why="reified, where a single covered value forces the whole array",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("c1", range(0, 3)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="array [1..1] of int: cover = [0];\n",
        constraint="fzn_int_global_cardinality_closed_reif([x1, x2], cover, [c1], b)",
        holds=lambda a: a["b"]
        == (
            all(x == 0 for x in arr(a, 2))
            and a["c1"] == sum(x == 0 for x in arr(a, 2))
        ),
    ),

    Case(
        ident="int_global_cardinality_low_up",
        why="the bounds are met exactly, and values outside the cover stay free",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("x3", range(0, 3)),
        ),
        params=(
            "array [1..2] of int: cover = [0, 1];\n"
            "array [1..2] of int: lo = [1, 1];\n"
            "array [1..2] of int: up = [1, 1];\n"
        ),
        constraint="fzn_int_global_cardinality_low_up([x1, x2, x3], cover, lo, up)",
        holds=lambda a: sum(x == 0 for x in arr(a, 3)) == 1
        and sum(x == 1 for x in arr(a, 3)) == 1,
    ),

    Case(
        ident="int_global_cardinality_low_up",
        why="a lower bound of zero constrains nothing on that value",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
        ),
        params=(
            "array [1..2] of int: cover = [0, 1];\n"
            "array [1..2] of int: lo = [0, 2];\n"
            "array [1..2] of int: up = [3, 3];\n"
        ),
        constraint="fzn_int_global_cardinality_low_up([x1, x2, x3], cover, lo, up)",
        holds=lambda a: sum(x == 1 for x in arr(a, 3)) >= 2,
    ),

    Case(
        ident="int_global_cardinality_low_up_reif",
        why="reified, with the lower and upper bound at the same count",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params=(
            "array [1..1] of int: cover = [1];\n"
            "array [1..1] of int: lo = [2];\n"
            "array [1..1] of int: up = [2];\n"
        ),
        constraint="fzn_int_global_cardinality_low_up_reif([x1, x2, x3], cover, lo, up, b)",
        holds=lambda a: a["b"] == (sum(x == 1 for x in arr(a, 3)) == 2),
    ),

    Case(
        ident="int_global_cardinality_low_up_closed",
        why="bounds wide open, so only the closedness bites",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("x3", range(0, 3)),
        ),
        params=(
            "array [1..2] of int: cover = [0, 1];\n"
            "array [1..2] of int: lo = [0, 0];\n"
            "array [1..2] of int: up = [3, 3];\n"
        ),
        constraint="fzn_int_global_cardinality_low_up_closed([x1, x2, x3], cover, lo, up)",
        holds=lambda a: all(x in (0, 1) for x in arr(a, 3)),
    ),

    Case(
        ident="int_global_cardinality_low_up_closed_reif",
        why="reified, closed and with a bound that one value must meet",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params=(
            "array [1..2] of int: cover = [0, 1];\n"
            "array [1..2] of int: lo = [1, 0];\n"
            "array [1..2] of int: up = [2, 2];\n"
        ),
        constraint="fzn_int_global_cardinality_low_up_closed_reif([x1, x2], cover, lo, up, b)",
        holds=lambda a: a["b"]
        == (
            all(x in (0, 1) for x in arr(a, 2))
            and sum(x == 0 for x in arr(a, 2)) >= 1
        ),
    ),

    # Membership, not equality: a set holding the covered value counts once
    # however many other values it holds.
    Case(
        ident="set_of_int_global_cardinality",
        why="a covered value counted by membership across the sets",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("c1", range(0, 3)),
            Decision("c2", range(0, 3)),
        ),
        params="array [1..2] of int: cover = [1, 2];\n",
        constraint="fzn_set_of_int_global_cardinality([s1, s2], cover, [c1, c2])",
        holds=lambda a: a["c1"] == sum(1 in s for s in arr(a, 2, "s"))
        and a["c2"] == sum(2 in s for s in arr(a, 2, "s")),
    ),

    Case(
        ident="set_of_int_global_cardinality",
        why="a covered value no set can hold counts zero",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("c1", range(0, 3)),
        ),
        params="array [1..1] of int: cover = [7];\n",
        constraint="fzn_set_of_int_global_cardinality([s1, s2], cover, [c1])",
        holds=lambda a: a["c1"] == 0,
    ),

    Case(
        ident="set_of_int_global_cardinality_closed",
        why="closed, so no set may hold a value outside the cover",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("c1", range(0, 3)),
        ),
        params="array [1..1] of int: cover = [1];\n",
        constraint="fzn_set_of_int_global_cardinality_closed([s1, s2], cover, [c1])",
        holds=lambda a: all(s <= {1} for s in arr(a, 2, "s"))
        and a["c1"] == sum(1 in s for s in arr(a, 2, "s")),
    ),

    Case(
        ident="set_of_int_global_cardinality_closed_reif",
        why="reified over a single set, which the cover closes to one value",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("c1", range(0, 2)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="array [1..1] of int: cover = [1];\n",
        constraint="fzn_set_of_int_global_cardinality_closed_reif([s1], cover, [c1], b)",
        holds=lambda a: a["b"]
        == (set(a["s1"]) <= {1} and a["c1"] == (1 in a["s1"])),
    ),

    Case(
        ident="set_of_int_global_cardinality_low_up",
        why="one value needed by every set, another by none",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
        ),
        params=(
            "array [1..2] of int: cover = [1, 2];\n"
            "array [1..2] of int: lo = [2, 0];\n"
            "array [1..2] of int: up = [2, 0];\n"
        ),
        constraint="fzn_set_of_int_global_cardinality_low_up([s1, s2], cover, lo, up)",
        holds=lambda a: sum(1 in s for s in arr(a, 2, "s")) == 2
        and sum(2 in s for s in arr(a, 2, "s")) == 0,
    ),

    Case(
        ident="set_of_int_global_cardinality_low_up_reif",
        why="reified, with the bound met by exactly one set",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("b", BOOLS, kind="bool"),
        ),
        params=(
            "array [1..1] of int: cover = [1];\n"
            "array [1..1] of int: lo = [1];\n"
            "array [1..1] of int: up = [1];\n"
        ),
        constraint="fzn_set_of_int_global_cardinality_low_up_reif([s1, s2], cover, lo, up, b)",
        holds=lambda a: a["b"] == (sum(1 in s for s in arr(a, 2, "s")) == 1),
    ),

    Case(
        ident="set_of_int_global_cardinality_low_up_closed",
        why="bounds wide open, so only the closedness bites",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
        ),
        params=(
            "array [1..1] of int: cover = [1];\n"
            "array [1..1] of int: lo = [0];\n"
            "array [1..1] of int: up = [2];\n"
        ),
        constraint="fzn_set_of_int_global_cardinality_low_up_closed([s1, s2], cover, lo, up)",
        holds=lambda a: all(s <= {1} for s in arr(a, 2, "s")),
    ),

    Case(
        ident="set_of_int_global_cardinality_low_up_closed_reif",
        why="reified, closed and with a lower bound of one",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("b", BOOLS, kind="bool"),
        ),
        params=(
            "array [1..1] of int: cover = [1];\n"
            "array [1..1] of int: lo = [1];\n"
            "array [1..1] of int: up = [2];\n"
        ),
        constraint="fzn_set_of_int_global_cardinality_low_up_closed_reif([s1, s2], cover, lo, up, b)",
        holds=lambda a: a["b"]
        == (
            all(s <= {1} for s in arr(a, 2, "s"))
            and sum(1 in s for s in arr(a, 2, "s")) >= 1
        ),
    ),

    Case(
        ident="int_distribute",
        why="the counted values are decisions, so the same value may be covered twice",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("v1", range(0, 2)),
            Decision("v2", range(0, 2)),
            Decision("c1", range(0, 3)),
            Decision("c2", range(0, 3)),
        ),
        constraint="fzn_int_distribute([x1, x2], [v1, v2], [c1, c2])",
        holds=lambda a: a["c1"] == sum(x == a["v1"] for x in arr(a, 2))
        and a["c2"] == sum(x == a["v2"] for x in arr(a, 2)),
    ),

    Case(
        ident="int_increasing",
        why="ties are allowed, one position out of order is not",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("x3", range(0, 3)),
        ),
        constraint="fzn_int_increasing([x1, x2, x3])",
        holds=lambda a: arr(a, 3) == sorted(arr(a, 3)),
    ),

    Case(
        ident="int_increasing",
        why="an array of one element is already in order",
        decisions=(Decision("x1", range(0, 3)),),
        constraint="fzn_int_increasing([x1])",
        holds=lambda a: True,
    ),

    Case(
        ident="int_increasing_reif",
        why="reified, so the false half must force a descent",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_int_increasing_reif([x1, x2], b)",
        holds=lambda a: a["b"] == (a["x1"] <= a["x2"]),
    ),

    Case(
        ident="bool_increasing",
        why="false below true, with ties allowed",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("x3", BOOLS, kind="bool"),
        ),
        constraint="fzn_bool_increasing([x1, x2, x3])",
        holds=lambda a: arr(a, 3) == sorted(arr(a, 3)),
    ),

    Case(
        ident="bool_increasing_reif",
        why="reified, where only true then false may falsify it",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_bool_increasing_reif([x1, x2], b)",
        holds=lambda a: a["b"] == (a["x1"] <= a["x2"]),
    ),

    Case(
        ident="set_of_int_increasing",
        why="the standard set order, where equal sets tie",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
        ),
        constraint="fzn_set_of_int_increasing([s1, s2])",
        holds=lambda a: rank(a["s1"]) <= rank(a["s2"]),
    ),

    Case(
        ident="set_of_int_increasing_reif",
        why="reified, which separates the tie from a descent",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_set_of_int_increasing_reif([s1, s2], b)",
        holds=lambda a: a["b"] == (rank(a["s1"]) <= rank(a["s2"])),
    ),

    Case(
        ident="int_strictly_increasing",
        why="a pair of equal values, which is what separates it from int_increasing",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("x3", range(0, 3)),
        ),
        constraint="fzn_int_strictly_increasing([x1, x2, x3])",
        holds=lambda a: all(x < y for x, y in zip(arr(a, 3), arr(a, 3)[1:])),
    ),

    Case(
        ident="int_strictly_increasing_reif",
        why="reified, with the tie false and the ascent true",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_int_strictly_increasing_reif([x1, x2], b)",
        holds=lambda a: a["b"] == (a["x1"] < a["x2"]),
    ),

    Case(
        ident="set_of_int_strictly_increasing",
        why="equal sets, which the non-strict form would allow",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
        ),
        constraint="fzn_set_of_int_strictly_increasing([s1, s2])",
        holds=lambda a: rank(a["s1"]) < rank(a["s2"]),
    ),

    Case(
        ident="set_of_int_strictly_increasing_reif",
        why="reified, with equal sets on the false side",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_set_of_int_strictly_increasing_reif([s1, s2], b)",
        holds=lambda a: a["b"] == (rank(a["s1"]) < rank(a["s2"])),
    ),

    Case(
        ident="int_lex_lesseq",
        why="equal arrays, and a difference at the first position only",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("y2", range(0, 2)),
        ),
        constraint="fzn_int_lex_lesseq([x1, x2], [y1, y2])",
        holds=lambda a: tuple(arr(a, 2)) <= tuple(arr(a, 2, "y")),
    ),

    Case(
        ident="int_lex_lesseq",
        why="unequal lengths, where a proper prefix is the smaller array",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("y2", range(0, 2)),
        ),
        constraint="fzn_int_lex_lesseq([x1], [y1, y2])",
        holds=lambda a: (a["x1"],) <= tuple(arr(a, 2, "y")),
    ),

    Case(
        ident="int_lex_lesseq_reif",
        why="reified, with equal arrays on the true side",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("y2", range(0, 2)),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_int_lex_lesseq_reif([x1, x2], [y1, y2], b)",
        holds=lambda a: a["b"] == (tuple(arr(a, 2)) <= tuple(arr(a, 2, "y"))),
    ),

    Case(
        ident="int_lex_less",
        why="equal arrays, which the non-strict form would allow",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("y2", range(0, 2)),
        ),
        constraint="fzn_int_lex_less([x1, x2], [y1, y2])",
        holds=lambda a: tuple(arr(a, 2)) < tuple(arr(a, 2, "y")),
    ),

    Case(
        ident="int_lex_less_reif",
        why="reified, with equal arrays on the false side",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("y1", range(0, 2)),
            Decision("y2", range(0, 2)),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_int_lex_less_reif([x1, x2], [y1, y2], b)",
        holds=lambda a: a["b"] == (tuple(arr(a, 2)) < tuple(arr(a, 2, "y"))),
    ),

    Case(
        ident="bool_lex_lesseq",
        why="false below true, with equal arrays allowed",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("y1", BOOLS, kind="bool"),
            Decision("y2", BOOLS, kind="bool"),
        ),
        constraint="fzn_bool_lex_lesseq([x1, x2], [y1, y2])",
        holds=lambda a: tuple(arr(a, 2)) <= tuple(arr(a, 2, "y")),
    ),

    Case(
        ident="bool_lex_lesseq_reif",
        why="reified over Boolean arrays",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("y1", BOOLS, kind="bool"),
            Decision("y2", BOOLS, kind="bool"),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_bool_lex_lesseq_reif([x1, x2], [y1, y2], b)",
        holds=lambda a: a["b"] == (tuple(arr(a, 2)) <= tuple(arr(a, 2, "y"))),
    ),

    Case(
        ident="bool_lex_less",
        why="equal Boolean arrays, which the non-strict form would allow",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("y1", BOOLS, kind="bool"),
            Decision("y2", BOOLS, kind="bool"),
        ),
        constraint="fzn_bool_lex_less([x1, x2], [y1, y2])",
        holds=lambda a: tuple(arr(a, 2)) < tuple(arr(a, 2, "y")),
    ),

    Case(
        ident="bool_lex_less_reif",
        why="reified, with equal Boolean arrays on the false side",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("y1", BOOLS, kind="bool"),
            Decision("y2", BOOLS, kind="bool"),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_bool_lex_less_reif([x1, x2], [y1, y2], b)",
        holds=lambda a: a["b"] == (tuple(arr(a, 2)) < tuple(arr(a, 2, "y"))),
    ),

    Case(
        ident="set_of_int_lex_lesseq",
        why="array lex over the set order, with equal arrays allowed",
        decisions=(
            Decision("s1", subsets([1]), kind="set of int"),
            Decision("s2", subsets([1]), kind="set of int"),
            Decision("t1", subsets([1]), kind="set of int"),
            Decision("t2", subsets([1]), kind="set of int"),
        ),
        constraint="fzn_set_of_int_lex_lesseq([s1, s2], [t1, t2])",
        holds=lambda a: tuple(rank(s) for s in arr(a, 2, "s"))
        <= tuple(rank(t) for t in arr(a, 2, "t")),
    ),

    Case(
        ident="set_of_int_lex_lesseq",
        why="arrays of one set, so the set order alone decides",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("t1", subsets([1, 2]), kind="set of int"),
        ),
        constraint="fzn_set_of_int_lex_lesseq([s1], [t1])",
        holds=lambda a: rank(a["s1"]) <= rank(a["t1"]),
    ),

    Case(
        ident="set_of_int_lex_lesseq_reif",
        why="reified over arrays of sets",
        decisions=(
            Decision("s1", subsets([1]), kind="set of int"),
            Decision("s2", subsets([1]), kind="set of int"),
            Decision("t1", subsets([1]), kind="set of int"),
            Decision("t2", subsets([1]), kind="set of int"),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_set_of_int_lex_lesseq_reif([s1, s2], [t1, t2], b)",
        holds=lambda a: a["b"]
        == (
            tuple(rank(s) for s in arr(a, 2, "s"))
            <= tuple(rank(t) for t in arr(a, 2, "t"))
        ),
    ),

    Case(
        ident="set_of_int_lex_less",
        why="equal arrays of sets, which the non-strict form would allow",
        decisions=(
            Decision("s1", subsets([1]), kind="set of int"),
            Decision("s2", subsets([1]), kind="set of int"),
            Decision("t1", subsets([1]), kind="set of int"),
            Decision("t2", subsets([1]), kind="set of int"),
        ),
        constraint="fzn_set_of_int_lex_less([s1, s2], [t1, t2])",
        holds=lambda a: tuple(rank(s) for s in arr(a, 2, "s"))
        < tuple(rank(t) for t in arr(a, 2, "t")),
    ),

    Case(
        ident="set_of_int_lex_less_reif",
        why="reified, with equal arrays of sets on the false side",
        decisions=(
            Decision("s1", subsets([1]), kind="set of int"),
            Decision("s2", subsets([1]), kind="set of int"),
            Decision("t1", subsets([1]), kind="set of int"),
            Decision("t2", subsets([1]), kind="set of int"),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_set_of_int_lex_less_reif([s1, s2], [t1, t2], b)",
        holds=lambda a: a["b"]
        == (
            tuple(rank(s) for s in arr(a, 2, "s"))
            < tuple(rank(t) for t in arr(a, 2, "t"))
        ),
    ),

    Case(
        ident="int_lex_chain_lesseq",
        why="three rows of two columns, where the columns may tie",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
            Decision("x4", range(0, 2)),
            Decision("x5", range(0, 2)),
            Decision("x6", range(0, 2)),
        ),
        params="int: c = 2;\n",
        constraint="fzn_int_lex_chain_lesseq([x1, x2, x3, x4, x5, x6], c)",
        holds=lambda a: chain_lesseq(arr(a, 6), 2, strict=False),
    ),

    Case(
        ident="int_lex_chain_lesseq",
        why="a single column, which has nothing to compare against",
        decisions=(Decision("x1", range(0, 2)), Decision("x2", range(0, 2))),
        params="int: c = 1;\n",
        constraint="fzn_int_lex_chain_lesseq([x1, x2], c)",
        holds=lambda a: True,
    ),

    Case(
        ident="int_lex_chain_lesseq_reif",
        why="reified over a two by two matrix",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
            Decision("x4", range(0, 2)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: c = 2;\n",
        constraint="fzn_int_lex_chain_lesseq_reif([x1, x2, x3, x4], c, b)",
        holds=lambda a: a["b"] == chain_lesseq(arr(a, 4), 2, strict=False),
    ),

    Case(
        ident="int_lex_chain_less",
        why="equal columns, which the non-strict form would allow",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
            Decision("x4", range(0, 2)),
            Decision("x5", range(0, 2)),
            Decision("x6", range(0, 2)),
        ),
        params="int: c = 2;\n",
        constraint="fzn_int_lex_chain_less([x1, x2, x3, x4, x5, x6], c)",
        holds=lambda a: chain_lesseq(arr(a, 6), 2, strict=True),
    ),

    Case(
        ident="int_lex_chain_less_reif",
        why="reified, with equal columns on the false side",
        decisions=(
            Decision("x1", range(0, 2)),
            Decision("x2", range(0, 2)),
            Decision("x3", range(0, 2)),
            Decision("x4", range(0, 2)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: c = 2;\n",
        constraint="fzn_int_lex_chain_less_reif([x1, x2, x3, x4], c, b)",
        holds=lambda a: a["b"] == chain_lesseq(arr(a, 4), 2, strict=True),
    ),

    Case(
        ident="bool_lex_chain_lesseq",
        why="three rows of two Boolean columns, which may tie",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("x3", BOOLS, kind="bool"),
            Decision("x4", BOOLS, kind="bool"),
            Decision("x5", BOOLS, kind="bool"),
            Decision("x6", BOOLS, kind="bool"),
        ),
        params="int: c = 2;\n",
        constraint="fzn_bool_lex_chain_lesseq([x1, x2, x3, x4, x5, x6], c)",
        holds=lambda a: chain_lesseq(arr(a, 6), 2, strict=False),
    ),

    Case(
        ident="bool_lex_chain_lesseq_reif",
        why="reified over a two by two Boolean matrix",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("x3", BOOLS, kind="bool"),
            Decision("x4", BOOLS, kind="bool"),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: c = 2;\n",
        constraint="fzn_bool_lex_chain_lesseq_reif([x1, x2, x3, x4], c, b)",
        holds=lambda a: a["b"] == chain_lesseq(arr(a, 4), 2, strict=False),
    ),

    Case(
        ident="bool_lex_chain_less",
        why="equal Boolean columns, which the non-strict form would allow",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("x3", BOOLS, kind="bool"),
            Decision("x4", BOOLS, kind="bool"),
            Decision("x5", BOOLS, kind="bool"),
            Decision("x6", BOOLS, kind="bool"),
        ),
        params="int: c = 2;\n",
        constraint="fzn_bool_lex_chain_less([x1, x2, x3, x4, x5, x6], c)",
        holds=lambda a: chain_lesseq(arr(a, 6), 2, strict=True),
    ),

    Case(
        ident="bool_lex_chain_less_reif",
        why="reified, with equal Boolean columns on the false side",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("x3", BOOLS, kind="bool"),
            Decision("x4", BOOLS, kind="bool"),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: c = 2;\n",
        constraint="fzn_bool_lex_chain_less_reif([x1, x2, x3, x4], c, b)",
        holds=lambda a: a["b"] == chain_lesseq(arr(a, 4), 2, strict=True),
    ),

    Case(
        ident="bool_lex_chain_lesseq_orbitope_partition",
        why="a row of all false, which the partition form forbids",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("x3", BOOLS, kind="bool"),
            Decision("x4", BOOLS, kind="bool"),
            Decision("x5", BOOLS, kind="bool"),
            Decision("x6", BOOLS, kind="bool"),
        ),
        params="int: c = 2;\n",
        constraint="fzn_bool_lex_chain_lesseq_orbitope_partition([x1, x2, x3, x4, x5, x6], c)",
        holds=lambda a: chain_lesseq(arr(a, 6), 2, strict=False)
        and all(sum(row) == 1 for row in (arr(a, 6)[i : i + 2] for i in (0, 2, 4))),
    ),

    Case(
        ident="bool_lex_chain_lesseq_orbitope_partition_reif",
        why="reified, where the row condition and the column order both bite",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("x3", BOOLS, kind="bool"),
            Decision("x4", BOOLS, kind="bool"),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: c = 2;\n",
        constraint="fzn_bool_lex_chain_lesseq_orbitope_partition_reif([x1, x2, x3, x4], c, b)",
        holds=lambda a: a["b"]
        == (
            chain_lesseq(arr(a, 4), 2, strict=False)
            and all(sum(row) == 1 for row in (arr(a, 4)[i : i + 2] for i in (0, 2)))
        ),
    ),

    Case(
        ident="bool_lex_chain_lesseq_orbitope_packing",
        why="a row of all false, which the packing form allows",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("x3", BOOLS, kind="bool"),
            Decision("x4", BOOLS, kind="bool"),
            Decision("x5", BOOLS, kind="bool"),
            Decision("x6", BOOLS, kind="bool"),
        ),
        params="int: c = 2;\n",
        constraint="fzn_bool_lex_chain_lesseq_orbitope_packing([x1, x2, x3, x4, x5, x6], c)",
        holds=lambda a: chain_lesseq(arr(a, 6), 2, strict=False)
        and all(sum(row) <= 1 for row in (arr(a, 6)[i : i + 2] for i in (0, 2, 4))),
    ),

    Case(
        ident="bool_lex_chain_lesseq_orbitope_packing_reif",
        why="reified, with two true in one row on the false side",
        decisions=(
            Decision("x1", BOOLS, kind="bool"),
            Decision("x2", BOOLS, kind="bool"),
            Decision("x3", BOOLS, kind="bool"),
            Decision("x4", BOOLS, kind="bool"),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: c = 2;\n",
        constraint="fzn_bool_lex_chain_lesseq_orbitope_packing_reif([x1, x2, x3, x4], c, b)",
        holds=lambda a: a["b"]
        == (
            chain_lesseq(arr(a, 4), 2, strict=False)
            and all(sum(row) <= 1 for row in (arr(a, 4)[i : i + 2] for i in (0, 2)))
        ),
    ),

    Case(
        ident="int_value_precede",
        why="the later value absent, present first, and present after the earlier one",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("x3", range(0, 3)),
        ),
        params="int: s = 1;\nint: t = 2;\n",
        constraint="fzn_int_value_precede(s, t, [x1, x2, x3])",
        holds=lambda a: precede(arr(a, 3), 1, 2),
    ),

    Case(
        ident="int_value_precede_reif",
        why="reified, where the first position taking the later value falsifies it",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: s = 1;\nint: t = 2;\n",
        constraint="fzn_int_value_precede_reif(s, t, [x1, x2], b)",
        holds=lambda a: a["b"] == precede(arr(a, 2), 1, 2),
    ),

    # A set holding both values is neither a witness nor a violation, which is
    # what makes the set form different from the integer one.
    Case(
        ident="set_of_int_value_precede",
        why="a set holding both values counts for neither side",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
        ),
        params="int: s = 1;\nint: t = 2;\n",
        constraint="fzn_set_of_int_value_precede(s, t, [s1, s2])",
        holds=lambda a: set_precede(arr(a, 2, "s"), 1, 2),
    ),

    Case(
        ident="set_of_int_value_precede_reif",
        why="reified over two sets",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: s = 1;\nint: t = 2;\n",
        constraint="fzn_set_of_int_value_precede_reif(s, t, [s1, s2], b)",
        holds=lambda a: a["b"] == set_precede(arr(a, 2, "s"), 1, 2),
    ),

    Case(
        ident="int_value_precede_chain",
        why="a chain of three, where skipping the middle value breaks it",
        decisions=(
            Decision("x1", range(1, 4)),
            Decision("x2", range(1, 4)),
            Decision("x3", range(1, 4)),
        ),
        params="array [1..3] of int: vals = [1, 2, 3];\n",
        constraint="fzn_int_value_precede_chain(vals, [x1, x2, x3])",
        holds=lambda a: all(
            precede(arr(a, 3), s, t) for s, t in ((1, 2), (2, 3))
        ),
    ),

    Case(
        ident="int_value_precede_chain_reif",
        why="reified over a chain of two, with an unlisted value left free",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="array [1..2] of int: vals = [1, 2];\n",
        constraint="fzn_int_value_precede_chain_reif(vals, [x1, x2], b)",
        holds=lambda a: a["b"] == precede(arr(a, 2), 1, 2),
    ),

    Case(
        ident="set_of_int_value_precede_chain",
        why="a chain of two over sets, where holding both values is neutral",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
        ),
        params="array [1..2] of int: vals = [1, 2];\n",
        constraint="fzn_set_of_int_value_precede_chain(vals, [s1, s2])",
        holds=lambda a: set_precede(arr(a, 2, "s"), 1, 2),
    ),

    Case(
        ident="set_of_int_value_precede_chain_reif",
        why="reified over a chain of two sets",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="array [1..2] of int: vals = [1, 2];\n",
        constraint="fzn_set_of_int_value_precede_chain_reif(vals, [s1, s2], b)",
        holds=lambda a: a["b"] == set_precede(arr(a, 2, "s"), 1, 2),
    ),

    Case(
        ident="int_seq_precede_chain",
        why="values at or below one are free, a jump of two is not",
        decisions=(
            Decision("x1", range(0, 4)),
            Decision("x2", range(0, 4)),
            Decision("x3", range(0, 4)),
        ),
        constraint="fzn_int_seq_precede_chain([x1, x2, x3])",
        holds=lambda a: all(
            any(arr(a, 3)[i] == arr(a, 3)[j] - 1 for i in range(j))
            for j in range(3)
            if arr(a, 3)[j] > 1
        ),
    ),

    Case(
        ident="int_seq_precede_chain_reif",
        why="reified, where a two in the first position falsifies it",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_int_seq_precede_chain_reif([x1, x2], b)",
        holds=lambda a: a["b"]
        == all(
            any(arr(a, 2)[i] == arr(a, 2)[j] - 1 for i in range(j))
            for j in range(2)
            if arr(a, 2)[j] > 1
        ),
    ),

    Case(
        ident="set_of_int_seq_precede_chain",
        why="a two needs a one in an earlier set, not the same one",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
        ),
        constraint="fzn_set_of_int_seq_precede_chain([s1, s2])",
        holds=lambda a: all(
            any(v - 1 in arr(a, 2, "s")[i] for i in range(j))
            for j in range(2)
            for v in arr(a, 2, "s")[j]
            if v > 1
        ),
    ),

    Case(
        ident="set_of_int_seq_precede_chain_reif",
        why="reified, with the empty set on the true side",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("b", BOOLS, kind="bool"),
        ),
        constraint="fzn_set_of_int_seq_precede_chain_reif([s1, s2], b)",
        holds=lambda a: a["b"]
        == all(
            any(v - 1 in arr(a, 2, "s")[i] for i in range(j))
            for j in range(2)
            for v in arr(a, 2, "s")[j]
            if v > 1
        ),
    ),

    Case(
        ident="int_inverse",
        why="offsets that differ, with each array's values inside the other's range",
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            Decision("y1", range(0, 2)),
            Decision("y2", range(0, 2)),
        ),
        params="int: fo = 0;\nint: io = 1;\n",
        constraint="fzn_int_inverse([x1, x2], fo, [y1, y2], io)",
        holds=lambda a: inverse_both(arr(a, 2), 0, arr(a, 2, "y"), 1),
    ),

    Case(
        ident="int_inverse_reif",
        why="reified, where the identity and the swap are the only true cases",
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            Decision("y1", range(1, 3)),
            Decision("y2", range(1, 3)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: fo = 1;\nint: io = 1;\n",
        constraint="fzn_int_inverse_reif([x1, x2], fo, [y1, y2], io, b)",
        holds=lambda a: a["b"] == inverse_both(arr(a, 2), 1, arr(a, 2, "y"), 1),
    ),

    Case(
        ident="int_inverse_in_range",
        why="a value outside the other array's range leaves that index free",
        decisions=(
            Decision("x1", range(1, 4)),
            Decision("x2", range(1, 4)),
            Decision("y1", range(1, 4)),
            Decision("y2", range(1, 4)),
        ),
        params="int: fo = 1;\nint: io = 1;\n",
        constraint="fzn_int_inverse_in_range([x1, x2], fo, [y1, y2], io)",
        holds=lambda a: inverse_in_range(arr(a, 2), 1, arr(a, 2, "y"), 1),
    ),

    Case(
        ident="int_inverse_in_range_reif",
        why="reified, with every value in range so it is a plain inverse",
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            Decision("y1", range(1, 3)),
            Decision("y2", range(1, 3)),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: fo = 1;\nint: io = 1;\n",
        constraint="fzn_int_inverse_in_range_reif([x1, x2], fo, [y1, y2], io, b)",
        holds=lambda a: a["b"] == inverse_in_range(arr(a, 2), 1, arr(a, 2, "y"), 1),
    ),

    Case(
        ident="set_of_int_inverse",
        why="offsets that differ, with the empty relation among the solutions",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("s2", subsets([1, 2]), kind="set of int"),
            Decision("t1", subsets([0, 1]), kind="set of int"),
            Decision("t2", subsets([0, 1]), kind="set of int"),
        ),
        params="int: fo = 0;\nint: io = 1;\n",
        constraint="fzn_set_of_int_inverse([s1, s2], fo, [t1, t2], io)",
        holds=lambda a: set_inverse(arr(a, 2, "s"), 0, arr(a, 2, "t"), 1),
    ),

    Case(
        ident="set_of_int_inverse_reif",
        why="reified, over arrays of one and of two sets",
        decisions=(
            Decision("s1", subsets([1, 2]), kind="set of int"),
            Decision("t1", subsets([1]), kind="set of int"),
            Decision("t2", subsets([1]), kind="set of int"),
            Decision("b", BOOLS, kind="bool"),
        ),
        params="int: fo = 1;\nint: io = 1;\n",
        constraint="fzn_set_of_int_inverse_reif([s1], fo, [t1, t2], io, b)",
        holds=lambda a: a["b"]
        == set_inverse([a["s1"]], 1, arr(a, 2, "t"), 1),
    ),
)
