//! FZnSO Protocol Library
//!
//! This crate defines an interface for the FZnSO protocol, which allows the
//! incremental usage of solvers that can solve decision and optimization
//! problems. The goal of the interface is to easily use different solvers in a
//! unified way, and to allow the dynamic loading of solver libraries (as DLLs).

/// Generate all required C ABI functions for a [`Solver`] type.
///
/// Invoke this macro once in the root of a `cdylib` crate to expose the solver
/// as a dynamically loaded library loadable by [`Library`]:
///
/// ```rust
/// use fznso::{Model, Solution, Solver, SolverType, Status, Value};
///
/// struct MySolution;
/// impl Solution for MySolution {
///     fn value(&self, _: usize) -> Value<'_> { todo!() }
///     fn statistic(&self, _: &str) -> Value<'_> { todo!() }
/// }
///
/// struct MySolver;
///
/// // What an instance can do.
/// impl Solver for MySolver {
///     type Solution<'s> = MySolution where MySolver: 's;
///     fn option_get(&self, _: &str) -> Value<'_> { todo!() }
///     fn option_set(&mut self, _: &str, _: Value<'_>) -> Result<(), String> { todo!() }
///     fn run<M, F, G>(&mut self, _: &M, _: &mut F, _: &mut G) -> Status
///     where
///         M: Model,
///         F: for<'s> FnMut(&'s MySolution),
///         G: FnMut(&str, Value<'_>),
///     { todo!() }
/// }
///
/// // What the type declares. Every list defaults to empty, so a solver that
/// // supports nothing yet need only provide `new`.
/// impl SolverType for MySolver {
///     fn new() -> Self { MySolver }
/// }
///
/// fznso::fznso_export!(MySolver);
///
/// fn main() {}
/// ```
///
/// A failing `option_set` or a `Status::Error` from `run` stores its message on
/// the solver instance, and the generated `fznso_solver_read_error` returns it,
/// so the caller sees the reason for the failure.
#[macro_export]
macro_rules! fznso_export {
	($T:ty $(,)?) => {
		#[unsafe(no_mangle)]
		pub extern "C" fn fznso_constraint_list() -> $crate::ConstraintList<'static> {
			<$T as $crate::SolverType>::CONSTRAINT_LIST
		}

		#[unsafe(no_mangle)]
		pub extern "C" fn fznso_decision_list() -> $crate::TypeList<'static> {
			<$T as $crate::SolverType>::DECISION_LIST
		}

		#[unsafe(no_mangle)]
		pub extern "C" fn fznso_objective_list() -> $crate::ObjectiveList<'static> {
			<$T as $crate::SolverType>::OBJECTIVE_LIST
		}

		#[unsafe(no_mangle)]
		pub extern "C" fn fznso_option_list() -> $crate::OptionList<'static> {
			<$T as $crate::SolverType>::OPTION_LIST
		}

		#[unsafe(no_mangle)]
		pub extern "C" fn fznso_statistic_list() -> $crate::StatisticList<'static> {
			<$T as $crate::SolverType>::STATISTIC_LIST
		}

		#[unsafe(no_mangle)]
		pub extern "C" fn fznso_solver_create() -> *mut $crate::fznso_types::FznsoSolver {
			// The solver is boxed alongside a string that holds the most recent
			// error, so `fznso_solver_read_error` can return it.
			let solver = <$T as $crate::SolverType>::new();
			let state: ::std::boxed::Box<($T, ::std::string::String)> =
				::std::boxed::Box::new((solver, ::std::string::String::new()));
			::std::boxed::Box::into_raw(state) as *mut _
		}

		#[unsafe(no_mangle)]
		pub unsafe extern "C" fn fznso_solver_free(solver: *mut $crate::fznso_types::FznsoSolver) {
			// SAFETY: the handle was produced by fznso_solver_create.
			unsafe {
				::std::mem::drop(::std::boxed::Box::from_raw(
					solver as *mut ($T, ::std::string::String),
				))
			}
		}

		#[unsafe(no_mangle)]
		pub unsafe extern "C" fn fznso_solver_option_get(
			solver: $crate::fznso_types::FznsoPtr<'_, $crate::fznso_types::FznsoSolver>,
			ident: *const ::core::ffi::c_char,
		) -> $crate::Value<'_> {
			// SAFETY: the handle was produced by fznso_solver_create.
			let state: &($T, ::std::string::String) = unsafe { &*solver.cast() };
			// SAFETY: `ident` is a null-terminated C string supplied by the caller.
			let name = unsafe { ::core::ffi::CStr::from_ptr(ident).to_str().unwrap() };
			<$T as $crate::Solver>::option_get(&state.0, name)
		}

		#[unsafe(no_mangle)]
		pub unsafe extern "C" fn fznso_solver_option_set(
			solver: *mut $crate::fznso_types::FznsoSolver,
			ident: *const ::core::ffi::c_char,
			value: $crate::Value<'_>,
		) -> bool {
			// SAFETY: the handle was produced by fznso_solver_create.
			let state: &mut ($T, ::std::string::String) = unsafe { &mut *solver.cast() };
			// SAFETY: `ident` is a null-terminated C string supplied by the caller.
			let name = unsafe { ::core::ffi::CStr::from_ptr(ident).to_str().unwrap() };
			match <$T as $crate::Solver>::option_set(&mut state.0, name, value) {
				::std::result::Result::Ok(()) => true,
				::std::result::Result::Err(message) => {
					state.1 = message;
					false
				}
			}
		}

		#[unsafe(no_mangle)]
		pub unsafe extern "C" fn fznso_solver_read_error(
			solver: *mut $crate::fznso_types::FznsoSolver,
			context: *mut ::core::ffi::c_void,
			read_error: extern "C" fn(*mut ::core::ffi::c_void, *const ::core::ffi::c_char),
		) {
			// SAFETY: the handle was produced by fznso_solver_create.
			let state: &($T, ::std::string::String) = unsafe { &*solver.cast() };
			// Interior null bytes cannot be conveyed as a C string; drop them.
			let message = ::std::ffi::CString::new(state.1.replace('\0', "")).unwrap_or_default();
			read_error(context, message.as_ptr());
		}

		#[unsafe(no_mangle)]
		pub unsafe extern "C" fn fznso_solver_run(
			solver: *mut $crate::fznso_types::FznsoSolver,
			model: $crate::fznso_types::FznsoModelRef<'_>,
			context: *mut ::core::ffi::c_void,
			on_solution: extern "C" fn(
				*mut ::core::ffi::c_void,
				$crate::fznso_types::FznsoSolutionRef<'_>,
			),
			on_message: extern "C" fn(
				*mut ::core::ffi::c_void,
				$crate::fznso_types::FznsoStr<'_>,
				$crate::Value<'_>,
			),
		) -> $crate::fznso_types::FznsoStatus {
			// SAFETY: the handle was produced by fznso_solver_create.
			let state: &mut ($T, ::std::string::String) = unsafe { &mut *solver.cast() };
			let adapter = $crate::ModelRefAdapter::new(model);
			let status = <$T as $crate::Solver>::run(
				&mut state.0,
				&adapter,
				&mut |sol| on_solution(context, $crate::solution_ref(sol)),
				&mut |scope: &str, value| on_message(context, scope.into(), value),
			);
			match status {
				$crate::Status::Complete => $crate::fznso_types::FznsoStatus::FznsoComplete,
				$crate::Status::Incomplete => $crate::fznso_types::FznsoStatus::FznsoIncomplete,
				$crate::Status::Error(message) => {
					state.1 = message;
					$crate::fznso_types::FznsoStatus::FznsoError
				}
			}
		}
	};
}

mod library;
mod model;
mod solution;
mod solver;
mod value;

#[doc(hidden)]
pub use fznso_types;
use fznso_types::{
	FznsoAnnotationRef, FznsoConstraintIdx, FznsoConstraintList, FznsoConstraintType,
	FznsoDecisionIdx, FznsoObjective, FznsoObjectiveList, FznsoOption, FznsoOptionList,
	FznsoStatistic, FznsoStatisticList, FznsoType, FznsoTypeBase, FznsoTypeList, FznsoValueKind,
	FznsoValueRef,
};
pub use rangelist::RangeList;

pub use crate::{
	library::Library,
	model::{
		Annotation, ConstraintView, DecisionView, LayerView, Model, ObjectiveView, ann_ref,
		layered::{LayeredModel, OwnedAnnotation, OwnedValue},
	},
	solution::{DynSolution, Solution},
	solver::{DynSolver, Solver, SolverType},
	value::{FloatRanges, IntRanges, ValueExt, ValueList, ValueView},
};
#[doc(hidden)]
pub use crate::{model::adapter::ModelRefAdapter, solution::solution_ref};

/// A reference to an annotation; see [`fznso_types::FznsoAnnotationRef`].
pub type AnnotationRef<'a> = FznsoAnnotationRef<'a>;
/// Index of a constraint; see [`fznso_types::FznsoConstraintIdx`].
pub type ConstraintIdx = FznsoConstraintIdx;
/// A list of constraint types; see [`fznso_types::FznsoConstraintList`].
pub type ConstraintList<'a> = FznsoConstraintList<'a>;
/// A single constraint type descriptor; see
/// [`fznso_types::FznsoConstraintType`].
pub type ConstraintType<'a> = FznsoConstraintType<'a>;
/// Index of a decision variable; see [`fznso_types::FznsoDecisionIdx`].
pub type DecisionIdx = FznsoDecisionIdx;
/// A single objective descriptor; see [`fznso_types::FznsoObjective`].
pub type Objective<'a> = FznsoObjective<'a>;
/// A list of objectives; see [`fznso_types::FznsoObjectiveList`].
pub type ObjectiveList<'a> = FznsoObjectiveList<'a>;
/// A single option descriptor; see [`fznso_types::FznsoOption`].
///
/// Named `OptionDef` rather than `Option` so that it does not shadow
/// [`std::option::Option`].
pub type OptionDef<'a> = FznsoOption<'a>;
/// A list of options; see [`fznso_types::FznsoOptionList`].
pub type OptionList<'a> = FznsoOptionList<'a>;
/// A single statistic descriptor; see [`fznso_types::FznsoStatistic`].
pub type Statistic<'a> = FznsoStatistic<'a>;
/// A list of statistics; see [`fznso_types::FznsoStatisticList`].
pub type StatisticList<'a> = FznsoStatisticList<'a>;

/// Status enumeration that represents the outcome of [`Solver::run`].
///
/// This is an extended version of [`fznso_types::FznsoStatus`], which does not
/// include the actual error message when an error occurs.
#[derive(Debug)]
pub enum Status {
	/// The solver explored the full search space.
	Complete,
	/// The solver stopped early (timeout or other termination).
	Incomplete,
	/// An error occurred; the string contains the error message.
	Error(String),
}
/// A value type descriptor; see [`fznso_types::FznsoType`].
pub type Type = FznsoType;
/// The base of a value type descriptor; see [`fznso_types::FznsoTypeBase`].
pub type TypeBase = FznsoTypeBase;
/// A list of value type descriptors; see [`fznso_types::FznsoTypeList`].
pub type TypeList<'a> = FznsoTypeList<'a>;
/// A reference to a value; see [`fznso_types::FznsoValueRef`].
pub type Value<'a> = FznsoValueRef<'a>;
/// Which payload a [`Value`] holds; see [`fznso_types::FznsoValueKind`].
///
/// Prefer matching on [`ValueView`], which carries the payload with it.
pub type ValueKind = FznsoValueKind;

impl From<Status> for fznso_types::FznsoStatus {
	fn from(s: Status) -> Self {
		match s {
			Status::Complete => fznso_types::FznsoStatus::FznsoComplete,
			Status::Incomplete => fznso_types::FznsoStatus::FznsoIncomplete,
			Status::Error(_) => fznso_types::FznsoStatus::FznsoError,
		}
	}
}

#[cfg(test)]
#[allow(
	missing_docs,
	unreachable_pub,
	reason = "the ABI entry points these tests drive are macro-generated, and are \
	          only `pub` within this private module"
)]
mod tests {
	//! Drives a solver through the C ABI that [`fznso_export`] generates.
	//!
	//! The generated entry points are plain `extern "C"` functions, so they can
	//! be called directly. That covers the opaque solver handle round trip, the
	//! model vtable, [`ModelRefAdapter`], the solution wrapper and the
	//! `on_solution` trampoline — everything except `libloading`, which
	//! `tests/dylib.rs` covers instead.

	use std::ffi;

	use fznso_types::{
		FznsoConstraintIdx, FznsoDecisionIdx, FznsoPtr, FznsoSolutionRef, FznsoStatus,
	};

	use crate::{
		LayeredModel, Model, OwnedValue, Solution, Solver, SolverType, Status, Value,
		model::model_ref,
	};

	/// A solution that reports a fixed assignment and one statistic.
	struct TestSolution {
		/// The value assigned to every decision variable.
		assignment: i64,
		/// The value reported for the `"nodes"` statistic.
		nodes: i64,
	}

	impl Solution for TestSolution {
		fn statistic(&self, name: &str) -> Value<'_> {
			match name {
				"nodes" => (&self.nodes).into(),
				_ => (&()).into(),
			}
		}

		fn value(&self, _decision_idx: usize) -> Value<'_> {
			(&self.assignment).into()
		}
	}

	/// A solver that ignores the model and emits two fixed solutions.
	struct TestSolver {
		/// Current value of the `"seed"` option.
		seed: i64,
	}

	impl Solver for TestSolver {
		type Solution<'s> = TestSolution;

		fn option_get(&self, name: &str) -> Value<'_> {
			match name {
				"seed" => (&self.seed).into(),
				_ => (&()).into(),
			}
		}

		fn option_set(&mut self, name: &str, value: Value<'_>) -> Result<(), String> {
			match name {
				"seed" => {
					self.seed = value.get_int();
					Ok(())
				}
				_ => Err(format!("unknown option {name}")),
			}
		}

		fn run<M, F, G>(&mut self, model: &M, on_solution: &mut F, on_message: &mut G) -> Status
		where
			M: Model,
			F: for<'s> FnMut(&'s Self::Solution<'s>),
			G: FnMut(&str, Value<'_>),
		{
			on_message("warn", (&String::from("ignoring all constraints")).into());
			// Read back through the model vtable, so the model handle is exercised.
			assert_eq!(model.constraint_len(), 1);
			assert_eq!(model.constraint_ident(FznsoConstraintIdx(0)), "int_le");
			assert_eq!(model.decision_name(FznsoDecisionIdx(0)), Some("x"));
			assert_eq!(model.objective_ident(), "minimize");

			for i in 0..2 {
				on_solution(&TestSolution {
					assignment: self.seed + i,
					nodes: 42 + i,
				});
			}
			Status::Complete
		}
	}

	impl SolverType for TestSolver {
		fn new() -> Self {
			TestSolver { seed: 100 }
		}
	}

	crate::fznso_export!(TestSolver);

	/// What the two callbacks record. Both share the single `context` pointer
	/// the ABI provides, so they must agree on its type.
	#[derive(Default)]
	struct Collected {
		/// `(value, statistic)` per solution.
		solutions: Vec<(i64, i64)>,
		/// `(scope, payload)` per message.
		messages: Vec<(String, String)>,
	}

	/// Collects the scopes and payloads passed to the `on_message` callback.
	extern "C" fn collect_messages(
		ctx: *mut ffi::c_void,
		scope: fznso_types::FznsoStr<'_>,
		value: Value<'_>,
	) {
		// SAFETY: `ctx` is the `&mut Collected` handed to `fznso_solver_run`.
		let out = unsafe { &mut *(ctx as *mut Collected) };
		// SAFETY: the scope is valid UTF-8 for the duration of this call.
		let scope = unsafe { scope.as_str() }.unwrap_or_default();
		out.messages
			.push((scope.to_owned(), value.get_string().to_owned()));
	}

	/// Collects the values passed to the `on_solution` callback.
	extern "C" fn collect(ctx: *mut ffi::c_void, sol: FznsoSolutionRef<'_>) {
		// SAFETY: `ctx` is the `&mut Collected` handed to `fznso_solver_run`.
		let out = unsafe { &mut *(ctx as *mut Collected) };
		let value = (sol.methods.value)(sol.data, 0).get_int();
		let stat = {
			let name = ffi::CString::new("nodes").unwrap();
			(sol.methods.statistic)(sol.data, name.as_ptr()).get_int()
		};
		out.solutions.push((value, stat));
	}

	/// A one-decision, one-constraint model with an objective.
	fn model() -> LayeredModel {
		let mut m = LayeredModel::default();
		let x = m.add_decision(
			OwnedValue::IntSet(crate::RangeList::from(0_i64..=10)),
			Some("x".into()),
			false,
			vec![],
		);
		let _ = m.add_constraint(
			"int_le",
			vec![OwnedValue::Int(0), OwnedValue::Decision(x)],
			None,
			vec![],
		);
		m.set_objective(Some("minimize"), OwnedValue::Decision(x), vec![]);
		m
	}

	#[test]
	fn abi_round_trip_through_generated_entry_points() {
		let solver = fznso_solver_create();

		// Option round trip across the opaque solver handle.
		let name = ffi::CString::new("seed").unwrap();
		// SAFETY: `solver` came from `fznso_solver_create`.
		let seed = unsafe { fznso_solver_option_get(FznsoPtr::new(solver), name.as_ptr()) };
		assert_eq!(seed.get_int(), 100);
		// SAFETY: `solver` came from `fznso_solver_create`.
		assert!(unsafe { fznso_solver_option_set(solver, name.as_ptr(), (&7_i64).into()) });
		// SAFETY: `solver` came from `fznso_solver_create`.
		let seed = unsafe { fznso_solver_option_get(FznsoPtr::new(solver), name.as_ptr()) };
		assert_eq!(seed.get_int(), 7);

		// Unknown options are rejected rather than silently accepted.
		let bogus = ffi::CString::new("nope").unwrap();
		// SAFETY: `solver` came from `fznso_solver_create`.
		assert!(!unsafe { fznso_solver_option_set(solver, bogus.as_ptr(), (&1_i64).into()) });

		let m = model();
		let mut out = Collected::default();
		// SAFETY: `solver` came from `fznso_solver_create`, and `out` outlives the
		// call.
		let status = unsafe {
			fznso_solver_run(
				solver,
				model_ref(&m),
				(&raw mut out) as *mut ffi::c_void,
				collect,
				collect_messages,
			)
		};

		assert_eq!(status, FznsoStatus::FznsoComplete);
		assert_eq!(out.solutions, vec![(7, 42), (8, 43)]);
		assert_eq!(
			out.messages,
			vec![("warn".to_owned(), "ignoring all constraints".to_owned())]
		);

		// SAFETY: `solver` came from `fznso_solver_create` and is not used after.
		unsafe { fznso_solver_free(solver) };
	}
}
