//! FZnSO Protocol Solver Template
//!
//! This crate declares the thirteen entry points a solver implements to speak
//! the FZnSO protocol, which allows the incremental usage of solvers that can
//! solve decision and optimization problems. The goal of the interface is to
//! easily use different solvers in a unified way, and to allow the dynamic
//! loading of solver libraries.
//!
//! It is a template to copy, not a library to link against. Every function
//! below is `unimplemented!()`.
//!
//! The entry points are named `fznso_NAME_...`. The `fznso_` marker is fixed;
//! `NAME` is a placeholder that a real solver replaces with its library name —
//! the base name of the built library file (e.g. `gecode` for `libgecode.so`).
//! Prefixing every symbol this way is what lets several solvers be statically
//! linked into one binary without colliding. The naming rules are specified at
//! <https://fznso.minizinc.dev/spec/naming-and-versioning/>.
//!
//! Each function below states its own contract. The rules that hold across the
//! whole interface — opaque handles, accessor preconditions, what stays valid
//! for how long, and how the version is matched — are specified once at
//! <https://fznso.minizinc.dev/spec/lifetimes-and-ownership/>.

#![allow(
	non_snake_case,
	reason = "the `fznso_NAME_` entry points are an illustrative placeholder prefix"
)]

use core::ffi;

use fznso_types::{
	FznsoConstraintList, FznsoModelRef, FznsoObjectiveList, FznsoOptionList, FznsoPtr,
	FznsoSolutionRef, FznsoSolver, FznsoStatisticList, FznsoStatus, FznsoStr, FznsoTypeList,
	FznsoValueRef,
};

/// Report the FZnSO ABI version this solver was built against. A loader rejects
/// the solver if this does not match its own.
#[unsafe(no_mangle)]
pub extern "C" fn fznso_NAME_abi_version() -> u32 {
	fznso_types::FZNSO_ABI_VERSION
}

/// Returns the constraints that can be posted to the solver.
///
/// The array is `static`: valid for as long as the library is loaded, and never
/// freed by the caller.
#[unsafe(no_mangle)]
pub extern "C" fn fznso_NAME_constraint_list() -> FznsoConstraintList<'static> {
	unimplemented!()
}

/// Returns the types for which the solver can create decision variables.
///
/// The array is `static`: valid for as long as the library is loaded, and never
/// freed by the caller.
#[unsafe(no_mangle)]
pub extern "C" fn fznso_NAME_decision_list() -> FznsoTypeList<'static> {
	unimplemented!()
}

/// Returns the objective strategies the solver can pursue.
///
/// The array is `static`: valid for as long as the library is loaded, and never
/// freed by the caller.
#[unsafe(no_mangle)]
pub extern "C" fn fznso_NAME_objective_list() -> FznsoObjectiveList<'static> {
	unimplemented!()
}

/// Returns the options that can be set on the solver.
///
/// The array is `static`: valid for as long as the library is loaded, and never
/// freed by the caller.
#[unsafe(no_mangle)]
pub extern "C" fn fznso_NAME_option_list() -> FznsoOptionList<'static> {
	unimplemented!()
}

/// Create a new solver instance
#[unsafe(no_mangle)]
pub extern "C" fn fznso_NAME_solver_create() -> *mut FznsoSolver {
	unimplemented!()
}

/// Free a solver instance, releasing all resources associated with it.
///
/// The pointer to the solver instance will be invalid after this function has
/// been called.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fznso_NAME_solver_free(solver: *mut FznsoSolver) {
	let _ = solver;
	unimplemented!()
}

/// Get the current value of an option for the solver.
///
/// Only valid for options named by `fznso_NAME_option_list`.
///
/// The returned value must stay valid until the next call that can change the
/// solver (`_solver_option_set`, `_solver_run` or `_solver_free`). Borrowing
/// the solver's own state is the intent; the caller never frees it.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to [`fznso_NAME_solver_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fznso_NAME_solver_option_get<'a>(
	solver: FznsoPtr<'a, FznsoSolver>,
	ident: FznsoStr<'_>,
) -> FznsoValueRef<'a> {
	let _ = solver;
	let _ = ident;
	unimplemented!()
}

/// Set the current value of an option for the solver.
///
/// Only valid for options named by `fznso_NAME_option_list`, and the value must
/// have the declared type. Returns `true` if the option was set, `false`
/// otherwise; the reason is then available through
/// [`fznso_NAME_solver_read_error`].
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to [`fznso_NAME_solver_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fznso_NAME_solver_option_set(
	solver: *mut FznsoSolver,
	ident: FznsoStr<'_>,
	value: FznsoValueRef<'_>,
) -> bool {
	let _ = solver;
	let _ = ident;
	let _ = value;
	unimplemented!()
}

/// Read an error message from the solver.
///
/// Called after an interaction signals an error: [`fznso_NAME_solver_run`]
/// returning [`FznsoStatus::FznsoError`], or
/// [`fznso_NAME_solver_option_set`] returning `false`.
///
/// The message is handed to `read_error` rather than returned, so it need only
/// stay valid for the duration of that callback.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to [`fznso_NAME_solver_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fznso_NAME_solver_read_error(
	solver: *mut FznsoSolver,
	context: *mut ffi::c_void,
	read_error: extern "C" fn(context: *mut ffi::c_void, error: FznsoStr<'_>),
) {
	let _ = solver;
	let _ = context;
	let _ = read_error;
	unimplemented!()
}

/// Run the solver with the given model.
///
/// `model` is borrowed for the duration of this call and is read-only
/// throughout. The solution passed to `on_solution`, and the scope and value
/// passed to `on_message`, are likewise valid only for the duration of that one
/// callback.
///
/// Both `on_message` and `should_stop` may be [`None`], and the difference is
/// worth honouring: `None` means *nobody is listening* and *the caller will
/// never cancel* respectively, so a solver should skip building diagnostics and
/// drop the poll from its search loop, rather than doing the work and
/// discarding it.
///
/// Failures are not reported through `on_message`. Return
/// [`FznsoStatus::FznsoError`] and make the reason available through
/// [`fznso_NAME_solver_read_error`].
///
/// The full contracts — when `should_stop` must be polled, and what a
/// multi-threaded solver may do with the callbacks and the model — are
/// specified once at
/// <https://fznso.minizinc.dev/spec/running/> and
/// <https://fznso.minizinc.dev/spec/threading/>.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to [`fznso_NAME_solver_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fznso_NAME_solver_run(
	solver: *mut FznsoSolver,
	model: FznsoModelRef,
	context: *mut ffi::c_void,
	on_solution: extern "C" fn(context: *mut ffi::c_void, solution: FznsoSolutionRef),
	on_message: Option<
		extern "C" fn(context: *mut ffi::c_void, scope: FznsoStr, value: FznsoValueRef),
	>,
	should_stop: Option<extern "C" fn(context: *mut ffi::c_void) -> bool>,
) -> FznsoStatus {
	let _ = solver;
	let _ = model;
	let _ = context;
	let _ = on_solution;
	let _ = on_message;
	let _ = should_stop;
	unimplemented!()
}

/// Get the current value of a statistic from the solver instance.
///
/// Only valid for statistics named by `fznso_NAME_statistic_list` whose
/// `solver` flag is set; those carrying only the `solution` flag are read from
/// a solution, using `FznsoSolutionMethods::statistic`. An unknown identifier
/// yields a value of kind `FznsoValueAbsent`.
///
/// As with `_solver_option_get`, the returned value must stay valid until the
/// next call that can change the solver.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to [`fznso_NAME_solver_free`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fznso_NAME_solver_statistic<'a>(
	solver: FznsoPtr<'a, FznsoSolver>,
	ident: FznsoStr<'_>,
) -> FznsoValueRef<'a> {
	let _ = solver;
	let _ = ident;
	unimplemented!()
}

/// Returns the statistics that can be read from the solver and its solutions.
///
/// The array is `static`: valid for as long as the library is loaded, and never
/// freed by the caller.
#[unsafe(no_mangle)]
pub extern "C" fn fznso_NAME_statistic_list() -> FznsoStatisticList<'static> {
	unimplemented!()
}
