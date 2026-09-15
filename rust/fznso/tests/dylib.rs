//! Loads a solver `cdylib` through [`fznso::Library`] and drives it.
//!
//! This is the only test that covers the dynamic-loading path: symbol
//! resolution in [`fznso::Library`], [`fznso::DynSolver`],
//! [`fznso::DynSolution`] and the `on_solution` trampoline. Everything else
//! calls the generated entry points directly (see `abi.rs`), which is what lets
//! miri check them — miri cannot `dlopen`, so this test is skipped under it.
//!
//! It loads the `example_solver` example, a throwaway fixture. Point it at a
//! real solver and replace that example once one exists.

#![allow(
	clippy::tests_outside_test_module,
	reason = "an integration test is a test crate by definition"
)]

use std::{
	env,
	path::{Path, PathBuf},
	process::Command,
	sync::Once,
};

use fznso::{
	LayeredModel, Library, LoadError, OwnedValue, Solution, Solver, Status, Type, TypeBase,
};

/// Build every `cdylib` example. See [`example_path`].
fn build_examples() {
	let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
	let mut cmd = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
	let _ = cmd
		.arg("build")
		.arg("--manifest-path")
		.arg(&manifest)
		.arg("--examples");
	if !cfg!(debug_assertions) {
		let _ = cmd.arg("--release");
	}
	let status = cmd.status().expect("failed to run cargo");
	assert!(status.success(), "could not build the examples");
}

/// Return the path to the named `cdylib` example, building the examples first.
///
/// `cargo test` builds test targets, not example `cdylib`s, so the library
/// would otherwise be missing on a clean tree and — worse — silently stale
/// whenever the fixture changes. Building it here keeps the test honest no
/// matter how it was invoked. The outer cargo has already finished building and
/// released its lock by the time test binaries run, so this nested invocation
/// does not deadlock.
///
/// Every example is built once, by whichever test gets here first. Building
/// per-test raced instead: cargo replaces the unsuffixed `libexample_solver.so`
/// non-atomically, so a concurrent test could look for it during the gap and
/// find nothing.
fn example_path(name: &str) -> PathBuf {
	static BUILD: Once = Once::new();
	BUILD.call_once(build_examples);

	// The test binary lives in `<target>/<profile>/deps/`, so example `cdylib`s
	// sit in the sibling `examples/` directory. Deriving it this way survives a
	// custom CARGO_TARGET_DIR.
	let mut dir = env::current_exe().expect("test binary path");
	let _ = dir.pop();
	if dir.ends_with("deps") {
		let _ = dir.pop();
	}
	let path = dir.join("examples").join(format!(
		"{}{name}{}",
		env::consts::DLL_PREFIX,
		env::consts::DLL_SUFFIX
	));
	assert!(
		path.exists(),
		"{} not found after building it",
		path.display()
	);
	path
}

/// A model with two integer decisions whose domains start at 3 and 10.
fn model() -> LayeredModel {
	let mut m = LayeredModel::default();
	let a = m.add_decision(
		Type::new(TypeBase::FznsoTypeBaseInt).decision(true),
		OwnedValue::IntSet(fznso::RangeList::from(3_i64..=20)),
		Some("a".into()),
		false,
		true,
		vec![],
	);
	let _b = m.add_decision(
		Type::new(TypeBase::FznsoTypeBaseInt).decision(true),
		OwnedValue::IntSet(fznso::RangeList::from(10_i64..=20)),
		Some("b".into()),
		false,
		true,
		vec![],
	);
	let _ = m.add_constraint(
		"int_le",
		vec![OwnedValue::Int(0), OwnedValue::Decision(a)],
		None,
		vec![],
	);
	m
}

#[test]
#[cfg_attr(miri, ignore = "miri cannot dynamically load a library")]
fn load_and_run_a_dynamic_solver() {
	let path = example_path("example_solver");

	// SAFETY: the library is this workspace's own example solver, so it exports
	// the full FZnSO ABI.
	let lib = unsafe { Library::new(&path) }.expect("load example solver");
	let mut solver = lib.create_solver();

	// Options round trip through the loaded library.
	assert_eq!(solver.option_get("solution_limit").get_int(), 1);
	solver
		.option_set("solution_limit", (&3_i64).into())
		.expect("set solution_limit");
	assert_eq!(solver.option_get("solution_limit").get_int(), 3);

	// An unknown option reports an error, and the message set by the solver's
	// `option_set` reaches the caller through `read_error`.
	let error = solver
		.option_set("no_such_option", (&1_i64).into())
		.expect_err("unknown option is rejected");
	assert_eq!(error, "unknown option `no_such_option`");

	// Identifiers cross as a pointer/length pair, so an interior nul is just an
	// ordinary byte: the name is carried through intact and simply matches no
	// declared option. There is no re-encoding step left to fail.
	assert_eq!(
		solver.option_get("bad\0name").kind(),
		fznso::ValueKind::FznsoValueAbsent
	);
	assert_eq!(
		solver
			.option_set("bad\0name", (&1_i64).into())
			.expect_err("unknown option is rejected"),
		"unknown option `bad\0name`"
	);

	// Solver-level statistics are readable from the solver instance itself, and
	// an unknown name is absent rather than an error.
	assert_eq!(solver.statistic("runs").get_int(), 0);
	assert_eq!(
		solver.statistic("no_such_statistic").kind(),
		fznso::ValueKind::FznsoValueAbsent
	);
	// `solutions` is declared solution-only, so it does not answer here.
	assert_eq!(
		solver.statistic("solutions").kind(),
		fznso::ValueKind::FznsoValueAbsent
	);

	// The declared metadata says where each statistic can be read from.
	let stats = solver.statistic_list();
	// SAFETY: the library returns a valid array of `len` statistics, valid for
	// as long as the library is loaded.
	let stats = unsafe { std::slice::from_raw_parts(stats.stats, stats.len) };
	let scopes: Vec<(&str, bool, bool)> = stats
		.iter()
		.map(|s| {
			// SAFETY: the library guarantees each `ident` is valid UTF-8 for as
			// long as it stays loaded.
			let ident = unsafe { s.ident.as_str() }.unwrap_or_default();
			(ident, s.solution, s.solver)
		})
		.collect();
	assert_eq!(
		scopes,
		vec![("solutions", true, false), ("runs", false, true)]
	);

	let m = model();
	let mut solutions: Vec<(i64, i64, i64)> = Vec::new();
	let mut messages: Vec<(String, String)> = Vec::new();
	let status = solver.run(
		&m,
		&mut |sol: &fznso::DynSolution<'_>| {
			solutions.push((
				sol.value(0).get_int(),
				sol.value(1).get_int(),
				sol.statistic("solutions").get_int(),
			));
		},
		Some(&mut |scope: &str, value: fznso::Value<'_>| {
			messages.push((scope.to_owned(), value.get_string().to_owned()));
		}),
		fznso::dont_interrupt(),
	);

	assert!(matches!(status, Status::Complete), "got {status:?}");
	// The solver-level statistic reflects state accumulated by the run.
	assert_eq!(solver.statistic("runs").get_int(), 1);
	// The fixture assigns each domain's lower bound, shifting the first
	// variable by the solution index.
	assert_eq!(solutions, vec![(3, 10, 0), (4, 10, 1), (5, 10, 2)]);
	// The fixture warns that it ignores the model's single constraint.
	assert_eq!(
		messages,
		vec![("warn".to_owned(), "ignoring 1 constraint(s)".to_owned())]
	);
}

#[test]
#[cfg_attr(miri, ignore = "miri cannot dynamically load a library")]
fn find_loads_a_solver_from_the_search_path() {
	// Stage the built example under a name matching the file it will be found
	// by, in a directory pointed at by FZNSO_SOLVER_PATH.
	let built = example_path("example_solver");
	let dir = env::temp_dir().join(format!("fznso-find-{}", std::process::id()));
	let _ = std::fs::remove_dir_all(&dir);
	std::fs::create_dir_all(&dir).unwrap();
	let staged = dir.join(built.file_name().unwrap());
	let _ = std::fs::copy(&built, &staged).unwrap();

	// SAFETY: this test is single-threaded, and the var is removed again below.
	unsafe { env::set_var("FZNSO_SOLVER_PATH", &dir) };

	// It is discovered by name without a path...
	let listed = Library::discover();
	assert!(
		listed.iter().any(|d| d.name == "example_solver"),
		"discover() did not list it: {listed:?}"
	);

	// ...and loading by name runs it just like loading by path.
	// SAFETY: the staged library is this workspace's own example solver.
	let lib = unsafe { Library::find("example_solver") }.expect("find example solver");
	let solver = lib.create_solver();
	assert_eq!(solver.option_get("solution_limit").get_int(), 1);

	// The staged example carries no version, so an empty request finds it and a
	// specific request does not.
	// SAFETY: the staged library is this workspace's own example solver.
	assert!(unsafe { Library::find_version("example_solver", "") }.is_ok());
	// SAFETY: as above; no such version is installed, so it must not load.
	let versioned = unsafe { Library::find_version("example_solver", "9") };
	assert!(
		matches!(versioned, Err(LoadError::NotFound { .. })),
		"got {versioned:?}"
	);

	// An unknown name reports where it looked.
	// SAFETY: no such solver exists; `find` must return NotFound.
	let missing = unsafe { Library::find("definitely_absent") };
	assert!(
		matches!(missing, Err(LoadError::NotFound { .. })),
		"got {missing:?}"
	);

	// SAFETY: single-threaded test; restore the environment.
	unsafe { env::remove_var("FZNSO_SOLVER_PATH") };
	let _ = std::fs::remove_dir_all(&dir);
}

/// A caller can abandon a run it has already started, by way of `should_stop`.
#[test]
#[cfg_attr(miri, ignore = "miri cannot dynamically load a library")]
fn a_run_can_be_stopped_early() {
	use std::sync::atomic::{AtomicBool, Ordering};

	let path = example_path("example_solver");
	// SAFETY: the library is this workspace's own example solver.
	let lib = unsafe { Library::new(&path) }.expect("load example solver");
	let mut solver = lib.create_solver();
	solver
		.option_set("solution_limit", (&5_i64).into())
		.expect("set solution_limit");
	let m = model();

	// "One solution is enough": tripped from inside the callback, which the
	// contract covers by requiring a poll after each `on_solution` returns.
	let stop = AtomicBool::new(false);
	let mut seen = 0_usize;
	let status = solver.run(
		&m,
		&mut |_sol: &fznso::DynSolution<'_>| {
			seen += 1;
			stop.store(true, Ordering::Relaxed);
		},
		fznso::ignore_messages(),
		Some(&|| stop.load(Ordering::Relaxed)),
	);
	assert_eq!(seen, 1, "the solver kept going after being asked to stop");
	assert!(
		matches!(status, Status::Incomplete),
		"a stopped run is incomplete, got {status:?}"
	);

	// Already asked to stop before the run begins: no search at all.
	let mut seen = 0_usize;
	let status = solver.run(
		&m,
		&mut |_sol: &fznso::DynSolution<'_>| seen += 1,
		fznso::ignore_messages(),
		Some(&|| true),
	);
	assert_eq!(seen, 0, "a pre-stopped run must not search");
	assert!(matches!(status, Status::Incomplete), "got {status:?}");
}

/// The predicate may be tripped from *another* thread, which is what lets a
/// caller cancel a long search that has not produced anything yet.
#[test]
#[cfg_attr(miri, ignore = "miri cannot dynamically load a library")]
fn a_watchdog_thread_can_stop_a_run() {
	use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

	let path = example_path("example_solver");
	// SAFETY: the library is this workspace's own example solver.
	let lib = unsafe { Library::new(&path) }.expect("load example solver");
	let mut solver = lib.create_solver();
	// Far more solutions than the watchdog will allow, so that finishing the
	// loop (rather than being stopped) fails the assertion instead of hanging.
	let limit = 100_000_i64;
	solver
		.option_set("solution_limit", (&limit).into())
		.expect("set solution_limit");
	let m = model();

	let stop = AtomicBool::new(false);
	let seen = AtomicUsize::new(0);
	let status = std::thread::scope(|scope| {
		let watchdog = scope.spawn(|| {
			while seen.load(Ordering::Relaxed) < 3 {
				std::thread::yield_now();
			}
			stop.store(true, Ordering::Relaxed);
		});
		let status = solver.run(
			&m,
			&mut |_sol: &fznso::DynSolution<'_>| {
				let _ = seen.fetch_add(1, Ordering::Relaxed);
			},
			fznso::ignore_messages(),
			Some(&|| stop.load(Ordering::Relaxed)),
		);
		watchdog.join().expect("watchdog thread panicked");
		status
	});

	assert!(matches!(status, Status::Incomplete), "got {status:?}");
	let count = seen.load(Ordering::Relaxed);
	assert!(
		(3..usize::try_from(limit).unwrap()).contains(&count),
		"expected the watchdog to stop the run early, but it emitted {count}"
	);
}

/// A solver may read the model and report from its own threads, which is what
/// the [`fznso::Solver::run`] contract permits.
///
/// This is the only test that drives a solver which actually does so, and so
/// the only one that relies on `M: Sync`, `F`/`G: Send` and the `unsafe impl
/// Sync` on the model adapter rather than merely declaring them.
#[test]
#[cfg_attr(miri, ignore = "miri cannot dynamically load a library")]
fn a_solver_may_report_from_a_worker_thread() {
	let path = example_path("threaded_solver");
	// SAFETY: the library is this workspace's own example solver, so it exports
	// the full FZnSO ABI.
	let lib = unsafe { Library::new(&path) }.expect("load threaded solver");
	let mut solver = lib.create_solver();

	let m = model();
	let mut values: Vec<i64> = Vec::new();
	let mut messages: Vec<(String, String)> = Vec::new();
	let status = solver.run(
		&m,
		&mut |sol: &fznso::DynSolution<'_>| values.push(sol.value(0).get_int()),
		Some(&mut |scope: &str, value: fznso::Value<'_>| {
			messages.push((scope.to_owned(), value.get_string().to_owned()));
		}),
		fznso::dont_interrupt(),
	);

	assert!(matches!(status, Status::Complete), "got {status:?}");
	// The worker thread read the model across the thread boundary: the fixture
	// reports the decision count it saw, and the model has two.
	assert_eq!(values, vec![2]);
	assert_eq!(
		messages,
		vec![(
			"log".to_owned(),
			"read 2 decision(s) from a worker thread".to_owned()
		)]
	);
}

#[test]
#[cfg_attr(miri, ignore = "miri cannot dynamically load a library")]
fn loading_a_non_solver_library_fails() {
	// SAFETY: the path does not name a solver library; `Library::new` is
	// expected to fail while resolving symbols rather than succeed.
	let result = unsafe { Library::new("definitely-not-a-solver-library") };
	assert!(result.is_err());
}

#[test]
#[cfg_attr(miri, ignore = "miri cannot dynamically load a library")]
fn loading_a_mismatched_abi_fails() {
	let path = example_path("bad_abi_solver");
	// SAFETY: the library exports a deliberately wrong FZnSO ABI version, which
	// `Library::new` must reject rather than proceed to use.
	let result = unsafe { Library::new(&path) };
	assert!(
		matches!(result, Err(LoadError::AbiMismatch { .. })),
		"expected an ABI mismatch, got {result:?}"
	);
}
