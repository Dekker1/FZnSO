//! FZnSO Protocol Types
//!
//! This crate defines the types of the FZnSO protocol, which allows the
//! incremental usage of solvers that can solve decision and optimization
//! problems. The goal of the interface is to easily use different solvers in a
//! unified way, and to allow the dynamic loading of solver libraries (as DLLs).

mod rust_interaction;

use std::{ffi, marker::PhantomData};

/// The FZnSO ABI version this crate defines.
///
/// A solver exports it as `fznso_<name>_abi_version`, and a loader rejects any
/// solver whose reported version differs from its own.
pub const FZNSO_ABI_VERSION: u32 = 1;

#[repr(transparent)]
#[derive(Debug)]
/// The handle of an annotation on a decision, constraint or objective.
///
/// Opaque: only the side that created it may interpret it. Read it through
/// [`FznsoAnnotationRef`], which pairs it with the callbacks that can.
pub struct FznsoAnnotation(pub ffi::c_void);

#[repr(C)]
#[derive(Debug)]
/// The callbacks that read a [`FznsoAnnotation`].
pub struct FznsoAnnotationMethods {
	/// Retrieve the identifier of an annotation.
	///
	/// The returned string can be assumed to have the the same lifetime as the
	/// annotation reference and must be valid UTF8.
	pub ident: extern "C" fn(ann: FznsoPtr<'_, FznsoAnnotation>) -> FznsoStr<'_>,
	/// Retrieve the number of arguments of an annotation.
	pub argument_len: extern "C" fn(ann: FznsoPtr<'_, FznsoAnnotation>) -> usize,
	/// Retrieve the value of the annotation's argument at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `annotation_argument_len` for the given annotation.
	pub argument:
		extern "C" fn(ann: FznsoPtr<'_, FznsoAnnotation>, index: usize) -> FznsoValueRef<'_>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// An annotation together with the callbacks that read it.
pub struct FznsoAnnotationRef<'a> {
	/// The data pointer of the annotation.
	pub data: FznsoPtr<'a, FznsoAnnotation>,
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
	pub ident: FznsoStr<'a>,
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

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
/// An inclusive `[min, max]` float range, returned by
/// [`FznsoValueMethods::get_range_float`].
pub struct FznsoFloatRange {
	/// The inclusive lower bound.
	pub min: f64,
	/// The inclusive upper bound.
	pub max: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
/// An inclusive `[min, max]` integer range, returned by
/// [`FznsoValueMethods::get_range_int`].
pub struct FznsoIntRange {
	/// The inclusive lower bound.
	pub min: i64,
	/// The inclusive upper bound.
	pub max: i64,
}

#[repr(transparent)]
/// A pointer to an FZnSO handle, tagged with the lifetime for which the handle
/// stays valid.
///
/// This is deliberately a raw pointer rather than a Rust reference, because the
/// interface relies on two things a reference cannot do:
///
/// - **Casting back to the implementation's own type.** A handle is produced by
///   casting e.g. `*const MyModel` to `*const FznsoModel`, and the callbacks
///   cast it back before use. A `&FznsoModel` would only carry permission to
///   access the opaque handle type itself, so reading the larger original type
///   through it is undefined behaviour.
/// - **Encoding a value directly in the pointer word.** Small values such as a
///   [`FznsoDecisionIdx`] are stored *in* the pointer rather than behind it, so
///   that handing one across the interface never requires an allocation. See
///   [`FznsoPtr::from_addr`].
///
/// The `'a` lifetime costs nothing at runtime. It exists so that callback
/// signatures state how long the handle — and anything derived from it —
/// remains valid, which is what lets the return types of
/// [`FznsoModelMethods`] and friends borrow from their argument.
///
/// cbindgen:no-export
pub struct FznsoPtr<'a, T> {
	/// The pointer to the handle, or a value encoded in the pointer word.
	pub ptr: *const T,
	/// The lifetime for which `ptr` stays valid.
	pub lifetime: PhantomData<&'a T>,
}

#[repr(transparent)]
#[derive(Debug)]
/// The handle of a model instance.
///
/// Opaque: only the side that created it may interpret it. Read it through
/// [`FznsoModelRef`], which pairs it with the callbacks that can.
pub struct FznsoModel(pub ffi::c_void);

#[repr(C)]
#[derive(Debug)]
/// The callbacks that read a [`FznsoModel`].
///
/// Every index argument has a matching length callback, and must be below it.
/// Everything these return (e.g. values, strings, and annotations) borrows from
/// the model, and so is valid for as long as the model handle itself.
pub struct FznsoModelMethods {
	/// Returns the current number of model layers currently contained in the
	/// model.
	///
	/// Layers are how an application adds and retracts groups of decision
	/// variables and constraints between runs; see [`layer_permanent`] for the
	/// ones that can never be retracted.
	pub layer_len: extern "C" fn(model: FznsoPtr<'_, FznsoModel>) -> usize,
	/// Returns the number of layers that have been unchanged since the last
	/// call to [`fznso_solver_run`].
	///
	/// Unchanged layers must be consecutive starting from layer 0.
	pub layer_unchanged: extern "C" fn(model: FznsoPtr<'_, FznsoModel>) -> usize,
	/// Returns the number of permanent layers in the model.
	///
	/// Permanent layers are consecutive, starting from layer 0, and can never
	/// be retracted, so a solver may assume their number only ever grows.
	///
	/// Permanent layers can, however, still be found to be redundant, allowing
	/// the solver to remove the decision variables and constraints in these
	/// layers, if convenient. This is signaled using [`layer_redundant_len`]
	/// and [`layer_redundant_index`].
	pub layer_permanent: extern "C" fn(model: FznsoPtr<'_, FznsoModel>) -> usize,
	/// Returns the number of permanent layers that have been marked as
	/// redundant.
	///
	/// The solver can remove the decision variables and constraints in these
	/// layers, if convenient. Once a layer is marked as redundant, it can
	/// forever be considered redundant. As such, the solver can assume that
	/// the number of redundant layers only ever increases.
	pub layer_redundant_len: extern "C" fn(model: FznsoPtr<'_, FznsoModel>) -> usize,
	/// Returns the index of the n-th permanent layer that has been marked as
	/// redundant.
	///
	/// The `n` argument must be less than the value returned by
	/// [`layer_redundant_len`].
	pub layer_redundant_index: extern "C" fn(model: FznsoPtr<'_, FznsoModel>, n: usize) -> usize,

	/// Retrieve the number of decisions currently contained in the model.
	pub decision_len: extern "C" fn(model: FznsoPtr<'_, FznsoModel>) -> usize,
	/// Retrieve the decision index of the last decision variable in the layer.
	///
	/// This can be equivalent to `decision_layer_end` of `layer-1` if the layer
	/// does not add any new decisions.
	pub decision_layer_end: extern "C" fn(model: FznsoPtr<'_, FznsoModel>, layer: usize) -> usize,
	/// Retrieve the domain of the given decision.
	///
	/// Note that the the value might have [`FznsoValueKind::FznsoValueAbsent`]
	/// if the decision variable does not have an explicit domain.
	pub decision_domain: extern "C" fn(
		model: FznsoPtr<'_, FznsoModel>,
		decision: FznsoDecisionIdx,
	) -> FznsoValueRef<'_>,
	/// Retrieve the name of a decision variable, if it exists.
	///
	/// Note that names are only available for debugging purposes. Decisions are
	/// identified using their index in the model. If the decision variable does
	/// not have a name, the returned string has a null `ptr`.
	pub decision_name:
		extern "C" fn(model: FznsoPtr<'_, FznsoModel>, decision: FznsoDecisionIdx) -> FznsoStr<'_>,
	/// Check whether the decision variable is defined by a constraint.
	pub decision_defined:
		extern "C" fn(model: FznsoPtr<'_, FznsoModel>, decision: FznsoDecisionIdx) -> bool,
	/// Retrieve the number of annotations on a decision variable.
	pub decision_annotation_len:
		extern "C" fn(model: FznsoPtr<'_, FznsoModel>, decision: FznsoDecisionIdx) -> usize,
	/// Retrieve the annotation on a decision variable at the given index.
	///
	/// Requires `index` to be less than [`decision_annotation_len`] for that
	/// decision variable.
	pub decision_annotation: extern "C" fn(
		model: FznsoPtr<'_, FznsoModel>,
		decision: FznsoDecisionIdx,
		index: usize,
	) -> FznsoAnnotationRef<'_>,

	/// Retrieve the number of constraints currently contained in the model.
	pub constraint_len: extern "C" fn(model: FznsoPtr<'_, FznsoModel>) -> usize,
	/// Retrieve the constraint index of the first new constraint in the layer.
	///
	/// This can be equivalent to `constraint_len` if the layer does not add any
	/// new constraints.
	pub constraint_layer_end: extern "C" fn(model: FznsoPtr<'_, FznsoModel>, layer: usize) -> usize,
	/// Retrieve the identifier of a constraint.
	///
	/// The returned string can be assumed to have the the same lifetime as the
	/// model reference and must be valid UTF8.
	pub constraint_ident: extern "C" fn(
		model: FznsoPtr<'_, FznsoModel>,
		constraint: FznsoConstraintIdx,
	) -> FznsoStr<'_>,
	/// Retrieve the number of arguments of a constraint.
	pub constraint_argument_len:
		extern "C" fn(model: FznsoPtr<'_, FznsoModel>, constraint: FznsoConstraintIdx) -> usize,
	/// Retrieve the value of the constraint's argument at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `constraint_argument_len` for the given constraint.
	pub constraint_argument: extern "C" fn(
		model: FznsoPtr<'_, FznsoModel>,
		constraint: FznsoConstraintIdx,
		index: usize,
	) -> FznsoValueRef<'_>,
	/// Check whether the decision variable is functionally defined by a
	/// constraint.
	///
	/// This function returns a [`FznsoValue`] that is either
	/// [`FznsoValueKind::FznsoValueDecision`] if it defined a decision variable
	/// or [`FznsoValueKind::FznsoValueAbsent`] otherwise.
	pub constraint_defines: extern "C" fn(
		model: FznsoPtr<'_, FznsoModel>,
		constraint: FznsoConstraintIdx,
	) -> FznsoValueRef<'_>,
	/// Retrieve the number of annotations on a constraint.
	pub constraint_annotation_len:
		extern "C" fn(model: FznsoPtr<'_, FznsoModel>, constraint: FznsoConstraintIdx) -> usize,
	/// Retrieve the annotation on a constraint at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `constraint_annotation_len` for the given constraint.
	pub constraint_annotation: extern "C" fn(
		model: FznsoPtr<'_, FznsoModel>,
		constraint: FznsoConstraintIdx,
		index: usize,
	) -> FznsoAnnotationRef<'_>,

	/// Request the identifier of the type of objective strategy to be used when
	/// solving the model.
	///
	/// Note that the returned string has a null `ptr` if the model does not
	/// have an objective strategy.
	pub objective_ident: extern "C" fn(model: FznsoPtr<'_, FznsoModel>) -> FznsoStr<'_>,
	/// Retrieve the argument of the objective strategy.
	pub objective_arg: extern "C" fn(model: FznsoPtr<'_, FznsoModel>) -> FznsoValueRef<'_>,
	/// Retrieve the number of annotations on an objective
	pub objective_annotation_len: extern "C" fn(model: FznsoPtr<'_, FznsoModel>) -> usize,
	/// Retrieve the annotation on the objective at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `objective_annotation_len`.
	pub objective_annotation:
		extern "C" fn(model: FznsoPtr<'_, FznsoModel>, index: usize) -> FznsoAnnotationRef<'_>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// A model together with the callbacks that read it.
pub struct FznsoModelRef<'a> {
	/// The handle to the data of the model instance.
	pub data: FznsoPtr<'a, FznsoModel>,
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
	pub ident: FznsoStr<'a>,
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
	/// The number of elements in the `objectives` array.
	pub len: usize,
	/// An array of objective strategies.
	pub objectives: *const FznsoObjective<'a>,
	/// The lifetime of the `options` attribute.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// The definition of an option that is available to be set for the solver.
pub struct FznsoOption<'a> {
	/// The identifier used to set the option or get the current value of the
	/// option.
	pub ident: FznsoStr<'a>,
	/// The type of value that is expected for this option.
	pub arg_ty: FznsoType,
	/// The default value for this option.
	pub arg_def: FznsoValueRef<'a>,
	/// The lifetime of the `ident` and `arg_def` attributes.
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
/// The handle of a solution reported by a solver.
///
/// Opaque: only the side that created it may interpret it. Read it through
/// [`FznsoSolutionRef`], which pairs it with the callbacks that can.
pub struct FznsoSolution(pub ffi::c_void);

#[repr(C)]
#[derive(Clone, Debug)]
/// The callbacks that read a [`FznsoSolution`].
///
/// The values these return borrow from the solution, so they last only as long
/// as it does (i.e. for the duration of the `on_solution` callback that
/// delivered it). Copy out anything that has to outlive the call.
pub struct FznsoSolutionMethods {
	/// Function callback to retrieve the value assigned to a decision variable
	/// in the solution.
	pub value: extern "C" fn(
		data: FznsoPtr<'_, FznsoSolution>,
		decision_index: usize,
	) -> FznsoValueRef<'_>,
	/// Function callback to retrieve the statistical information made available
	/// by the solver about the search process so far.
	pub statistic: for<'a, 'i> extern "C" fn(
		data: FznsoPtr<'a, FznsoSolution>,
		ident: FznsoStr<'i>,
	) -> FznsoValueRef<'a>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// A solution together with the callbacks that read it.
pub struct FznsoSolutionRef<'a> {
	/// The data pointer to be used as the first argument in the function
	/// callbacks provided by the `methods` field.
	pub data: FznsoPtr<'a, FznsoSolution>,
	/// Reference to the structure containing the function callbacks used to
	/// interact with the solution.
	pub methods: &'static FznsoSolutionMethods,
}

#[repr(transparent)]
#[derive(Debug)]
/// The handle of a solver instance, as returned by `fznso_solver_create`.
///
/// Opaque: only the solver library may interpret it. An application passes it
/// back to the library's entry points and releases it with
/// `fznso_solver_free`.
pub struct FznsoSolver(pub ffi::c_void);

#[repr(C)]
#[derive(Clone, Debug)]
/// The definition of statistical information that is made available by the
/// solver.
///
/// The `solution` and `solver` flags say *where* the statistic can be read
/// from, and are independent: a statistic may be available from a solution,
/// from the solver instance, or from both. At least one of them must be set —
/// a statistic with neither could be read from nowhere.
pub struct FznsoStatistic<'a> {
	/// The identifier used to retrieve the statistical information from the
	/// solver or a solution.
	pub ident: FznsoStr<'a>,
	/// The type of value that is expected for this statistic.
	pub ty: FznsoType,
	/// Whether the statistic is available from a solution, read with
	/// [`FznsoSolutionMethods::statistic`].
	pub solution: bool,
	/// Whether the statistic is available from the solver instance, read with
	/// `fznso_solver_statistic`.
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
	/// An array of statistic definitions.
	pub stats: *const FznsoStatistic<'a>,
	/// The lifetime of the `stats` attribute.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// A borrowed UTF-8 string, passed across the interface as a pointer/length
/// pair.
///
/// The bytes are **not** required to be null-terminated; consumers must use
/// `len` to determine the string extent. This matches the convention used by
/// [`FznsoValueMethods::as_string`], and allows implementations to hand out
/// slices of strings they already hold without copying them into a
/// null-terminated buffer.
///
/// A null `ptr` signals the absence of a string, which is used where a name or
/// identifier is optional (e.g. [`FznsoModelMethods::decision_name`]).
pub struct FznsoStr<'a> {
	/// Pointer to the first UTF-8 byte of the string, or null if absent.
	pub ptr: *const ffi::c_char,
	/// The number of bytes in the string, **not** counting any trailing null
	/// byte.
	pub len: usize,
	/// The lifetime for which `ptr` stays valid.
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
	/// The number of elements in the `types` array.
	pub len: usize,
	/// An array of decision-variable types.
	pub types: *const FznsoType,
	/// The lifetime for which the `types` attribute is allocated.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(transparent)]
#[derive(Debug)]
/// The handle of a value: a constraint argument, a variable domain, a solution
/// assignment, an option or a statistic.
///
/// Opaque: only the side that created it may interpret it. Read it through
/// [`FznsoValueRef`], which pairs it with the callbacks that can.
///
/// This is the handle least likely to be a pointer at all: a small payload such
/// as a decision index is normally stored *in* the pointer word rather than
/// behind it, and the absent value carries a null. Dispatch on
/// [`FznsoValueMethods::kind`] and use the accessor it permits; never
/// dereference the handle itself.
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
/// The callbacks that read a [`FznsoValue`].
///
/// [`FznsoValueMethods::kind`] decides which of the others are meaningful; each
/// states the precondition it needs. A nested value or string that these return
/// borrows from the value it came from.
pub struct FznsoValueMethods {
	/// Function callback that returns the kind of the value.
	pub kind: extern "C" fn(FznsoPtr<'_, FznsoValue>) -> FznsoValueKind,
	/// Function callback that returns the length of the value.
	///
	/// In case [`FznsoValueMethods::kind`] returns
	/// [`FznsoValueKind::FznsoValueString`], the length is the number of bytes
	/// in the string **not** counting any trailing null byte.  The pointer
	/// returned by [`FznsoValueMethods::as_string`] is not required to be null-
	/// terminated; callers must use this length to determine the string extent.
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
	/// Requires [`FznsoValueMethods::kind`] to return one of
	/// [`FznsoValueKind::FznsoValueString`],
	/// [`FznsoValueKind::FznsoValueList`],
	/// [`FznsoValueKind::FznsoValueSetInt`], or
	/// [`FznsoValueKind::FznsoValueSetFloat`].
	pub len: extern "C" fn(FznsoPtr<'_, FznsoValue>) -> usize,
	/// Function callback that returns the decision variable index contained in
	/// the value.
	///
	/// Requires [`FznsoValueMethods::kind`] to return
	/// [`FznsoValueKind::FznsoValueDecision`].
	pub as_decision: extern "C" fn(FznsoPtr<'_, FznsoValue>) -> FznsoDecisionIdx,
	/// Function callback that returns the constraint index contained in the
	/// value.
	///
	/// Requires [`FznsoValueMethods::kind`] to return
	/// [`FznsoValueKind::FznsoValueConstraint`].
	pub as_constraint: extern "C" fn(FznsoPtr<'_, FznsoValue>) -> FznsoConstraintIdx,
	/// Function callback that returns the integer value contained in the value.
	///
	/// Requires [`FznsoValueMethods::kind`] to return
	/// [`FznsoValueKind::FznsoValueInt`].
	pub as_int: extern "C" fn(FznsoPtr<'_, FznsoValue>) -> i64,
	/// Function callback that returns the floating point value contained in the
	/// value.
	///
	/// Requires [`FznsoValueMethods::kind`] to return
	/// [`FznsoValueKind::FznsoValueFloat`].
	pub as_float: extern "C" fn(FznsoPtr<'_, FznsoValue>) -> f64,
	/// Function callback that returns a pointer to the UTF-8 bytes of a string
	/// value.  The pointer is valid for [`FznsoValueMethods::len`] bytes and is
	/// **not** required to be null-terminated (a trailing null byte may or may
	/// not be present).
	///
	/// Requires [`FznsoValueMethods::kind`] to return
	/// [`FznsoValueKind::FznsoValueString`].
	pub as_string: extern "C" fn(FznsoPtr<'_, FznsoValue>) -> *const ffi::c_char,
	/// Function callback that returns the Boolean value contained in the value.
	///
	/// Requires [`FznsoValueMethods::kind`] to return
	/// [`FznsoValueKind::FznsoValueBool`].
	pub as_bool: extern "C" fn(FznsoPtr<'_, FznsoValue>) -> bool,
	/// Function callback that returns a range from a range list representing
	/// the integer set contained in the value.
	///
	/// Requires [`FznsoValueMethods::kind`] to return
	/// [`FznsoValueKind::FznsoValueSetInt`], and `index` to be below
	/// [`FznsoValueMethods::len`].
	pub get_range_int: extern "C" fn(FznsoPtr<'_, FznsoValue>, index: usize) -> FznsoIntRange,
	/// Function callback that returns a range from a range list representing
	/// the floating point set contained in the value.
	///
	/// Requires [`FznsoValueMethods::kind`] to return
	/// [`FznsoValueKind::FznsoValueSetFloat`], and `index` to be below
	/// [`FznsoValueMethods::len`].
	pub get_range_float: extern "C" fn(FznsoPtr<'_, FznsoValue>, index: usize) -> FznsoFloatRange,
	/// Function callback that returns an element from the list contained in the
	/// value.
	///
	/// Requires [`FznsoValueMethods::kind`] to return
	/// [`FznsoValueKind::FznsoValueList`], and `index` to be below
	/// [`FznsoValueMethods::len`].
	pub get_element: extern "C" fn(FznsoPtr<'_, FznsoValue>, index: usize) -> FznsoValueRef<'_>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
/// A value together with the callbacks that read it.
pub struct FznsoValueRef<'a> {
	/// The data pointer to be the first argument of `get_value`.
	pub data: FznsoPtr<'a, FznsoValue>,
	/// Reference to the structure containing the function callbacks used to
	/// interact with the value.
	pub methods: &'static FznsoValueMethods,
}

#[cfg(test)]
mod tests {
	use crate::{
		FznsoConstraintIdx, FznsoDecisionIdx, FznsoModelRef, FznsoPtr, FznsoSolutionRef, FznsoStr,
		FznsoType, FznsoValueKind, FznsoValueRef,
	};

	/// The by-value conversions must not borrow their (stack-local) argument.
	#[test]
	fn by_value_conversions() {
		/// Force the source value out of scope before the reference is read.
		fn make<T: Into<FznsoValueRef<'static>>>(v: T) -> FznsoValueRef<'static> {
			v.into()
		}

		let t = make(true);
		assert_eq!(t.kind(), FznsoValueKind::FznsoValueBool);
		assert!(t.get_bool());

		let f = make(false);
		assert_eq!(f.kind(), FznsoValueKind::FznsoValueBool);
		assert!(!f.get_bool());

		let d = make(FznsoDecisionIdx(7));
		assert_eq!(d.kind(), FznsoValueKind::FznsoValueDecision);
		assert_eq!(d.get_decision(), FznsoDecisionIdx(7));

		let c = make(FznsoConstraintIdx(3));
		assert_eq!(c.kind(), FznsoValueKind::FznsoValueConstraint);
		assert_eq!(c.get_constraint(), FznsoConstraintIdx(3));
	}

	#[test]
	fn memory() {
		assert_eq!(size_of::<FznsoPtr<'_, ()>>(), size_of::<u64>());
		assert_eq!(size_of::<FznsoType>(), size_of::<u64>());
		assert_eq!(size_of::<FznsoStr>(), size_of::<u128>());
		assert_eq!(size_of::<FznsoModelRef>(), size_of::<u128>());
		assert_eq!(size_of::<FznsoValueRef>(), size_of::<u128>());
		assert_eq!(size_of::<FznsoSolutionRef>(), size_of::<u128>());
	}

	/// Scalars must report the kind that matches the getter they support.
	#[test]
	fn scalar_kinds() {
		assert_eq!(
			FznsoValueRef::from(&1.5_f64).kind(),
			FznsoValueKind::FznsoValueFloat
		);
		assert_eq!(FznsoValueRef::from(&1.5_f64).get_float(), 1.5);
		assert_eq!(
			FznsoValueRef::from(&1_i64).kind(),
			FznsoValueKind::FznsoValueInt
		);
		assert_eq!(
			FznsoValueRef::from(&()).kind(),
			FznsoValueKind::FznsoValueAbsent
		);
	}
}
