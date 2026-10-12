# fznso-conform

Holds a solver to the registry, in two ways. Does what it declares match the
types the registry defines, and does it answer each constraint correctly?

```sh
uv run fznso-conform types <solver>
uv run fznso-conform solutions <solver>
```

`<solver>` is a MiniZinc solver configuration (`.msc`) or the solver library
itself. A configuration says everything: which library to load, and which
MiniZinc library the solver ships. A bare library says only itself, so the tool
writes a configuration naming it, and the solver's own MiniZinc library, if it
has one, is then not on the include path.

`types` reads the four capability lists through the ABI, with the Python
bindings. It needs no compiler and no solve.

`solutions` states each registry constraint in Python and checks that the
solutions the solver permits are exactly the ones that meaning allows. Each
case states the constraint call, the decisions it ranges over, and what the
constraint means. A solver that declares the constraint answers with its own
propagator, and one that does not gets the registry's decomposition, so the
same case covers both paths.

The cases probe the boundaries rather than the typical call: an empty array, an
index one past each end, an index set numbered from a negative offset, equal
arguments, and a reifying variable that must be false.

## What it needs from MiniZinc

The suite drives a solver through MiniZinc rather than through the ABI, so it
tests what a MiniZinc application sees. That puts two requirements on the pair.

**The model has to compile for the solver.** The harness writes a model naming
one registry constraint and compiles it with that solver's configuration. A
solver that declares the constraint gets it under its own name, and one that
does not gets the registry decomposition from `fznso_constraints`. Either way
the compiler has to produce a model the solver accepts.

**The solutions have to be collectable one of two ways.** The solver declares
the all-solutions flag and enumerates them itself, or the harness adds a
constraint ruling out each answer as it is found and compiles the model again.
A solver that offers neither cannot be tested here.

The second way is the sequence an incremental client performs, and it is the
reason the suite is written against MiniZinc at all: once the compiler keeps
its state between calls, the same loop becomes an incremental test. Today each
round recompiles the whole model, so a case with many solutions costs one
compile per solution.

## Running

```sh
just test-conformance
```

That runs both checks over every configuration in one go, which is what the
commands above do for one solver. `uv run pytest -q` inside this directory does
the same.

Every solver runs by default. Two environment variables narrow or redirect that:

- `FZNSO_SOLVERS`: a comma-separated list of solver identifiers.
- `FZNSO_SOLVER_PATH`: where the `.msc` configurations live.

## Collecting the solutions

Where a solver declares the all-solutions flag, it enumerates. Otherwise the
harness adds a constraint ruling out each answer as it is found and solves
again, which is the sequence an incremental client performs.

`FZNSO_NOGOODS` asks for that loop even from a solver that declares the flag.
It takes a comma-separated list of solver identifiers, or `all`.

**Gurobi needs it for two cases.** It answers the flag from its solution pool,
which proves that no further solution ties with the ones it holds. For
`int_subcircuit` and `int_lex_least_under_transposition` that proof does not
finish within the 60 s each case is given, and the harness then reports a
timeout rather than a wrong answer. Both pass in seconds through the loop,
because each added constraint is a row that cuts off the region it excludes:

```sh
FZNSO_SOLVERS=org.minizinc.fznso.gurobi FZNSO_NOGOODS=all uv run pytest -q
```

## Reading a failure

A failing case reports which solutions are missing or extra, whether the solver
declared the constraint or decomposed it, and how the solutions were collected.
A case the solver can neither declare nor decompose is skipped, and the skip
names the library's own refusal.
