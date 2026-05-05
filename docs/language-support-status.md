# AX Language Support Status

> This page is the AI-readable status sheet for the current AX language surface.
> It is not a roadmap. Roadmap decisions live in [`../执行路线.md`](../执行路线.md); retired plans live in [`../曾经的计划/`](../曾经的计划/).

AX is growing from an AI-first tool language into a backend-capable language. The current stable execution path is the interpreter. `axc build` emits source, HIR, MIR, a build manifest, and a textual LLVM IR artifact for a growing MIR subset, including single-entry, multi-source, module/import, multiple `std.*` project workloads, host runtime v0, and local path package project-backed AOT. With explicit linking and clang, the current AOT parity smoke compares 123 native executable cases against the interpreter, including 26 `AX.toml` project cases. It still does not guarantee native executable output by default.

## Status Keys

| Mark | Meaning |
| --- | --- |
| `stable` | Supported on the main path and covered by normal validation. |
| `partial` | Implemented in a useful form, but still missing important edge cases or contract freeze. |
| `planned` | Deliberately planned, not implemented yet. |
| `deferred` | Not a near-term mainline task. |
| `not_started` | No implementation has started. |

## Current Feature Matrix

| Feature | parse/fmt | check | run | diagnostics / AI | context | build skeleton | AOT | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `fn`, parameters, explicit return type | stable | stable | stable | stable | stable | stable | partial | LLVM IR v0 supports a minimal single-file `fn main() -> i32` and same-file direct function calls. |
| `let`, `let mut`, assignment, `return` | stable | stable | stable | stable | stable | stable | partial | Explicit local types are part of the AI-first surface; LLVM IR v0 lowers simple locals and returns. |
| Primitive types `bool / i32 / f32 / string` | stable | stable | stable | stable | stable | stable | partial | LLVM IR v0 supports `bool`, `i32`, `f32` core values/arithmetic/comparisons/formatting, and string v0. |
| `if / else`, `while`, `for`, `for in` | stable | stable | stable | stable | stable | stable | partial | LLVM IR v0 can lower basic MIR branch/goto flow for supported value types, including fixed-array `for in`, slice range `for in`, and runtime `[string]` slice iteration returned by `string_split_lines`; richer loop/runtime surfaces remain interpreter-first. |
| Arrays and slices | stable | stable | stable | stable | stable | stable | partial | Fixed array read/write/equality, explicit zero-length array literals such as `let values: [i32; 0] = []`, `to_string(array)`, direct `println(array)`, and slice v0 are in LLVM AOT v0, including fixed-array iteration, `values[start:end]`, direct iteration over slice ranges, `len(slice)`, `slice[index]`, mutable slice element assignment, `to_string(slice)`, direct `println(slice)`, same-file slice parameter calls, slice equality, and `string_split_lines` runtime `[string]` slice iteration. Host/runtime-produced slices beyond `string_split_lines`, cross-project slice ABI, and full ownership/lifetime rules remain pending. Dynamic collections are still represented through dedicated std helpers. |
| Structs and field access | stable | stable | stable | stable | stable | stable | partial | Non-generic struct literal, field read/write, params, returns, field-wise equality, `to_string(struct)`, direct `println(struct)`, and full-field shorthand struct pattern bindings are in LLVM AOT v0; generic structs and partial/nested destructuring remain pending. |
| Enum variants without payload | stable | stable | stable | stable | stable | stable | partial | Non-generic unit enum tags, params, returns, equality, and statement match are in LLVM AOT v0. |
| Payload enum variants | partial | partial | partial | partial | stable | stable | partial | Non-generic payload enum constructor, payload read, params/returns, statement match, payload-aware equality v0, fixed-array/slice payload equality, `to_string(enum)` formatter v0, and direct `println(enum)` for `i32/bool/string` plus fixed-array/struct/slice payloads are in LLVM AOT v0; deeper payload composition and deeper payload equality remain pending. |
| `match` expression / statement | partial | partial | partial | partial | stable | stable | partial | Statement match and expression match v0 are in LLVM AOT parity for simple bindings, payload bindings, full-field shorthand struct bindings, block-valued arms, string literal patterns, `i32` inclusive range patterns, no-binding or patterns, and bool match guards; binding-bearing or patterns and richer nested destructuring remain interpreter-first/native unsupported. |
| `module` / `import` | partial | partial | partial | partial | stable | stable | partial | Minimal explicit module mode; project-backed AOT already has a first slice, but there is still no alias or wildcard import and package visibility remains minimal. |
| `pub` top-level marker | partial | partial | stable | partial | stable | stable | not_started | Visible as syntax, formatter, AST/HIR/MIR, context, and AI focus metadata; package visibility is not fully mature. |
| `const` | stable | stable | stable | stable | stable | stable | partial | Top-level read-only constants are supported; LLVM AOT v0 can inline current AOT-subset `i32/bool/string` const references. |
| Type aliases | partial | partial | partial | partial | stable | stable | not_started | Recursive aliases are not supported. |
| Generic structs / functions / enums | partial | partial | partial | partial | stable | stable | partial | Useful first slice exists; no explicit turbofish calls. LLVM AOT v0 supports concrete generic enum instances, generic structs/functions/impl methods, generic trait impl static dispatch, and Result static constructor inference in the current single-file subset; full project/std monomorphization remains pending. |
| `impl` and methods | partial | partial | partial | partial | stable | stable | partial | Supports value methods, static methods, generic impl, and generic methods; same-file AOT lowering covers the current non-overloaded subset, while mutable receiver and cross-project method ABI remain pending. |
| `trait` / interface | partial | partial | partial | partial | stable | stable | partial | Supports signatures, impl checks, trait bounds, and same-file AOT static dispatch for current bound calls; no dynamic dispatch, associated types, default methods, or generic traits. |
| `Option` / `Result` conventions | partial | partial | partial | partial | stable | stable | partial | Implemented as AX source-level std modules, not hidden Rust crate imports. LLVM AOT v0 supports same-file concrete `Option<i32>` and `Result<i32,string>` enum instances, including formatter/direct print parity; std.option/std.result project imports now have first native parity; broader std/native package linking is still pending. |
| Result propagation `?` | partial | partial | partial | stable | stable | stable | partial | First slice exists for `Result<T, E>`; it is not an exception system and does not perform implicit error conversion. LLVM AOT v0 supports same-file concrete Result instances with Ok unwrap and Err early return; project/std helper methods and richer generic monomorphization remain pending. |
| `std.text` | partial | partial | partial | partial | stable | stable | partial | Std-1 candidate module; current project-backed AOT workloads exercise the first text helper slice. |
| `std.cli` | partial | partial | partial | partial | stable | stable | partial | Std-1 candidate module; current project-backed AOT workloads exercise the first CLI helper slice. |
| `std.fs` | partial | partial | partial | partial | stable | stable | partial | Wraps current host fs builtins behind AX-facing helpers; fs host ABI v0 has AOT parity for the current supported subset. |
| `std.path` | partial | partial | partial | partial | stable | stable | partial | Std-1 candidate path helper layer; path host ABI v0 has AOT parity, including Windows separator behavior. |
| `std.env` | partial | partial | partial | partial | stable | stable | partial | Current host env access remains intentionally explicit; `env_has/env_get` and `std.env.try_get` are in project-backed AOT parity. |
| `std.process` | partial | partial | partial | partial | stable | stable | partial | Host process boundary is intentionally visible to context; process cwd/run/capture ABI v0 has AOT parity for short command-style workloads. |
| `std.report` | partial | partial | partial | partial | stable | stable | partial | Shared reporting helpers for project-backed tools; current AOT parity includes report-producing project workloads. |
| `std.workspace` | partial | partial | partial | partial | stable | stable | partial | Workspace traversal helpers for tool workloads; current AOT parity includes workspace-oriented project workloads. |
| `std.collections` | partial | partial | partial | partial | stable | stable | partial | Currently focused on minimal `string_list` wrappers and queries; `project_collections_core` verifies the source wrappers in project-backed AOT parity. |
| `AX.toml + sources` | stable | stable | stable | stable | stable | stable | partial | Main project organization path; all repository `AX.toml` examples are listed in the default AOT parity script, with project-backed IR readiness and growing executable parity. |
| Local path package v0 | partial | partial | partial | stable | stable | stable | partial | `[dependencies] alias = { path = "..." }` loads local AX package sources as `alias.*` modules; the current AOT path has project-backed local package parity, but full cross-package ABI is not frozen. |
| `AX.lock` v0 | stable | stable | n/a | stable | stable | n/a | n/a | `axc lock <project>` freezes local path package graphs; `axc lock --check` reports stable `LX****` drift reasons and repair hints; no registry lock solving exists yet. |
| Registry packages | partial | partial | partial | stable | stable | partial | partial | Curated registry metadata, `axc pkg search/info/check/tree/add/install/hash`, checksum-backed install preview, and AX-PKG source packages exist. There is still no public upload server, mature ownership model, full semver solver, or native extension ABI. |
| Native AOT executable output | planned | planned | planned | planned | partial | partial | partial | LLVM IR v0 can emit `generated/main.ll` for the current AOT subset, including single-entry, multi-source, module/import, multiple `std.*` project workloads, host runtime v0, and local path package `AX.toml` projects; executable linking is opt-in with `AX_LLVM_AOT_LINK=1` or `--emit exe` and depends on clang. The parity smoke now covers 123 cases and compares exit code, stdout, and stderr against `axc run`; 26 cases are `AX.toml` project examples, and all repository project examples are listed in the default parity script. Coverage includes project-backed AOT, i32 overflow/runtime-error guards, f32 core arithmetic/comparison/formatting, string predicates/runtime helpers, `string_list`, argv/env/fs/path/process host ABI v0, arrays, slices, structs, enums, match variants, concrete generic enum formatting/printing, generic functions/impls/trait-bound static dispatch in the current subset, and Result static constructors/try. |
| JIT | deferred | deferred | deferred | deferred | deferred | deferred | not_started | Only evaluated after AOT proves whether compile latency is a real bottleneck. |
| Closures / lambda | planned | planned | planned | planned | planned | planned | planned | Later language expansion; not part of the current stable core. |
| Async / await | planned | planned | planned | planned | planned | planned | planned | Backend and runtime model must mature first. |
| Network library | planned | planned | planned | planned | planned | planned | planned | Belongs after package, runtime, and async/concurrency design are clearer. |
| Direct Rust crate import | deferred | deferred | deferred | deferred | deferred | deferred | deferred | AX users should depend on AX packages; Rust/native code belongs behind package or host extension contracts. |

## What This Means For Agents

- Prefer explicit types, explicit imports, explicit module paths, and canonical syntax.
- Use `axc check`, `axc run`, `axc fmt`, `axc context evidence`, and `axc lock --check` as the current validation loop.
- Do not assume every valid AX program can be lowered to a native executable yet. Treat `axc build` as a real AOT path for the current supported subset, not as a mature native backend for the whole language.
- Do not generate registry package syntax, direct Rust crate imports, async code, closures, dynamic dispatch, associated types, default trait methods, or generic traits yet.
- When deciding whether a missing feature is a bug or a roadmap item, check this file first, then [`../执行路线.md`](../执行路线.md).
