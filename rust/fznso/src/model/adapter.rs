//! [`ModelRefAdapter`]: wraps an ABI [`FznsoModelRef`] as a [`Model`] for use in Rust solvers.

use fznso_types::{FznsoModelRef, FznsoType};

use crate::{AnnotationRef, ConstraintIdx, DecisionIdx, Value, model::Model};

/// Wraps a [`FznsoModelRef`] to implement the [`Model`] trait, giving a
/// trait-based view over a model received through the C ABI.
///
/// Only reachable so that [`fznso_export`](crate::fznso_export) can name it in
/// the code it generates; there is no reason to use it directly.
#[doc(hidden)]
#[derive(Debug)]
pub struct ModelRefAdapter<'a>(FznsoModelRef<'a>);

// SAFETY: an adapter only ever wraps a `FznsoModelRef` received across the
// FZnSO interface, and the `Solver::run` threading contract requires such a
// model to be safe to query concurrently. `Sync` only permits sharing `&self`
// across threads, which is exactly a concurrent query; the wrapped data is
// never mutated through `&self`. The auto-impl is absent only because of the
// raw data pointer inside.
unsafe impl Sync for ModelRefAdapter<'_> {}

impl<'a> ModelRefAdapter<'a> {
	/// Return the inner [`FznsoModelRef`].
	pub fn into_inner(self) -> FznsoModelRef<'a> {
		self.0
	}

	/// Create a new adapter wrapping the given [`FznsoModelRef`].
	pub fn new(model: FznsoModelRef<'a>) -> Self {
		Self(model)
	}
}

impl<'a> Model for ModelRefAdapter<'a> {
	fn constraint_annotation(&self, con: ConstraintIdx, index: usize) -> AnnotationRef<'_> {
		(self.0.methods.constraint_annotation)(self.0.data, con, index)
	}

	fn constraint_annotation_len(&self, con: ConstraintIdx) -> usize {
		(self.0.methods.constraint_annotation_len)(self.0.data, con)
	}

	fn constraint_argument(&self, con: ConstraintIdx, index: usize) -> Value<'_> {
		(self.0.methods.constraint_argument)(self.0.data, con, index)
	}

	fn constraint_argument_len(&self, con: ConstraintIdx) -> usize {
		(self.0.methods.constraint_argument_len)(self.0.data, con)
	}

	fn constraint_defines(&self, con: ConstraintIdx) -> Option<DecisionIdx> {
		let val = (self.0.methods.constraint_defines)(self.0.data, con);
		match val.kind() {
			fznso_types::FznsoValueKind::FznsoValueDecision => Some(val.get_decision()),
			_ => None,
		}
	}

	fn constraint_ident(&self, con: ConstraintIdx) -> &str {
		// SAFETY: The string returned by the model vtable is valid UTF-8 with
		// lifetime at least as long as &self.
		unsafe { (self.0.methods.constraint_ident)(self.0.data, con).as_str() }.unwrap_or("")
	}

	fn constraint_layer_end(&self, layer: usize) -> usize {
		(self.0.methods.constraint_layer_end)(self.0.data, layer)
	}

	fn constraint_len(&self) -> usize {
		(self.0.methods.constraint_len)(self.0.data)
	}

	fn decision_annotation(&self, decision: DecisionIdx, index: usize) -> AnnotationRef<'_> {
		(self.0.methods.decision_annotation)(self.0.data, decision, index)
	}

	fn decision_annotation_len(&self, decision: DecisionIdx) -> usize {
		(self.0.methods.decision_annotation_len)(self.0.data, decision)
	}

	fn decision_defined(&self, decision: DecisionIdx) -> bool {
		(self.0.methods.decision_defined)(self.0.data, decision)
	}

	fn decision_domain(&self, decision: DecisionIdx) -> Value<'_> {
		(self.0.methods.decision_domain)(self.0.data, decision)
	}

	fn decision_layer_end(&self, layer: usize) -> usize {
		(self.0.methods.decision_layer_end)(self.0.data, layer)
	}

	fn decision_type(&self, decision: DecisionIdx) -> FznsoType {
		(self.0.methods.decision_type)(self.0.data, decision)
	}

	fn decision_len(&self) -> usize {
		(self.0.methods.decision_len)(self.0.data)
	}

	fn decision_name(&self, decision: DecisionIdx) -> Option<&str> {
		// SAFETY: The string returned by the model vtable is valid UTF-8 with
		// lifetime at least as long as &self.
		unsafe { (self.0.methods.decision_name)(self.0.data, decision).as_str() }
	}

	fn layer_len(&self) -> usize {
		(self.0.methods.layer_len)(self.0.data)
	}

	fn layer_permanent(&self) -> usize {
		(self.0.methods.layer_permanent)(self.0.data)
	}

	fn layer_redundant_index(&self, index: usize) -> usize {
		(self.0.methods.layer_redundant_index)(self.0.data, index)
	}

	fn layer_redundant_len(&self) -> usize {
		(self.0.methods.layer_redundant_len)(self.0.data)
	}

	fn layer_unchanged(&self) -> usize {
		(self.0.methods.layer_unchanged)(self.0.data)
	}

	fn objective_annotation(&self, index: usize) -> AnnotationRef<'_> {
		(self.0.methods.objective_annotation)(self.0.data, index)
	}

	fn objective_annotation_len(&self) -> usize {
		(self.0.methods.objective_annotation_len)(self.0.data)
	}

	fn objective_arg(&self) -> Value<'_> {
		(self.0.methods.objective_arg)(self.0.data)
	}

	fn objective_ident(&self) -> &str {
		// SAFETY: The string returned by the model vtable is valid UTF-8 with
		// lifetime at least as long as &self.
		unsafe { (self.0.methods.objective_ident)(self.0.data).as_str() }.unwrap_or("")
	}
}
