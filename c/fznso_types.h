#ifndef fznso_types_h
#define fznso_types_h

#include <stdbool.h>
#include <stdint.h>

// Representation of the base type of a value.
typedef enum FznsoTypeBase {
  // Boolean type
  FznsoTypeBaseBool,
  // Integer numeric type
  FznsoTypeBaseInt,
  // Floating point numeric type
  FznsoTypeBaseFloat,
  // Character string type
  FznsoTypeBaseString,
} FznsoTypeBase;

// Enumerated type used to mark the kind of [`FznsoValue`]. This is used to
// determine which "get" method in [`FznsoValueMethods`] is safe to call.
typedef enum FznsoValueKind {
  // No value is available.
  FznsoValueAbsent,
  // The value is available using [`FznsoValueMethods::get_decision`].
  FznsoValueDecision,
  // The value is available using [`FznsoValueMethods::get_constraint`].
  FznsoValueConstraint,
  // The value is available using [`FznsoValueMethods::get_bool`].
  FznsoValueBool,
  // The value is available using [`FznsoValueMethods::get_int`].
  FznsoValueInt,
  // The value is available using [`FznsoValueMethods::get_float`].
  FznsoValueFloat,
  // The value is available using [`FznsoValueMethods::get_string`].
  FznsoValueString,
  // Sets of integers are represented using a range list. The number of
  // ranges is available using [`FznsoValueMethods::len`], and the ranges can
  // be accessed using [`FznsoValueMethods::get_range_int`].
  FznsoValueSetInt,
  // Sets of floats are represented using a range list. The number of
  // ranges is available using [`FznsoValueMethods::len`], and the ranges can
  // be accessed using [`FznsoValueMethods::get_range_float`].
  FznsoValueSetFloat,
  // The length of the list can be accessed using
  // [`FznsoValueMethods::len`], and elements in the list can be
  // accessed using [`FznsoValueMethods::get_element`]
  FznsoValueList,
} FznsoValueKind;

// The status returned by `fznso_solver_run`, indicating whether the solver
// completed its search.
typedef enum FznsoStatus {
  // The solver explored the full search space and yielded all relevant
  // solutions.
  FznsoComplete,
  // The solver did not explore the full search space due to a timeout or
  // other termination condition. Additional (better) solutions might be
  // possible.
  FznsoIncomplete,
  // An error occurred during the solver's execution.
  //
  // [`fznso_solver_read_error`] can be used to retrieve the error message.
  FznsoError,
} FznsoStatus;

// The handle to a the data of a solution instance.
//
// This type is opaque to the user. Only pointers of this type are ever used.
//
// In implementation of the FZnSO interface, a pointer is generally cast to
// this type, i.e. `(FznsoSolutionData*) my_solution`. A similar cast can be
// used to cast the pointer back to the original type, e.g. `(MySolution*)
// fznso_solution`.
typedef void FznsoAnnotation;

// Representation of a type to signal and check whether an argument takes the
// correct type.
typedef struct FznsoType {
  // Whether the type is a list of values.
  bool list_of;
  // Whether the argument can be or contain decision variables (represented
  // as decision indexes).
  bool decision;
  // Whether expected type is an set of values of the base type.
  bool set_of;
  // Whether the expected type is optional (and can take the value of
  // [`FznsoValueKind::FznsoValueAbsent`]).
  bool opt;
  // The expected base type of the argument.
  enum FznsoTypeBase base;
} FznsoType;

// Representation of a type of constraint, discerned by its identifier and the
// types of its arguments.
typedef struct FznsoConstraintType {
  // The identifier of the constraint type.
  const char *ident;
  // The number of expected arguments for the constraint type.
  size_t arg_len;
  // The types of the expected arguments for the constraint type.
  const struct FznsoType *arg_types;
} FznsoConstraintType;

// A list of [`FznsoConstraintType`]s.
//
// This type is for example used to return from [`fznso_constraint_list`].
typedef struct FznsoConstraintList {
  // The number of elements in the `constraints` array.
  size_t len;
  // An array of constraint types.
  const struct FznsoConstraintType *constraints;
} FznsoConstraintList;

// The handle of a model instance.
//
// This type is opaque to the user. Only pointers of this type are ever used.
//
// In implementation of the FZnSO interface, a pointer is generally cast to
// this type, i.e. `(FznsoModelData*) my_model`. A similar cast can be used to
// cast the pointer back to the original type, e.g. `(MyModel*) fznso_model`.
typedef void FznsoModel;

// The type of a value used as an argument or solution assignment.
//
// This type is opaque to the user. Only pointers of this type are ever used.
//
// In implementation of the FZnSO interface, a pointer is generally cast to
// this type, e.g. `(FznsoValue*) my_value`. A similar cast can be used to cast
// the pointer back to the original type, e.g. `(MyValue*) fznso_value`.
typedef void FznsoValue;

// Wrapper type for the indexes that represent decision variables in the model.
typedef size_t FznsoDecisionIdx;

// Wrapper type for the indexes that represent constraints in the model.
typedef size_t FznsoConstraintIdx;

// A struct containing the function pointers to interact with a
// [`FznsoValue`]
typedef struct FznsoValueMethods {
  // Function callback that returns the kind of the value.
  enum FznsoValueKind (*kind)(const FznsoValue*);
  // Function callback that returns the length of the value.
  //
  // In case [`FznsoValueMethods::kind`] returns
  // [`FznsoValueKind::FznsoValueList`], the length is the number of elements
  // in the list, accessible using [`FznsoValueMethods::get_element`].
  //
  // In case [`FznsoValueMethods::kind`] returns
  // [`FznsoValueKind::FznsoValueSetInt`] or
  // [`FznsoValueKind::FznsoValueSetFloat`], the length is the number of
  // ranges in the set, accessible using
  // [`FznsoValueMethods::get_range_int`] or
  // [`FznsoValueMethods::get_range_float`].
  //
  // # Panics
  //
  // This function callback may panic if [`FznsoValueMethods::kind`] does not
  // return [`FznsoValueKind::FznsoValueList`],
  // [`FznsoValueKind::FznsoValueSetInt`], or
  // [`FznsoValueKind::FznsoValueSetFloat`].
  size_t (*len)(const FznsoValue*);
  // Function callback that returns the decision variable index contained in
  // the value.
  //
  // # Panics
  //
  // This function callback may panic if [`FznsoValueMethods::kind`] does not
  // return [`FznsoValueKind::FznsoValueDecision`].
  FznsoDecisionIdx (*get_decision)(const FznsoValue*);
  // Function callback that returns the constraint index contained in the
  // value.
  //
  // # Panics
  //
  // This function callback may panic if [`FznsoValueMethods::kind`] does not
  // return [`FznsoValueKind::FznsoValueConstraint`].
  FznsoConstraintIdx (*get_constraint)(const FznsoValue*);
  // Function callback that returns the integer value contained in the value.
  //
  // # Panics
  //
  // This function callback may panic if [`FznsoValueMethods::kind`] does not
  // return [`FznsoValueKind::FznsoValueInt`].
  int64_t (*get_int)(const FznsoValue*);
  // Function callback that returns the floating point value contained in the
  // value.
  //
  // # Panics
  //
  // This function callback may panic if [`FznsoValueMethods::kind`] does not
  // return [`FznsoValueKind::FznsoValueFloat`].
  double (*get_float)(const FznsoValue*);
  // Function callback that returns the string pointer value contained in the
  // value.
  //
  // # Panics
  //
  // This function callback may panic if [`FznsoValueMethods::kind`] does not
  // return [`FznsoValueKind::FznsoValueString`].
  const char *(*get_string)(const FznsoValue*);
  // Function callback that returns the Boolean value contained in the value.
  //
  // # Panics
  //
  // This function callback may panic if [`FznsoValueMethods::kind`] does not
  // return [`FznsoValueKind::FznsoValueBool`].
  bool (*get_bool)(const FznsoValue*);
  // Function callback that returns a range from a range list representing
  // the integer set contained in the value.
  //
  // # Panics
  //
  // This function callback may panic if [`FznsoValueMethods::kind`] does not
  // return [`FznsoValueKind::FznsoValueSetInt`].
  int64_t ((*get_range_int)(const FznsoValue*, size_t index))[2];
  // Function callback that returns a range from a range list representing
  // the floating point set contained in the value.
  //
  // # Panics
  //
  // This function callback may panic if [`FznsoValueMethods::kind`] does not
  // return [`FznsoValueKind::FznsoValueSetFloat`].
  double ((*get_range_float)(const FznsoValue*, size_t index))[2];
  // Function callback that returns an element from the list contained in the
  // value.
  //
  // # Panics
  //
  // This function callback may panic if [`FznsoValueMethods::kind`] does not
  // return [`FznsoValueKind::FznsoValueList`].
  struct FznsoValueRef (*get_element)(const FznsoValue*, size_t index);
} FznsoValueMethods;

// Handle for a value
//
// The caller can use the `get_value` function to retrieve the value of the
// used decision variables.
typedef struct FznsoValueRef {
  // The data pointer to be the first argument of `get_value`.
  const FznsoValue *data;
  // Reference to the structure containing the function callbacks used to
  // interact with the value.
  const struct FznsoValueMethods *methods;
} FznsoValueRef;

// A struct containing the function pointers to interact with the
// [`FznsoModel`]
typedef struct FznsoModelMethods {
  // Returns the current number of model layers currently contained in the
  // model.
  //
  // Layers provides a way for the modelling user to add and retract
  size_t (*layer_len)(const FznsoModel *model);
  // Returns the number of layers that have been unchanged since the last
  // call to [`fznso_solver_run`].
  //
  // Unchanged layers must be consecutive starting from layer 0.
  size_t (*layer_unchanged)(const FznsoModel *model);
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
  size_t (*layer_permanent)(const FznsoModel *model);
  // Returns the number of permanent layers that have been marked as
  // redundant.
  //
  // The solver can remove the decision variables and constraints in these
  // layers, if convenient. Once a layer is marked as redundant, it can
  // forever be considered redundant. As such, the solver can assume that
  // the number of redundant layers only ever increases.
  size_t (*layer_redundant_len)(const FznsoModel *model);
  // Returns the index of the n-th permanent layer that has been marked as
  // redundant.
  //
  // The `n` argument must be less than the value returned by
  // [`layer_redundant_len`].
  size_t (*layer_redundant_index)(const FznsoModel *model, size_t n);
  // Retrieve the number of decisions currently contained in the model.
  size_t (*decision_len)(const FznsoModel *model);
  // Retrieve the decision index of the last decision variable in the layer.
  //
  // This can be equivalent to `decision_layer_end` of `layer-1` if the layer
  // does not add any new decisions.
  size_t (*decision_layer_end)(const FznsoModel *model, size_t layer);
  // Retrieve the domain of the given decision.
  //
  // Note that the the value might have [`FznsoValueKind::FznsoValueAbsent`]
  // if the decision variable does not have an explicit domain.
  struct FznsoValueRef (*decision_domain)(const FznsoModel *model, FznsoDecisionIdx decision);
  // Retrieve the name of a decision variable, if it exists.
  //
  // Note that names are only available for debugging purposes. Decisions are
  // identified using their index in the model. If the decision variable does
  // not have a name, this function returns a null pointer.
  const char *(*decision_name)(const FznsoModel *model, FznsoDecisionIdx decision);
  // Check whether the decision variable is defined by a constraint.
  bool (*decision_defined)(const FznsoModel *model, FznsoDecisionIdx decision);
  // Retrieve the number of annotations of a constraint.
  size_t (*decision_annotation_len)(const FznsoModel *model, FznsoDecisionIdx decision);
  // Retrieve the value of the constraint's annotation at the given index.
  //
  // The `index` argument must be less than the value returned by
  // [`decision_annotation_len`] for the given decision variable.
  const FznsoAnnotation *(*decision_annotation)(const FznsoModel *model,
                                                FznsoDecisionIdx decision,
                                                size_t index);
  // Retrieve the number of constraints currently contained in the model.
  size_t (*constraint_len)(const FznsoModel *model);
  // Retrieve the constraint index of the first new constraint in the layer.
  //
  // This can be equivalent to `constraint_len` if the layer does not add any
  // new constraints.
  size_t (*constraint_layer_end)(const FznsoModel *model);
  // Retrieve the identifier of a constraint.
  //
  // The returned pointer can be assumed to have the the same lifetime as the
  // model reference and must be valid UTF8.
  const char *(*constraint_ident)(const FznsoModel *model, FznsoConstraintIdx constraint);
  // Retrieve the number of arguments of a constraint.
  size_t (*constraint_argument_len)(const FznsoModel *model, FznsoConstraintIdx constraint);
  // Retrieve the value of the constraint's argument at the given index.
  //
  // The `index` argument must be less than the value returned by
  // `constraint_argument_len` for the given constraint.
  struct FznsoValueRef (*constraint_argument)(const FznsoModel *model,
                                              FznsoConstraintIdx constraint,
                                              size_t index);
  // Check whether the decision variable is functionally defined by a
  // constraint.
  //
  // This function returns a [`FznsoValue`] that is either
  // [`FznsoValueKind::FznsoValueDecision`] if it defined a decision variable
  // or [`FznsoValueKind::FznsoValueAbsent`] otherwise.
  struct FznsoValueRef (*constraint_defines)(const FznsoModel *model, FznsoConstraintIdx constraint);
  // Retrieve the number of annotations on a constraint.
  size_t (*constraint_annotation_len)(const FznsoModel *model, FznsoConstraintIdx constraint);
  // Retrieve the annotation on a constraint at the given index.
  //
  // The `index` argument must be less than the value returned by
  // `constraint_annotation_len` for the given constraint.
  const FznsoAnnotation *(*constraint_annotation)(const FznsoModel *model,
                                                  FznsoConstraintIdx constraint,
                                                  size_t index);
  // Request the identifier of the type of objective strategy to be used when
  // solving the model.
  //
  // Note that the function can return a null pointer if the model does not
  // have an objective strategy.
  const char *(*objective_ident)(const FznsoModel *model);
  // Retrieve the argument of the objective strategy.
  struct FznsoValueRef (*objective_arg)(const FznsoModel *model);
  // Retrieve the number of annotations on an objective
  size_t (*objective_annotation_len)(const FznsoModel *model);
  // Retrieve the annotation on the objective at the given index.
  //
  // The `index` argument must be less than the value returned by
  // `objective_annotation_len`.
  const FznsoAnnotation *(*objective_annotation)(const FznsoModel *model, size_t index);
  // Retrieve the identifier of an annotation.
  //
  // The returned pointer can be assumed to have the the same lifetime as the
  // annotation reference and must be valid UTF8.
  const char *(*annotation_ident)(const FznsoAnnotation *ann);
  // Retrieve the number of arguments of an annotation.
  size_t (*annotation_argument_len)(const FznsoAnnotation *ann);
  // Retrieve the value of the annotation's argument at the given index.
  //
  // The `index` argument must be less than the value returned by
  // `annotation_argument_len` for the given annotation.
  struct FznsoValueRef (*annotation_argument)(const FznsoAnnotation *ann, size_t index);
} FznsoModelMethods;

// An interface to a model instance used to communicate with the solver.
//
// The solver can use the included function callbacks to interact with the
// model.
typedef struct FznsoModelRef {
  // The handle to the data of the model instance.
  const FznsoModel *data;
  // Reference to the structure containing the function callbacks used to
  // interact with the model.
  const struct FznsoModelMethods *methods;
} FznsoModelRef;

// Representation of a type of objective strategies, discerned by its
// identifier and the type of its argument.
typedef struct FznsoObjective {
  // The identifier of the objective type.
  const char *ident;
  // The type of the expected argument for the constraint type.
  struct FznsoType arg_type;
} FznsoObjective;

// A list of [`FznsoObjective`]s.
//
// This type is, for example, used to return from [`fznso_objective_list`].
typedef struct FznsoObjectiveList {
  // The number of elements in the `options` array.
  size_t len;
  // An array of option definitions.
  const struct FznsoObjective *options;
} FznsoObjectiveList;

// The definition of an option that is available to be set for the solver.
typedef struct FznsoOption {
  // The identifier used to set the option or get the current value of the
  // option.
  const char *ident;
  // The type of value that is expected for this option.
  struct FznsoValueRef arg_ty;
  // The default value for this option.
  struct FznsoType arg_def;
} FznsoOption;

// A list of [`FznsoOption`]s.
//
// This type is, for example, used to return from [`fznso_option_list`].
typedef struct FznsoOptionList {
  // The number of elements in the `options` array.
  size_t len;
  // An array of option definitions.
  const struct FznsoOption *options;
} FznsoOptionList;

// The handle to a the data of a solution instance.
//
// This type is opaque to the user. Only pointers of this type are ever used.
//
// In implementation of the FZnSO interface, a pointer is generally cast to
// this type, i.e. `(FznsoSolutionData*) my_solution`. A similar cast can be
// used to cast the pointer back to the original type, e.g. `(MySolution*)
// fznso_solution`.
typedef void FznsoSolution;

// A struct containing the function pointers to interact with a
// [`FznsoSolution`]
typedef struct FznsoSolutionMethods {
  // Function callback to retrieve the value assigned to a decision variable
  // in the solution.
  struct FznsoValueRef (*get_value)(const FznsoSolution *data, size_t decision_index);
  // Function callback to retrieve the statistical information made available
  // by the solver about the search process so far.
  struct FznsoValueRef (*get_statistic)(const FznsoSolution *data, const char *ident);
} FznsoSolutionMethods;

// Handle for a solution emitted by the solver.
typedef struct FznsoSolutionRef {
  // The data pointer to be the first argument of `get_value`.
  const FznsoSolution *data;
  // Reference to the structure containing the function callbacks used to
  // interact with the solution.
  const struct FznsoSolutionMethods *methods;
} FznsoSolutionRef;

// The handle to a solver instance.
//
// This type is opaque to the user. Only pointers of this type are ever used.
//
// In implementation of the FZnSO interface, a pointer is generally cast to
// this type, i.e. `(FznsoSolver*) my_solver`. A similar cast can be used to
// cast the pointer back to the original type, e.g. `(MySolverType*)
// fznso_solver`.
typedef void FznsoSolver;

// The definition of statistical information that is made available by the
// solver.
typedef struct FznsoStatistic {
  // The identifier used to retrieve the statistical information from the
  // solver or a solution.
  const char *ident;
  // The type of value that is expected for this option.
  struct FznsoValueRef ty;
  // Whether the statistical information is available as part of solutions.
  bool solution;
  // Whether the statistical information is generally available from the
  // solver instance.
  bool solver;
} FznsoStatistic;

// A list of [`FznsoStatistic`]s.
//
// This type is, for example, used to return from [`fznso_statistic_list`].
typedef struct FznsoStatisticList {
  // The number of elements in the `stats` array.
  size_t len;
  // An array of statistical information definitions.
  const struct FznsoStatistic *stats;
} FznsoStatisticList;

// A list of [`FznsoType`]s.
//
// This type is for example used to return from [`fznso_decision_list`].
typedef struct FznsoTypeList {
  // The number of elements in the `constraints` array.
  size_t len;
  // An array of constraint types.
  const struct FznsoType *types;
} FznsoTypeList;

#endif  /* fznso_types_h */
