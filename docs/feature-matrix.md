# AX Feature Matrix

This page answers one question: what does AX currently support?

It is not a roadmap. Active planning should point back to the current release
line:

```text
0.1 Alpha / Developer Preview
0.2 Package Preview in progress
1.0 Backend Systems Language roadmap
```

## Status Keys

| Mark | Meaning |
| --- | --- |
| `[x]` | Mainline capability with normal validation. |
| `[~]` | Useful preview or partial implementation; still expanding or not fully frozen. |
| `[ ]` | Not implemented or intentionally deferred. |

## Main Capability Matrix

| Area | Status | Current fact | Boundary |
| --- | --- | --- | --- |
| CLI | `[x]` | `axc check / run / fmt / build / context / pkg` exist. | CLI polish and docs continue. |
| Frontend | `[x]` | Lexer, parser, semantic checks, HIR, and MIR are shared. | New syntax must keep interpreter/build/context aligned. |
| Interpreter | `[x]` | `axc run` is the stable semantic reference. | Host/runtime edge cases continue to harden. |
| Diagnostics | `[x]` | Text, `--json`, and `--json --ai` outputs exist. | More rules can be added, but protocol stability matters. |
| AI context | `[x]` | `overview / boundaries / topology / flow / symbol / impact / evidence`. | Live-model impact claims still need future evidence. |
| Repair benchmark | `[x]` | Export, replay, score, compare, smoke, and deterministic evidence assets exist. | Cross-language/live-model claims are future work. |
| Project mode | `[x]` | `AX.toml + sources` is the main project organization path. | Visibility/package semantics are still minimal. |
| Module/import | `[~]` | First explicit module/import mode works. | No mature alias/wildcard/package visibility model yet. |
| Local path packages | `[~]` | `[dependencies] alias = { path = "..." }` works with `AX.lock` v0. | Full cross-package native ABI is not frozen. |
| Registry packages | `[~]` | Curated registry metadata, `axc pkg`, checksum-backed install preview, and AX-PKG source repo exist. | No public upload server or mature semver solver. |
| Standard library | `[~]` | Tooling std modules plus `std.bytes / encoding / json / hash / http` foundations exist. | Not a complete standard library. |
| AOT build | `[~]` | LLVM AOT v0 can emit IR and supported native executables with clang/linking. | Not a mature production native backend. |
| Backend Profile v1 | `[~]` | Draft roadmap exists for the 1.0 native-build target. | Not yet an implemented 1.0 backend profile. |
| AOT parity | `[~]` | Default run-vs-exe parity covers `123` cases, including `26` project cases. | Parity only proves the supported subset. |
| Native ABI | `[~]` | Native ABI v1 docs and runtime/symbol/linking foundations exist. | Ownership/free/GC/FFI are not mature. |
| Platform | `[~]` | Windows has full workflow; Linux has core support. | macOS is not yet a committed support tier. |
| Public package upload | `[ ]` | Not implemented. | Curated review comes first. |
| Production HTTP/TLS/DB/crypto | `[ ]` | Pure helpers and host-boundary previews exist. | Real runtime/native ABI work remains. |
| Async/JIT/IDE/FFI | `[ ]` | Deferred. | Not current release scope. |

## Language Surface

| Feature | Status | Current fact |
| --- | --- | --- |
| Functions, params, returns | `[x]` | Explicit typed functions are supported. |
| Locals and assignment | `[x]` | `let`, `let mut`, assignment, and `return` are supported. |
| Primitive values | `[x]` | `bool`, `i32`, `f32`, `string`, and `bytes` exist on the mainline. |
| Control flow | `[x]` | `if`, `while`, `for`, `for in`, `break`, and `continue` exist. |
| Arrays and slices | `[~]` | Useful fixed array and slice support exists; ownership/lifetime is not complete. |
| Structs | `[~]` | Struct literal, field read/write, equality/formatting in current subsets. |
| Enums | `[~]` | Unit and payload enums work in current subsets. |
| Match | `[~]` | Statement/expression match, range/or/guard/struct patterns in current subsets. |
| Generics | `[~]` | Useful generic functions/structs/enums/impls/methods exist. |
| Traits | `[~]` | Traits, impl checks, and trait bounds exist in the current static subset. |
| Option/Result | `[~]` | `std.option` and `std.result` conventions exist, including first `?` support. |
| Methods/impl | `[~]` | Value/static/generic methods exist in current subsets. |
| Async/closures/macros | `[ ]` | Deferred. |

## AOT Facts

Current accurate wording:

```text
LLVM AOT v0 is executable-capable for the supported subset.
It is not just an IR demo.
It is not yet a mature native backend.
```

Current fixed facts:

- default parity cases: `123`
- project parity cases: `26`
- repository project examples in parity list: `26`
- build manifest schema: `10`
- AOT readiness schema: `3`

## Package Facts

Current fixed facts:

- registry catalog: `31` curated packages
- stable pure-AX package smoke: `29` packages
- host-boundary preview packages: `http_tools`, `net_tools`
- package source repository: `https://github.com/AX-FDN/AX-PKG.git`
- package-backed AOT readiness smoke: `scripts/smoke-package-registry-aot.ps1`

Package v0 is deliberately curated and source-only. It is useful now, but it is
not a public upload ecosystem yet.

## Agent Guidance

Agents should:

- prefer explicit types and explicit imports
- run `axc check`, `axc run`, and focused smokes before broad validation
- treat `axc build` blockers as backend/toolchain/runtime-layer facts, not
  automatic source bugs
- use package/lock diagnostics before editing dependency source
- avoid generating async, FFI, public upload, real crypto, TLS, or DB-driver code
  unless a task is explicitly about designing those future layers
