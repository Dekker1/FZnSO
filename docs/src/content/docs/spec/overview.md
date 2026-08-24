---
title: Overview & conformance
description: The two roles in the FZnSO protocol, what each must provide, and what conformance means.
sidebar:
  order: 1
---

FZnSO is an application binary interface, not a library you link against.
Both sides can be written in any language with a C FFI, and neither needs to have heard of the other at build time.

## The two roles

**A solver** is a dynamically loadable library that exports the [thirteen entry points](../entry-points/), named `fznso_<name>_…` after the library's own [name](../naming-and-versioning/).
It declares what it can do through five capability lists, creates solver instances on request, and searches when asked.

**An application** loads a solver library, checks its ABI version, sets options, and calls `fznso_<name>_solver_run` with a [model](../model/) — not a data structure it hands over, but a callback table the solver reads through while it searches, on however many threads it searches with.

Nothing is serialised and nothing is copied, which is [the point of the design](../../start/what-is-fznso/#the-problem-it-solves) rather than an optimisation of it.

## Linking

The usual arrangement is the one the name describes: a solver is a
[dynamically loadable library](../discovery/), found by name at run time, and an application that has
never heard of it can drive it.

**The interface also works statically linked**, and that is not an afterthought.
Some targets have no dynamic loader to speak of — WebAssembly is the obvious one — so a solver has to
be linked into the application ahead of time.
Nothing in the interface depends on the library being loaded rather than linked: the entry points are
plain C symbols, and an application that resolves them at build time rather than through a loader
sees exactly the same protocol.

This is why every symbol carries the solver's name.
Were the entry points simply `fznso_solver_run`, two solvers could never be linked into one binary,
because their symbols would collide at link time.
With `fznso_gecode_solver_run` and `fznso_chuffed_solver_run` they coexist, and an application can
ship a portfolio in a single executable.

The prefix earns its keep dynamically too — it keeps a solver's symbols out of the namespace of
whatever library it wraps, and lets a loader
[recover the name from the file name](../naming-and-versioning/) — but static linking is what makes
it necessary rather than merely tidy.

## Conformance

A **conforming solver** must:

1. export all thirteen entry points, using its own name in each symbol;
2. return [`FZNSO_ABI_VERSION`](../abi-stability/) from `fznso_<name>_abi_version`;
3. honour the [lifetime and ownership rules](../lifetimes-and-ownership/) for everything it receives and everything it hands out;
4. honour the [threading contract](../threading/) if it searches on more than one thread;
5. only accept constraints, objectives and options it has declared in its capability lists.

A solver **need not** support any particular constraint, objective or option.
A solver that declares five capability lists that are all empty, and answers every `run` with `FznsoComplete` and no solutions, is conforming, just not useful.
What it must not do is accept a name it never declared, or declare a name whose meaning differs from the [registry](../../registry/)'s if the registry defines one.

A **conforming application** must:

1. [match the ABI version exactly](../lifetimes-and-ownership/#the-version-must-match-exactly), before calling anything else;
2. honour the same lifetime and ownership rules;
3. make its model safe to read concurrently, and its `should_stop` predicate safe to call concurrently, as the [threading contract](../threading/) requires;
4. not pass a constraint, objective or option the solver did not declare.

## Rules that hold everywhere

Four rules apply across the whole interface and are stated once, in [Lifetimes & ownership](../lifetimes-and-ownership/), rather than repeated on every declaration:

- handles are opaque;
- accessors have preconditions;
- nothing transfers ownership;
- the ABI version must match exactly.

Read that page before implementing either side.
