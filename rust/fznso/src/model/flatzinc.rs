use flatzinc_serde::{
	Annotation as FznAnnotation, AnnotationArgument, AnnotationLiteral, Argument, Array, Domain,
	FlatZinc, Literal, RangeList, Variable,
};
use fznso_types::{
	FznsoDecisionIdx, FznsoFloatRange, FznsoIntRange, FznsoPtr, FznsoValue, FznsoValueKind,
	FznsoValueMethods, FznsoValueRef,
};
use indexmap::IndexMap;

use crate::{
	AnnotationRef, ConstraintIdx, DecisionIdx, Value,
	model::{AnnWrapper, Annotation as OwnedAnnotation, Model},
};

/// A [`FlatZinc`] model with the concrete container types used by this adapter.
type FlatZincModel =
	FlatZinc<String, IndexMap<String, Variable<String>>, IndexMap<String, Array<String>>>;

/// Build a [`Value`] for an annotation argument.
fn annotation_argument_value(arg: &AnnotationArgument<String>) -> Value<'_> {
	match arg {
		AnnotationArgument::Literal(lit) => annotation_literal_value(lit),
		AnnotationArgument::Array(values) => annotation_literal_list_value(values),
	}
}

/// Build a [`Value`] for a list of annotation literals.
fn annotation_literal_list_value(values: &Vec<AnnotationLiteral<String>>) -> Value<'_> {
	extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
		FznsoValueKind::FznsoValueList
	}

	extern "C" fn len(val: FznsoPtr<'_, FznsoValue>) -> usize {
		// SAFETY: The data pointer stores `&Vec<AnnotationLiteral<String>>`.
		let val: &Vec<AnnotationLiteral<String>> = unsafe { &*val.cast() };
		val.len()
	}

	extern "C" fn get_element<'a>(
		val: FznsoPtr<'_, FznsoValue>,
		index: usize,
	) -> FznsoValueRef<'a> {
		// SAFETY: The data pointer stores `&Vec<AnnotationLiteral<String>>`.
		let val: &Vec<AnnotationLiteral<String>> = unsafe { &*val.cast() };
		annotation_literal_value(&val[index])
	}

	const METHODS: &FznsoValueMethods = &FznsoValueMethods {
		kind,
		len,
		get_element,
		..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
	};

	FznsoValueRef {
		// SAFETY: The value is a reference to `Vec<AnnotationLiteral<String>>`.
		data: FznsoPtr::from_ref(values),
		methods: METHODS,
	}
}

/// Build a [`Value`] for a single annotation literal.
fn annotation_literal_value(lit: &AnnotationLiteral<String>) -> Value<'_> {
	match lit {
		AnnotationLiteral::BaseLiteral(lit) => literal_value_unresolved(lit),
		AnnotationLiteral::Annotation(_) => (&()).into(),
	}
}

/// Build a [`Value`] for a constraint argument, resolving identifiers against
/// `model`.
fn argument_value<'a>(model: &'a FlatZincModel, arg: &'a Argument<String>) -> Value<'a> {
	match arg {
		Argument::Literal(lit) => literal_value(model, lit),
		Argument::Array(values) => literal_list_value(values),
	}
}

/// Resolve a variable identifier to its [`DecisionIdx`], if it names a
/// variable.
fn decision_index(model: &FlatZincModel, ident: &str) -> Option<DecisionIdx> {
	model.variables.get_index_of(ident).map(FznsoDecisionIdx)
}

/// Build a [`Value`] for a float set, represented as a range list.
fn float_set_value(ranges: &RangeList<f64>) -> Value<'_> {
	extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
		FznsoValueKind::FznsoValueSetFloat
	}

	extern "C" fn len(val: FznsoPtr<'_, FznsoValue>) -> usize {
		// SAFETY: The data pointer stores `&RangeList<f64>`.
		let val: &RangeList<f64> = unsafe { &*val.cast() };
		val.iter().count()
	}

	extern "C" fn get_range_float(val: FznsoPtr<'_, FznsoValue>, index: usize) -> FznsoFloatRange {
		// SAFETY: The data pointer stores `&RangeList<f64>`.
		let val: &RangeList<f64> = unsafe { &*val.cast() };
		let range = val.iter().nth(index).unwrap();
		FznsoFloatRange {
			min: *range.start(),
			max: *range.end(),
		}
	}

	const METHODS: &FznsoValueMethods = &FznsoValueMethods {
		kind,
		len,
		get_range_float,
		..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
	};

	FznsoValueRef {
		// SAFETY: The value is a reference to `RangeList<f64>`.
		data: FznsoPtr::from_ref(ranges),
		methods: METHODS,
	}
}

/// Build a [`Value`] for an integer set, represented as a range list.
fn int_set_value(ranges: &RangeList<i64>) -> Value<'_> {
	extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
		FznsoValueKind::FznsoValueSetInt
	}

	extern "C" fn len(val: FznsoPtr<'_, FznsoValue>) -> usize {
		// SAFETY: The data pointer stores `&RangeList<i64>`.
		let val: &RangeList<i64> = unsafe { &*val.cast() };
		val.iter().count()
	}

	extern "C" fn get_range_int(val: FznsoPtr<'_, FznsoValue>, index: usize) -> FznsoIntRange {
		// SAFETY: The data pointer stores `&RangeList<i64>`.
		let val: &RangeList<i64> = unsafe { &*val.cast() };
		let range = val.iter().nth(index).unwrap();
		FznsoIntRange {
			min: *range.start(),
			max: *range.end(),
		}
	}

	const METHODS: &FznsoValueMethods = &FznsoValueMethods {
		kind,
		len,
		get_range_int,
		..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
	};

	FznsoValueRef {
		// SAFETY: The value is a reference to `RangeList<i64>`.
		data: FznsoPtr::from_ref(ranges),
		methods: METHODS,
	}
}

/// Build a [`Value`] for a list of literals.
fn literal_list_value(values: &Vec<Literal<String>>) -> Value<'_> {
	extern "C" fn kind(_: FznsoPtr<'_, FznsoValue>) -> FznsoValueKind {
		FznsoValueKind::FznsoValueList
	}

	extern "C" fn len(val: FznsoPtr<'_, FznsoValue>) -> usize {
		// SAFETY: The data pointer stores `&Vec<Literal<String>>`.
		let val: &Vec<Literal<String>> = unsafe { &*val.cast() };
		val.len()
	}

	extern "C" fn get_element<'a>(
		val: FznsoPtr<'_, FznsoValue>,
		index: usize,
	) -> FznsoValueRef<'a> {
		// SAFETY: The data pointer stores `&Vec<Literal<String>>`.
		let val: &Vec<Literal<String>> = unsafe { &*val.cast() };
		literal_value_unresolved(&val[index])
	}

	const METHODS: &FznsoValueMethods = &FznsoValueMethods {
		kind,
		len,
		get_element,
		..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
	};

	FznsoValueRef {
		// SAFETY: The value is a reference to `Vec<Literal<String>>`.
		data: FznsoPtr::from_ref(values),
		methods: METHODS,
	}
}

/// Build a [`Value`] for a literal, resolving identifiers against `model`.
fn literal_value<'a>(model: &'a FlatZincModel, lit: &'a Literal<String>) -> Value<'a> {
	match lit {
		Literal::Int(i) => i.into(),
		Literal::Float(f) => f.into(),
		Literal::Bool(b) => b.into(),
		Literal::IntSet(set) => int_set_value(set),
		Literal::FloatSet(set) => float_set_value(set),
		Literal::Identifier(ident) => {
			if let Some(decision) = decision_index(model, ident) {
				decision.into()
			} else if let Some(array) = model.arrays.get(ident) {
				literal_list_value(&array.contents)
			} else {
				(&()).into()
			}
		}
		Literal::String(_) => (&()).into(),
	}
}

/// Build a [`Value`] for a literal without resolving identifiers.
fn literal_value_unresolved(lit: &Literal<String>) -> Value<'_> {
	match lit {
		Literal::Int(i) => i.into(),
		Literal::Float(f) => f.into(),
		Literal::Bool(b) => b.into(),
		Literal::IntSet(set) => int_set_value(set),
		Literal::FloatSet(set) => float_set_value(set),
		Literal::Identifier(_) | Literal::String(_) => (&()).into(),
	}
}

impl Model for FlatZincModel {
	fn constraint_annotation(&self, con: ConstraintIdx, index: usize) -> AnnotationRef<'_> {
		AnnWrapper::<FznAnnotation<String>>(&self.constraints[con.0].ann[index]).into()
	}

	fn constraint_annotation_len(&self, con: ConstraintIdx) -> usize {
		self.constraints.get(con.0).map_or(0, |con| con.ann.len())
	}

	fn constraint_argument(&self, con: ConstraintIdx, index: usize) -> Value<'_> {
		let Some(constraint) = self.constraints.get(con.0) else {
			return (&()).into();
		};
		let Some(arg) = constraint.args.get(index) else {
			return (&()).into();
		};
		argument_value(self, arg)
	}

	fn constraint_argument_len(&self, con: ConstraintIdx) -> usize {
		self.constraints
			.get(con.0)
			.map_or(0, |constraint| constraint.args.len())
	}

	fn constraint_defines(&self, con: ConstraintIdx) -> Option<DecisionIdx> {
		self.constraints
			.get(con.0)
			.and_then(|constraint| constraint.defines.as_ref())
			.and_then(|ident| decision_index(self, ident))
	}

	fn constraint_ident(&self, con: ConstraintIdx) -> &str {
		self.constraints
			.get(con.0)
			.map(|constraint| constraint.id.as_str())
			.unwrap_or("")
	}

	fn constraint_layer_end(&self, _: usize) -> usize {
		self.constraint_len()
	}

	fn constraint_len(&self) -> usize {
		self.constraints.len()
	}

	fn decision_annotation(&self, decision: DecisionIdx, index: usize) -> AnnotationRef<'_> {
		AnnWrapper::<FznAnnotation<String>>(&self.variables[decision.0].ann[index]).into()
	}

	fn decision_annotation_len(&self, decision: DecisionIdx) -> usize {
		self.variables[decision.0].ann.len()
	}

	fn decision_defined(&self, decision: DecisionIdx) -> bool {
		self.variables[decision.0].defined
	}

	fn decision_domain(&self, decision: DecisionIdx) -> Value<'_> {
		let var = &self.variables[decision.0];
		match &var.domain {
			Some(Domain::Int(set)) => int_set_value(set),
			Some(Domain::Float(set)) => float_set_value(set),
			None => (&()).into(),
		}
	}

	fn decision_layer_end(&self, _: usize) -> usize {
		self.decision_len()
	}

	fn decision_len(&self) -> usize {
		self.variables.len()
	}

	fn decision_name(&self, decision: DecisionIdx) -> Option<&str> {
		self.variables
			.get_index(decision.0)
			.map(|(k, _)| k.as_str())
	}

	fn layer_len(&self) -> usize {
		1
	}

	fn layer_permanent(&self) -> usize {
		1
	}

	fn layer_redundant_index(&self, _: usize) -> usize {
		0
	}

	fn layer_redundant_len(&self) -> usize {
		0
	}

	fn layer_unchanged(&self) -> usize {
		0
	}

	fn objective_annotation(&self, index: usize) -> AnnotationRef<'_> {
		AnnWrapper::<FznAnnotation<String>>(&self.solve.ann[index]).into()
	}

	fn objective_annotation_len(&self) -> usize {
		self.solve.ann.len()
	}

	fn objective_arg(&self) -> Value<'_> {
		self.solve
			.objective
			.as_ref()
			.map_or_else(|| (&()).into(), |obj| literal_value(self, obj))
	}

	fn objective_ident(&self) -> &str {
		match self.solve.method {
			flatzinc_serde::Method::Satisfy => "",
			flatzinc_serde::Method::Minimize => "minimize",
			flatzinc_serde::Method::Maximize => "maximize",
		}
	}
}

impl OwnedAnnotation for FznAnnotation<String> {
	fn argument(&self, index: usize) -> Value<'_> {
		match self {
			FznAnnotation::Atom(_) => (&()).into(),
			FznAnnotation::Call(call) => call
				.args
				.get(index)
				.map_or_else(|| (&()).into(), annotation_argument_value),
		}
	}

	fn argument_len(&self) -> usize {
		match self {
			FznAnnotation::Atom(_) => 0,
			FznAnnotation::Call(call) => call.args.len(),
		}
	}

	fn ident(&self) -> &str {
		match self {
			FznAnnotation::Atom(ident) => ident.as_str(),
			FznAnnotation::Call(call) => call.id.as_str(),
		}
	}
}
