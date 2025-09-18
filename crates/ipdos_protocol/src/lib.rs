//! IPDOS Protocol
//!
//! This crate defines the IPDOS protocol, which allows the incremental usage of
//! solvers that can solve decision and optimization problems. The goal of the
//! interface is to easily use different solvers in a unified way.
#![allow(missing_debug_implementations)]

use std::{ffi, marker::PhantomData};

#[repr(C)]
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

#[repr(C)]
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
/// An interface to a model instance used to communicate with the solver.
///
/// The solver can use the included function callbacks to interact with the
/// model.
pub struct IpdosModel<'a> {
	/// The handle to the data of the model instance.
	///
	/// This handle is passed to all the different function callbacks to
	/// interact with the model.
	data: &'a IpdosModelData,
	/// Returns the current number of model layers currently contained in the
	/// model.
	///
	/// Note that it is required that the solver calls [`ipdos_solver_push`] and
	/// [`ipdos_solver_pop`] whenever a layer is added or removed.
	layers: extern "C" fn(model: &IpdosModelData) -> usize,

	/// Retrieve the number of constraints currently contained in the model.
	constraint_len: extern "C" fn(model: &IpdosModelData) -> usize,
	/// Retrieve the constraint index of the first new constraint in the layer.
	///
	/// This can be equivalent to `constraint_len` if the layer does not add any
	/// new constraints.
	constraint_layer_start: extern "C" fn(model: &IpdosModelData) -> usize,
	/// Retrieve the identifier of a constraint.
	///
	/// The returned pointer can be assumed to have the the same lifetime as the
	/// constraint reference and must be valid UTF8.
	constraint_ident:
		extern "C" fn(model: &IpdosModelData, constraint: usize) -> *const ffi::c_char,
	/// Retrieve the number of arguments of a constraint.
	constraint_argument_len: extern "C" fn(model: &IpdosModelData, constraint: usize) -> usize,
	/// Retrieve the value of the constraint's argument at the given index.
	///
	/// The `index` argument must be less than the value returned by
	/// `constraint_argument_len` for the given constraint.
	constraint_argument:
		extern "C" fn(model: &IpdosModelData, constraint: usize, index: usize) -> IpdosValue<'_>,

	/// Retrieve the number of decisions currently contained in the model.
	decision_len: extern "C" fn(model: &IpdosModelData) -> usize,
	/// Retrieve the decision index of the first new decision in the layer.
	///
	/// This can be equivalent to `decision_len` if the layer does not add any
	/// new decisions.
	decision_layer_start: extern "C" fn(model: &IpdosModelData) -> usize,
	/// Retrieve the domain of the given decision.
	///
	/// Note that the the value might have [`IpdosValueKind::IpdosValueAbsent`] if
	/// the decision variable does not have an explicit domain.
	decision_domain: extern "C" fn(model: &IpdosModelData, decision: usize) -> IpdosValue<'_>,
	/// Retrieve the name of a decision variable, if it exists.
	///
	/// Note that names are only available for debugging purposes. Decisions are
	/// identified using their index in the model. If the decision variable does
	/// not have a name, this function returns a null pointer.
	decision_name: extern "C" fn(model: &IpdosModelData, decision: usize) -> *const ffi::c_char,
	/// Check whether the decision variable is functionally defined by a
	/// constraint.
	decision_defined: extern "C" fn(model: &IpdosModelData, decision: usize) -> bool,

	/// Request the identifier of the type of objective strategy to be used when
	/// solving the model.
	///
	/// Note that the function can return a null pointer if the model does not
	/// have an objective strategy.
	objective_ident: extern "C" fn(model: &IpdosModelData) -> *const ffi::c_char,
	/// Retrieve the argument of the objective strategy.
	objective_arg: extern "C" fn(model: &IpdosModelData) -> IpdosValue<'_>,
}

#[repr(transparent)]
/// The handle to a the data of a model instance.
///
/// This type is opaque to the user. Only pointers of this type are ever used.
///
/// In implementation of the IPDOS interface, a pointer is generally cast to
/// this type, i.e. `(IpdosModelData*) my_model`. A similar cast can be used to
/// cast the pointer back to the original type, e.g. `(MyModel*) ipdos_model`.
pub struct IpdosModelData(ffi::c_void);

#[repr(C)]
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
/// The definition of an option that is available to be set for the solver.
pub struct IpdosOption<'a> {
	/// The identifier used to set the option or get the current value of the option.
	pub ident: *const ffi::c_char,
	/// The type of value that is expected for this option.
	pub arg_ty: IpdosValue<'a>,
	/// The default value for this option.
	pub arg_def: IpdosType,
	/// The lifetime of the `ident` attribute.
	pub lifetime: PhantomData<&'a ()>,
}

#[repr(C)]
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

#[repr(C)]
/// Structure used to represent a solution emitted by the solver.
///
/// The caller can use the `get_value` function to retrieve the value of the
/// used decision variables.
pub struct IpdosSolution<'a> {
	/// The data pointer to be the first argument of `get_value`.
	data: &'a IpdosSolutionData,
	/// Function callback to retrieve the value assigned to a decision variable
	/// in the solution.
	get_value: extern "C" fn(data: &IpdosSolutionData, decision_index: usize) -> IpdosValue<'_>,
}

#[repr(transparent)]
/// The handle to a the data of a solution instance.
///
/// This type is opaque to the user. Only pointers of this type are ever used.
///
/// In implementation of the IPDOS interface, a pointer is generally cast to
/// this type, i.e. `(IpdosSolutionData*) my_solution`. A similar cast can be
/// used to cast the pointer back to the original type, e.g. `(MySolution*)
/// ipdos_solution`.
pub struct IpdosSolutionData(ffi::c_void);

#[repr(transparent)]
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
	/// `ipdos_solver_read_error` can be used to retrieve the error message.
	IpdosError,
}

#[repr(C)]
/// Representation of a type to signal and check whether an argument takes the
/// correct type.
pub struct IpdosType {
	/// Whether the type is a list of values.
	pub list_of: bool,
	/// Whether the argument can be or contain decision variables (represented as
	/// decision indexes).
	pub decision: bool,
	/// Whether expected type is an set of values of the base type.
	pub set_of: bool,
	/// The expected base type of the argument.
	pub base: IpdosTypeBase,
}

#[repr(C)]
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

#[repr(C)]
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
/// The storage of the value content of a [`IpdosValue`].
pub union IpdosValueContent<'a> {
	/// Decision index storage
	pub decision_index: usize,
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

#[unsafe(no_mangle)]
/// Returns the list of available constraint that can be added to the solver.
pub extern "C" fn ipdos_constraint_list() -> IpdosConstraintList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of types for which decision variable can be created by the
/// solver.
pub extern "C" fn ipdos_decision_list() -> IpdosTypeList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of available objective that can be achieved by the solver.
pub extern "C" fn ipdos_objective_list() -> IpdosObjectiveList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Returns the list of available options that can be set of the solver.
pub extern "C" fn ipdos_option_list() -> IpdosOptionList<'static> {
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Create a new solver instance
pub extern "C" fn ipdos_solver_create() -> Box<IpdosSolver> {
	unimplemented!()
}

/// Free a solver instance, releasing all resources associated with it.
///
/// The pointer to the solver instance will be invalid after this function has
/// been called.
#[unsafe(no_mangle)]
pub extern "C" fn ipdos_solver_free(solver: Box<IpdosSolver>) {
	let _ = solver;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Get the current value of an option for the solver.
///
/// Note that this is only valid to be called with options named by
/// `ipdos_option_list`.
pub extern "C" fn ipdos_solver_option_get(
	solver: &IpdosSolver,
	ident: *const ffi::c_char,
) -> IpdosValue<'_> {
	let _ = solver;
	let _ = ident;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Set the current value of an option for the solver.
///
/// Note that this is only valid to be called with options named by
/// `ipdos_option_list`, and the value must be of the correct type.
pub extern "C" fn ipdos_solver_option_set(
	solver: &mut IpdosSolver,
	ident: *const ffi::c_char,
	value: IpdosValue<'_>,
) -> bool {
	let _ = solver;
	let _ = ident;
	let _ = value;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Remove a model layer from the solver.
///
/// It is required that `model.layers` is less than the number of layers in the
/// last call to `ipdos_solver_push` or `ipdos_solver_pop`. It is allowed to
/// remove multiple model layers at the same time.
pub extern "C" fn ipdos_solver_pop(solver: &mut IpdosSolver, model: IpdosModel) {
	let _ = solver;
	let _ = model;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Add a new model layer to the solver.
///
/// It is required that `model.layers` is exactly one larger than the previous
/// call to `ipdos_solver_push` or `ipdos_solver_pop`. It is NOT allowed to add
/// multiple model layers at the same time.
pub extern "C" fn ipdos_solver_push(solver: &mut IpdosSolver, model: IpdosModel) -> bool {
	let _ = solver;
	let _ = model;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Read an error message from the solver.
///
/// This function is expected to be called after solver interactions signal an
/// error has occurred. For example, if [`ipdos_solver_run`] returns
/// [`IpdosError`] or [`ipdos_solver_push`] returns `false`.
pub extern "C" fn ipdos_solver_read_error(
	solver: &mut IpdosSolver,
	context: &mut ffi::c_void,
	read_error: extern "C" fn(context: &mut ffi::c_void, error: *const ffi::c_char),
) {
	let _ = solver;
	let _ = context;
	let _ = read_error;
	unimplemented!()
}

#[unsafe(no_mangle)]
/// Run the solver with the given model
pub extern "C" fn ipdos_solver_run(
	solver: &mut IpdosSolver,
	context: &mut ffi::c_void,
	on_solution: extern "C" fn(context: &mut ffi::c_void, solution: &IpdosSolution),
) -> IpdosStatus {
	let _ = solver;
	let _ = context;
	let _ = on_solution;
	unimplemented!()
}

#[cfg(test)]
mod tests {
	use crate::{IpdosType, IpdosValue};

	#[test]
	fn memory() {
		assert_eq!(size_of::<IpdosValue>(), size_of::<u128>());
		assert_eq!(size_of::<IpdosType>(), size_of::<u64>());
	}
}
