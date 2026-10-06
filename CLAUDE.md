# FZnSO

An ABI for constraint solvers. A *solver* is a shared library exporting thirteen
`fznso_<name>_…` symbols; an *application* loads one, checks its version, and
asks it to search. Either side can be written in any language with a C FFI.

The full specification is in `docs/src/content/docs/spec/` — read the page for
whatever you are touching. This file is the part that is easy to get wrong.

## The one idea

**The model is not a data structure that gets handed over. It is a callback
table the solver reads through while it searches.**

Nothing is serialised and nothing is copied. The application's own problem
representation *is* the model, as long as it can answer `FznsoModelMethods`.
A solver pulls what it needs — how many decisions, what type each has, what
constraints apply — and answers come straight out of whatever the application
already holds.

This is the point of the interface, not an optimisation. An application that
solves, adds a constraint, and solves again pays no translation cost, and the
`layer` mechanism tells the solver exactly which part it has already seen.

So, when writing either side:

- **Never build an intermediate copy of the model.** If an adapter finds itself
  materialising a vector of constraints to answer `constraint_argument`, the
  design is wrong. Answer from the structure you already have.
- **Accessors are called in inner loops.** Keep them O(1) and allocation-free.
  A linear scan per decision is a quadratic model build; if you cannot avoid one,
  say so in a comment and explain what a caller should do instead.
- **Values are borrowed for a stated window**, never owned. See
  `spec/lifetimes-and-ownership.md`. Copy anything that must outlive it.
- A small payload (a decision index, a bool) is stored *in* the handle's pointer
  word rather than addressed. Never dereference a handle; go through `methods`.

## Traps

- **Names are for debugging.** `decision_name` may be absent, duplicated, or
  meaningless. Never infer anything from it — not identity, not whether a value
  is wanted.
- **Adding a field to any `…Methods` struct is an ABI break.** It moves every
  offset after it. While unreleased, `FZNSO_ABI_VERSION` stays 1 and the copies
  vendored by consumers are simply re-vendored together; once released, it
  bumps.
- **A solver declares only what it implements natively.** The capability lists
  are the whole negotiation: a consumer adapts to them, so an over-broad
  declaration is worse than a missing one.
- **A reformulation is not a propagator.** If a solver would express a
  constraint by introducing variables and posting *other* constraints, that
  belongs in whatever library the consumer decomposes with — not hand-written in
  the solver's own posting code, where its preconditions cannot be checked.
- Consumers vendor the headers. Change the ABI, re-vendor every copy, or a
  loader rejects the library.

## Layout

| | |
| --- | --- |
| `c/` | the ABI: `fznso_types.h`, `fznso_solver_template.c` — **generated** |
| `cpp/` | header-only C++ bindings for both roles |
| `rust/fznso-types` | the ABI in Rust; cbindgen generates `c/` from it |
| `rust/fznso` | Rust bindings, plus `LayeredModel` and a FlatZinc adapter |
| `rust/fznso-record` | `librecord`, a proxy solver that logs every call, for debugging |
| `rust/fznso-record-format` | the log format that `fznso-record` and `fznso-replay` share |
| `rust/fznso-replay` | replays a log: the `fznso-replay` binary to a solver, `libreplay` to an application |
| `python/` | pyo3 bindings |
| `docs/registry/*.toml` | the shared constraint/option/statistic vocabulary |
| `solver_impls/` | real solvers behind the interface |

`docs/registry` is normative for names and argument order: a solver declaring a
constraint declares the registry's spelling, and a consumer lowers its own names
to it. Adding an entry is not an ABI change.

## Commands

- `just` — build and lint; run it before you claim anything works.
- `just test` — Rust, C++ and Python suites. Run it on its own; it invokes cargo
  several ways and contends with itself if something else is building.
- `just format` — **uses nightly rustfmt**. Plain `cargo fmt` silently skips the
  unstable options this repo sets, and `just check` will then fail.

Never hand-edit `c/fznso_types.h`, `c/fznso_solver_template.c`, or
`docs/src/content/docs/reference/*.mdx`. Edit the Rust source or the cbindgen
preamble in `cbindgen_types.toml`, then `just build` / `just docs-api`.
