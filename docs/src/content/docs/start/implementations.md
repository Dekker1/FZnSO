---
title: Implementations & tools
description: The known FZnSO bindings, solver libraries, applications, and tools, with where to find each one.
sidebar:
  order: 4
---

These are the FZnSO projects we know of.
Each one talks to the others only through the ABI, so any application below can load any solver library below.

## Bindings

Bindings present the ABI in the types and conventions of one language, so neither side writes ABI code by hand.
They live in the [FZnSO repository](https://github.com/Dekker1/FZnSO) (MIT).

| Binding | Applications | Solvers | Reference |
| --- | --- | --- | --- |
| Rust crate `fznso` (`rust/fznso`) | yes | yes | [Rust](../../reference/rust/) |
| C++17 headers `fznso.hpp` and `fznso_export.hpp` (`cpp/`) | yes | yes | [C++](../../reference/cpp/) |
| Python package `fznso` (`python/`) | yes | no | [Python](../../reference/python/) |
| C header `fznso_types.h` (`c/`) | yes | yes | [ABI types](../../reference/abi-types/) |

Any other language with a C foreign-function interface can use the C header directly.

## Solver libraries

| Solver | Paradigm | Repository | Licence |
| --- | --- | --- | --- |
| [Gecode](https://www.gecode.org) | constraint programming | [Dekker1/gecode](https://github.com/Dekker1/gecode/tree/feat/fznso), branch `feat/fznso` | MIT, as Gecode |
| [Huub](https://huub.solutions) | lazy clause generation | [Dekker1/huub](https://github.com/Dekker1/huub/tree/feat/fznso), branch `feat/fznso` | MPL-2.0, as Huub |
| [HiGHS](https://highs.dev) | mixed-integer programming | [Dekker1/FZnSO-MIP](https://github.com/Dekker1/FZnSO-MIP) | MIT |
| [Gurobi](https://www.gurobi.com) | mixed-integer programming | [Dekker1/FZnSO-MIP](https://github.com/Dekker1/FZnSO-MIP) | MIT; Gurobi itself needs a licence |
| [Z3](https://github.com/Z3Prover/z3) | satisfiability modulo theories | [Dekker1/FZnSO-Z3](https://github.com/Dekker1/FZnSO-Z3) | MIT |

The HiGHS and Gurobi libraries share one translation into a linear model, but each declares the capabilities of its own solver.
[Installing solvers](../../guides/installing-solvers/) explains where to put a library so that applications find it.

## Applications

| Application | Repository | Licence |
| --- | --- | --- |
| [MiniZinc](https://www.minizinc.org), through a generic FZnSO driver | [Dekker1/libminizinc](https://github.com/Dekker1/libminizinc/tree/feat/fznso), branch `feat/fznso` | MPL-2.0, as MiniZinc |

The MiniZinc driver loads any FZnSO solver library by name and uses its capability declarations to decide which constraints stay native and which are decomposed.

## Tools

These are in the [FZnSO repository](https://github.com/Dekker1/FZnSO) (MIT), under `tools/`.

| Tool | What it does | Guide |
| --- | --- | --- |
| `librecord` (solver `record`) | Sits in place of a solver, passes every call on, and writes a log | [Recording and replaying](../../guides/record-replay/) |
| `fznso-replay` | Replays the application's side of a log against a solver | [Recording and replaying](../../guides/record-replay/) |
| `libreplay` (solver `replay`) | Replays the solver's side of a log to an application, with no solver | [Recording and replaying](../../guides/record-replay/) |
| `fznso-conform` | Checks a solver library's declarations and answers against the [registry](../../registry/) | `tools/fznso-conform/README.md` |

## Add your project

If you build a binding, solver library, application, or tool for FZnSO, open a pull request that adds it to this page.
