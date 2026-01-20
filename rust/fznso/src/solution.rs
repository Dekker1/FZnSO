use std::ffi::CString;

use fznso_types::FznsoSolutionRef;

use crate::Value;

/// Wrapper around [`FznsoSolutionRef`] to provide safe interactions with
/// solutions.
pub struct Solution<'a> {
	pub inner: FznsoSolutionRef<'a>,
}

impl<'a> Solution<'a> {
	pub fn value(&self, decision_idx: usize) -> Value<'a> {
		(self.inner.methods.value)(self.inner.data, decision_idx)
	}

	pub fn statistic(&self, name: &str) -> Value<'a> {
		let name = CString::new(name).unwrap();
		(self.inner.methods.statistic)(self.inner.data, name.as_ptr())
	}
}
