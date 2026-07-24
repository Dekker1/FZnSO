//! Dynamic loading of FZnSO solver libraries.

use std::{
	ffi::{self, CStr},
	mem,
	sync::Arc,
};

use fznso_types::{
	FznsoConstraintList, FznsoModelRef, FznsoObjectiveList, FznsoOptionList, FznsoPtr,
	FznsoSolutionRef, FznsoSolver, FznsoStatisticList, FznsoStatus, FznsoStr, FznsoTypeList,
	FznsoValueRef,
};
use libloading::Symbol;

use crate::{DynSolution, DynSolver, Model, Status};

/// A dynamically loaded FZnSO solver library.
///
/// Holds [`libloading`] symbols for every required ABI entry point.  Normally
/// obtained via [`Library::new`] and held behind an [`Arc`] so that
/// [`DynSolver`] instances can share ownership.
#[derive(Debug)]
pub struct Library {
	/// The underlying dynamic library handle; kept alive as long as this struct
	/// lives.
	_lib: libloading::Library,
	/// Symbol for `fznso_<name>_constraint_list`.
	constraint_list: Symbol<'static, unsafe extern "C" fn() -> FznsoConstraintList<'static>>,
	/// Symbol for `fznso_<name>_decision_list`.
	decision_list: Symbol<'static, unsafe extern "C" fn() -> FznsoTypeList<'static>>,
	/// Symbol for `fznso_<name>_objective_list`.
	objective_list: Symbol<'static, unsafe extern "C" fn() -> FznsoObjectiveList<'static>>,
	/// Symbol for `fznso_<name>_option_list`.
	option_list: Symbol<'static, unsafe extern "C" fn() -> FznsoOptionList<'static>>,
	/// Symbol for `fznso_<name>_solver_create`.
	solver_create: Symbol<'static, unsafe extern "C" fn() -> *mut FznsoSolver>,
	/// Symbol for `fznso_<name>_solver_free`.
	solver_free: Symbol<'static, unsafe extern "C" fn(*mut FznsoSolver)>,
	/// Symbol for `fznso_<name>_solver_option_get`.
	solver_option_get: Symbol<
		'static,
		unsafe extern "C" fn(FznsoPtr<'_, FznsoSolver>, *const ffi::c_char) -> FznsoValueRef<'_>,
	>,
	/// Symbol for `fznso_<name>_solver_option_set`.
	solver_option_set: Symbol<
		'static,
		unsafe extern "C" fn(*mut FznsoSolver, *const ffi::c_char, FznsoValueRef<'_>) -> bool,
	>,
	/// Symbol for `fznso_<name>_solver_read_error`.
	solver_read_error: Symbol<
		'static,
		unsafe extern "C" fn(
			*mut FznsoSolver,
			*mut ffi::c_void,
			extern "C" fn(*mut ffi::c_void, *const ffi::c_char),
		),
	>,
	/// Symbol for `fznso_<name>_solver_run`.
	solver_run: Symbol<
		'static,
		unsafe extern "C" fn(
			*mut FznsoSolver,
			FznsoModelRef,
			*mut ffi::c_void,
			extern "C" fn(*mut ffi::c_void, FznsoSolutionRef),
			extern "C" fn(*mut ffi::c_void, FznsoStr<'_>, FznsoValueRef<'_>),
		) -> FznsoStatus,
	>,
	/// Symbol for `fznso_<name>_statistic_list`.
	statistic_list: Symbol<'static, unsafe extern "C" fn() -> FznsoStatisticList<'static>>,
}

/// ABI callback passed to `fznso_solver_run`; wraps a [`FznsoSolutionRef`] in
/// a [`DynSolution`] and forwards it to the Rust closure stored behind `ctx`.
/// The two Rust callbacks `fznso_solver_run` needs, behind the single `context`
/// pointer the ABI provides.
struct RunCallbacks<'c, F, G> {
	/// Invoked for each solution the solver reports.
	on_solution: &'c mut F,
	/// Invoked for each diagnostic the solver reports.
	on_message: &'c mut G,
}

/// ABI callback passed to `fznso_solver_run`; forwards a diagnostic to the Rust
/// closure stored behind `ctx`.
extern "C" fn wrap_on_message_cb<F, G>(
	ctx: *mut ffi::c_void,
	scope: FznsoStr<'_>,
	value: FznsoValueRef<'_>,
) where
	F: for<'s> FnMut(&'s DynSolution<'s>),
	G: FnMut(&str, FznsoValueRef<'_>),
{
	let ctx = ctx as *mut RunCallbacks<'_, F, G>;
	// SAFETY: ctx was created from &mut RunCallbacks in solver_run; the callback
	// is only invoked during that call, so it is still live.
	let cbs = unsafe { &mut *ctx };
	// SAFETY: the solver guarantees `scope` is valid UTF-8 for this call.
	let scope = unsafe { scope.as_str() }.unwrap_or_default();
	(cbs.on_message)(scope, value);
}

/// ABI callback passed to `fznso_solver_run`; wraps a [`FznsoSolutionRef`] in
/// a [`DynSolution`] and forwards it to the Rust closure stored behind `ctx`.
extern "C" fn wrap_on_solution_cb<F, G>(ctx: *mut ffi::c_void, sol: FznsoSolutionRef<'_>)
where
	F: for<'s> FnMut(&'s DynSolution<'s>),
	G: FnMut(&str, FznsoValueRef<'_>),
{
	let ctx = ctx as *mut RunCallbacks<'_, F, G>;
	// SAFETY: ctx was created from &mut RunCallbacks in solver_run; the callback
	// is only invoked during that call, so it is still live.
	let cbs = unsafe { &mut *ctx };
	let sol = DynSolution { inner: sol };
	(cbs.on_solution)(&sol);
}

/// ABI callback passed to `fznso_solver_read_error`; receives a C string and
/// forwards it as `&str` to the Rust closure stored behind `ctx`.
extern "C" fn wrap_read_error_cb<F: FnMut(&str)>(ctx: *mut ffi::c_void, msg: *const ffi::c_char) {
	let ctx = ctx as *mut F;
	// SAFETY: ctx was created from &mut F in solver_read_error; the callback is
	// only invoked during that call, so F is still live.
	let read_err = unsafe { &mut *ctx };
	// SAFETY: msg is a valid null-terminated C string provided by the library.
	let msg = unsafe { CStr::from_ptr(msg).to_str().unwrap() };
	read_err(msg);
}

/// Derive a solver's library name from its file name, borrowing from it.
///
/// Strips a leading `lib` and everything from the first `.`, so
/// `libgecode.6.2.1.dylib`, `libgecode.so.6` and `gecode.dll` all yield
/// `gecode`. A non-UTF-8 file name yields `""` — it cannot name ASCII symbols,
/// so loading then fails.
fn name_from_filename(filename: &ffi::OsStr) -> &str {
	let base = std::path::Path::new(filename)
		.file_name()
		.and_then(|n| n.to_str())
		.unwrap_or("");
	let name = base.strip_prefix("lib").unwrap_or(base);
	name.split('.').next().unwrap_or(name)
}

impl Library {
	/// Get the list of constraints supported by solvers from this library.
	pub fn constraint_types(&self) -> FznsoConstraintList<'_> {
		// SAFETY: The symbol is a valid function pointer for as long as self lives.
		unsafe { (*self.constraint_list)() }
	}

	/// Creates a new [`DynSolver`] instance from this library.
	pub fn create_solver(self: &Arc<Self>) -> DynSolver {
		// SAFETY: fznso_solver_create returns an owning pointer; ownership is
		// transferred to DynSolver, which calls fznso_solver_free on drop.
		let solver = unsafe { (*self.solver_create)() };
		DynSolver {
			solver,
			lib: Arc::clone(self),
		}
	}

	/// Get the list of decision-variable types supported by solvers from this
	/// library.
	pub fn decision_types(&self) -> FznsoTypeList<'_> {
		// SAFETY: The symbol is a valid function pointer for as long as self lives.
		unsafe { (*self.decision_list)() }
	}

	/// Internal helper: loads a named symbol from `lib`, extending its lifetime
	/// to `'static` by tying it to the Library owner.
	///
	/// # Safety
	///
	/// The caller must ensure the returned symbol is not used after `lib` is
	/// dropped.  [`Library`] upholds this by storing `_lib` alongside every
	/// symbol it loads.
	unsafe fn get_symbol<T>(
		lib: &libloading::Library,
		symbol: &[u8],
	) -> Result<Symbol<'static, T>, libloading::Error> {
		// SAFETY: The symbol's true lifetime is that of `lib`. The Library struct
		// keeps `_lib` alive as long as any symbol is reachable, making 'static safe.
		let sym: Symbol<'static, T> = unsafe { mem::transmute(lib.get::<T>(symbol)?) };
		Ok(sym)
	}

	/// Dynamically load a FZnSO solver library from `filename`.
	///
	/// The solver's entry points are `fznso_<name>_...`, where `<name>` is
	/// recovered from the file name: the base name with any leading `lib` and
	/// everything from the first `.` removed. So `libgecode.so.6`,
	/// `libgecode.6.2.1.dylib` and `gecode.dll` all load a solver whose entry
	/// points are prefixed `fznso_gecode_`. This is the only supported naming,
	/// so the file name and the exported symbols must agree.
	///
	/// # Safety
	///
	/// The library at `filename` must export valid implementations of all
	/// eleven FZnSO ABI functions under the `fznso_<name>_` prefix
	/// (`fznso_<name>_constraint_list`, `fznso_<name>_decision_list`,
	/// `fznso_<name>_objective_list`, `fznso_<name>_option_list`,
	/// `fznso_<name>_statistic_list`, `fznso_<name>_solver_create`,
	/// `fznso_<name>_solver_free`, `fznso_<name>_solver_option_get`,
	/// `fznso_<name>_solver_option_set`, `fznso_<name>_solver_read_error`,
	/// `fznso_<name>_solver_run`). Behaviour is undefined if any symbol is
	/// missing or does not conform to the FZnSO ABI.
	pub unsafe fn new<S: AsRef<ffi::OsStr>>(filename: S) -> Result<Arc<Self>, libloading::Error> {
		let filename = filename.as_ref();
		let name = name_from_filename(filename);
		// SAFETY: Caller guarantees the library is a valid FZnSO solver DLL.
		let lib = unsafe {
			let lib = libloading::Library::new(filename)?;
			let sym = |suffix: &str| format!("fznso_{name}{suffix}\0");
			let constraint_list = Self::get_symbol(&lib, sym("_constraint_list").as_bytes())?;
			let decision_list = Self::get_symbol(&lib, sym("_decision_list").as_bytes())?;
			let objective_list = Self::get_symbol(&lib, sym("_objective_list").as_bytes())?;
			let option_list = Self::get_symbol(&lib, sym("_option_list").as_bytes())?;
			let solver_create = Self::get_symbol(&lib, sym("_solver_create").as_bytes())?;
			let solver_free = Self::get_symbol(&lib, sym("_solver_free").as_bytes())?;
			let solver_option_get = Self::get_symbol(&lib, sym("_solver_option_get").as_bytes())?;
			let solver_option_set = Self::get_symbol(&lib, sym("_solver_option_set").as_bytes())?;
			let solver_read_error = Self::get_symbol(&lib, sym("_solver_read_error").as_bytes())?;
			let solver_run = Self::get_symbol(&lib, sym("_solver_run").as_bytes())?;
			let statistic_list = Self::get_symbol(&lib, sym("_statistic_list").as_bytes())?;
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
				statistic_list,
			}
		};
		Ok(Arc::new(lib))
	}

	/// Get the list of objective strategies supported by solvers from this
	/// library.
	pub fn objective_list(&self) -> FznsoObjectiveList<'_> {
		// SAFETY: The symbol is a valid function pointer for as long as self lives.
		unsafe { (*self.objective_list)() }
	}

	/// Get the list of options accepted by solvers from this library.
	pub fn option_list(&self) -> FznsoOptionList<'static> {
		// SAFETY: The symbol is a valid function pointer for as long as self lives.
		unsafe { (*self.option_list)() }
	}

	/// Call `fznso_solver_free` on the given raw solver pointer.
	pub(crate) fn solver_free(&self, solver: *mut FznsoSolver) {
		// SAFETY: solver was returned by fznso_solver_create from this library;
		// this is the one call that releases its ownership.
		unsafe {
			(*self.solver_free)(solver);
		}
	}

	/// Call `fznso_solver_option_get` and return the resulting value reference.
	pub(crate) fn solver_option_get(
		&self,
		solver: *const FznsoSolver,
		name: &str,
	) -> FznsoValueRef<'_> {
		let cname = ffi::CString::new(name).unwrap();
		// SAFETY: solver is a valid solver pointer; cname is a valid C string
		// that outlives the call.
		unsafe { (*self.solver_option_get)(FznsoPtr::new(solver), cname.as_ptr()) }
	}

	/// Call `fznso_solver_option_set`; on failure, retrieve the error via
	/// `fznso_solver_read_error` and return it as an `Err(String)`.
	pub(crate) fn solver_option_set(
		&self,
		solver: *mut FznsoSolver,
		name: &str,
		value: FznsoValueRef<'_>,
	) -> Result<(), String> {
		let cname = ffi::CString::new(name).unwrap();
		// SAFETY: solver is valid; cname and value outlive the call.
		let ok = unsafe { (*self.solver_option_set)(solver, cname.as_ptr(), value) };
		if !ok {
			let mut err = String::default();
			self.solver_read_error(solver, |msg| err.push_str(msg));
			Err(err)
		} else {
			Ok(())
		}
	}

	/// Call `fznso_solver_read_error`, forwarding each error chunk to
	/// `read_err`.
	fn solver_read_error<F: FnMut(&str)>(&self, solver: *mut FznsoSolver, mut read_err: F) {
		let ctx: *mut F = &mut read_err;
		// SAFETY: ctx points to a live F on the stack; wrap_read_error_cb casts
		// it back to &mut F inside the callback, which cannot outlive this call.
		unsafe {
			(*self.solver_read_error)(solver, ctx as *mut ffi::c_void, wrap_read_error_cb::<F>);
		}
	}

	/// Call `fznso_solver_run`, translating the result into [`Status`].
	///
	/// On [`FznsoStatus::FznsoError`] the error string is read via
	/// `fznso_solver_read_error` and stored in [`Status::Error`].
	pub(crate) fn solver_run<M, F, G>(
		&self,
		solver: *mut FznsoSolver,
		model: &M,
		on_solution: &mut F,
		on_message: &mut G,
	) -> Status
	where
		M: Model,
		F: for<'s> FnMut(&'s DynSolution<'s>),
		G: FnMut(&str, FznsoValueRef<'_>),
	{
		let model = crate::model::model_ref(model);
		let mut callbacks = RunCallbacks {
			on_solution,
			on_message,
		};
		let ctx: *mut RunCallbacks<'_, F, G> = &mut callbacks;
		// SAFETY: ctx points to live callbacks on the stack; the trampolines cast
		// it back in the callbacks, which are only called during this function.
		// model is a valid FznsoModelRef for the duration of the call.
		let status = unsafe {
			(*self.solver_run)(
				solver,
				model,
				ctx as *mut ffi::c_void,
				wrap_on_solution_cb::<F, G>,
				wrap_on_message_cb::<F, G>,
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

	/// Get the list of statistics produced by solvers from this library.
	pub fn statistic_list(&self) -> FznsoStatisticList<'_> {
		// SAFETY: The symbol is a valid function pointer for as long as self lives.
		unsafe { (*self.statistic_list)() }
	}
}
