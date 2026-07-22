//! Solution types and the [`Solution`] trait for Rust solver implementations.

use std::ffi;

use fznso_types::{FznsoPtr, FznsoSolution, FznsoSolutionMethods, FznsoSolutionRef};

use crate::Value;

/// A solution emitted by a dynamically-loaded solver
/// ([`DynSolver`](crate::DynSolver)).
///
/// Implements [`Solution`] and provides direct access to the inner
/// [`FznsoSolutionRef`].
#[derive(Debug)]
pub struct DynSolution<'a> {
	/// The inner [`FznsoSolutionRef`] handle.
	pub(crate) inner: FznsoSolutionRef<'a>,
}

/// Trait implemented by solution types produced by a [`Solver`](crate::Solver).
///
/// Implement this alongside [`Solver`](crate::Solver) to describe the values
/// and statistics emitted for each solution found.
pub trait Solution {
	/// Retrieve a named statistic.
	fn statistic(&self, name: &str) -> Value<'_>;

	/// Retrieve the value assigned to a decision variable by its index.
	fn value(&self, decision_idx: usize) -> Value<'_>;
}

/// Wraps a reference to a [`Solution`] implementation to produce a
/// [`FznsoSolutionRef`] suitable for passing across the C ABI boundary.
///
/// Wraps `&'a S` so it can be converted into [`FznsoSolutionRef`] via a static
/// vtable.  Used by [`solution_ref`].
pub(crate) struct SolutionWrapper<'a, S: Solution>(pub(crate) &'a S);

/// Convert a reference to a [`Solution`] implementation into the
/// [`FznsoSolutionRef`] handle expected by the C ABI.
///
/// Only reachable so that [`fznso_export`](crate::fznso_export) can name it in
/// the code it generates; there is no reason to call it directly.
#[doc(hidden)]
pub fn solution_ref<S: Solution + 'static>(sol: &S) -> FznsoSolutionRef<'_> {
	SolutionWrapper(sol).into_ref()
}

/// Vtable thunk: delegates to [`Solution::statistic`].
extern "C" fn wrap_sol_statistic<S: Solution + 'static>(
	data: FznsoPtr<'_, FznsoSolution>,
	ident: *const ffi::c_char,
) -> fznso_types::FznsoValueRef<'_> {
	// SAFETY: data was created from &S in SolutionWrapper::into_ref via transmute.
	let sol: &S = unsafe { &*data.cast::<S>() };
	// SAFETY: ident is a valid null-terminated C string provided by the caller.
	let name = unsafe { ffi::CStr::from_ptr(ident).to_str().unwrap() };
	sol.statistic(name)
}

/// Vtable thunk: delegates to [`Solution::value`].
extern "C" fn wrap_sol_value<S: Solution + 'static>(
	data: FznsoPtr<'_, FznsoSolution>,
	decision_index: usize,
) -> fznso_types::FznsoValueRef<'_> {
	// SAFETY: data was created from &S in SolutionWrapper::into_ref via transmute.
	let sol: &S = unsafe { &*data.cast::<S>() };
	sol.value(decision_index)
}

impl Solution for DynSolution<'_> {
	fn statistic(&self, name: &str) -> Value<'_> {
		let cname = ffi::CString::new(name).unwrap();
		(self.inner.methods.statistic)(self.inner.data, cname.as_ptr())
	}

	fn value(&self, decision_idx: usize) -> Value<'_> {
		(self.inner.methods.value)(self.inner.data, decision_idx)
	}
}

impl<'a, S: Solution + 'static> SolutionWrapper<'a, S> {
	/// Static vtable for `S`'s solution interface.
	const METHODS: &'static FznsoSolutionMethods = &FznsoSolutionMethods {
		value: wrap_sol_value::<S>,
		statistic: wrap_sol_statistic::<S>,
	};

	/// Convert this wrapper into a [`FznsoSolutionRef`].
	fn into_ref(self) -> FznsoSolutionRef<'a> {
		FznsoSolutionRef {
			data: FznsoPtr::from_ref(self.0),
			methods: Self::METHODS,
		}
	}
}
