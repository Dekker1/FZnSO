---
title: Naming & versioning
description: How a solver's name is derived from its file name, what the version in that file name means, and why it says nothing about protocol compatibility.
sidebar:
  order: 3
---

## Naming

A solver's *name* is the identifier pasted into its entry points (`fznso_<name>_solver_run` and friends), and it is recovered from the file name.
It therefore has to satisfy both C and the platform's library naming at once:

1. **It must be a valid C identifier**: ASCII letters, digits and underscores, not starting with a digit (`[A-Za-z_][A-Za-z0-9_]*`), lowercase by convention.
   No `-`, no `.`, nothing non-ASCII, since those cannot appear in a symbol name.
2. **It must not begin with `lib`**, which could not be told apart from the platform prefix: `libssat` would read back as `libssat` from `liblibssat.so` but as `ssat` from `libssat.dll`.
3. **The file's base name must be `<name>`**, optionally prefixed with `lib`, followed by any version and extension components.

The name is read back by stripping a leading `lib` and everything from the first `.`, so `libgecode.6.2.1.{dll,dylib,so}` all name `gecode` and must export `fznso_gecode_abi_version`, `fznso_gecode_solver_run`, and so on.
The `lib` prefix follows the platform, because that is what build tools emit; only the version placement below is ours to fix.

A file name that yields something unusable is rejected when loading, rather than failing later as a missing symbol.

## Versioning

The version in a file name is the **solver's own release version** (Gecode 6.2.1 is installed as `libgecode.6.2.1.so`), and we ask that it be written as dotted numeric components, most significant first.

That shape is what makes selecting a solver work: asking for one by name gives the newest installed version, and an application can pin a series by naming a prefix of whole components, so `6` matches any `6.x.y` and `6.2` matches `6.2.x` but not `6.1.0`.
Components are compared numerically, so `10` is newer than `9`.

Semantic versioning fits this well, but it is the shape rather than semver in particular that we depend on: a date-based `2024.11` orders and pins just as predictably.
A version in some other form is still loaded; it simply sorts and pins less usefully, so prefer plain numeric components without leading zeros or trailing tags.

### The version says nothing about protocol compatibility

A solver's own major version is its own business: releasing Gecode 7 does not change how it speaks FZnSO.
Compatibility is reported separately, through `fznso_<name>_abi_version`, which an application checks before using any other entry point and which changes only when the protocol itself makes a breaking change; see [ABI stability](../abi-stability/).

Solvers built for different revisions of the protocol may therefore share a directory: when resolving a solver by name, a candidate built for another ABI version is skipped and the search continues.

### Where the version goes

The version goes **between the name and the extension on every platform** (`libgecode.6.2.1.so`, `libgecode.6.2.1.dylib`, `gecode.6.2.1.dll`) rather than following each platform's native library convention.
One rule then covers all three, and Windows stops being a special case: `gecode-6.dll` is not a legal name (rule 1 above), and there is no `.dll.6` convention to fall back on.

Deviating from the platform here costs nothing, because nothing ever links against a solver: it is opened by path from a directory [off the linker search path](../discovery/#why-a-dedicated-directory), so the whole `libfoo.so.MAJOR` apparatus is inert.
Python names extension modules the same way, and for the same reason (`foo.cpython-312-x86_64-linux-gnu.so`).

The native form is still parsed, so a distribution-packaged `libgecode.so.6` loads as `gecode` version `6`.

Several versions of one solver can sit side by side, since everything from the first `.` is ignored when reading the name back: `libgecode.6.so` and `libgecode.7.so` both name `gecode`.
