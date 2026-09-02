---
title: Overview & conformance
description: The two roles in the FZnSO protocol, what each must provide, and what conformance means.
sidebar:
  order: 1
---

FZnSO is an application binary interface, not a library you link against.
Both sides can be written in any language with a C FFI, and neither needs to have heard of the other at build time.

## The two roles

**A solver** is a dynamically loadable library exporting the [thirteen entry points](../entry-points/), named `fznso_<name>_…` after the library's own [name](../naming-and-versioning/).
It declares what it can do through five capability lists, creates solver instances on request, and searches when asked.

**An application** loads a solver library, checks its ABI version, sets options, and calls `fznso_<name>_solver_run` with a [model](../model/) — a callback table the solver reads through while it searches, on however many threads it searches with.
Nothing is serialised and nothing is copied.

## Conformance

A **conforming solver** must:

1. export all thirteen entry points, using its own name in each symbol;
2. return [`FZNSO_ABI_VERSION`](../abi-stability/) from `fznso_<name>_abi_version`;
3. honour the [lifetime and ownership rules](../lifetimes-and-ownership/) for everything it receives and everything it hands out;
4. honour the [threading contract](../threading/) if it searches on more than one thread;
5. accept only constraints, objectives and options it has declared in its capability lists.

A **conforming application** must:

1. [match the ABI version exactly](../lifetimes-and-ownership/#the-version-must-match-exactly), before calling anything else;
2. honour the same lifetime and ownership rules;
3. make its model safe to read concurrently, and its `should_stop` predicate safe to call concurrently, as the [threading contract](../threading/) requires;
4. not pass a constraint, objective or option the solver did not declare.

A solver need not support any particular constraint, objective or option.
One that declares five empty capability lists and answers every `run` with `FznsoComplete` and no solutions is conforming, just not useful.
What it must not do is accept a name it never declared, or declare a name whose meaning differs from the [registry](../../registry/)'s.

Four rules apply across the whole interface and are stated once, in [Lifetimes & ownership](../lifetimes-and-ownership/): handles are opaque, accessors have preconditions, nothing transfers ownership, and the ABI version must match exactly.
Read that page before implementing either side.
