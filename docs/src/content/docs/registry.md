---
title: About the registry
description: What the common names are for, what declaring one commits you to, and how to name anything the registry does not cover.
---

FZnSO lets a solver declare any names it likes for its constraints, options, objectives and statistics.
Nothing in the protocol constrains them.

That freedom is also the problem.
Solvers genuinely differ in what they can do: a MIP solver and a SAT solver accept different constraints, and no vocabulary can paper over that.
But there is no reason for them to differ in what they *call* a time limit, and an application that has to handle `time_limit` on one solver and `timeout` on the next is paying for a difference that carries no information.
The registry is the agreed vocabulary that removes those.

## What it is

Five lists of names with a fixed meaning:

- **Constraints** — the largest list, split by purpose:
  [arithmetic](constraints/arithmetic/), [array](constraints/array/), [set](constraints/set/),
  [ordering & counting](constraints/ordering/), [scheduling & packing](constraints/scheduling/),
  and [graph & extensional](constraints/graph/).
- [Objective strategies](objectives/) — how a solver should optimise.
- [Options](options/) — how a solver should be configured.
- [Statistics](statistics/) — what a solver can report about its search.
- [Message scopes](message-scopes/) — the kinds of diagnostic a solver can emit while running.

## What it is not

**Not mandatory.**
A solver implements what it can.
Declaring nothing at all is [conforming](../spec/overview/#conformance), just not useful.

**Not part of the ABI.**
Adding a registry entry never changes a struct layout or a signature, so it never bumps [`FZNSO_ABI_VERSION`](../spec/abi-stability/).
An application discovers what a solver supports at run time through the capability lists, not at compile time.

## How a constraint entry reads

Every constraint gives three things, and they say different kinds of thing:

- a **signature**, naming each argument as well as typing it, so the rest of the entry has something
  to refer to. Arguments are written `type: name`, as MiniZinc declares a predicate;
- a **description** in prose, which is what the constraint is *for*;
- a **definition** as a formula, which is what it *means*. Where the two ever seem to disagree, the
  formula is the claim a solver is measured against.

### Naming

An identifier means one thing, so a constraint that exists for several types is several
identifiers, each named for the type it takes:

| | |
| --- | --- |
| `int_` | `int_lin_le`, `int_all_different`, `int_cumulative` |
| `bool_` | `bool_clause`, `bool_lex_lesseq` |
| `float_` | `float_lin_lt`, `float_array_maximum` |
| `set_of_int_` | `set_of_int_subset`, `set_of_int_all_different` |

Array constraints put the element type first as well: `int_array_element`, not `array_int_element`.

The graph constraints are the exception. What they constrain is the shape a subgraph takes, not the
type of anything in it, so they share a `graph_` prefix instead: `graph_path`, `graph_tree`,
`graph_network_flow`.
Where one works on directed edges it takes a `_directed` suffix, so a constraint and its directed
form sort together.

### Signatures are the most permissive form

An entry's signature is the **widest** shape the constraint takes, so `int_in` types its set
argument `var set of int` even though most uses have a constant there.
A solver may then
[declare a narrower type](../spec/entry-points/#declaring-a-narrower-argument-type) — `set of int`,
meaning "only when the set is fixed" — and it is still the same constraint under the same
identifier.

So a signature here tells you what the name *means*; the solver's `constraint_list` tells you what
that solver will *accept*. Check the second before building a model.

### Shapes and offsets travel as arguments

Some arguments look like bookkeeping: `int: columns` and `int: dimensions` give a flattened matrix
its row length, and `int: offset` — named for its array, and always immediately after it — gives
that array the number it is indexed from.
They are the part of the type that a [flat, one-based
list](../spec/types-and-values/#a-list-is-flat-and-one-based) cannot carry itself.

### The result comes last

Where a constraint [computes a value](#reified-and-implied-forms), that value is its **last**
argument — `int_count(xs, value, n)`, `int_array_element(xs, offset, index, value)` — so the inputs
come first and what the solver has to work out comes at the end.
The offset rule above outranks it: an offset stays with its array even when that array is the
result, which is why `int_bin_packing_load` ends `(…, load, offset)`.

### One constraint, one identifier

The list is meant to be comprehensive but not repetitive.
An entry is left out when it is **the same constraint in a different argument shape**, because
rewriting into the kept form costs nothing:

| Absent | Because it is | 
| --- | --- |
| `set_of_int_superset` | `set_of_int_subset` with the arguments swapped |
| `int_lex_greater` | `int_lex_less` with the arguments swapped |
| `int_decreasing` | `int_increasing` over the reversed array |
| `int_diffn` | renamed `int_no_overlap`, which says what it constrains |
| `int_le`, `int_plus` | `int_lin_le`, `int_lin_eq` with a coefficient list |
| `int_max`, `bool_and` | `int_array_maximum`, `bool_array_and` with a two-element list |
| `bool_array_or` | `bool_clause_reif` with an empty list of negative literals |
| `at_least`, `at_most`, `int_count_leq`, `int_count_geq` | `int_count` alongside an `int_lin_le` on the count |
| `int_count_eq` | renamed `int_count`: with no comparison forms left, the relation named nothing |
| `all_different_except_0` | `int_all_different_except` with `{0}` |
| `lex2` | `int_lex_chain_lesseq` on a matrix and on its transpose |

The rule reaches further once a constraint is [functional](#reified-and-implied-forms): its result
is a value, and comparing a value is what the linear constraints are for.
Hence one counting entry, and no `_eq` on its name — there is no sibling relation left to tell it
apart from.

An entry **stays** when the solution sets genuinely differ, even if one constraint implies the
other and even if the difference looks small.
`int_circuit` and `int_subcircuit` are both here, as are `int_global_cardinality` and its `_closed`
form, and `float_lin_lt` beside `float_lin_le` — there is no next float to shift a bound by.
`int_lin_le` and `bool_lin_le` stay on a different reading: a linear inequality is its own
propagator, not a sum computed and then compared.

### Reified and implied forms

**Every constraint that is not functional also names a reified form `_reif` and an implied form
`_imp`.** They take the base constraint's arguments followed by a Boolean `r`:

- `C_reif(…, r)` holds when `r ↔ C`;
- `C_imp(…, r)` holds when `r → C`, leaving `r` free when `C` holds.

Neither is written out anywhere: name and meaning both follow from the base entry, so a list would
be a second copy to keep in step.
A solver declares whichever of the three it supports.

A constraint is **functional** when one argument is a decision variable the others determine, so a
solver assigns it rather than testing it.
Reifying that says no more than a reified equality on the result would, so a functional entry names
neither form; the registry source marks them, being the shorter list.

What decides it is whether that argument is a *variable*.
`int_count(xs, value, n)` and `int_array_element` are functional; `int_lin_eq` is not, because its
`total` is a constant to compare against.
`bool_lin_eq`'s is a variable — nothing in a Boolean list can absorb an integer total, so it has
to name one — which is why two entries that look alike differ here.

## The one rule

> **If you declare a registry name, it must mean what the registry says.**

This is the whole contract.
A solver is free to implement `time_limit` or not; a solver that declares `time_limit` and treats it as seconds has broken the interface more thoroughly than one that never declared it, because nothing detects the difference.

If your solver's version of a concept differs from the registry's, do not reuse the name.
Give it your own, as below.

## Naming anything else

For anything the registry does not cover (a solver-specific search heuristic, a tuning knob, an internal counter), **prefix the name with your solver's own name**, the same one that appears in your entry points:

```text
gecode_afc_decay          ✓  a Gecode-specific option
chuffed_vsids             ✓  a Chuffed-specific option
restart_luby_scale        ✗  unprefixed, and not in the registry
time_limit_seconds        ✗  close enough to a registry name to be confusing
```

The prefix costs nothing and buys two things.
Two solvers can offer knobs with the same natural name without either having to give it up.
And an application reading a capability list can tell at a glance which names it can rely on across solvers and which tie it to one.

The same applies to [message scopes](message-scopes/), which are the one kind of name not declared up front: `gecode.propagation.trace` rather than `trace`.

## Proposing an entry

A name earns a registry entry when more than one solver would plausibly implement it with the same meaning.
If that is true of something your solver does, please propose it; see [Adding a registry entry](../contributing/registry-entry/).
