---
title: Recording and replaying
description: "Record every call an application makes into a solver, then replay either side of it: the application's calls without the application, or the solver's answers without the solver."
sidebar:
  order: 5
---

A solver bug often shows only inside one application.
The recorder captures what that application asks of the solver, so you can reproduce the bug with nothing but the log and the solver library.

A log works in both directions:

- `librecord` (crate `fznso-record`) is a solver named `record` that passes every call on to another solver and writes it to a log.
- The `fznso-replay` binary (crate `fznso-replay`) replays the application's side: it makes the same calls on a solver and checks that it answers the same.
- `libreplay` (crate `fznso-replay`) replays the solver's side: it is a solver named `replay` that answers an application from the log, with no solver.

## Record

Build the recorder with `cargo build -p fznso-record`, then point the application at the solver `record` instead of the real one.
Put the directory that holds `librecord` on `FZNSO_SOLVER_PATH`, as for any solver under development.

The ABI has no way to configure a solver before it exists, so the recorder reads its settings from the environment:

| Variable | Meaning |
| --- | --- |
| `FZNSO_RECORD_SOLVER` | Required. The solver to record: a name to find on the search path, or a path to its library. |
| `FZNSO_RECORD_FILE` | The log to write. By default `fznso-record-<pid>.msgpack` in the current directory. |

The recorder flushes the log after each call, so the log is complete up to the moment a solver crashes.

## Replay to a solver

```sh
fznso-replay session.msgpack
fznso-replay session.msgpack --solver path/to/libmysolver.dylib
fznso-replay session.msgpack --strict
```

By default the replay loads the solver the log recorded.
An answer that differs from the recording is a warning, and the replay goes on to the end.
The log, not the solver's answers, decides every later call, so a run after a difference still gets the recorded model.
This lets you replay a partial fix: a different search decision can change a solution, and the later layers are still worth running.
Only the first differing solution of each run is shown, since one different decision usually changes every solution after it.
With `--strict`, the replay stops at the first difference instead.

It compares option results, solutions and the status of each run.
It asks for statistics too, but does not compare them, because they may measure time.

The exit status is 0 if every answer matches, 2 if some differ, and 1 if the replay cannot go on, such as for an unreadable log.

## Replay to an application

To debug an application, such as a modelling system, without the solver, point it at the solver `replay` and name the log:

| Variable | Meaning |
| --- | --- |
| `FZNSO_REPLAY_FILE` | Required. The log to answer from. |

The replay solver declares the constraints, decision types, objectives, options and statistics that the recorded solver declared.
It returns the recorded option and statistic values, and each run reports the recorded messages and solutions and returns the recorded status.
It asks `should_stop` before a run and after each solution, as a solver does, and stops the run there if the answer is yes.

Before each run it reads the application's model and compares it with the recorded one.
If they differ, it names the first place on standard error, for example `decision 4 of layer 1 differs`, with both versions.
That shows where a changed modelling system starts to produce a different model, without a solver run.
The run still plays the recorded solutions.

The application is expected to make the recorded calls in the recorded order, for each solver instance.
Where it does not, the replay solver says so on standard error and goes on:

- A call the log holds further on is answered from there, and the recorded calls before it are skipped. The search stops at the next run.
- A call the log does not hold gets an empty answer: an absent value, a successful `option_set`, or an error from `run`.

## Model checks

The recorder reads the whole model before each run and checks it against the rules in [The model](../../spec/model/): that layers reported as unchanged did not change, that permanent layers stay, that redundant layers stay redundant, that indices are in range, and that sets are sorted and do not overlap.
It reports each problem on standard error and in the log, and passes the call on regardless.

## What replay cannot make deterministic

Replay gives the solver the same inputs in the same order.
It does not control the solver's own sources of variation:

- More than one thread. `should_stop` is replayed by the position of the call that first returned `true`, and that position changes between runs.
- A `time_limit`.
- An absent `random_seed`, which lets the solver choose one.

To record a session you can replay exactly, set `threads` to 1 and a fixed `random_seed`, and do not rely on a time limit.
Replay reports any difference these cause. It does not hide one.

The recorder joins adjacent integer ranges, such as `1..2` and `3..4`, into one range when it copies a set, so the log and the replay hold the joined form.
The set is the same, but a solver that reads the ranges one by one sees fewer of them.

## The log format

The log is a stream of [MessagePack](https://msgpack.org) values: one header, then one entry per call.
Structs are maps with field names, and enum variants appear by name, so any MessagePack reader can show a log:

```python
import msgpack
for entry in msgpack.Unpacker(open("session.msgpack", "rb")):
    print(entry)
```

The log types are in `tools/fznso-record-format/src/format.rs`. Values, annotations, types and statuses use the optional `serde` feature of the `fznso` crate.
