# AX Type System v1 Skeleton

This document defines the type-system areas that must be frozen before AX 1.0.

## Stable Core Types

- `bool`
- `i32`
- `f32`
- `string`
- `bytes`

Each core type must define:

- literal syntax
- equality and comparison rules
- formatting rules
- interpreter behavior
- AOT representation in Backend Profile v1

## Composite Types

The v1 contract must cover:

- fixed arrays `[T; N]`
- slices `[T]`
- structs
- unit enums
- payload enums
- `Option<T>`
- `Result<T,E>`

For each composite type, the spec must define construction, field or element
access, equality, formatting, function parameter/return behavior, and AOT
readiness boundaries.

## Generics

Current AX has useful generic functions, structs, enums, impls, and methods, but
1.0 must distinguish:

- generic behavior supported by interpreter/check
- generic behavior inside Backend Profile v1
- generic behavior that remains preview or future

Cross-package monomorphization must be specified before generic package APIs can
be called mature native ABI.

## Traits And Impl

The v1 spec must define:

- trait declarations
- trait bounds
- impl blocks
- method lookup
- static methods
- method receiver rules
- AOT method ABI status

Full dynamic dispatch is not required for 1.0 unless explicitly promoted later.

## Result Propagation

The `?` operator must define:

- accepted expression types
- Ok payload continuation
- Err early return
- interaction with function return type
- diagnostics for invalid use
- AOT lowering for supported concrete instances
