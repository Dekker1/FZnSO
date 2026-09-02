---
title: Running a search
description: The fznso_solver_run call, its callbacks, how a search is stopped early, what the status means, and how failures are reported.
sidebar:
  order: 7
---

```c
FznsoStatus fznso_NAME_solver_run(
    FznsoSolver *solver,
    FznsoModelRef model,
    void *context,
    void (*on_solution)(void *context, FznsoSolutionRef solution),
    void (*on_message)(void *context, FznsoStr scope, FznsoValueRef value),
    bool (*should_stop)(void *context));
```

One call performs one search.
`context` is passed back to every callback untouched; the solver never interprets it.
`model` is borrowed for the duration of the call and read-only throughout, as is everything the solver hands *out*; see [Lifetimes & ownership](../lifetimes-and-ownership/#nothing-transfers-ownership).

## Solutions

`on_solution` is called once per solution found, and is not optional.
The `FznsoSolutionRef` is valid **only for the duration of the callback**; copy out anything you want to [keep](../lifetimes-and-ownership/#nothing-transfers-ownership).

Which solutions are reported depends on the model's [objective](../model/#the-objective) and on the [`all_solutions` and `intermediate`](../../registry/options/) options:

- with no objective, every solution found is reported, until the search is complete or stopped;
- with an objective and `intermediate` set, each improving solution is reported as it is found;
- with an objective and `intermediate` unset, only the final (optimal) solution is reported;
- with `all_solutions` set, the search does not stop at the first optimal solution.

Under `intermediate`, a solver must report improving solutions **in improving order**, discarding any a worker produces out of order rather than reporting one worse than the last.

## Stopping early

Set the [`time_limit`](../../registry/options/) option.
That is the general way to bound a search, and a solver told its budget up front can plan around it: a portfolio solver decides from the time remaining whether to keep improving a solution or start proving the one it has optimal, which a solver merely interrupted cannot.

`should_stop` covers what an option cannot express: a user pressing cancel or a condition that depends on the answers received so far.
It is a predicate the solver polls; a `true` answer means give up as soon as convenient and return `FznsoIncomplete`.

A solver is expected to poll it:

- **once before starting**, so that an application which has already asked to stop gets no search at all;
- **at least once after each `on_solution` returns**, which is what makes "stop after this solution" reliable;
- otherwise **as often as is reasonable**, so a cancel is acted on promptly rather than at the end of the search.

`should_stop` may be **null**, which tells the solver the application will never ask to stop, so it can drop the check from its search loop entirely.
It is also [the one callback an application must make thread-safe](../threading/#should_stop-may-be-called-concurrently).

## Diagnostics

A solver reports non-fatal diagnostics through `on_message`.
Each message carries a *scope* naming its kind and a value carrying its payload.
Scopes are dot-separated from least to most specific, so an application can filter on a prefix without knowing the full set; the common ones are in the [registry](../../registry/message-scopes/).

Unlike options, constraints and statistics, scopes are **not** declared up front.
Messages are pushed to the application rather than retrieved by name, so it never needs to know a scope in advance to receive one, and can always report an unrecognised scope verbatim.

`on_message` may be **null**, meaning nobody is listening.
A solver should then skip *building* diagnostics rather than formatting them and handing them to a sink that discards them.

:::caution
This channel is **not** for failures.
A solver that cannot continue returns `FznsoError` and reports the reason through `fznso_<name>_solver_read_error`.
:::

## Status

| Status | Meaning |
| --- | --- |
| `FznsoComplete` | The solver explored the full search space and yielded all relevant solutions. If any solution exists, one was reported; if none was reported, none exists. |
| `FznsoIncomplete` | The search stopped early — a time limit, a `should_stop`, or any other termination condition. Additional or better solutions may exist. |
| `FznsoError` | The solver could not continue. Read the reason with `fznso_<name>_solver_read_error`. |

`FznsoComplete` with an objective and no `should_stop` means the last solution reported is optimal.
`FznsoIncomplete` means only that the solutions reported are valid, not that they are the best.
