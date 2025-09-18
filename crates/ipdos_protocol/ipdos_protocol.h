#ifndef ipdos_protocol_h
#define ipdos_protocol_h

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

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
  // `ipdos_solver_read_error` can be used to retrieve the error message.
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

// Representation of a type to signal and check whether an argument takes the
// correct type.
typedef struct IpdosType {
  // Whether the type is a list of values.
  bool list_of;
  // Whether the argument can be or contain decision variables (represented as
  // decision indexes).
  bool decision;
  // Whether expected type is an set of values of the base type.
  bool set_of;
  // The expected base type of the argument.
  enum IpdosTypeBase base;
} IpdosType;

// Representation of a type of constraint, discerned by its identifier and the
// types of its arguments.
typedef struct IpdosConstraintType {
  // The identifier of the constraint type.
  const char *ident;
  // The number of expected arguments for the constraint type.
  uintptr_t arg_len;
  // The types of the expected arguments for the constraint type.
  const struct IpdosType *arg_types;
} IpdosConstraintType;

// A list of [`IpdosConstraintType`]s.
//
// This type is for example used to return from [`ipdos_constraint_list`].
typedef struct IpdosConstraintList {
  // The number of elements in the `constraints` array.
  uintptr_t len;
  // An array of constraint types.
  const struct IpdosConstraintType *constraints;
} IpdosConstraintList;

// A list of [`IpdosType`]s.
//
// This type is for example used to return from [`ipdos_decision_list`].
typedef struct IpdosTypeList {
  // The number of elements in the `constraints` array.
  uintptr_t len;
  // An array of constraint types.
  const struct IpdosType *types;
} IpdosTypeList;

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
  uintptr_t len;
  // An array of option definitions.
  const struct IpdosObjective *options;
} IpdosObjectiveList;

// The storage of the value content of a [`IpdosValue`].
typedef union IpdosValueContent {
  // Decision index storage
  uintptr_t decision_index;
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

// The definition of an option that is available to be set for the solver.
typedef struct IpdosOption {
  // The identifier used to set the option or get the current value of the option.
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
  uintptr_t len;
  // An array of option definitions.
  const struct IpdosOption *options;
} IpdosOptionList;

// The handle to a solver instance.
//
// This type is opaque to the user. Only pointers of this type are ever used.
//
// In implementation of the IPDOS interface, a pointer is generally cast to
// this type, i.e. `(IpdosSolver*) my_solver`. A similar cast can be used to
// cast the pointer back to the original type, e.g. `(MySolverType*)
// ipdos_solver`.
typedef void IpdosSolver;

// The handle to a the data of a model instance.
//
// This type is opaque to the user. Only pointers of this type are ever used.
//
// In implementation of the IPDOS interface, a pointer is generally cast to
// this type, i.e. `(IpdosModelData*) my_model`. A similar cast can be used to
// cast the pointer back to the original type, e.g. `(MyModel*) ipdos_model`.
typedef void IpdosModelData;

// An interface to a model instance used to communicate with the solver.
//
// The solver can use the included function callbacks to interact with the
// model.
typedef struct IpdosModel {
  // The handle to the data of the model instance.
  //
  // This handle is passed to all the different function callbacks to
  // interact with the model.
  const IpdosModelData *data;
  // Returns the current number of model layers currently contained in the
  // model.
  //
  // Note that it is required that the solver calls [`ipdos_solver_push`] and
  // [`ipdos_solver_pop`] whenever a layer is added or removed.
  uintptr_t (*layers)(const IpdosModelData *model);
  // Retrieve the number of constraints currently contained in the model.
  uintptr_t (*constraint_len)(const IpdosModelData *model);
  // Retrieve the constraint index of the first new constraint in the layer.
  //
  // This can be equivalent to `constraint_len` if the layer does not add any
  // new constraints.
  uintptr_t (*constraint_layer_start)(const IpdosModelData *model);
  // Retrieve the identifier of a constraint.
  //
  // The returned pointer can be assumed to have the the same lifetime as the
  // constraint reference and must be valid UTF8.
  const char *(*constraint_ident)(const IpdosModelData *model, uintptr_t constraint);
  // Retrieve the number of arguments of a constraint.
  uintptr_t (*constraint_argument_len)(const IpdosModelData *model, uintptr_t constraint);
  // Retrieve the value of the constraint's argument at the given index.
  //
  // The `index` argument must be less than the value returned by
  // `constraint_argument_len` for the given constraint.
  struct IpdosValue (*constraint_argument)(const IpdosModelData *model,
                                           uintptr_t constraint,
                                           uintptr_t index);
  // Retrieve the number of decisions currently contained in the model.
  uintptr_t (*decision_len)(const IpdosModelData *model);
  // Retrieve the decision index of the first new decision in the layer.
  //
  // This can be equivalent to `decision_len` if the layer does not add any
  // new decisions.
  uintptr_t (*decision_layer_start)(const IpdosModelData *model);
  // Retrieve the domain of the given decision.
  //
  // Note that the the value might have [`IpdosValueKind::IpdosValueAbsent`] if
  // the decision variable does not have an explicit domain.
  struct IpdosValue (*decision_domain)(const IpdosModelData *model, uintptr_t decision);
  // Retrieve the name of a decision variable, if it exists.
  //
  // Note that names are only available for debugging purposes. Decisions are
  // identified using their index in the model. If the decision variable does
  // not have a name, this function returns a null pointer.
  const char *(*decision_name)(const IpdosModelData *model, uintptr_t decision);
  // Check whether the decision variable is functionally defined by a
  // constraint.
  bool (*decision_defined)(const IpdosModelData *model, uintptr_t decision);
  // Request the identifier of the type of objective strategy to be used when
  // solving the model.
  //
  // Note that the function can return a null pointer if the model does not
  // have an objective strategy.
  const char *(*objective_ident)(const IpdosModelData *model);
  // Retrieve the argument of the objective strategy.
  struct IpdosValue (*objective_arg)(const IpdosModelData *model);
} IpdosModel;

// The handle to a the data of a solution instance.
//
// This type is opaque to the user. Only pointers of this type are ever used.
//
// In implementation of the IPDOS interface, a pointer is generally cast to
// this type, i.e. `(IpdosSolutionData*) my_solution`. A similar cast can be
// used to cast the pointer back to the original type, e.g. `(MySolution*)
// ipdos_solution`.
typedef void IpdosSolutionData;

// Structure used to represent a solution emitted by the solver.
//
// The caller can use the `get_value` function to retrieve the value of the
// used decision variables.
typedef struct IpdosSolution {
  // The data pointer to be the first argument of `get_value`.
  const IpdosSolutionData *data;
  // Function callback to retrieve the value assigned to a decision variable
  // in the solution.
  struct IpdosValue (*get_value)(const IpdosSolutionData *data, uintptr_t decision_index);
} IpdosSolution;

// Returns the list of available constraint that can be added to the solver.
struct IpdosConstraintList ipdos_constraint_list(void);

// Returns the list of types for which decision variable can be created by the
// solver.
struct IpdosTypeList ipdos_decision_list(void);

// Returns the list of available objective that can be achieved by the solver.
struct IpdosObjectiveList ipdos_objective_list(void);

// Returns the list of available options that can be set of the solver.
struct IpdosOptionList ipdos_option_list(void);

// Create a new solver instance
IpdosSolver *ipdos_solver_create(void);

// Free a solver instance, releasing all resources associated with it.
//
// The pointer to the solver instance will be invalid after this function has
// been called.
void ipdos_solver_free(IpdosSolver *solver);

// Get the current value of an option for the solver.
//
// Note that this is only valid to be called with options named by
// `ipdos_option_list`.
struct IpdosValue ipdos_solver_option_get(const IpdosSolver *solver, const char *ident);

// Set the current value of an option for the solver.
//
// Note that this is only valid to be called with options named by
// `ipdos_option_list`, and the value must be of the correct type.
bool ipdos_solver_option_set(IpdosSolver *solver, const char *ident, struct IpdosValue value);

// Remove a model layer from the solver.
//
// It is required that `model.layers` is less than the number of layers in the
// last call to `ipdos_solver_push` or `ipdos_solver_pop`. It is allowed to
// remove multiple model layers at the same time.
void ipdos_solver_pop(IpdosSolver *solver, struct IpdosModel model);

// Add a new model layer to the solver.
//
// It is required that `model.layers` is exactly one larger than the previous
// call to `ipdos_solver_push` or `ipdos_solver_pop`. It is NOT allowed to add
// multiple model layers at the same time.
bool ipdos_solver_push(IpdosSolver *solver, struct IpdosModel model);

// Read an error message from the solver.
//
// This function is expected to be called after solver interactions signal an
// error has occurred. For example, if [`ipdos_solver_run`] returns
// [`IpdosError`] or [`ipdos_solver_push`] returns `false`.
void ipdos_solver_read_error(IpdosSolver *solver,
                             void *context,
                             void (*read_error)(void *context, const char *error));

// Run the solver with the given model
enum IpdosStatus ipdos_solver_run(IpdosSolver *solver,
                                  void *context,
                                  void (*on_solution)(void *context,
                                                      const struct IpdosSolution *solution));

#endif  /* ipdos_protocol_h */
