#ifndef ipdos_types_h
#define ipdos_types_h

#include <stdbool>
#include <stdint>

// The status returned by `ipdos_solver_run`, indicating whether the solver
// completed its search.
typedef enum IpdosStatus {
  // The solver explored the full search space and yielded all relevant
  // solutions.
  IpdosComplete,
  // The solver did not explore the full search space due to a timeout or
  // other termination condition. Additional (better) solutions might be
  // possible.
  IpdosIncomplete,
  // An error occurred during the solver's execution.
  //
  // [`ipdos_solver_read_error`] can be used to retrieve the error message.
  IpdosError,
} IpdosStatus;

// Representation of the base type of a value.
typedef enum IpdosTypeBase {
  // Boolean type
  IpdosTypeBaseBool,
  // Integer numeric type
  IpdosTypeBaseInt,
  // Floating point numeric type
  IpdosTypeBaseFloat,
  // Character string type
  IpdosTypeBaseString,
} IpdosTypeBase;

// Enumerated type used to mark the kind of [`IpdosValue`]. This is used to
// determine which field in the [`IpdosValueContent`] union to access.
typedef enum IpdosValueKind {
  // No value is stored.
  IpdosValueAbsent,
  // The value is stored in `decision_index`.
  IpdosValueDecision,
  // The value is stored in `boolean_value`.
  IpdosValueBoolean,
  // The value is stored in `integer_value`.
  IpdosValueInteger,
  // The value is stored in `float_value`.
  IpdosValueFloat,
  // The value is stored in `string_value`.
  IpdosValueString,
  // The value is stored in `list_value`.
  IpdosValueList,
} IpdosValueKind;

// The handle to a the data of a solution instance.
//
// This type is opaque to the user. Only pointers of this type are ever used.
//
// In implementation of the IPDOS interface, a pointer is generally cast to
// this type, i.e. `(IpdosSolutionData*) my_solution`. A similar cast can be
// used to cast the pointer back to the original type, e.g. `(MySolution*)
// ipdos_solution`.
typedef void IpdosAnnotation;

// Representation of a type to signal and check whether an argument takes the
// correct type.
typedef struct IpdosType {
  // Whether the type is a list of values.
  bool list_of;
  // Whether the argument can be or contain decision variables (represented
  // as decision indexes).
  bool decision;
  // Whether expected type is an set of values of the base type.
  bool set_of;
  // Whether the expected type is optional (and can take the value of
  // [`IpdosValueKind::IpdosValueAbsent`]).
  bool opt;
  // The expected base type of the argument.
  enum IpdosTypeBase base;
} IpdosType;

// Representation of a type of constraint, discerned by its identifier and the
// types of its arguments.
typedef struct IpdosConstraintType {
  // The identifier of the constraint type.
  const char *ident;
  // The number of expected arguments for the constraint type.
  uint64_t arg_len;
  // The types of the expected arguments for the constraint type.
  const struct IpdosType *arg_types;
} IpdosConstraintType;

// A list of [`IpdosConstraintType`]s.
//
// This type is for example used to return from [`ipdos_constraint_list`].
typedef struct IpdosConstraintList {
  // The number of elements in the `constraints` array.
  uint64_t len;
  // An array of constraint types.
  const struct IpdosConstraintType *constraints;
} IpdosConstraintList;

// The handle of a model instance.
//
// This type is opaque to the user. Only pointers of this type are ever used.
//
// In implementation of the IPDOS interface, a pointer is generally cast to
// this type, i.e. `(IpdosModelData*) my_model`. A similar cast can be used to
// cast the pointer back to the original type, e.g. `(MyModel*) ipdos_model`.
typedef void IpdosModel;

// The storage of the value content of a [`IpdosValue`].
typedef union IpdosValueContent {
  // Decision index storage
  uint64_t decision_index;
  // Boolean value storage
  bool bool_value;
  // Integer value storage
  int64_t integer_value;
  // Float value storage
  double float_value;
  // Set of integers value storage
  //
  // The set of integers is represented using a range list. The array of
  // `i64` values should be interpreted as a list of inclusive ranges,
  // where each range is represented by two values.
  //
  // Note that the number of `i64` values is stored in the `len` field. Since
  // each range is represented by two values, it can be assumed that `len mod
  // 2 == 0`.
  const int64_t *set_of_int_value;
  // Set of floating point value storage
  //
  // The set of floating point values is represented using a range list. The
  // array of `f64` values should be interpreted as a list of inclusive
  // ranges, where each range is represented by two values.
  //
  // Note that the number of `f64` values is stored in the `len` field. Since
  // each range is represented by two values, it can be assumed that `len mod
  // 2 == 0`.
  const int64_t *set_of_float_value;
  // String value storage
  //
  // Note that the `string_value` field is intended to be interpreted as a
  // C-string. It should be \0 terminated, use UTF-8 encoding, and be valid
  // for the lifetime of the value.
  const char *string_value;
  // List value storage
  //
  // Note that the the number of elements in the list is stored in the `len`
  // field.
  const struct IpdosValue *list_value;
} IpdosValueContent;

// The value representation used for values assigned to decision variables in
// solution, as constraint/annotation arguments, and option parameters.
typedef struct IpdosValue {
  // The kind of the value
  //
  // This field is used to determine what field in the `value` union is
  // allowed to be accessed.
  enum IpdosValueKind kind;
  // A field containing the size of the value.
  //
  // This field is used when the value is a list, set of float/int, and
  // string, to determine the number of elements in the C array type.
  uint32_t len;
  // The storage of the actual data of the value.
  union IpdosValueContent content;
} IpdosValue;

// A struct containing the function pointers to interact with theIpdosModel`
typedef struct IpdosModelMethods {
  // Returns the current number of model layers currently contained in the
  // model.
  //
  // Layers provides a way for the modelling user to add and retract
  uint64_t (*layer_len)(const IpdosModel *model);
  // Returns the number of layers that have been unchanged since the last
  // call to [`ipdos_solver_run`].
  //
  // Unchanged layers must be consecutive starting from layer 0.
  uint64_t (*layer_unchanged)(const IpdosModel *model);
  // Returns the number of permanent layers in the model.
  //
  // Permanent layers are laid out must be consecutive starting from layer 0.
  // Under no circumstances can they be retracted. As such, the solver can
  // assume that the number of permanent layers only ever increases.
  //
  // Permanent layers can, however, still be found to be redundant, allowing
  // the solver to remove the decision variables and constraints in these
  // layers, if convenient. This is signaled using [`layer_redundant_len`]
  // and [`layer_redundant_index`].
  uint64_t (*layer_permanent)(const IpdosModel *model);
  // Returns the number of permanent layers that have been marked as
  // redundant.
  //
  // The solver can remove the decision variables and constraints in these
  // layers, if convenient. Once a layer is marked as redundant, it can
  // forever be considered redundant. As such, the solver can assume that
  // the number of redundant layers only ever increases.
  uint64_t (*layer_redundant_len)(const IpdosModel *model);
  // Returns the index of the n-th permanent layer that has been marked as
  // redundant.
  //
  // The `n` argument must be less than the value returned by
  // [`layer_redundant_len`].
  uint64_t (*layer_redundant_index)(const IpdosModel *model, uint64_t n);
  // Retrieve the number of decisions currently contained in the model.
  uint64_t (*decision_len)(const IpdosModel *model);
  // Retrieve the decision index of the last decision variable in the layer.
  //
  // This can be equivalent to `decision_layer_end` of `layer-1` if the layer
  // does not add any new decisions.
  uint64_t (*decision_layer_end)(const IpdosModel *model, uint64_t layer);
  // Retrieve the domain of the given decision.
  //
  // Note that the the value might have [`IpdosValueKind::IpdosValueAbsent`]
  // if the decision variable does not have an explicit domain.
  struct IpdosValue (*decision_domain)(const IpdosModel *model, uint64_t decision);
  // Retrieve the name of a decision variable, if it exists.
  //
  // Note that names are only available for debugging purposes. Decisions are
  // identified using their index in the model. If the decision variable does
  // not have a name, this function returns a null pointer.
  const char *(*decision_name)(const IpdosModel *model, uint64_t decision);
  // Check whether the decision variable is defined by a constraint.
  bool (*decision_defined)(const IpdosModel *model, uint64_t decision);
  // Retrieve the number of annotations of a constraint.
  uint64_t (*decision_annotation_len)(const IpdosModel *model, uint64_t decision);
  // Retrieve the value of the constraint's annotation at the given index.
  //
  // The `index` argument must be less than the value returned by
  // [`decision_annotation_len`] for the given decision variable.
  const IpdosAnnotation *(*decision_annotation)(const IpdosModel *model,
                                                uint64_t decision,
                                                uint64_t index);
  // Retrieve the number of constraints currently contained in the model.
  uint64_t (*constraint_len)(const IpdosModel *model);
  // Retrieve the constraint index of the first new constraint in the layer.
  //
  // This can be equivalent to `constraint_len` if the layer does not add any
  // new constraints.
  uint64_t (*constraint_layer_end)(const IpdosModel *model);
  // Retrieve the identifier of a constraint.
  //
  // The returned pointer can be assumed to have the the same lifetime as the
  // model reference and must be valid UTF8.
  const char *(*constraint_ident)(const IpdosModel *model, uint64_t constraint);
  // Retrieve the number of arguments of a constraint.
  uint64_t (*constraint_argument_len)(const IpdosModel *model, uint64_t constraint);
  // Retrieve the value of the constraint's argument at the given index.
  //
  // The `index` argument must be less than the value returned by
  // `constraint_argument_len` for the given constraint.
  struct IpdosValue (*constraint_argument)(const IpdosModel *model,
                                           uint64_t constraint,
                                           uint64_t index);
  // Check whether the decision variable is functionally defined by a
  // constraint.
  //
  // This function returns a [`IpdosValue`] that is either
  // [`IpdosValueKind::IpdosValueDecision`] if it defined a decision variable
  // or [`IpdosValueKind::IpdosValueAbsent`] otherwise.
  struct IpdosValue (*constraint_defines)(const IpdosModel *model, uint64_t decision);
  // Retrieve the number of annotations on a constraint.
  uint64_t (*constraint_annotation_len)(const IpdosModel *model, uint64_t constraint);
  // Retrieve the annotation on a constraint at the given index.
  //
  // The `index` argument must be less than the value returned by
  // `constraint_annotation_len` for the given constraint.
  const IpdosAnnotation *(*constraint_annotation)(const IpdosModel *model,
                                                  uint64_t constraint,
                                                  uint64_t index);
  // Request the identifier of the type of objective strategy to be used when
  // solving the model.
  //
  // Note that the function can return a null pointer if the model does not
  // have an objective strategy.
  const char *(*objective_ident)(const IpdosModel *model);
  // Retrieve the argument of the objective strategy.
  struct IpdosValue (*objective_arg)(const IpdosModel *model);
  // Retrieve the number of annotations on an objective
  uint64_t (*objective_annotation_len)(const IpdosModel *model);
  // Retrieve the annotation on the objective at the given index.
  //
  // The `index` argument must be less than the value returned by
  // `objective_annotation_len`.
  const IpdosAnnotation *(*objective_annotation)(const IpdosModel *model, uint64_t index);
  // Retrieve the identifier of an annotation.
  //
  // The returned pointer can be assumed to have the the same lifetime as the
  // annotation reference and must be valid UTF8.
  const char *(*annotation_ident)(const IpdosAnnotation *ann);
  // Retrieve the number of arguments of an annotation.
  uint64_t (*annotation_argument_len)(const IpdosAnnotation *ann);
  // Retrieve the value of the annotation's argument at the given index.
  //
  // The `index` argument must be less than the value returned by
  // `annotation_argument_len` for the given annotation.
  struct IpdosValue (*annotation_argument)(const IpdosAnnotation *ann, uint64_t index);
} IpdosModelMethods;

// An interface to a model instance used to communicate with the solver.
//
// The solver can use the included function callbacks to interact with the
// model.
typedef struct IpdosModelRef {
  // The handle to the data of the model instance.
  const IpdosModel *data;
  // Reference to the structure containing the function callbacks used to
  // interact with the model.
  const struct IpdosModelMethods *methods;
} IpdosModelRef;

// Representation of a type of objective strategies, discerned by its
// identifier and the type of its argument.
typedef struct IpdosObjective {
  // The identifier of the objective type.
  const char *ident;
  // The type of the expected argument for the constraint type.
  struct IpdosType arg_type;
} IpdosObjective;

// A list of [`IpdosObjective`]s.
//
// This type is, for example, used to return from [`ipdos_objective_list`].
typedef struct IpdosObjectiveList {
  // The number of elements in the `options` array.
  uint64_t len;
  // An array of option definitions.
  const struct IpdosObjective *options;
} IpdosObjectiveList;

// The definition of an option that is available to be set for the solver.
typedef struct IpdosOption {
  // The identifier used to set the option or get the current value of the
  // option.
  const char *ident;
  // The type of value that is expected for this option.
  struct IpdosValue arg_ty;
  // The default value for this option.
  struct IpdosType arg_def;
} IpdosOption;

// A list of [`IpdosOption`]s.
//
// This type is, for example, used to return from [`ipdos_option_list`].
typedef struct IpdosOptionList {
  // The number of elements in the `options` array.
  uint64_t len;
  // An array of option definitions.
  const struct IpdosOption *options;
} IpdosOptionList;

// The handle to a the data of a solution instance.
//
// This type is opaque to the user. Only pointers of this type are ever used.
//
// In implementation of the IPDOS interface, a pointer is generally cast to
// this type, i.e. `(IpdosSolutionData*) my_solution`. A similar cast can be
// used to cast the pointer back to the original type, e.g. `(MySolution*)
// ipdos_solution`.
typedef void IpdosSolution;

// Structure used to represent a solution emitted by the solver.
//
// The caller can use the `get_value` function to retrieve the value of the
// used decision variables.
typedef struct IpdosSolutionRef {
  // The data pointer to be the first argument of `get_value`.
  const IpdosSolution *data;
  // Function callback to retrieve the value assigned to a decision variable
  // in the solution.
  struct IpdosValue (*get_value)(const IpdosSolution *data, uint64_t decision_index);
} IpdosSolutionRef;

// The handle to a solver instance.
//
// This type is opaque to the user. Only pointers of this type are ever used.
//
// In implementation of the IPDOS interface, a pointer is generally cast to
// this type, i.e. `(IpdosSolver*) my_solver`. A similar cast can be used to
// cast the pointer back to the original type, e.g. `(MySolverType*)
// ipdos_solver`.
typedef void IpdosSolver;

// A list of [`IpdosType`]s.
//
// This type is for example used to return from [`ipdos_decision_list`].
typedef struct IpdosTypeList {
  // The number of elements in the `constraints` array.
  uint64_t len;
  // An array of constraint types.
  const struct IpdosType *types;
} IpdosTypeList;

#endif  /* ipdos_types_h */
