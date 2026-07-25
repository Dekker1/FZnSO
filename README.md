# Flat White Vitriol (FZnSO) - An incremental solver protocol for combinatorial solving using shared objects 

## Introduction

## Threading

A solver may search on several threads (see the `threads` option). The protocol
fixes how that interacts with the callbacks and the model passed to
`fznso_solver_run`:

- The `on_solution` and `on_message` callbacks are invoked **serially** — never
  two at once, and never one while the other runs — but may be called from any
  thread, as long as each call happens-before the next. A consumer therefore
  never has to make its callbacks safe to call concurrently; if it wants to
  process solutions in parallel it does so behind a serial callback (e.g. by
  pushing to a queue). Serial reporting also keeps solutions in the improving
  order that `intermediate` and `progress.bound` rely on.
- The model is a **read-only** view for the duration of the call and may be
  queried **concurrently** from any number of threads, so a consumer must make
  its model safe for concurrent reads.

## Common Functionality

Although solvers are able to define their own functionality using the FZnSO protocol, we advocate for the following common functionality to be implemented by different solvers.
This will allow for a more consistent user experience when interacting with different solvers.
Even if a solver does not implement all of these functions, we recommend that the solvers do not use the same name for different functionality.

### Common Options

A solver will expose its available options through `fznso_option_list`.
We recommend that solvers eagerly implement the following options:

- `all_optimal` (`bool`, default: `false`): If set to `true`, the solver will after finding an optimal solution, continue to search for other solutions with the same objective value.
- `fixed_search` (`bool`, default: `false`): If set to `true`, the solver will strictly follow the search order defined by the user.
- `intermediate` (`bool`, default: `false`): If set to `true` for a problem with an objective strategy set, the solver will trigger its `on_solution` callback when it finds an intermediate solution.
  Afterward, the solver will continue the search until it finds the next solution, or it proves that no better solutions exist (returning the `FznsoComplete` status).
- `threads` (`int`, default: `1`): For multithreaded solvers, this option will set the number of threads to use.
- `time_limit` (`int`, default: `-1`): If set to a positive integer, the solver will abandon the search after the specified number of milliseconds.
- `random_seed` (`opt int`, default: `<>`): If set to a positive integer, the solver will use the given value as the seed for its random number generator.
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
