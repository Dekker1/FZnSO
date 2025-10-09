//! IPDOS Protocol Types
//!
//! This crate defines the types of the IPDOS protocol, which allows the
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
/// In implementation of the IPDOS interface, a pointer is generally cast to
/// this type, i.e. `(IpdosSolutionData*) my_solution`. A similar cast can be
/// used to cast the pointer back to the original type, e.g. `(MySolution*)
/// ipdos_solution`.
pub struct IpdosAnnotation(ffi::c_void);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Wrapper type for the indexes that represent constraints in the model.
pub struct IpdosConstraintIdx(pub usize);

#[repr(C)]
#[derive(Clone, Debug)]
/// A list of [`IpdosConstraintType`]s.
///
/// This type is for example used to return from [`ipdos_constraint_list`].
pub struct IpdosConstraintList<'a> {
	/// The number of elements in the `constraints` array.
	pub len: usize,
	/// An array of constraint types.
	pub constraints: *const IpdosConstraintType<'a>,
	/// The lifetime for which the `constraints` attribute is allocated.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// Representation of a type of constraint, discerned by its identifier and the
/// types of its arguments.
pub struct IpdosConstraintType<'a> {
	/// The identifier of the constraint type.
	pub ident: *const ffi::c_char,
	/// The number of expected arguments for the constraint type.
	pub arg_len: usize,
	/// The types of the expected arguments for the constraint type.
	pub arg_types: *const IpdosType,
	/// The lifetime for which the `ident` and `arg_types` attributes are
	/// allocated.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// Wrapper type for the indexes that represent decision variables in the model.
pub struct IpdosDecisionIdx(pub usize);

#[repr(transparent)]
#[derive(Debug)]
/// The handle of a model instance.
///
/// This type is opaque to the user. Only pointers of this type are ever used.
///
/// In implementation of the IPDOS interface, a pointer is generally cast to
/// this type, i.e. `(IpdosModelData*) my_model`. A similar cast can be used to
/// cast the pointer back to the original type, e.g. `(MyModel*) ipdos_model`.
pub struct IpdosModel(ffi::c_void);

#[repr(C)]
#[derive(Debug)]
/// A struct containing the function pointers to interact with the
/// [`IpdosModel`]
pub struct IpdosModelMethods {
	/// Returns the current number of model layers currently contained in the
	/// model.
	///
	/// Layers provides a way for the modelling user to add and retract
	layer_len: extern "C" fn(model: &IpdosModel) -> usize,
	/// Returns the number of layers that have been unchanged since the last
	/// call to [`ipdos_solver_run`].
	///
	/// Unchanged layers must be consecutive starting from layer 0.
	layer_unchanged: extern "C" fn(model: &IpdosModel) -> usize,
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
	layer_permanent: extern "C" fn(model: &IpdosModel) -> usize,
	/// Returns the number of permanent layers that have been marked as
	/// redundant.
	///
	/// The solver can remove the decision variables and constraints in these
	/// layers, if convenient. Once a layer is marked as redundant, it can
	/// forever be considered redundant. As such, the solver can assume that
	/// the number of redundant layers only ever increases.
	layer_redundant_len: extern "C" fn(model: &IpdosModel) -> usize,
	/// Returns the index of the n-th permanent layer that has been marked as
	/// redundant.
	///
	/// The `n` argument must be less than the value returned by
	/// [`layer_redundant_len`].
	layer_redundant_index: extern "C" fn(model: &IpdosModel, n: usize) -> usize,

	/// Retrieve the number of decisions currently contained in the model.
	decision_len: extern "C" fn(model: &IpdosModel) -> usize,
	/// Retrieve the decision index of the last decision variable in the layer.
	///
	/// This can be equivalent to `decision_layer_end` of `layer-1` if the layer
	/// does not add any new decisions.
	decision_layer_end: extern "C" fn(model: &IpdosModel, layer: usize) -> usize,
	/// Retrieve the domain of the given decision.
	///
	/// Note that the the value might have [`IpdosValueKind::IpdosValueAbsent`]
	/// if the decision variable does not have an explicit domain.
	decision_domain:
		extern "C" fn(model: &IpdosModel, decision: IpdosDecisionIdx) -> IpdosValueRef<'_>,
	/// Retrieve the name of a decision variable, if it exists.
	///
	/// Note that names are only available for debugging purposes. Decisions are
	/// identified using their index in the model. If the decision variable does
	/// not have a name, this function returns a null pointer.
	decision_name:
		extern "C" fn(model: &IpdosModel, decision: IpdosDecisionIdx) -> *const ffi::c_char,
	/// Check whether the decision variable is defined by a constraint.
	decision_defined: extern "C" fn(model: &IpdosModel, decision: IpdosDecisionIdx) -> bool,
	/// Retrieve the number of annotations of a constraint.
	decision_annotation_len: extern "C" fn(model: &IpdosModel, decision: IpdosDecisionIdx) -> usize,
	/// Retrieve the value of the constraint's annotation at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// [`decision_annotation_len`] for the given decision variable.
	decision_annotation: extern "C" fn(
		model: &IpdosModel,
		decision: IpdosDecisionIdx,
		index: usize,
	) -> &IpdosAnnotation,

	/// Retrieve the number of constraints currently contained in the model.
	constraint_len: extern "C" fn(model: &IpdosModel) -> usize,
	/// Retrieve the constraint index of the first new constraint in the layer.
	///
	/// This can be equivalent to `constraint_len` if the layer does not add any
	/// new constraints.
	constraint_layer_end: extern "C" fn(model: &IpdosModel) -> usize,
	/// Retrieve the identifier of a constraint.
	///
	/// The returned pointer can be assumed to have the the same lifetime as the
	/// model reference and must be valid UTF8.
	constraint_ident:
		extern "C" fn(model: &IpdosModel, constraint: IpdosConstraintIdx) -> *const ffi::c_char,
	/// Retrieve the number of arguments of a constraint.
	constraint_argument_len:
		extern "C" fn(model: &IpdosModel, constraint: IpdosConstraintIdx) -> usize,
	/// Retrieve the value of the constraint's argument at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `constraint_argument_len` for the given constraint.
	constraint_argument: extern "C" fn(
		model: &IpdosModel,
		constraint: IpdosConstraintIdx,
		index: usize,
	) -> IpdosValueRef<'_>,
	/// Check whether the decision variable is functionally defined by a
	/// constraint.
	///
	/// This function returns a [`IpdosValue`] that is either
	/// [`IpdosValueKind::IpdosValueDecision`] if it defined a decision variable
	/// or [`IpdosValueKind::IpdosValueAbsent`] otherwise.
	constraint_defines:
		extern "C" fn(model: &IpdosModel, constraint: IpdosConstraintIdx) -> IpdosValueRef<'_>,
	/// Retrieve the number of annotations on a constraint.
	constraint_annotation_len:
		extern "C" fn(model: &IpdosModel, constraint: IpdosConstraintIdx) -> usize,
	/// Retrieve the annotation on a constraint at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `constraint_annotation_len` for the given constraint.
	constraint_annotation: extern "C" fn(
		model: &IpdosModel,
		constraint: IpdosConstraintIdx,
		index: usize,
	) -> &IpdosAnnotation,

	/// Request the identifier of the type of objective strategy to be used when
	/// solving the model.
	///
	/// Note that the function can return a null pointer if the model does not
	/// have an objective strategy.
	objective_ident: extern "C" fn(model: &IpdosModel) -> *const ffi::c_char,
	/// Retrieve the argument of the objective strategy.
	objective_arg: extern "C" fn(model: &IpdosModel) -> IpdosValueRef<'_>,
	/// Retrieve the number of annotations on an objective
	objective_annotation_len: extern "C" fn(model: &IpdosModel) -> usize,
	/// Retrieve the annotation on the objective at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `objective_annotation_len`.
	objective_annotation: extern "C" fn(model: &IpdosModel, index: usize) -> &IpdosAnnotation,

	/// Retrieve the identifier of an annotation.
	///
	/// The returned pointer can be assumed to have the the same lifetime as the
	/// annotation reference and must be valid UTF8.
	annotation_ident: extern "C" fn(ann: &IpdosAnnotation) -> *const ffi::c_char,
	/// Retrieve the number of arguments of an annotation.
	annotation_argument_len: extern "C" fn(ann: &IpdosAnnotation) -> usize,
	/// Retrieve the value of the annotation's argument at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `annotation_argument_len` for the given annotation.
	annotation_argument: extern "C" fn(ann: &IpdosAnnotation, index: usize) -> IpdosValueRef<'_>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// An interface to a model instance used to communicate with the solver.
///
/// The solver can use the included function callbacks to interact with the
/// model.
pub struct IpdosModelRef<'a> {
	/// The handle to the data of the model instance.
	data: &'a IpdosModel,
	/// Reference to the structure containing the function callbacks used to
	/// interact with the model.
	methods: &'static IpdosModelMethods,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// Representation of a type of objective strategies, discerned by its
/// identifier and the type of its argument.
pub struct IpdosObjective<'a> {
	/// The identifier of the objective type.
	pub ident: *const ffi::c_char,
	/// The type of the expected argument for the constraint type.
	pub arg_type: IpdosType,
	/// The lifetime for which the `ident` attribute is allocated.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// A list of [`IpdosObjective`]s.
///
/// This type is, for example, used to return from [`ipdos_objective_list`].
pub struct IpdosObjectiveList<'a> {
	/// The number of elements in the `options` array.
	pub len: usize,
	/// An array of option definitions.
	pub options: *const IpdosObjective<'a>,
	/// The lifetime of the `options` attribute.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// The definition of an option that is available to be set for the solver.
pub struct IpdosOption<'a> {
	/// The identifier used to set the option or get the current value of the
	/// option.
	pub ident: *const ffi::c_char,
	/// The type of value that is expected for this option.
	pub arg_ty: IpdosValueRef<'a>,
	/// The default value for this option.
	pub arg_def: IpdosType,
	/// The lifetime of the `ident` attribute.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// A list of [`IpdosOption`]s.
///
/// This type is, for example, used to return from [`ipdos_option_list`].
pub struct IpdosOptionList<'a> {
	/// The number of elements in the `options` array.
	pub len: usize,
	/// An array of option definitions.
	pub options: *const IpdosOption<'a>,
	/// The lifetime of the `options` attribute.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(transparent)]
#[derive(Debug)]
/// The handle to a the data of a solution instance.
///
/// This type is opaque to the user. Only pointers of this type are ever used.
///
/// In implementation of the IPDOS interface, a pointer is generally cast to
/// this type, i.e. `(IpdosSolutionData*) my_solution`. A similar cast can be
/// used to cast the pointer back to the original type, e.g. `(MySolution*)
/// ipdos_solution`.
pub struct IpdosSolution(ffi::c_void);

#[repr(C)]
#[derive(Clone, Debug)]
/// A struct containing the function pointers to interact with a
/// [`IpdosSolution`]
pub struct IpdosSolutionMethods {
	/// Function callback to retrieve the value assigned to a decision variable
	/// in the solution.
	get_value: extern "C" fn(data: &IpdosSolution, decision_index: usize) -> IpdosValueRef<'_>,
	/// Function callback to retrieve the statistical information made available
	/// by the solver about the search process so far.
	get_statistic:
		extern "C" fn(data: &IpdosSolution, ident: *const ffi::c_char) -> IpdosValueRef<'_>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// Handle for a solution emitted by the solver.
pub struct IpdosSolutionRef<'a> {
	/// The data pointer to be the first argument of `get_value`.
	data: &'a IpdosSolution,
	/// Reference to the structure containing the function callbacks used to
	/// interact with the solution.
	methods: &'static IpdosSolutionMethods,
}

#[repr(transparent)]
#[derive(Debug)]
/// The handle to a solver instance.
///
/// This type is opaque to the user. Only pointers of this type are ever used.
///
/// In implementation of the IPDOS interface, a pointer is generally cast to
/// this type, i.e. `(IpdosSolver*) my_solver`. A similar cast can be used to
/// cast the pointer back to the original type, e.g. `(MySolverType*)
/// ipdos_solver`.
pub struct IpdosSolver(ffi::c_void);

#[repr(C)]
#[derive(Clone, Debug)]
/// The definition of statistical information that is made available by the
/// solver.
pub struct IpdosStatistic<'a> {
	/// The identifier used to retrieve the statistical information from the
	/// solver or a solution.
	pub ident: *const ffi::c_char,
	/// The type of value that is expected for this option.
	pub ty: IpdosValueRef<'a>,
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
/// A list of [`IpdosStatistic`]s.
///
/// This type is, for example, used to return from [`ipdos_statistic_list`].
pub struct IpdosStatisticList<'a> {
	/// The number of elements in the `stats` array.
	pub len: usize,
	/// An array of statistical information definitions.
	pub stats: *const IpdosStatistic<'a>,
	/// The lifetime of the `stats` attribute.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// The status returned by `ipdos_solver_run`, indicating whether the solver
/// completed its search.
pub enum IpdosStatus {
	/// The solver explored the full search space and yielded all relevant
	/// solutions.
	IpdosComplete,
	/// The solver did not explore the full search space due to a timeout or
	/// other termination condition. Additional (better) solutions might be
	/// possible.
	IpdosIncomplete,
	/// An error occurred during the solver's execution.
	///
	/// [`ipdos_solver_read_error`] can be used to retrieve the error message.
	IpdosError,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Representation of a type to signal and check whether an argument takes the
/// correct type.
pub struct IpdosType {
	/// Whether the type is a list of values.
	pub list_of: bool,
	/// Whether the argument can be or contain decision variables (represented
	/// as decision indexes).
	pub decision: bool,
	/// Whether expected type is an set of values of the base type.
	pub set_of: bool,
	/// Whether the expected type is optional (and can take the value of
	/// [`IpdosValueKind::IpdosValueAbsent`]).
	pub opt: bool,
	/// The expected base type of the argument.
	pub base: IpdosTypeBase,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Representation of the base type of a value.
pub enum IpdosTypeBase {
	/// Boolean type
	IpdosTypeBaseBool,
	/// Integer numeric type
	IpdosTypeBaseInt,
	/// Floating point numeric type
	IpdosTypeBaseFloat,
	/// Character string type
	IpdosTypeBaseString,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// A list of [`IpdosType`]s.
///
/// This type is for example used to return from [`ipdos_decision_list`].
pub struct IpdosTypeList<'a> {
	/// The number of elements in the `constraints` array.
	pub len: usize,
	/// An array of constraint types.
	pub types: *const IpdosType,
	/// The lifetime for which the `types` attribute is allocated.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(transparent)]
#[derive(Debug)]
/// The type of a value used as an argument or solution assignment.
///
/// This type is opaque to the user. Only pointers of this type are ever used.
///
/// In implementation of the IPDOS interface, a pointer is generally cast to
/// this type, e.g. `(IpdosValue*) my_value`. A similar cast can be used to cast
/// the pointer back to the original type, e.g. `(MyValue*) ipdos_value`.
pub struct IpdosValue(ffi::c_void);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Enumerated type used to mark the kind of [`IpdosValue`]. This is used to
/// determine which "get" method in [`IpdosValueMethods`] is safe to call.
pub enum IpdosValueKind {
	/// No value is available.
	IpdosValueAbsent,
	/// The value is available using [`IpdosValueMethods::get_decision`].
	IpdosValueDecision,
	/// The value is available using [`IpdosValueMethods::get_constraint`].
	IpdosValueConstraint,
	/// The value is available using [`IpdosValueMethods::get_bool`].
	IpdosValueBool,
	/// The value is available using [`IpdosValueMethods::get_int`].
	IpdosValueInt,
	/// The value is available using [`IpdosValueMethods::get_float`].
	IpdosValueFloat,
	/// The value is available using [`IpdosValueMethods::get_string`].
	IpdosValueString,
	/// Sets of integers are represented using a range list. The number of
	/// ranges is available using [`IpdosValueMethods::len`], and the ranges can
	/// be accessed using [`IpdosValueMethods::get_range_int`].
	IpdosValueSetInt,
	/// Sets of floats are represented using a range list. The number of
	/// ranges is available using [`IpdosValueMethods::len`], and the ranges can
	/// be accessed using [`IpdosValueMethods::get_range_float`].
	IpdosValueSetFloat,
	/// The length of the list can be accessed using
	/// [`IpdosValueMethods::len`], and elements in the list can be
	/// accessed using [`IpdosValueMethods::get_element`]
	IpdosValueList,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// A struct containing the function pointers to interact with a
/// [`IpdosValue`]
pub struct IpdosValueMethods {
	/// Function callback that returns the kind of the value.
	kind: extern "C" fn(&IpdosValue) -> IpdosValueKind,
	/// Function callback that returns the length of the value.
	///
	/// In case [`IpdosValueMethods::kind`] returns
	/// [`IpdosValueKind::IpdosValueList`], the length is the number of elements
	/// in the list, accessible using [`IpdosValueMethods::get_element`].
	///
	/// In case [`IpdosValueMethods::kind`] returns
	/// [`IpdosValueKind::IpdosValueSetInt`] or
	/// [`IpdosValueKind::IpdosValueSetFloat`], the length is the number of
	/// ranges in the set, accessible using
	/// [`IpdosValueMethods::get_range_int`] or
	/// [`IpdosValueMethods::get_range_float`].
	///
	/// # Panics
	///
	/// This function callback may panic if [`IpdosValueMethods::kind`] does not
	/// return [`IpdosValueKind::IpdosValueList`],
	/// [`IpdosValueKind::IpdosValueSetInt`], or
	/// [`IpdosValueKind::IpdosValueSetFloat`].
	len: extern "C" fn(&IpdosValue) -> usize,
	/// Function callback that returns the decision variable index contained in
	/// the value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`IpdosValueMethods::kind`] does not
	/// return [`IpdosValueKind::IpdosValueDecision`].
	get_decision: extern "C" fn(&IpdosValue) -> IpdosDecisionIdx,
	/// Function callback that returns the constraint index contained in the
	/// value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`IpdosValueMethods::kind`] does not
	/// return [`IpdosValueKind::IpdosValueConstraint`].
	get_constraint: extern "C" fn(&IpdosValue) -> IpdosConstraintIdx,
	/// Function callback that returns the integer value contained in the value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`IpdosValueMethods::kind`] does not
	/// return [`IpdosValueKind::IpdosValueInt`].
	get_int: extern "C" fn(&IpdosValue) -> i64,
	/// Function callback that returns the floating point value contained in the
	/// value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`IpdosValueMethods::kind`] does not
	/// return [`IpdosValueKind::IpdosValueFloat`].
	get_float: extern "C" fn(&IpdosValue) -> f64,
	/// Function callback that returns the string pointer value contained in the
	/// value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`IpdosValueMethods::kind`] does not
	/// return [`IpdosValueKind::IpdosValueString`].
	get_string: extern "C" fn(&IpdosValue) -> *const ffi::c_char,
	/// Function callback that returns the Boolean value contained in the value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`IpdosValueMethods::kind`] does not
	/// return [`IpdosValueKind::IpdosValueBool`].
	get_bool: extern "C" fn(&IpdosValue) -> bool,
	/// Function callback that returns a range from a range list representing
	/// the integer set contained in the value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`IpdosValueMethods::kind`] does not
	/// return [`IpdosValueKind::IpdosValueSetInt`].
	get_range_int: extern "C" fn(&IpdosValue, index: usize) -> [i64; 2],
	/// Function callback that returns a range from a range list representing
	/// the floating point set contained in the value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`IpdosValueMethods::kind`] does not
	/// return [`IpdosValueKind::IpdosValueSetFloat`].
	get_range_float: extern "C" fn(&IpdosValue, index: usize) -> [f64; 2],
	/// Function callback that returns an element from the list contained in the
	/// value.
	///
	/// # Panics
	///
	/// This function callback may panic if [`IpdosValueMethods::kind`] does not
	/// return [`IpdosValueKind::IpdosValueList`].
	get_element: extern "C" fn(&IpdosValue, index: usize) -> IpdosValueRef<'_>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// Handle for a value
///
/// The caller can use the `get_value` function to retrieve the value of the
/// used decision variables.
pub struct IpdosValueRef<'a> {
	/// The data pointer to be the first argument of `get_value`.
	data: &'a IpdosValue,
	/// Reference to the structure containing the function callbacks used to
	/// interact with the value.
	methods: &'static IpdosValueMethods,
}

#[cfg(test)]
mod tests {
	use crate::{IpdosModelRef, IpdosSolutionRef, IpdosType, IpdosValueRef};

	#[test]
	fn memory() {
		assert_eq!(size_of::<IpdosModelRef>(), size_of::<u128>());
		assert_eq!(size_of::<IpdosSolutionRef>(), size_of::<u128>());
		assert_eq!(size_of::<IpdosType>(), size_of::<u64>());
		assert_eq!(size_of::<IpdosValueRef>(), size_of::<u128>());
	}
}
