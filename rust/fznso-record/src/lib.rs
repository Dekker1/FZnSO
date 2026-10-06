//! Record the calls into an FZnSO solver, to replay them later.
//!
//! Built as `librecord`, this crate is a solver named `record` that passes
//! every call on to another solver and writes it to a log. The `fznso-replay`
//! crate replays a log in either direction: its binary makes the same calls
//! on a solver, with nothing of the application needed, and its library
//! answers an application from the log, with no solver.
//!
//! The ABI has no way to configure a solver before it is created, so the
//! recorder reads two environment variables:
//!
//! - `FZNSO_RECORD_SOLVER` (required): the solver to record, a name to find on
//!   the search path or a path to its library.
//! - `FZNSO_RECORD_FILE`: the log to write. By default
//!   `fznso-record-<pid>.msgpack` in the current directory.
//!
//! The log format is in [`fznso_record_format::format`].

use std::{
	ffi::c_void,
	fs::File,
	io::{BufWriter, Write},
	panic::{AssertUnwindSafe, catch_unwind},
	sync::{
		Arc, Mutex, OnceLock,
		atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
	},
};

use fznso::{
	ConstraintList, DynSolution, DynSolver, Library, ModelRefAdapter, ObjectiveList, OptionList,
	OwnedValue, SendCtx, Solution, Solver, StatisticList, Status, TypeList, Value,
	fznso_types::{
		FZNSO_ABI_VERSION, FznsoModelRef, FznsoPtr, FznsoSolutionRef, FznsoSolver, FznsoStatus,
		FznsoStr,
	},
	panic_message,
};
use fznso_record_format::{
	capabilities::Capabilities,
	format::{self, Entry, Event, Header},
	load_solver,
	snapshot::{self, Seen},
};

/// The process-wide state of the recorder, set up by the first call that
/// needs it.
#[derive(Debug)]
struct Global {
	/// The recorded solver's library, loaded for the life of the process so
	/// the lists it returns stay valid.
	lib: Arc<Library>,
	/// The log.
	log: Mutex<BufWriter<File>>,
	/// The id of the next solver instance.
	next_id: AtomicU32,
	/// The statistics the recorded solver declares for solutions.
	solution_statistics: Vec<String>,
}

/// A solver instance of the recorder.
#[derive(Debug)]
struct Recorder {
	/// This instance's id in the log.
	id: u32,
	/// The recorded solver's instance.
	inner: DynSolver,
	/// The most recent error, for `solver_read_error`.
	error: String,
	/// The model of the previous run.
	seen: Option<Seen>,
}

/// The recorder's state, set up on first use.
///
/// Aborts if it cannot be set up: no entry point can report an error before a
/// solver exists.
fn global() -> &'static Global {
	/// The state, once set up.
	static GLOBAL: OnceLock<Global> = OnceLock::new();
	GLOBAL.get_or_init(|| {
		Global::open().unwrap_or_else(|message| {
			eprintln!("fznso-record: {message}");
			std::process::abort()
		})
	})
}

impl Global {
	/// Load the recorded solver and start the log.
	fn open() -> Result<Self, String> {
		let solver = std::env::var("FZNSO_RECORD_SOLVER")
			.map_err(|_| "set FZNSO_RECORD_SOLVER to the solver to record".to_owned())?;
		if solver == "record" {
			return Err("FZNSO_RECORD_SOLVER cannot be the recorder itself".to_owned());
		}
		// SAFETY: the user names an FZnSO solver to record.
		let lib =
			unsafe { load_solver(&solver) }.map_err(|e| format!("cannot load `{solver}`: {e}"))?;
		let path = std::env::var("FZNSO_RECORD_FILE")
			.unwrap_or_else(|_| format!("fznso-record-{}.msgpack", std::process::id()));
		let file = File::create(&path).map_err(|e| format!("cannot create `{path}`: {e}"))?;
		let mut out = BufWriter::new(file);
		let capabilities = Capabilities::of(&lib);
		let solution_statistics = capabilities
			.statistics
			.iter()
			.filter(|s| s.solution)
			.map(|s| s.ident.clone())
			.collect();
		let header = Header {
			magic: format::MAGIC.to_owned(),
			format: format::FORMAT,
			abi: FZNSO_ABI_VERSION,
			solver,
			capabilities,
		};
		format::write(&mut out, &header)
			.and_then(|()| out.flush())
			.map_err(|e| format!("cannot write `{path}`: {e}"))?;
		Ok(Self {
			lib,
			log: Mutex::new(out),
			next_id: AtomicU32::new(0),
			solution_statistics,
		})
	}

	/// Write one event, and flush it so that it survives a crash.
	fn log(&self, solver: u32, event: &Event) {
		let mut out = self.log.lock().unwrap_or_else(|e| e.into_inner());
		let result = format::write(&mut *out, &Entry { solver, event }).and_then(|()| out.flush());
		if let Err(e) = result {
			eprintln!("fznso-record: cannot write the log: {e}");
		}
	}
}

/// See `fznso_<name>_abi_version` in the specification.
#[unsafe(export_name = "fznso_record_abi_version")]
pub extern "C" fn abi_version() -> u32 {
	FZNSO_ABI_VERSION
}

/// The recorded solver's list.
#[unsafe(export_name = "fznso_record_constraint_list")]
pub extern "C" fn constraint_list() -> ConstraintList<'static> {
	global().lib.constraint_types()
}

/// The recorded solver's list.
#[unsafe(export_name = "fznso_record_decision_list")]
pub extern "C" fn decision_list() -> TypeList<'static> {
	global().lib.decision_types()
}

/// The recorded solver's list.
#[unsafe(export_name = "fznso_record_objective_list")]
pub extern "C" fn objective_list() -> ObjectiveList<'static> {
	global().lib.objective_list()
}

/// The recorded solver's list.
#[unsafe(export_name = "fznso_record_option_list")]
pub extern "C" fn option_list() -> OptionList<'static> {
	global().lib.option_list()
}

/// The recorded solver's list.
#[unsafe(export_name = "fznso_record_statistic_list")]
pub extern "C" fn statistic_list() -> StatisticList<'static> {
	global().lib.statistic_list()
}

/// Create an instance of the recorded solver, and log it.
#[unsafe(export_name = "fznso_record_solver_create")]
pub extern "C" fn solver_create() -> *mut FznsoSolver {
	let g = global();
	let id = g.next_id.fetch_add(1, Ordering::Relaxed);
	g.log(id, &Event::Create);
	let recorder = Recorder {
		id,
		inner: g.lib.create_solver(),
		error: String::new(),
		seen: None,
	};
	Box::into_raw(Box::new(recorder)).cast()
}

/// Free the instance, and log it.
///
/// # Safety
///
/// `solver` came from [`solver_create`] and is not used again.
#[unsafe(export_name = "fznso_record_solver_free")]
pub unsafe extern "C" fn solver_free(solver: *mut FznsoSolver) {
	// SAFETY: the handle came from `solver_create`.
	let recorder = unsafe { Box::from_raw(solver.cast::<Recorder>()) };
	global().log(recorder.id, &Event::Free);
}

/// Get an option from the instance, and log it.
///
/// # Safety
///
/// `solver` came from [`solver_create`], and `ident` is valid UTF-8.
#[unsafe(export_name = "fznso_record_solver_option_get")]
pub unsafe extern "C" fn solver_option_get<'a>(
	solver: FznsoPtr<'a, FznsoSolver>,
	ident: FznsoStr<'_>,
) -> Value<'a> {
	// SAFETY: forwarded to the caller.
	unsafe {
		logged_read(solver, ident, Solver::option_get, |ident, value| {
			Event::OptionGet { ident, value }
		})
	}
}

/// Get a statistic from the instance, and log it.
///
/// # Safety
///
/// `solver` came from [`solver_create`], and `ident` is valid UTF-8.
#[unsafe(export_name = "fznso_record_solver_statistic")]
pub unsafe extern "C" fn solver_statistic<'a>(
	solver: FznsoPtr<'a, FznsoSolver>,
	ident: FznsoStr<'_>,
) -> Value<'a> {
	// SAFETY: forwarded to the caller.
	unsafe {
		logged_read(solver, ident, Solver::statistic, |ident, value| {
			Event::Statistic { ident, value }
		})
	}
}

/// Read `ident` from the instance with `read`, and log the result as the
/// event `event` makes.
///
/// # Safety
///
/// `solver` came from [`solver_create`], and `ident` is valid UTF-8.
unsafe fn logged_read<'a>(
	solver: FznsoPtr<'a, FznsoSolver>,
	ident: FznsoStr<'_>,
	read: impl FnOnce(&'a DynSolver, &str) -> Value<'a>,
	event: impl FnOnce(String, OwnedValue) -> Event,
) -> Value<'a> {
	// SAFETY: the handle came from `solver_create`.
	let recorder: &'a Recorder = unsafe { &*solver.cast::<Recorder>() };
	// SAFETY: the caller guarantees `ident` is valid UTF-8.
	let name = unsafe { ident.as_str() }.unwrap_or_default();
	let value = read(&recorder.inner, name);
	global().log(recorder.id, &event(name.to_owned(), value.into()));
	value
}

/// Set an option on the instance, and log it.
///
/// # Safety
///
/// `solver` came from [`solver_create`], and `ident` is valid UTF-8.
#[unsafe(export_name = "fznso_record_solver_option_set")]
pub unsafe extern "C" fn solver_option_set(
	solver: *mut FznsoSolver,
	ident: FznsoStr<'_>,
	value: Value<'_>,
) -> bool {
	// SAFETY: the handle came from `solver_create`.
	let recorder = unsafe { &mut *solver.cast::<Recorder>() };
	// SAFETY: the caller guarantees `ident` is valid UTF-8.
	let name = unsafe { ident.as_str() }.unwrap_or_default();
	let result = catch_unwind(AssertUnwindSafe(|| recorder.inner.option_set(name, value)))
		.unwrap_or_else(|payload| Err(panic_message(payload)));
	global().log(
		recorder.id,
		&Event::OptionSet {
			ident: name.to_owned(),
			value: value.into(),
			error: result.clone().err(),
		},
	);
	match result {
		Ok(()) => true,
		Err(message) => {
			recorder.error = message;
			false
		}
	}
}

/// Report the instance's most recent error.
///
/// # Safety
///
/// `solver` came from [`solver_create`].
#[unsafe(export_name = "fznso_record_solver_read_error")]
pub unsafe extern "C" fn solver_read_error(
	solver: *mut FznsoSolver,
	context: *mut c_void,
	read_error: extern "C" fn(*mut c_void, FznsoStr<'_>),
) {
	// SAFETY: the handle came from `solver_create`.
	let recorder = unsafe { &*solver.cast::<Recorder>() };
	read_error(context, FznsoStr::from(recorder.error.as_str()));
}

/// Log the model, run the instance, and log what the run reports.
///
/// # Safety
///
/// As for `fznso_<name>_solver_run` in the specification.
#[unsafe(export_name = "fznso_record_solver_run")]
pub unsafe extern "C" fn solver_run(
	solver: *mut FznsoSolver,
	model: FznsoModelRef<'_>,
	context: *mut c_void,
	on_solution: extern "C" fn(*mut c_void, FznsoSolutionRef<'_>),
	on_message: Option<extern "C" fn(*mut c_void, FznsoStr<'_>, Value<'_>)>,
	should_stop: Option<extern "C" fn(*mut c_void) -> bool>,
) -> FznsoStatus {
	// SAFETY: the handle came from `solver_create`.
	let recorder = unsafe { &mut *solver.cast::<Recorder>() };
	let g = global();
	let id = recorder.id;
	let model = ModelRefAdapter::new(model);
	let taken = snapshot::take(&model, &mut recorder.seen);
	for message in taken.violations {
		eprintln!("fznso-record: solver {id}: {message}");
		g.log(id, &Event::Violation { message });
	}
	let mut run = Event::Run {
		snapshot: taken.snapshot,
		on_message: on_message.is_some(),
		should_stop: should_stop.is_some(),
	};
	g.log(id, &run);
	if let (Some(seen), Event::Run { snapshot, .. }) = (&mut recorder.seen, &mut run) {
		seen.restore(&mut snapshot.layers);
	}

	let context = SendCtx(context);
	let in_solution = &taken.in_solution;
	let mut solution = move |sol: &DynSolution<'_>| {
		let context = context;
		let values = in_solution.iter().map(|&d| sol.value(d).into()).collect();
		let statistics = g
			.solution_statistics
			.iter()
			.map(|name| (name.clone(), sol.statistic(name).into()))
			.collect();
		g.log(id, &Event::Solution { values, statistics });
		on_solution(context.0, sol.as_raw());
	};
	let mut message = on_message.map(|emit| {
		move |scope: &str, value: Value<'_>| {
			let context = context;
			g.log(
				id,
				&Event::Message {
					scope: scope.to_owned(),
					value: value.into(),
				},
			);
			emit(context.0, scope.into(), value);
		}
	});
	let calls = AtomicU64::new(0);
	let stopped = AtomicBool::new(false);
	let stop = should_stop.map(|poll| {
		let (calls, stopped) = (&calls, &stopped);
		move || {
			let context = context;
			let call = calls.fetch_add(1, Ordering::Relaxed);
			let stop = poll(context.0);
			if stop && !stopped.swap(true, Ordering::Relaxed) {
				g.log(id, &Event::StopAt { call });
			}
			stop
		}
	});
	let status = catch_unwind(AssertUnwindSafe(|| {
		recorder
			.inner
			.run(&model, &mut solution, message.as_mut(), stop.as_ref())
	}))
	.unwrap_or_else(|payload| Status::Error(panic_message(payload)));

	if let Status::Error(message) = &status {
		recorder.error.clone_from(message);
	}
	let result = status.clone().into();
	g.log(id, &Event::RunEnd { status });
	result
}
