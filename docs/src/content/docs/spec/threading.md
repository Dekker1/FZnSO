---
title: Threading
description: How a multi-threaded solver may interact with the callbacks and the model passed to fznso_solver_run.
sidebar:
  order: 8
---

A solver may search on several threads (see the [`threads`](../../registry/options/) option).
The protocol fixes how that interacts with the callbacks and the model passed to `fznso_<name>_solver_run`.

Only one of the three rules below places any obligation on the application.

## `on_solution` and `on_message` are called serially

Never two at once, and never one while the other runs, but either may be called from any thread, for example the worker that found a solution, as long as each call happens-before the next.

An application therefore never has to make its callbacks safe to call concurrently.
If it wants to process solutions in parallel it does so behind a serial callback, for example by pushing to a queue.

Serial reporting is also what keeps solutions in the improving order that [`intermediate`](../../registry/options/) and [`progress.bound`](../../registry/message-scopes/) rely on.

## The model may be read concurrently

The model is a **read-only** view for the duration of the call, and a parallel solver may query it from any number of threads at once.

**An application must therefore make its model safe for concurrent reads.**
In practice this means the `FznsoModelMethods` callbacks must not mutate anything observable: no lazily-filled caches without synchronisation, no returning pointers into a buffer that a later call overwrites.

## `should_stop` may be called concurrently

From any thread, and even while `on_solution` or `on_message` is running, every worker may poll it independently.

It is the one callback an application must make thread-safe, which the usual implementation (reading an atomic flag) already is.

## Summary

| | Called from | Concurrently? | Application must synchronise? |
| --- | --- | --- | --- |
| `on_solution` | any thread | no, serial with `on_message` | no |
| `on_message` | any thread | no, serial with `on_solution` | no |
| model accessors | any thread | yes | **yes** |
| `should_stop` | any thread | yes, incl. during the other callbacks | **yes** |
