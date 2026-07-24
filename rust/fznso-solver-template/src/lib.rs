//! FZnSO Protocol Solver Template
//!
//! This crate defines the solver functionality to be implemented FZnSO
//! protocol, which allows the incremental usage of solvers that can solve
//! decision and optimization problems. The goal of the interface is to easily
//! use different solvers in a unified way, and to allow the dynamic loading of
//! solver libraries (as DLLs).
//!
//! The entry points below are named `fznso_NAME_...`. The `fznso_` marker is
//! fixed; `NAME` is a placeholder that a real solver replaces with its library
//! name — the base name of the built library file (e.g. `gecode` for
//! `libgecode.so`). Prefixing every symbol this way lets several solvers be
//! linked into one binary without colliding, and keeps them out of any wrapped
//! library's namespace.

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

#[unsafe(no_mangle)]
/// Report the FZnSO ABI version this solver was built against. A loader rejects
/// the solver if this does not match its own.
pub extern "C" fn fznso_NAME_abi_version() -> u32 {
	fznso_types::FZNSO_ABI_VERSION
}

#[unsafe(no_mangle)]
/// Returns the list of available constraint that can be added to the solver.
pub extern "C" fn fznso_NAME_constraint_list() -> FznsoConstraintList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of types for which decision variable can be created by the
/// solver.
pub extern "C" fn fznso_NAME_decision_list() -> FznsoTypeList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of available objective that can be achieved by the solver.
pub extern "C" fn fznso_NAME_objective_list() -> FznsoObjectiveList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of available options that can be set of the solver.
pub extern "C" fn fznso_NAME_option_list() -> FznsoOptionList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Create a new solver instance
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

#[unsafe(no_mangle)]
/// Get the current value of an option for the solver.
///
/// Note that this is only valid to be called with options named by
/// `fznso_NAME_option_list`.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to [`fznso_NAME_solver_free`].
pub unsafe extern "C" fn fznso_NAME_solver_option_get(
	solver: FznsoPtr<'_, FznsoSolver>,
	ident: *const ffi::c_char,
) -> FznsoValueRef<'_> {
	let _ = solver;
	let _ = ident;
	unimplemented!()
}

#[unsafe(no_mangle)]
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
pub unsafe extern "C" fn fznso_NAME_solver_option_set(
	solver: *mut FznsoSolver,
	ident: *const ffi::c_char,
	value: FznsoValueRef<'_>,
) -> bool {
	let _ = solver;
	let _ = ident;
	let _ = value;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Read an error message from the solver.
///
/// This function is expected to be called after solver interactions signal an
/// error has occurred. For example, if [`fznso_NAME_solver_run`] returns
/// [`FznsoError`] or [`fznso_NAME_solver_option_set`] returns `false`.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to [`fznso_NAME_solver_free`].
pub unsafe extern "C" fn fznso_NAME_solver_read_error(
	solver: *mut FznsoSolver,
	context: *mut ffi::c_void,
	read_error: extern "C" fn(context: *mut ffi::c_void, error: *const ffi::c_char),
) {
	let _ = solver;
	let _ = context;
	let _ = read_error;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Run the solver with the given model
///
/// `on_message` reports non-fatal diagnostics: warnings, progress and search
/// information. `scope` names the kind of message and is dot-separated so that
/// callers can filter on a prefix. Failures are not reported here; they are
/// signalled by returning [`FznsoError`] and retrieved with
/// [`fznso_NAME_solver_read_error`].
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to [`fznso_NAME_solver_free`].
pub unsafe extern "C" fn fznso_NAME_solver_run(
	solver: *mut FznsoSolver,
	model: FznsoModelRef,
	context: *mut ffi::c_void,
	on_solution: extern "C" fn(context: *mut ffi::c_void, solution: FznsoSolutionRef),
	on_message: extern "C" fn(context: *mut ffi::c_void, scope: FznsoStr, value: FznsoValueRef),
) -> FznsoStatus {
	let _ = solver;
	let _ = model;
	let _ = context;
	let _ = on_solution;
	let _ = on_message;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of available statistical information that can be requested
/// from the solver and its solutions.
pub extern "C" fn fznso_NAME_statistic_list() -> FznsoStatisticList<'static> {
	unimplemented!()
}
