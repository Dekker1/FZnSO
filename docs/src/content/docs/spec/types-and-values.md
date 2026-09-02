---
title: Types & values
description: How FZnSO describes the type of an argument, how a value carries data across the interface, and how sets and strings are represented.
sidebar:
  order: 5
---

Everything that crosses the interface as data — a constraint argument, a variable domain, an option, a statistic, a solution assignment, an annotation argument — is an `FznsoValue`.
What *shape* of value is expected somewhere is an `FznsoType`.
Every declaration named here is in the [ABI types reference](../../reference/abi-types/).

## Types

`FznsoType` is a base type plus four independent qualifier flags:

```c
typedef struct FznsoType {
  bool list_of;      // a list of the element type
  bool decision;     // may be, or contain, decision variables
  bool set_of;       // over sets of the base type
  bool opt;          // may also be the absent value
  FznsoTypeBase base; // Bool, Int, Float or String
} FznsoType;
```

The flags compose, giving sixteen shapes over each of the four base types.
Written out, qualifiers go outermost first: `list of var opt set of int` is `list_of`, `decision`, `opt`, `set_of` and `FznsoTypeBaseInt` all at once.

`decision` does not mean "this *is* a variable"; it means the position accepts one.
An argument typed `var int` takes either a decision index or a literal integer, which is why the variables of `int_lin_le` may equally be constants.

A solver declares which decision types it can create through `fznso_<name>_decision_list`, and which argument types each constraint takes through `fznso_<name>_constraint_list`.

## Values

An `FznsoValue` is an [opaque handle](../lifetimes-and-ownership/#handles-are-opaque).
To read one, call `kind()` on its `FznsoValueRef` and then the accessor that answer permits; calling any other is undefined.

### Value kinds

| `FznsoValueKind` | Read with | Notes |
| --- | --- | --- |
| `FznsoValueAbsent` | — | No value. |
| `FznsoValueBool` | `as_bool` | |
| `FznsoValueInt` | `as_int` | `int64_t` |
| `FznsoValueFloat` | `as_float` | `double` |
| `FznsoValueString` | `as_string` + `len` | See [Strings](#strings) |
| `FznsoValueDecision` | `as_decision` | An index into the model's decision variables |
| `FznsoValueConstraint` | `as_constraint` | An index into the model's constraints |
| `FznsoValueSetInt` | `len` + `get_range_int` | See [Sets](#sets) |
| `FznsoValueSetFloat` | `len` + `get_range_float` | See [Sets](#sets) |
| `FznsoValueList` | `len` + `get_element` | Elements are themselves values |

`FznsoValueAbsent` is not an error.
Expect it wherever a type has `opt` set, wherever a domain is unconstrained, from an optional argument that was omitted, and from any lookup by name that missed.

### Sets

Sets are **range lists**: an ordered, non-overlapping sequence of inclusive `[min, max]` ranges, read with `get_range_int` or `get_range_float` for indices below `len`.
`1..1000000` is one range rather than a million elements, and a set can be scanned in one pass.

### Lists

`FznsoValueList` holds `len` elements, each read with `get_element` and each an `FznsoValue` in its own right.
Lists nest, and may mix kinds.

### A list is flat, and one-based

A list has one dimension and no index set, so **a constraint whose arguments are really a matrix, or really an array numbered from something other than 1, says so in its arguments** — the shape cannot travel with the value.
Two conventions cover it, and both are ordinary arguments a solver reads like any other:

- **A lost dimension travels as a count.**
  A matrix is passed row-major in a flat list, with the row length beside it — unless that length is already fixed by another argument, as `int_table`'s rows are by the array being constrained.
- **A lost index-set origin travels as an offset**, a plain `int` **immediately after** the array it belongs to.
  A list `xs` with offset `k` is read as if numbered `k .. k + length(xs) - 1`.
  `int_circuit(list of var int: xs, int: offset)` takes one because `xs[i]` names a position in `xs`; `int_inverse` takes two, named `f_offset` and `invf_offset` after the arrays they follow, because each array indexes the other.

A solver can meet an offset by padding the front of its own array, which only works from zero upwards.
An application numbering from below zero should decompose the constraint rather than expect a negative offset to be accepted.

## Strings

Strings cross the interface in two forms:

| | Used where a string is | Read with |
| --- | --- | --- |
| `FznsoStr` | a *field*: a constraint identifier, a variable's name, an option's identifier, a message scope | its own `ptr` and `len` |
| kind `FznsoValueString` | a *value*: a string constraint argument, a string-typed option | `as_string` + `len` |

Both are **UTF-8, pointer + length, and not null-terminated** — a trailing null may or may not be present, so use the length.
That lets an implementation hand out a slice of a string it already holds rather than copying into a null-terminated buffer.
Both are also borrowed: copy before storing, per [Lifetimes & ownership](../lifetimes-and-ownership/#nothing-transfers-ownership).

A null `ptr` in an `FznsoStr` signals the *absence* of a string, which is how an optional name or identifier says it is not there.
It is distinct from a present, empty string, which has a non-null `ptr` and a `len` of zero.
