---
title: What is FZnSO?
description: What the FZnSO protocol is for, what problem it solves, and where the name comes from.
sidebar:
  order: 1
---

FZnSO is a protocol that lets an application drive a constraint or optimisation solver through a plain C interface, with the solver loaded at run time as an ordinary dynamically loadable library.

Its two goals:

- **use different solvers in a unified way**, so that finding, loading, configuring and driving a solver works the same way whichever one it is; and
- **solve incrementally**, so that adding a constraint and solving again does not mean rebuilding and re-communicating the whole problem.

:::note[What "unified" does and does not mean]
It is the *mechanism* that is uniform, not the *model*.
Discovery, loading, the ABI check, the callback tables, the run lifecycle and the error channel are identical across solvers, and that is a large part of what makes writing against several of them practical at all.

What is not uniform is what a given solver can actually do.
Solvers support different constraints, different objective strategies and different options, and they say so through their [capability lists](../../spec/entry-points/#the-library).
Moving a problem from a CP solver to a MIP solver still means reformulating constraints it does not accept, and retuning the options it does.

The [registry](../../registry/) narrows this by removing *gratuitous* differences (two solvers should not spell the same time limit `time_limit` and `timeout`), but it cannot remove real ones.
:::

## The problem it solves

The established way to talk to a combinatorial solver is to flatten a model to FlatZinc, write it to a file, and run the solver as a subprocess.
That works, and it is why there are so many FlatZinc solvers.
It also means that every solve pays for serialising the problem, parsing it back, and starting a process, and that anything the solver learns dies when the process exits.

For a single solve of a large problem, that overhead is noise.
For the loop that actually shows up in practice:

> solve, look at the answer, add a constraint, solve again

the overhead is most of the work, and it is paid again on every iteration.

FZnSO removes all three costs at once.
The solver runs inside the process, so there is no subprocess.
The model is read through callbacks rather than serialised, so there is no text format and no parse.
And the solver keeps its instance between runs, so [layers](../../spec/model/#layers) let it see exactly which part of the problem is new.

## Loaded, or linked

Usually a solver is a library found by name at run time, which is what lets an application ship
without knowing which solvers a user has installed.
But the interface is only C symbols, so a solver can equally be **statically linked** — which is what
targets with no dynamic loader, WebAssembly above all, need.
See [Linking](../../spec/overview/#linking) for why that is what puts the solver's own name in every
symbol.

## What it is not

- **Not a library.**
  There is nothing to link against *as a dependency*; see [the two roles](../../spec/overview/#the-two-roles).
  The Rust, C++ and Python packages in this repository are conveniences, not the interface.
- **Not a modelling language.**
  FZnSO carries a flat problem: decision variables, constraints over them, an objective.
  Producing that from a high-level model is MiniZinc's job, or your own.
- **Not a replacement for FlatZinc the language.**
  The constraint vocabulary is deliberately the familiar one; see the [registry](../../registry/).

## Where the name comes from

**F** + **Zn** + **SO**: **F**lat**Z**i**n**c over a **s**hared **o**bject, the one place in this documentation where that term is used, because it is what the letters stand for.

It also reads as a chemical formula.
ZnSO₄, zinc sulfate, was known to alchemists as *white vitriol*, which is where the project's full name, Flat White Vitriol, comes from.

:::note[Terminology]
Everywhere else, the thing a solver ships as is called a **dynamically loadable library**.
"Shared object", "DLL" and "`.so`" each name one platform's spelling of it, and FZnSO targets all three equally.
:::

## Next

- [Concepts](../concepts/) — the objects in the protocol and how a session runs.
- [Quickstart](../quickstart/) — running code, from either side.
- [Specification](../../spec/overview/) — the exact rules.
