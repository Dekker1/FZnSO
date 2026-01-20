use std::{ffi, mem};

use fznso_types::{
	FznsoAnnotation, FznsoAnnotationMethods, FznsoAnnotationRef, FznsoConstraintIdx,
	FznsoDecisionIdx, FznsoModelMethods, FznsoModelRef, FznsoValue, FznsoValueKind,
	FznsoValueMethods, FznsoValueRef,
};

use crate::{ConstraintIdx, DecisionIdx, Value};

pub trait Annotation {
	fn ident(&self) -> &str;
	fn argument(&self, index: usize) -> Value<'_>;
	fn argument_len(&self) -> usize;
}

struct AnnWrapper<'a, A: Annotation>(&'a A);

/// Methods provided to a [`Solver`] to interact with model instances provided
/// to be solved.
pub trait Model {
	type Ann: Annotation;

	fn layer_len(&self) -> usize;
	fn layer_unchanged(&self) -> usize;
	fn layer_permanent(&self) -> usize;
	fn layer_redundant_len(&self) -> usize;
	fn layer_redundant_index(&self, index: usize) -> usize;

	fn decision_len(&self) -> usize;
	fn decision_layer_end(&self, layer: usize) -> usize;
	fn decision_domain(&self, decision: DecisionIdx) -> Value<'_>;
	fn decision_name(&self, decision: DecisionIdx) -> &str;
	fn decision_defined(&self, decision: DecisionIdx) -> bool;
	fn decision_annotation_len(&self, decision: DecisionIdx) -> usize;
	fn decision_annotation(&self, decision: DecisionIdx, index: usize) -> &Self::Ann;

	fn constraint_len(&self) -> usize;
	fn constraint_layer_end(&self, layer: usize) -> usize;
	fn constraint_ident(&self, con: ConstraintIdx) -> &str;
	fn constraint_argument_len(&self, con: ConstraintIdx) -> usize;
	fn constraint_argument(&self, con: ConstraintIdx, index: usize) -> Value<'_>;
	fn constraint_defines(&self, con: ConstraintIdx) -> Option<DecisionIdx>;
	fn constraint_annotation_len(&self, con: ConstraintIdx) -> usize;
	fn constraint_annotation(&self, con: ConstraintIdx, index: usize) -> &Self::Ann;

	fn objective_ident(&self) -> &str;
	fn objective_arg(&self) -> Value<'_>;
	fn objective_annotation_len(&self) -> usize;
	fn objective_annotation(&self, index: usize) -> &Self::Ann;
}

pub(crate) struct ModelWrapper<'a, M: Model>(pub(crate) &'a M);

extern "C" fn wrap_ann_ident<A: Annotation>(ptr: &FznsoAnnotation) -> *const ffi::c_char {
	let ann: &A = unsafe { mem::transmute(ptr) };
	ann.ident().as_bytes().as_ptr() as *const ffi::c_char
}

extern "C" fn wrap_ann_argument_len<A: Annotation>(ptr: &FznsoAnnotation) -> usize {
	let ann: &A = unsafe { mem::transmute(ptr) };
	ann.argument_len()
}

extern "C" fn wrap_ann_argument<A: Annotation + 'static>(
	ptr: &FznsoAnnotation,
	index: usize,
) -> FznsoValueRef<'_> {
	let ann: &A = unsafe { mem::transmute(ptr) };
	ann.argument(index)
}

extern "C" fn wrap_layer_len<M: Model>(ptr: &fznso_types::FznsoModel) -> usize {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.layer_len()
}

extern "C" fn wrap_layer_unchanged<M: Model>(ptr: &fznso_types::FznsoModel) -> usize {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.layer_unchanged()
}

extern "C" fn wrap_layer_permanent<M: Model>(ptr: &fznso_types::FznsoModel) -> usize {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.layer_permanent()
}

extern "C" fn wrap_layer_redundant_len<M: Model>(ptr: &fznso_types::FznsoModel) -> usize {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.layer_redundant_len()
}

extern "C" fn wrap_layer_redundant_index<M: Model>(
	ptr: &fznso_types::FznsoModel,
	index: usize,
) -> usize {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.layer_redundant_index(index)
}

extern "C" fn wrap_decision_len<M: Model>(ptr: &fznso_types::FznsoModel) -> usize {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.decision_len()
}

extern "C" fn wrap_decision_layer_end<M: Model>(
	ptr: &fznso_types::FznsoModel,
	layer: usize,
) -> usize {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.decision_layer_end(layer)
}

extern "C" fn wrap_decision_domain<M: Model + 'static>(
	ptr: &fznso_types::FznsoModel,
	index: FznsoDecisionIdx,
) -> FznsoValueRef<'_> {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.decision_domain(index)
}

extern "C" fn wrap_decision_name<M: Model>(
	ptr: &fznso_types::FznsoModel,
	index: FznsoDecisionIdx,
) -> *const ffi::c_char {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.decision_name(index).as_bytes().as_ptr() as *const ffi::c_char
}

extern "C" fn wrap_decision_defined<M: Model>(
	ptr: &fznso_types::FznsoModel,
	index: FznsoDecisionIdx,
) -> bool {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.decision_defined(index)
}

extern "C" fn wrap_decision_annotation_len<M: Model>(
	ptr: &fznso_types::FznsoModel,
	index: FznsoDecisionIdx,
) -> usize {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.decision_annotation_len(index)
}

extern "C" fn wrap_decision_annotation<M: Model + 'static>(
	ptr: &fznso_types::FznsoModel,
	decision: FznsoDecisionIdx,
	index1: usize,
) -> FznsoAnnotationRef<'_> {
	let model: &M = unsafe { mem::transmute(ptr) };
	let ann = model.decision_annotation(decision, index1);
	AnnWrapper::<M::Ann>(ann).into()
}

extern "C" fn wrap_constraint_len<M: Model>(ptr: &fznso_types::FznsoModel) -> usize {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.constraint_len()
}

extern "C" fn wrap_constraint_layer_end<M: Model>(
	ptr: &fznso_types::FznsoModel,
	layer: usize,
) -> usize {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.constraint_layer_end(layer)
}

extern "C" fn wrap_constraint_ident<M: Model>(
	ptr: &fznso_types::FznsoModel,
	index: FznsoConstraintIdx,
) -> *const ffi::c_char {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.constraint_ident(index).as_bytes().as_ptr() as *const ffi::c_char
}

extern "C" fn wrap_constraint_argument_len<M: Model>(
	ptr: &fznso_types::FznsoModel,
	index: FznsoConstraintIdx,
) -> usize {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.constraint_argument_len(index)
}

extern "C" fn wrap_constraint_argument<M: Model + 'static>(
	ptr: &fznso_types::FznsoModel,
	con: FznsoConstraintIdx,
	index: usize,
) -> FznsoValueRef<'_> {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.constraint_argument(con, index)
}

extern "C" fn wrap_constraint_defines<M: Model>(
	ptr: &fznso_types::FznsoModel,
	index: FznsoConstraintIdx,
) -> FznsoValueRef<'_> {
	extern "C" fn kind(_: &FznsoValue) -> FznsoValueKind {
		FznsoValueKind::FznsoValueDecision
	}
	extern "C" fn as_decision(val: &FznsoValue) -> FznsoDecisionIdx {
		// SAFETY: The value is a usize, but was transmuted to a &'a FznsoValue
		// to fit [`FznsoValueRef`].
		let val: usize = unsafe { mem::transmute(val) };
		FznsoDecisionIdx(val)
	}
	const METHODS: &FznsoValueMethods = &FznsoValueMethods {
		kind,
		as_decision,
		..*FznsoValueRef::FZNSO_DEFAULT_VALUE_METHODS
	};
	let model: &M = unsafe { mem::transmute(ptr) };
	let opt = model.constraint_defines(index);
	match opt {
		Some(def) => FznsoValueRef::<'static> {
			data: unsafe { mem::transmute(def.0) },
			methods: METHODS,
		},
		None => (&()).into(),
	}
}

extern "C" fn wrap_constraint_annotation_len<M: Model>(
	ptr: &fznso_types::FznsoModel,
	index: FznsoConstraintIdx,
) -> usize {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.constraint_annotation_len(index)
}

extern "C" fn wrap_constraint_annotation<M: Model + 'static>(
	ptr: &fznso_types::FznsoModel,
	con: FznsoConstraintIdx,
	index: usize,
) -> FznsoAnnotationRef<'_> {
	let model: &M = unsafe { mem::transmute(ptr) };
	let ann = model.constraint_annotation(con, index);
	AnnWrapper::<M::Ann>(ann).into()
}

extern "C" fn wrap_objective_ident<M: Model>(ptr: &fznso_types::FznsoModel) -> *const ffi::c_char {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.objective_ident().as_bytes().as_ptr() as *const ffi::c_char
}

extern "C" fn wrap_objective_arg<M: Model + 'static>(
	ptr: &fznso_types::FznsoModel,
) -> FznsoValueRef<'_> {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.objective_arg()
}

extern "C" fn wrap_objective_annotation_len<M: Model>(ptr: &fznso_types::FznsoModel) -> usize {
	let model: &M = unsafe { mem::transmute(ptr) };
	model.objective_annotation_len()
}

extern "C" fn wrap_objective_annotation<M: Model + 'static>(
	ptr: &fznso_types::FznsoModel,
	index: usize,
) -> FznsoAnnotationRef<'_> {
	let model: &M = unsafe { mem::transmute(ptr) };
	let ann = model.objective_annotation(index);
	AnnWrapper::<M::Ann>(ann).into()
}

impl<'a, A: Annotation + 'static> AnnWrapper<'a, A> {
	const METHODS: &'static FznsoAnnotationMethods = &FznsoAnnotationMethods {
		ident: wrap_ann_ident::<A>,
		argument_len: wrap_ann_argument_len::<A>,
		argument: wrap_ann_argument::<A>,
	};
}

impl<'a, A: Annotation + 'static> From<AnnWrapper<'a, A>> for FznsoAnnotationRef<'a> {
	fn from(value: AnnWrapper<'a, A>) -> Self {
		FznsoAnnotationRef {
			methods: AnnWrapper::<'a, A>::METHODS,
			data: unsafe { mem::transmute(value.0) },
		}
	}
}

impl<'a, M: Model + 'static> ModelWrapper<'a, M> {
	const METHODS: &'static FznsoModelMethods = &FznsoModelMethods {
		layer_len: wrap_layer_len::<M>,
		layer_unchanged: wrap_layer_unchanged::<M>,
		layer_permanent: wrap_layer_permanent::<M>,
		layer_redundant_len: wrap_layer_redundant_len::<M>,
		layer_redundant_index: wrap_layer_redundant_index::<M>,
		decision_len: wrap_decision_len::<M>,
		decision_layer_end: wrap_decision_layer_end::<M>,
		decision_domain: wrap_decision_domain::<M>,
		decision_name: wrap_decision_name::<M>,
		decision_defined: wrap_decision_defined::<M>,
		decision_annotation_len: wrap_decision_annotation_len::<M>,
		decision_annotation: wrap_decision_annotation::<M>,
		constraint_len: wrap_constraint_len::<M>,
		constraint_layer_end: wrap_constraint_layer_end::<M>,
		constraint_ident: wrap_constraint_ident::<M>,
		constraint_argument_len: wrap_constraint_argument_len::<M>,
		constraint_argument: wrap_constraint_argument::<M>,
		constraint_defines: wrap_constraint_defines::<M>,
		constraint_annotation_len: wrap_constraint_annotation_len::<M>,
		constraint_annotation: wrap_constraint_annotation::<M>,
		objective_ident: wrap_objective_ident::<M>,
		objective_arg: wrap_objective_arg::<M>,
		objective_annotation_len: wrap_objective_annotation_len::<M>,
		objective_annotation: wrap_objective_annotation::<M>,
	};
}

impl<'a, M: Model + 'static> From<ModelWrapper<'a, M>> for FznsoModelRef<'a> {
	fn from(value: ModelWrapper<'a, M>) -> Self {
		FznsoModelRef {
			methods: ModelWrapper::<'a, M>::METHODS,
			data: unsafe { mem::transmute(value.0) },
		}
	}
}
