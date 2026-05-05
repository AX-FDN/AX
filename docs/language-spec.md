# AX Language Specification v1 Skeleton

This file is the top-level language contract for AX 1.0 planning. It currently
maps the existing implementation into a specification shape; individual details
will be tightened as tests and examples are attached.

## Source Files

- AX source files use UTF-8 without a required BOM.
- A project is described by `AX.toml`.
- A project entry defaults to `src/main.ax` unless configured.
- Additional project sources are listed in `AX.toml`.

## Lexical Structure

The specification must cover:

- identifiers
- keywords
- integer, float, bool, string, and bytes literals
- comments
- operators
- delimiters
- invalid-character diagnostics

## Items

Current item families:

- function
- const
- type alias
- struct
- enum
- trait
- impl
- module declaration
- import declaration

## Statements

Current statement families:

- `let` and `let mut`
- assignment
- expression statement
- `return`
- `break`
- `continue`
- `if`
- `while`
- `for`
- `for in`
- block
- match statement

## Expressions

Current expression families:

- literals
- names and qualified names
- calls
- unary and binary expressions
- struct literals
- array literals
- field access
- index access
- slice expressions
- block expressions
- match expressions
- `?` result propagation

## Types

See `type-system.md` for the detailed contract.

Required v1 type families:

- `i32`
- `bool`
- `f32`
- `string`
- `bytes`
- arrays and slices
- structs
- enums and payload enums
- generic type instances
- `Option<T>`
- `Result<T,E>`

## Modules And Packages

See `module-system.md` and `package-semantics.md`.

## Execution Paths

The same language facts must feed:

- `axc check`
- `axc run`
- `axc build`
- `axc context`
- diagnostics and AI guidance

Unsupported native behavior must be reported as an AOT/runtime/toolchain/package
blocker, not as a fake source error.

## Spec Freeze Rule

After v1 freeze, new language features must update:

- this specification
- examples or focused tests
- diagnostics behavior
- interpreter behavior
- build/AOT readiness behavior
- context behavior when applicable
