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
//! Prefixing every symbol this way lets several solvers be linked into one
//! binary without colliding, and keeps them out of any wrapped library's
//! namespace. The naming rules are specified at
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

/// Returns the list of available constraint that can be added to the solver.
///
/// The returned array must stay valid for as long as the library is loaded, so
/// it is `static`. The caller never frees it.
#[unsafe(no_mangle)]
pub extern "C" fn fznso_NAME_constraint_list() -> FznsoConstraintList<'static> {
	unimplemented!()
}

/// Returns the list of types for which decision variable can be created by the
/// solver.
///
/// The returned array must stay valid for as long as the library is loaded, so
/// it is `static`. The caller never frees it.
#[unsafe(no_mangle)]
pub extern "C" fn fznso_NAME_decision_list() -> FznsoTypeList<'static> {
	unimplemented!()
}

/// Returns the list of available objective that can be achieved by the solver.
///
/// The returned array must stay valid for as long as the library is loaded, so
/// it is `static`. The caller never frees it.
#[unsafe(no_mangle)]
pub extern "C" fn fznso_NAME_objective_list() -> FznsoObjectiveList<'static> {
	unimplemented!()
}

/// Returns the list of available options that can be set of the solver.
///
/// The returned array must stay valid for as long as the library is loaded, so
/// it is `static`. The caller never frees it.
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
/// Note that this is only valid to be called with options named by
/// `fznso_NAME_option_list`.
///
/// The returned value must stay valid until the next call that can change the
/// solver (e.g. `_solver_option_set`, `_solver_run`, or `_solver_free`).
/// Returning a borrow of the solver's own state is the intent; the caller never
/// frees it.
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
/// Note that this is only valid to be called with options named by
/// `fznso_NAME_option_list`, and the value must be of the correct type. This
/// function returns `true` if the option was successfully set, and `false`
/// otherwise.
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
/// This function is expected to be called after solver interactions signal an
/// error has occurred. For example, if [`fznso_NAME_solver_run`] returns
/// [`FznsoStatus::FznsoError`] or [`fznso_NAME_solver_option_set`] returns
/// `false`.
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
/// Note that this is only valid to be called with statistics named by
/// `fznso_NAME_statistic_list` whose `solver` flag is set; statistics carrying
/// only the `solution` flag are read from a solution instead, using
/// `FznsoSolutionMethods::statistic`. An unknown identifier yields a value of
/// kind `FznsoValueAbsent`.
///
/// As with `_solver_option_get`, the returned value must stay valid until
/// the next call that can change the solver.
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

/// Returns the list of available statistical information that can be requested
/// from the solver and its solutions.
///
/// The returned array must stay valid for as long as the library is loaded, so
/// it is `static`. The caller never frees it.
#[unsafe(no_mangle)]
pub extern "C" fn fznso_NAME_statistic_list() -> FznsoStatisticList<'static> {
	unimplemented!()
}
