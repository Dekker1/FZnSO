//! FZnSO Protocol Solver Template
//!
//! This crate defines the solver functionality to be implemented FZnSO
//! protocol, which allows the incremental usage of solvers that can solve
//! decision and optimization problems. The goal of the interface is to easily
//! use different solvers in a unified way, and to allow the dynamic loading of
//! solver libraries (as DLLs).
//!
//! The entry points below are named `fznso_NAME_...`. The `fznso_` marker is
//! fixed; `NAME` is a placeholder that a real solver replaces with its library
//! name — the base name of the built library file (e.g. `gecode` for
//! `libgecode.so`). Prefixing every symbol this way lets several solvers be
//! linked into one binary without colliding, and keeps them out of any wrapped
//! library's namespace.

#![allow(
	non_snake_case,
	reason = "the `fznso_NAME_` entry points are an illustrative placeholder prefix"
)]

use core::ffi;

use fznso_types::{
	FznsoConstraintList, FznsoModelRef, FznsoObjectiveList, FznsoOptionList, FznsoPtr,
	FznsoSolutionRef, FznsoSolver, FznsoStatisticList, FznsoStatus, FznsoStr, FznsoTypeList,
	FznsoValueRef,
};

#[unsafe(no_mangle)]
/// Report the FZnSO ABI version this solver was built against. A loader rejects
/// the solver if this does not match its own.
pub extern "C" fn fznso_NAME_abi_version() -> u32 {
	fznso_types::FZNSO_ABI_VERSION
}

#[unsafe(no_mangle)]
/// Returns the list of available constraint that can be added to the solver.
///
/// The returned array must stay valid for as long as the library is loaded, so
/// it is `static`. The caller never frees it.
pub extern "C" fn fznso_NAME_constraint_list() -> FznsoConstraintList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of types for which decision variable can be created by the
/// solver.
///
/// The returned array must stay valid for as long as the library is loaded, so
/// it is `static`. The caller never frees it.
pub extern "C" fn fznso_NAME_decision_list() -> FznsoTypeList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of available objective that can be achieved by the solver.
///
/// The returned array must stay valid for as long as the library is loaded, so
/// it is `static`. The caller never frees it.
pub extern "C" fn fznso_NAME_objective_list() -> FznsoObjectiveList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of available options that can be set of the solver.
///
/// The returned array must stay valid for as long as the library is loaded, so
/// it is `static`. The caller never frees it.
pub extern "C" fn fznso_NAME_option_list() -> FznsoOptionList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Create a new solver instance
pub extern "C" fn fznso_NAME_solver_create() -> *mut FznsoSolver {
	unimplemented!()
}

/// Free a solver instance, releasing all resources associated with it.
///
/// The pointer to the solver instance will be invalid after this function has
/// been called.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to this function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fznso_NAME_solver_free(solver: *mut FznsoSolver) {
	let _ = solver;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Get the current value of an option for the solver.
///
/// Note that this is only valid to be called with options named by
/// `fznso_NAME_option_list`.
///
/// The returned value must stay valid until the next call that can change the
/// solver (e.g. `_solver_option_set`, `_solver_run`, or `_solver_free`).
/// Returning a borrow of the solver's own state is the intent; the caller never
/// frees it.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to [`fznso_NAME_solver_free`].
pub unsafe extern "C" fn fznso_NAME_solver_option_get<'a>(
	solver: FznsoPtr<'a, FznsoSolver>,
	ident: FznsoStr<'_>,
) -> FznsoValueRef<'a> {
	let _ = solver;
	let _ = ident;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Set the current value of an option for the solver.
///
/// Note that this is only valid to be called with options named by
/// `fznso_NAME_option_list`, and the value must be of the correct type. This
/// function returns `true` if the option was successfully set, and `false`
/// otherwise.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to [`fznso_NAME_solver_free`].
pub unsafe extern "C" fn fznso_NAME_solver_option_set(
	solver: *mut FznsoSolver,
	ident: FznsoStr<'_>,
	value: FznsoValueRef<'_>,
) -> bool {
	let _ = solver;
	let _ = ident;
	let _ = value;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Read an error message from the solver.
///
/// This function is expected to be called after solver interactions signal an
/// error has occurred. For example, if [`fznso_NAME_solver_run`] returns
/// [`FznsoStatus::FznsoError`] or [`fznso_NAME_solver_option_set`] returns
/// `false`.
///
/// The message is handed to `read_error` rather than returned, so it need only
/// stay valid for the duration of that callback.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to [`fznso_NAME_solver_free`].
pub unsafe extern "C" fn fznso_NAME_solver_read_error(
	solver: *mut FznsoSolver,
	context: *mut ffi::c_void,
	read_error: extern "C" fn(context: *mut ffi::c_void, error: FznsoStr<'_>),
) {
	let _ = solver;
	let _ = context;
	let _ = read_error;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Run the solver with the given model
///
/// `on_message` reports non-fatal diagnostics: warnings, progress and search
/// information. `scope` names the kind of message and is dot-separated so that
/// callers can filter on a prefix. Failures are not reported here; they are
/// signalled by returning [`FznsoError`] and retrieved with
/// [`fznso_NAME_solver_read_error`].
///
/// `on_message` may be [`None`], meaning nobody is listening. A solver should
/// then skip *building* diagnostics rather than formatting them and handing
/// them to a sink that discards them — that is the point of making it nullable.
///
/// # What stays valid for how long
///
/// `model` is borrowed for the duration of this call and is read-only
/// throughout. Everything the solver hands *out* is borrowed the same way: the
/// solution passed to `on_solution`, and the scope and value passed to
/// `on_message`, need only stay valid for the duration of that one callback,
/// and a caller that wants to keep any of it must copy it.
///
/// # Stopping early
///
/// `should_stop` lets the caller abandon the search: the solver polls it, and a
/// `true` answer means stop as soon as convenient and return
/// [`FznsoIncomplete`]. It may be [`None`], which means the caller will never
/// ask to stop: a solver then skips polling entirely rather than calling a
/// predicate that always answers `false`.
///
/// A solver is expected to:
///
/// - poll it **once before starting**, so that a caller which has already asked
///   to stop gets no search at all;
/// - poll it **at least once after each `on_solution` returns**, so that a
///   caller can stop from inside the callback — this is what makes "stop after
///   this solution" reliable;
/// - otherwise poll it as often as it reasonably can, so that a cancel is acted
///   on promptly rather than at the end of the search.
///
/// To set a deadline, solvers should prefer the `time_limit` option over
/// polling a clock here, because a solver that knows its deadline up front can
/// plan its search around it.
///
/// # Threading
///
/// A solver may search on several threads. The contract for the callbacks and
/// the model is:
///
/// - `on_solution` and `on_message` must be invoked **serially**: never two at
///   once, and never one while the other runs. They may be called from any
///   thread (for example the worker that found a solution), as long as each
///   call happens-before the next. A caller therefore need not make its
///   callbacks safe to call concurrently. (Serial reporting also keeps
///   solutions in the improving order that intermediate/bound reporting relies
///   on.)
/// - `model` may be queried **concurrently**: it is a read-only view for the
///   duration of the call, so a parallel solver may read it from many threads
///   at once. A caller must therefore make the model safe for concurrent reads.
/// - `should_stop` may be called **concurrently**, from any thread, including
///   while `on_solution` or `on_message` is running, i.e. every worker may poll
///   it independently. It is the one callback a caller must make thread-safe,
///   which for the usual implementation (reading an atomic flag) it already is.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to [`fznso_NAME_solver_free`].
pub unsafe extern "C" fn fznso_NAME_solver_run(
	solver: *mut FznsoSolver,
	model: FznsoModelRef,
	context: *mut ffi::c_void,
	on_solution: extern "C" fn(context: *mut ffi::c_void, solution: FznsoSolutionRef),
	on_message: Option<
		extern "C" fn(context: *mut ffi::c_void, scope: FznsoStr, value: FznsoValueRef),
	>,
	should_stop: Option<extern "C" fn(context: *mut ffi::c_void) -> bool>,
) -> FznsoStatus {
	let _ = solver;
	let _ = model;
	let _ = context;
	let _ = on_solution;
	let _ = on_message;
	let _ = should_stop;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Get the current value of a statistic from the solver instance.
///
/// Note that this is only valid to be called with statistics named by
/// `fznso_NAME_statistic_list` whose `solver` flag is set; statistics carrying
/// only the `solution` flag are read from a solution instead, using
/// `FznsoSolutionMethods::statistic`. An unknown identifier yields a value of
/// kind `FznsoValueAbsent`.
///
/// As with `_solver_option_get`, the returned value must stay valid until
/// the next call that can change the solver.
///
/// # Safety
///
/// `solver` must be a pointer returned by [`fznso_NAME_solver_create`] that has
/// not yet been passed to [`fznso_NAME_solver_free`].
pub unsafe extern "C" fn fznso_NAME_solver_statistic<'a>(
	solver: FznsoPtr<'a, FznsoSolver>,
	ident: FznsoStr<'_>,
) -> FznsoValueRef<'a> {
	let _ = solver;
	let _ = ident;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of available statistical information that can be requested
/// from the solver and its solutions.
///
/// The returned array must stay valid for as long as the library is loaded, so
/// it is `static`. The caller never frees it.
pub extern "C" fn fznso_NAME_statistic_list() -> FznsoStatisticList<'static> {
	unimplemented!()
}
