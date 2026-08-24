use std::sync::Arc;

use flatzinc_serde::{
	Annotation as FznAnnotation, AnnotationArgument, AnnotationLiteral, Argument, FlatZinc,
	Literal, Method, NamedRef, RangeList, Type,
};
use fznso_types::{
	FznsoDecisionIdx, FznsoFloatRange, FznsoIntRange, FznsoPtr, FznsoType, FznsoTypeBase,
	FznsoValue, FznsoValueKind, FznsoValueMethods, FznsoValueRef,
};

use crate::{
	AnnotationRef, ConstraintIdx, DecisionIdx, Value,
	model::{AnnWrapper, Annotation as OwnedAnnotation, Model},
};

/// The concrete [`FlatZinc`] model this adapter implements [`Model`] for.
type FlatZincModel = FlatZinc<String>;

/// Build a [`Value`] for an annotation argument.
fn annotation_argument_value(arg: &AnnotationArgument<String>) -> Value<'_> {
	match arg {
		AnnotationArgument::Literal(lit) => annotation_literal_value(lit),
		AnnotationArgument::Array(values) => annotation_literal_list_value(values),
		// A named-array reference cannot be borrowed for the value's lifetime
		// from here, so it is reported as absent.
		AnnotationArgument::ArrayNamed(_) => (&()).into(),
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
		AnnotationLiteral::Int(i) => i.into(),
		AnnotationLiteral::Float(f) => f.into(),
		AnnotationLiteral::Bool(b) => b.into(),
		AnnotationLiteral::IntSet(set) => int_set_value(set),
		AnnotationLiteral::FloatSet(set) => float_set_value(set),
		// A variable reference cannot be resolved to an index without the model,
		// and strings and nested annotations have no value representation.
		AnnotationLiteral::Variable(_)
		| AnnotationLiteral::String(_)
		| AnnotationLiteral::Annotation(_) => (&()).into(),
	}
}

/// Build a [`Value`] for a constraint argument, resolving identifiers against
/// `model`.
fn argument_value<'a>(model: &'a FlatZincModel, arg: &'a Argument<String>) -> Value<'a> {
	match arg {
		Argument::Literal(lit) => literal_value(model, lit),
		Argument::Array(values) => literal_list_value(values),
		Argument::ArrayNamed(array) => literal_list_value(&array.contents),
	}
}

/// Resolve a variable name to its [`DecisionIdx`], if it names a variable.
fn decision_index(model: &FlatZincModel, name: &str) -> Option<DecisionIdx> {
	model
		.variables
		.iter()
		.position(|var| var.name == name)
		.map(FznsoDecisionIdx)
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

/// Build a [`Value`] for a list of literals, without resolving identifiers.
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

/// Build a [`Value`] for a literal, resolving variable references against
/// `model`.
fn literal_value<'a>(model: &'a FlatZincModel, lit: &'a Literal<String>) -> Value<'a> {
	match lit {
		Literal::Int(i) => i.into(),
		Literal::Float(f) => f.into(),
		Literal::Bool(b) => b.into(),
		Literal::IntSet(set) => int_set_value(set),
		Literal::FloatSet(set) => float_set_value(set),
		Literal::Variable(var) => {
			decision_index(model, &var.name).map_or_else(|| (&()).into(), Into::into)
		}
		Literal::String(_) => (&()).into(),
	}
}

/// Build a [`Value`] for a literal without resolving variable references.
fn literal_value_unresolved(lit: &Literal<String>) -> Value<'_> {
	match lit {
		Literal::Int(i) => i.into(),
		Literal::Float(f) => f.into(),
		Literal::Bool(b) => b.into(),
		Literal::IntSet(set) => int_set_value(set),
		Literal::FloatSet(set) => float_set_value(set),
		Literal::Variable(_) | Literal::String(_) => (&()).into(),
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
			.and_then(|named| decision_index(self, named.name()))
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

	fn decision_in_solution(&self, decision: DecisionIdx) -> bool {
		// What the instance says has to be reported, which is not the same as
		// what the compiler did not introduce: the elements of an output array
		// are usually introduced variables, and a variable can be declared in
		// the model and still be left out of the output.
		//
		// Linear in the size of the output section, and a solver asks once per
		// decision. A consumer handing over a large instance repeatedly is
		// better served by a model of its own that answers from a precomputed
		// set.
		let var = &self.variables[decision.0];
		self.output.iter().any(|entry| match entry {
			NamedRef::Variable(v) => Arc::ptr_eq(v, var),
			NamedRef::Array(a) => a.contents.iter().any(|lit| match lit {
				Literal::Variable(v) => Arc::ptr_eq(v, var),
				_ => false,
			}),
		})
	}

	fn decision_domain(&self, decision: DecisionIdx) -> Value<'_> {
		match &self.variables[decision.0].ty {
			Type::Int(Some(set)) | Type::IntSet(Some(set)) => int_set_value(set),
			Type::Float(Some(set)) => float_set_value(set),
			Type::Bool | Type::Int(None) | Type::Float(None) | Type::IntSet(None) => (&()).into(),
		}
	}

	fn decision_type(&self, decision: DecisionIdx) -> FznsoType {
		match &self.variables[decision.0].ty {
			Type::Bool => FznsoType::new(FznsoTypeBase::FznsoTypeBaseBool).decision(true),
			Type::Int(_) => FznsoType::new(FznsoTypeBase::FznsoTypeBaseInt).decision(true),
			Type::Float(_) => FznsoType::new(FznsoTypeBase::FznsoTypeBaseFloat).decision(true),
			Type::IntSet(_) => FznsoType::new(FznsoTypeBase::FznsoTypeBaseInt)
				.decision(true)
				.set(true),
		}
	}

	fn decision_layer_end(&self, _: usize) -> usize {
		self.decision_len()
	}

	fn decision_len(&self) -> usize {
		self.variables.len()
	}

	fn decision_name(&self, decision: DecisionIdx) -> Option<&str> {
		self.variables.get(decision.0).map(|var| var.name.as_str())
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
		match &self.solve.method {
			Method::Minimize(objective) | Method::Maximize(objective) => {
				literal_value(self, objective)
			}
			Method::Satisfy => (&()).into(),
		}
	}

	fn objective_ident(&self) -> &str {
		match &self.solve.method {
			Method::Satisfy => "",
			Method::Minimize(_) => "minimize",
			Method::Maximize(_) => "maximize",
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
