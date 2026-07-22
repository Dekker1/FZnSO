//! FZnSO Protocol Solver Template
//!
//! This crate defines the solver functionality to be implemented FZnSO
//! protocol, which allows the incremental usage of solvers that can solve
//! decision and optimization problems. The goal of the interface is to easily
//! use different solvers in a unified way, and to allow the dynamic loading of
//! solver libraries (as DLLs).

use core::ffi;

use fznso_types::{
	FznsoConstraintList, FznsoModelRef, FznsoObjectiveList, FznsoOptionList, FznsoPtr,
	FznsoSolutionRef, FznsoSolver, FznsoStatisticList, FznsoStatus, FznsoTypeList, FznsoValueRef,
};

#[unsafe(no_mangle)]
/// Returns the list of available constraint that can be added to the solver.
pub extern "C" fn fznso_constraint_list() -> FznsoConstraintList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of types for which decision variable can be created by the
/// solver.
pub extern "C" fn fznso_decision_list() -> FznsoTypeList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of available objective that can be achieved by the solver.
pub extern "C" fn fznso_objective_list() -> FznsoObjectiveList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of available options that can be set of the solver.
pub extern "C" fn fznso_option_list() -> FznsoOptionList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Create a new solver instance
pub extern "C" fn fznso_solver_create() -> *mut FznsoSolver {
	unimplemented!()
}

/// Free a solver instance, releasing all resources associated with it.
///
/// The pointer to the solver instance will be invalid after this function has
/// been called.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_solver_create`] that has not
/// yet been passed to this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fznso_solver_free(solver: *mut FznsoSolver) {
	let _ = solver;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Get the current value of an option for the solver.
///
/// Note that this is only valid to be called with options named by
/// `fznso_option_list`.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_solver_create`] that has not
/// yet been passed to [`fznso_solver_free`].
pub unsafe extern "C" fn fznso_solver_option_get(
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
/// `fznso_option_list`, and the value must be of the correct type. This
/// function returns `true` if the option was successfully set, and `false`
/// otherwise.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_solver_create`] that has not
/// yet been passed to [`fznso_solver_free`].
pub unsafe extern "C" fn fznso_solver_option_set(
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
/// error has occurred. For example, if [`fznso_solver_run`] returns
/// [`FznsoError`] or [`fznso_solver_option_set`] returns `false`.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_solver_create`] that has not
/// yet been passed to [`fznso_solver_free`].
pub unsafe extern "C" fn fznso_solver_read_error(
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
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_solver_create`] that has not
/// yet been passed to [`fznso_solver_free`].
pub unsafe extern "C" fn fznso_solver_run(
	solver: *mut FznsoSolver,
	model: FznsoModelRef,
	context: *mut ffi::c_void,
	on_solution: extern "C" fn(context: *mut ffi::c_void, solution: FznsoSolutionRef),
) -> FznsoStatus {
	let _ = solver;
	let _ = model;
	let _ = context;
	let _ = on_solution;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of available statistical information that can be requested
/// from the solver and its solutions.
pub extern "C" fn fznso_statistic_list() -> FznsoStatisticList<'static> {
	unimplemented!()
}
