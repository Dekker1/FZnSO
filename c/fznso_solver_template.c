// FZnSO solver entry points to implement in C.
//
// Each function below is named `fznso_NAME_...`. The `fznso_` marker is fixed;
// `NAME` is a placeholder that a real solver replaces with its library name —
// the base name of the built library file (e.g. `gecode` for `libgecode.so`).
// Prefixing every symbol this way lets several solvers be linked into one binary
// without colliding (with each other or with any wrapped library), and lets a
// loader recover the name from the file name.

#include "fznso_types.h"

// Report the FZnSO ABI version this solver was built against. A loader rejects
// the solver if this does not match its own.
uint32_t fznso_NAME_abi_version(void);

// Returns the list of available constraint that can be added to the solver.
FznsoConstraintList fznso_NAME_constraint_list(void);

// Returns the list of types for which decision variable can be created by the
// solver.
FznsoTypeList fznso_NAME_decision_list(void);

// Returns the list of available objective that can be achieved by the solver.
FznsoObjectiveList fznso_NAME_objective_list(void);

// Returns the list of available options that can be set of the solver.
FznsoOptionList fznso_NAME_option_list(void);

// Create a new solver instance
FznsoSolver *fznso_NAME_solver_create(void);

// Free a solver instance, releasing all resources associated with it.
//
// The pointer to the solver instance will be invalid after this function has
// been called.
//
// # Safety
//
// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
// not yet been passed to this function.
void fznso_NAME_solver_free(FznsoSolver *solver);

// Get the current value of an option for the solver.
//
// Note that this is only valid to be called with options named by
// `fznso_NAME_option_list`.
//
// # Safety
//
// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
// not yet been passed to [`fznso_NAME_solver_free`].
FznsoValueRef fznso_NAME_solver_option_get(const FznsoSolver * solver, FznsoStr ident);

// Set the current value of an option for the solver.
//
// Note that this is only valid to be called with options named by
// `fznso_NAME_option_list`, and the value must be of the correct type. This
// function returns `true` if the option was successfully set, and `false`
// otherwise.
//
// # Safety
//
// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
// not yet been passed to [`fznso_NAME_solver_free`].
bool fznso_NAME_solver_option_set(FznsoSolver *solver, FznsoStr ident, FznsoValueRef value);

// Read an error message from the solver.
//
// This function is expected to be called after solver interactions signal an
// error has occurred. For example, if [`fznso_NAME_solver_run`] returns
// [`FznsoError`] or [`fznso_NAME_solver_option_set`] returns `false`.
//
// # Safety
//
// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
// not yet been passed to [`fznso_NAME_solver_free`].
void fznso_NAME_solver_read_error(FznsoSolver *solver,
                                  void *context,
                                  void (*read_error)(void *context, FznsoStr error));

// Run the solver with the given model
//
// `on_message` reports non-fatal diagnostics: warnings, progress and search
// information. `scope` names the kind of message and is dot-separated so that
// callers can filter on a prefix. Failures are not reported here; they are
// signalled by returning [`FznsoError`] and retrieved with
// [`fznso_NAME_solver_read_error`].
//
// # Threading
//
// A solver may search on several threads. The contract for the callbacks and
// the model is:
//
// - `on_solution` and `on_message` must be invoked **serially**: never two at
//   once, and never one while the other runs. They may be called from any
//   thread (for example the worker that found a solution), as long as each call
//   happens-before the next. A caller therefore need not make its callbacks
//   safe to call concurrently. (Serial reporting also keeps solutions in the
//   improving order that intermediate/bound reporting relies on.)
// - `model` may be queried **concurrently**: it is a read-only view for the
//   duration of the call, so a parallel solver may read it from many threads at
//   once. A caller must therefore make the model safe for concurrent reads.
//
// # Safety
//
// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
// not yet been passed to [`fznso_NAME_solver_free`].
FznsoStatus fznso_NAME_solver_run(FznsoSolver *solver,
                                  FznsoModelRef model,
                                  void *context,
                                  void (*on_solution)(void *context, FznsoSolutionRef solution),
                                  void (*on_message)(void *context,
                                                     FznsoStr scope,
                                                     FznsoValueRef value));

// Get the current value of a statistic from the solver instance.
//
// Note that this is only valid to be called with statistics named by
// `fznso_NAME_statistic_list` whose `solver` flag is set; statistics carrying
// only the `solution` flag are read from a solution instead, using
// `FznsoSolutionMethods::statistic`. An unknown identifier yields a value of
// kind `FznsoValueAbsent`.
//
// # Safety
//
// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
// not yet been passed to [`fznso_NAME_solver_free`].
FznsoValueRef fznso_NAME_solver_statistic(const FznsoSolver * solver, FznsoStr ident);

// Returns the list of available statistical information that can be requested
// from the solver and its solutions.
FznsoStatisticList fznso_NAME_statistic_list(void);
