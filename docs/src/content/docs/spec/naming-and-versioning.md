---
title: Naming & versioning
description: Why every symbol carries the solver's name, how that name is derived from the file name, and what the version in that file name means.
sidebar:
  order: 3
---

## Why the name is in every symbol

So that two solvers can be **statically linked into one binary**, which targets with no dynamic loader — WebAssembly above all — require.
`fznso_gecode_solver_run` and `fznso_chuffed_solver_run` coexist; two copies of a bare `fznso_solver_run` would collide at link time.

## Naming

A solver's *name* is the identifier pasted into its entry points, so it has to satisfy both C and the platform's library naming at once:

1. **A valid C identifier**: ASCII letters, digits and underscores, not starting with a digit (`[A-Za-z_][A-Za-z0-9_]*`), lowercase by convention.
   No `-`, no `.`, nothing non-ASCII — none of those can appear in a symbol name.
2. **It must not begin with `lib`**, which could not be told apart from the platform prefix: `libssat` would read back as `libssat` from `liblibssat.so` but as `ssat` from `libssat.dll`.
3. **The file's base name must be `<name>`**, optionally prefixed with `lib`, followed by any version and extension components.

The name is read back by stripping a leading `lib` and everything from the first `.`, so `libgecode.6.2.1.{dll,dylib,so}` all name `gecode` and must export `fznso_gecode_abi_version`, `fznso_gecode_solver_run` and the rest.
The `lib` prefix follows the platform, because that is what build tools emit.
A file name that yields something unusable is rejected when loading, rather than failing later as a missing symbol.

Because everything from the first `.` is ignored, several versions of one solver sit side by side: `libgecode.6.so` and `libgecode.7.so` both name `gecode`.

## Versioning

The version in a file name is the **solver's own release version** — Gecode 6.2.1 is installed as `libgecode.6.2.1.so` — and we ask that it be written as dotted numeric components, most significant first.

That shape is what makes selecting a solver work: asking for one by name gives the newest installed version, and naming a prefix of whole components pins a series, so `6` matches any `6.x.y` and `6.2` matches `6.2.x` but not `6.1.0`.
Components are compared numerically, so `10` is newer than `9`.

Semantic versioning fits this well, but it is the shape rather than semver in particular that we depend on: a date-based `2024.11` orders and pins just as predictably.
A version in some other form is still loaded; it simply sorts and pins less usefully.

### Where the version goes

Between the name and the extension **on every platform** — `libgecode.6.2.1.so`, `libgecode.6.2.1.dylib`, `gecode.6.2.1.dll` — rather than following each platform's native library convention.
One rule then covers all three, and Windows stops being a special case: `gecode-6.dll` is not a legal name (rule 1 above), and there is no `.dll.6` convention to fall back on.

Deviating from the platform costs nothing because [nothing ever links against a solver](../discovery/#why-a-dedicated-directory), so the whole `libfoo.so.MAJOR` apparatus is inert.
Python names extension modules the same way, and for the same reason.

The native form is still parsed, so a distribution-packaged `libgecode.so.6` loads as `gecode` version `6`.

### The version says nothing about protocol compatibility

Releasing Gecode 7 does not change how it speaks FZnSO.
Compatibility is reported separately, through `fznso_<name>_abi_version`, which changes only when the protocol itself makes a breaking change; see [ABI stability](../abi-stability/).

Solvers built for different revisions may therefore share a directory: a candidate built for another ABI version is [skipped](../discovery/), and the search continues.
