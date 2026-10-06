//! Records a session with the `example_solver` through `librecord`, then runs
//! the same session against `libreplay` and expects the application to see
//! the same answers.
//!
//! Both libraries keep one log per process, so everything happens in one test.

#![allow(
	clippy::tests_outside_test_module,
	reason = "an integration test is a test crate by definition"
)]

use std::{
	env,
	path::{Path, PathBuf},
	process::Command,
	sync::{
		Arc,
		atomic::{AtomicBool, Ordering},
	},
};

use fznso::{
	DynSolution, LayeredModel, Library, OwnedValue, RangeList, Solution, Solver, Type, TypeBase,
};

/// Build the three libraries, which `cargo test` does not, and return the
/// paths of `librecord`, `libreplay` and `fznso`'s example solver.
fn build() -> [PathBuf; 3] {
	let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
	let mut cmd = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
	let _ = cmd
		.arg("build")
		.arg("--manifest-path")
		.arg(&manifest)
		.args(["-p", "fznso-record", "-p", "fznso-replay", "--lib"])
		.args(["-p", "fznso", "--example", "example_solver"]);
	if !cfg!(debug_assertions) {
		let _ = cmd.arg("--release");
	}
	assert!(cmd.status().expect("failed to run cargo").success());

	let mut dir = env::current_exe().expect("test binary path");
	let _ = dir.pop();
	if dir.ends_with("deps") {
		let _ = dir.pop();
	}
	let dylib = |name: &str| {
		format!(
			"{}{name}{}",
			env::consts::DLL_PREFIX,
			env::consts::DLL_SUFFIX
		)
	};
	[
		dir.join(dylib("record")),
		dir.join(dylib("replay")),
		dir.join("examples").join(dylib("example_solver")),
	]
}

/// An integer decision with domain `lb..=20`.
fn int_decision(m: &mut LayeredModel, name: &str, lb: i64) {
	let _ = m.add_decision(
		Type::new(TypeBase::FznsoTypeBaseInt).decision(true),
		OwnedValue::IntSet(RangeList::from(lb..=20)),
		Some(name.into()),
		false,
		true,
		vec![],
	);
}

/// Drive a session on a solver of `lib`, and return everything the
/// application sees of it.
fn session(lib: &Arc<Library>) -> Vec<String> {
	let mut seen = Vec::new();
	let statistics = lib.statistic_list();
	// SAFETY: the library returns a valid array of `len` statistics.
	let statistics = unsafe { std::slice::from_raw_parts(statistics.stats, statistics.len) };
	for s in statistics {
		// SAFETY: the library guarantees `ident` is valid UTF-8.
		let ident = unsafe { s.ident.as_str() }.unwrap_or_default();
		seen.push(format!("declares {ident} {} {}", s.solution, s.solver));
	}

	let mut solver = lib.create_solver();
	seen.push(format!(
		"{:?}",
		solver.option_set("solution_limit", (&3_i64).into())
	));
	seen.push(format!(
		"{:?}",
		solver.option_set("no_such_option", (&1_i64).into())
	));
	seen.push(format!("{}", solver.option_get("solution_limit").get_int()));

	// A run with messages, then a second layer and a run stopped early.
	let mut m = LayeredModel::default();
	int_decision(&mut m, "a", 3);
	let mut solutions = Vec::new();
	let mut messages = Vec::new();
	let status = solver.run(
		&m,
		&mut |sol: &DynSolution<'_>| {
			solutions.push(format!(
				"solution a={} #{}",
				sol.value(0).get_int(),
				sol.statistic("solutions").get_int()
			));
		},
		Some(&mut |scope: &str, value: fznso::Value<'_>| {
			messages.push(format!("message {scope}: {}", value.get_string()));
		}),
		fznso::dont_interrupt(),
	);
	assert_eq!(solutions.len(), 3);
	assert!(!messages.is_empty());
	seen.append(&mut solutions);
	seen.append(&mut messages);
	seen.push(format!("{status:?}"));

	m.set_unchanged(1);
	m.push_layer();
	int_decision(&mut m, "b", 10);
	let found = AtomicBool::new(false);
	let status = solver.run(
		&m,
		&mut |sol: &DynSolution<'_>| {
			solutions.push(format!(
				"solution a={} b={}",
				sol.value(0).get_int(),
				sol.value(1).get_int()
			));
			found.store(true, Ordering::Relaxed);
		},
		fznso::ignore_messages(),
		Some(&|| found.load(Ordering::Relaxed)),
	);
	assert_eq!(solutions.len(), 1);
	seen.append(&mut solutions);
	seen.push(format!("{status:?}"));
	seen.push(format!("{}", solver.statistic("runs").get_int()));
	seen
}

#[test]
#[cfg_attr(miri, ignore = "miri cannot dynamically load a library")]
fn replay_gives_the_recorded_answers() {
	let dir = env::temp_dir().join(format!("fznso-replay-solver-test-{}", std::process::id()));
	std::fs::create_dir_all(&dir).unwrap();
	let log = dir.join("session.msgpack");
	let [recorder, replay, example] = build();
	// SAFETY: this is the only test in this binary, so no other thread reads
	// the environment.
	unsafe {
		env::set_var("FZNSO_RECORD_SOLVER", example);
		env::set_var("FZNSO_RECORD_FILE", &log);
		env::set_var("FZNSO_REPLAY_FILE", &log);
	}

	// SAFETY: `librecord` is this workspace's own FZnSO solver.
	let recorded = session(&unsafe { Library::new(&recorder) }.expect("load librecord"));
	// SAFETY: as is `libreplay`. It reads the log on its first call, which
	// is after the recorder wrote the whole session.
	let played = session(&unsafe { Library::new(&replay) }.expect("load libreplay"));
	assert_eq!(recorded, played);

	std::fs::remove_dir_all(&dir).unwrap();
}
