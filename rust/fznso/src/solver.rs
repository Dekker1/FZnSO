//! The [`Solver`] trait and [`DynSolver`] for dynamically-loaded solvers.

use std::sync::Arc;

use fznso_types::FznsoSolver;

use crate::{
	ConstraintList, DynSolution, Library, Model, ObjectiveList, OptionList, Solution,
	StatisticList, Status, TypeList, Value,
};

/// A stand-in message-sink type, for callers that want no diagnostics.
///
/// Never called; it exists only to give [`ignore_messages`] a concrete type,
/// since a bare `None` leaves [`Solver::run`]'s sink parameter unconstrained.
pub type IgnoredMessages = fn(&str, Value<'_>);

/// A stand-in stop-predicate type, for callers that never interrupt a run. See
/// [`dont_interrupt`].
pub type NoInterrupt = fn() -> bool;

/// Pass as [`Solver::run`]'s `on_message` when the caller wants no diagnostics.
///
/// Nothing is merely discarded: the solver is told the sink is absent, so it
/// can skip *building* the messages in the first place.
#[must_use]
pub fn ignore_messages<'a>() -> Option<&'a mut IgnoredMessages> {
	None
}

/// Pass as [`Solver::run`]'s `should_stop` when the caller will never interrupt
/// the run, so a solver can drop the check from its search loop.
///
/// This says nothing about whether the *solver* stops — it will still finish,
/// or hit its `time_limit`. It only promises that no interruption arrives from
/// this side.
///
/// For a plain deadline prefer the `time_limit` option over a predicate that
/// watches a clock: a solver told its budget up front can plan around it.
#[must_use]
pub fn dont_interrupt<'a>() -> Option<&'a NoInterrupt> {
	None
}

/// A solver instance from a dynamically loaded library, via [`Library`].
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
/// [`fznso_export`](crate::fznso_export) to expose it as a dynamically loaded
/// library loadable by [`Library`].
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

	/// The current value of a named solver-level statistic.
	///
	/// Only statistics this solver declares with the `solver` flag set in
	/// [`SolverType::STATISTIC_LIST`] are readable here; statistics carrying
	/// only the `solution` flag are read from a [`Solution`] instead. An
	/// unknown name yields the absent value, which is what the default
	/// implementation returns — a solver with no solver-level statistics need
	/// not override it.
	fn statistic(&self, name: &str) -> Value<'_> {
		let _ = name;
		(&()).into()
	}

	/// Run the solver against the given model, invoking `on_solution` for each
	/// solution found.  Returns the completion [`Status`].
	///
	/// `on_message` receives non-fatal diagnostics the solver wants to report:
	/// warnings, progress, search information. `scope` names the kind of
	/// message and is dot-separated so that consumers can filter on a prefix
	/// (`warn`, `warn.domain`, `progress.objective`); see the README for the
	/// scopes solvers are encouraged to use. Failures are *not* reported here —
	/// those surface as [`Status::Error`].
	///
	/// # Threading
	///
	/// A solver may search on several threads, so the contract is defined in
	/// terms of them:
	///
	/// - **Callbacks are invoked serially.** The solver must never call
	///   `on_solution` and `on_message` concurrently — not with themselves, and
	///   not with each other. It *may* call them from any thread (for example
	///   the worker that found a solution), as long as it synchronises so that
	///   each call happens-before the next. That is why the callbacks need only
	///   be [`Send`], not `Sync`: an ordinary closure that is not safe to call
	///   concurrently is enough. For an optimising solver this falls out of the
	///   incumbent update it already serialises, and it keeps solutions in the
	///   improving order that `intermediate`/`progress.bound` rely on.
	/// - **The model may be queried concurrently.** The model is a read-only
	///   view for the duration of the run, so the solver may query it from many
	///   threads at once (the search hot path). That is why the model must be
	///   [`Sync`].
	///
	/// A consumer that wants to handle solutions in parallel does so *behind* a
	/// serial callback (push to a channel, fan out downstream); it never has to
	/// make the callback itself thread-safe.
	///
	/// `should_stop` is the exception: the solver may poll it from *several*
	/// threads at once, so it is `Fn + Sync` rather than `FnMut`. Reading an
	/// atomic — the usual implementation — already satisfies that.
	///
	/// # Optional callbacks
	///
	/// `on_message` and `should_stop` are both [`Option`]s, and `None` is a
	/// signal a solver should act on rather than paper over. Absent ones are
	/// spelled [`ignore_messages()`] and [`dont_interrupt()`], which supply the
	/// concrete type a bare `None` cannot infer:
	///
	/// - `on_message: None` means nobody is listening to diagnostics, so a
	///   solver should skip *building* them — not format a message and hand it
	///   to a sink that discards it.
	/// - `should_stop: None` means the caller will never ask to stop, so a
	///   solver need not poll at all.
	///
	/// A caller that wants only a deadline should set the `time_limit` option
	/// rather than polling a clock through `should_stop`: a solver told its
	/// deadline up front can plan its search around it.
	///
	/// `on_solution` has no `None` case — reporting solutions is what a run is
	/// for.
	///
	/// # Stopping early
	///
	/// `should_stop` is how a caller abandons a run it has already started, and
	/// the only way to stop one: return `true` and the solver gives up as soon
	/// as it can, yielding [`Status::Incomplete`]. It covers both "one
	/// solution is enough" — trip it from inside `on_solution` — and
	/// cancelling a long search that has produced nothing, by tripping it from
	/// another thread.
	///
	/// An implementation must poll it before starting and at least once after
	/// each `on_solution` returns, and should otherwise poll as often as is
	/// reasonable. The answer is monotone, so it may be cached once `true`.
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
		H: Fn() -> bool + Send + Sync;
}

/// What a solver implementation declares about itself, independently of any
/// particular instance.
///
/// [`Solver`] describes what an individual solver object can *do*; this trait
/// describes what the solver *type* supports, which is fixed at compile time.
/// The C entry points for these (`fznso_constraint_list` and friends) take no
/// solver instance, which is why they live here rather than on [`Solver`], and
/// why [`fznso_export`](crate::fznso_export) needs only the type that
/// implements this trait and the library's name — `fznso_export!(MySolver,
/// "mysolver")`. See that macro's documentation for a worked example.
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

	/// The constraint list to export. Override this instead of
	/// [`CONSTRAINT_LIST`](Self::CONSTRAINT_LIST) only if the list is not known
	/// until the library runs; it must then stay valid and unchanged for as
	/// long as the library is loaded. The same holds for the four lists below.
	fn constraint_list() -> ConstraintList<'static> {
		Self::CONSTRAINT_LIST
	}

	/// The decision-type list to export; see
	/// [`constraint_list`](Self::constraint_list).
	fn decision_list() -> TypeList<'static> {
		Self::DECISION_LIST
	}

	/// The objective list to export; see
	/// [`constraint_list`](Self::constraint_list).
	fn objective_list() -> ObjectiveList<'static> {
		Self::OBJECTIVE_LIST
	}

	/// The option list to export; see
	/// [`constraint_list`](Self::constraint_list).
	fn option_list() -> OptionList<'static> {
		Self::OPTION_LIST
	}

	/// The statistic list to export; see
	/// [`constraint_list`](Self::constraint_list).
	fn statistic_list() -> StatisticList<'static> {
		Self::STATISTIC_LIST
	}
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

	fn statistic(&self, name: &str) -> Value<'_> {
		self.lib.solver_statistic(self.solver, name)
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
		self.lib
			.solver_run(self.solver, model, on_solution, on_message, should_stop)
	}
}
