---
title: Entry points
description: The thirteen symbols every FZnSO solver library must export, and what each one is for.
sidebar:
  order: 2
---

A solver library exports exactly thirteen symbols.
Each is named `fznso_<name>_…`, where `<name>` is the library's own [name](../naming-and-versioning/): `fznso_gecode_solver_run` for a solver named `gecode`.

The `fznso_` marker is fixed; the name in the middle is what makes [linking](../overview/#linking) several solvers into one binary possible.

All thirteen are **mandatory**.
A loader looks every one of them up when opening a library, and a missing symbol fails the load.

## The library

These describe the library itself.
They take no solver instance, and may be called before one exists.

| Symbol | Returns | Purpose |
| --- | --- | --- |
| `fznso_<name>_abi_version` | `uint32_t` | The protocol revision this solver was built against. Checked first; see [ABI stability](../abi-stability/). |
| `fznso_<name>_constraint_list` | `FznsoConstraintList` | Constraint identifiers and argument types the solver accepts. |
| `fznso_<name>_decision_list` | `FznsoTypeList` | Decision-variable types the solver can create. |
| `fznso_<name>_objective_list` | `FznsoObjectiveList` | Objective strategies the solver can pursue. |
| `fznso_<name>_option_list` | `FznsoOptionList` | Options the solver accepts, with types and defaults. |
| `fznso_<name>_statistic_list` | `FznsoStatisticList` | Statistics the solver can report, and whether each comes from a solution, the solver, or both. |

The five lists are a solver's declaration of what it can do.
An application must not use a name that does not appear in them, and a solver must not accept one.
Where a name has a [registry](../../registry/) entry, declaring it means committing to the registry's meaning.

Every list is `static`: the arrays stay valid for as long as the library is loaded, and the caller never frees them.

### Declaring a narrower argument type

A capability list does not merely name what a solver accepts; it **types** it.
`FznsoConstraintType` carries an `arg_types` array, and those types are the solver's own answer, not
a copy of the [registry](../../registry/)'s signature.

A solver may declare an argument type **narrower** than the registry's, and that is a normal thing to
do rather than a deviation.
The registry gives each constraint its most permissive signature, so that every solver's real
capability is a specialisation of it.

The common case is a solver that handles an argument only as a parameter:

```text
registry     set_of_int_in(var int: x, var set of int: values)
a solver     set_of_int_in(var int: x, set of int: values)
```

That solver has not declared a different constraint.
It has declared the same constraint, restricted to the case where the set is fixed — which is what
its propagator can actually do.

**The obligation lands on the application.**
Having read `arg_types`, it must not pass a decision variable where the solver asked for a parameter.
The registry says what a name *means*; the capability list says what this solver will *accept*, and
the second is the one to check before building a model.

:::caution
Narrowing only. A solver must not declare an argument **wider** than the registry's signature, since
an application relying on the registry would then have no way to know the extra cases exist, and two
solvers declaring the same name would no longer agree on its meaning.
:::

Narrowing applies to every part of a type: `var int` to `int`, `var set of int` to `set of int`,
`opt int` to `int`. It is the same relation the [type qualifiers](../types-and-values/#types)
describe, read as "accepts fewer things".

## The solver instance

| Symbol | Purpose |
| --- | --- |
| `fznso_<name>_solver_create` | Create an instance. Returns `FznsoSolver *`. |
| `fznso_<name>_solver_free` | Destroy an instance and release everything it holds. |
| `fznso_<name>_solver_option_set` | Set one option. Returns `false` on failure. |
| `fznso_<name>_solver_option_get` | Read one option's current value. |
| `fznso_<name>_solver_statistic` | Read one statistic from the instance. |
| `fznso_<name>_solver_read_error` | Hand the most recent error message to a callback. |
| `fznso_<name>_solver_run` | Search. See [Running a search](../running/). |

An instance is created, configured, run any number of times, and freed.
Options may be changed between runs.
Only `fznso_<name>_solver_run` is expected to take a long time.

### Errors

Two entry points can fail in a way that carries a message: `option_set` returns `false`, and `run` returns [`FznsoError`](../running/#status).
Neither returns the reason directly.
The caller then calls `fznso_<name>_solver_read_error`, which passes the message to a callback rather than returning it, so the message need only stay valid for the duration of that callback.

This is the only error channel.
The `on_message` callback during a run [is not for failures](../running/#diagnostics).

## The template

The canonical declaration of all thirteen, with the full contract on each, is the solver template:

- [ABI entry points](../../reference/abi-entry-points/) — the contract on each, generated from the template
- `c/fznso_solver_template.c` — the C template to copy, generated by cbindgen
- [`FZNSO_EXPORT_SOLVER`](../../reference/cpp-export/) — C++, which writes all thirteen for you
- [`fznso_export!`](../../reference/rust/#fznso_export) — Rust, likewise

Copy the template and fill it in; it is not a library to link against.
