//! Loads a solver `cdylib` through [`fznso::Library`] and drives it.
//!
//! This is the only test that covers the dynamic-loading path: symbol
//! resolution in [`fznso::Library`], [`fznso::DynSolver`],
//! [`fznso::DynSolution`] and the `on_solution` trampoline. Everything else
//! calls the generated entry points directly (see `abi.rs`), which is what lets
//! miri check them — miri cannot `dlopen`, so this test is skipped under it.
//!
//! It loads `fznso-example-solver`, a throwaway fixture. Point it at a real
//! solver and delete that crate once one exists.

#![allow(
	clippy::tests_outside_test_module,
	reason = "an integration test is a test crate by definition"
)]

use std::{
	env,
	path::{Path, PathBuf},
	process::Command,
};

use fznso::{LayeredModel, Library, ModelValue, Solution, Solver, Status};
// Not every dependency of the crate under test is needed here.
use fznso_types as _;
use libloading as _;
use rangelist as _;
#[cfg(feature = "flatzinc-serde")]
use {flatzinc_serde as _, indexmap as _};

/// Build the example solver's `cdylib` and return the path to it.
///
/// `cargo test` builds test targets, not `cdylib` artifacts, so this library
/// would otherwise be missing on a clean tree and — worse — silently stale
/// whenever the fixture changes. Building it here keeps the test honest no
/// matter how it was invoked. The outer cargo has already finished building and
/// released its lock by the time test binaries run, so this nested invocation
/// does not deadlock.
fn example_solver_path() -> PathBuf {
	let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fznso-example-solver/Cargo.toml");
	let mut cmd = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
	let _ = cmd.arg("build").arg("--manifest-path").arg(&manifest);
	if !cfg!(debug_assertions) {
		let _ = cmd.arg("--release");
	}
	let status = cmd.status().expect("failed to run cargo");
	assert!(status.success(), "could not build fznso-example-solver");

	// The test binary lives in `<target>/<profile>/deps/`, so the cdylib sits one
	// directory up. Deriving it this way survives a custom CARGO_TARGET_DIR.
	let mut dir = env::current_exe().expect("test binary path");
	let _ = dir.pop();
	if dir.ends_with("deps") {
		let _ = dir.pop();
	}
	let path = dir.join(format!(
		"{}fznso_example_solver{}",
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
		ModelValue::IntSet(fznso::RangeList::from(3_i64..=20)),
		Some("a".into()),
		false,
		vec![],
	);
	let _b = m.add_decision(
		ModelValue::IntSet(fznso::RangeList::from(10_i64..=20)),
		Some("b".into()),
		false,
		vec![],
	);
	let _ = m.add_constraint(
		"int_le",
		vec![ModelValue::Int(0), ModelValue::Decision(a)],
		None,
		vec![],
	);
	m
}

#[test]
#[cfg_attr(miri, ignore = "miri cannot dlopen a shared library")]
fn load_and_run_a_dynamic_solver() {
	let path = example_solver_path();

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

	// An unknown option reports an error rather than succeeding silently.
	assert!(
		solver
			.option_set("no_such_option", (&1_i64).into())
			.is_err()
	);

	let m = model();
	let mut solutions: Vec<(i64, i64, i64)> = Vec::new();
	let status = solver.run(&m, &mut |sol: &fznso::DynSolution<'_>| {
		solutions.push((
			sol.value(0).get_int(),
			sol.value(1).get_int(),
			sol.statistic("solutions").get_int(),
		));
	});

	assert!(matches!(status, Status::Complete), "got {status:?}");
	// The fixture assigns each domain's lower bound, shifting the first variable
	// by the solution index.
	assert_eq!(solutions, vec![(3, 10, 0), (4, 10, 1), (5, 10, 2)]);
}

#[test]
#[cfg_attr(miri, ignore = "miri cannot dlopen a shared library")]
fn loading_a_non_solver_library_fails() {
	// SAFETY: the path does not name a solver library; `Library::new` is
	// expected to fail while resolving symbols rather than succeed.
	let result = unsafe { Library::new("definitely-not-a-solver-library") };
	assert!(result.is_err());
}
