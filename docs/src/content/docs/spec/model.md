---
title: The model
description: What a solver can ask about a problem (decision variables, constraints, the objective, annotations), and how layers express incrementality.
sidebar:
  order: 6
---

A model is not a data structure handed to the solver.
It is an `FznsoModelRef`: an [opaque handle](../lifetimes-and-ownership/#handles-are-opaque) plus a table of callbacks, `FznsoModelMethods`, that the solver calls while it works.

The application's own representation of the problem *is* the model, provided it can answer these questions.
It is read-only for the duration of a run, and may be [read concurrently](../threading/#the-model-may-be-read-concurrently).

Every accessor that takes an index has a matching length accessor, and the index must be below it.
Each one is listed, with its own anchor, under [`FznsoModelMethods`](../../reference/abi-types/#fznsomodelmethods).

## Decision variables

Decision variables are numbered `0 .. decision_len()`.
For each:

| Accessor | Answers |
| --- | --- |
| `decision_type` | The [type](../types-and-values/#types) the variable must take. |
| `decision_domain` | The values it may take, as a value — or `FznsoValueAbsent` if it has no explicit domain. |
| `decision_name` | A name, or an `FznsoStr` with a null `ptr` if it has none. |
| `decision_defined` | Whether some constraint functionally defines this variable. |
| `decision_in_solution` | Whether the application may ask a solution for this variable's value. |
| `decision_annotation_len` / `decision_annotation` | Its [annotations](#annotations). |

`decision_type` and `decision_domain` answer different questions.
The type says what *kind* of value this is (`var int`, `var set of int`); the domain says *which* of them it may take (`1..10`, `{2, 4, 6}`).
A variable with an absent domain is unconstrained apart from its type.

`decision_in_solution` is the only thing that obliges a solver to assign a variable.
A solver must give every variable it is true for a value in every solution it reports; it may leave any of the others open if the constraints do not pin them down, which is what lets it skip the scaffolding a model is mostly made of.

:::note
Names are for debugging only.
Variables are identified by index everywhere in the protocol, and a solver must never depend on a name being present or unique — in particular, a name says nothing about whether a value is needed. That is `decision_in_solution`.
:::

## Constraints

Constraints are numbered `0 .. constraint_len()`.
For each:

| Accessor | Answers |
| --- | --- |
| `constraint_ident` | The identifier, e.g. `int_lin_le`. Must be one the solver declared in `fznso_<name>_constraint_list`. |
| `constraint_argument_len` / `constraint_argument` | Its arguments, as values. |
| `constraint_defines` | The variable this constraint functionally defines, as a value of kind `FznsoValueDecision` — or `FznsoValueAbsent` if it defines none. |
| `constraint_annotation_len` / `constraint_annotation` | Its [annotations](#annotations). |

`constraint_defines` and `decision_defined` are two views of the same fact, and a model must keep them consistent: if `constraint_defines(c)` names variable `d`, then `decision_defined(d)` is true.
A solver may use this to propagate functionally, or ignore it entirely.

## The objective

A model has at most one objective:

| Accessor | Answers |
| --- | --- |
| `objective_ident` | The strategy's identifier, e.g. `int_minimize` — or a null `ptr` if the model has no objective. |
| `objective_arg` | Its argument: a variable for `int_minimize`, a list of them for `int_lex_minimize`. |
| `objective_annotation_len` / `objective_annotation` | Its [annotations](#annotations). |

**With no objective set, the solver finds any assignment satisfying the constraints.**
With one set, it must be an identifier the solver declared in `fznso_<name>_objective_list`; the common strategies are in the [registry](../../registry/objectives/).

## Annotations

An annotation is an identifier plus a list of argument values, attached to a decision variable, a constraint or the objective.
They carry advice (a search strategy, an output hint) that a solver may act on or ignore.

A solver that ignores an annotation it recognises as meaningful should say so through the [`warn`](../../registry/message-scopes/) message scope.
A solver that does not recognise one at all may silently ignore it.

## Layers

Layers are how a model expresses **incrementality**: which part of the problem the solver has already seen, and which part is new.

Decisions and constraints are grouped into consecutively numbered layers.
Their global indices follow layer order, so every layer owns a contiguous range:

- `layer_len()` — how many layers the model currently has.
- `decision_layer_end(l)` — the **exclusive upper bound** on the decision indices belonging to layers `0..=l`.
  Layer `l` therefore owns `decision_layer_end(l-1) .. decision_layer_end(l)`, taking the bound below layer 0 as 0.
  It equals `decision_layer_end(l-1)` when the layer adds no decisions, and `decision_len()` for the final layer.
- `constraint_layer_end(l)` — the same, for constraints.

An application adds a group of decisions and constraints by pushing a layer, and retracts the group by popping it.
Because indices follow layer order, popping a layer never renumbers anything that survives.

### Unchanged layers

`layer_unchanged()` is the number of layers that have not changed since the last call to `fznso_<name>_solver_run`.
**Unchanged layers are consecutive starting from layer 0.**

This is the whole point of the mechanism.
A solver that kept its state from the previous run can skip re-posting everything below `layer_unchanged()` and process only what follows.
A solver that keeps no state simply ignores the number and reads the whole model, which is always correct, just not incremental.

### Permanent layers

`layer_permanent()` is the number of layers that can never be retracted.
They too are consecutive starting from layer 0, and **their count only ever grows**, so a solver may commit to them irreversibly: simplify against them, fold them into its own representation, discard the original form.

### Redundant layers

A permanent layer may additionally be marked *redundant*, meaning the application has determined its decisions and constraints no longer affect the answer.
The solver **may** drop them if that is convenient, and may equally keep them.

`layer_redundant_len()` gives how many permanent layers are so marked, and `layer_redundant_index(n)` the index of the n-th.
**Once marked redundant a layer stays redundant**, so this count only grows too.

Redundancy exists because "permanent" and "still relevant" are different properties.
A layer that has been folded irreversibly into the solver's state cannot be popped, but the application may still learn that it has become vacuous, for example a bound that a later, tighter bound subsumes.

### A worked example

A model with three layers, five decisions and four constraints:

| Layer | Decisions | Constraints | `decision_layer_end` | `constraint_layer_end` |
| --- | --- | --- | --- | --- |
| 0 | 0, 1, 2 | 0 | 3 | 1 |
| 1 | — | 1, 2 | 3 | 3 |
| 2 | 3, 4 | 3 | 5 | 4 |

Layer 1 adds no decisions, so `decision_layer_end(1) == decision_layer_end(0) == 3`.
The final layer's bounds equal `decision_len()` and `constraint_len()`.

If `layer_permanent()` is 1 and `layer_unchanged()` is 2, then a solver resuming from its previous state knows that layer 0 is committed for good, that layers 0 and 1 are exactly as it last saw them, and that it need only process layer 2: decisions 3 and 4, and constraint 3.
