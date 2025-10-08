#ifndef ipdos_types_h
#define ipdos_types_h

#include <stdbool.h>
#include <stdint.h>

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
// determine which "get" method in [`IpdosValueMethods`] is safe to call.
typedef enum IpdosValueKind {
  // No value is available.
  IpdosValueAbsent,
  // The value is available using [`IpdosValueMethods::get_decision`].
  IpdosValueDecision,
  // The value is available using [`IpdosValueMethods::get_bool`].
  IpdosValueBool,
  // The value is available using [`IpdosValueMethods::get_int`].
  IpdosValueInt,
  // The value is available using [`IpdosValueMethods::get_float`].
  IpdosValueFloat,
  // The value is available using [`IpdosValueMethods::get_string`].
  IpdosValueString,
  // Sets of integers are represented using a range list. The number of
  // ranges is available using [`IpdosValueMethods::len`], and the ranges can
  // be accessed using [`IpdosValueMethods::get_range_int`].
  IpdosValueSetInt,
  // Sets of floats are represented using a range list. The number of
  // ranges is available using [`IpdosValueMethods::len`], and the ranges can
  // be accessed using [`IpdosValueMethods::get_range_float`].
  IpdosValueSetFloat,
  // The length of the list can be accessed using
  // [`IpdosValueMethods::len`], and elements in the list can be
  // accessed using [`IpdosValueMethods::get_element`]
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

// The type of a value used as an argument or solution assignment.
//
// This type is opaque to the user. Only pointers of this type are ever used.
//
// In implementation of the IPDOS interface, a pointer is generally cast to
// this type, e.g. `(IpdosValue*) my_value`. A similar cast can be used to cast
// the pointer back to the original type, e.g. `(MyValue*) ipdos_value`.
typedef void IpdosValue;

// A struct containing the function pointers to interact with a
// [`IpdosValue`]
typedef struct IpdosValueMethods {
  // Function callback that returns the kind of the value.
  enum IpdosValueKind (*kind)(const IpdosValue*);
  // Function callback that returns the length of the value.
  //
  // In case [`IpdosValueMethods::kind`] returns
  // [`IpdosValueKind::IpdosValueList`], the length is the number of elements
  // in the list, accessible using [`IpdosValueMethods::get_element`].
  //
  // In case [`IpdosValueMethods::kind`] returns
  // [`IpdosValueKind::IpdosValueSetInt`] or
  // [`IpdosValueKind::IpdosValueSetFloat`], the length is the number of
  // ranges in the set, accessible using
  // [`IpdosValueMethods::get_range_int`] or
  // [`IpdosValueMethods::get_range_float`].
  //
  // # Panics
  //
  // This function callback may panic if [`IpdosValueMethods::kind`] does not
  // return [`IpdosValueKind::IpdosValueList`],
  // [`IpdosValueKind::IpdosValueSetInt`], or
  // [`IpdosValueKind::IpdosValueSetFloat`].
  uint64_t (*len)(const IpdosValue*);
  // Function callback that returns the decision variable index contained in
  // the value.
  //
  // # Panics
  //
  // This function callback may panic if [`IpdosValueMethods::kind`] does not
  // return [`IpdosValueKind::IpdosValueDecision`].
  uint64_t (*get_decision)(const IpdosValue*);
  // Function callback that returns the integer value contained in the value.
  //
  // # Panics
  //
  // This function callback may panic if [`IpdosValueMethods::kind`] does not
  // return [`IpdosValueKind::IpdosValueInt`].
  int64_t (*get_int)(const IpdosValue*);
  // Function callback that returns the floating point value contained in the
  // value.
  //
  // # Panics
  //
  // This function callback may panic if [`IpdosValueMethods::kind`] does not
  // return [`IpdosValueKind::IpdosValueFloat`].
  double (*get_float)(const IpdosValue*);
  // Function callback that returns the string pointer value contained in the
  // value.
  //
  // # Panics
  //
  // This function callback may panic if [`IpdosValueMethods::kind`] does not
  // return [`IpdosValueKind::IpdosValueString`].
  const char *(*get_string)(const IpdosValue*);
  // Function callback that returns the Boolean value contained in the value.
  //
  // # Panics
  //
  // This function callback may panic if [`IpdosValueMethods::kind`] does not
  // return [`IpdosValueKind::IpdosValueBool`].
  bool (*get_bool)(const IpdosValue*);
  // Function callback that returns a range from a range list representing
  // the integer set contained in the value.
  //
  // # Panics
  //
  // This function callback may panic if [`IpdosValueMethods::kind`] does not
  // return [`IpdosValueKind::IpdosValueSetInt`].
  int64_t ((*get_range_int)(const IpdosValue*, uint64_t index))[2];
  // Function callback that returns a range from a range list representing
  // the floating point set contained in the value.
  //
  // # Panics
  //
  // This function callback may panic if [`IpdosValueMethods::kind`] does not
  // return [`IpdosValueKind::IpdosValueSetFloat`].
  double ((*get_range_float)(const IpdosValue*, uint64_t index))[2];
  // Function callback that returns an element from the list contained in the
  // value.
  //
  // # Panics
  //
  // This function callback may panic if [`IpdosValueMethods::kind`] does not
  // return [`IpdosValueKind::IpdosValueList`].
  struct IpdosValueRef (*get_element)(const IpdosValue*, uint64_t index);
} IpdosValueMethods;

// Handle for a value
//
// The caller can use the `get_value` function to retrieve the value of the
// used decision variables.
typedef struct IpdosValueRef {
  // The data pointer to be the first argument of `get_value`.
  const IpdosValue *data;
  // Reference to the structure containing the function callbacks used to
  // interact with the value.
  const struct IpdosValueMethods *methods;
} IpdosValueRef;

// A struct containing the function pointers to interact with the
// [`IpdosModel`]
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
  struct IpdosValueRef (*decision_domain)(const IpdosModel *model, uint64_t decision);
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
  struct IpdosValueRef (*constraint_argument)(const IpdosModel *model,
                                              uint64_t constraint,
                                              uint64_t index);
  // Check whether the decision variable is functionally defined by a
  // constraint.
  //
  // This function returns a [`IpdosValue`] that is either
  // [`IpdosValueKind::IpdosValueDecision`] if it defined a decision variable
  // or [`IpdosValueKind::IpdosValueAbsent`] otherwise.
  struct IpdosValueRef (*constraint_defines)(const IpdosModel *model, uint64_t decision);
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
  struct IpdosValueRef (*objective_arg)(const IpdosModel *model);
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
  struct IpdosValueRef (*annotation_argument)(const IpdosAnnotation *ann, uint64_t index);
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
  struct IpdosValueRef arg_ty;
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

// A struct containing the function pointers to interact with a
// [`IpdosSolution`]
typedef struct IpdosSolutionMethods {
  // Function callback to retrieve the value assigned to a decision variable
  // in the solution.
  struct IpdosValueRef (*get_value)(const IpdosSolution *data, uint64_t decision_index);
  // Function callback to retrieve the statistical information made available
  // by the solver about the search process so far.
  struct IpdosValueRef (*get_statistic)(const IpdosSolution *data, const char *ident);
} IpdosSolutionMethods;

// Handle for a solution emitted by the solver.
typedef struct IpdosSolutionRef {
  // The data pointer to be the first argument of `get_value`.
  const IpdosSolution *data;
  // Reference to the structure containing the function callbacks used to
  // interact with the solution.
  const struct IpdosSolutionMethods *methods;
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

// The definition of statistical information that is made available by the
// solver.
typedef struct IpdosStatistic {
  // The identifier used to retrieve the statistical information from the
  // solver or a solution.
  const char *ident;
  // The type of value that is expected for this option.
  struct IpdosValueRef ty;
  // Whether the statistical information is available as part of solutions.
  bool solution;
  // Whether the statistical information is generally available from the
  // solver instance.
  bool solver;
} IpdosStatistic;

// A list of [`IpdosStatistic`]s.
//
// This type is, for example, used to return from [`ipdos_statistic_list`].
typedef struct IpdosStatisticList {
  // The number of elements in the `stats` array.
  uint64_t len;
  // An array of statistical information definitions.
  const struct IpdosStatistic *stats;
} IpdosStatisticList;

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
