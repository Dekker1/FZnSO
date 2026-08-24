---
title: Proposing an ABI change
description: What counts as a breaking change to the FZnSO interface, and what a proposal needs to cover.
sidebar:
  order: 3
---

The interface is a set of structs with function pointers at fixed offsets, matched [exactly](../../spec/abi-stability/) between solver and application.
There is no room for a compatible addition: a new field is a new layout, and a new layout is a new version.

That makes changes expensive (every solver must be rebuilt), so they are worth getting right rather than getting soon.

## Check first whether you need one

Several things that feel like interface changes are not:

| You want to | Do this instead |
| --- | --- |
| Add a constraint, option, objective or statistic | [Add a registry entry](../registry-entry/). No ABI change. |
| Report something new during a search | Use a [message scope](../../registry/message-scopes/). Scopes are not declared, so nothing needs to change. |
| Expose something solver-specific | Prefix it with your solver's name. No ABI change. |
| Pass richer data through an existing argument | Values are already recursive — a list of lists, a set, a string all fit through `FznsoValue`. |

The last one covers more than it looks.
If a proposal is really "I need to pass this extra thing", the answer is often an annotation or a structured value, neither of which touches the layout.

## What does need a new version

[ABI stability](../../spec/abi-stability/#what-forces-a-new-version) lists them: any change to a
struct layout, an enum's variants, an entry point's signature, or **the contract of an existing
callback**.

That last one is the easy one to miss.
Tightening "may be called from any thread" to "must be called from the calling thread" changes no bytes and breaks every solver relying on the old promise.
It is a version bump.

## What a proposal needs

**The problem, not the patch.**
What can you not express today?
A worked example of a solver or application that cannot be written is worth more than a diff.

**Why it cannot be a registry entry, an annotation or a value.**
Most proposals can be, and this is the question that decides it.

**The cost.**
Which side changes, what a solver must do to migrate, and whether an old solver can report the old version and keep working alongside. It can, since [candidates of the wrong version are skipped](../../spec/discovery/), so a transition does not have to be atomic.

**Both sides of the interface.**
A change that suits solvers and burdens every application, or the reverse, usually has a better shape.

## If it is accepted

A change lands as a coordinated set:

1. `rust/fznso-types` — the types and their doc comments, which are the contract;
2. `FZNSO_ABI_VERSION` incremented;
3. `just build` — regenerates `c/fznso_types.h` and `c/fznso_solver_template.c`;
4. the Rust, C++ and Python bindings;
5. the [specification pages](../../spec/overview/) — the normative prose lives there, not in the code comments;
6. the example solvers and every test suite.

Steps 1 and 5 are the ones that matter for anyone reading later.
Per-item contracts belong in the doc comment; rules that span the interface belong in the spec.
Writing the same paragraph in both is what the documentation is structured to avoid; see [Build & test](../build-and-test/#writing-documentation).
