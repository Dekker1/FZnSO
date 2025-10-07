#include "ipdos_types.h"

// Returns the list of available constraint that can be added to the solver.
IpdosConstraintList ipdos_constraint_list(void);

// Returns the list of types for which decision variable can be created by the
// solver.
IpdosTypeList ipdos_decision_list(void);

// Returns the list of available objective that can be achieved by the solver.
IpdosObjectiveList ipdos_objective_list(void);

// Returns the list of available options that can be set of the solver.
IpdosOptionList ipdos_option_list(void);

// Create a new solver instance
IpdosSolver *ipdos_solver_create(void);

// Free a solver instance, releasing all resources associated with it.
//
// The pointer to the solver instance will be invalid after this function has
// been called.
void ipdos_solver_free(IpdosSolver *solver);

// Get the current value of an option for the solver.
//
// Note that this is only valid to be called with options named by
// `ipdos_option_list`.
IpdosValue ipdos_solver_option_get(const IpdosSolver *solver, const char *ident);

// Set the current value of an option for the solver.
//
// Note that this is only valid to be called with options named by
// `ipdos_option_list`, and the value must be of the correct type.
bool ipdos_solver_option_set(IpdosSolver *solver, const char *ident, IpdosValue value);

// Read an error message from the solver.
//
// This function is expected to be called after solver interactions signal an
// error has occurred. For example, if [`ipdos_solver_run`] returns
// [`IpdosError`] or [`ipdos_solver_push`] returns `false`.
void ipdos_solver_read_error(IpdosSolver *solver,
                             void *context,
                             void (*read_error)(void *context, const char *error));
