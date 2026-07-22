//! The [`Solver`] trait and [`DynSolver`] for dynamically-loaded solvers.

use std::sync::Arc;

use fznso_types::FznsoSolver;

use crate::{
	ConstraintList, DynSolution, Library, Model, ObjectiveList, OptionList, Solution,
	StatisticList, Status, TypeList, Value,
};

/// A solver instance loaded from a shared library via [`Library`].
///
/// Created by [`Library::create_solver`].
#[derive(Debug)]
pub struct DynSolver {
	/// Raw pointer to the opaque solver instance allocated by the library.
	pub(crate) solver: *mut FznsoSolver,
	/// Reference to the loaded library that owns all ABI symbols.
	pub(crate) lib: Arc<Library>,
}

/// The unified solver interface.
///
/// Implement this trait to write a Rust-native solver and then use
/// [`fznso_export`](crate::fznso_export) to expose it as a shared library
/// loadable by [`Library`].
///
/// See the [`fznso_export`](crate::fznso_export) documentation for a complete
/// worked example.
pub trait Solver {
	/// The concrete solution type emitted by this solver.
	///
	/// The lifetime parameter `'s` reflects that a solution may borrow from the
	/// solver or the current search state, lasting only for the duration of a
	/// single [`run`](Self::run) callback invocation.
	type Solution<'s>: Solution + 's;

	/// Get the current value of a named solver option.
	fn option_get(&self, name: &str) -> Value<'_>;

	/// Set the value of a named solver option.  Returns `Err` with a
	/// human-readable message if the option is unknown or the value is invalid.
	fn option_set(&mut self, name: &str, value: Value<'_>) -> Result<(), String>;

	/// Run the solver against the given model, invoking `on_solution` for each
	/// solution found.  Returns the completion [`Status`].
	fn run<M, F>(&mut self, model: &M, on_solution: &mut F) -> Status
	where
		M: Model,
		F: for<'s> FnMut(&'s Self::Solution<'s>);
}

/// What a solver implementation declares about itself, independently of any
/// particular instance.
///
/// [`Solver`] describes what an individual solver object can *do*; this trait
/// describes what the solver *type* supports, which is fixed at compile time.
/// The C entry points for these (`fznso_constraint_list` and friends) take no
/// solver instance, which is why they live here rather than on [`Solver`], and
/// why [`fznso_export`](crate::fznso_export) needs nothing but the type that
/// implements this trait — `fznso_export!(MySolver)`. See that macro's
/// documentation for a worked example.
///
/// A [`DynSolver`] deliberately does *not* implement this trait: a solver
/// loaded at runtime cannot answer these statically, so it exposes the same
/// information through inherent methods that ask its [`Library`] instead.
///
/// Every list defaults to empty, so an implementation need only declare the
/// capabilities it actually has.
pub trait SolverType: Solver + Sized {
	/// The constraints this solver accepts, exported as
	/// `fznso_constraint_list`.
	const CONSTRAINT_LIST: ConstraintList<'static> = ConstraintList::empty();
	/// The decision-variable types this solver accepts, exported as
	/// `fznso_decision_list`.
	const DECISION_LIST: TypeList<'static> = TypeList::empty();
	/// The objective strategies this solver supports, exported as
	/// `fznso_objective_list`.
	const OBJECTIVE_LIST: ObjectiveList<'static> = ObjectiveList::empty();
	/// The options this solver accepts, exported as `fznso_option_list`.
	const OPTION_LIST: OptionList<'static> = OptionList::empty();
	/// The statistics this solver reports, exported as `fznso_statistic_list`.
	const STATISTIC_LIST: StatisticList<'static> = StatisticList::empty();

	/// Create a new solver instance, exported as `fznso_solver_create`.
	fn new() -> Self;
}

impl DynSolver {
	/// The constraints the loaded solver accepts.
	pub fn constraint_types(&self) -> ConstraintList<'_> {
		self.lib.constraint_types()
	}

	/// The decision-variable types the loaded solver accepts.
	pub fn decision_types(&self) -> TypeList<'_> {
		self.lib.decision_types()
	}

	/// The objective strategies the loaded solver supports.
	pub fn objective_list(&self) -> ObjectiveList<'_> {
		self.lib.objective_list()
	}

	/// The options the loaded solver accepts.
	pub fn option_list(&self) -> OptionList<'_> {
		self.lib.option_list()
	}

	/// The statistics the loaded solver reports.
	pub fn statistic_list(&self) -> StatisticList<'_> {
		self.lib.statistic_list()
	}
}

impl Drop for DynSolver {
	fn drop(&mut self) {
		self.lib.solver_free(self.solver);
	}
}

impl Solver for DynSolver {
	type Solution<'s> = DynSolution<'s>;

	fn option_get(&self, name: &str) -> Value<'_> {
		self.lib.solver_option_get(self.solver, name)
	}

	fn option_set(&mut self, name: &str, value: Value<'_>) -> Result<(), String> {
		self.lib.solver_option_set(self.solver, name, value)
	}

	fn run<M, F>(&mut self, model: &M, on_solution: &mut F) -> Status
	where
		M: Model,
		F: for<'s> FnMut(&'s Self::Solution<'s>),
	{
		self.lib.solver_run(self.solver, model, on_solution)
	}
}
