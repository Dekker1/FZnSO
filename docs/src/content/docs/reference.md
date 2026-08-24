---
title: About the reference
description: What each reference page is generated from, and how to read it.
sidebar:
  order: 0
---

The reference is generated from the code itself, on every build.
Nothing on these pages is hand-written, so nothing on them can drift from what the code actually says.

## What is generated from what

| Page | Generated from | Via |
| --- | --- | --- |
| [ABI types](abi-types/) | `rust/fznso-types` | rustdoc JSON |
| [ABI entry points](abi-entry-points/) | `rust/fznso-solver-template` | rustdoc JSON |
| [Rust bindings](rust/) | `rust/fznso` | rustdoc JSON |
| [C++ bindings](cpp/) | `cpp/fznso.hpp` | Doxygen XML |
| [C++ solver export](cpp-export/) | `cpp/fznso_export.hpp` | Doxygen XML |
| [Python bindings](python/) | `python/src/lib.rs` | griffe |

## The ABI has one page, not two

There is no separate C reference, by design.

`c/fznso_types.h` is generated from `rust/fznso-types` by cbindgen, so the C struct and the Rust type
are the same declaration with two spellings.
Documenting them twice would be the [duplication this documentation is organised to avoid](../contributing/build-and-test/#writing-documentation), and the copy that drifted would be the one a C
programmer read.

So [ABI types](abi-types/) is the C reference as much as the Rust one.
Where the two spellings differ (`FznsoPtr<'a, T>` in Rust is `const FznsoX *` in C), the difference
is [C-ABI-neutral](../spec/lifetimes-and-ownership/#handles-are-opaque) and the header is the
authority on the C form.

## How to read a page

Each entry shows its signature, its documentation, and a link to the line of source it came from.
Members (struct fields, enum variants, methods, properties) are listed under the type that owns
them, and each has its own anchor, so the specification can link straight to a single callback.

The filter box at the top of each page narrows within that page.
The site search covers all of them at once, alongside the guides, the specification and the registry.

## Normative status

**The reference describes; the [specification](../spec/overview/) governs.**

A doc comment states the contract of one function, type or field.
Rules that span the interface (what a handle is, how long a borrow lasts, what a multi-threaded
solver may do) are specified once in the spec, and the reference links to it rather than restating
it.
Where the two ever disagree, the specification is what an implementation is measured against.
