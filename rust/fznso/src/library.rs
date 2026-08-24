//! Dynamic loading of FZnSO solver libraries.
//!
//! Solvers are found by name rather than by path, from a fixed list of
//! directories. The search order, the per-platform locations, and the rules a
//! file name must satisfy for a name to be recovered from it are specified at
//! <https://fznso.minizinc.dev/spec/discovery/> and
//! <https://fznso.minizinc.dev/spec/naming-and-versioning/>.

use std::{ffi, mem, sync::Arc};

use fznso_types::{
	FZNSO_ABI_VERSION, FznsoConstraintList, FznsoModelRef, FznsoObjectiveList, FznsoOptionList,
	FznsoPtr, FznsoSolutionRef, FznsoSolver, FznsoStatisticList, FznsoStatus, FznsoStr,
	FznsoTypeList, FznsoValueRef,
};
use libloading::Symbol;

use crate::{DynSolution, DynSolver, Model, Status};

/// Why loading a solver library through [`Library::new`] failed.
#[derive(Debug)]
pub enum LoadError {
	/// The library could not be opened, or a required symbol was missing.
	Library(libloading::Error),
	/// The file name does not yield a usable solver name.
	InvalidName {
		/// The file name the loader was given.
		file_name: String,
		/// The name derived from it, which cannot spell the `fznso_<name>_`
		/// entry points.
		derived: String,
	},
	/// No solver of the requested name was found on the search path.
	NotFound {
		/// The solver name that was looked for.
		name: String,
		/// The directories that were searched.
		searched: Vec<std::path::PathBuf>,
	},
	/// The solver reports an ABI version this library cannot use.
	AbiMismatch {
		/// The version the solver reported.
		found: u32,
		/// The version this library requires
		/// ([`fznso_types::FZNSO_ABI_VERSION`]).
		expected: u32,
	},
}

impl std::fmt::Display for LoadError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			LoadError::Library(err) => write!(f, "{err}"),
			LoadError::NotFound { name, searched } => write!(
				f,
				"no solver named `{name}` found in {}",
				searched
					.iter()
					.map(|p| p.display().to_string())
					.collect::<Vec<_>>()
					.join(", ")
			),
			LoadError::InvalidName { file_name, derived } => write!(
				f,
				"cannot derive a solver name from file name `{file_name}`: `{derived}` is not a \
				 valid FZnSO solver name (expected `[A-Za-z_][A-Za-z0-9_]*`, so that it can spell \
				 the `fznso_<name>_` entry points)"
			),
			LoadError::AbiMismatch { found, expected } => write!(
				f,
				"solver ABI version {found} is incompatible with this library's ABI version \
				 {expected}"
			),
		}
	}
}

impl std::error::Error for LoadError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			LoadError::Library(err) => Some(err),
			LoadError::InvalidName { .. }
			| LoadError::NotFound { .. }
			| LoadError::AbiMismatch { .. } => None,
		}
	}
}

impl From<libloading::Error> for LoadError {
	fn from(err: libloading::Error) -> Self {
		LoadError::Library(err)
	}
}

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
		for<'a, 'i> unsafe extern "C" fn(
			FznsoPtr<'a, FznsoSolver>,
			FznsoStr<'i>,
		) -> FznsoValueRef<'a>,
	>,
	/// Symbol for `fznso_<name>_solver_option_set`.
	solver_option_set: Symbol<
		'static,
		unsafe extern "C" fn(*mut FznsoSolver, FznsoStr<'_>, FznsoValueRef<'_>) -> bool,
	>,
	/// Symbol for `fznso_<name>_solver_read_error`.
	solver_read_error: Symbol<
		'static,
		unsafe extern "C" fn(
			*mut FznsoSolver,
			*mut ffi::c_void,
			extern "C" fn(*mut ffi::c_void, FznsoStr<'_>),
		),
	>,
	/// Symbol for `fznso_<name>_solver_run`.
	solver_run: Symbol<'static, RunFn>,
	/// Symbol for `fznso_<name>_solver_statistic`.
	solver_statistic: Symbol<
		'static,
		for<'a, 'i> unsafe extern "C" fn(
			FznsoPtr<'a, FznsoSolver>,
			FznsoStr<'i>,
		) -> FznsoValueRef<'a>,
	>,
	/// Symbol for `fznso_<name>_statistic_list`.
	statistic_list: Symbol<'static, unsafe extern "C" fn() -> FznsoStatisticList<'static>>,
}

/// The ABI type of the `on_message` thunk, spelled out so that the
/// higher-ranked lifetimes match the loaded symbol exactly.
type MessageThunk = for<'a, 'b> extern "C" fn(*mut ffi::c_void, FznsoStr<'a>, FznsoValueRef<'b>);

/// The ABI type of the `should_stop` thunk.
type StopThunk = extern "C" fn(*mut ffi::c_void) -> bool;

/// The ABI type of the `on_solution` thunk.
type SolutionThunk = for<'a> extern "C" fn(*mut ffi::c_void, FznsoSolutionRef<'a>);

/// The ABI type of `fznso_solver_run` itself.
///
/// `on_message` and `should_stop` are nullable, which is what tells a solver
/// nobody is listening and nothing will cancel, so it can skip that work.
type RunFn = for<'m> unsafe extern "C" fn(
	*mut FznsoSolver,
	FznsoModelRef<'m>,
	*mut ffi::c_void,
	SolutionThunk,
	Option<MessageThunk>,
	Option<StopThunk>,
) -> FznsoStatus;

/// The Rust callbacks `fznso_solver_run` needs, behind the single `context`
/// pointer the ABI provides.
///
/// The fields are raw pointers rather than references on purpose. `should_stop`
/// may be polled *concurrently* with `on_solution`/`on_message`, so no `&mut`
/// may ever cover this struct — the trampolines share it (`&*ctx`) and only
/// then form an exclusive reference to the individual closure they invoke,
/// which the serial-callback contract makes sound.
struct RunCallbacks<F, G, H> {
	/// Invoked for each solution the solver reports.
	on_solution: *mut F,
	/// Invoked for each diagnostic the solver reports, if any listener wants
	/// them. `None` is passed through to the solver as a null thunk, so it can
	/// skip building diagnostics entirely.
	on_message: Option<*mut G>,
	/// Polled to ask whether the search should be abandoned, if the caller has
	/// anything to cancel.
	should_stop: Option<*const H>,
}

/// ABI callback passed to `fznso_solver_run`; forwards a diagnostic to the Rust
/// closure stored behind `ctx`.
extern "C" fn wrap_on_message_cb<F, G, H>(
	ctx: *mut ffi::c_void,
	scope: FznsoStr<'_>,
	value: FznsoValueRef<'_>,
) where
	F: for<'s> FnMut(&'s DynSolution<'s>) + Send,
	G: FnMut(&str, FznsoValueRef<'_>) + Send,
	H: Fn() -> bool + Send + Sync,
{
	let ctx = ctx as *const RunCallbacks<F, G, H>;
	// SAFETY: ctx points at the RunCallbacks built in solver_run, live for the
	// whole call. Shared, never exclusive: `should_stop` may be polled from
	// another thread at this very moment.
	let cbs = unsafe { &*ctx };
	// This thunk is only handed to the solver when a listener exists, so the
	// option is always `Some` here.
	let Some(on_message) = cbs.on_message else {
		return;
	};
	// SAFETY: `on_message` points at the caller's live sink, exclusive by the
	// serial-callback contract; the `Send` bound covers the calling thread.
	let on_message = unsafe { &mut *on_message };
	// SAFETY: the solver guarantees `scope` is valid UTF-8 for this call.
	let scope = unsafe { scope.as_str() }.unwrap_or_default();
	on_message(scope, value);
}

/// ABI callback passed to `fznso_solver_run`; answers whether the search should
/// be abandoned, by polling the Rust predicate stored behind `ctx`.
extern "C" fn wrap_should_stop_cb<F, G, H>(ctx: *mut ffi::c_void) -> bool
where
	F: for<'s> FnMut(&'s DynSolution<'s>) + Send,
	G: FnMut(&str, FznsoValueRef<'_>) + Send,
	H: Fn() -> bool + Send + Sync,
{
	let ctx = ctx as *const RunCallbacks<F, G, H>;
	// SAFETY: as in the other trampolines — shared only.
	let cbs = unsafe { &*ctx };
	// Only handed over when the caller has a predicate, so always `Some` here.
	let Some(should_stop) = cbs.should_stop else {
		return false;
	};
	// SAFETY: `should_stop` points at the caller's live predicate. It is only
	// ever borrowed shared, and the `Sync` bound makes concurrent polls sound.
	let should_stop = unsafe { &*should_stop };
	should_stop()
}

/// ABI callback passed to `fznso_solver_run`; wraps a [`FznsoSolutionRef`] in
/// a [`DynSolution`] and forwards it to the Rust closure stored behind `ctx`.
extern "C" fn wrap_on_solution_cb<F, G, H>(ctx: *mut ffi::c_void, sol: FznsoSolutionRef<'_>)
where
	F: for<'s> FnMut(&'s DynSolution<'s>) + Send,
	G: FnMut(&str, FznsoValueRef<'_>) + Send,
	H: Fn() -> bool + Send + Sync,
{
	let ctx = ctx as *const RunCallbacks<F, G, H>;
	// SAFETY: see `wrap_on_message_cb` — shared borrow of the struct only.
	let cbs = unsafe { &*ctx };
	// SAFETY: `on_solution` points at the caller's live closure, exclusive by the
	// serial-callback contract; `F: Send` covers the calling thread.
	let on_solution = unsafe { &mut *cbs.on_solution };
	let sol = DynSolution { inner: sol };
	on_solution(&sol);
}

/// ABI callback passed to `fznso_solver_read_error`; receives a C string and
/// forwards it as `&str` to the Rust closure stored behind `ctx`.
extern "C" fn wrap_read_error_cb<F: FnMut(&str)>(ctx: *mut ffi::c_void, msg: FznsoStr<'_>) {
	let ctx = ctx as *mut F;
	// SAFETY: ctx was created from &mut F in solver_read_error; the callback is
	// only invoked during that call, so F is still live.
	let read_err = unsafe { &mut *ctx };
	let bytes = if msg.ptr.is_null() {
		&[][..]
	} else {
		// SAFETY: the library guarantees `ptr` covers `len` bytes for this call.
		unsafe { std::slice::from_raw_parts(msg.ptr.cast::<u8>(), msg.len) }
	};
	// A solver may emit non-UTF-8 bytes; lossily convert rather than panic across
	// the FFI boundary (which would abort the whole process while reporting an
	// error).
	read_err(&String::from_utf8_lossy(bytes));
}

/// Derive a solver's library name from its file name, borrowing from it.
///
/// Strips a leading `lib` and everything from the first `.`, so
/// `libgecode.6.2.1.dylib`, `libgecode.so.6` and `gecode.dll` all yield
/// `gecode`. A non-UTF-8 file name yields `""` — it cannot name ASCII symbols,
/// so [`is_valid_name`] then rejects it.
///
/// See [`Library::new`] for the naming rules this assumes.
fn name_from_filename(filename: &ffi::OsStr) -> &str {
	let base = std::path::Path::new(filename)
		.file_name()
		.and_then(|n| n.to_str())
		.unwrap_or("");
	let name = base.strip_prefix("lib").unwrap_or(base);
	name.split('.').next().unwrap_or(name)
}

/// Whether `name` can spell the `fznso_<name>_...` entry points.
///
/// The name is pasted straight into a symbol, so it must be a C identifier:
/// ASCII letters, digits and underscores, not starting with a digit. This
/// rejects the cases that would otherwise fail later as a baffling
/// missing-symbol error — an empty name from a dotfile, a `-` from a versioned
/// Windows file name like `gecode-6.dll`, or non-ASCII.
fn is_valid_name(name: &str) -> bool {
	let mut chars = name.chars();
	chars
		.next()
		.is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
		&& chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
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
	/// # Naming rules
	///
	/// A solver's *name* is the identifier pasted into its entry points
	/// (`fznso_<name>_solver_run` and friends). It has to satisfy both C and
	/// the platform's library naming at once, so:
	///
	/// 1. **It must be a valid C identifier**: ASCII letters, digits and
	///    underscores, not starting with a digit (`[A-Za-z_][A-Za-z0-9_]*`).
	///    Lowercase is the convention. No `-`, no `.`, nothing non-ASCII —
	///    those cannot appear in a symbol name.
	/// 2. **It must not begin with `lib`.** On Unix the file is `lib<name>.so`,
	///    so a name that itself began with `lib` could not be told apart from
	///    the platform prefix on a platform that has none: `libssat` would be
	///    read back as `libssat` from `liblibssat.so` but as `ssat` from
	///    `libssat.dll`.
	/// 3. **The file's base name must be `<name>`**, optionally prefixed with
	///    `lib`, followed by any version and extension components. Everything
	///    from the first `.` is ignored, so `<name>` must not contain a `.`.
	///
	/// The name is recovered from the file name by stripping a leading `lib`
	/// and everything from the first `.`, so `libgecode.6.2.1.so`,
	/// `libgecode.6.2.1.dylib` and `gecode.6.2.1.dll` all load a solver whose
	/// entry points are prefixed `fznso_gecode_`. This is the only supported
	/// naming, so the file name and the exported symbols must agree. A file
	/// name that yields something unusable is rejected with
	/// [`LoadError::InvalidName`] rather than failing later as a missing
	/// symbol.
	///
	/// # Versioning
	///
	/// The version goes **between the name and the extension on every
	/// platform** — `libgecode.6.2.1.so`, `libgecode.6.2.1.dylib`,
	/// `gecode.6.2.1.dll` — rather than following each platform's native
	/// library convention, so that one rule covers all three and Windows is not
	/// a special case.
	///
	/// Nothing ever links against a solver: it is opened by path from a
	/// directory that is deliberately off the linker search path, so no soname
	/// is resolved and `ldconfig` never sees it. The ELF `libfoo.so.MAJOR`
	/// scheme therefore buys nothing here, and putting the version first is
	/// what Python does for extension modules
	/// (`foo.cpython-312-x86_64-linux-gnu.so`) for the same reason. The native
	/// form is still parsed, so a distribution-packaged `libgecode.so.6` also
	/// loads.
	///
	/// Several versions of one solver can sit side by side — `libgecode.6.so`
	/// and `libgecode.7.so` both name `gecode`. They export the same
	/// `fznso_gecode_*` symbols, but the library is opened with `RTLD_LOCAL`
	/// (and Windows resolves per module), so more than one can be loaded in a
	/// single process without colliding. Which one is used is the caller's
	/// choice: pass the exact path, or select by version with
	/// [`find_version`](Library::find_version).
	///
	/// The solver's `fznso_<name>_abi_version` is checked first: loading fails
	/// with [`LoadError::AbiMismatch`] if it does not equal
	/// [`fznso_types::FZNSO_ABI_VERSION`], guarding against a solver built
	/// against an incompatible version of the ABI.
	///
	/// # Safety
	///
	/// The library at `filename` must export valid implementations of all
	/// thirteen FZnSO ABI functions under the `fznso_<name>_` prefix
	/// (`abi_version`, `constraint_list`, `decision_list`, `objective_list`,
	/// `option_list`, `statistic_list`, `solver_create`, `solver_free`,
	/// `solver_option_get`, `solver_option_set`, `solver_read_error`,
	/// `solver_run`, `solver_statistic`). Behaviour is undefined if any symbol
	/// is missing or does not conform to the FZnSO ABI.
	pub unsafe fn new<S: AsRef<ffi::OsStr>>(filename: S) -> Result<Arc<Self>, LoadError> {
		let filename = filename.as_ref();
		let name = name_from_filename(filename);
		if !is_valid_name(name) {
			return Err(LoadError::InvalidName {
				file_name: filename.to_string_lossy().into_owned(),
				derived: name.to_owned(),
			});
		}
		// SAFETY: Caller guarantees the library is a valid FZnSO solver library.
		let lib = unsafe {
			let lib = libloading::Library::new(filename)?;
			let sym = |suffix: &str| format!("fznso_{name}{suffix}\0");

			// Reject an incompatible ABI before touching any other entry point.
			let abi_version: Symbol<'static, unsafe extern "C" fn() -> u32> =
				Self::get_symbol(&lib, sym("_abi_version").as_bytes())?;
			let found = (*abi_version)();
			if found != FZNSO_ABI_VERSION {
				return Err(LoadError::AbiMismatch {
					found,
					expected: FZNSO_ABI_VERSION,
				});
			}

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
			let solver_statistic = Self::get_symbol(&lib, sym("_solver_statistic").as_bytes())?;
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
				solver_statistic,
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
		// SAFETY: solver is a valid solver pointer; `name` outlives the call.
		unsafe { (*self.solver_option_get)(FznsoPtr::new(solver), FznsoStr::from(name)) }
	}

	/// Call `fznso_solver_option_set`; on failure, retrieve the error via
	/// `fznso_solver_read_error` and return it as an `Err(String)`.
	pub(crate) fn solver_option_set(
		&self,
		solver: *mut FznsoSolver,
		name: &str,
		value: FznsoValueRef<'_>,
	) -> Result<(), String> {
		// SAFETY: solver is valid; `name` and `value` outlive the call.
		let ok = unsafe { (*self.solver_option_set)(solver, FznsoStr::from(name), value) };
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
	pub(crate) fn solver_run<M, F, G, H>(
		&self,
		solver: *mut FznsoSolver,
		model: &M,
		on_solution: &mut F,
		on_message: Option<&mut G>,
		should_stop: Option<&H>,
	) -> Status
	where
		M: Model + Sync,
		F: for<'s> FnMut(&'s DynSolution<'s>) + Send,
		G: FnMut(&str, FznsoValueRef<'_>) + Send,
		H: Fn() -> bool + Send + Sync,
	{
		let model = crate::model::model_ref(model);
		// Raw pointers, and a *shared* context: `should_stop` may be polled while
		// a solution callback is running, so nothing may hold this exclusively.
		let callbacks = RunCallbacks {
			on_solution: std::ptr::from_mut(on_solution),
			on_message: on_message.map(std::ptr::from_mut),
			should_stop: should_stop.map(std::ptr::from_ref),
		};
		// A thunk is only handed over when there is something behind it, so a
		// solver sees a null pointer and can skip the work altogether.
		let message_thunk: Option<MessageThunk> = if callbacks.on_message.is_some() {
			Some(wrap_on_message_cb::<F, G, H>)
		} else {
			None
		};
		let stop_thunk: Option<StopThunk> = if callbacks.should_stop.is_some() {
			Some(wrap_should_stop_cb::<F, G, H>)
		} else {
			None
		};
		let ctx: *const RunCallbacks<F, G, H> = &callbacks;
		// SAFETY: ctx points to live callbacks on the stack; the trampolines cast
		// it back in the callbacks, which are only called during this function.
		// model is a valid FznsoModelRef for the duration of the call.
		let status = unsafe {
			(*self.solver_run)(
				solver,
				model,
				ctx.cast_mut().cast(),
				wrap_on_solution_cb::<F, G, H>,
				message_thunk,
				stop_thunk,
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

	/// Call `fznso_solver_statistic` and return the resulting value reference.
	pub(crate) fn solver_statistic(
		&self,
		solver: *const FznsoSolver,
		name: &str,
	) -> FznsoValueRef<'_> {
		// SAFETY: solver is a valid solver pointer; `name` outlives the call.
		unsafe { (*self.solver_statistic)(FznsoPtr::new(solver), FznsoStr::from(name)) }
	}

	/// Get the list of statistics produced by solvers from this library.
	pub fn statistic_list(&self) -> FznsoStatisticList<'_> {
		// SAFETY: The symbol is a valid function pointer for as long as self lives.
		unsafe { (*self.statistic_list)() }
	}
}

#[cfg(test)]
mod tests {
	use std::path::PathBuf;

	use super::{
		Discovered, discover_in, is_valid_name, name_from_filename, natural_cmp, search_paths_from,
		version_from_filename, version_matches,
	};

	/// The name a file yields, and whether it can spell the entry points.
	#[test]
	fn solver_names_are_derived_and_validated() {
		fn derive(f: &str) -> &str {
			name_from_filename(std::ffi::OsStr::new(f))
		}

		// The convention — version between the name and the extension — names
		// `gecode` on every platform, and so do the platforms' native forms,
		// which is what lets several versions sit side by side.
		for file in [
			"libgecode.so",
			"libgecode.6.2.1.so",
			"libgecode.dylib",
			"libgecode.6.2.1.dylib",
			"gecode.dll",
			"gecode.6.2.1.dll",
			"libgecode.so.6",
			"libgecode.so.6.2.1",
			"/usr/lib/fznso/libgecode.so.6",
		] {
			assert_eq!(derive(file), "gecode", "deriving from {file}");
			assert!(is_valid_name(derive(file)), "{file}");
		}

		// Rejected rather than left to fail as a missing symbol: a dotfile yields
		// nothing, and a versioned Windows name yields a `-`.
		assert_eq!(derive(".hidden.so"), "");
		assert!(!is_valid_name(derive(".hidden.so")));
		assert_eq!(derive("gecode-6.dll"), "gecode-6");
		assert!(!is_valid_name(derive("gecode-6.dll")));

		// A leading digit cannot start a C identifier.
		assert!(!is_valid_name("2fast"));
		assert!(is_valid_name("_private"));
		assert!(is_valid_name("or_tools2"));

		// Rule 2: a name starting with `lib` reads back differently per platform,
		// which is exactly why such names are forbidden.
		assert_eq!(derive("liblibssat.so"), "libssat");
		assert_eq!(derive("libssat.dll"), "ssat");
	}

	/// The search path is override, then user, then system, in that order.
	#[test]
	fn search_path_is_ordered() {
		let dirs = search_paths_from(
			Some(std::ffi::OsStr::new("/env/a")),
			Some(PathBuf::from("/user")),
			&["/usr/local/lib/fznso", "/usr/lib/fznso"],
		);
		assert_eq!(
			dirs,
			vec![
				PathBuf::from("/env/a"),
				PathBuf::from("/user"),
				PathBuf::from("/usr/local/lib/fznso"),
				PathBuf::from("/usr/lib/fznso"),
			]
		);

		// An absent override and user directory just leave the system ones.
		assert_eq!(
			search_paths_from(None, None, &["/usr/lib/fznso"]),
			vec![PathBuf::from("/usr/lib/fznso")]
		);
	}

	/// Versions sort naturally: `10` after `9`, and zero-padding does not
	/// matter.
	#[test]
	fn versions_sort_naturally() {
		let mut v = ["libx.so.9", "libx.so.10", "libx.so.2"];
		v.sort_by(|a, b| natural_cmp(a, b));
		assert_eq!(v, ["libx.so.2", "libx.so.9", "libx.so.10"]);
		assert_eq!(
			natural_cmp("libx.so.08", "libx.so.8"),
			std::cmp::Ordering::Equal
		);
	}

	/// A directory scan skips non-libraries and unusable names, and orders each
	/// solver's versions newest-first.
	#[test]
	#[cfg_attr(miri, ignore = "reads a real directory")]
	fn discovery_scans_a_directory() {
		let dir = std::env::temp_dir().join(format!("fznso-discover-{}", std::process::id()));
		let _ = std::fs::remove_dir_all(&dir);
		std::fs::create_dir_all(&dir).unwrap();
		let marker = super::LIB_MARKER;
		for file in [
			format!("libgecode{marker}.6"),
			format!("libgecode{marker}.10"),
			format!("libchuffed{marker}"),
			"README.txt".to_owned(),     // not a library
			format!("lib2fast{marker}"), // derives `2fast`, not a C identifier
		] {
			std::fs::write(dir.join(file), b"").unwrap();
		}

		let found = discover_in(&dir);
		let listed: Vec<(&str, &str)> = found
			.iter()
			.map(|d| (d.name.as_str(), d.version.as_str()))
			.collect();
		// chuffed and gecode only; the txt file and the digit-led name are skipped.
		// gecode appears twice, its highest version first, each with its version.
		assert_eq!(
			listed,
			vec![("chuffed", ""), ("gecode", "10"), ("gecode", "6")]
		);

		let _ = std::fs::remove_dir_all(&dir);
	}

	/// The version is parsed out of the file name across the platform schemes.
	#[test]
	fn versions_are_parsed_from_file_names() {
		// Only the marker for the current platform is recognised, so test that one.
		let marker = super::LIB_MARKER;
		// The convention: the version sits between the name and the extension on
		// every platform.
		assert_eq!(
			version_from_filename(&format!("libgecode.6.2.1{marker}")),
			"6.2.1"
		);
		assert_eq!(
			version_from_filename(&format!("gecode.6.2.1{marker}")),
			"6.2.1"
		);
		// A platform's native form is still parsed, so a distribution-packaged
		// library loads too.
		assert_eq!(
			version_from_filename(&format!("libgecode{marker}.6.2.1")),
			"6.2.1"
		);
		assert_eq!(version_from_filename(&format!("libgecode{marker}")), "");
		assert_eq!(version_from_filename(&format!("gecode{marker}")), "");
	}

	/// A version request matches by whole dotted components.
	#[test]
	fn version_requests_match_by_component() {
		assert!(version_matches("", "6.2.1")); // empty matches anything
		assert!(version_matches("6", "6.2.1"));
		assert!(version_matches("6.2", "6.2.1"));
		assert!(version_matches("6.2.1", "6.2.1"));
		assert!(!version_matches("6", "60")); // component, not prefix
		assert!(!version_matches("6.2", "6.1.0"));
		assert!(!version_matches("6.2.1.0", "6.2.1")); // request more specific
	}

	/// `Discovered` is public data; keep it constructible in the doctest sense.
	#[test]
	fn discovered_is_a_plain_record() {
		let d = Discovered {
			name: "gecode".to_owned(),
			version: "6.2.1".to_owned(),
			path: PathBuf::from("/usr/lib/fznso/libgecode.so.6.2.1"),
		};
		assert_eq!(d.name, "gecode");
		assert_eq!(d.version, "6.2.1");
	}
}

/// A solver library found on the search path, reported without loading it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Discovered {
	/// The solver's name, derived from the file name.
	pub name: String,
	/// The version, parsed from the file name, or empty if the file carries
	/// none. Compared component-wise, so `10` is newer than `9`.
	pub version: String,
	/// The full path to the library.
	pub path: std::path::PathBuf,
}

/// The file-name marker that identifies a shared library on this platform.
const LIB_MARKER: &str = if cfg!(target_os = "windows") {
	".dll"
} else if cfg!(target_os = "macos") {
	".dylib"
} else {
	".so"
};

/// The version a solver file name carries, or `""` if it carries none.
///
/// It is the file name with the leading `lib`, the solver name, the platform's
/// library marker and any surrounding dots removed — so `libgecode.so.6.2.1`
/// and `libgecode.6.2.1.dylib` both yield `6.2.1`, `gecode.dll` and
/// `libgecode.so` yield `""`.
fn version_from_filename(file_name: &str) -> String {
	let stripped = file_name.strip_prefix("lib").unwrap_or(file_name);
	let name = stripped.split('.').next().unwrap_or(stripped);
	let rest = &stripped[name.len()..];
	rest.replacen(LIB_MARKER, "", 1)
		.trim_matches('.')
		.to_owned()
}

/// Whether `requested` selects `actual`, matching whole dotted components so
/// that `6` selects `6.2.1` but not `60`, and `6.2` selects `6.2.1` but not
/// `6`. An empty request matches any version.
fn version_matches(requested: &str, actual: &str) -> bool {
	if requested.is_empty() {
		return true;
	}
	let mut req = requested.split('.');
	let mut act = actual.split('.');
	loop {
		match (req.next(), act.next()) {
			(None, _) => return true,
			(Some(_), None) => return false,
			(Some(r), Some(a)) if r == a => {}
			(Some(_), Some(_)) => return false,
		}
	}
}

/// Compare two file names so that embedded numbers order numerically, putting
/// `libgecode.so.10` after `libgecode.so.9` rather than before it.
fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
	/// Split off the leading run of digits, with insignificant zeros trimmed.
	fn take_digits<'a>(s: &mut &'a [u8]) -> &'a [u8] {
		let end = s
			.iter()
			.position(|c| !c.is_ascii_digit())
			.unwrap_or(s.len());
		let (digits, rest) = s.split_at(end);
		*s = rest;
		let trimmed = digits
			.iter()
			.position(|c| *c != b'0')
			.unwrap_or(digits.len());
		&digits[trimmed..]
	}

	let (mut x, mut y) = (a.as_bytes(), b.as_bytes());
	loop {
		match (x.first(), y.first()) {
			(None, None) => return std::cmp::Ordering::Equal,
			(None, Some(_)) => return std::cmp::Ordering::Less,
			(Some(_), None) => return std::cmp::Ordering::Greater,
			(Some(p), Some(q)) if p.is_ascii_digit() && q.is_ascii_digit() => {
				// A longer digit run is the larger number once zeros are trimmed.
				let (dx, dy) = (take_digits(&mut x), take_digits(&mut y));
				match dx.len().cmp(&dy.len()).then_with(|| dx.cmp(dy)) {
					std::cmp::Ordering::Equal => {}
					other => return other,
				}
			}
			(Some(p), Some(q)) => {
				match p.cmp(q) {
					std::cmp::Ordering::Equal => {}
					other => return other,
				}
				x = &x[1..];
				y = &y[1..];
			}
		}
	}
}

/// Build the search path from its inputs, so the ordering can be tested without
/// touching the environment.
fn search_paths_from(
	override_var: Option<&ffi::OsStr>,
	user_dir: Option<std::path::PathBuf>,
	system_dirs: &[&str],
) -> Vec<std::path::PathBuf> {
	let mut dirs: Vec<std::path::PathBuf> = Vec::new();
	if let Some(var) = override_var {
		dirs.extend(std::env::split_paths(var).filter(|p| !p.as_os_str().is_empty()));
	}
	dirs.extend(user_dir);
	dirs.extend(system_dirs.iter().map(std::path::PathBuf::from));
	dirs
}

/// The per-user solver directory, following each platform's convention.
fn user_solver_dir() -> Option<std::path::PathBuf> {
	let home = || std::env::var_os("HOME").map(std::path::PathBuf::from);
	if cfg!(target_os = "windows") {
		std::env::var_os("LOCALAPPDATA")
			.map(std::path::PathBuf::from)
			.map(|p| p.join("fznso"))
	} else if cfg!(target_os = "macos") {
		home().map(|p| p.join("Library/Application Support/fznso"))
	} else {
		std::env::var_os("XDG_DATA_HOME")
			.map(std::path::PathBuf::from)
			.or_else(|| home().map(|p| p.join(".local/share")))
			.map(|p| p.join("fznso"))
	}
}

/// List the solver libraries in one directory, best candidate first.
///
/// Files that are not shared libraries, or whose name cannot spell the entry
/// points, are skipped rather than reported as errors — a solver directory may
/// hold unrelated files. Several versions of one solver all appear, ordered so
/// that the highest version comes first.
fn discover_in(dir: &std::path::Path) -> Vec<Discovered> {
	let Ok(entries) = std::fs::read_dir(dir) else {
		return Vec::new();
	};
	let mut found: Vec<Discovered> = entries
		.flatten()
		.filter_map(|entry| {
			let path = entry.path();
			let file_name = path.file_name()?.to_str()?;
			if !file_name.contains(LIB_MARKER) {
				return None;
			}
			let name = name_from_filename(path.file_name()?);
			if !is_valid_name(name) {
				return None;
			}
			let version = version_from_filename(file_name);
			let name = name.to_owned();
			Some(Discovered {
				name,
				version,
				path,
			})
		})
		.collect();
	// Group by name, and within a name put the highest version first, so a bare
	// name resolves to the newest build in this directory.
	found.sort_by(|a, b| {
		natural_cmp(&a.name, &b.name).then_with(|| natural_cmp(&b.version, &a.version))
	});
	found
}

impl Library {
	/// The directories searched for solver libraries, in search order.
	///
	/// Solvers live in a directory of their own rather than on the normal
	/// library path: a FZnSO shim is usually named after the solver it wraps,
	/// so `libgecode.so` would otherwise collide with real Gecode, and a
	/// dedicated directory is what makes [`discover`](Library::discover)
	/// possible at all.
	///
	/// The order is
	/// 1. `$FZNSO_SOLVER_PATH`, split on the platform's path separator,
	/// 2. the per-user directory (`$XDG_DATA_HOME/fznso`,
	///    `~/Library/Application Support/fznso`, or `%LOCALAPPDATA%\fznso`),
	/// 3. the system directories (`/usr/local/lib/fznso`, `/usr/lib/fznso`, or
	///    `fznso` beside the running executable on Windows).
	pub fn search_paths() -> Vec<std::path::PathBuf> {
		let system: Vec<String> = if cfg!(target_os = "windows") {
			std::env::current_exe()
				.ok()
				.and_then(|exe| Some(exe.parent()?.join("fznso").to_str()?.to_owned()))
				.into_iter()
				.collect()
		} else {
			vec![
				"/usr/local/lib/fznso".to_owned(),
				"/usr/lib/fznso".to_owned(),
			]
		};
		let system: Vec<&str> = system.iter().map(String::as_str).collect();
		search_paths_from(
			std::env::var_os("FZNSO_SOLVER_PATH").as_deref(),
			user_solver_dir(),
			&system,
		)
	}

	/// List the solver libraries on the search path, without loading any.
	///
	/// Directories are visited in [`search_paths`](Library::search_paths) order
	/// and, within one directory, the highest version of a solver comes first.
	/// Nothing is opened, so the ABI version is *not* checked here — a
	/// directory may hold solvers built for another ABI, and
	/// [`find`](Library::find) skips those.
	pub fn discover() -> Vec<Discovered> {
		Self::search_paths()
			.iter()
			.flat_map(|dir| discover_in(dir))
			.collect()
	}

	/// Load a solver by name from the search path, choosing the newest version.
	///
	/// Candidates are tried in [`discover`](Library::discover) order, so the
	/// highest version in the earliest directory wins. Directory precedence
	/// comes first — a solver placed in `$FZNSO_SOLVER_PATH` or the per-user
	/// directory overrides a system one even if the system one is newer — and
	/// the newest version is chosen only among candidates of equal precedence.
	/// To pin a version regardless, use
	/// [`find_version`](Library::find_version).
	///
	/// A candidate built for a different ABI version is skipped and the search
	/// continues, since solvers for several ABI versions can share one
	/// directory.
	///
	/// # Safety
	///
	/// The libraries found on the search path must be valid FZnSO solvers; see
	/// [`Library::new`].
	pub unsafe fn find(name: &str) -> Result<Arc<Self>, LoadError> {
		// SAFETY: guaranteed by the caller.
		unsafe { Self::find_selected(name, None) }
	}

	/// Load a solver by name and version from the search path.
	///
	/// `version` selects by whole dotted components, so `"6"` matches `6.2.1`
	/// and `"6.2"` matches `6.2.1` but not `6.1.0`; an empty string matches any
	/// version, i.e. behaves like [`find`](Library::find). Among the matches
	/// the newest is chosen, in [`discover`](Library::discover) order.
	///
	/// # Safety
	///
	/// The libraries found on the search path must be valid FZnSO solvers; see
	/// [`Library::new`].
	pub unsafe fn find_version(name: &str, version: &str) -> Result<Arc<Self>, LoadError> {
		// SAFETY: guaranteed by the caller.
		unsafe { Self::find_selected(name, Some(version)) }
	}

	/// Shared implementation of [`find`](Library::find) and
	/// [`find_version`](Library::find_version).
	///
	/// # Safety
	///
	/// The libraries found on the search path must be valid FZnSO solvers.
	unsafe fn find_selected(name: &str, version: Option<&str>) -> Result<Arc<Self>, LoadError> {
		let mut last_mismatch = None;
		let matches = Self::discover()
			.into_iter()
			.filter(|d| d.name == name && version.is_none_or(|v| version_matches(v, &d.version)));
		for candidate in matches {
			// SAFETY: guaranteed by the caller.
			match unsafe { Self::new(&candidate.path) } {
				Ok(lib) => return Ok(lib),
				// Another ABI generation may sit in the same directory; keep looking.
				Err(err @ LoadError::AbiMismatch { .. }) => last_mismatch = Some(err),
				Err(err) => return Err(err),
			}
		}
		Err(last_mismatch.unwrap_or_else(|| LoadError::NotFound {
			name: name.to_owned(),
			searched: Self::search_paths(),
		}))
	}
}
