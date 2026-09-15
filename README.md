# Flat White Vitriol (FZnSO)

An incremental solver protocol for combinatorial solving using dynamically loadable libraries.

FZnSO lets an application drive a constraint or optimisation solver through a plain C interface, with the solver loaded at run time as an ordinary library.
No subprocess, no text format, and no re-flattening the problem between solves — which is what makes the *solve → inspect → add a constraint → solve again* loop cheap.

## 📖 [Documentation](https://fznso.minizinc.org/)

Everything is on the site: the [specification](https://fznso.minizinc.org/spec/overview/), guides for [using](https://fznso.minizinc.org/guides/using-a-solver/) and [implementing](https://fznso.minizinc.org/guides/implementing-a-solver/) a solver, the [registry](https://fznso.minizinc.org/registry/) of common constraint, option and objective names, and generated API references for all four language surfaces.

## Repository layout

| Path | What it is |
| --- | --- |
| `rust/fznso-types` | The protocol's types. **The interface is defined here.** |
| `rust/fznso-solver-template` | The thirteen entry points a solver implements, as a template |
| `rust/fznso` | Rust bindings — both sides of the interface |
| `c/` | **Generated** by cbindgen from the two crates above. Do not edit by hand. |
| `cpp/` | Header-only C++17 bindings: `fznso.hpp` to use a solver, `fznso_export.hpp` to write one |
| `python/` | Python bindings, built with maturin |
| `docs/` | The documentation site (Astro Starlight) and the registry data |

## Building

Everything runs through [`just`](https://just.systems); `just --list` shows the rest.

```sh
just          # build + check
just test     # Rust, C++ and Python suites
just docs     # the documentation site, including all three API references
```

See [Build & test](https://fznso.minizinc.org/contributing/build-and-test/) for what you need installed, and why `c/` must never be edited by hand.

## Contributing

- [Adding a registry entry](https://fznso.minizinc.org/contributing/registry-entry/) — proposing a common name
- [Proposing an ABI change](https://fznso.minizinc.org/contributing/abi-change/) — what counts as breaking, and what a proposal needs

## Licence

[MIT](LICENSE).
Implementing the FZnSO interface carries no obligation, and the headers and solver template are meant to be copied.
