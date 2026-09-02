---
title: What is FZnSO?
description: What the FZnSO protocol is for, the problem it solves, and where the name comes from.
sidebar:
  order: 1
---

FZnSO lets an application drive a constraint or optimisation solver through a plain C interface, with the solver loaded at run time as an ordinary dynamically loadable library.

Two goals:

- **drive any solver the same way**, so that finding, loading, configuring and running one does not depend on which one it is; and
- **solve incrementally**, so that adding a constraint and solving again does not mean rebuilding and re-communicating the whole problem.

The *mechanism* is uniform; what a solver can do is not.
Solvers accept different constraints, objectives and options, and say which through their [capability lists](../../spec/entry-points/#the-library); moving a problem from a CP solver to a MIP solver still means reformulating.
What the [registry](../../registry/) removes is only the differences that carry no information — two solvers spelling the same time limit `time_limit` and `timeout`.

## The problem it solves

The established way to talk to a combinatorial solver is to flatten a model to FlatZinc, write it to a file, and run the solver as a subprocess.
Every solve then pays to serialise the problem, parse it back and start a process, and whatever the solver learned dies with that process.
For one solve of a large problem that is noise; for the loop that actually shows up in practice —

> solve, look at the answer, add a constraint, solve again

— it is most of the work, paid again on every iteration.

FZnSO removes all three costs: the solver runs in-process, the model is [read through callbacks](../concepts/) rather than serialised, and the instance survives between runs, so [layers](../../spec/model/#layers) tell it which part of the problem is new.

## Loaded, or linked

A solver is normally found by name at run time, which is what lets an application ship without knowing which solvers a user has installed.
But the entry points are plain C symbols, so a solver can equally be linked in ahead of time — which targets with no dynamic loader, WebAssembly above all, need.
Every symbol carries the solver's own name, so several can coexist in one binary; see [Naming & versioning](../../spec/naming-and-versioning/).

## What it is not

- **Not a library.**
  There is nothing to link against as a dependency; see [the two roles](../../spec/overview/#the-two-roles).
  The Rust, C++ and Python packages here are conveniences.
- **Not a modelling language.**
  FZnSO carries a flat problem: decision variables, constraints over them, an objective.
  Producing that from a high-level model is MiniZinc's job, or your own.
- **Not a replacement for FlatZinc the language.**
  The constraint vocabulary is deliberately the familiar one; see the [registry](../../registry/).

## The name

**F** + **Zn** + **SO**: FlatZinc over a shared object.
It also reads as a chemical formula — ZnSO₄, zinc sulfate, known to alchemists as *white vitriol*, hence Flat White Vitriol.
Everywhere else these pages say **dynamically loadable library**, since "shared object", "DLL" and "`.so`" each name one platform's spelling of the same thing.

Next: [Concepts](../concepts/) for the objects in the protocol, [Quickstart](../quickstart/) for running code.
