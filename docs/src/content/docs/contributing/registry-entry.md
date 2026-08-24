---
title: Adding a registry entry
description: When a name belongs in the FZnSO registry, and how to propose one.
sidebar:
  order: 2
---

The [registry](../../registry/) is the agreed vocabulary that keeps two solvers from spelling the same concept differently.
Adding to it is cheap (it never changes the ABI), but a bad entry is expensive, because a name that means two things is worse than no name at all.

## When a name belongs here

Ask one question:

> Would a second solver plausibly implement this, with the same meaning?

If yes, propose it.
If the answer is "only mine would", give it [your solver's prefix](../../registry/#naming-anything-else) instead. That is the correct choice, not a fallback.

Then check it is not already here under a different shape.
An entry that is [the same constraint with different arguments](../../registry/#one-constraint-one-identifier) does not earn its own identifier, and one that exists for several types
needs [one identifier per type](../../registry/#naming).

Good candidates:

- a constraint with an established meaning in the field, so that two solvers would implement the same thing;
- an option two solvers already offer under different names;
- a statistic most solvers of a given class track anyway.

Not candidates:

- a knob specific to one solver's internals;
- a name that is nearly a registry name (`timeout` beside `time_limit`) — that creates confusion rather than resolving it;
- a constraint that is a kept one with the arguments swapped, a scalar where a list works, or a constant where the general form takes an argument.

## How to add one

Edit the TOML.
Nothing else is hand-written; the pages are generated.

```toml
# docs/registry/constraints/ordering.toml
[[entry]]
ident      = "all_different"
args       = ["list of var int: xs"]
reified    = ["all_different_reif"]           # optional
summary    = "No value occurs twice in the array."
definition = '\forall i < j : xs_i \neq xs_j'
notes      = "Optional. Undefinedness, index conventions, equal-length requirements."
```

Then `just docs` (or `pnpm --dir docs gen`) to regenerate.

Constraints **must** give `args`, `summary` and `definition`; the build fails without them.
Other categories differ slightly: `arg` for objectives, `type` and `default` for options, `type` for
statistics and message scopes.
The existing entries are the reference, and `scripts/gen-registry.mjs` is where the shapes are
defined.

### Which file

Constraints are split across six pages by purpose, so put the entry with the ones it resembles:
`docs/registry/constraints/{arithmetic,array,set,ordering,scheduling,graph}.toml`.
Adding a new page means dropping a TOML file in that directory with an `order` and a `label` in
its `[meta]`; nothing else needs changing, since the generator and the sidebar both read the
directory.

### Writing the definition

The `definition` is LaTeX, in a **single-quoted** TOML string so backslashes stay as written.

You do not need to mark up argument names: the generator wraps multi-letter names in `\mathit`, and
escapes the underscores inside them, so write `lower_bound` and `xs_i` plainly.

Four checks run before anything is rendered, and each fails the build:

- the formula must typeset — KaTeX runs with `throwOnError`;
- **every name it refers to must be one of the entry's arguments**, so a formula cannot drift from
  its signature when an argument is renamed;
- an operator name may not contain an unescaped underscore, which would silently become a subscript;
- no two entries may share an identifier.

Prefer stating the condition to naming another constraint.
`\operatorname{geost}` and an extra clause is not a definition; the extra clause alone is.

Only standard mathematics counts as a definition.
A formula whose claim sits inside `\text{…}` has not defined anything, and neither has one that
invokes an operator no one has defined.
Where a page's constraints genuinely share a symbol — reachability in a graph, a column of a matrix
passed flat — declare it once in that file's `[[notation]]`:

```toml
[[notation]]
tex = 'u \rightsquigarrow v'
means = "reachability: the reflexive-transitive closure of the adjacency above"
```

`is` is optional and holds the symbol's own definition, typeset beside it.
The table renders under the page's `## Notation` heading, through the same KaTeX as the definitions,
so a notation entry cannot look or read like a different language from the formulas that use it.
Argument names are marked up against every name declared on the page, so write them plainly here
too.

## What a good entry says

**`summary` is prose, not a formula.**
It says what the constraint is *for*, in a sentence or two; the formula belongs in `definition`.
"No value occurs twice in the array", not "∀i&lt;j : xs_i ≠ xs_j".

**`args` names as well as types each argument**, written `"type: name"` — MiniZinc's order, since
the types are written in MiniZinc's notation.
Types use [`FznsoType`](../../spec/types-and-values/#types) spellings: `list of var int`, not
`array[int] of var int`.
Remember that `var int` means the position *accepts* a variable; a constant is equally valid there.

Give arguments names that mean something — `start`, `duration`, `capacity`, `weight`,
`lower_bound` — and fall back to convention only where no real name exists: `xs` and `ys` for
arrays, `n` for a count, `r` for a reified result, `a`/`b`/`c` for the anonymous operands of a
primitive.

**`notes` covers what an implementer would otherwise get wrong**: undefined cases (division by zero), index conventions, interactions with other entries.
Leave it out if there is nothing to say.

**Say what it is for in `summary`, and what it means in `definition`.**
Prose that restates the formula is wasted, and a formula that restates the prose is not a definition.
`summary` should tell a reader why they would reach for this constraint; `definition` should be
precise enough to settle an argument about an edge case.

## Proposing a change to an existing entry

Changing what a name *means* breaks every solver that already declares it, silently: no version check catches it, because the [ABI version](../../spec/abi-stability/) does not cover the registry.

So: clarifications and better wording are welcome any time.
A change of meaning needs a new name.
