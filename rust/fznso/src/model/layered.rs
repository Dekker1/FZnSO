//! [`LayeredModel`]: an in-memory, multi-layer [`Model`] implementation for testing and modelling.

use fznso_types::{
	FznsoAnnotation, FznsoAnnotationMethods, FznsoAnnotationRef, FznsoConstraintIdx,
	FznsoDecisionIdx, FznsoFloatRange, FznsoIntRange, FznsoPtr, FznsoStr, FznsoValue,
	FznsoValueKind, FznsoValueMethods, FznsoValueRef,
};
use rangelist::RangeList;

use crate::{
	AnnotationRef, ConstraintIdx, DecisionIdx, Value,
	model::{Annotation, Model},
};

/// Stores the data for a single constraint within a layer.
#[derive(Clone, Debug, Default)]
struct ConstraintData {
	/// The constraint identifier (e.g. `"int_le"`).
	ident: String,
	/// The constraint's argument values.
	arguments: Vec<ModelValue>,
	/// Decision variable defined by this constraint, if any.
	defines: Option<DecisionIdx>,
	/// Annotations attached to this constraint.
	annotations: Vec<ModelAnnotation>,
}

/// Stores the data for a single decision variable within a layer.
#[derive(Clone, Debug, Default)]
struct DecisionData {
	/// The domain of the decision variable.
	domain: ModelValue,
	/// Optional name of the decision variable.
	name: Option<String>,
	/// Whether this variable is defined by some constraint.
	defined: bool,
	/// Annotations attached to this decision variable.
	annotations: Vec<ModelAnnotation>,
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
/// use fznso::{LayeredModel, ModelValue};
/// use rangelist::RangeList;
///
/// let mut m = LayeredModel::default();
/// let d = m.add_decision(
///     ModelValue::IntSet(RangeList::from(-10i64..=10)),
///     Some("x".into()),
///     false,
///     vec![],
/// );
/// m.add_constraint("int_le", vec![ModelValue::Int(-10), ModelValue::Decision(d)], None, vec![]);
/// m.set_objective(Some("minimize"), ModelValue::Decision(d), vec![]);
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

/// An annotation for use with [`SimpleModel`].
#[derive(Clone, Debug)]
pub struct ModelAnnotation {
	/// The annotation identifier (e.g. `"output_var"`).
	ident: String,
	/// The annotation's argument values.
	arguments: Vec<ModelValue>,
}

/// An owned value for use with [`SimpleModel`].
///
/// Mirrors all variants of [`FznsoValueKind`] but owns the contained data.
#[derive(Clone, Debug, Default)]
pub enum ModelValue {
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
	IntSet(RangeList<i64>),
	/// Float set as a list of inclusive `[min, max]` ranges.
	FloatSet(RangeList<f64>),
	/// An ordered list of nested values.
	List(Vec<ModelValue>),
}

/// The model's single global objective.
#[derive(Clone, Debug, Default)]
struct ObjectiveData {
	/// Objective identifier (e.g. `"minimize"`), empty for satisfaction.
	ident: String,
	/// The objective argument.
	arg: ModelValue,
	/// Annotations on the objective.
	annotations: Vec<ModelAnnotation>,
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

/// Build a [`FznsoValueRef`] for a list of [`ModelValue`] items.
fn list_value(items: &Vec<ModelValue>) -> FznsoValueRef<'_> {
	extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
		FznsoValueKind::FznsoValueList
	}
	extern "C" fn len(val: FznsoPtr<'_, FznsoValue>) -> usize {
		// SAFETY: val was created from &Vec<ModelValue> via transmute in list_value.
		let val: &Vec<ModelValue> = unsafe { &*val.cast() };
		val.len()
	}
	extern "C" fn get_element<'a>(
		val: FznsoPtr<'_, FznsoValue>,
		index: usize,
	) -> FznsoValueRef<'a> {
		// SAFETY: val was created from &Vec<ModelValue> via transmute in list_value.
		let val: &Vec<ModelValue> = unsafe { &*val.cast() };
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

impl<'a> From<&'a ModelValue> for FznsoValueRef<'a> {
	fn from(val: &'a ModelValue) -> Self {
		match val {
			ModelValue::Absent => (&()).into(),
			ModelValue::Bool(b) => b.into(),
			ModelValue::Int(i) => i.into(),
			ModelValue::Float(f) => f.into(),
			ModelValue::String(s) => s.into(),
			ModelValue::Decision(d) => d.into(),
			ModelValue::Constraint(c) => c.into(),
			ModelValue::IntSet(ranges) => int_range_list_value(ranges),
			ModelValue::FloatSet(ranges) => float_range_list_value(ranges),
			ModelValue::List(items) => list_value(items),
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
		arguments: Vec<ModelValue>,
		defines: Option<DecisionIdx>,
		annotations: Vec<ModelAnnotation>,
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
	/// # Panics
	///
	/// Panics if there are no layers.
	pub fn add_decision(
		&mut self,
		domain: ModelValue,
		name: Option<String>,
		defined: bool,
		annotations: Vec<ModelAnnotation>,
	) -> DecisionIdx {
		assert!(!self.layers.is_empty(), "no layers in model");
		let global_idx = self.decision_len();
		self.layers
			.last_mut()
			.unwrap()
			.decisions
			.push(DecisionData {
				domain,
				name,
				defined,
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
		assert!(
			self.layers.len() > self.permanent,
			"cannot pop a permanent layer"
		);
		let _ = self.layers.pop();
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
		arg: ModelValue,
		annotations: Vec<ModelAnnotation>,
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
}

impl Default for LayeredModel {
	/// Create a model with one permanent base layer.
	fn default() -> Self {
		Self {
			layers: vec![LayerData::default()],
			objective: ObjectiveData::default(),
			permanent: 1,
			redundant: Vec::new(),
			unchanged: 0,
		}
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

impl ModelAnnotation {
	/// Create a new annotation with the given identifier (must be valid UTF-8
	/// and contain no interior null bytes) and arguments.
	pub fn new(ident: impl Into<String>, arguments: Vec<ModelValue>) -> Self {
		Self {
			ident: ident.into(),
			arguments,
		}
	}
}

impl ModelAnnotation {
	/// Build a [`FznsoAnnotationRef`] that borrows this annotation via a local
	/// vtable.  Used when returning annotations from the [`Model`] impl below.
	fn as_fznso_ann_ref(&self) -> FznsoAnnotationRef<'_> {
		extern "C" fn ident<'a>(ptr: FznsoPtr<'_, FznsoAnnotation>) -> FznsoStr<'a> {
			// SAFETY: ptr was created from &ModelAnnotation via transmute below.
			let ann: &ModelAnnotation = unsafe { &*ptr.cast() };
			ann.ident.as_str().into()
		}
		extern "C" fn argument_len(ptr: FznsoPtr<'_, FznsoAnnotation>) -> usize {
			// SAFETY: ptr was created from &ModelAnnotation via transmute below.
			let ann: &ModelAnnotation = unsafe { &*ptr.cast() };
			ann.arguments.len()
		}
		extern "C" fn argument<'a>(
			ptr: FznsoPtr<'a, FznsoAnnotation>,
			index: usize,
		) -> FznsoValueRef<'a> {
			// SAFETY: ptr was created from &ModelAnnotation via transmute below.
			let ann: &ModelAnnotation = unsafe { &*ptr.cast() };
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

impl Annotation for ModelAnnotation {
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
