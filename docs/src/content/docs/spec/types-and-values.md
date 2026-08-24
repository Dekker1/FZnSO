---
title: Types & values
description: How FZnSO describes the type of an argument, how a value carries data across the interface, and how sets and strings are represented.
sidebar:
  order: 5
---

Every declaration named here is in the [ABI types reference](../../reference/abi-types/).

Everything that crosses the interface as data (a constraint argument, a variable domain, an option, a statistic, a solution assignment, an annotation argument) is an `FznsoValue`.
What *shape* of value is expected somewhere is an `FznsoType`.

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

The flags are independent and compose, so the four bits describe sixteen shapes over each of the four base types.
Written out, qualifiers go outermost first: `list of var opt set of int` is `list_of`, `decision`, `opt`, `set_of` and `FznsoTypeBaseInt` all at once.

`decision` is the one worth dwelling on.
It does not mean "this *is* a variable"; it means the position accepts one.
A constraint argument typed `var int` accepts either a decision index or a literal integer, which is why the variables of `int_lin_le` may equally be constants.

A solver declares which decision types it can create through `fznso_<name>_decision_list`, and which argument types each constraint takes through `fznso_<name>_constraint_list`.

## Values

An `FznsoValue` is an [opaque handle](../lifetimes-and-ownership/#handles-are-opaque).
To read one, call `kind()` on its `FznsoValueRef` and then the accessor that answer permits; calling any other is undefined.

### Value kinds

| `FznsoValueKind` | Read with | Notes |
| --- | --- | --- |
| `FznsoValueAbsent` | — | No value. What an optional argument that was omitted, an unknown statistic, or a variable with no explicit domain reports. |
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
It is the ordinary answer for "there is nothing here", and code reading values should expect it wherever a type has `opt` set, wherever a domain is unconstrained, and from any lookup by name that missed.

### Sets

Sets are represented as **range lists**: an ordered sequence of inclusive `[min, max]` ranges, read with `get_range_int` or `get_range_float` for indices below `len`.

This is a compact form for the domains that actually occur (`1..1000000` is one range, not a million elements), and it is the form a solver's own domain representation is usually already in.
The ranges are ordered and non-overlapping, so a set can be scanned in one pass.

### Lists

`FznsoValueList` holds `len` elements, each read with `get_element` and each an `FznsoValue` in its
own right.
Lists nest, and a list may mix kinds, though a list used where the type says `list of var int` will
not.

### A list is flat, and one-based

A list has one dimension and no index set.
That is the whole of it: there is no two-dimensional list, and no list whose first element is
numbered anything but 1.

The consequence is that **a constraint whose arguments are really a matrix, or really an array
numbered from something other than 1, has to say so in its arguments** — the shape cannot travel with
the value.
Two conventions cover it, and both are ordinary arguments a solver reads like any other:

- **A lost dimension travels as a count.**
  A matrix is passed row-major in a flat list, with the row length beside it, unless that length is
  already fixed by another argument.
  `int_table` needs no count because its rows are as long as the array being constrained;
  `int_regular` passes its transition table as a flat list because the state and symbol counts are
  already arguments.
- **A lost index-set origin travels as an offset.**
  Where a constraint reads one array's *values* as indices into another, the number the caller
  indexes from follows that array as a plain `int`, **immediately after it**.
  A list `xs` with offset `k` is read as if it were numbered `k .. k + length(xs) - 1`.
  Where a constraint takes more than one, each is named for the array it belongs to: `f_offset`
  after `f`, `invf_offset` after `invf`.

So `int_circuit(list of var int: xs, int: offset)` takes an offset because `xs[i]` names a position
in `xs`, and `int_inverse` takes two because each array indexes the other.

:::note
A solver can meet an offset by padding the front of its own array, which it can only do from zero
upwards.
An application numbering from below zero should decompose the constraint rather than expect a solver
to accept a negative offset.
:::

## Strings

Strings cross the interface in two forms, and both are **pointer + length, not null-terminated**:

- **`FznsoStr`** — a struct carrying `ptr` and `len` together.
  Used where a string is a *field* of something: a constraint identifier, a decision variable's name, an option's identifier, a message scope.
- **A value of kind `FznsoValueString`** — read the bytes with `as_string` and the byte count with `len`.
  Used where a string is a *value*: a string constraint argument, a string-typed option.

They exist separately because the first is a plain field that can be returned directly, while the second has to go through the value machinery so that a string can appear anywhere any other value can.

Two rules apply to both:

1. **The bytes are UTF-8 and are not null-terminated.**
   A trailing null byte may or may not be present; use the length.
   This lets an implementation hand out a slice of a string it already holds rather than copying it into a null-terminated buffer.
2. **They are borrowed.**
   Copy before storing; see [Lifetimes & ownership](../lifetimes-and-ownership/#nothing-transfers-ownership).

A null `ptr` in an `FznsoStr` signals the *absence* of a string, which is how an optional name or identifier says it is not there.
It is distinct from a present, empty string, which has a non-null `ptr` and a `len` of zero.
