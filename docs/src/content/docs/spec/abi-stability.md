---
title: ABI stability
description: What FZNSO_ABI_VERSION guarantees, when it changes, and how solvers built for different revisions coexist.
sidebar:
  order: 10
---

The protocol carries a single version number, `FZNSO_ABI_VERSION`, currently **1**.

Every solver exports it as `fznso_<name>_abi_version`.
Every application checks it **before calling any other entry point**, and rejects the solver outright if it differs from its own.

## Exact match, in both directions

There is no forward compatibility and no backward compatibility.
Version 1 and version 2 do not interoperate, in either direction.

This is deliberate.
The interface is a set of `struct`s with function pointers at fixed offsets; a mismatch is not a missing feature but a call through the wrong pointer.
Detecting it cheaply and refusing is the only safe behaviour, and it costs one symbol lookup per library.

Because the check happens per candidate rather than per directory, solvers built for different revisions can share an installation directory.
A loader resolving a solver by name [skips a candidate](../discovery/) whose version does not match and continues the search, so upgrading one solver never breaks the others.

## What forces a new version

Any change that alters the meaning or layout of what crosses the interface:

- adding, removing or reordering a field in `FznsoModelMethods`, `FznsoValueMethods`, `FznsoSolutionMethods` or `FznsoAnnotationMethods`, or any other `struct` in `fznso_types.h`;
- adding, removing or reordering a variant of `FznsoValueKind`, `FznsoTypeBase` or `FznsoStatus`;
- adding, removing or changing the signature of any of the [thirteen entry points](../entry-points/);
- changing the contract of an existing callback — when it may be called, what it may return, what the caller may do with the result.

## What does not

- **Anything in the [registry](../../registry/).**
  Adding a constraint, option, objective, statistic or message scope changes no layout and no signature.
  A solver that does not implement a new registry entry simply does not declare it, and applications discover that through the capability lists.
- **A solver's own release version.**
  See [Naming & versioning](../naming-and-versioning/#the-version-says-nothing-about-protocol-compatibility).
- **Documentation that clarifies without changing behaviour.**

## For implementers

Return the constant from the headers you built against, rather than a literal:

```c
uint32_t fznso_mysolver_abi_version(void) { return FZNSO_ABI_VERSION; }
```

Then a rebuild against a newer `fznso_types.h` reports the new version automatically, and the mismatch is caught at load time rather than becoming a crash inside a callback.

The Rust `fznso_export!` macro and the C++ `FZNSO_EXPORT_SOLVER` macro both do this for you.
