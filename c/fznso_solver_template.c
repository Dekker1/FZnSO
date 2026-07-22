#include "fznso_types.h"

// Returns the list of available constraint that can be added to the solver.
FznsoConstraintList fznso_constraint_list(void);

// Returns the list of types for which decision variable can be created by the
// solver.
FznsoTypeList fznso_decision_list(void);

// Returns the list of available objective that can be achieved by the solver.
FznsoObjectiveList fznso_objective_list(void);

// Returns the list of available options that can be set of the solver.
FznsoOptionList fznso_option_list(void);

// Create a new solver instance
FznsoSolver *fznso_solver_create(void);

// Free a solver instance, releasing all resources associated with it.
//
// The pointer to the solver instance will be invalid after this function has
// been called.
//
// # Safety
//
// `solver` must be a pointer returned by [`fznso_solver_create`] that has not
// yet been passed to this function.
void fznso_solver_free(FznsoSolver *solver);

// Get the current value of an option for the solver.
//
// Note that this is only valid to be called with options named by
// `fznso_option_list`.
//
// # Safety
//
// `solver` must be a pointer returned by [`fznso_solver_create`] that has not
// yet been passed to [`fznso_solver_free`].
FznsoValueRef fznso_solver_option_get(const FznsoSolver * solver, const char *ident);

// Set the current value of an option for the solver.
//
// Note that this is only valid to be called with options named by
// `fznso_option_list`, and the value must be of the correct type. This
// function returns `true` if the option was successfully set, and `false`
// otherwise.
//
// # Safety
//
// `solver` must be a pointer returned by [`fznso_solver_create`] that has not
// yet been passed to [`fznso_solver_free`].
bool fznso_solver_option_set(FznsoSolver *solver, const char *ident, FznsoValueRef value);

// Read an error message from the solver.
//
// This function is expected to be called after solver interactions signal an
// error has occurred. For example, if [`fznso_solver_run`] returns
// [`FznsoError`] or [`fznso_solver_option_set`] returns `false`.
//
// # Safety
//
// `solver` must be a pointer returned by [`fznso_solver_create`] that has not
// yet been passed to [`fznso_solver_free`].
void fznso_solver_read_error(FznsoSolver *solver,
                             void *context,
                             void (*read_error)(void *context, const char *error));

// Run the solver with the given model
//
// # Safety
//
// `solver` must be a pointer returned by [`fznso_solver_create`] that has not
// yet been passed to [`fznso_solver_free`].
FznsoStatus fznso_solver_run(FznsoSolver *solver,
                             FznsoModelRef model,
                             void *context,
                             void (*on_solution)(void *context, FznsoSolutionRef solution));

// Returns the list of available statistical information that can be requested
// from the solver and its solutions.
FznsoStatisticList fznso_statistic_list(void);
