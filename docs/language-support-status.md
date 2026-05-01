# AX Language Support Status

> This page is the AI-readable status sheet for the current AX language surface.
> It is not a roadmap. Roadmap decisions live in [`../PLAN.md`](../PLAN.md), and active tasks live in [`../WORKLIST.md`](../WORKLIST.md).

AX is growing from an AI-first tool language into a backend-capable language. The current stable execution path is the interpreter. `axc build` emits source, HIR, MIR, a build manifest, and now a textual LLVM IR artifact for a very small single-file MIR subset. It still does not guarantee native executable output by default.

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
| Primitive types `bool / i32 / f32 / string` | stable | stable | stable | stable | stable | stable | partial | LLVM IR v0 supports `bool` and `i32`; `f32` and `string` still need native ABI work. |
| `if / else`, `while`, `for`, `for in` | stable | stable | stable | stable | stable | stable | partial | LLVM IR v0 can lower basic MIR branch/goto flow for supported value types; richer loop/runtime surfaces remain interpreter-first. |
| Arrays and read-only slices | stable | stable | stable | stable | stable | stable | not_started | Dynamic collections are still represented through dedicated std helpers. |
| Structs and field access | stable | stable | stable | stable | stable | stable | not_started | Struct literals are supported. |
| Enum variants without payload | stable | stable | stable | stable | stable | stable | not_started | Mainline enum support is usable. |
| Payload enum variants | partial | partial | partial | partial | stable | stable | not_started | Usable in representative project examples; still not a complete algebraic data type system. |
| `match` expression / statement | partial | partial | partial | partial | stable | stable | not_started | Supports common variant, literal, range, multi-pattern, guard, binding, linear block-valued expression arm cases, and full-field shorthand struct destructuring. Struct patterns require every declared field exactly once and reject alias/partial/duplicate/unknown fields; no nested/array/tuple destructuring yet. |
| `module` / `import` | partial | partial | partial | partial | stable | stable | not_started | Minimal explicit module mode; no alias or wildcard import. |
| `pub` top-level marker | partial | partial | stable | partial | stable | stable | not_started | Visible as syntax, formatter, AST/HIR/MIR, context, and AI focus metadata; package visibility is not fully mature. |
| `const` | stable | stable | stable | stable | stable | stable | not_started | Top-level read-only constants are supported. |
| Type aliases | partial | partial | partial | partial | stable | stable | not_started | Recursive aliases are not supported. |
| Generic structs / functions / enums | partial | partial | partial | partial | stable | stable | not_started | Useful first slice exists; no explicit turbofish calls. |
| `impl` and methods | partial | partial | partial | partial | stable | stable | not_started | Supports value methods, static methods, generic impl, and generic methods; no overloads or mutable receiver model yet. |
| `trait` / interface | partial | partial | partial | partial | stable | stable | not_started | Supports signatures, impl checks, and trait bounds; no dynamic dispatch, associated types, default methods, or generic traits. |
| `Option` / `Result` conventions | partial | partial | partial | partial | stable | stable | not_started | Implemented as AX source-level std modules, not hidden Rust crate imports. |
| Result propagation `?` | partial | partial | partial | stable | stable | stable | not_started | First slice exists for `Result<T, E>`; it is not an exception system and does not perform implicit error conversion. |
| `std.text` | partial | partial | partial | partial | stable | stable | not_started | Std-1 candidate module. |
| `std.cli` | partial | partial | partial | partial | stable | stable | not_started | Std-1 candidate module. |
| `std.fs` | partial | partial | partial | partial | stable | stable | not_started | Wraps current host fs builtins behind AX-facing helpers. |
| `std.path` | partial | partial | partial | partial | stable | stable | not_started | Std-1 candidate path helper layer. |
| `std.env` | partial | partial | partial | partial | stable | stable | not_started | Current host env access remains intentionally explicit. |
| `std.process` | partial | partial | partial | partial | stable | stable | not_started | Host process boundary is intentionally visible to context. |
| `std.report` | partial | partial | partial | partial | stable | stable | not_started | Shared reporting helpers for project-backed tools. |
| `std.workspace` | partial | partial | partial | partial | stable | stable | not_started | Workspace traversal helpers for tool workloads. |
| `std.collections` | partial | partial | partial | partial | stable | stable | not_started | Currently focused on minimal `string_list` wrappers and queries. |
| `AX.toml + sources` | stable | stable | stable | stable | stable | stable | not_started | Main project organization path. |
| Local path package v0 | partial | partial | partial | stable | stable | stable | not_started | `[dependencies] alias = { path = "..." }` loads local AX package sources as `alias.*` modules. |
| `AX.lock` v0 | stable | stable | n/a | stable | stable | n/a | n/a | `axc lock <project>` freezes local path package graphs; `axc lock --check` reports stable `LX****` drift reasons and repair hints; no registry lock solving exists yet. |
| Registry packages | planned | planned | planned | planned | planned | planned | planned | P5+ work after standard library and AOT are stable enough. |
| Native AOT executable output | planned | planned | planned | planned | partial | partial | partial | LLVM IR v0 can emit `generated/main.ll` for `examples/aot_return.ax`; executable linking is opt-in with `AX_LLVM_AOT_LINK=1` and depends on clang. |
| JIT | deferred | deferred | deferred | deferred | deferred | deferred | not_started | Only evaluated after AOT proves whether compile latency is a real bottleneck. |
| Closures / lambda | planned | planned | planned | planned | planned | planned | planned | Later language expansion; not part of the current stable core. |
| Async / await | planned | planned | planned | planned | planned | planned | planned | Backend and runtime model must mature first. |
| Network library | planned | planned | planned | planned | planned | planned | planned | Belongs after package, runtime, and async/concurrency design are clearer. |
| Direct Rust crate import | deferred | deferred | deferred | deferred | deferred | deferred | deferred | AX users should depend on AX packages; Rust/native code belongs behind package or host extension contracts. |

## What This Means For Agents

- Prefer explicit types, explicit imports, explicit module paths, and canonical syntax.
- Use `axc check`, `axc run`, `axc fmt`, `axc context evidence`, and `axc lock --check` as the current validation loop.
- Do not assume `axc build` produces a native executable. Treat LLVM IR generation as a Build-1 prototype artifact, not as a mature native backend.
- Do not generate registry package syntax, direct Rust crate imports, async code, closures, dynamic dispatch, associated types, default trait methods, or generic traits yet.
- When deciding whether a missing feature is a bug or a roadmap item, check this file first, then [`../WORKLIST.md`](../WORKLIST.md).
