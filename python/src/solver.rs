//! Running a solver: options, statistics, the search itself, and the solutions
//! it reports.

use fznso_core::{Solver as _, Status as CoreStatus, Value};
use pyo3::{
	exceptions::{PyRuntimeError, PyValueError},
	prelude::*,
};

use crate::{
	model::{LayeredModel, PyModelAdapter},
	value::{Decision, native_value_ref, value_to_py},
};

/// A solution reported to the `on_solution` callback.
///
/// Valid only during that callback; using it afterwards raises an error.
#[pyclass(unsendable, module = "fznso")]
pub(crate) struct Solution {
	// A raw pointer to the borrowed solution, valid only during the callback.
	solution: *const fznso_core::DynSolution<'static>,
	live: bool,
}

impl Solution {
	fn get(&self) -> PyResult<&fznso_core::DynSolution<'static>> {
		if !self.live {
			return Err(PyRuntimeError::new_err(
				"solution used outside the on_solution callback",
			));
		}
		// SAFETY: `live` is only true while the borrowed solution is alive.
		Ok(unsafe { &*self.solution })
	}
}

#[pymethods]
impl Solution {
	/// The value assigned to a decision variable (a `Decision` or an index).
	fn value(&self, py: Python<'_>, decision: &Bound<'_, PyAny>) -> PyResult<PyObject> {
		let index = match decision.extract::<Decision>() {
			Ok(d) => d.index,
			Err(_) => decision.extract::<usize>()?,
		};
		value_to_py(py, &fznso_core::Solution::value(self.get()?, index))
	}

	/// A named statistic for this solution.
	fn statistic(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
		value_to_py(py, &fznso_core::Solution::statistic(self.get()?, name))
	}

	fn __getitem__(&self, py: Python<'_>, decision: &Bound<'_, PyAny>) -> PyResult<PyObject> {
		self.value(py, decision)
	}
}

/// The outcome of a run.
#[pyclass(frozen, eq, module = "fznso")]
#[derive(Clone, PartialEq)]
pub(crate) enum Status {
	/// The solver explored the whole search space.
	Complete,
	/// The solver stopped early; better solutions may exist.
	Incomplete,
}

/// A solver instance created from a [`Library`].
#[pyclass(unsendable, module = "fznso")]
pub(crate) struct DynSolver {
	pub(crate) inner: fznso_core::DynSolver,
}

/// Carries the solver handle into [`Python::allow_threads`].
///
/// `allow_threads` requires its closure to be `Send`, which pyo3 uses as a
/// proxy for "holds no GIL-bound reference" — the closure itself runs on the
/// calling thread, so nothing is actually sent anywhere. A
/// [`fznso_core::DynSolver`] is `!Send` only because it holds the solver's raw
/// handle, which is not a GIL-bound reference, so carrying it in is sound.
struct SendSolver<'a>(&'a mut fznso_core::DynSolver);

// SAFETY: as above. The closure `allow_threads` runs stays on this thread, and
// the wrapped handle borrows nothing owned by the interpreter.
unsafe impl Send for SendSolver<'_> {}

impl<'a> SendSolver<'a> {
	/// Unwrap the handle inside the closure.
	///
	/// A method rather than a field access: `self.0` inside a closure would let
	/// edition-2021 disjoint capture grab the `!Send` reference directly and
	/// defeat the wrapper.
	fn into_inner(self) -> &'a mut fznso_core::DynSolver {
		self.0
	}
}

#[pymethods]
impl DynSolver {
	/// The current value of a named option.
	fn option_get(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
		value_to_py(py, &self.inner.option_get(name))
	}

	/// The current value of a named solver-level statistic, or `None` if the
	/// solver does not report it.
	fn statistic(&self, py: Python<'_>, name: &str) -> PyResult<PyObject> {
		value_to_py(py, &self.inner.statistic(name))
	}

	/// Set a named option. Raises `ValueError` with the solver's message on
	/// failure.
	fn option_set(&mut self, name: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
		// `value` lives for this call and the solver reads it synchronously, so
		// borrow it directly rather than copying it into an owned value.
		self.inner
			.option_set(name, native_value_ref(value.as_ptr()))
			.map_err(PyValueError::new_err)
	}

	/// Run the solver over `model`, calling `on_solution` for each solution and
	/// `on_message` for each diagnostic. Returns the completion status, or
	/// raises `RuntimeError` if the solver reports an error.
	///
	/// `model` may be a [`LayeredModel`] built with the builder, or any object
	/// implementing the [`Model`] protocol.
	///
	/// `should_stop`, if given, is polled by the solver and ends the search
	/// when it returns a truthy value; the run then reports
	/// `Status.Incomplete`. Use it to cancel from a watchdog thread. It may be
	/// called from several solver threads at once, so keep it cheap and
	/// thread-safe.
	#[pyo3(signature = (model, on_solution=None, on_message=None, should_stop=None))]
	fn run(
		&mut self,
		// The GIL is *released* for the duration of the search, so that a solver
		// which reports from its own threads can acquire it; everything that
		// re-enters Python takes it back with `Python::with_gil`.
		py: Python<'_>,
		model: &Bound<'_, PyAny>,
		on_solution: Option<PyObject>,
		on_message: Option<PyObject>,
		should_stop: Option<PyObject>,
	) -> PyResult<Status> {
		// A callback error is stashed and re-raised once `run` returns, since
		// the solver's callbacks cannot themselves fail. A `Mutex` rather
		// than a `RefCell`: the callbacks must be `Send`, because the solver
		// may invoke them from any thread (one at a time — see
		// `Solver::run`).
		let callback_error: std::sync::Mutex<Option<PyErr>> = std::sync::Mutex::new(None);
		let stash = |err: PyErr| {
			let mut slot = callback_error.lock().unwrap_or_else(|e| e.into_inner());
			if slot.is_none() {
				*slot = Some(err);
			}
		};
		let failed = || {
			callback_error
				.lock()
				.unwrap_or_else(|e| e.into_inner())
				.is_some()
		};

		let mut on_sol = |sol: &fznso_core::DynSolution<'_>| {
			if failed() {
				return;
			}
			let Some(handler) = &on_solution else {
				return;
			};
			// SAFETY: the wrapper is invalidated before this closure returns,
			// so its pointer never outlives `sol`.
			let widened: *const fznso_core::DynSolution<'static> =
				unsafe { std::mem::transmute(sol as *const fznso_core::DynSolution<'_>) };
			// The GIL is acquired here rather than captured: the callback may
			// run on a solver thread that does not already hold it.
			Python::with_gil(|py| {
				match Py::new(
					py,
					Solution {
						solution: widened,
						live: true,
					},
				) {
					Ok(wrapper) => {
						if let Err(err) = handler.call1(py, (&wrapper,)) {
							stash(err);
						}
						wrapper.borrow_mut(py).live = false;
					}
					Err(err) => stash(err),
				}
			});
		};

		// Built only when a handler was supplied, so a solver that asks "is
		// anyone listening?" gets a truthful no and can skip formatting
		// diagnostics.
		let mut on_msg = on_message.map(|handler| {
			move |scope: &str, value: Value<'_>| {
				if failed() {
					return;
				}
				Python::with_gil(|py| match value_to_py(py, &value) {
					Ok(obj) => {
						if let Err(err) = handler.call1(py, (scope, obj)) {
							stash(err);
						}
					}
					Err(err) => stash(err),
				});
			}
		});

		// Polled by the solver to ask whether to keep searching. Unlike the two
		// handlers this may be called from several solver threads at once, so
		// it is `Fn` and takes the GIL per poll. A callback that has already
		// raised also answers "stop", so a Python exception ends the search
		// promptly instead of being held until the solver finishes on its
		// own.
		let stop = || {
			if failed() {
				return true;
			}
			let Some(handler) = &should_stop else {
				return false;
			};
			Python::with_gil(|py| match handler.call0(py) {
				Ok(answer) => match answer.bind(py).is_truthy() {
					Ok(stop) => stop,
					Err(err) => {
						stash(err);
						true
					}
				},
				Err(err) => {
					stash(err);
					true
				}
			})
		};

		// The predicate is supplied whenever it could answer "stop": either the
		// caller gave one, or a callback exists that might raise and should cut
		// the search short. With neither, the solver is told nothing will
		// cancel.
		let wants_stop = should_stop.is_some() || on_solution.is_some() || on_msg.is_some();
		let message_sink = on_msg.as_mut();
		let stop_poll = if wants_stop { Some(&stop) } else { None };

		// A native `LayeredModel` runs directly; anything else is driven
		// through the adapter, which may itself record a Python error.
		//
		// Both run inside `allow_threads`: holding the GIL across the search
		// would deadlock any solver whose worker threads call back into
		// Python, since the callbacks and the adapter both wait for it.
		// Releasing it costs a real acquire per callback instead of a cheap
		// re-entrant one, which is the price of supporting a threaded solver
		// at all.
		let mut model_error: Option<PyErr> = None;
		let status = if let Ok(native) = model.downcast::<LayeredModel>() {
			let native = native.borrow();
			// The `PyRef` deliberately stays *outside* the closure: it holds
			// pyo3's borrow flag for the whole run, so Python code that
			// tries to mutate the model from a callback raises "already
			// borrowed" rather than aliasing it while the solver reads it.
			let model_ref: &fznso_core::LayeredModel = &native.inner;
			let solver = SendSolver(&mut self.inner);
			py.allow_threads(move || {
				solver
					.into_inner()
					.run(model_ref, &mut on_sol, message_sink, stop_poll)
			})
		} else {
			let adapter = PyModelAdapter::new(model.clone().unbind());
			let status = {
				let adapter_ref = &adapter;
				let solver = SendSolver(&mut self.inner);
				py.allow_threads(move || {
					solver
						.into_inner()
						.run(adapter_ref, &mut on_sol, message_sink, stop_poll)
				})
			};
			model_error = adapter
				.error
				.into_inner()
				.unwrap_or_else(|e| e.into_inner());
			status
		};

		if let Some(err) = callback_error
			.into_inner()
			.unwrap_or_else(|e| e.into_inner())
		{
			return Err(err);
		}
		if let Some(err) = model_error {
			return Err(err);
		}
		match status {
			CoreStatus::Complete => Ok(Status::Complete),
			CoreStatus::Incomplete => Ok(Status::Incomplete),
			CoreStatus::Error(message) => Err(PyRuntimeError::new_err(message)),
		}
	}
}
