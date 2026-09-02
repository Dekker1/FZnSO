---
title: The model
description: What a solver can ask about a problem (decision variables, constraints, the objective, annotations), and how layers express incrementality.
sidebar:
  order: 6
---

A model is an `FznsoModelRef`: an [opaque handle](../lifetimes-and-ownership/#handles-are-opaque) plus a table of callbacks, `FznsoModelMethods`, that the solver calls while it works.
The application's own representation of the problem *is* the model, provided it can answer these questions.
It is read-only for the duration of a run, and may be [read concurrently](../threading/#the-model-may-be-read-concurrently).

Every accessor that takes an index has a matching length accessor, and the index must be below it.
Each is listed, with its own anchor, under [`FznsoModelMethods`](../../reference/abi-types/#fznsomodelmethods).

## Decision variables

Numbered `0 .. decision_len()`. For each:

| Accessor | Answers |
| --- | --- |
| `decision_type` | The [type](../types-and-values/#types) the variable must take. |
| `decision_domain` | The values it may take, as a value — or `FznsoValueAbsent` if it has no explicit domain. |
| `decision_name` | A name, or an `FznsoStr` with a null `ptr` if it has none. |
| `decision_defined` | Whether some constraint functionally defines this variable. |
| `decision_in_solution` | Whether the application may ask a solution for this variable's value. |
| `decision_annotation_len` / `decision_annotation` | Its [annotations](#annotations). |

`decision_type` and `decision_domain` answer different questions: the type says what *kind* of value this is (`var int`, `var set of int`), the domain says *which* of them it may take (`1..10`, `{2, 4, 6}`).

`decision_in_solution` is the only thing that obliges a solver to assign a variable.
Every variable it is true for gets a value in every reported solution; the rest may be left open if the constraints do not pin them down.

:::note
Names are for debugging only.
Variables are identified by index everywhere in the protocol, so never depend on a name being present or unique — and a name says nothing about whether a value is wanted, which is `decision_in_solution`.
:::

## Constraints

Numbered `0 .. constraint_len()`. For each:

| Accessor | Answers |
| --- | --- |
| `constraint_ident` | The identifier, e.g. `int_lin_le`. Must be one the solver declared in `fznso_<name>_constraint_list`. |
| `constraint_argument_len` / `constraint_argument` | Its arguments, as values. |
| `constraint_defines` | The variable this constraint functionally defines, as a value of kind `FznsoValueDecision` — or `FznsoValueAbsent` if it defines none. |
| `constraint_annotation_len` / `constraint_annotation` | Its [annotations](#annotations). |

`constraint_defines` and `decision_defined` are two views of the same fact, and a model must keep them consistent: if `constraint_defines(c)` names variable `d`, then `decision_defined(d)` is true.
A solver may propagate functionally on this, or ignore it.

## The objective

A model has at most one objective:

| Accessor | Answers |
| --- | --- |
| `objective_ident` | The strategy's identifier, e.g. `int_minimize` — or a null `ptr` if the model has no objective. |
| `objective_arg` | Its argument: a variable for `int_minimize`, a list of them for `int_lex_minimize`. |
| `objective_annotation_len` / `objective_annotation` | Its [annotations](#annotations). |

With no objective, the solver finds any assignment satisfying the constraints.
With one, it must be an identifier the solver declared in `fznso_<name>_objective_list`; the common strategies are in the [registry](../../registry/objectives/).

## Annotations

An annotation is an identifier plus a list of argument values, attached to a decision variable, a constraint or the objective.
They carry advice — a search strategy, an output hint — that a solver may act on or ignore.

A solver that recognises an annotation as meaningful and ignores it anyway should say so through the [`warn`](../../registry/message-scopes/) message scope.
One it does not recognise at all may be silently ignored.

## Layers

Layers are how a model says which part of the problem the solver has already seen.

Decisions and constraints are grouped into consecutively numbered layers, and their global indices follow layer order, so every layer owns a contiguous range:

- `layer_len()` — how many layers the model currently has.
- `decision_layer_end(l)` — the **exclusive upper bound** on the decision indices belonging to layers `0..=l`.
  Layer `l` therefore owns `decision_layer_end(l-1) .. decision_layer_end(l)`, taking the bound below layer 0 as 0.
  It equals `decision_layer_end(l-1)` when the layer adds no decisions, and `decision_len()` for the final layer.
- `constraint_layer_end(l)` — the same, for constraints.

An application adds a group of decisions and constraints by pushing a layer, and retracts the group by popping it.
Because indices follow layer order, popping never renumbers anything that survives.

### The three counts

Before each run the model answers three questions about how much the solver can reuse:

| Callback | Says how many layers | A solver may assume |
| --- | --- | --- |
| `layer_unchanged()` | are exactly as the solver last saw them | consecutive from layer 0 |
| `layer_permanent()` | can never be retracted | consecutive from layer 0, and the count only grows |
| `layer_redundant_len()`, `layer_redundant_index(n)` | are permanent but no longer constrain anything | once redundant, always redundant, and the count only grows |

A solver that kept its state from the previous run re-posts only what lies above `layer_unchanged()`.
One that keeps no state ignores the number and reads the whole model, which is always correct, just not incremental.

Permanent layers may be committed to irreversibly: simplify against them, fold them into the solver's own representation, discard the original form.
A permanent layer marked *redundant* is one a later layer's stronger constraint has subsumed — it cannot be retracted, but it no longer rules anything out, so the solver **may** drop it and may equally keep it.

### A worked example

A model with three layers, five decisions and four constraints:

| Layer | Decisions | Constraints | `decision_layer_end` | `constraint_layer_end` |
| --- | --- | --- | --- | --- |
| 0 | 0, 1, 2 | 0 | 3 | 1 |
| 1 | — | 1, 2 | 3 | 3 |
| 2 | 3, 4 | 3 | 5 | 4 |

Layer 1 adds no decisions, so `decision_layer_end(1) == decision_layer_end(0) == 3`.
The final layer's bounds equal `decision_len()` and `constraint_len()`.

If `layer_permanent()` is 1 and `layer_unchanged()` is 2, a solver resuming from its previous state knows layer 0 is committed for good, layers 0 and 1 are exactly as it last saw them, and it need only process layer 2: decisions 3 and 4, and constraint 3.
