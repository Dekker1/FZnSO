---
title: Discovery
description: Where solver libraries are installed, the order the directories are searched, and why solvers live off the normal library path.
sidebar:
  order: 4
---

A solver is a dynamically loadable library installed into a `fznso` directory of its own, rather than onto the normal library path.
Applications find one by name, never a path, and the directories are searched in this order:

1. `$FZNSO_SOLVER_PATH`, split on the platform's path separator (`:`, or `;` on Windows);
2. the per-user directory: `$XDG_DATA_HOME/fznso` (falling back to `~/.local/share/fznso`), `~/Library/Application Support/fznso` on macOS, or `%LOCALAPPDATA%\fznso` on Windows;
3. the system directories: `/usr/local/lib/fznso` and `/usr/lib/fznso`, or a `fznso` directory beside the running executable on Windows.

Earlier directories win outright, so a solver dropped in `$FZNSO_SOLVER_PATH` or the per-user directory overrides a system one even if the system one is newer; the newest [version](../naming-and-versioning/#versioning) is chosen only among candidates of equal precedence.

A candidate whose `fznso_<name>_abi_version` does not match the application's is skipped, and the search continues with the next candidate.

## Why a dedicated directory

The dedicated directory is there for three reasons:

- **Names would otherwise collide.**
  An FZnSO solver is usually a shim named after the library it wraps, so `libgecode.so` would sit on top of real Gecode.
- **Enumeration stays cheap.**
  Listing the installed solvers is a single directory listing, rather than a scan of every system library.
- **Nothing loads one by accident.**
  Solvers stay off the default linker search path, where nothing should find them.

The third point is also what frees the [version placement](../naming-and-versioning/#where-the-version-goes) from platform convention: nothing ever links against a solver, so no soname is ever resolved.
