# IPDOS: An Incremental Protocol for Decision and Optimization Solvers

## Introduction

## Common Functionality

Although solvers are able to define their own functionality using the IPDOS protocol, we advocate for the following common functionality to be implemented by different solvers.
This will allow for a more consistent user experience when interacting with different solvers.
Even if a solver does not implement all of these functions, we recommend that the solvers do not use the same name for different functionality.

### Common Options

A solver will expose its available options through `ipdos_option_list`.
We recommend that solvers eagerly implement the following options:

- `all_optimal` (`bool`, default: `false`): If set to `true`, the solver will after finding an optimal solution, continue to search for other solutions with the same objective value.
- `fixed_search` (`bool`, default: `false`): If set to `true`, the solver will strictly follow the search order defined by the user.
- `intermediate` (`bool`, default: `false`): If set to `true` for a problem with an objective strategy set, the solver will trigger its `on_solution` callback when it finds an intermediate solution.
  Afterward, the solver will continue the search until it finds the next solution, or it proves that no better solutions exist (returning the `IpdosComplete` status).
- `threads` (`int`, default: `1`): For multithreaded solvers, this option will set the number of threads to use.
- `time_limit` (`int`, default: `-1`): If set to a positive integer, the solver will abandon the search after the specified number of milliseconds.
- `random_seed` (`opt int`, default: `<>`): If set to a positive integer, the solver will use the given value as the seed for its random number generator.
  Solvers are encouraged to use a variable seed for the default value.
- `verbose` (`bool`, default: `false`): If set to `true`, the solver will print additional information about its process to `stderr`.

### Common Constraints

A solver will expose the constraints that can be used in a `IpdosModel` through `ipdos_constraint_list`.
We encourage solvers to support the constraints using the names and definitions from the [FlatZinc Builtins](https://docs.minizinc.dev/en/stable/lib-flatzinc.html).
Other MiniZinc (global) constraints are also encouraged to be implemented, using the `fzn_` prefix.

### Common Objective Strategies

A solver will expose the objective strategies that can be used in a `IpdosModel` through `ipdos_objective_list`.
We encourage solvers to support the following objective strategies if possible:

- `lex_maximize_int` (`list of var int`) | `lex_maximize_float` (`list of var float`): The solver will maximize the list of decision variables in lexicographical order, i.e., the first variable is maximized first, then the second variable, etc.
- `lex_minimize_int` (`list of var int`) | `lex_minimize_float` (`list of var float`): The solver will minimize the list of decision variables in lexicographical order, i.e., the first variable is minimized first, then the second variable, etc.
- `maximize_int` (`var int`) | `maximize_float` (`var float`): The solver will maximize the given decision variable.
- `minimize_int` (`var int`) | `minimize_float` (`var float`): The solver will minimize the given decision variable.
- `pareto_maximize_int` (`list of var int`) | `pareto_maximize_float` (`list of
var float`): The solver will output solutions where at least one decision variable is assigned a higher value than it was assigned in all previous solutions.

Note that if no objective is set, then the solver is expected to find any valid assignment of the decision variables that satisfies the constraints added to the solver.
