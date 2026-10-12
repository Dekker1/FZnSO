"""Graph and extensional constraints.

The graph constraints fix a representation: `from` and `to` give one entry per
edge, `node_offset` is the number the caller's vertices start from, and the
`nodes` and `edges` Booleans select the subgraph. Each case states its graph in
`params` and repeats it in the predicate, so the predicate reads the registry's
definition and nothing else.

Two things the registry states once rather than per entry. Well-formedness is
`graph_subgraph`, "the condition every other graph constraint here assumes", so
the predicates below conjoin it, and a case that would otherwise turn on an
ill-formed selection keeps its nodes in instead. And a functional entry names
no `_reif` or `_imp` form, so the seven entries that report a weight or a cost
get neither; the other nineteen get both. An `_imp` form has no
`fzn_<ident>_imp.mzn` to include, by the library's own decision, so a
half-reified case calls the base constraint under an implication and lets the
compiler choose the form.

Each case aims at an edge: a graph of two nodes, a self loop, a duplicated
edge, an isolated node, a numbering that does not start at one.
"""

from __future__ import annotations

from fznso_conform.case import Case, Decision

# ---------------------------------------------------------------------------
# The graphs, each given once and used both ways
# ---------------------------------------------------------------------------

#: Two nodes, one edge between them.
EDGE12 = ((1,), (2,))
#: Two nodes, two parallel copies of the same edge.
TWIN = ((1, 1), (2, 2))
#: Two nodes, one arc each way.
BOTH_WAYS = ((1, 2), (2, 1))
#: The triangle on three nodes.
TRIANGLE = ((1, 2, 3), (2, 3, 1))
#: Three nodes in a chain, as arcs.
CHAIN = ((1, 2), (2, 3))
#: A chain with the short cut that skips its middle node.
FORK = ((1, 2, 1), (2, 3, 3))
#: A two-cycle with a third node hanging off it.
LOLLIPOP = ((1, 2, 2), (2, 1, 3))
#: One node and a loop on it.
LOOP = ((1,), (1,))


def _graph(frm, to, **arrays):
    """The parameter declarations for a fixed edge list."""
    decls = [
        f"array [1..{len(frm)}] of int: FR = {list(frm)};",
        f"array [1..{len(to)}] of int: TO = {list(to)};",
    ]
    for name, values in arrays.items():
        decls.append(f"array [1..{len(values)}] of int: {name} = {list(values)};")
    return "\n".join(decls) + "\n"


def _bool(name):
    return Decision(name, (False, True), "bool")


# ---------------------------------------------------------------------------
# The definitions, in Python
# ---------------------------------------------------------------------------


def _sel_nodes(offset, ns):
    """N: the selected vertices, in the caller's numbering."""
    return [offset + i for i, b in enumerate(ns) if b]


def _sel_edges(frm, to, es):
    """E: the selected edges, as (from, to) pairs."""
    return [(frm[e], to[e]) for e, b in enumerate(es) if b]


def _both_ways(arcs):
    """Each edge read in both directions, for the undirected constraints."""
    return list(arcs) + [(v, u) for u, v in arcs]


def _reaches(arcs, source):
    """Every vertex reachable from the source, the source included."""
    seen = {source}
    while True:
        more = {v for u, v in arcs if u in seen} - seen
        if not more:
            return seen
        seen |= more


def _subgraph(frm, to, offset, ns, es):
    """Both endpoints of every selected edge are selected."""
    selected = set(_sel_nodes(offset, ns))
    return all(u in selected and v in selected for u, v in _sel_edges(frm, to, es))


def _all_reach(arcs, nodes):
    """Every selected vertex reaches every other one."""
    return all(v in _reaches(arcs, u) for u in nodes for v in nodes)


def _degree(frm, to, es, v):
    """How many selected edges touch v, a loop counting once."""
    return sum(1 for u, w in _sel_edges(frm, to, es) if v in (u, w))


def _in_degree(frm, to, es, v):
    """How many selected arcs end at v."""
    return sum(1 for _, w in _sel_edges(frm, to, es) if w == v)


def _rooted(arcs, nodes):
    """Some selected vertex reaches every selected vertex, itself included."""
    nodes = list(nodes)
    return bool(nodes) and any(set(nodes) <= _reaches(arcs, u) for u in nodes)


def _connected(frm, to, offset, ns, es):
    return _subgraph(frm, to, offset, ns, es) and _rooted(
        _both_ways(_sel_edges(frm, to, es)), _sel_nodes(offset, ns)
    )


def _connected_directed(frm, to, offset, ns, es):
    """One-way connectivity, which is what `dconnected` asks for.

    The standard library reads a root off the node index set and asks that it
    reach every selected node, so an arc each way between two nodes is not
    required; the registry's definition says the same.
    """
    return _subgraph(frm, to, offset, ns, es) and _rooted(
        _sel_edges(frm, to, es), _sel_nodes(offset, ns)
    )


def _reachable(frm, to, offset, root, ns, es):
    arcs = _both_ways(_sel_edges(frm, to, es))
    return _subgraph(frm, to, offset, ns, es) and set(_sel_nodes(offset, ns)) <= _reaches(
        arcs, root
    )


def _reachable_directed(frm, to, offset, root, ns, es):
    arcs = _sel_edges(frm, to, es)
    return _subgraph(frm, to, offset, ns, es) and set(_sel_nodes(offset, ns)) <= _reaches(
        arcs, root
    )


def _acyclic_directed(frm, to, offset, ns, es):
    arcs = _sel_edges(frm, to, es)
    on_cycle = any(
        v in _reaches(arcs, w) for v in _sel_nodes(offset, ns) for u, w in arcs if u == v
    )
    return _subgraph(frm, to, offset, ns, es) and not on_cycle


def _path(frm, to, offset, source, sink, ns, es):
    nodes = _sel_nodes(offset, ns)
    wanted = {v: 2 - (v == source) - (v == sink) for v in nodes}
    return (
        _subgraph(frm, to, offset, ns, es)
        and _all_reach(_both_ways(_sel_edges(frm, to, es)), nodes)
        and all(_degree(frm, to, es, v) == wanted[v] for v in nodes)
    )


def _path_directed(frm, to, offset, source, sink, ns, es):
    nodes = _sel_nodes(offset, ns)
    arcs = _sel_edges(frm, to, es)
    return (
        _subgraph(frm, to, offset, ns, es)
        and sink in _reaches(arcs, source)
        and all(_in_degree(frm, to, es, v) == (v != source) for v in nodes)
        and len(arcs) == len(nodes) - 1
    )


def _tree_shape(frm, to, offset, ns, es):
    """Connected, with one edge fewer than it has vertices."""
    nodes = _sel_nodes(offset, ns)
    return (
        _subgraph(frm, to, offset, ns, es)
        and _all_reach(_both_ways(_sel_edges(frm, to, es)), nodes)
        and len(_sel_edges(frm, to, es)) == len(nodes) - 1
    )


def _tree(frm, to, offset, root, ns, es):
    return _tree_shape(frm, to, offset, ns, es) and root in _sel_nodes(offset, ns)


def _tree_directed(frm, to, offset, root, ns, es):
    nodes = _sel_nodes(offset, ns)
    reached = _reaches(_sel_edges(frm, to, es), root)
    return (
        _subgraph(frm, to, offset, ns, es)
        and all(_in_degree(frm, to, es, v) == (v != root) for v in nodes)
        and set(nodes) <= reached
    )


def _weight(weight, es):
    """The total weight of the selected edges."""
    return sum(w for w, b in zip(weight, es) if b)


def _balanced(frm, to, offset, balance, flow):
    """Outflow less inflow is the node's balance, supply being positive."""
    for i, b in enumerate(balance):
        v = offset + i
        inflow = sum(f for e, f in enumerate(flow) if to[e] == v)
        outflow = sum(f for e, f in enumerate(flow) if frm[e] == v)
        if outflow - inflow != b:
            return False
    return True


def _table(xs, tuples):
    rows = [list(tuples[i : i + len(xs)]) for i in range(0, len(tuples), len(xs))]
    return list(xs) in rows


def _dfa(xs, states, alphabet, offset, transition, initial, accepting):
    state = initial
    for x in xs:
        if not offset <= x < offset + alphabet:
            return False
        state = transition[(state - 1) * alphabet + (x - offset)]
        if state == 0:  # a transition to 0 rejects the string
            return False
    return state in accepting


def _dfa_cost(xs, states, alphabet, offset, transition, initial, accepting, weight):
    """The accepted strings, each with the cost of the transitions it takes."""
    if not _dfa(xs, states, alphabet, offset, transition, initial, accepting):
        return None
    state, cost = initial, 0
    for x in xs:
        cost += weight[(state - 1) * alphabet + (x - offset)]
        state = transition[(state - 1) * alphabet + (x - offset)]
    return cost


def _nfa(xs, states, alphabet, offset, transition, initial, accepting):
    live = {initial}
    for x in xs:
        if not offset <= x < offset + alphabet:
            return False
        live = {
            q for state in live for q in transition[(state - 1) * alphabet + (x - offset)]
        }
        if not live:
            return False
    return bool(live & accepting)


def _mdd(xs, frm, label, to):
    """A path of one edge per variable, from the root, each label holding its value."""
    at = {1}
    for j, x in enumerate(xs):
        at = {to[e] for e in range(len(frm)) if frm[e] in at and x in label[e]}
        if not at:
            return False
    return True


def _mdd_cost(xs, frm, label, to, weight):
    """The cheapest accepting path's weight, or None where there is none."""
    costs = {1: 0}
    for x in xs:
        step = {}
        for e in range(len(frm)):
            if frm[e] in costs and x in label[e]:
                cost = costs[frm[e]] + weight[e]
                step[to[e]] = min(cost, step.get(to[e], cost))
        if not step:
            return None
        costs = step
    return min(costs.values())


def _circuit(xs, offset):
    """A permutation of the nodes whose cycle through the first covers them all."""
    nodes = list(range(offset, offset + len(xs)))
    if sorted(xs) != nodes:
        return False
    v = offset
    for _ in range(len(xs) - 1):
        v = xs[v - offset]
        if v == offset:  # back at the start too early, so the cycle is short
            return False
    return True


def _subcircuit(xs, offset):
    """A permutation whose nodes that move form one cycle; the rest are self loops."""
    nodes = list(range(offset, offset + len(xs)))
    if sorted(xs) != nodes:
        return False
    moved = [v for v in nodes if xs[v - offset] != v]
    if not moved:
        return True
    cycle, v = [], moved[0]
    while v not in cycle:
        cycle.append(v)
        v = xs[v - offset]
    return len(cycle) == len(moved)


CASES = (
    Case(
        ident="int_circuit",
        why="two nodes, where the swap is the only circuit and a self loop is not one",
        decisions=(Decision("x1", range(1, 3)), Decision("x2", range(1, 3))),
        constraint="fzn_int_circuit([x1, x2], 1)",
        holds=lambda a: _circuit([a["x1"], a["x2"]], 1),
    ),

    Case(
        ident="int_circuit",
        why="four nodes numbered from two, where two disjoint cycles are not one circuit",
        decisions=(
            Decision("x1", range(2, 6)),
            Decision("x2", range(2, 6)),
            Decision("x3", range(2, 6)),
            Decision("x4", range(2, 6)),
        ),
        constraint="fzn_int_circuit([x1, x2, x3, x4], 2)",
        holds=lambda a: _circuit([a["x1"], a["x2"], a["x3"], a["x4"]], 2),
    ),

    Case(
        ident="int_subcircuit",
        why="two nodes, where the empty subcircuit is as good as the full cycle",
        decisions=(Decision("x1", range(1, 3)), Decision("x2", range(1, 3))),
        constraint="fzn_int_subcircuit([x1, x2], 1)",
        holds=lambda a: _subcircuit([a["x1"], a["x2"]], 1),
    ),

    Case(
        ident="int_subcircuit",
        why="three nodes numbered from zero, where a node may sit out as a self loop",
        decisions=(
            Decision("x1", range(0, 3)),
            Decision("x2", range(0, 3)),
            Decision("x3", range(0, 3)),
        ),
        constraint="fzn_int_subcircuit([x1, x2, x3], 0)",
        holds=lambda a: _subcircuit([a["x1"], a["x2"], a["x3"]], 0),
    ),

    Case(
        ident="int_subcircuit",
        why="four nodes, where two disjoint two-cycles must be rejected",
        decisions=(
            Decision("x1", range(1, 5)),
            Decision("x2", range(1, 5)),
            Decision("x3", range(1, 5)),
            Decision("x4", range(1, 5)),
        ),
        constraint="fzn_int_subcircuit([x1, x2, x3, x4], 1)",
        holds=lambda a: _subcircuit([a["x1"], a["x2"], a["x3"], a["x4"]], 1),
    ),

    Case(
        ident="int_circuit_reif",
        why="a false reifying variable must allow the self loop the constraint forbids",
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            _bool("b"),
        ),
        constraint="fzn_int_circuit_reif([x1, x2], 1, b)",
        holds=lambda a: a["b"] == _circuit([a["x1"], a["x2"]], 1),
    ),

    Case(
        ident="int_circuit",
        why="half-reified, where a false antecedent constrains nothing",
        extra_includes=("fzn_int_circuit_reif.mzn",),
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            _bool("b"),
        ),
        constraint="b -> fzn_int_circuit([x1, x2], 1)",
        holds=lambda a: not a["b"] or _circuit([a["x1"], a["x2"]], 1),
    ),

    Case(
        ident="int_subcircuit_reif",
        why="a false reifying variable must allow two disjoint cycles",
        decisions=(
            Decision("x1", range(1, 5)),
            Decision("x2", range(1, 5)),
            Decision("x3", range(1, 5)),
            Decision("x4", range(1, 5)),
            _bool("b"),
        ),
        constraint="fzn_int_subcircuit_reif([x1, x2, x3, x4], 1, b)",
        holds=lambda a: a["b"]
        == _subcircuit([a["x1"], a["x2"], a["x3"], a["x4"]], 1),
    ),

    Case(
        ident="int_subcircuit",
        why="half-reified, where a false antecedent constrains nothing",
        extra_includes=("fzn_int_subcircuit_reif.mzn",),
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            _bool("b"),
        ),
        constraint="b -> fzn_int_subcircuit([x1, x2], 1)",
        holds=lambda a: not a["b"] or _subcircuit([a["x1"], a["x2"]], 1),
    ),

    Case(
        ident="graph_subgraph",
        why="two nodes, where the one edge needs both of its endpoints",
        decisions=(_bool("n1"), _bool("n2"), _bool("e1")),
        params=_graph(*EDGE12),
        constraint="fzn_graph_subgraph(FR, TO, 1, [n1, n2], [e1])",
        holds=lambda a: _subgraph(*EDGE12, 1, [a["n1"], a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_subgraph",
        why="a self loop needs only its own node, with the numbering from zero",
        decisions=(_bool("n0"), _bool("n1"), _bool("e1"), _bool("e2")),
        params=_graph((0, 0), (0, 1)),
        constraint="fzn_graph_subgraph(FR, TO, 0, [n0, n1], [e1, e2])",
        holds=lambda a: _subgraph(
            (0, 0), (0, 1), 0, [a["n0"], a["n1"]], [a["e1"], a["e2"]]
        ),
    ),

    Case(
        ident="graph_subgraph_reif",
        why="a false reifying variable must allow an edge whose endpoint is out",
        decisions=(_bool("n1"), _bool("n2"), _bool("e1"), _bool("b")),
        params=_graph(*EDGE12),
        constraint="fzn_graph_subgraph_reif(FR, TO, 1, [n1, n2], [e1], b)",
        holds=lambda a: a["b"] == _subgraph(*EDGE12, 1, [a["n1"], a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_subgraph",
        why="half-reified, where a false antecedent constrains nothing",
        decisions=(_bool("n1"), _bool("n2"), _bool("e1"), _bool("b")),
        params=_graph(*EDGE12),
        constraint="b -> fzn_graph_subgraph(FR, TO, 1, [n1, n2], [e1])",
        holds=lambda a: not a["b"]
        or _subgraph(*EDGE12, 1, [a["n1"], a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_connected",
        why="three nodes in a chain, where dropping the middle one parts the ends",
        decisions=(_bool("n2"), _bool("n3"), _bool("e1"), _bool("e2")),
        params=_graph(*CHAIN),
        constraint="fzn_graph_connected(FR, TO, 1, [true, n2, n3], [e1, e2])",
        holds=lambda a: _connected(
            *CHAIN, 1, [True, a["n2"], a["n3"]], [a["e1"], a["e2"]]
        ),
    ),

    Case(
        ident="graph_connected",
        why="the empty subgraph, which has no node to be the root",
        decisions=(_bool("n1"), _bool("n2"), _bool("e1")),
        params=_graph(*EDGE12),
        constraint="fzn_graph_connected(FR, TO, 1, [n1, n2], [e1])",
        holds=lambda a: _connected(*EDGE12, 1, [a["n1"], a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_connected_directed",
        why="two nodes, which one arc either way already connects",
        decisions=(_bool("n2"), _bool("e1"), _bool("e2")),
        params=_graph(*BOTH_WAYS),
        constraint="fzn_graph_connected_directed(FR, TO, 1, [true, n2], [e1, e2])",
        holds=lambda a: _connected_directed(
            *BOTH_WAYS, 1, [True, a["n2"]], [a["e1"], a["e2"]]
        ),
    ),

    Case(
        ident="graph_connected_directed",
        why="a two-cycle with a third node, which one of the two arcs leads to",
        decisions=(_bool("n2"), _bool("n3"), _bool("e1"), _bool("e2"), _bool("e3")),
        params=_graph(*LOLLIPOP),
        constraint="fzn_graph_connected_directed(FR, TO, 1, [true, n2, n3], [e1, e2, e3])",
        holds=lambda a: _connected_directed(
            *LOLLIPOP, 1, [True, a["n2"], a["n3"]], [a["e1"], a["e2"], a["e3"]]
        ),
    ),

    Case(
        ident="graph_connected_reif",
        why="a false reifying variable must allow two nodes and no edge",
        decisions=(_bool("n2"), _bool("e1"), _bool("b")),
        params=_graph(*EDGE12),
        constraint="fzn_graph_connected_reif(FR, TO, 1, [true, n2], [e1], b)",
        holds=lambda a: a["b"] == _connected(*EDGE12, 1, [True, a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_connected",
        why="half-reified, where a false antecedent constrains nothing",
        extra_includes=("fzn_graph_connected_reif.mzn",),
        decisions=(_bool("n2"), _bool("e1"), _bool("b")),
        params=_graph(*EDGE12),
        constraint="b -> fzn_graph_connected(FR, TO, 1, [true, n2], [e1])",
        holds=lambda a: not a["b"] or _connected(*EDGE12, 1, [True, a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_connected_directed_reif",
        why="a false reifying variable must allow a single arc between two nodes",
        decisions=(_bool("n2"), _bool("e1"), _bool("e2"), _bool("b")),
        params=_graph(*BOTH_WAYS),
        constraint="fzn_graph_connected_directed_reif(FR, TO, 1, [true, n2], [e1, e2], b)",
        holds=lambda a: a["b"]
        == _connected_directed(*BOTH_WAYS, 1, [True, a["n2"]], [a["e1"], a["e2"]]),
    ),

    Case(
        ident="graph_connected_directed",
        why="half-reified, where a false antecedent constrains nothing",
        extra_includes=("fzn_graph_connected_directed_reif.mzn",),
        decisions=(_bool("n2"), _bool("e1"), _bool("b")),
        params=_graph(*EDGE12),
        constraint="b -> fzn_graph_connected_directed(FR, TO, 1, [true, n2], [e1])",
        holds=lambda a: not a["b"]
        or _connected_directed(*EDGE12, 1, [True, a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_acyclic_directed_reif",
        why="a false reifying variable must allow the two-cycle",
        decisions=(_bool("n2"), _bool("e1"), _bool("e2"), _bool("b")),
        params=_graph(*BOTH_WAYS),
        constraint="fzn_graph_acyclic_directed_reif(FR, TO, 1, [true, n2], [e1, e2], b)",
        holds=lambda a: a["b"]
        == _acyclic_directed(*BOTH_WAYS, 1, [True, a["n2"]], [a["e1"], a["e2"]]),
    ),

    Case(
        ident="graph_reachable",
        why="three nodes with one isolated, which no root reaches",
        decisions=(Decision("r", range(1, 4)), _bool("n2"), _bool("n3"), _bool("e1")),
        params=_graph(*EDGE12),
        constraint="fzn_graph_reachable(FR, TO, 1, r, [true, n2, n3], [e1])",
        holds=lambda a: _reachable(
            *EDGE12, 1, a["r"], [True, a["n2"], a["n3"]], [a["e1"]]
        ),
    ),

    Case(
        ident="graph_reachable",
        why="an edge is reachable from either end, with the numbering from zero",
        decisions=(Decision("r", range(0, 2)), _bool("n1"), _bool("e1")),
        params=_graph((0,), (1,)),
        constraint="fzn_graph_reachable(FR, TO, 0, r, [true, n1], [e1])",
        holds=lambda a: _reachable((0,), (1,), 0, a["r"], [True, a["n1"]], [a["e1"]]),
    ),

    Case(
        ident="graph_reachable_reif",
        why="a false reifying variable must allow an unreachable node",
        decisions=(Decision("r", range(1, 3)), _bool("n2"), _bool("e1"), _bool("b")),
        params=_graph(*EDGE12),
        constraint="fzn_graph_reachable_reif(FR, TO, 1, r, [true, n2], [e1], b)",
        holds=lambda a: a["b"]
        == _reachable(*EDGE12, 1, a["r"], [True, a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_reachable",
        why="half-reified, where a false antecedent constrains nothing",
        decisions=(Decision("r", range(1, 3)), _bool("n2"), _bool("e1"), _bool("b")),
        params=_graph(*EDGE12),
        constraint="b -> fzn_graph_reachable(FR, TO, 1, r, [true, n2], [e1])",
        holds=lambda a: not a["b"]
        or _reachable(*EDGE12, 1, a["r"], [True, a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_reachable_directed",
        why="a chain of arcs, whose root has to be the tail",
        decisions=(
            Decision("r", range(1, 4)),
            _bool("n2"),
            _bool("n3"),
            _bool("e1"),
            _bool("e2"),
        ),
        params=_graph(*CHAIN),
        constraint="fzn_graph_reachable_directed(FR, TO, 1, r, [true, n2, n3], [e1, e2])",
        holds=lambda a: _reachable_directed(
            *CHAIN, 1, a["r"], [True, a["n2"], a["n3"]], [a["e1"], a["e2"]]
        ),
    ),

    Case(
        ident="graph_reachable_directed",
        why="half-reified, where a false antecedent constrains nothing",
        decisions=(Decision("r", range(1, 3)), _bool("n2"), _bool("e1"), _bool("b")),
        params=_graph(*EDGE12),
        constraint="b -> fzn_graph_reachable_directed(FR, TO, 1, r, [true, n2], [e1])",
        holds=lambda a: not a["b"]
        or _reachable_directed(*EDGE12, 1, a["r"], [True, a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_acyclic_directed",
        # Both nodes stay in, because an arc whose endpoint is out is outside
        # the well-formedness this constraint assumes rather than a cycle.
        why="a two-cycle is rejected, though either of its arcs alone is not",
        decisions=(_bool("e1"), _bool("e2")),
        params=_graph(*BOTH_WAYS),
        constraint="fzn_graph_acyclic_directed(FR, TO, 1, [true, true], [e1, e2])",
        holds=lambda a: _acyclic_directed(
            *BOTH_WAYS, 1, [True, True], [a["e1"], a["e2"]]
        ),
    ),

    Case(
        ident="graph_acyclic_directed",
        why="a self loop is a cycle of one",
        decisions=(_bool("e1"),),
        params=_graph(*LOOP),
        constraint="fzn_graph_acyclic_directed(FR, TO, 1, [true], [e1])",
        holds=lambda a: _acyclic_directed(*LOOP, 1, [True], [a["e1"]]),
    ),

    Case(
        ident="graph_acyclic_directed",
        why="half-reified, where a false antecedent constrains nothing",
        decisions=(_bool("e1"), _bool("e2"), _bool("b")),
        params=_graph(*BOTH_WAYS),
        constraint="b -> fzn_graph_acyclic_directed(FR, TO, 1, [true, true], [e1, e2])",
        holds=lambda a: not a["b"]
        or _acyclic_directed(*BOTH_WAYS, 1, [True, True], [a["e1"], a["e2"]]),
    ),

    Case(
        ident="graph_path",
        why="a triangle, where the third edge would close the path",
        decisions=(
            Decision("t", range(1, 4)),
            _bool("n2"),
            _bool("n3"),
            _bool("e1"),
            _bool("e2"),
            _bool("e3"),
        ),
        params=_graph(*TRIANGLE),
        constraint="fzn_graph_path(FR, TO, 1, 1, t, [true, n2, n3], [e1, e2, e3])",
        holds=lambda a: _path(
            *TRIANGLE,
            1,
            1,
            a["t"],
            [True, a["n2"], a["n3"]],
            [a["e1"], a["e2"], a["e3"]],
        ),
    ),

    Case(
        ident="graph_path",
        why="a duplicated edge, of which a path may take only one copy",
        decisions=(_bool("n2"), _bool("e1"), _bool("e2")),
        params=_graph(*TWIN),
        constraint="fzn_graph_path(FR, TO, 1, 1, 2, [true, n2], [e1, e2])",
        holds=lambda a: _path(*TWIN, 1, 1, 2, [True, a["n2"]], [a["e1"], a["e2"]]),
    ),

    Case(
        ident="graph_path_directed",
        why="an arc back into the source is rejected",
        decisions=(_bool("n2"), _bool("n3"), _bool("e1"), _bool("e2"), _bool("e3")),
        params=_graph(*LOLLIPOP),
        constraint="fzn_graph_path_directed(FR, TO, 1, 1, 3, [true, n2, n3], [e1, e2, e3])",
        holds=lambda a: _path_directed(
            *LOLLIPOP,
            1,
            1,
            3,
            [True, a["n2"], a["n3"]],
            [a["e1"], a["e2"], a["e3"]],
        ),
    ),

    Case(
        ident="graph_path_directed",
        why="a duplicated arc, of which a path may take only one copy",
        decisions=(_bool("n2"), _bool("e1"), _bool("e2")),
        params=_graph(*TWIN),
        constraint="fzn_graph_path_directed(FR, TO, 1, 1, 2, [true, n2], [e1, e2])",
        holds=lambda a: _path_directed(
            *TWIN, 1, 1, 2, [True, a["n2"]], [a["e1"], a["e2"]]
        ),
    ),

    Case(
        ident="graph_path_directed",
        why="half-reified, where a false antecedent constrains nothing",
        decisions=(_bool("n2"), _bool("e1"), _bool("b")),
        params=_graph((2,), (1,)),
        constraint="b -> fzn_graph_path_directed(FR, TO, 1, 1, 2, [true, n2], [e1])",
        holds=lambda a: not a["b"]
        or _path_directed((2,), (1,), 1, 1, 2, [True, a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_tree",
        why="a triangle has three spanning trees, and the cycle itself is not one",
        decisions=(_bool("n2"), _bool("n3"), _bool("e1"), _bool("e2"), _bool("e3")),
        params=_graph(*TRIANGLE),
        constraint="fzn_graph_tree(FR, TO, 1, 1, [true, n2, n3], [e1, e2, e3])",
        holds=lambda a: _tree(
            *TRIANGLE, 1, 1, [True, a["n2"], a["n3"]], [a["e1"], a["e2"], a["e3"]]
        ),
    ),

    Case(
        ident="graph_tree",
        why="a duplicated edge, where taking both copies is a cycle of two",
        decisions=(_bool("n2"), _bool("e1"), _bool("e2")),
        params=_graph(*TWIN),
        constraint="fzn_graph_tree(FR, TO, 1, 1, [true, n2], [e1, e2])",
        holds=lambda a: _tree(*TWIN, 1, 1, [True, a["n2"]], [a["e1"], a["e2"]]),
    ),

    Case(
        ident="graph_tree_directed",
        why="three nodes, where every arc must point away from the root",
        decisions=(_bool("n2"), _bool("n3"), _bool("e1"), _bool("e2"), _bool("e3")),
        params=_graph(*FORK),
        constraint="fzn_graph_tree_directed(FR, TO, 1, 1, [true, n2, n3], [e1, e2, e3])",
        holds=lambda a: _tree_directed(
            *FORK, 1, 1, [True, a["n2"], a["n3"]], [a["e1"], a["e2"], a["e3"]]
        ),
    ),

    Case(
        ident="graph_tree_directed",
        why="the root decides which arc of a pair may be taken",
        decisions=(Decision("r", range(1, 3)), _bool("e1"), _bool("e2")),
        params=_graph(*BOTH_WAYS),
        constraint="fzn_graph_tree_directed(FR, TO, 1, r, [true, true], [e1, e2])",
        holds=lambda a: _tree_directed(
            *BOTH_WAYS, 1, a["r"], [True, True], [a["e1"], a["e2"]]
        ),
    ),

    Case(
        ident="graph_tree_directed",
        why="half-reified, where a false antecedent constrains nothing",
        decisions=(Decision("r", range(1, 3)), _bool("n2"), _bool("e1"), _bool("b")),
        params=_graph(*EDGE12),
        constraint="b -> fzn_graph_tree_directed(FR, TO, 1, r, [true, n2], [e1])",
        holds=lambda a: not a["b"]
        or _tree_directed(*EDGE12, 1, a["r"], [True, a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_steiner_tree",
        why="which nodes to include is part of the decision, and each tree weighs its own",
        decisions=(
            _bool("n1"),
            _bool("n2"),
            _bool("n3"),
            _bool("e1"),
            _bool("e2"),
            _bool("e3"),
            Decision("w", range(0, 8)),
        ),
        params=_graph(*TRIANGLE, W=(1, 2, 4)),
        constraint="fzn_graph_steiner_tree(FR, TO, 1, W, [n1, n2, n3], [e1, e2, e3], w)",
        holds=lambda a: _tree_shape(
            *TRIANGLE, 1, [a["n1"], a["n2"], a["n3"]], [a["e1"], a["e2"], a["e3"]]
        )
        and a["w"] == _weight((1, 2, 4), [a["e1"], a["e2"], a["e3"]]),
    ),

    Case(
        ident="graph_steiner_tree_directed",
        why="the arcs chosen decide both the shape and the weight",
        decisions=(
            _bool("n2"),
            _bool("n3"),
            _bool("e1"),
            _bool("e2"),
            _bool("e3"),
            Decision("w", range(0, 8)),
        ),
        params=_graph(*FORK, W=(1, 2, 4)),
        constraint=(
            "fzn_graph_steiner_tree_directed(FR, TO, 1, W, 1, [true, n2, n3], [e1, e2, e3], w)"
        ),
        holds=lambda a: _tree_directed(
            *FORK, 1, 1, [True, a["n2"], a["n3"]], [a["e1"], a["e2"], a["e3"]]
        )
        and a["w"] == _weight((1, 2, 4), [a["e1"], a["e2"], a["e3"]]),
    ),

    Case(
        ident="graph_path_reif",
        why="a false reifying variable must allow a source with no edge at all",
        decisions=(_bool("n2"), _bool("e1"), _bool("b")),
        params=_graph(*EDGE12),
        constraint="fzn_graph_path_reif(FR, TO, 1, 1, 2, [true, n2], [e1], b)",
        holds=lambda a: a["b"]
        == _path(*EDGE12, 1, 1, 2, [True, a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_path",
        why="half-reified, where a false antecedent constrains nothing",
        extra_includes=("fzn_graph_path_reif.mzn",),
        decisions=(_bool("n2"), _bool("e1"), _bool("b")),
        params=_graph(*EDGE12),
        constraint="b -> fzn_graph_path(FR, TO, 1, 1, 2, [true, n2], [e1])",
        holds=lambda a: not a["b"]
        or _path(*EDGE12, 1, 1, 2, [True, a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_path_directed_reif",
        why="a false reifying variable must allow an arc against the path",
        decisions=(_bool("n2"), _bool("e1"), _bool("e2"), _bool("b")),
        params=_graph(*BOTH_WAYS),
        constraint="fzn_graph_path_directed_reif(FR, TO, 1, 1, 2, [true, n2], [e1, e2], b)",
        holds=lambda a: a["b"]
        == _path_directed(*BOTH_WAYS, 1, 1, 2, [True, a["n2"]], [a["e1"], a["e2"]]),
    ),

    Case(
        ident="graph_tree_reif",
        why="a false reifying variable must allow a root outside the subgraph",
        decisions=(_bool("n1"), _bool("n2"), _bool("e1"), _bool("b")),
        params=_graph(*EDGE12),
        constraint="fzn_graph_tree_reif(FR, TO, 1, 1, [n1, n2], [e1], b)",
        holds=lambda a: a["b"]
        == _tree(*EDGE12, 1, 1, [a["n1"], a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_tree",
        why="half-reified, where a false antecedent constrains nothing",
        extra_includes=("fzn_graph_tree_reif.mzn",),
        decisions=(_bool("n1"), _bool("n2"), _bool("e1"), _bool("b")),
        params=_graph(*EDGE12),
        constraint="b -> fzn_graph_tree(FR, TO, 1, 1, [n1, n2], [e1])",
        holds=lambda a: not a["b"]
        or _tree(*EDGE12, 1, 1, [a["n1"], a["n2"]], [a["e1"]]),
    ),

    Case(
        ident="graph_tree_directed_reif",
        why="a false reifying variable must allow an arc into the root",
        decisions=(_bool("n2"), _bool("e1"), _bool("e2"), _bool("b")),
        params=_graph(*BOTH_WAYS),
        constraint="fzn_graph_tree_directed_reif(FR, TO, 1, 1, [true, n2], [e1, e2], b)",
        holds=lambda a: a["b"]
        == _tree_directed(*BOTH_WAYS, 1, 1, [True, a["n2"]], [a["e1"], a["e2"]]),
    ),

    Case(
        ident="graph_spanning_tree",
        why="every node is covered, so the triangle's trees differ only in weight",
        decisions=(_bool("e1"), _bool("e2"), _bool("e3"), Decision("w", range(0, 8))),
        params=_graph(*TRIANGLE, W=(1, 2, 4)),
        constraint="fzn_graph_spanning_tree(FR, TO, 1, 3, W, [e1, e2, e3], w)",
        holds=lambda a: _tree_shape(
            *TRIANGLE, 1, [True, True, True], [a["e1"], a["e2"], a["e3"]]
        )
        and a["w"] == _weight((1, 2, 4), [a["e1"], a["e2"], a["e3"]]),
    ),

    Case(
        ident="graph_spanning_tree",
        why="a numbering from two with a node no edge touches, which nothing spans",
        decisions=(_bool("e1"), Decision("w", range(0, 2))),
        params=_graph((2,), (3,), W=(1,)),
        constraint="fzn_graph_spanning_tree(FR, TO, 2, 3, W, [e1], w)",
        holds=lambda a: _tree_shape((2,), (3,), 2, [True, True, True], [a["e1"]])
        and a["w"] == _weight((1,), [a["e1"]]),
    ),

    Case(
        ident="graph_spanning_tree_directed",
        why="the root picks its arc, over two nodes numbered from zero",
        decisions=(
            Decision("r", range(0, 2)),
            _bool("e1"),
            _bool("e2"),
            Decision("w", range(0, 4)),
        ),
        params=_graph((0, 1), (1, 0), W=(1, 2)),
        constraint="fzn_graph_spanning_tree_directed(FR, TO, 0, 2, W, r, [e1, e2], w)",
        holds=lambda a: _tree_directed(
            (0, 1), (1, 0), 0, a["r"], [True, True], [a["e1"], a["e2"]]
        )
        and a["w"] == _weight((1, 2), [a["e1"], a["e2"]]),
    ),

    Case(
        ident="graph_network_flow",
        why="one arc, the supply at its tail balancing the demand at its head",
        decisions=(Decision("f1", range(-1, 2)),),
        params=_graph(*EDGE12, B=(-1, 1)),
        constraint="fzn_graph_network_flow(FR, TO, 1, B, [f1])",
        holds=lambda a: _balanced(*EDGE12, 1, (-1, 1), [a["f1"]]),
    ),

    Case(
        ident="graph_network_flow",
        why="a node of no net balance passes its flow on, numbered from zero",
        decisions=(Decision("f1", range(0, 3)), Decision("f2", range(0, 3))),
        params=_graph((0, 1), (1, 2), B=(-1, 0, 1)),
        constraint="fzn_graph_network_flow(FR, TO, 0, B, [f1, f2])",
        holds=lambda a: _balanced(
            (0, 1), (1, 2), 0, (-1, 0, 1), [a["f1"], a["f2"]]
        ),
    ),

    Case(
        ident="graph_network_flow_reif",
        why="a false reifying variable must allow an unbalanced node",
        decisions=(Decision("f1", range(-1, 2)), _bool("b")),
        params=_graph(*EDGE12, B=(-1, 1)),
        constraint="fzn_graph_network_flow_reif(FR, TO, 1, B, [f1], b)",
        holds=lambda a: a["b"] == _balanced(*EDGE12, 1, (-1, 1), [a["f1"]]),
    ),

    Case(
        ident="graph_network_flow",
        why="half-reified, where a false antecedent constrains nothing",
        decisions=(Decision("f1", range(-1, 2)), _bool("b")),
        params=_graph(*EDGE12, B=(-1, 1)),
        constraint="b -> fzn_graph_network_flow(FR, TO, 1, B, [f1])",
        holds=lambda a: not a["b"] or _balanced(*EDGE12, 1, (-1, 1), [a["f1"]]),
    ),

    Case(
        ident="graph_network_flow_cost",
        why="parallel arcs of different cost, either of which meets the demand",
        decisions=(
            Decision("f1", range(0, 2)),
            Decision("f2", range(0, 2)),
            Decision("c", range(0, 4)),
        ),
        params=_graph(*TWIN, B=(-1, 1), W=(1, 2)),
        constraint="fzn_graph_network_flow_cost(FR, TO, 1, B, W, [f1, f2], c)",
        holds=lambda a: _balanced(*TWIN, 1, (-1, 1), [a["f1"], a["f2"]])
        and a["c"] == a["f1"] * 1 + a["f2"] * 2,
    ),

    Case(
        ident="int_table",
        why="a row given twice, and a value no row allows",
        decisions=(Decision("x1", range(0, 3)), Decision("x2", range(0, 3))),
        params="array [1..6] of int: TUP = [0, 1, 0, 1, 2, 0];\n",
        constraint="fzn_int_table([x1, x2], TUP)",
        holds=lambda a: _table([a["x1"], a["x2"]], (0, 1, 0, 1, 2, 0)),
    ),

    Case(
        ident="int_table",
        why="a table of one column over negative values",
        decisions=(Decision("x1", range(-1, 2)),),
        params="array [1..2] of int: TUP = [-1, 1];\n",
        constraint="fzn_int_table([x1], TUP)",
        holds=lambda a: _table([a["x1"]], (-1, 1)),
    ),

    Case(
        ident="int_table_reif",
        why="a false reifying variable must forbid every row",
        decisions=(Decision("x1", range(0, 3)), Decision("x2", range(0, 3)), _bool("b")),
        params="array [1..6] of int: TUP = [0, 1, 0, 1, 2, 0];\n",
        constraint="fzn_int_table_reif([x1, x2], TUP, b)",
        holds=lambda a: a["b"] == _table([a["x1"], a["x2"]], (0, 1, 0, 1, 2, 0)),
    ),

    Case(
        ident="int_table",
        why="half-reified, where a false antecedent constrains nothing",
        decisions=(Decision("x1", range(0, 3)), Decision("x2", range(0, 3)), _bool("b")),
        params="array [1..6] of int: TUP = [0, 1, 0, 1, 2, 0];\n",
        constraint="b -> fzn_int_table([x1, x2], TUP)",
        holds=lambda a: not a["b"] or _table([a["x1"], a["x2"]], (0, 1, 0, 1, 2, 0)),
    ),

    Case(
        ident="bool_table",
        why="a column every row fixes false",
        decisions=(_bool("x1"), _bool("x2")),
        params="array [1..4] of bool: TUP = [true, false, false, false];\n",
        constraint="fzn_bool_table([x1, x2], TUP)",
        holds=lambda a: _table([a["x1"], a["x2"]], (True, False, False, False)),
    ),

    Case(
        ident="bool_table_reif",
        why="a false reifying variable must forbid every row",
        decisions=(_bool("x1"), _bool("x2"), _bool("b")),
        params="array [1..4] of bool: TUP = [true, false, false, false];\n",
        constraint="fzn_bool_table_reif([x1, x2], TUP, b)",
        holds=lambda a: a["b"] == _table([a["x1"], a["x2"]], (True, False, False, False)),
    ),

    Case(
        ident="bool_table",
        why="half-reified, where a false antecedent constrains nothing",
        decisions=(_bool("x1"), _bool("x2"), _bool("b")),
        params="array [1..4] of bool: TUP = [true, false, false, false];\n",
        constraint="b -> fzn_bool_table([x1, x2], TUP)",
        holds=lambda a: not a["b"]
        or _table([a["x1"], a["x2"]], (True, False, False, False)),
    ),

    Case(
        ident="int_regular",
        why="the automaton counts the twos, so only an even count is accepted",
        decisions=(Decision("x1", range(1, 3)), Decision("x2", range(1, 3))),
        params="array [1..4] of int: TR = [1, 2, 2, 1];\n",
        constraint="fzn_int_regular([x1, x2], 2, 2, 1, TR, 1, {1})",
        holds=lambda a: _dfa([a["x1"], a["x2"]], 2, 2, 1, (1, 2, 2, 1), 1, {1}),
    ),

    Case(
        ident="int_regular",
        why="a transition to zero rejects outright, with symbols numbered from zero",
        decisions=(Decision("x1", range(0, 2)), Decision("x2", range(0, 2))),
        params="array [1..4] of int: TR = [2, 0, 2, 1];\n",
        constraint="fzn_int_regular([x1, x2], 2, 2, 0, TR, 1, {1})",
        holds=lambda a: _dfa([a["x1"], a["x2"]], 2, 2, 0, (2, 0, 2, 1), 1, {1}),
    ),

    Case(
        ident="int_regular_reif",
        why="a false reifying variable must allow a string the automaton rejects",
        decisions=(Decision("x1", range(1, 3)), Decision("x2", range(1, 3)), _bool("b")),
        params="array [1..4] of int: TR = [1, 2, 2, 1];\n",
        constraint="fzn_int_regular_reif([x1, x2], 2, 2, 1, TR, 1, {1}, b)",
        holds=lambda a: a["b"]
        == _dfa([a["x1"], a["x2"]], 2, 2, 1, (1, 2, 2, 1), 1, {1}),
    ),

    Case(
        ident="int_regular",
        why="half-reified, where a false antecedent constrains nothing",
        decisions=(Decision("x1", range(1, 3)), Decision("x2", range(1, 3)), _bool("b")),
        params="array [1..4] of int: TR = [1, 2, 2, 1];\n",
        constraint="b -> fzn_int_regular([x1, x2], 2, 2, 1, TR, 1, {1})",
        holds=lambda a: not a["b"]
        or _dfa([a["x1"], a["x2"]], 2, 2, 1, (1, 2, 2, 1), 1, {1}),
        extra_includes=("fzn_int_regular_reif.mzn",),
    ),

    Case(
        ident="int_regular_nondeterministic",
        why="two states carry the same symbol, and an empty transition set kills a path",
        decisions=(Decision("x1", range(1, 3)), Decision("x2", range(1, 3))),
        params="array [1..4] of set of int: TR = [{1, 2}, {1}, {}, {2}];\n",
        constraint="fzn_int_regular_nondeterministic([x1, x2], 2, 2, 1, TR, 1, {2})",
        holds=lambda a: _nfa(
            [a["x1"], a["x2"]], 2, 2, 1, ({1, 2}, {1}, set(), {2}), 1, {2}
        ),
    ),

    Case(
        ident="int_regular_nondeterministic",
        why="half-reified, where a false antecedent constrains nothing",
        decisions=(Decision("x1", range(1, 3)), Decision("x2", range(1, 3)), _bool("b")),
        params="array [1..4] of set of int: TR = [{1, 2}, {1}, {}, {2}];\n",
        constraint="b -> fzn_int_regular_nondeterministic([x1, x2], 2, 2, 1, TR, 1, {2})",
        holds=lambda a: not a["b"]
        or _nfa([a["x1"], a["x2"]], 2, 2, 1, ({1, 2}, {1}, set(), {2}), 1, {2}),
    ),

    Case(
        ident="int_regular_nondeterministic_reif",
        why="a false reifying variable must allow a string no path accepts",
        decisions=(Decision("x1", range(1, 3)), Decision("x2", range(1, 3)), _bool("b")),
        params="array [1..4] of set of int: TR = [{1, 2}, {1}, {}, {2}];\n",
        constraint="fzn_int_regular_nondeterministic_reif([x1, x2], 2, 2, 1, TR, 1, {2}, b)",
        holds=lambda a: a["b"]
        == _nfa([a["x1"], a["x2"]], 2, 2, 1, ({1, 2}, {1}, set(), {2}), 1, {2}),
    ),

    Case(
        ident="int_regular_cost",
        why="the cost counts the twos the automaton reads",
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            Decision("c", range(0, 3)),
        ),
        params="array [1..4] of int: TR = [1, 2, 2, 1];\narray [1..4] of int: WT = [0, 1, 0, 1];\n",
        constraint="fzn_int_regular_cost([x1, x2], 2, 2, 1, TR, 1, {1}, WT, c)",
        holds=lambda a: a["c"]
        == _dfa_cost([a["x1"], a["x2"]], 2, 2, 1, (1, 2, 2, 1), 1, {1}, (0, 1, 0, 1)),
    ),

    Case(
        ident="int_mdd",
        why="two levels, where the first value decides which node the second leaves",
        decisions=(Decision("x1", range(1, 3)), Decision("x2", range(1, 3))),
        params=(
            "array [1..3] of int: LV = [1, 2, 2];\n"
            "array [1..4] of int: MF = [1, 1, 2, 3];\n"
            "array [1..4] of set of int: LB = [{1}, {2}, {1}, {2}];\n"
            "array [1..4] of int: MT = [2, 3, 0, 0];\n"
        ),
        constraint="fzn_int_mdd([x1, x2], 3, LV, 4, MF, LB, MT)",
        holds=lambda a: _mdd(
            [a["x1"], a["x2"]], (1, 1, 2, 3), ({1}, {2}, {1}, {2}), (2, 3, 0, 0)
        ),
    ),

    Case(
        ident="int_mdd_reif",
        why="a false reifying variable must allow an assignment off every path",
        decisions=(Decision("x1", range(1, 3)), Decision("x2", range(1, 3)), _bool("b")),
        params=(
            "array [1..3] of int: LV = [1, 2, 2];\n"
            "array [1..4] of int: MF = [1, 1, 2, 3];\n"
            "array [1..4] of set of int: LB = [{1}, {2}, {1}, {2}];\n"
            "array [1..4] of int: MT = [2, 3, 0, 0];\n"
        ),
        constraint="fzn_int_mdd_reif([x1, x2], 3, LV, 4, MF, LB, MT, b)",
        holds=lambda a: a["b"]
        == _mdd([a["x1"], a["x2"]], (1, 1, 2, 3), ({1}, {2}, {1}, {2}), (2, 3, 0, 0)),
    ),

    Case(
        ident="int_mdd",
        why="half-reified, where a false antecedent constrains nothing",
        decisions=(Decision("x1", range(1, 3)), Decision("x2", range(1, 3)), _bool("b")),
        params=(
            "array [1..3] of int: LV = [1, 2, 2];\n"
            "array [1..4] of int: MF = [1, 1, 2, 3];\n"
            "array [1..4] of set of int: LB = [{1}, {2}, {1}, {2}];\n"
            "array [1..4] of int: MT = [2, 3, 0, 0];\n"
        ),
        constraint="b -> fzn_int_mdd([x1, x2], 3, LV, 4, MF, LB, MT)",
        holds=lambda a: not a["b"]
        or _mdd([a["x1"], a["x2"]], (1, 1, 2, 3), ({1}, {2}, {1}, {2}), (2, 3, 0, 0)),
    ),

    Case(
        ident="int_mdd_nondeterministic",
        why="one label leaves the root twice, so two paths accept the same first value",
        decisions=(Decision("x1", range(1, 3)), Decision("x2", range(1, 3))),
        params=(
            "array [1..3] of int: LV = [1, 2, 2];\n"
            "array [1..5] of int: MF = [1, 1, 2, 3, 1];\n"
            "array [1..5] of set of int: LB = [{1}, {2}, {1}, {2}, {1}];\n"
            "array [1..5] of int: MT = [2, 3, 0, 0, 3];\n"
        ),
        constraint="fzn_int_mdd_nondeterministic([x1, x2], 3, LV, 5, MF, LB, MT)",
        holds=lambda a: _mdd(
            [a["x1"], a["x2"]],
            (1, 1, 2, 3, 1),
            ({1}, {2}, {1}, {2}, {1}),
            (2, 3, 0, 0, 3),
        ),
    ),

    Case(
        ident="int_mdd_nondeterministic_reif",
        why="a false reifying variable must allow an assignment off every path",
        decisions=(Decision("x1", range(1, 3)), Decision("x2", range(1, 3)), _bool("b")),
        params=(
            "array [1..3] of int: LV = [1, 2, 2];\n"
            "array [1..5] of int: MF = [1, 1, 2, 3, 1];\n"
            "array [1..5] of set of int: LB = [{1}, {2}, {1}, {2}, {1}];\n"
            "array [1..5] of int: MT = [2, 3, 0, 0, 3];\n"
        ),
        constraint="fzn_int_mdd_nondeterministic_reif([x1, x2], 3, LV, 5, MF, LB, MT, b)",
        holds=lambda a: a["b"]
        == _mdd(
            [a["x1"], a["x2"]],
            (1, 1, 2, 3, 1),
            ({1}, {2}, {1}, {2}, {1}),
            (2, 3, 0, 0, 3),
        ),
    ),

    Case(
        ident="int_mdd_nondeterministic",
        why="half-reified, where a false antecedent constrains nothing",
        decisions=(Decision("x1", range(1, 3)), Decision("x2", range(1, 3)), _bool("b")),
        params=(
            "array [1..3] of int: LV = [1, 2, 2];\n"
            "array [1..5] of int: MF = [1, 1, 2, 3, 1];\n"
            "array [1..5] of set of int: LB = [{1}, {2}, {1}, {2}, {1}];\n"
            "array [1..5] of int: MT = [2, 3, 0, 0, 3];\n"
        ),
        constraint="b -> fzn_int_mdd_nondeterministic([x1, x2], 3, LV, 5, MF, LB, MT)",
        holds=lambda a: not a["b"]
        or _mdd(
            [a["x1"], a["x2"]],
            (1, 1, 2, 3, 1),
            ({1}, {2}, {1}, {2}, {1}),
            (2, 3, 0, 0, 3),
        ),
    ),

    Case(
        ident="int_mdd_cost",
        why="the weight of every edge on the path adds up",
        decisions=(
            Decision("x1", range(1, 3)),
            Decision("x2", range(1, 3)),
            Decision("c", range(0, 12)),
        ),
        params=(
            "array [1..3] of int: LV = [1, 2, 2];\n"
            "array [1..4] of int: MF = [1, 1, 2, 3];\n"
            "array [1..4] of set of int: LB = [{1}, {2}, {1}, {2}];\n"
            "array [1..4] of int: WT = [1, 2, 4, 8];\n"
            "array [1..4] of int: MT = [2, 3, 0, 0];\n"
        ),
        constraint="fzn_int_mdd_cost([x1, x2], 3, LV, 4, MF, LB, WT, MT, c)",
        holds=lambda a: a["c"]
        == _mdd_cost(
            [a["x1"], a["x2"]],
            (1, 1, 2, 3),
            ({1}, {2}, {1}, {2}),
            (2, 3, 0, 0),
            (1, 2, 4, 8),
        ),
    ),
)
