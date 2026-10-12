//! Records a session with the `example_solver` through `librecord`, then
//! replays the log with `fznso-replay`.
//!
//! The recorder keeps one log per process, so everything happens in one test.

#![allow(
	clippy::tests_outside_test_module,
	reason = "an integration test is a test crate by definition"
)]

use std::{
	env,
	fs::File,
	io::BufReader,
	path::{Path, PathBuf},
	process::{Command, Output},
	sync::atomic::{AtomicBool, Ordering},
};

use fznso::{
	DynSolution, LayeredModel, Library, OwnedValue, RangeList, Solver, Status, Type, TypeBase,
};
use fznso_record_format::format::{self, Entry, Event, Header};

/// The directory cargo builds this test's profile into, such as
/// `target/debug`.
fn profile_dir() -> PathBuf {
	let mut dir = env::current_exe().expect("test binary path");
	let _ = dir.pop();
	if dir.ends_with("deps") {
		let _ = dir.pop();
	}
	dir
}

/// The file name of the shared library `name`.
fn dylib(name: &str) -> String {
	format!(
		"{}{name}{}",
		env::consts::DLL_PREFIX,
		env::consts::DLL_SUFFIX
	)
}

/// Build the `librecord` `cdylib` and `fznso`'s example solver, which
/// `cargo test` does not, and return their paths.
fn build() -> (PathBuf, PathBuf) {
	let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
	let mut cmd = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
	let _ = cmd
		.arg("build")
		.arg("--manifest-path")
		.arg(&manifest)
		.args([
			"-p",
			"fznso-record",
			"--lib",
			"-p",
			"fznso",
			"--example",
			"example_solver",
		]);
	if !cfg!(debug_assertions) {
		let _ = cmd.arg("--release");
	}
	assert!(cmd.status().expect("failed to run cargo").success());
	let dir = profile_dir();
	(
		dir.join(dylib("record")),
		dir.join("examples").join(dylib("example_solver")),
	)
}

/// Run `fznso-replay` on `log`, with `flags`.
fn replay(log: &Path, flags: &[&str]) -> Output {
	Command::new(env!("CARGO_BIN_EXE_fznso-replay"))
		.arg(log)
		.args(flags)
		.output()
		.expect("failed to run fznso-replay")
}

/// Read a log.
fn read_log(log: &Path) -> (Header, Vec<Entry>) {
	let mut input = BufReader::new(File::open(log).unwrap());
	let header = format::read_header(&mut input).unwrap();
	let entries = format::entries(input).map(Result::unwrap).collect();
	(header, entries)
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

#[test]
#[cfg_attr(miri, ignore = "miri cannot dynamically load a library")]
fn record_and_replay() {
	let dir = env::temp_dir().join(format!("fznso-replay-binary-test-{}", std::process::id()));
	std::fs::create_dir_all(&dir).unwrap();
	let log = dir.join("session.msgpack");
	let (recorder, example) = build();
	// SAFETY: this is the only test in this binary, so no other thread reads
	// the environment.
	unsafe {
		env::set_var("FZNSO_RECORD_SOLVER", example);
		env::set_var("FZNSO_RECORD_FILE", &log);
	}
	// SAFETY: `librecord` is this crate's own FZnSO solver.
	let lib = unsafe { Library::new(&recorder) }.expect("load librecord");

	// A session: options, a run, a new layer, and a run stopped early.
	let mut solver = lib.create_solver();
	solver
		.option_set("solution_limit", (&3_i64).into())
		.unwrap();
	assert!(
		solver
			.option_set("no_such_option", (&1_i64).into())
			.is_err()
	);
	assert_eq!(solver.option_get("solution_limit").get_int(), 3);

	let mut m = LayeredModel::default();
	int_decision(&mut m, "a", 3);
	let mut count = 0;
	let status = solver.run(
		&m,
		&mut |_: &DynSolution<'_>| count += 1,
		fznso::ignore_messages(),
		fznso::dont_interrupt(),
	);
	assert!(matches!(status, Status::Complete));
	assert_eq!(count, 3);

	m.set_unchanged(1);
	m.push_layer();
	int_decision(&mut m, "b", 10);
	let found = AtomicBool::new(false);
	let status = solver.run(
		&m,
		&mut |_: &DynSolution<'_>| found.store(true, Ordering::Relaxed),
		Some(&mut |_: &str, _: fznso::Value<'_>| {}),
		Some(&|| found.load(Ordering::Relaxed)),
	);
	assert!(matches!(status, Status::Incomplete));

	// A third layer on top, while layer 1 stays not permanent.
	m.set_unchanged(2);
	m.push_layer();
	int_decision(&mut m, "c", 5);
	let _ = solver.run(
		&m,
		&mut |_: &DynSolution<'_>| {},
		fznso::ignore_messages(),
		fznso::dont_interrupt(),
	);
	assert_eq!(solver.statistic("runs").get_int(), 3);
	drop(solver);

	// A second instance whose model breaks a rule: layer 0 changes although
	// the model says it did not.
	let mut solver = lib.create_solver();
	let mut m = LayeredModel::default();
	int_decision(&mut m, "x", 0);
	let _ = solver.run(
		&m,
		&mut |_: &DynSolution<'_>| {},
		fznso::ignore_messages(),
		fznso::dont_interrupt(),
	);
	m.set_unchanged(1);
	int_decision(&mut m, "y", 0);
	let _ = solver.run(
		&m,
		&mut |_: &DynSolution<'_>| {},
		fznso::ignore_messages(),
		fznso::dont_interrupt(),
	);
	drop(solver);

	let (header, entries) = read_log(&log);
	assert_eq!(header.magic, format::MAGIC);
	let runs: Vec<_> = entries
		.iter()
		.filter_map(|e| match &e.event {
			Event::Run { snapshot, .. } => Some(snapshot),
			_ => None,
		})
		.collect();
	// Only the new layer is written again, also above a layer that is not
	// permanent.
	assert_eq!((runs[1].from_layer, runs[1].layers.len()), (1, 1));
	assert_eq!(
		(runs[2].from_layer, runs[2].layers.len(), runs[2].permanent),
		(2, 1, 1)
	);
	assert!(entries.contains(&Entry {
		solver: 0,
		event: Event::StopAt { call: 1 },
	}));
	assert!(entries.iter().any(|e| e.solver == 1
		&& matches!(&e.event, Event::Violation { message } if message.contains("layer 0 differs"))));

	let out = replay(&log, &[]);
	assert!(
		out.status.success(),
		"replay failed: {}",
		String::from_utf8_lossy(&out.stderr)
	);

	let tampered = dir.join("tampered.msgpack");
	let mut file = File::create(&tampered).unwrap();
	format::write(&mut file, &header).unwrap();
	let mut changed = false;
	for mut entry in entries {
		if let Event::Solution { values, .. } = &mut entry.event
			&& !changed
		{
			values[0] = OwnedValue::Int(-1);
			changed = true;
		}
		format::write(&mut file, &entry).unwrap();
	}
	drop(file);
	// A log whose recorded solution differs gives a warning, and the replay
	// still goes to the end.
	let out = replay(&tampered, &[]);
	assert_eq!(out.status.code(), Some(2));
	let stderr = String::from_utf8_lossy(&out.stderr);
	assert!(stderr.contains("warning: entry"), "{stderr}");
	assert!(stderr.contains("solution 0 differs"), "{stderr}");
	let stdout = String::from_utf8_lossy(&out.stdout);
	assert!(stdout.contains("1 of"), "{stdout}");
	let all = stdout.into_owned();

	// With `--strict`, the replay stops at that solution's run.
	let out = replay(&tampered, &["--strict"]);
	assert_eq!(out.status.code(), Some(2));
	let stdout = String::from_utf8_lossy(&out.stdout);
	assert!(stdout.contains("1 of"), "{stdout}");
	assert_ne!(stdout, all, "the strict replay went to the end");

	std::fs::remove_dir_all(&dir).unwrap();
}
