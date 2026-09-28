//! [`LayeredModel`]: an in-memory, multi-layer [`Model`] implementation for testing and modelling.

use fznso_types::{
	FznsoAnnotation, FznsoAnnotationMethods, FznsoAnnotationRef, FznsoConstraintIdx,
	FznsoDecisionIdx, FznsoFloatRange, FznsoIntRange, FznsoPtr, FznsoStr, FznsoType, FznsoValue,
	FznsoValueKind, FznsoValueMethods, FznsoValueRef,
};
use rangelist::RangeList;

use crate::{
	AnnotationRef, ConstraintIdx, DecisionIdx, Value, ValueExt, ValueView,
	model::{Annotation, Model},
};

/// (De)serialize a [`RangeList`] as a list of `(min, max)` pairs, since
/// `rangelist` has no serde support.
#[cfg(feature = "serde")]
mod ranges {
	use rangelist::RangeList;
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	/// Serialize the ranges of `ranges`.
	pub(super) fn serialize<E, S>(ranges: &RangeList<E>, serializer: S) -> Result<S::Ok, S::Error>
	where
		E: Copy + PartialOrd + Serialize,
		S: Serializer,
	{
		serializer.collect_seq(ranges.iter().map(|r| (*r.start(), *r.end())))
	}

	/// Deserialize a list of ranges.
	pub(super) fn deserialize<'de, E, D>(deserializer: D) -> Result<RangeList<E>, D::Error>
	where
		E: rangelist::Adjacent + Clone + PartialOrd + Deserialize<'de>,
		D: Deserializer<'de>,
	{
		let pairs = Vec::<(E, E)>::deserialize(deserializer)?;
		Ok(pairs.into_iter().map(|(min, max)| min..=max).collect())
	}
}

/// Stores the data for a single constraint within a layer.
#[derive(Clone, Debug, Default)]
struct ConstraintData {
	/// The constraint identifier (e.g. `"int_le"`).
	ident: String,
	/// The constraint's argument values.
	arguments: Vec<OwnedValue>,
	/// Decision variable defined by this constraint, if any.
	defines: Option<DecisionIdx>,
	/// Annotations attached to this constraint.
	annotations: Vec<OwnedAnnotation>,
}

/// Stores the data for a single decision variable within a layer.
#[derive(Clone, Debug)]
struct DecisionData {
	/// The type of the decision variable.
	ty: FznsoType,
	/// The domain of the decision variable.
	domain: OwnedValue,
	/// Optional name of the decision variable.
	name: Option<String>,
	/// Whether this variable is defined by some constraint.
	defined: bool,
	/// Whether a solution may be asked for this variable's value.
	in_solution: bool,
	/// Annotations attached to this decision variable.
	annotations: Vec<OwnedAnnotation>,
}

/// Decisions and constraints belonging to one model layer.
#[derive(Clone, Debug, Default)]
struct LayerData {
	/// Decision variables added in this layer.
	decisions: Vec<DecisionData>,
	/// Constraints added in this layer.
	constraints: Vec<ConstraintData>,
}

/// An in-memory, multi-layer implementation of [`Model`].
///
/// Layers correspond to the incremental-solving concept in the FZnSO protocol:
/// decisions and constraints are added layer-by-layer. Layers can be pushed,
/// popped, marked permanent, and marked redundant.
///
/// The model is created with one permanent base layer already in place.
/// Additional layers can be pushed and popped on top of it; only the base
/// layer (and any layers subsequently committed via [`mark_permanent`]) can
/// never be popped.
///
/// The objective is global (not per-layer) and can be set at any time.
///
/// # Example
///
/// ```rust
/// use fznso::{LayeredModel, OwnedValue, Type, TypeBase};
/// use rangelist::RangeList;
///
/// let mut m = LayeredModel::default();
/// let d = m.add_decision(
///     Type::new(TypeBase::FznsoTypeBaseInt).decision(true),
///     OwnedValue::IntSet(RangeList::from(-10i64..=10)),
///     Some("x".into()),
///     false,
///     true,
///     vec![],
/// );
/// m.add_constraint("int_le", vec![OwnedValue::Int(-10), OwnedValue::Decision(d)], None, vec![]);
/// m.set_objective(Some("minimize"), OwnedValue::Decision(d), vec![]);
/// ```
///
/// [`mark_permanent`]: LayeredModel::mark_permanent
#[derive(Clone, Debug)]
pub struct LayeredModel {
	/// The stack of layers, ordered from oldest (permanent) to newest.
	layers: Vec<LayerData>,
	/// The model's single global objective.
	objective: ObjectiveData,
	/// Number of layers that are permanently committed.
	permanent: usize,
	/// Indices of permanent layers marked as redundant.
	redundant: Vec<usize>,
	/// Number of layers considered unchanged since the last solver run.
	unchanged: usize,
}

/// An annotation for use with [`LayeredModel`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OwnedAnnotation {
	/// The annotation identifier (e.g. `"output_var"`).
	ident: String,
	/// The annotation's argument values.
	arguments: Vec<OwnedValue>,
}

/// An owned value for use with [`LayeredModel`].
///
/// Mirrors all variants of [`FznsoValueKind`] but owns the contained data.
///
/// Copy a borrowed [`Value`] with [`From`]. That copy stores a set as a
/// [`RangeList`], which merges adjacent integer ranges such as `1..=2` and
/// `3..=4`.
///
/// With the `serde` feature, a set serializes as a list of `(min, max)` pairs.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OwnedValue {
	/// The absent / null value.
	#[default]
	Absent,
	/// A Boolean value.
	Bool(bool),
	/// A 64-bit signed integer value.
	Int(i64),
	/// A 64-bit IEEE 754 double-precision float value.
	Float(f64),
	/// A UTF-8 string value.
	String(String),
	/// A reference to a decision variable by its global index.
	Decision(DecisionIdx),
	/// A reference to a constraint by its global index.
	Constraint(ConstraintIdx),
	/// Integer set as a list of inclusive `[min, max]` ranges.
	IntSet(#[cfg_attr(feature = "serde", serde(with = "ranges"))] RangeList<i64>),
	/// Float set as a list of inclusive `[min, max]` ranges.
	FloatSet(#[cfg_attr(feature = "serde", serde(with = "ranges"))] RangeList<f64>),
	/// An ordered list of nested values.
	List(Vec<OwnedValue>),
}

/// The model's single global objective.
#[derive(Clone, Debug, Default)]
struct ObjectiveData {
	/// Objective identifier (e.g. `"minimize"`), empty for satisfaction.
	ident: String,
	/// The objective argument.
	arg: OwnedValue,
	/// Annotations on the objective.
	annotations: Vec<OwnedAnnotation>,
}

/// Build a [`FznsoValueRef`] for a float range list.
fn float_range_list_value(rl: &RangeList<f64>) -> FznsoValueRef<'_> {
	extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
		FznsoValueKind::FznsoValueSetFloat
	}
	extern "C" fn len(val: FznsoPtr<'_, FznsoValue>) -> usize {
		// SAFETY: val was created from &RangeList<f64> via transmute below.
		let rl: &RangeList<f64> = unsafe { &*val.cast() };
		rl.into_iter().count()
	}
	extern "C" fn get_range_float(val: FznsoPtr<'_, FznsoValue>, index: usize) -> FznsoFloatRange {
		// SAFETY: val was created from &RangeList<f64> via transmute below.
		let rl: &RangeList<f64> = unsafe { &*val.cast() };
		let r = rl.into_iter().nth(index).unwrap();
		FznsoFloatRange {
			min: **r.start(),
			max: **r.end(),
		}
	}
	const METHODS: &FznsoValueMethods = &FznsoValueMethods {
		kind,
		len,
		get_range_float,
		..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
	};
	FznsoValueRef {
		data: FznsoPtr::from_ref(rl),
		methods: METHODS,
	}
}

/// Build a [`FznsoValueRef`] for an integer range list.
fn int_range_list_value(rl: &RangeList<i64>) -> FznsoValueRef<'_> {
	extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
		FznsoValueKind::FznsoValueSetInt
	}
	extern "C" fn len(val: FznsoPtr<'_, FznsoValue>) -> usize {
		// SAFETY: val was created from &RangeList<i64> via transmute below.
		let rl: &RangeList<i64> = unsafe { &*val.cast() };
		rl.into_iter().count()
	}
	extern "C" fn get_range_int(val: FznsoPtr<'_, FznsoValue>, index: usize) -> FznsoIntRange {
		// SAFETY: val was created from &RangeList<i64> via transmute below.
		let rl: &RangeList<i64> = unsafe { &*val.cast() };
		let r = rl.into_iter().nth(index).unwrap();
		FznsoIntRange {
			min: **r.start(),
			max: **r.end(),
		}
	}
	const METHODS: &FznsoValueMethods = &FznsoValueMethods {
		kind,
		len,
		get_range_int,
		..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
	};
	FznsoValueRef {
		data: FznsoPtr::from_ref(rl),
		methods: METHODS,
	}
}

/// Build a [`FznsoValueRef`] for a list of [`OwnedValue`] items.
fn list_value(items: &Vec<OwnedValue>) -> FznsoValueRef<'_> {
	extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
		FznsoValueKind::FznsoValueList
	}
	extern "C" fn len(val: FznsoPtr<'_, FznsoValue>) -> usize {
		// SAFETY: val was created from &Vec<OwnedValue> via transmute in
		// list_value.
		let val: &Vec<OwnedValue> = unsafe { &*val.cast() };
		val.len()
	}
	extern "C" fn get_element<'a>(
		val: FznsoPtr<'_, FznsoValue>,
		index: usize,
	) -> FznsoValueRef<'a> {
		// SAFETY: val was created from &Vec<OwnedValue> via transmute in
		// list_value.
		let val: &Vec<OwnedValue> = unsafe { &*val.cast() };
		FznsoValueRef::from(&val[index])
	}
	const METHODS: &FznsoValueMethods = &FznsoValueMethods {
		kind,
		len,
		get_element,
		..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
	};
	FznsoValueRef {
		data: FznsoPtr::from_ref(items),
		methods: METHODS,
	}
}

impl<'a> From<&'a OwnedValue> for FznsoValueRef<'a> {
	fn from(val: &'a OwnedValue) -> Self {
		match val {
			OwnedValue::Absent => (&()).into(),
			OwnedValue::Bool(b) => b.into(),
			OwnedValue::Int(i) => i.into(),
			OwnedValue::Float(f) => f.into(),
			OwnedValue::String(s) => s.into(),
			OwnedValue::Decision(d) => d.into(),
			OwnedValue::Constraint(c) => c.into(),
			OwnedValue::IntSet(ranges) => int_range_list_value(ranges),
			OwnedValue::FloatSet(ranges) => float_range_list_value(ranges),
			OwnedValue::List(items) => list_value(items),
		}
	}
}

impl From<Value<'_>> for OwnedValue {
	fn from(value: Value<'_>) -> Self {
		match value.view() {
			ValueView::Absent => Self::Absent,
			ValueView::Bool(b) => Self::Bool(b),
			ValueView::Constraint(c) => Self::Constraint(c),
			ValueView::Decision(d) => Self::Decision(d),
			ValueView::Float(f) => Self::Float(f),
			ValueView::FloatSet(ranges) => {
				Self::FloatSet(ranges.iter().map(|(min, max)| min..=max).collect())
			}
			ValueView::Int(i) => Self::Int(i),
			ValueView::IntSet(ranges) => {
				Self::IntSet(ranges.iter().map(|(min, max)| min..=max).collect())
			}
			ValueView::List(items) => Self::List(items.iter().map(Self::from).collect()),
			ValueView::Str(s) => Self::String(s.to_owned()),
		}
	}
}

impl LayeredModel {
	/// Add a constraint to the current (top) layer.
	///
	/// # Panics
	///
	/// Panics if there are no layers.
	pub fn add_constraint(
		&mut self,
		ident: impl Into<String>,
		arguments: Vec<OwnedValue>,
		defines: Option<DecisionIdx>,
		annotations: Vec<OwnedAnnotation>,
	) -> ConstraintIdx {
		assert!(!self.layers.is_empty(), "no layers in model");
		let global_idx = self.constraint_len();
		self.layers
			.last_mut()
			.unwrap()
			.constraints
			.push(ConstraintData {
				ident: ident.into(),
				arguments,
				defines,
				annotations,
			});
		FznsoConstraintIdx(global_idx)
	}

	/// Add a decision variable to the current (top) layer.
	///
	/// `ty` says what the variable is (see
	/// [`decision_type`](Model::decision_type)); `domain` says which values it
	/// may take, and may be [`OwnedValue::Absent`] for a variable with no
	/// explicit domain.
	///
	/// # Panics
	///
	/// Panics if there are no layers.
	pub fn add_decision(
		&mut self,
		ty: FznsoType,
		domain: OwnedValue,
		name: Option<String>,
		defined: bool,
		in_solution: bool,
		annotations: Vec<OwnedAnnotation>,
	) -> DecisionIdx {
		assert!(!self.layers.is_empty(), "no layers in model");
		let global_idx = self.decision_len();
		self.layers
			.last_mut()
			.unwrap()
			.decisions
			.push(DecisionData {
				ty,
				domain,
				name,
				defined,
				in_solution,
				annotations,
			});
		FznsoDecisionIdx(global_idx)
	}

	/// Resolve a global constraint index to a reference to its
	/// [`ConstraintData`].
	///
	/// # Panics
	///
	/// Panics if `idx` is out of range.
	fn constraint_data(&self, idx: ConstraintIdx) -> &ConstraintData {
		let mut remaining = idx.0;
		for layer in &self.layers {
			if remaining < layer.constraints.len() {
				return &layer.constraints[remaining];
			}
			remaining -= layer.constraints.len();
		}
		panic!("constraint index {} out of range", idx.0);
	}

	/// Resolve a global decision index to a reference to its [`DecisionData`]
	/// and its original global index as `usize`.
	///
	/// # Panics
	///
	/// Panics if `idx` is out of range.
	fn decision_location(&self, idx: DecisionIdx) -> (&DecisionData, usize) {
		let mut remaining = idx.0;
		for layer in &self.layers {
			if remaining < layer.decisions.len() {
				return (&layer.decisions[remaining], idx.0);
			}
			remaining -= layer.decisions.len();
		}
		panic!("decision index {} out of range", idx.0);
	}

	/// Mark all current layers as permanent.
	pub fn mark_permanent(&mut self) {
		self.permanent = self.layers.len();
	}

	/// Mark layers `0..n` as permanent.
	///
	/// # Panics
	///
	/// Panics if `n` is below the number of layers already permanent, or above
	/// the number of layers.
	pub fn mark_permanent_to(&mut self, n: usize) {
		assert!(
			(self.permanent..=self.layers.len()).contains(&n),
			"cannot mark {n} layers permanent: {} already are, and there are {}",
			self.permanent,
			self.layers.len()
		);
		self.permanent = n;
	}

	/// Mark a permanent layer as redundant.
	///
	/// `layer` must be a valid permanent layer index (0..permanent).
	pub fn mark_redundant(&mut self, layer: usize) {
		assert!(layer < self.permanent, "layer index out of range");
		if !self.redundant.contains(&layer) {
			self.redundant.push(layer);
		}
	}

	/// Pop the top layer from the model.
	///
	/// # Panics
	///
	/// Panics if there are no layers, or if the top layer is permanent.
	pub fn pop_layer(&mut self) {
		assert!(!self.layers.is_empty(), "no layers in model");
		self.truncate(self.layers.len() - 1);
	}

	/// Push a new (empty) layer onto the model.
	pub fn push_layer(&mut self) {
		self.layers.push(LayerData::default());
	}

	/// Set the model's objective.
	///
	/// Pass `None` for `ident` to clear the objective (satisfy-only).
	pub fn set_objective(
		&mut self,
		ident: Option<impl Into<String>>,
		arg: OwnedValue,
		annotations: Vec<OwnedAnnotation>,
	) {
		self.objective = ObjectiveData {
			ident: ident.map(Into::into).unwrap_or_default(),
			arg,
			annotations,
		};
	}

	/// Set the number of layers considered unchanged since the last solver run.
	pub fn set_unchanged(&mut self, n: usize) {
		self.unchanged = n;
	}

	/// Pop every layer from `n` upward, keeping layers `0..n`.
	///
	/// Does nothing if the model has `n` layers or fewer.
	///
	/// # Panics
	///
	/// Panics if a layer to pop is permanent, that is if `n` is less than
	/// the number of permanent layers.
	pub fn truncate(&mut self, n: usize) {
		assert!(n >= self.permanent, "cannot pop a permanent layer");
		self.layers.truncate(n);
		// A popped layer is not one the solver saw, so it cannot be unchanged.
		self.unchanged = self.unchanged.min(self.layers.len());
	}

	/// Create a model with no layers, and so no permanent layers.
	///
	/// Push a layer before adding anything. [`Default`] is the usual
	/// starting point; this is for a caller that must control which layers are
	/// permanent from the first one on.
	pub fn empty() -> Self {
		Self {
			layers: Vec::new(),
			objective: ObjectiveData::default(),
			permanent: 0,
			redundant: Vec::new(),
			unchanged: 0,
		}
	}
}

impl Default for LayeredModel {
	/// Create a model with one permanent base layer.
	fn default() -> Self {
		let mut model = Self::empty();
		model.push_layer();
		model.mark_permanent();
		model
	}
}

impl Model for LayeredModel {
	fn constraint_annotation(&self, con: ConstraintIdx, index: usize) -> AnnotationRef<'_> {
		self.constraint_data(con).annotations[index].as_fznso_ann_ref()
	}

	fn constraint_annotation_len(&self, con: ConstraintIdx) -> usize {
		self.constraint_data(con).annotations.len()
	}

	fn constraint_argument(&self, con: ConstraintIdx, index: usize) -> Value<'_> {
		FznsoValueRef::from(&self.constraint_data(con).arguments[index])
	}

	fn constraint_argument_len(&self, con: ConstraintIdx) -> usize {
		self.constraint_data(con).arguments.len()
	}

	fn constraint_defines(&self, con: ConstraintIdx) -> Option<DecisionIdx> {
		self.constraint_data(con).defines
	}

	fn constraint_ident(&self, con: ConstraintIdx) -> &str {
		&self.constraint_data(con).ident
	}

	fn constraint_layer_end(&self, layer: usize) -> usize {
		self.layers[..=layer]
			.iter()
			.map(|l| l.constraints.len())
			.sum()
	}

	fn constraint_len(&self) -> usize {
		self.layers.iter().map(|l| l.constraints.len()).sum()
	}

	fn decision_annotation(&self, decision: DecisionIdx, index: usize) -> AnnotationRef<'_> {
		let (data, _) = self.decision_location(decision);
		data.annotations[index].as_fznso_ann_ref()
	}

	fn decision_annotation_len(&self, decision: DecisionIdx) -> usize {
		let (data, _) = self.decision_location(decision);
		data.annotations.len()
	}

	fn decision_defined(&self, decision: DecisionIdx) -> bool {
		let (data, _) = self.decision_location(decision);
		data.defined
	}

	fn decision_in_solution(&self, decision: DecisionIdx) -> bool {
		let (data, _) = self.decision_location(decision);
		data.in_solution
	}

	fn decision_domain(&self, decision: DecisionIdx) -> Value<'_> {
		let (data, _) = self.decision_location(decision);
		FznsoValueRef::from(&data.domain)
	}

	fn decision_layer_end(&self, layer: usize) -> usize {
		self.layers[..=layer]
			.iter()
			.map(|l| l.decisions.len())
			.sum()
	}

	fn decision_len(&self) -> usize {
		self.layers.iter().map(|l| l.decisions.len()).sum()
	}

	fn decision_name(&self, decision: DecisionIdx) -> Option<&str> {
		let (data, _) = self.decision_location(decision);
		data.name.as_deref()
	}

	fn decision_type(&self, decision: DecisionIdx) -> FznsoType {
		let (data, _) = self.decision_location(decision);
		data.ty
	}

	fn layer_len(&self) -> usize {
		self.layers.len()
	}

	fn layer_permanent(&self) -> usize {
		self.permanent
	}

	fn layer_redundant_index(&self, index: usize) -> usize {
		self.redundant[index]
	}

	fn layer_redundant_len(&self) -> usize {
		self.redundant.len()
	}

	fn layer_unchanged(&self) -> usize {
		self.unchanged
	}

	fn objective_annotation(&self, index: usize) -> AnnotationRef<'_> {
		self.objective.annotations[index].as_fznso_ann_ref()
	}

	fn objective_annotation_len(&self) -> usize {
		self.objective.annotations.len()
	}

	fn objective_arg(&self) -> Value<'_> {
		FznsoValueRef::from(&self.objective.arg)
	}

	fn objective_ident(&self) -> &str {
		&self.objective.ident
	}
}

impl OwnedAnnotation {
	/// Create a new annotation with the given identifier (must be valid UTF-8
	/// and contain no interior null bytes) and arguments.
	pub fn new(ident: impl Into<String>, arguments: Vec<OwnedValue>) -> Self {
		Self {
			ident: ident.into(),
			arguments,
		}
	}
}

impl OwnedAnnotation {
	/// Build a [`FznsoAnnotationRef`] that borrows this annotation via a local
	/// vtable.  Used when returning annotations from the [`Model`] impl below.
	fn as_fznso_ann_ref(&self) -> FznsoAnnotationRef<'_> {
		extern "C" fn ident<'a>(ptr: FznsoPtr<'_, FznsoAnnotation>) -> FznsoStr<'a> {
			// SAFETY: ptr was created from &OwnedAnnotation via transmute
			// below.
			let ann: &OwnedAnnotation = unsafe { &*ptr.cast() };
			ann.ident.as_str().into()
		}
		extern "C" fn argument_len(ptr: FznsoPtr<'_, FznsoAnnotation>) -> usize {
			// SAFETY: ptr was created from &OwnedAnnotation via transmute
			// below.
			let ann: &OwnedAnnotation = unsafe { &*ptr.cast() };
			ann.arguments.len()
		}
		extern "C" fn argument<'a>(
			ptr: FznsoPtr<'a, FznsoAnnotation>,
			index: usize,
		) -> FznsoValueRef<'a> {
			// SAFETY: ptr was created from &OwnedAnnotation via transmute
			// below.
			let ann: &OwnedAnnotation = unsafe { &*ptr.cast() };
			ann.arguments
				.get(index)
				.map(FznsoValueRef::from)
				.unwrap_or_else(|| (&()).into())
		}
		const METHODS: &FznsoAnnotationMethods = &FznsoAnnotationMethods {
			ident,
			argument_len,
			argument,
		};
		FznsoAnnotationRef {
			data: FznsoPtr::from_ref(self),
			methods: METHODS,
		}
	}
}

impl Annotation for OwnedAnnotation {
	fn argument(&self, index: usize) -> Value<'_> {
		self.arguments
			.get(index)
			.map(FznsoValueRef::from)
			.unwrap_or_else(|| (&()).into())
	}

	fn argument_len(&self) -> usize {
		self.arguments.len()
	}

	fn ident(&self) -> &str {
		&self.ident
	}
}
