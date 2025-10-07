//! IPDOS Protocol Solver Template
//!
//! This crate defines the solver functionality to be implemented IPDOS
//! protocol, which allows the incremental usage of solvers that can solve
//! decision and optimization problems. The goal of the interface is to easily
//! use different solvers in a unified way, and to allow the dynamic loading of
//! solver libraries (as DLLs).

use core::ffi;

use ipdos_types::{
	IpdosConstraintList, IpdosModelRef, IpdosObjectiveList, IpdosOptionList, IpdosSolution,
	IpdosSolver, IpdosStatus, IpdosTypeList, IpdosValue,
};

#[unsafe(no_mangle)]
/// Returns the list of available constraint that can be added to the solver.
pub extern "C" fn ipdos_constraint_list() -> IpdosConstraintList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of types for which decision variable can be created by the
/// solver.
pub extern "C" fn ipdos_decision_list() -> IpdosTypeList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of available objective that can be achieved by the solver.
pub extern "C" fn ipdos_objective_list() -> IpdosObjectiveList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of available options that can be set of the solver.
pub extern "C" fn ipdos_option_list() -> IpdosOptionList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Create a new solver instance
pub extern "C" fn ipdos_solver_create() -> Box<IpdosSolver> {
	unimplemented!()
}

/// Free a solver instance, releasing all resources associated with it.
///
/// The pointer to the solver instance will be invalid after this function has
/// been called.
#[unsafe(no_mangle)]
pub extern "C" fn ipdos_solver_free(solver: Box<IpdosSolver>) {
	let _ = solver;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Get the current value of an option for the solver.
///
/// Note that this is only valid to be called with options named by
/// `ipdos_option_list`.
pub extern "C" fn ipdos_solver_option_get(
	solver: &IpdosSolver,
	ident: *const ffi::c_char,
) -> IpdosValue<'_> {
	let _ = solver;
	let _ = ident;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Set the current value of an option for the solver.
///
/// Note that this is only valid to be called with options named by
/// `ipdos_option_list`, and the value must be of the correct type.
pub extern "C" fn ipdos_solver_option_set(
	solver: &mut IpdosSolver,
	ident: *const ffi::c_char,
	value: IpdosValue<'_>,
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
/// error has occurred. For example, if [`ipdos_solver_run`] returns
/// [`IpdosError`] or [`ipdos_solver_push`] returns `false`.
pub extern "C" fn ipdos_solver_read_error(
	solver: &mut IpdosSolver,
	context: &mut ffi::c_void,
	read_error: extern "C" fn(context: &mut ffi::c_void, error: *const ffi::c_char),
) {
	let _ = solver;
	let _ = context;
	let _ = read_error;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Run the solver with the given model
///
/// cbindgen:no-export
pub extern "C" fn ipdos_solver_run(
	solver: &mut IpdosSolver,
	model: IpdosModelRef,
	context: &mut ffi::c_void,
	on_solution: extern "C" fn(context: &mut ffi::c_void, solution: &IpdosSolution),
) -> IpdosStatus {
	let _ = solver;
	let _ = model;
	let _ = context;
	let _ = on_solution;
	unimplemented!()
}
