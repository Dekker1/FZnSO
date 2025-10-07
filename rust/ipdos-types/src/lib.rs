//! IPDOS Protocol Types
//!
//! This crate defines the types of the IPDOS protocol, which allows the
//! incremental usage of solvers that can solve decision and optimization
//! problems. The goal of the interface is to easily use different solvers in a
//! unified way, and to allow the dynamic loading of solver libraries (as DLLs).

#![no_std]
use core::{ffi, fmt, marker::PhantomData, ptr::slice_from_raw_parts};

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

#[repr(C)]
#[derive(Clone, Debug)]
/// Representation of a type of constraint, discerned by its identifier and the
/// types of its arguments.
pub struct IpdosConstraintType<'a> {
	/// The identifier of the constraint type.
	pub ident: *const ffi::c_char,
	/// The number of expected arguments for the constraint type.
	pub arg_len: u64,
	/// The types of the expected arguments for the constraint type.
	pub arg_types: *const IpdosType,
	/// The lifetime for which the `ident` and `arg_types` attributes are
	/// allocated.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// A list of [`IpdosConstraintType`]s.
///
/// This type is for example used to return from [`ipdos_constraint_list`].
pub struct IpdosConstraintList<'a> {
	/// The number of elements in the `constraints` array.
	pub len: u64,
	/// An array of constraint types.
	pub constraints: *const IpdosConstraintType<'a>,
	/// The lifetime for which the `constraints` attribute is allocated.
	pub lifetime: PhantomData<&'a ()>,
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
/// A struct containing the function pointers to interact with theIpdosModel`
pub struct IpdosModelMethods {
	/// Returns the current number of model layers currently contained in the
	/// model.
	///
	/// Layers provides a way for the modelling user to add and retract
	layer_len: extern "C" fn(model: &IpdosModel) -> u64,
	/// Returns the number of layers that have been unchanged since the last
	/// call to [`ipdos_solver_run`].
	///
	/// Unchanged layers must be consecutive starting from layer 0.
	layer_unchanged: extern "C" fn(model: &IpdosModel) -> u64,
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
	layer_permanent: extern "C" fn(model: &IpdosModel) -> u64,
	/// Returns the number of permanent layers that have been marked as
	/// redundant.
	///
	/// The solver can remove the decision variables and constraints in these
	/// layers, if convenient. Once a layer is marked as redundant, it can
	/// forever be considered redundant. As such, the solver can assume that
	/// the number of redundant layers only ever increases.
	layer_redundant_len: extern "C" fn(model: &IpdosModel) -> u64,
	/// Returns the index of the n-th permanent layer that has been marked as
	/// redundant.
	///
	/// The `n` argument must be less than the value returned by
	/// [`layer_redundant_len`].
	layer_redundant_index: extern "C" fn(model: &IpdosModel, n: u64) -> u64,

	/// Retrieve the number of decisions currently contained in the model.
	decision_len: extern "C" fn(model: &IpdosModel) -> u64,
	/// Retrieve the decision index of the last decision variable in the layer.
	///
	/// This can be equivalent to `decision_layer_end` of `layer-1` if the layer
	/// does not add any new decisions.
	decision_layer_end: extern "C" fn(model: &IpdosModel, layer: u64) -> u64,
	/// Retrieve the domain of the given decision.
	///
	/// Note that the the value might have [`IpdosValueKind::IpdosValueAbsent`]
	/// if the decision variable does not have an explicit domain.
	decision_domain: extern "C" fn(model: &IpdosModel, decision: u64) -> IpdosValue<'_>,
	/// Retrieve the name of a decision variable, if it exists.
	///
	/// Note that names are only available for debugging purposes. Decisions are
	/// identified using their index in the model. If the decision variable does
	/// not have a name, this function returns a null pointer.
	decision_name: extern "C" fn(model: &IpdosModel, decision: u64) -> *const ffi::c_char,
	/// Check whether the decision variable is defined by a constraint.
	decision_defined: extern "C" fn(model: &IpdosModel, decision: u64) -> bool,
	/// Retrieve the number of annotations of a constraint.
	decision_annotation_len: extern "C" fn(model: &IpdosModel, decision: u64) -> u64,
	/// Retrieve the value of the constraint's annotation at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// [`decision_annotation_len`] for the given decision variable.
	decision_annotation:
		extern "C" fn(model: &IpdosModel, decision: u64, index: u64) -> &IpdosAnnotation,

	/// Retrieve the number of constraints currently contained in the model.
	constraint_len: extern "C" fn(model: &IpdosModel) -> u64,
	/// Retrieve the constraint index of the first new constraint in the layer.
	///
	/// This can be equivalent to `constraint_len` if the layer does not add any
	/// new constraints.
	constraint_layer_end: extern "C" fn(model: &IpdosModel) -> u64,
	/// Retrieve the identifier of a constraint.
	///
	/// The returned pointer can be assumed to have the the same lifetime as the
	/// model reference and must be valid UTF8.
	constraint_ident: extern "C" fn(model: &IpdosModel, constraint: u64) -> *const ffi::c_char,
	/// Retrieve the number of arguments of a constraint.
	constraint_argument_len: extern "C" fn(model: &IpdosModel, constraint: u64) -> u64,
	/// Retrieve the value of the constraint's argument at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `constraint_argument_len` for the given constraint.
	constraint_argument:
		extern "C" fn(model: &IpdosModel, constraint: u64, index: u64) -> IpdosValue<'_>,
	/// Check whether the decision variable is functionally defined by a
	/// constraint.
	///
	/// This function returns a [`IpdosValue`] that is either
	/// [`IpdosValueKind::IpdosValueDecision`] if it defined a decision variable
	/// or [`IpdosValueKind::IpdosValueAbsent`] otherwise.
	constraint_defines: extern "C" fn(model: &IpdosModel, decision: u64) -> IpdosValue<'_>,
	/// Retrieve the number of annotations on a constraint.
	constraint_annotation_len: extern "C" fn(model: &IpdosModel, constraint: u64) -> u64,
	/// Retrieve the annotation on a constraint at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `constraint_annotation_len` for the given constraint.
	constraint_annotation:
		extern "C" fn(model: &IpdosModel, constraint: u64, index: u64) -> &IpdosAnnotation,

	/// Request the identifier of the type of objective strategy to be used when
	/// solving the model.
	///
	/// Note that the function can return a null pointer if the model does not
	/// have an objective strategy.
	objective_ident: extern "C" fn(model: &IpdosModel) -> *const ffi::c_char,
	/// Retrieve the argument of the objective strategy.
	objective_arg: extern "C" fn(model: &IpdosModel) -> IpdosValue<'_>,
	/// Retrieve the number of annotations on an objective
	objective_annotation_len: extern "C" fn(model: &IpdosModel) -> u64,
	/// Retrieve the annotation on the objective at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `objective_annotation_len`.
	objective_annotation: extern "C" fn(model: &IpdosModel, index: u64) -> &IpdosAnnotation,

	/// Retrieve the identifier of an annotation.
	///
	/// The returned pointer can be assumed to have the the same lifetime as the
	/// annotation reference and must be valid UTF8.
	annotation_ident: extern "C" fn(ann: &IpdosAnnotation) -> *const ffi::c_char,
	/// Retrieve the number of arguments of an annotation.
	annotation_argument_len: extern "C" fn(ann: &IpdosAnnotation) -> u64,
	/// Retrieve the value of the annotation's argument at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `annotation_argument_len` for the given annotation.
	annotation_argument: extern "C" fn(ann: &IpdosAnnotation, index: u64) -> IpdosValue<'_>,
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
	pub len: u64,
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
	pub arg_ty: IpdosValue<'a>,
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
	pub len: u64,
	/// An array of option definitions.
	pub options: *const IpdosOption<'a>,
	/// The lifetime of the `options` attribute.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Debug)]
/// Structure used to represent a solution emitted by the solver.
///
/// The caller can use the `get_value` function to retrieve the value of the
/// used decision variables.
pub struct IpdosSolutionRef<'a> {
	/// The data pointer to be the first argument of `get_value`.
	data: &'a IpdosSolution,
	/// Function callback to retrieve the value assigned to a decision variable
	/// in the solution.
	get_value: extern "C" fn(data: &IpdosSolution, decision_index: u64) -> IpdosValue<'_>,
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
	pub len: u64,
	/// An array of constraint types.
	pub types: *const IpdosType,
	/// The lifetime for which the `types` attribute is allocated.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// The value representation used for values assigned to decision variables in
/// solution, as constraint/annotation arguments, and option parameters.
pub struct IpdosValue<'a> {
	/// The kind of the value
	///
	/// This field is used to determine what field in the `value` union is
	/// allowed to be accessed.
	pub kind: IpdosValueKind,
	/// A field containing the size of the value.
	///
	/// This field is used when the value is a list, set of float/int, and
	/// string, to determine the number of elements in the C array type.
	pub len: u32,
	/// The storage of the actual data of the value.
	pub content: IpdosValueContent<'a>,
	/// Lifetime of the value (only relevant in Rust).
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// The storage of the value content of a [`IpdosValue`].
pub union IpdosValueContent<'a> {
	/// Decision index storage
	pub decision_index: u64,
	/// Boolean value storage
	pub bool_value: bool,
	/// Integer value storage
	pub integer_value: i64,
	/// Float value storage
	pub float_value: f64,
	/// Set of integers value storage
	///
	/// The set of integers is represented using a range list. The array of
	/// `i64` values should be interpreted as a list of inclusive ranges,
	/// where each range is represented by two values.
	///
	/// Note that the number of `i64` values is stored in the `len` field. Since
	/// each range is represented by two values, it can be assumed that `len mod
	/// 2 == 0`.
	pub set_of_int_value: *const i64,
	/// Set of floating point value storage
	///
	/// The set of floating point values is represented using a range list. The
	/// array of `f64` values should be interpreted as a list of inclusive
	/// ranges, where each range is represented by two values.
	///
	/// Note that the number of `f64` values is stored in the `len` field. Since
	/// each range is represented by two values, it can be assumed that `len mod
	/// 2 == 0`.
	pub set_of_float_value: *const i64,
	/// String value storage
	///
	/// Note that the `string_value` field is intended to be interpreted as a
	/// C-string. It should be \0 terminated, use UTF-8 encoding, and be valid
	/// for the lifetime of the value.
	pub string_value: *const ffi::c_char,
	/// List value storage
	///
	/// Note that the the number of elements in the list is stored in the `len`
	/// field.
	pub list_value: *const IpdosValue<'a>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Enumerated type used to mark the kind of [`IpdosValue`]. This is used to
/// determine which field in the [`IpdosValueContent`] union to access.
pub enum IpdosValueKind {
	/// No value is stored.
	IpdosValueAbsent,
	/// The value is stored in `decision_index`.
	IpdosValueDecision,
	/// The value is stored in `boolean_value`.
	IpdosValueBoolean,
	/// The value is stored in `integer_value`.
	IpdosValueInteger,
	/// The value is stored in `float_value`.
	IpdosValueFloat,
	/// The value is stored in `string_value`.
	IpdosValueString,
	/// The value is stored in `list_value`.
	IpdosValueList,
}

impl fmt::Debug for IpdosValue<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("IpdosValue")
			.field("kind", &self.kind)
			.field("len", &self.len)
			// Safety: Access `self.content` according to the type information
			// provided explicitly by `self.kind`.
			.field("content", unsafe {
				match self.kind {
					IpdosValueKind::IpdosValueAbsent => &(),
					IpdosValueKind::IpdosValueDecision => &self.content.decision_index,
					IpdosValueKind::IpdosValueBoolean => &self.content.bool_value,
					IpdosValueKind::IpdosValueInteger => &self.content.integer_value,
					IpdosValueKind::IpdosValueFloat => &self.content.float_value,
					IpdosValueKind::IpdosValueString => &self.content.string_value,
					IpdosValueKind::IpdosValueList => &self.content.list_value,
				}
			})
			.field("lifetime", &self.lifetime)
			.finish()
	}
}

impl PartialEq for IpdosValue<'_> {
	fn eq(&self, other: &Self) -> bool {
		self.kind == other.kind
		// Safety: Access `self.content` and `other.content` according to the type
		// information provided explicitly by `self.kind` and `other.kind`.
			&& unsafe {
				match self.kind {
					IpdosValueKind::IpdosValueAbsent => true,
					IpdosValueKind::IpdosValueDecision => {
						self.content.decision_index == other.content.decision_index
					}
					IpdosValueKind::IpdosValueBoolean => {
						self.content.bool_value == other.content.bool_value
					}
					IpdosValueKind::IpdosValueInteger => {
						self.content.integer_value == other.content.integer_value
					}
					IpdosValueKind::IpdosValueFloat => {
						self.content.float_value == other.content.float_value
					}
					IpdosValueKind::IpdosValueString => {
						if self.len != other.len {
							return false;
						}
						if self.len == 0 {
							return true;
						}
						let self_str =
							&*slice_from_raw_parts(self.content.string_value, self.len as usize);
						let other_str =
							&*slice_from_raw_parts(other.content.string_value, other.len as usize);
						self_str == other_str
					}
					IpdosValueKind::IpdosValueList => {
						if self.len != other.len {
							return false;
						}
						let self_li =
							&*slice_from_raw_parts(self.content.list_value, self.len as usize);
						let other_li =
							&*slice_from_raw_parts(other.content.list_value, other.len as usize);
						self_li == other_li
					}
				}
			}
	}
}

impl fmt::Debug for IpdosValueContent<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// Safety: interprets the storage as raw 64 bits, no type information
		// necessary.
		write!(f, "{:#X}", unsafe { self.decision_index })
	}
}

#[cfg(test)]
mod tests {
	use crate::{IpdosModelRef, IpdosSolutionRef, IpdosType, IpdosValue};

	#[test]
	fn memory() {
		assert_eq!(size_of::<IpdosModelRef>(), size_of::<u128>());
		assert_eq!(size_of::<IpdosSolutionRef>(), size_of::<u128>());
		assert_eq!(size_of::<IpdosType>(), size_of::<u64>());
		assert_eq!(size_of::<IpdosValue>(), size_of::<u128>());
	}
}
