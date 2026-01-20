//! FZnSO Protocol Types
//!
//! This crate defines the types of the FZnSO protocol, which allows the
//! incremental usage of solvers that can solve decision and optimization
//! problems. The goal of the interface is to easily use different solvers in a
//! unified way, and to allow the dynamic loading of solver libraries (as DLLs).

use std::{ffi, marker::PhantomData};

mod rust_interaction;

#[repr(transparent)]
#[derive(Debug)]
/// The handle to a the data of a solution instance.
///
/// This type is opaque to the user. Only pointers of this type are ever used.
///
/// In implementation of the FZnSO interface, a pointer is generally cast to
/// this type, i.e. `(FznsoSolutionData*) my_solution`. A similar cast can be
/// used to cast the pointer back to the original type, e.g. `(MySolution*)
/// fznso_solution`.
pub struct FznsoAnnotation(pub ffi::c_void);

#[repr(C)]
#[derive(Debug)]
/// A struct containing the function pointers to interact with the
/// [`FznsoModel`]
pub struct FznsoAnnotationMethods {
	/// Retrieve the identifier of an annotation.
	///
	/// The returned pointer can be assumed to have the the same lifetime as the
	/// annotation reference and must be valid UTF8.
	pub ident: extern "C" fn(ann: &FznsoAnnotation) -> *const ffi::c_char,
	/// Retrieve the number of arguments of an annotation.
	pub argument_len: extern "C" fn(ann: &FznsoAnnotation) -> usize,
	/// Retrieve the value of the annotation's argument at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `annotation_argument_len` for the given annotation.
	pub argument: extern "C" fn(ann: &FznsoAnnotation, index: usize) -> FznsoValueRef<'_>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// Handle for an annotation.
///
/// The caller can use the `methods` virtual table to interact with the
/// annotation.
pub struct FznsoAnnotationRef<'a> {
	/// The data pointer of the annotation.
	pub data: &'a FznsoAnnotation,
	/// Reference to the structure containing the function callbacks used to
	/// interact with the annotation.
	pub methods: &'static FznsoAnnotationMethods,
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Wrapper type for the indexes that represent constraints in the model.
pub struct FznsoConstraintIdx(pub usize);

#[repr(C)]
#[derive(Clone, Debug)]
/// A list of [`FznsoConstraintType`]s.
///
/// This type is for example used to return from [`fznso_constraint_list`].
pub struct FznsoConstraintList<'a> {
	/// The number of elements in the `constraints` array.
	pub len: usize,
	/// An array of constraint types.
	pub constraints: *const FznsoConstraintType<'a>,
	/// The lifetime for which the `constraints` attribute is allocated.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// Representation of a type of constraint, discerned by its identifier and the
/// types of its arguments.
pub struct FznsoConstraintType<'a> {
	/// The identifier of the constraint type.
	pub ident: *const ffi::c_char,
	/// The number of expected arguments for the constraint type.
	pub arg_len: usize,
	/// The types of the expected arguments for the constraint type.
	pub arg_types: *const FznsoType,
	/// The lifetime for which the `ident` and `arg_types` attributes are
	/// allocated.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Wrapper type for the indexes that represent decision variables in the model.
pub struct FznsoDecisionIdx(pub usize);

#[repr(transparent)]
#[derive(Debug)]
/// The handle of a model instance.
///
/// This type is opaque to the user. Only pointers of this type are ever used.
///
/// In implementation of the FZnSO interface, a pointer is generally cast to
/// this type, i.e. `(FznsoModelData*) my_model`. A similar cast can be used to
/// cast the pointer back to the original type, e.g. `(MyModel*) fznso_model`.
pub struct FznsoModel(pub ffi::c_void);

#[repr(C)]
#[derive(Debug)]
/// A struct containing the function pointers to interact with the
/// [`FznsoModel`]
pub struct FznsoModelMethods {
	/// Returns the current number of model layers currently contained in the
	/// model.
	///
	/// Layers provides a way for the modelling user to add and retract
	pub layer_len: extern "C" fn(model: &FznsoModel) -> usize,
	/// Returns the number of layers that have been unchanged since the last
	/// call to [`fznso_solver_run`].
	///
	/// Unchanged layers must be consecutive starting from layer 0.
	pub layer_unchanged: extern "C" fn(model: &FznsoModel) -> usize,
	/// Returns the number of permanent layers in the model.
	///
	/// Permanent layers are laid out must be consecutive starting from layer 0.
	/// Under no circumstances can they be retracted. As such, the solver can
	/// assume that the number of permanent layers only ever increases.
	///
	/// Permanent layers can, however, still be found to be redundant, allowing
	/// the solver to remove the decision variables and constraints in these
	/// layers, if convenient. This is signaled using [`layer_redundant_len`]
	/// and [`layer_redundant_index`].
	pub layer_permanent: extern "C" fn(model: &FznsoModel) -> usize,
	/// Returns the number of permanent layers that have been marked as
	/// redundant.
	///
	/// The solver can remove the decision variables and constraints in these
	/// layers, if convenient. Once a layer is marked as redundant, it can
	/// forever be considered redundant. As such, the solver can assume that
	/// the number of redundant layers only ever increases.
	pub layer_redundant_len: extern "C" fn(model: &FznsoModel) -> usize,
	/// Returns the index of the n-th permanent layer that has been marked as
	/// redundant.
	///
	/// The `n` argument must be less than the value returned by
	/// [`layer_redundant_len`].
	pub layer_redundant_index: extern "C" fn(model: &FznsoModel, n: usize) -> usize,

	/// Retrieve the number of decisions currently contained in the model.
	pub decision_len: extern "C" fn(model: &FznsoModel) -> usize,
	/// Retrieve the decision index of the last decision variable in the layer.
	///
	/// This can be equivalent to `decision_layer_end` of `layer-1` if the layer
	/// does not add any new decisions.
	pub decision_layer_end: extern "C" fn(model: &FznsoModel, layer: usize) -> usize,
	/// Retrieve the domain of the given decision.
	///
	/// Note that the the value might have [`FznsoValueKind::FznsoValueAbsent`]
	/// if the decision variable does not have an explicit domain.
	pub decision_domain:
		extern "C" fn(model: &FznsoModel, decision: FznsoDecisionIdx) -> FznsoValueRef<'_>,
	/// Retrieve the name of a decision variable, if it exists.
	///
	/// Note that names are only available for debugging purposes. Decisions are
	/// identified using their index in the model. If the decision variable does
	/// not have a name, this function returns a null pointer.
	pub decision_name:
		extern "C" fn(model: &FznsoModel, decision: FznsoDecisionIdx) -> *const ffi::c_char,
	/// Check whether the decision variable is defined by a constraint.
	pub decision_defined: extern "C" fn(model: &FznsoModel, decision: FznsoDecisionIdx) -> bool,
	/// Retrieve the number of annotations of a constraint.
	pub decision_annotation_len:
		extern "C" fn(model: &FznsoModel, decision: FznsoDecisionIdx) -> usize,
	/// Retrieve the value of the constraint's annotation at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// [`decision_annotation_len`] for the given decision variable.
	pub decision_annotation: extern "C" fn(
		model: &FznsoModel,
		decision: FznsoDecisionIdx,
		index: usize,
	) -> FznsoAnnotationRef<'_>,

	/// Retrieve the number of constraints currently contained in the model.
	pub constraint_len: extern "C" fn(model: &FznsoModel) -> usize,
	/// Retrieve the constraint index of the first new constraint in the layer.
	///
	/// This can be equivalent to `constraint_len` if the layer does not add any
	/// new constraints.
	pub constraint_layer_end: extern "C" fn(model: &FznsoModel, layer: usize) -> usize,
	/// Retrieve the identifier of a constraint.
	///
	/// The returned pointer can be assumed to have the the same lifetime as the
	/// model reference and must be valid UTF8.
	pub constraint_ident:
		extern "C" fn(model: &FznsoModel, constraint: FznsoConstraintIdx) -> *const ffi::c_char,
	/// Retrieve the number of arguments of a constraint.
	pub constraint_argument_len:
		extern "C" fn(model: &FznsoModel, constraint: FznsoConstraintIdx) -> usize,
	/// Retrieve the value of the constraint's argument at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `constraint_argument_len` for the given constraint.
	pub constraint_argument: extern "C" fn(
		model: &FznsoModel,
		constraint: FznsoConstraintIdx,
		index: usize,
	) -> FznsoValueRef<'_>,
	/// Check whether the decision variable is functionally defined by a
	/// constraint.
	///
	/// This function returns a [`FznsoValue`] that is either
	/// [`FznsoValueKind::FznsoValueDecision`] if it defined a decision variable
	/// or [`FznsoValueKind::FznsoValueAbsent`] otherwise.
	pub constraint_defines:
		extern "C" fn(model: &FznsoModel, constraint: FznsoConstraintIdx) -> FznsoValueRef<'_>,
	/// Retrieve the number of annotations on a constraint.
	pub constraint_annotation_len:
		extern "C" fn(model: &FznsoModel, constraint: FznsoConstraintIdx) -> usize,
	/// Retrieve the annotation on a constraint at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `constraint_annotation_len` for the given constraint.
	pub constraint_annotation: extern "C" fn(
		model: &FznsoModel,
		constraint: FznsoConstraintIdx,
		index: usize,
	) -> FznsoAnnotationRef<'_>,

	/// Request the identifier of the type of objective strategy to be used when
	/// solving the model.
	///
	/// Note that the function can return a null pointer if the model does not
	/// have an objective strategy.
	pub objective_ident: extern "C" fn(model: &FznsoModel) -> *const ffi::c_char,
	/// Retrieve the argument of the objective strategy.
	pub objective_arg: extern "C" fn(model: &FznsoModel) -> FznsoValueRef<'_>,
	/// Retrieve the number of annotations on an objective
	pub objective_annotation_len: extern "C" fn(model: &FznsoModel) -> usize,
	/// Retrieve the annotation on the objective at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `objective_annotation_len`.
	pub objective_annotation:
		extern "C" fn(model: &FznsoModel, index: usize) -> FznsoAnnotationRef<'_>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// An interface to a model instance used to communicate with the solver.
///
/// The solver can use the included function callbacks to interact with the
/// model.
pub struct FznsoModelRef<'a> {
	/// The handle to the data of the model instance.
	pub data: &'a FznsoModel,
	/// Reference to the structure containing the function callbacks used to
	/// interact with the model.
	pub methods: &'static FznsoModelMethods,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// Representation of a type of objective strategies, discerned by its
/// identifier and the type of its argument.
pub struct FznsoObjective<'a> {
	/// The identifier of the objective type.
	pub ident: *const ffi::c_char,
	/// The type of the expected argument for the constraint type.
	pub arg_type: FznsoType,
	/// The lifetime for which the `ident` attribute is allocated.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// A list of [`FznsoObjective`]s.
///
/// This type is, for example, used to return from [`fznso_objective_list`].
pub struct FznsoObjectiveList<'a> {
	/// The number of elements in the `options` array.
	pub len: usize,
	/// An array of option definitions.
	pub options: *const FznsoObjective<'a>,
	/// The lifetime of the `options` attribute.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// The definition of an option that is available to be set for the solver.
pub struct FznsoOption<'a> {
	/// The identifier used to set the option or get the current value of the
	/// option.
	pub ident: *const ffi::c_char,
	/// The type of value that is expected for this option.
	pub arg_ty: FznsoValueRef<'a>,
	/// The default value for this option.
	pub arg_def: FznsoType,
	/// The lifetime of the `ident` attribute.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// A list of [`FznsoOption`]s.
///
/// This type is, for example, used to return from [`fznso_option_list`].
pub struct FznsoOptionList<'a> {
	/// The number of elements in the `options` array.
	pub len: usize,
	/// An array of option definitions.
	pub options: *const FznsoOption<'a>,
	/// The lifetime of the `options` attribute.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(transparent)]
#[derive(Debug)]
/// The handle to a the data of a solution instance.
///
/// This type is opaque to the user. Only pointers of this type are ever used.
///
/// In implementation of the FZnSO interface, a pointer is generally cast to
/// this type, i.e. `(FznsoSolutionData*) my_solution`. A similar cast can be
/// used to cast the pointer back to the original type, e.g. `(MySolution*)
/// fznso_solution`.
pub struct FznsoSolution(pub ffi::c_void);

#[repr(C)]
#[derive(Clone, Debug)]
/// A struct containing the function pointers to interact with a
/// [`FznsoSolution`]
pub struct FznsoSolutionMethods {
	/// Function callback to retrieve the value assigned to a decision variable
	/// in the solution.
	pub value: extern "C" fn(data: &FznsoSolution, decision_index: usize) -> FznsoValueRef<'_>,
	/// Function callback to retrieve the statistical information made available
	/// by the solver about the search process so far.
	pub statistic:
		extern "C" fn(data: &FznsoSolution, ident: *const ffi::c_char) -> FznsoValueRef<'_>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// Handle for a solution emitted by the solver.
pub struct FznsoSolutionRef<'a> {
	/// The data pointer to be used as the first argument in the function
	/// callbacks provided by the `methods` field.
	pub data: &'a FznsoSolution,
	/// Reference to the structure containing the function callbacks used to
	/// interact with the solution.
	pub methods: &'static FznsoSolutionMethods,
}

#[repr(transparent)]
#[derive(Debug)]
/// The handle to a solver instance.
///
/// This type is opaque to the user. Only pointers of this type are ever used.
///
/// In implementation of the FZnSO interface, a pointer is generally cast to
/// this type, i.e. `(FznsoSolver*) my_solver`. A similar cast can be used to
/// cast the pointer back to the original type, e.g. `(MySolverType*)
/// fznso_solver`.
pub struct FznsoSolver(pub ffi::c_void);

#[repr(C)]
#[derive(Clone, Debug)]
/// The definition of statistical information that is made available by the
/// solver.
pub struct FznsoStatistic<'a> {
	/// The identifier used to retrieve the statistical information from the
	/// solver or a solution.
	pub ident: *const ffi::c_char,
	/// The type of value that is expected for this option.
	pub ty: FznsoValueRef<'a>,
	/// Whether the statistical information is available as part of solutions.
	pub solution: bool,
	/// Whether the statistical information is generally available from the
	/// solver instance.
	pub solver: bool,
	/// The lifetime of the `ident` attribute.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// A list of [`FznsoStatistic`]s.
///
/// This type is, for example, used to return from [`fznso_statistic_list`].
pub struct FznsoStatisticList<'a> {
	/// The number of elements in the `stats` array.
	pub len: usize,
	/// An array of statistical information definitions.
	pub stats: *const FznsoStatistic<'a>,
	/// The lifetime of the `stats` attribute.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// The status returned by `fznso_solver_run`, indicating whether the solver
/// completed its search.
pub enum FznsoStatus {
	/// The solver explored the full search space and yielded all relevant
	/// solutions.
	FznsoComplete,
	/// The solver did not explore the full search space due to a timeout or
	/// other termination condition. Additional (better) solutions might be
	/// possible.
	FznsoIncomplete,
	/// An error occurred during the solver's execution.
	///
	/// [`fznso_solver_read_error`] can be used to retrieve the error message.
	FznsoError,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Representation of a type to signal and check whether an argument takes the
/// correct type.
pub struct FznsoType {
	/// Whether the type is a list of values.
	pub list_of: bool,
	/// Whether the argument can be or contain decision variables (represented
	/// as decision indexes).
	pub decision: bool,
	/// Whether expected type is an set of values of the base type.
	pub set_of: bool,
	/// Whether the expected type is optional (and can take the value of
	/// [`FznsoValueKind::FznsoValueAbsent`]).
	pub opt: bool,
	/// The expected base type of the argument.
	pub base: FznsoTypeBase,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Representation of the base type of a value.
pub enum FznsoTypeBase {
	/// Boolean type
	FznsoTypeBaseBool,
	/// Integer numeric type
	FznsoTypeBaseInt,
	/// Floating point numeric type
	FznsoTypeBaseFloat,
	/// Character string type
	FznsoTypeBaseString,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// A list of [`FznsoType`]s.
///
/// This type is for example used to return from [`fznso_decision_list`].
pub struct FznsoTypeList<'a> {
	/// The number of elements in the `constraints` array.
	pub len: usize,
	/// An array of constraint types.
	pub types: *const FznsoType,
	/// The lifetime for which the `types` attribute is allocated.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(transparent)]
#[derive(Debug)]
/// The type of a value used as an argument or solution assignment.
///
/// This type is opaque to the user. Only pointers of this type are ever used.
///
/// In implementation of the FZnSO interface, a pointer is generally cast to
/// this type, e.g. `(FznsoValue*) my_value`. A similar cast can be used to cast
/// the pointer back to the original type, e.g. `(MyValue*) fznso_value`.
pub struct FznsoValue(ffi::c_void);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Enumerated type used to mark the kind of [`FznsoValue`]. This is used to
/// determine which "get" method in [`FznsoValueMethods`] is safe to call.
pub enum FznsoValueKind {
	/// No value is available.
	FznsoValueAbsent,
	/// The value is available using [`FznsoValueMethods::get_decision`].
	FznsoValueDecision,
	/// The value is available using [`FznsoValueMethods::get_constraint`].
	FznsoValueConstraint,
	/// The value is available using [`FznsoValueMethods::get_bool`].
	FznsoValueBool,
	/// The value is available using [`FznsoValueMethods::get_int`].
	FznsoValueInt,
	/// The value is available using [`FznsoValueMethods::get_float`].
	FznsoValueFloat,
	/// The value is available using [`FznsoValueMethods::get_string`].
	FznsoValueString,
	/// Sets of integers are represented using a range list. The number of
	/// ranges is available using [`FznsoValueMethods::len`], and the ranges can
	/// be accessed using [`FznsoValueMethods::get_range_int`].
	FznsoValueSetInt,
	/// Sets of floats are represented using a range list. The number of
	/// ranges is available using [`FznsoValueMethods::len`], and the ranges can
	/// be accessed using [`FznsoValueMethods::get_range_float`].
	FznsoValueSetFloat,
	/// The length of the list can be accessed using
	/// [`FznsoValueMethods::len`], and elements in the list can be
	/// accessed using [`FznsoValueMethods::get_element`]
	FznsoValueList,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// A struct containing the function pointers to interact with a
/// [`FznsoValue`]
pub struct FznsoValueMethods {
	/// Function callback that returns the kind of the value.
	pub kind: extern "C" fn(&FznsoValue) -> FznsoValueKind,
	/// Function callback that returns the length of the value.
	///
	/// In case [`FznsoValueMethods::kind`] returns
	/// [`FznsoValueKind::FznsoValueList`], the length is the number of elements
	/// in the list, accessible using [`FznsoValueMethods::get_element`].
	///
	/// In case [`FznsoValueMethods::kind`] returns
	/// [`FznsoValueKind::FznsoValueSetInt`] or
	/// [`FznsoValueKind::FznsoValueSetFloat`], the length is the number of
	/// ranges in the set, accessible using
	/// [`FznsoValueMethods::get_range_int`] or
	/// [`FznsoValueMethods::get_range_float`].
	///
	/// # Panics
	///
	/// This function callback may panic if [`FznsoValueMethods::kind`] does not
	/// return [`FznsoValueKind::FznsoValueList`],
	/// [`FznsoValueKind::FznsoValueSetInt`], or
	/// [`FznsoValueKind::FznsoValueSetFloat`].
	pub len: extern "C" fn(&FznsoValue) -> usize,
	/// Function callback that returns the decision variable index contained in
	/// the value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`FznsoValueMethods::kind`] does not
	/// return [`FznsoValueKind::FznsoValueDecision`].
	pub as_decision: extern "C" fn(&FznsoValue) -> FznsoDecisionIdx,
	/// Function callback that returns the constraint index contained in the
	/// value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`FznsoValueMethods::kind`] does not
	/// return [`FznsoValueKind::FznsoValueConstraint`].
	pub as_constraint: extern "C" fn(&FznsoValue) -> FznsoConstraintIdx,
	/// Function callback that returns the integer value contained in the value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`FznsoValueMethods::kind`] does not
	/// return [`FznsoValueKind::FznsoValueInt`].
	pub as_int: extern "C" fn(&FznsoValue) -> i64,
	/// Function callback that returns the floating point value contained in the
	/// value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`FznsoValueMethods::kind`] does not
	/// return [`FznsoValueKind::FznsoValueFloat`].
	pub as_float: extern "C" fn(&FznsoValue) -> f64,
	/// Function callback that returns the string pointer value contained in the
	/// value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`FznsoValueMethods::kind`] does not
	/// return [`FznsoValueKind::FznsoValueString`].
	pub as_string: extern "C" fn(&FznsoValue) -> *const ffi::c_char,
	/// Function callback that returns the Boolean value contained in the value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`FznsoValueMethods::kind`] does not
	/// return [`FznsoValueKind::FznsoValueBool`].
	pub as_bool: extern "C" fn(&FznsoValue) -> bool,
	/// Function callback that returns a range from a range list representing
	/// the integer set contained in the value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`FznsoValueMethods::kind`] does not
	/// return [`FznsoValueKind::FznsoValueSetInt`].
	pub get_range_int: extern "C" fn(&FznsoValue, index: usize) -> [i64; 2],
	/// Function callback that returns a range from a range list representing
	/// the floating point set contained in the value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`FznsoValueMethods::kind`] does not
	/// return [`FznsoValueKind::FznsoValueSetFloat`].
	pub get_range_float: extern "C" fn(&FznsoValue, index: usize) -> [f64; 2],
	/// Function callback that returns an element from the list contained in the
	/// value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`FznsoValueMethods::kind`] does not
	/// return [`FznsoValueKind::FznsoValueList`].
	pub get_element: extern "C" fn(&FznsoValue, index: usize) -> FznsoValueRef<'_>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// Handle for a value
///
/// The caller can use the `get_value` function to retrieve the value of the
/// used decision variables.
pub struct FznsoValueRef<'a> {
	/// The data pointer to be the first argument of `get_value`.
	pub data: &'a FznsoValue,
	/// Reference to the structure containing the function callbacks used to
	/// interact with the value.
	pub methods: &'static FznsoValueMethods,
}

#[cfg(test)]
mod tests {
	use crate::{FznsoModelRef, FznsoSolutionRef, FznsoType, FznsoValueRef};

	#[test]
	fn memory() {
		assert_eq!(size_of::<FznsoModelRef>(), size_of::<u128>());
		assert_eq!(size_of::<FznsoSolutionRef>(), size_of::<u128>());
		assert_eq!(size_of::<FznsoType>(), size_of::<u64>());
		assert_eq!(size_of::<FznsoValueRef>(), size_of::<u128>());
	}
}
