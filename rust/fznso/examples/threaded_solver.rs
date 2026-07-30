//! A solver that does all of its work on a worker thread, so the threading
//! contract in [`fznso::Solver::run`] is exercised end to end.
//!
//! It is deliberately trivial *as a solver*: it assigns every decision the same
//! value and reports one solution. What matters is **where** the work happens.
//! The model is read, and both callbacks are invoked, from a spawned thread —
//! which is exactly what the contract permits and what a real parallel solver
//! does. That makes this the fixture for two properties nothing else covers:
//!
//! - the `M: Sync` / `F: Send` / `G: Send` bounds, and the `unsafe impl Sync`
//!   on `ModelRefAdapter`, are actually relied upon rather than merely
//!   declared;
//! - a binding that holds a process-wide lock across the run — the Python GIL,
//!   say — deadlocks against this, so loading it proves the lock is released.
//!
//! The contract still requires reporting to be *serial*, which it is here:
//! only one thread ever calls the callbacks, and the main thread just waits.

#![allow(missing_docs, reason = "the exported ABI entry points are generated")]

use fznso::{Model, Solution, Solver, SolverType, Status, Value};

/// The solution reported from the worker thread.
#[derive(Debug)]
pub struct ThreadedSolution {
	/// How many decisions the worker read from the model, reported as every
	/// decision's value so a consumer can observe that the cross-thread read
	/// reached the real model.
	decisions: i64,
}

impl Solution for ThreadedSolution {
	fn statistic(&self, name: &str) -> Value<'_> {
		match name {
			"decisions" => (&self.decisions).into(),
			_ => (&()).into(),
		}
	}

	fn value(&self, _decision_idx: usize) -> Value<'_> {
		(&self.decisions).into()
	}
}

/// A solver that reports from a thread other than the one `run` was called on.
#[derive(Debug, Default)]
pub struct ThreadedSolver;

impl Solver for ThreadedSolver {
	type Solution<'s> = ThreadedSolution;

	fn option_get(&self, _name: &str) -> Value<'_> {
		(&()).into()
	}

	fn option_set(&mut self, name: &str, _value: Value<'_>) -> Result<(), String> {
		Err(format!("unknown option `{name}`"))
	}

	fn run<M, F, G, H>(
		&mut self,
		model: &M,
		on_solution: &mut F,
		on_message: Option<&mut G>,
		should_stop: Option<&H>,
	) -> Status
	where
		M: Model + Sync,
		F: for<'s> FnMut(&'s Self::Solution<'s>) + Send,
		G: FnMut(&str, Value<'_>) + Send,
		H: Fn() -> bool + Send + Sync,
	{
		// A scoped thread, so the borrowed model and callbacks need no `'static`.
		// Reading the model here is what requires `M: Sync`; reporting from here
		// is what requires the callbacks to be `Send`.
		std::thread::scope(|scope| {
			let mut on_message = on_message;
			let stopped = || should_stop.is_some_and(|poll| poll());
			let worker = scope.spawn(move || {
				if stopped() {
					return Status::Incomplete;
				}
				let decisions = i64::try_from(model.decision_len()).unwrap_or(-1);
				// Only formatted when someone is listening.
				if let Some(emit) = &mut on_message {
					let note = format!("read {decisions} decision(s) from a worker thread");
					emit("log", (&note).into());
				}
				on_solution(&ThreadedSolution { decisions });
				if stopped() {
					return Status::Incomplete;
				}
				Status::Complete
			});
			worker
				.join()
				.unwrap_or_else(|_| Status::Error("the worker thread panicked".to_owned()))
		})
	}
}

impl SolverType for ThreadedSolver {
	/// This fixture declares no capabilities; only the constructor is required.
	fn new() -> Self {
		Self
	}
}

// The library name matches the built cdylib's base name
// (`libthreaded_solver.*`), so `Library::new` recovers it from the file name.
fznso::fznso_export!(ThreadedSolver, "threaded_solver");
