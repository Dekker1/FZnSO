/// @file
/// FZnSO solver entry points to implement in C.
///
/// This is a template to copy, not a header to include: replace every `NAME`
/// with your solver's name and fill in the bodies.
///
/// The `fznso_` marker is fixed; `NAME` is the base name of the built library
/// file (e.g. `gecode` for `libgecode.so`). Prefixing every symbol this way lets
/// several solvers be linked into one binary without colliding — with each other
/// or with any wrapped library — and lets a loader recover the name from the file
/// name. The naming rules are specified at
/// <https://fznso.minizinc.dev/spec/naming-and-versioning/>.
///
/// The rules that hold across the whole interface — opaque handles, accessor
/// preconditions, what stays valid for how long, and how the version is matched —
/// are stated once at the top of `fznso_types.h`.

#include "fznso_types.h"

/**
 * Report the FZnSO ABI version this solver was built against. A loader rejects
 * the solver if this does not match its own.
 */
uint32_t fznso_NAME_abi_version(void);

/**
 * Returns the list of available constraint that can be added to the solver.
 *
 * The returned array must stay valid for as long as the library is loaded, so
 * it is `static`. The caller never frees it.
 */
FznsoConstraintList fznso_NAME_constraint_list(void);

/**
 * Returns the list of types for which decision variable can be created by the
 * solver.
 *
 * The returned array must stay valid for as long as the library is loaded, so
 * it is `static`. The caller never frees it.
 */
FznsoTypeList fznso_NAME_decision_list(void);

/**
 * Returns the list of available objective that can be achieved by the solver.
 *
 * The returned array must stay valid for as long as the library is loaded, so
 * it is `static`. The caller never frees it.
 */
FznsoObjectiveList fznso_NAME_objective_list(void);

/**
 * Returns the list of available options that can be set of the solver.
 *
 * The returned array must stay valid for as long as the library is loaded, so
 * it is `static`. The caller never frees it.
 */
FznsoOptionList fznso_NAME_option_list(void);

/**
 * Create a new solver instance
 */
FznsoSolver *fznso_NAME_solver_create(void);

/**
 * Free a solver instance, releasing all resources associated with it.
 *
 * The pointer to the solver instance will be invalid after this function has
 * been called.
 *
 * # Safety
 *
 * `solver` must be a pointer returned by fznso_NAME_solver_create that has
 * not yet been passed to this function.
 */
void fznso_NAME_solver_free(FznsoSolver *solver);

/**
 * Get the current value of an option for the solver.
 *
 * Note that this is only valid to be called with options named by
 * `fznso_NAME_option_list`.
 *
 * The returned value must stay valid until the next call that can change the
 * solver (e.g. `_solver_option_set`, `_solver_run`, or `_solver_free`).
 * Returning a borrow of the solver's own state is the intent; the caller never
 * frees it.
 *
 * # Safety
 *
 * `solver` must be a pointer returned by fznso_NAME_solver_create that has
 * not yet been passed to fznso_NAME_solver_free.
 */
FznsoValueRef fznso_NAME_solver_option_get(const FznsoSolver * solver, FznsoStr ident);

/**
 * Set the current value of an option for the solver.
 *
 * Note that this is only valid to be called with options named by
 * `fznso_NAME_option_list`, and the value must be of the correct type. This
 * function returns `true` if the option was successfully set, and `false`
 * otherwise.
 *
 * # Safety
 *
 * `solver` must be a pointer returned by fznso_NAME_solver_create that has
 * not yet been passed to fznso_NAME_solver_free.
 */
bool fznso_NAME_solver_option_set(FznsoSolver *solver, FznsoStr ident, FznsoValueRef value);

/**
 * Read an error message from the solver.
 *
 * This function is expected to be called after solver interactions signal an
 * error has occurred. For example, if fznso_NAME_solver_run returns
 * FznsoStatus::FznsoError or fznso_NAME_solver_option_set returns
 * `false`.
 *
 * The message is handed to `read_error` rather than returned, so it need only
 * stay valid for the duration of that callback.
 *
 * # Safety
 *
 * `solver` must be a pointer returned by fznso_NAME_solver_create that has
 * not yet been passed to fznso_NAME_solver_free.
 */
void fznso_NAME_solver_read_error(FznsoSolver *solver,
                                  void *context,
                                  void (*read_error)(void *context, FznsoStr error));

/**
 * Run the solver with the given model.
 *
 * `model` is borrowed for the duration of this call and is read-only
 * throughout. The solution passed to `on_solution`, and the scope and value
 * passed to `on_message`, are likewise valid only for the duration of that one
 * callback.
 *
 * Both `on_message` and `should_stop` may be None, and the difference is
 * worth honouring: `None` means *nobody is listening* and *the caller will
 * never cancel* respectively, so a solver should skip building diagnostics and
 * drop the poll from its search loop, rather than doing the work and
 * discarding it.
 *
 * Failures are not reported through `on_message`. Return
 * FznsoStatus::FznsoError and make the reason available through
 * fznso_NAME_solver_read_error.
 *
 * The full contracts — when `should_stop` must be polled, and what a
 * multi-threaded solver may do with the callbacks and the model — are
 * specified once at
 * <https://fznso.minizinc.dev/spec/running/> and
 * <https://fznso.minizinc.dev/spec/threading/>.
 *
 * # Safety
 *
 * `solver` must be a pointer returned by fznso_NAME_solver_create that has
 * not yet been passed to fznso_NAME_solver_free.
 */
FznsoStatus fznso_NAME_solver_run(FznsoSolver *solver,
                                  FznsoModelRef model,
                                  void *context,
                                  void (*on_solution)(void *context, FznsoSolutionRef solution),
                                  void (*on_message)(void *context,
                                                     FznsoStr scope,
                                                     FznsoValueRef value),
                                  bool (*should_stop)(void *context));

/**
 * Get the current value of a statistic from the solver instance.
 *
 * Note that this is only valid to be called with statistics named by
 * `fznso_NAME_statistic_list` whose `solver` flag is set; statistics carrying
 * only the `solution` flag are read from a solution instead, using
 * `FznsoSolutionMethods::statistic`. An unknown identifier yields a value of
 * kind `FznsoValueAbsent`.
 *
 * As with `_solver_option_get`, the returned value must stay valid until
 * the next call that can change the solver.
 *
 * # Safety
 *
 * `solver` must be a pointer returned by fznso_NAME_solver_create that has
 * not yet been passed to fznso_NAME_solver_free.
 */
FznsoValueRef fznso_NAME_solver_statistic(const FznsoSolver * solver, FznsoStr ident);

/**
 * Returns the list of available statistical information that can be requested
 * from the solver and its solutions.
 *
 * The returned array must stay valid for as long as the library is loaded, so
 * it is `static`. The caller never frees it.
 */
FznsoStatisticList fznso_NAME_statistic_list(void);
