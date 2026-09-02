---
title: Lifetimes & ownership
description: The four rules that hold across the whole FZnSO interface, covering opaque handles, accessor preconditions, borrowed data, and exact version matching.
sidebar:
  order: 9
---

These four rules apply to every declaration in the interface, and are stated here so that individual functions need only name their own preconditions.

## Handles are opaque

`FznsoModel`, `FznsoValue`, `FznsoSolution`, `FznsoAnnotation` and `FznsoSolver` are handles, meaningful only to the side that created one.
The receiver passes a handle back to the callbacks it arrived with, and never inspects or dereferences it.

A handle need not address memory at all: a small payload such as a decision index is commonly stored *in* the pointer word rather than behind it, so passing one across the interface never allocates, and a null handle is a legitimate value rather than an error.

Reach the contents through the `methods` table of the matching `…Ref` struct — `FznsoValueRef`, `FznsoModelRef`, `FznsoSolutionRef`, `FznsoAnnotationRef` — which pairs the handle with the callbacks that can read it.

:::caution
Dereferencing a handle, or casting it to anything but the type its creator used, is undefined behaviour.
This is why the Rust bindings model handles as `FznsoPtr<'a, T>`, a raw pointer with a lifetime, rather than as `&T`: a reference would carry permission to access only the opaque type itself, not the larger real type behind it.
:::

## Accessors have preconditions

Most `methods` entries are meaningful only in a particular state:

- a `get_*` on a value requires `kind()` to return the matching [`FznsoValueKind`](../types-and-values/#value-kinds);
- an index argument requires a value below the matching length callback.

**An implementation is not required to check.**
Violating a precondition may read nonsense, or abort the process.
Read `kind`, or the relevant length, first, and call only what the answer permits.

## Nothing transfers ownership

Every pointer, string and handle crossing the interface is borrowed.
The receiver never frees it, and must not keep it past the window in which it is valid:

| What | Valid for |
| --- | --- |
| A capability list (`fznso_<name>_constraint_list` and friends) | as long as the library is loaded |
| A value, string or annotation read from a model, solution or annotation | as long as that handle |
| A solution | the duration of the `on_solution` callback only |
| The scope and value passed to `on_message` | the duration of that callback only |
| The model passed to `fznso_<name>_solver_run` | the duration of that call |
| A value from `fznso_<name>_solver_option_get` or `…_solver_statistic` | until the next call that can change the solver |

**Copy anything that has to outlive its window.**
A consumer keeping a solution must copy the values out before returning from `on_solution`; a solver keeping part of a model must copy it before returning from `run`.

Strings are the common trap: [`FznsoStr`](../types-and-values/#strings) and the bytes behind `FznsoValueMethods::as_string` are **not** null-terminated, and are not yours.
Use the length, and copy before storing.

## The version must match exactly

An application rejects a solver whose `fznso_<name>_abi_version` differs from its own, before calling any other entry point.
There is no forward or backward compatibility between revisions.
See [ABI stability](../abi-stability/).
