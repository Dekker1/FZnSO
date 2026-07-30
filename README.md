# Flat White Vitriol (FZnSO) - An incremental solver protocol for combinatorial solving using shared objects 

## Introduction

## Solver libraries

### Installation and discovery

A solver is a shared object installed into a `fznso` directory of its own, rather than onto the normal library path.
Consumers find one by name — they never need a path — and the directories are searched in this order:

1. `$FZNSO_SOLVER_PATH`, split on the platform's path separator (`:`, or `;` on Windows),
2. the per-user directory: `$XDG_DATA_HOME/fznso` (falling back to `~/.local/share/fznso`), `~/Library/Application Support/fznso` on macOS, or `%LOCALAPPDATA%\fznso` on Windows,
3. the system directories: `/usr/local/lib/fznso` and `/usr/lib/fznso`, or a `fznso` directory beside the running executable on Windows.

Earlier directories win outright, so a solver dropped in `$FZNSO_SOLVER_PATH` or the per-user directory overrides a system one even if the system one is newer; the newest version is chosen only among candidates of equal precedence.

The dedicated directory is not incidental.
A FZnSO solver is usually a shim named after the library it wraps, so `libgecode.so` would otherwise collide with real Gecode; enumerating the installed solvers is a single directory listing rather than a scan of every system library; and solvers stay off the default linker search path, where nothing should load them by accident.

### Naming

A solver's *name* is the identifier pasted into its entry points (`fznso_<name>_solver_run` and friends), and it is recovered from the file name.
It therefore has to satisfy both C and the platform's library naming at once:

1. **It must be a valid C identifier**: ASCII letters, digits and underscores, not starting with a digit (`[A-Za-z_][A-Za-z0-9_]*`), lowercase by convention.
   No `-`, no `.`, nothing non-ASCII — those cannot appear in a symbol name.
2. **It must not begin with `lib`**, which could not be told apart from the platform prefix: `libssat` would read back as `libssat` from `liblibssat.so` but as `ssat` from `libssat.dll`.
3. **The file's base name must be `<name>`**, optionally prefixed with `lib`, followed by any version and extension components.

The name is read back by stripping a leading `lib` and everything from the first `.`, so `libgecode.6.2.1.{dll,dylib,so}` all name `gecode` and must export `fznso_gecode_abi_version`, `fznso_gecode_solver_run`, and so on.
The `lib` prefix follows the platform, because that is what build tools emit; only the version placement below is ours to fix.
A file name that yields something unusable is rejected when loading, rather than failing later as a missing symbol.

### Versioning

The version in a file name is the **solver's own release version** — Gecode 6.2.1 is installed as `libgecode.6.2.1.so` — and we ask that it be written as dotted numeric components, most significant first.
That shape is what makes selecting a solver work: asking for one by name gives the newest installed version, and a consumer can pin a series by naming a prefix of whole components, so `6` matches any `6.x.y` and `6.2` matches `6.2.x` but not `6.1.0`.
Components are compared numerically, so `10` is newer than `9`.
Semantic versioning fits this well, but it is the shape rather than semver in particular that we depend on — a date-based `2024.11` orders and pins just as predictably.
A version in some other form is still loaded; it simply sorts and pins less usefully, so prefer plain numeric components without leading zeros or trailing tags.

**This version says nothing about protocol compatibility.**
A solver's own major version is its own business: releasing Gecode 7 does not change how it speaks FZnSO.
Compatibility is reported separately, through `fznso_<name>_abi_version`, which a consumer checks before using any other entry point and which changes only when the protocol itself makes a breaking change.
Solvers built for different revisions of the protocol may therefore share a directory: when resolving a solver by name, a candidate built for another ABI version is skipped and the search continues.

The version goes **between the name and the extension on every platform** — `libgecode.6.2.1.so`, `libgecode.6.2.1.dylib`, `gecode.6.2.1.dll` — rather than following each platform's native library convention.
One rule then covers all three, and Windows stops being a special case: `gecode-6.dll` is not a legal name (rule 1), and there is no `.dll.6` convention to fall back on.

Deviating from the platform here costs nothing, because nothing ever links against a solver.
It is opened by path from a directory that is deliberately off the linker search path, so no soname is resolved, `ldconfig` never sees it, and no other library declares a dependency on it — the whole `libfoo.so.MAJOR` apparatus is inert for a plugin.
Python names extension modules the same way, and for the same reason (`foo.cpython-312-x86_64-linux-gnu.so`).
The native form is still parsed, so a distribution-packaged `libgecode.so.6` loads as `gecode` version `6`.

Several versions of one solver can sit side by side, since everything from the first `.` is ignored when reading the name back: `libgecode.6.so` and `libgecode.7.so` both name `gecode`.

## Stopping a search

If you want to set a time limit, set the `time_limit` option.
A solver told its budget up front can *plan* around it: sizing restart schedules, deciding when to stop diversifying and dive for a feasible solution, budgeting time between portfolio strategies.

For other scenarios, `fznso_solver_run` takes a `should_stop` predicate alongside the solution and message callbacks; the solver polls it, and a `true` answer means give up as soon as convenient and return `FznsoIncomplete`.
Use `should_stop` for what a deadline cannot express: a user pressing cancel, “five solutions are enough”, or a condition that depends on the answers received so far.

A solver is expected to poll it once before starting (so a caller that has already asked to stop gets no search at all), at least once after each `on_solution` returns (which is what makes "stop after this solution" reliable), and otherwise as often as is reasonable.
`should_stop` may be **null**, which tells the solver the caller will never ask to stop, so it can drop the check from its search loop entirely.

## Threading

A solver may search on several threads (see the `threads` option).
The protocol fixes how that interacts with the callbacks and the model passed to `fznso_solver_run`:

- The `on_solution` and `on_message` callbacks are invoked **serially** — never two at once, and never one while the other runs — but may be called from any thread, as long as each call happens-before the next.
  A consumer therefore never has to make its callbacks safe to call concurrently; if it wants to process solutions in parallel it does so behind a serial callback (e.g. by pushing to a queue).
  Serial reporting also keeps solutions in the improving order that `intermediate` and `progress.bound` rely on.
- The model is a **read-only** view for the duration of the call and may be queried **concurrently** from any number of threads, so a consumer must make its model safe for concurrent reads.
- `should_stop` may likewise be called **concurrently**, from any thread, and even while `on_solution` or `on_message` is running — every worker may poll it independently.
  It is the one callback a consumer must make thread-safe, which the usual implementation (reading an atomic flag) already is.

## Common Functionality

Although solvers are able to define their own functionality using the FZnSO protocol, we advocate for the following common functionality to be implemented by different solvers.
This will allow for a more consistent user experience when interacting with different solvers.
Even if a solver does not implement all of these functions, we recommend that the solvers do not use the same name for different functionality.

### Common Options

A solver will expose its available options through `fznso_option_list`.
We recommend that solvers eagerly implement the following options:

- `all_solutions` (`bool`, default: `false`): If set to `true`, the solver will after finding an (optimal) solution, continue to search for other solutions with the same objective value.
- `fixed_search` (`bool`, default: `false`): If set to `true`, the solver will strictly follow the search order defined by the user.
- `intermediate` (`bool`, default: `false`): If set to `true` for a problem with an objective strategy set, the solver will trigger its `on_solution` callback when it finds an intermediate solution.
  Afterward, the solver will continue the search until it finds the next solution, or it proves that no better solutions exist (returning the `FznsoComplete` status).
- `threads` (`int`, default: `1`): For multithreaded solvers, this option will set the number of threads to use.
- `time_limit` (`opt 1..`, default: `<>`): If set to a positive integer, the solver will abandon the search after the specified number of milliseconds.
- `random_seed` (`opt 1..`, default: `<>`): If set to a positive integer, the solver will use the given value as the seed for its random number generator.
  Solvers are encouraged to use a variable seed for the default value.
- `verbose` (`bool`, default: `false`): If set to `true`, the solver will output additional information about its process using the `on_message` `"log"` scope.

### Common Constraints

A solver will expose the constraints that can be used in a `FznsoModel` through `fznso_constraint_list`.
We encourage solvers to support the constraints using the names and definitions from the [FlatZinc Builtins](https://docs.minizinc.dev/en/stable/lib-flatzinc.html).
Other MiniZinc (global) constraints are also encouraged to be implemented, using the `fzn_` prefix.

### Common Message Scopes

While solving, a solver can report non-fatal diagnostics through the `on_message` callback given to `fznso_solver_run`.
Each message carries a *scope* naming its kind and a value carrying its payload.
Unlike options, constraints and statistics, scopes are deliberately **not** declared up front: messages are pushed to the caller rather than retrieved by name, so a consumer never needs to know a scope in advance to receive one, and can always fall back to reporting an unrecognised scope verbatim.

Scopes are dot-separated from least to most specific, so that a consumer can filter on a prefix without knowing the full set.
We recommend the following:

- `log` (`string`): freeform diagnostic text, generally enabled by the `verbose` option.
- `warn` (`string`): something about the model the user should know, but which did not prevent solving (for example an annotation the solver ignored).
- `progress.bound` (`int` | `float`): a new dual bound on the objective.
- `core.assumptions` (`list of var bool`): a subset of the Boolean decision variables the caller assumed, which cannot all hold at once.

Note that this channel is *not* for failures.
A solver that cannot continue returns `FznsoError` from `fznso_solver_run` and reports the reason through `fznso_solver_read_error`.

### Common Objective Strategies

A solver will expose the objective strategies that can be used in a `FznsoModel` through `fznso_objective_list`.
We encourage solvers to support the following objective strategies if possible:

- `lex_maximize_int` (`list of var int`) | `lex_maximize_float` (`list of var float`): The solver will maximize the list of decision variables in lexicographical order, i.e., the first variable is maximized first, then the second variable, etc.
- `lex_minimize_int` (`list of var int`) | `lex_minimize_float` (`list of var float`): The solver will minimize the list of decision variables in lexicographical order, i.e., the first variable is minimized first, then the second variable, etc.
- `maximize_int` (`var int`) | `maximize_float` (`var float`): The solver will maximize the given decision variable.
- `minimize_int` (`var int`) | `minimize_float` (`var float`): The solver will minimize the given decision variable.
- `pareto_maximize_int` (`list of var int`) | `pareto_maximize_float` (`list of
var float`): The solver will output solutions where at least one decision variable is assigned a higher value than it was assigned in all previous solutions.

Note that if no objective is set, then the solver is expected to find any valid assignment of the decision variables that satisfies the constraints added to the solver.
