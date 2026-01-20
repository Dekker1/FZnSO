use std::{
	ffi::{self, CStr},
	mem,
	sync::Arc,
};

use fznso_types::{
	FznsoConstraintList, FznsoModelRef, FznsoObjectiveList, FznsoOptionList, FznsoSolutionRef,
	FznsoSolver, FznsoStatus, FznsoTypeList, FznsoValueRef,
};
use libloading::Symbol;

use crate::{Model, Solution, Solver, Status, model::ModelWrapper};

#[derive(Debug)]
pub struct Library {
	_lib: libloading::Library,
	constraint_list: Symbol<'static, unsafe extern "C" fn() -> FznsoConstraintList<'static>>,
	decision_list: Symbol<'static, unsafe extern "C" fn() -> FznsoTypeList<'static>>,
	objective_list: Symbol<'static, unsafe extern "C" fn() -> FznsoObjectiveList<'static>>,
	option_list: Symbol<'static, unsafe extern "C" fn() -> FznsoOptionList<'static>>,
	solver_create: Symbol<'static, unsafe extern "C" fn() -> *mut FznsoSolver>,
	solver_free: Symbol<'static, unsafe extern "C" fn(*mut FznsoSolver)>,
	solver_option_get: Symbol<
		'static,
		unsafe extern "C" fn(&FznsoSolver, *const ffi::c_char) -> FznsoValueRef<'_>,
	>,
	solver_option_set: Symbol<
		'static,
		unsafe extern "C" fn(*mut FznsoSolver, *const ffi::c_char, FznsoValueRef<'_>) -> bool,
	>,
	solver_read_error: Symbol<
		'static,
		unsafe extern "C" fn(
			*const FznsoSolver,
			*mut ffi::c_void,
			extern "C" fn(*mut ffi::c_void, *const ffi::c_char),
		),
	>,
	solver_run: Symbol<
		'static,
		unsafe extern "C" fn(
			*const FznsoSolver,
			FznsoModelRef,
			*mut ffi::c_void,
			extern "C" fn(*mut ffi::c_void, FznsoSolutionRef),
		) -> FznsoStatus,
	>,
}

impl Library {
	/// Internal method to get a [`Symbol`] from a library in [`Self::new`].
	unsafe fn get_symbol<T>(
		lib: &libloading::Library,
		symbol: &[u8],
	) -> Result<Symbol<'static, T>, libloading::Error> {
		let sym: Symbol<'static, T> = unsafe { mem::transmute(lib.get::<T>(symbol)?) };
		Ok(sym)
	}

	/// Creates a new [`Solver`] instance using the functions in the
	/// [`Library`].
	pub fn create_solver(self: &Arc<Self>) -> Solver {
		let solver = unsafe { (*self.solver_create)() };
		Solver {
			solver,
			lib: self.clone(),
		}
	}

	/// Find and load a dynamic FZnSO library.
	///
	/// Note that this method follows the same conventions as
	/// [`libloading::Library::new`], but in addition also finds and loads the
	/// FZnSO specific symbols.
	pub unsafe fn new<S: AsRef<ffi::OsStr>>(filename: S) -> Result<Arc<Self>, libloading::Error> {
		let lib = unsafe {
			let lib = libloading::Library::new(filename)?;
			let constraint_list = Self::get_symbol(&lib, b"fznso_constraint_list\0")?;
			let decision_list = Self::get_symbol(&lib, b"fznso_decision_list\0")?;
			let objective_list = Self::get_symbol(&lib, b"fznso_objective_list\0")?;
			let option_list = Self::get_symbol(&lib, b"fznso_option_list\0")?;
			let solver_create = Self::get_symbol(&lib, b"fznso_solver_create\0")?;
			let solver_free = Self::get_symbol(&lib, b"fznso_solver_free\0")?;
			let solver_option_get = Self::get_symbol(&lib, b"fznso_solver_option_get\0")?;
			let solver_option_set = Self::get_symbol(&lib, b"fznso_solver_option_set\0")?;
			let solver_read_error = Self::get_symbol(&lib, b"fznso_solver_read_error\0")?;
			let solver_run = Self::get_symbol(&lib, b"fznso_solver_run\0")?;
			Self {
				_lib: lib,
				constraint_list,
				decision_list,
				objective_list,
				option_list,
				solver_create,
				solver_free,
				solver_option_get,
				solver_option_set,
				solver_read_error,
				solver_run,
			}
		};
		Ok(Arc::new(lib))
	}

	/// Get the list of constraints supported by solvers creating using the
	/// loaded library.
	pub fn constraint_types(&self) -> FznsoConstraintList<'_> {
		unsafe { (*self.constraint_list)() }
	}

	/// Get the list of decision variable types supported by solvers creating
	/// using the loaded library.
	pub fn decision_types(&self) -> FznsoTypeList<'_> {
		unsafe { (*self.decision_list)() }
	}

	/// Get the list of the types of objectives supported by solvers creating
	/// using the loaded library.
	pub fn objective_list(&self) -> FznsoObjectiveList<'_> {
		unsafe { (*self.objective_list)() }
	}

	/// Get the list of options supported by the loaded library.
	pub fn option_list(&self) -> FznsoOptionList<'static> {
		unsafe { (*self.option_list)() }
	}

	pub(crate) fn solver_free(&self, solver: *mut FznsoSolver) {
		unsafe { (*self.solver_free)(solver) }
	}

	pub(crate) fn solver_option_get(
		&self,
		solver: *const FznsoSolver,
		name: &str,
	) -> FznsoValueRef<'_> {
		let cname = ffi::CString::new(name).unwrap();
		unsafe { (*self.solver_option_get)(&*solver, cname.as_ptr()) }
	}

	pub(crate) fn solver_option_set(
		&self,
		solver: *mut FznsoSolver,
		name: &str,
		value: FznsoValueRef<'_>,
	) -> Result<(), String> {
		let cname = ffi::CString::new(name).unwrap();
		let ok = unsafe { (*self.solver_option_set)(solver, cname.as_ptr(), value) };
		if !ok {
			let mut err = String::default();
			self.solver_read_error(solver, |msg| err.push_str(msg));
			Err(err)
		} else {
			Ok(())
		}
	}

	pub fn solver_read_error<F: FnMut(&str)>(&self, solver: *const FznsoSolver, mut read_err: F) {
		let ctx: *mut F = &mut read_err;
		unsafe {
			(*self.solver_read_error)(solver, ctx as *mut ffi::c_void, wrap_read_error_cb::<F>)
		}
	}

	pub(crate) fn solver_run<M, F>(
		&self,
		solver: *const FznsoSolver,
		model: &M,
		mut on_sol: F,
	) -> Status
	where
		M: Model + 'static,
		F: FnMut(&Solution),
	{
		let model = ModelWrapper(model).into();
		let ctx: *mut F = &mut on_sol;
		let status = unsafe {
			(*self.solver_run)(
				solver,
				model,
				ctx as *mut ffi::c_void,
				wrap_on_solution_cb::<F>,
			)
		};

		match status {
			FznsoStatus::FznsoComplete => Status::Complete,
			FznsoStatus::FznsoIncomplete => Status::Incomplete,
			FznsoStatus::FznsoError => {
				let mut err = String::default();
				self.solver_read_error(solver, |msg| err.push_str(msg));
				Status::Error(err)
			}
		}
	}
}

extern "C" fn wrap_read_error_cb<F: FnMut(&str)>(ctx: *mut ffi::c_void, msg: *const ffi::c_char) {
	let ctx = ctx as *mut F;
	let read_err = unsafe { &mut *ctx };
	let msg = unsafe { CStr::from_ptr(msg).to_str().unwrap() };
	read_err(msg);
}

extern "C" fn wrap_on_solution_cb<F: FnMut(&Solution)>(
	ctx: *mut ffi::c_void,
	sol: FznsoSolutionRef<'_>,
) {
	let ctx = ctx as *mut F;
	let on_sol = unsafe { &mut *ctx };
	let sol = Solution { inner: sol };
	on_sol(&sol);
}
