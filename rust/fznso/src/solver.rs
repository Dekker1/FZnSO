use std::sync::Arc;

use fznso_types::FznsoSolver;

use crate::{Library, Model, Solution, Status, Value};

#[derive(Debug)]
/// Representation of a solver instance, created from a [`Library`].
pub struct Solver {
	pub(crate) solver: *mut FznsoSolver,
	pub(crate) lib: Arc<Library>,
}

impl Solver {
	/// Run the given model instance on the solver, calling the provided
	/// callback for each solution found, and returning the status of the run.
	pub fn run<M, F>(&mut self, model: &M, on_solution: F) -> Status
	where
		M: Model + 'static,
		F: FnMut(&Solution),
	{
		self.lib.solver_run(self.solver, model, on_solution)
	}

	/// Get the current value of an option by name.
	pub fn option_get(&self, name: &str) -> Value<'_> {
		self.lib.solver_option_get(self.solver, name)
	}

	/// Set the value of an option by name
	pub fn option_set<'a>(
		&mut self,
		name: &str,
		value: impl Into<Value<'a>>,
	) -> Result<(), String> {
		let value = value.into();
		self.lib.solver_option_set(self.solver, name, value)
	}
}

impl Drop for Solver {
	fn drop(&mut self) {
		self.lib.solver_free(self.solver);
	}
}
