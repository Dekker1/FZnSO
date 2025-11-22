//! FZnSO Protocol Library
//!
//! This crate defines an interface for the FZnSO protocol, which allows the
//! incremental usage of solvers that can solve decision and optimization
//! problems. The goal of the interface is to easily use different solvers in a
//! unified way, and to allow the dynamic loading of solver libraries (as DLLs).

use fznso_types as _;
use libloading as _;

// use std::{ffi, mem, sync::Arc};
// use fznso_types::{
// 	FznsoConstraintList, FznsoModelRef, FznsoObjectiveList, FznsoOptionList,
// FznsoSolutionRef, 	FznsoSolver, FznsoStatus, FznsoTypeList, FznsoValueRef,
// };
// use libloading::Symbol;

// #[derive(Debug)]
// pub struct Library {
// 	_lib: libloading::Library,
// 	constraint_list: Symbol<'static, unsafe extern "C" fn() ->
// FznsoConstraintList<'static>>, 	decision_list: Symbol<'static, unsafe extern
// "C" fn() -> FznsoTypeList<'static>>, 	objective_list: Symbol<'static, unsafe
// extern "C" fn() -> FznsoObjectiveList<'static>>, 	option_list:
// Symbol<'static, unsafe extern "C" fn() -> FznsoOptionList<'static>>,
// 	solver_create: Symbol<'static, unsafe extern "C" fn() -> *mut FznsoSolver>,
// 	solver_free: Symbol<'static, unsafe extern "C" fn(*mut FznsoSolver)>,
// 	solver_option_get: Symbol<
// 		'static,
// 		unsafe extern "C" fn(&FznsoSolver, *const ffi::c_char) ->
// FznsoValueRef<'_>, 	>,
// 	solver_option_set: Symbol<
// 		'static,
// 		unsafe extern "C" fn(&FznsoSolver, *const ffi::c_char, FznsoValueRef<'_>)
// -> bool, 	>,
// 	solver_read_error: Symbol<
// 		'static,
// 		unsafe extern "C" fn(
// 			*const FznsoSolver,
// 			&mut ffi::c_void,
// 			extern "C" fn(&mut ffi::c_void, *const ffi::c_char),
// 		),
// 	>,
// 	solver_run: Symbol<
// 		'static,
// 		unsafe extern "C" fn(
// 			*const FznsoSolver,
// 			FznsoModelRef,
// 			&mut ffi::c_void,
// 			extern "C" fn(&mut ffi::c_void, FznsoSolutionRef),
// 		),
// 	>,
// }

// /// Methods provided to a [`Solver`] to interact with model instances
// provided /// to be solved.
// pub trait Model {}

// #[derive(Debug)]
// /// Representation of a solver instance, created from a [`Library`].
// pub struct Solver {
// 	solver: *mut FznsoSolver,
// 	lib: Arc<Library>,
// }

// /// Methods for interacting with solutions emitted by the solver when running
// a /// model.
// pub trait Solution {
// 	/// Get the value that a decision, identified using its index, takes in the
// 	/// solution.
// 	fn get_value(&self, decision_index: usize) -> FznsoValueRef<'_>;
// }

// impl Library {
// 	/// Internal method to get a [`Symbol`] from a library in [`Self::new`].
// 	///
// 	/// The
// 	unsafe fn get_symbol<T>(
// 		lib: &libloading::Library,
// 		symbol: &[u8],
// 	) -> Result<Symbol<'static, T>, libloading::Error> {
// 		let sym: Symbol<'static, T> = unsafe {
// mem::transmute(lib.get::<T>(symbol)?) }; 		Ok(sym)
// 	}

// 	/// Creates a new [`Solver`] instance using the functions in the
// 	/// [`Library`].
// 	pub fn create_solver(self: &Arc<Self>) -> Solver {
// 		let solver = unsafe { (*self.solver_create)() };
// 		Solver {
// 			solver,
// 			lib: self.clone(),
// 		}
// 	}

// 	/// Find and load a dynamic FZnSO library.
// 	///
// 	/// Note that this method follows the same conventions as
// 	/// [`libloading::Library::new`], but in addition also finds and loads the
// 	/// FZnSO specific symbols.
// 	pub unsafe fn new<S: AsRef<ffi::OsStr>>(filename: S) -> Result<Arc<Self>,
// libloading::Error> { 		let lib = unsafe {
// 			let lib = libloading::Library::new(filename)?;
// 			let constraint_list = Self::get_symbol(&lib, b"fznso_constraint_list\0")?;
// 			let decision_list = Self::get_symbol(&lib, b"fznso_decision_list\0")?;
// 			let objective_list = Self::get_symbol(&lib, b"fznso_objective_list\0")?;
// 			let option_list = Self::get_symbol(&lib, b"fznso_option_list\0")?;
// 			let solver_create = Self::get_symbol(&lib, b"fznso_solver_create\0")?;
// 			let solver_free = Self::get_symbol(&lib, b"fznso_solver_free\0")?;
// 			let solver_option_get = Self::get_symbol(&lib,
// b"fznso_solver_option_get\0")?; 			let solver_option_set =
// Self::get_symbol(&lib, b"fznso_solver_option_set\0")?;
// 			let solver_read_error = Self::get_symbol(&lib,
// b"fznso_solver_read_error\0")?; 			let solver_run = Self::get_symbol(&lib,
// b"fznso_solver_run\0")?; 			Self {
// 				_lib: lib,
// 				constraint_list,
// 				decision_list,
// 				objective_list,
// 				option_list,
// 				solver_create,
// 				solver_free,
// 				solver_option_get,
// 				solver_option_set,
// 				solver_read_error,
// 				solver_run,
// 			}
// 		};
// 		Ok(Arc::new(lib))
// 	}
// }

// impl Solver {
// 	/// Run the given model instance on the solver, calling the provided
// 	/// callback for each solution found, and returning the status of the run.
// 	pub fn run(
// 		&mut self,
// 		model: impl Model,
// 		on_solution: impl FnMut(FznsoSolutionRef),
// 	) -> FznsoStatus {
// 		todo!()
// 	}
// }

// impl Drop for Solver {
// 	fn drop(&mut self) {
// 		unsafe { (*self.lib.solver_free)(self.solver) }
// 	}
// }

// unsafe impl Send for Solver {}

// #[cfg(test)]
// mod tests {}
