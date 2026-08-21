//! The [`Model`] trait, [`Annotation`] helper trait, and internal vtable
//! wrappers.

/// [`ModelRefAdapter`]: wraps an ABI [`FznsoModelRef`] as a [`Model`].
pub(crate) mod adapter;
/// [`Model`] implementation for `flatzinc_serde::FlatZinc`.
#[cfg(feature = "flatzinc-serde")]
mod flatzinc;
/// [`SimpleModel`]: in-memory layered [`Model`] implementation.
pub(crate) mod layered;

use std::{fmt, mem};

use fznso_types::{
	FznsoAnnotation, FznsoAnnotationMethods, FznsoAnnotationRef, FznsoConstraintIdx,
	FznsoDecisionIdx, FznsoModelMethods, FznsoModelRef, FznsoPtr, FznsoStr, FznsoType,
	FznsoValueRef,
};

use crate::{AnnotationRef, ConstraintIdx, DecisionIdx, Value};

/// Reinterpret a vtable thunk that takes an untagged `*const FznsoModel` as one
/// that takes a lifetime-tagged [`FznsoPtr`].
///
/// Any thunk whose return type carries a lifetime would have to borrow from the
/// model for the tagged pointer's lifetime, which requires `M: 'static` and so
/// rules out models that themselves borrow — including
/// [`ModelRefAdapter`](adapter::ModelRefAdapter), which every exported solver
/// is handed. Such thunks therefore take an untagged pointer and return
/// `'static`, and are reattached to the tagged signature here.
///
/// # Safety
///
/// `Raw` and `Borrowed` must be function pointers that differ only in ways that
/// do not affect their ABI. That holds for the uses below because:
///
/// 1. [`FznsoPtr`] is `repr(transparent)` over `*const T`, so the argument
///    layout is identical.
/// 2. [`FznsoValueRef`], [`FznsoAnnotationRef`] and [`FznsoStr`] are covariant
///    in their lifetime, so a thunk returning `'static` satisfies the shorter
///    lifetime the caller observes.
const unsafe fn as_borrowing_thunk<Raw: Copy, Borrowed: Copy>(thunk: Raw) -> Borrowed {
	const { assert!(size_of::<Raw>() == size_of::<Borrowed>()) };
	// SAFETY: guaranteed by the caller.
	unsafe { mem::transmute_copy(&thunk) }
}

/// Wraps `&'a A` so it can be converted into [`FznsoAnnotationRef`] via a
/// static vtable.  Used internally by [`ann_ref`] and [`SimpleModel`].
pub(crate) struct AnnWrapper<'a, A: Annotation>(pub(crate) &'a A);

/// Helper trait for types that can be used as annotations in a [`Model`].
///
/// Implement this trait and use [`ann_ref`] to convert values into the
/// [`AnnotationRef`] type returned by [`Model`] annotation methods.
pub trait Annotation {
	/// The annotation argument at the given zero-based `index`.
	fn argument(&self, index: usize) -> Value<'_>;
	/// The number of arguments this annotation carries.
	fn argument_len(&self) -> usize;

	/// The annotation's identifier string (e.g. `"output_var"`).
	fn ident(&self) -> &str;
}

/// A view into a single constraint within a [`Model`].
///
/// Obtained via [`Model::constraint`] or by iterating [`Model::constraints`].
#[derive(Clone, Copy)]
pub struct ConstraintView<'m, M: ?Sized> {
	/// The model this constraint belongs to.
	model: &'m M,
	/// The global index of the constraint within `model`.
	idx: ConstraintIdx,
}

/// A view into a single decision variable within a [`Model`].
///
/// Obtained via [`Model::decision`] or by iterating [`Model::decisions`].
#[derive(Clone, Copy)]
pub struct DecisionView<'m, M: ?Sized> {
	/// The model this decision variable belongs to.
	model: &'m M,
	/// The global index of the decision variable within `model`.
	idx: DecisionIdx,
}

/// A view into a single layer within a [`Model`].
///
/// Obtained via [`Model::layer`] or by iterating [`Model::layers`].
#[derive(Clone, Copy)]
pub struct LayerView<'m, M: ?Sized> {
	/// The model this layer belongs to.
	model: &'m M,
	/// The index of the layer within `model`.
	idx: usize,
}

/// A view into the objective of a [`Model`].
///
/// Obtained via [`Model::objective`].
#[derive(Clone, Copy)]
pub struct ObjectiveView<'m, M: ?Sized> {
	/// The model whose objective this view describes.
	model: &'m M,
}

impl<M: ?Sized> fmt::Debug for ConstraintView<'_, M> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("ConstraintView").field(&self.idx).finish()
	}
}

impl<M: ?Sized> fmt::Debug for DecisionView<'_, M> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("DecisionView").field(&self.idx).finish()
	}
}

impl<M: ?Sized> fmt::Debug for LayerView<'_, M> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_tuple("LayerView").field(&self.idx).finish()
	}
}

impl<M: ?Sized> fmt::Debug for ObjectiveView<'_, M> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("ObjectiveView").finish()
	}
}

impl<'m, M: Model + ?Sized> ConstraintView<'m, M> {
	/// The global index of this constraint.
	pub fn idx(&self) -> ConstraintIdx {
		self.idx
	}
	/// The identifier of this constraint (e.g. `"int_le"`).
	pub fn ident(&self) -> &'m str {
		self.model.constraint_ident(self.idx)
	}
	/// Number of arguments of this constraint.
	pub fn argument_len(&self) -> usize {
		self.model.constraint_argument_len(self.idx)
	}
	/// The `i`-th argument of this constraint.
	pub fn argument(&self, i: usize) -> Value<'m> {
		self.model.constraint_argument(self.idx, i)
	}
	/// Iterator over all arguments of this constraint.
	pub fn arguments(&self) -> impl Iterator<Item = Value<'m>> + 'm
	where
		M: Sized,
	{
		let model = self.model;
		let idx = self.idx;
		(0..model.constraint_argument_len(idx)).map(move |i| model.constraint_argument(idx, i))
	}
	/// The decision variable functionally defined by this constraint, if any.
	pub fn defines(&self) -> Option<DecisionIdx> {
		self.model.constraint_defines(self.idx)
	}
	/// Number of annotations on this constraint.
	pub fn annotation_len(&self) -> usize {
		self.model.constraint_annotation_len(self.idx)
	}
	/// Iterator over all annotations on this constraint.
	pub fn annotations(&self) -> impl Iterator<Item = AnnotationRef<'m>> + 'm
	where
		M: Sized,
	{
		let model = self.model;
		let idx = self.idx;
		(0..model.constraint_annotation_len(idx)).map(move |i| model.constraint_annotation(idx, i))
	}
}

impl<'m, M: Model + ?Sized> DecisionView<'m, M> {
	/// The global index of this decision variable.
	pub fn idx(&self) -> DecisionIdx {
		self.idx
	}
	/// The domain of this decision variable.
	pub fn domain(&self) -> Value<'m> {
		self.model.decision_domain(self.idx)
	}
	/// The type of this decision variable.
	pub fn ty(&self) -> FznsoType {
		self.model.decision_type(self.idx)
	}
	/// The optional name of this decision variable.
	pub fn name(&self) -> Option<&'m str> {
		self.model.decision_name(self.idx)
	}
	/// Whether this decision variable is functionally defined by some
	/// constraint.
	pub fn defined(&self) -> bool {
		self.model.decision_defined(self.idx)
	}
	/// Whether a solution may be asked for this decision variable's value.
	pub fn in_solution(&self) -> bool {
		self.model.decision_in_solution(self.idx)
	}
	/// Number of annotations on this decision variable.
	pub fn annotation_len(&self) -> usize {
		self.model.decision_annotation_len(self.idx)
	}
	/// Iterator over all annotations on this decision variable.
	pub fn annotations(&self) -> impl Iterator<Item = AnnotationRef<'m>> + 'm
	where
		M: Sized,
	{
		let model = self.model;
		let idx = self.idx;
		(0..model.decision_annotation_len(idx)).map(move |i| model.decision_annotation(idx, i))
	}
}

impl<'m, M: Model + ?Sized> LayerView<'m, M> {
	/// The index of this layer within the model.
	pub fn idx(&self) -> usize {
		self.idx
	}
	/// Whether this layer is permanently committed (will never be popped).
	pub fn is_permanent(&self) -> bool {
		self.idx < self.model.layer_permanent()
	}
	/// Whether this layer's content is unchanged since the last solver run.
	pub fn is_unchanged(&self) -> bool {
		self.idx < self.model.layer_unchanged()
	}
	/// Whether this permanent layer has been marked redundant.
	pub fn is_redundant(&self) -> bool {
		(0..self.model.layer_redundant_len())
			.any(|i| self.model.layer_redundant_index(i) == self.idx)
	}
	/// Number of decision variables in this layer.
	pub fn decision_len(&self) -> usize {
		let end = self.model.decision_layer_end(self.idx);
		if self.idx == 0 {
			end
		} else {
			end - self.model.decision_layer_end(self.idx - 1)
		}
	}
	/// Number of constraints in this layer.
	pub fn constraint_len(&self) -> usize {
		let end = self.model.constraint_layer_end(self.idx);
		if self.idx == 0 {
			end
		} else {
			end - self.model.constraint_layer_end(self.idx - 1)
		}
	}
	/// Iterator over decision variables belonging to this layer.
	pub fn decisions(&self) -> impl Iterator<Item = DecisionView<'m, M>> + 'm
	where
		M: Sized,
	{
		let model = self.model;
		let idx = self.idx;
		let start = if idx == 0 {
			0
		} else {
			model.decision_layer_end(idx - 1)
		};
		let end = model.decision_layer_end(idx);
		(start..end).map(move |i| DecisionView {
			model,
			idx: FznsoDecisionIdx(i),
		})
	}
	/// Iterator over constraints belonging to this layer.
	pub fn constraints(&self) -> impl Iterator<Item = ConstraintView<'m, M>> + 'm
	where
		M: Sized,
	{
		let model = self.model;
		let idx = self.idx;
		let start = if idx == 0 {
			0
		} else {
			model.constraint_layer_end(idx - 1)
		};
		let end = model.constraint_layer_end(idx);
		(start..end).map(move |i| ConstraintView {
			model,
			idx: FznsoConstraintIdx(i),
		})
	}
}

impl<'m, M: Model + ?Sized> ObjectiveView<'m, M> {
	/// The objective function identifier (e.g. `"minimize"`), or `""` for
	/// satisfaction problems.
	pub fn ident(&self) -> &'m str {
		self.model.objective_ident()
	}
	/// Whether this is a satisfaction problem (no objective).
	pub fn is_satisfy(&self) -> bool {
		self.model.objective_ident().is_empty()
	}
	/// The argument of the objective function.
	pub fn arg(&self) -> Value<'m> {
		self.model.objective_arg()
	}
	/// Number of annotations on the objective.
	pub fn annotation_len(&self) -> usize {
		self.model.objective_annotation_len()
	}
	/// Iterator over all annotations on the objective.
	pub fn annotations(&self) -> impl Iterator<Item = AnnotationRef<'m>> + 'm
	where
		M: Sized,
	{
		let model = self.model;
		(0..model.objective_annotation_len()).map(move |i| model.objective_annotation(i))
	}
}

/// Read-only view of a constraint/optimization problem model.
///
/// The model is organised into *layers* that correspond to the incremental
/// solving concept in the FZnSO protocol.  Solvers receive a `&dyn Model` (or
/// a concrete implementation via [`ModelRefAdapter`](adapter::ModelRefAdapter))
/// and may query decisions, constraints, and the objective through this trait.
///
/// # Ergonomic access
///
/// Rather than calling the low-level index/len primitives directly, prefer the
/// provided methods, which return view types with richer APIs and iterators:
///
/// ```rust
/// use fznso::Model;
///
/// fn print_model<M: Model>(model: &M) {
///     for layer in model.layers() {
///         for dec in layer.decisions() {
///             println!("{:?} domain={:?}", dec.name(), dec.domain().kind());
///         }
///         for con in layer.constraints() {
///             println!("{} ({})", con.ident(), con.argument_len());
///         }
///     }
///     let obj = model.objective();
///     if !obj.is_satisfy() {
///         println!("objective: {} {:?}", obj.ident(), obj.arg().kind());
///     }
/// }
/// ```
pub trait Model {
	// --- Constraint primitives ---
	/// The `index`-th annotation on the given constraint.
	fn constraint_annotation(&self, con: ConstraintIdx, index: usize) -> AnnotationRef<'_>;
	/// Number of annotations on the given constraint.
	fn constraint_annotation_len(&self, con: ConstraintIdx) -> usize;
	/// The `index`-th argument of the given constraint.
	fn constraint_argument(&self, con: ConstraintIdx, index: usize) -> Value<'_>;
	/// Number of arguments of the given constraint.
	fn constraint_argument_len(&self, con: ConstraintIdx) -> usize;
	/// The decision variable defined by this constraint, if any.
	fn constraint_defines(&self, con: ConstraintIdx) -> Option<DecisionIdx>;
	/// The identifier of the given constraint (e.g. `"int_le"`).
	fn constraint_ident(&self, con: ConstraintIdx) -> &str;
	/// One-past-the-end global constraint index for constraints in layers
	/// `0..=layer`.
	fn constraint_layer_end(&self, layer: usize) -> usize;
	/// Total number of constraints across all layers.
	fn constraint_len(&self) -> usize;

	// --- Decision-variable primitives ---
	/// The `index`-th annotation on the given decision variable.
	fn decision_annotation(&self, decision: DecisionIdx, index: usize) -> AnnotationRef<'_>;
	/// Number of annotations on the given decision variable.
	fn decision_annotation_len(&self, decision: DecisionIdx) -> usize;
	/// Whether the given decision variable is defined by some constraint.
	fn decision_defined(&self, decision: DecisionIdx) -> bool;
	/// Whether a solution may be asked for the given decision variable's value.
	///
	/// A solver must give every such variable a value in every solution it
	/// reports, and is free to leave the others open. Having a name does not
	/// make a variable needed: names exist only for debugging.
	fn decision_in_solution(&self, decision: DecisionIdx) -> bool;
	/// The domain of the given decision variable.
	fn decision_domain(&self, decision: DecisionIdx) -> Value<'_>;
	/// One-past-the-end global decision index for variables in layers
	/// `0..=layer`.
	fn decision_layer_end(&self, layer: usize) -> usize;
	/// Total number of decision variables across all layers.
	fn decision_len(&self) -> usize;
	/// The optional name of the given decision variable.
	fn decision_name(&self, decision: DecisionIdx) -> Option<&str>;
	/// The type of the given decision variable.
	///
	/// What the variable *is*, as opposed to [`Model::decision_domain`], which
	/// says which values it may take. `list_of` is always clear and `decision`
	/// always set, so only `base`, `set_of` and `opt` carry information.
	fn decision_type(&self, decision: DecisionIdx) -> FznsoType;

	// --- Layer primitives ---
	/// Total number of layers currently in the model.
	fn layer_len(&self) -> usize;
	/// Number of layers that are permanently committed (will not be popped).
	fn layer_permanent(&self) -> usize;
	/// The global layer index of the `index`-th redundant permanent layer.
	fn layer_redundant_index(&self, index: usize) -> usize;
	/// Number of permanent layers marked as redundant.
	fn layer_redundant_len(&self) -> usize;
	/// Number of layers whose content is unchanged since the last solver run.
	fn layer_unchanged(&self) -> usize;

	// --- Objective primitives ---
	/// The `index`-th annotation on the objective.
	fn objective_annotation(&self, index: usize) -> AnnotationRef<'_>;
	/// Number of annotations on the objective.
	fn objective_annotation_len(&self) -> usize;
	/// The argument of the objective function.
	fn objective_arg(&self) -> Value<'_>;
	/// Identifier of the objective function (e.g. `"minimize"`), or `""` for
	/// satisfaction.
	fn objective_ident(&self) -> &str;

	// --- Provided ergonomic accessors ---

	/// A view into the constraint at `idx`.
	fn constraint(&self, idx: ConstraintIdx) -> ConstraintView<'_, Self>
	where
		Self: Sized,
	{
		ConstraintView { model: self, idx }
	}

	/// Iterator over all constraints in the model, ordered by layer.
	fn constraints(&self) -> impl Iterator<Item = ConstraintView<'_, Self>>
	where
		Self: Sized,
	{
		(0..self.constraint_len()).map(move |i| self.constraint(FznsoConstraintIdx(i)))
	}

	/// A view into the decision variable at `idx`.
	fn decision(&self, idx: DecisionIdx) -> DecisionView<'_, Self>
	where
		Self: Sized,
	{
		DecisionView { model: self, idx }
	}

	/// Iterator over all decision variables in the model, ordered by layer.
	fn decisions(&self) -> impl Iterator<Item = DecisionView<'_, Self>>
	where
		Self: Sized,
	{
		(0..self.decision_len()).map(move |i| self.decision(FznsoDecisionIdx(i)))
	}

	/// A view into the layer at `idx`.
	fn layer(&self, idx: usize) -> LayerView<'_, Self>
	where
		Self: Sized,
	{
		LayerView { model: self, idx }
	}

	/// Iterator over all layers, from the oldest (base) to the newest.
	fn layers(&self) -> impl Iterator<Item = LayerView<'_, Self>>
	where
		Self: Sized,
	{
		(0..self.layer_len()).map(move |i| self.layer(i))
	}

	/// A view into the model's objective.
	fn objective(&self) -> ObjectiveView<'_, Self>
	where
		Self: Sized,
	{
		ObjectiveView { model: self }
	}
}

/// Wraps `&'a M` so it can be converted into [`FznsoModelRef`] via a static
/// vtable.  Used by [`Library::solver_run`](crate::Library) and the
/// [`fznso_export`](crate::fznso_export) macro.
pub(crate) struct ModelWrapper<'a, M: Model>(pub(crate) &'a M);

/// Convert a reference to a [`Model`] implementation into the
/// [`FznsoModelRef`] handle expected by the C ABI.
///
/// Used by [`Library::solver_run`](crate::Library) and by the crate's own
/// tests; callers of the library never construct an ABI handle themselves.
pub(crate) fn model_ref<M: Model>(model: &M) -> FznsoModelRef<'_> {
	ModelWrapper(model).into()
}

/// Convert a reference to an [`Annotation`] implementation into an
/// [`AnnotationRef`] suitable for returning from [`Model`] trait methods.
pub fn ann_ref<'a, A: Annotation + 'static>(ann: &'a A) -> AnnotationRef<'a> {
	AnnWrapper(ann).into()
}

/// Vtable thunk: returns the `index`-th argument of annotation `A`.
extern "C" fn wrap_ann_argument<'a, A: Annotation + 'static>(
	ptr: FznsoPtr<'a, FznsoAnnotation>,
	index: usize,
) -> FznsoValueRef<'a> {
	// SAFETY: ptr was created from &A in AnnWrapper's From impl via transmute.
	let ann: &A = unsafe { &*ptr.cast::<A>() };
	ann.argument(index)
}

/// Vtable thunk: returns the argument count of annotation `A`.
extern "C" fn wrap_ann_argument_len<A: Annotation + 'static>(
	ptr: FznsoPtr<'_, FznsoAnnotation>,
) -> usize {
	// SAFETY: ptr was created from &A in AnnWrapper's From impl via transmute.
	let ann: &A = unsafe { &*ptr.cast::<A>() };
	ann.argument_len()
}

/// Vtable thunk: returns the identifier of annotation `A`.
extern "C" fn wrap_ann_ident<'a, A: Annotation + 'static>(
	ptr: FznsoPtr<'a, FznsoAnnotation>,
) -> FznsoStr<'a> {
	// SAFETY: ptr was created from &A in AnnWrapper's From impl via transmute.
	let ann: &A = unsafe { &*ptr.cast::<A>() };
	ann.ident().into()
}

/// Vtable thunk: delegates to [`Model::constraint_annotation`].
extern "C" fn wrap_constraint_annotation<M: Model>(
	ptr: *const fznso_types::FznsoModel,
	con: FznsoConstraintIdx,
	index: usize,
) -> FznsoAnnotationRef<'static> {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	// SAFETY: transmuting 'model to 'static; the caller (ModelRefAdapter) must
	// not use the returned reference past the model's actual lifetime.
	unsafe { mem::transmute(model.constraint_annotation(con, index)) }
}

/// Vtable thunk: delegates to [`Model::constraint_annotation_len`].
extern "C" fn wrap_constraint_annotation_len<M: Model>(
	ptr: FznsoPtr<'_, fznso_types::FznsoModel>,
	index: FznsoConstraintIdx,
) -> usize {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.constraint_annotation_len(index)
}

/// Vtable thunk: delegates to [`Model::constraint_argument`].
extern "C" fn wrap_constraint_argument<M: Model>(
	ptr: *const fznso_types::FznsoModel,
	con: FznsoConstraintIdx,
	index: usize,
) -> FznsoValueRef<'static> {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	// SAFETY: transmuting 'model to 'static; the caller (ModelRefAdapter) must
	// not use the returned reference past the model's actual lifetime.
	unsafe { mem::transmute(model.constraint_argument(con, index)) }
}

/// Vtable thunk: delegates to [`Model::constraint_argument_len`].
extern "C" fn wrap_constraint_argument_len<M: Model>(
	ptr: FznsoPtr<'_, fznso_types::FznsoModel>,
	index: FznsoConstraintIdx,
) -> usize {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.constraint_argument_len(index)
}

/// Vtable thunk: delegates to [`Model::constraint_defines`], encoding `None`
/// as the absent value.
extern "C" fn wrap_constraint_defines<M: Model>(
	ptr: *const fznso_types::FznsoModel,
	index: FznsoConstraintIdx,
) -> FznsoValueRef<'static> {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	match model.constraint_defines(index) {
		// From<FznsoDecisionIdx> for FznsoValueRef<'static> encodes the index
		// value directly in the pointer field — no borrow from model.
		Some(def) => def.into(),
		None => {
			static ABSENT: () = ();
			(&ABSENT).into()
		}
	}
}

/// Vtable thunk: delegates to [`Model::constraint_ident`].
extern "C" fn wrap_constraint_ident<M: Model>(
	ptr: *const fznso_types::FznsoModel,
	index: FznsoConstraintIdx,
) -> FznsoStr<'static> {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	// SAFETY: widening the borrow to 'static; `as_borrowing_thunk` reattaches
	// the caller's shorter lifetime before this is ever observed.
	unsafe {
		mem::transmute::<FznsoStr<'_>, FznsoStr<'static>>(model.constraint_ident(index).into())
	}
}

/// Vtable thunk: delegates to [`Model::constraint_layer_end`].
extern "C" fn wrap_constraint_layer_end<M: Model>(
	ptr: FznsoPtr<'_, fznso_types::FznsoModel>,
	layer: usize,
) -> usize {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.constraint_layer_end(layer)
}

/// Vtable thunk: delegates to [`Model::constraint_len`].
extern "C" fn wrap_constraint_len<M: Model>(ptr: FznsoPtr<'_, fznso_types::FznsoModel>) -> usize {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.constraint_len()
}

/// Vtable thunk: delegates to [`Model::decision_annotation`].
extern "C" fn wrap_decision_annotation<M: Model>(
	ptr: *const fznso_types::FznsoModel,
	decision: FznsoDecisionIdx,
	index1: usize,
) -> FznsoAnnotationRef<'static> {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	// SAFETY: transmuting 'model to 'static; the caller (ModelRefAdapter) must
	// not use the returned reference past the model's actual lifetime.
	unsafe { mem::transmute(model.decision_annotation(decision, index1)) }
}

/// Vtable thunk: delegates to [`Model::decision_annotation_len`].
extern "C" fn wrap_decision_annotation_len<M: Model>(
	ptr: FznsoPtr<'_, fznso_types::FznsoModel>,
	index: FznsoDecisionIdx,
) -> usize {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.decision_annotation_len(index)
}

/// Vtable thunk: delegates to [`Model::decision_defined`].
extern "C" fn wrap_decision_defined<M: Model>(
	ptr: FznsoPtr<'_, fznso_types::FznsoModel>,
	index: FznsoDecisionIdx,
) -> bool {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.decision_defined(index)
}

/// Vtable thunk: delegates to [`Model::decision_in_solution`].
extern "C" fn wrap_decision_in_solution<M: Model>(
	ptr: FznsoPtr<'_, fznso_types::FznsoModel>,
	index: FznsoDecisionIdx,
) -> bool {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.decision_in_solution(index)
}

/// Vtable thunk: delegates to [`Model::decision_domain`].
extern "C" fn wrap_decision_domain<M: Model>(
	ptr: *const fznso_types::FznsoModel,
	index: FznsoDecisionIdx,
) -> FznsoValueRef<'static> {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	// SAFETY: transmuting 'model to 'static; the caller (ModelRefAdapter) must
	// not use the returned reference past the model's actual lifetime.
	unsafe { mem::transmute(model.decision_domain(index)) }
}

/// Vtable thunk: delegates to [`Model::decision_type`].
extern "C" fn wrap_decision_type<M: Model>(
	ptr: FznsoPtr<'_, fznso_types::FznsoModel>,
	index: FznsoDecisionIdx,
) -> FznsoType {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.decision_type(index)
}

/// Vtable thunk: delegates to [`Model::decision_layer_end`].
extern "C" fn wrap_decision_layer_end<M: Model>(
	ptr: FznsoPtr<'_, fznso_types::FznsoModel>,
	layer: usize,
) -> usize {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.decision_layer_end(layer)
}

/// Vtable thunk: delegates to [`Model::decision_len`].
extern "C" fn wrap_decision_len<M: Model>(ptr: FznsoPtr<'_, fznso_types::FznsoModel>) -> usize {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.decision_len()
}

/// Vtable thunk: delegates to [`Model::decision_name`].
extern "C" fn wrap_decision_name<M: Model>(
	ptr: *const fznso_types::FznsoModel,
	index: FznsoDecisionIdx,
) -> FznsoStr<'static> {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	let name = model
		.decision_name(index)
		.map(FznsoStr::from)
		.unwrap_or(FznsoStr::NONE);
	// SAFETY: widening the borrow to 'static; `as_borrowing_thunk` reattaches
	// the caller's shorter lifetime before this is ever observed.
	unsafe { mem::transmute::<FznsoStr<'_>, FznsoStr<'static>>(name) }
}

/// Vtable thunk: delegates to [`Model::layer_len`].
extern "C" fn wrap_layer_len<M: Model>(ptr: FznsoPtr<'_, fznso_types::FznsoModel>) -> usize {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.layer_len()
}

/// Vtable thunk: delegates to [`Model::layer_permanent`].
extern "C" fn wrap_layer_permanent<M: Model>(ptr: FznsoPtr<'_, fznso_types::FznsoModel>) -> usize {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.layer_permanent()
}

/// Vtable thunk: delegates to [`Model::layer_redundant_index`].
extern "C" fn wrap_layer_redundant_index<M: Model>(
	ptr: FznsoPtr<'_, fznso_types::FznsoModel>,
	index: usize,
) -> usize {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.layer_redundant_index(index)
}

/// Vtable thunk: delegates to [`Model::layer_redundant_len`].
extern "C" fn wrap_layer_redundant_len<M: Model>(
	ptr: FznsoPtr<'_, fznso_types::FznsoModel>,
) -> usize {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.layer_redundant_len()
}

/// Vtable thunk: delegates to [`Model::layer_unchanged`].
extern "C" fn wrap_layer_unchanged<M: Model>(ptr: FznsoPtr<'_, fznso_types::FznsoModel>) -> usize {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.layer_unchanged()
}

/// Vtable thunk: delegates to [`Model::objective_annotation`].
extern "C" fn wrap_objective_annotation<M: Model>(
	ptr: *const fznso_types::FznsoModel,
	index: usize,
) -> FznsoAnnotationRef<'static> {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	// SAFETY: transmuting 'model to 'static; the caller (ModelRefAdapter) must
	// not use the returned reference past the model's actual lifetime.
	unsafe { mem::transmute(model.objective_annotation(index)) }
}

/// Vtable thunk: delegates to [`Model::objective_annotation_len`].
extern "C" fn wrap_objective_annotation_len<M: Model>(
	ptr: FznsoPtr<'_, fznso_types::FznsoModel>,
) -> usize {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	model.objective_annotation_len()
}

/// Vtable thunk: delegates to [`Model::objective_arg`].
extern "C" fn wrap_objective_arg<M: Model>(
	ptr: *const fznso_types::FznsoModel,
) -> FznsoValueRef<'static> {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl via transmute.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	// SAFETY: transmuting 'model to 'static; the caller (ModelRefAdapter) must
	// not use the returned reference past the model's actual lifetime.
	unsafe { mem::transmute(model.objective_arg()) }
}

/// Vtable thunk: delegates to [`Model::objective_ident`].
extern "C" fn wrap_objective_ident<M: Model>(
	ptr: *const fznso_types::FznsoModel,
) -> FznsoStr<'static> {
	// SAFETY: ptr was created from &M in ModelWrapper's From impl.
	let model: &M = unsafe { &*ptr.cast::<M>() };
	// SAFETY: widening the borrow to 'static; `as_borrowing_thunk` reattaches
	// the caller's shorter lifetime before this is ever observed.
	unsafe { mem::transmute::<FznsoStr<'_>, FznsoStr<'static>>(model.objective_ident().into()) }
}

impl<'a, A: Annotation + 'static> AnnWrapper<'a, A> {
	/// Vtable for `A`'s annotation interface. Each monomorphization is a
	/// distinct constant whose address is promoted to `'static` by the
	/// compiler.
	const METHODS: FznsoAnnotationMethods = FznsoAnnotationMethods {
		ident: wrap_ann_ident::<A>,
		argument_len: wrap_ann_argument_len::<A>,
		argument: wrap_ann_argument::<A>,
	};
}

impl<'a, A: Annotation + 'static> From<AnnWrapper<'a, A>> for FznsoAnnotationRef<'a> {
	fn from(value: AnnWrapper<'a, A>) -> Self {
		FznsoAnnotationRef {
			methods: &AnnWrapper::<A>::METHODS,
			data: FznsoPtr::from_ref(value.0),
		}
	}
}

impl<'a, M: Model> From<ModelWrapper<'a, M>> for FznsoModelRef<'a> {
	fn from(value: ModelWrapper<'a, M>) -> Self {
		FznsoModelRef {
			methods: &ModelWrapper::<M>::METHODS,
			data: FznsoPtr::from_ref(value.0),
		}
	}
}

impl<'a, M: Model> ModelWrapper<'a, M> {
	/// Vtable for `M`'s model interface. Each monomorphization is a distinct
	/// constant whose address is promoted to `'static` by the compiler.
	// SAFETY: every `as_borrowing_thunk` below reattaches a thunk to the tagged
	// signature it was written against; see that function's documentation.
	const METHODS: FznsoModelMethods = unsafe {
		FznsoModelMethods {
			layer_len: wrap_layer_len::<M>,
			layer_unchanged: wrap_layer_unchanged::<M>,
			layer_permanent: wrap_layer_permanent::<M>,
			layer_redundant_len: wrap_layer_redundant_len::<M>,
			layer_redundant_index: wrap_layer_redundant_index::<M>,
			decision_len: wrap_decision_len::<M>,
			decision_layer_end: wrap_decision_layer_end::<M>,
			decision_type: wrap_decision_type::<M>,
			decision_domain: as_borrowing_thunk::<
				extern "C" fn(
					*const fznso_types::FznsoModel,
					FznsoDecisionIdx,
				) -> FznsoValueRef<'static>,
				_,
			>(wrap_decision_domain::<M>),
			decision_name: as_borrowing_thunk::<
				extern "C" fn(
					*const fznso_types::FznsoModel,
					FznsoDecisionIdx,
				) -> FznsoStr<'static>,
				_,
			>(wrap_decision_name::<M>),
			decision_defined: wrap_decision_defined::<M>,
			decision_in_solution: wrap_decision_in_solution::<M>,
			decision_annotation_len: wrap_decision_annotation_len::<M>,
			decision_annotation: as_borrowing_thunk::<
				extern "C" fn(
					*const fznso_types::FznsoModel,
					FznsoDecisionIdx,
					usize,
				) -> FznsoAnnotationRef<'static>,
				_,
			>(wrap_decision_annotation::<M>),
			constraint_len: wrap_constraint_len::<M>,
			constraint_layer_end: wrap_constraint_layer_end::<M>,
			constraint_ident: as_borrowing_thunk::<
				extern "C" fn(
					*const fznso_types::FznsoModel,
					FznsoConstraintIdx,
				) -> FznsoStr<'static>,
				_,
			>(wrap_constraint_ident::<M>),
			constraint_argument_len: wrap_constraint_argument_len::<M>,
			constraint_argument: as_borrowing_thunk::<
				extern "C" fn(
					*const fznso_types::FznsoModel,
					FznsoConstraintIdx,
					usize,
				) -> FznsoValueRef<'static>,
				_,
			>(wrap_constraint_argument::<M>),
			constraint_defines: as_borrowing_thunk::<
				extern "C" fn(
					*const fznso_types::FznsoModel,
					FznsoConstraintIdx,
				) -> FznsoValueRef<'static>,
				_,
			>(wrap_constraint_defines::<M>),
			constraint_annotation_len: wrap_constraint_annotation_len::<M>,
			constraint_annotation: as_borrowing_thunk::<
				extern "C" fn(
					*const fznso_types::FznsoModel,
					FznsoConstraintIdx,
					usize,
				) -> FznsoAnnotationRef<'static>,
				_,
			>(wrap_constraint_annotation::<M>),
			objective_ident: as_borrowing_thunk::<
				extern "C" fn(*const fznso_types::FznsoModel) -> FznsoStr<'static>,
				_,
			>(wrap_objective_ident::<M>),
			objective_arg: as_borrowing_thunk::<
				extern "C" fn(*const fznso_types::FznsoModel) -> FznsoValueRef<'static>,
				_,
			>(wrap_objective_arg::<M>),
			objective_annotation_len: wrap_objective_annotation_len::<M>,
			objective_annotation: as_borrowing_thunk::<
				extern "C" fn(*const fznso_types::FznsoModel, usize) -> FznsoAnnotationRef<'static>,
				_,
			>(wrap_objective_annotation::<M>),
		}
	};
}

#[cfg(test)]
mod tests {
	use fznso_types::{FznsoDecisionIdx, FznsoValueKind};
	use rangelist::RangeList;

	use crate::{
		LayeredModel, Model, OwnedAnnotation, OwnedValue, Type, TypeBase,
		model::{ModelWrapper, adapter::ModelRefAdapter},
	};

	/// Build a model exercising every value kind, both annotation sites, and
	/// more than one layer.
	fn sample() -> LayeredModel {
		let mut m = LayeredModel::default();
		let x = m.add_decision(
			Type::new(TypeBase::FznsoTypeBaseInt).decision(true),
			OwnedValue::IntSet(RangeList::from(-10_i64..=10)),
			Some("x".into()),
			false,
			true,
			vec![OwnedAnnotation::new("output_var", vec![])],
		);
		let y = m.add_decision(
			Type::new(TypeBase::FznsoTypeBaseFloat).decision(true),
			OwnedValue::FloatSet(RangeList::from(0.5_f64..=2.5)),
			None,
			true,
			false,
			vec![],
		);
		let _ = m.add_constraint(
			"int_le",
			vec![OwnedValue::Int(-10), OwnedValue::Decision(x)],
			None,
			vec![],
		);
		m.mark_permanent();

		m.push_layer();
		let _ = m.add_constraint(
			"float_mixed",
			vec![
				OwnedValue::Float(1.5),
				OwnedValue::Bool(true),
				OwnedValue::Bool(false),
				OwnedValue::String("hello".into()),
				OwnedValue::List(vec![OwnedValue::Int(1), OwnedValue::Decision(y)]),
				OwnedValue::Absent,
			],
			Some(y),
			vec![OwnedAnnotation::new(
				"defines_var",
				vec![OwnedValue::Decision(y)],
			)],
		);
		m.set_objective(Some("minimize"), OwnedValue::Decision(x), vec![]);
		m
	}

	/// Every getter must give the same answer before and after a round trip
	/// through the C ABI vtables.
	#[test]
	fn abi_round_trip() {
		let m = sample();
		let abi = ModelRefAdapter::new(ModelWrapper(&m).into());

		assert_eq!(abi.layer_len(), m.layer_len());
		assert_eq!(abi.layer_permanent(), m.layer_permanent());
		assert_eq!(abi.decision_len(), m.decision_len());
		assert_eq!(abi.constraint_len(), m.constraint_len());
		assert_eq!(abi.objective_ident(), "minimize");

		for d in m.decisions().map(|d| d.idx()) {
			assert_eq!(abi.decision_name(d), m.decision_name(d));
			assert_eq!(abi.decision_defined(d), m.decision_defined(d));
			assert_eq!(abi.decision_in_solution(d), m.decision_in_solution(d));
			assert_eq!(abi.decision_domain(d).kind(), m.decision_domain(d).kind());
			assert_eq!(abi.decision_type(d), m.decision_type(d));
			assert_eq!(abi.decision_annotation_len(d), m.decision_annotation_len(d));
			for i in 0..m.decision_annotation_len(d) {
				assert_eq!(
					abi.decision_annotation(d, i).ident(),
					m.decision_annotation(d, i).ident()
				);
			}
		}

		for c in m.constraints().map(|c| c.idx()) {
			assert_eq!(abi.constraint_ident(c), m.constraint_ident(c));
			assert_eq!(abi.constraint_defines(c), m.constraint_defines(c));
			assert_eq!(abi.constraint_argument_len(c), m.constraint_argument_len(c));
			for i in 0..m.constraint_argument_len(c) {
				assert_eq!(
					abi.constraint_argument(c, i).kind(),
					m.constraint_argument(c, i).kind()
				);
			}
			for i in 0..m.constraint_annotation_len(c) {
				assert_eq!(
					abi.constraint_annotation(c, i).ident(),
					m.constraint_annotation(c, i).ident()
				);
			}
		}
	}

	/// The payload of each value must survive the round trip, not just its
	/// kind.
	#[test]
	fn abi_round_trip_values() {
		let m = sample();
		let abi = ModelRefAdapter::new(ModelWrapper(&m).into());
		let c = abi.constraints().nth(1).unwrap();

		assert_eq!(c.argument(0).kind(), FznsoValueKind::FznsoValueFloat);
		assert_eq!(c.argument(0).get_float(), 1.5);
		assert!(c.argument(1).get_bool());
		assert!(!c.argument(2).get_bool());
		assert_eq!(c.argument(3).get_string(), "hello");
		assert_eq!(c.argument(4).len(), 2);
		assert_eq!(c.argument(4).get_element(0).get_int(), 1);
		assert_eq!(c.argument(5).kind(), FznsoValueKind::FznsoValueAbsent);

		let x = abi.decision(FznsoDecisionIdx(0));
		assert_eq!(x.name(), Some("x"));
		assert_eq!(x.domain().kind(), FznsoValueKind::FznsoValueSetInt);
		assert_eq!(x.domain().get_range_int(0), (-10, 10));

		let y = abi.decision(FznsoDecisionIdx(1));
		assert_eq!(y.name(), None);
		assert_eq!(y.domain().kind(), FznsoValueKind::FznsoValueSetFloat);
		assert_eq!(y.domain().get_range_float(0), (0.5, 2.5));
	}
}
