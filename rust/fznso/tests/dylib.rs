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
};

use fznso::{LayeredModel, Library, LoadError, OwnedValue, Solution, Solver, Status};

/// Build the named `cdylib` example and return the path to it.
///
/// `cargo test` builds test targets, not example `cdylib`s, so the library
/// would otherwise be missing on a clean tree and — worse — silently stale
/// whenever the fixture changes. Building it here keeps the test honest no
/// matter how it was invoked. The outer cargo has already finished building and
/// released its lock by the time test binaries run, so this nested invocation
/// does not deadlock.
fn example_path(name: &str) -> PathBuf {
	let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
	let mut cmd = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
	let _ = cmd
		.arg("build")
		.arg("--manifest-path")
		.arg(&manifest)
		.arg("--example")
		.arg(name);
	if !cfg!(debug_assertions) {
		let _ = cmd.arg("--release");
	}
	let status = cmd.status().expect("failed to run cargo");
	assert!(status.success(), "could not build example `{name}`");

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
		OwnedValue::IntSet(fznso::RangeList::from(3_i64..=20)),
		Some("a".into()),
		false,
		vec![],
	);
	let _b = m.add_decision(
		OwnedValue::IntSet(fznso::RangeList::from(10_i64..=20)),
		Some("b".into()),
		false,
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
		&mut |scope: &str, value: fznso::Value<'_>| {
			messages.push((scope.to_owned(), value.get_string().to_owned()));
		},
	);

	assert!(matches!(status, Status::Complete), "got {status:?}");
	// The fixture assigns each domain's lower bound, shifting the first variable
	// by the solution index.
	assert_eq!(solutions, vec![(3, 10, 0), (4, 10, 1), (5, 10, 2)]);
	// The fixture warns that it ignores the model's single constraint.
	assert_eq!(
		messages,
		vec![("warn".to_owned(), "ignoring 1 constraint(s)".to_owned())]
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
