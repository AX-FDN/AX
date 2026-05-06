# AX Project Facts

This file is the current factual anchor for AX. It should stay short, concrete,
and synchronized with `README.md`, `docs/public-claims.md`, and
`docs/release-0.1-alpha.md`.

## One-Line Positioning

AX is an AI-native language toolchain: a stable interpreter path, an LLVM AOT v0
compiler path, structured diagnostics, AI-readable context, repair benchmarks,
and a curated package preview in one repository.

Current public version boundary:

```text
AX 0.1 Alpha / Developer Preview
interpreter-stable + LLVM AOT v0 executable-capable subset + 0.2 Package Preview in progress
```

AX is not a production-ready 1.0 language yet.

Long-range 1.0 direction:

```text
Backend systems language
Windows + Linux
default reliable AOT for Backend Profile v1
runtime ABI before production HTTP/TLS/DB/async
AI-native diagnostics and repair guidance preserved
```

## Current Core Facts

| Area | Current fact |
| --- | --- |
| CLI | `axc check / run / fmt / build / context / pkg` are in the mainline. |
| Frontend | Lexer, parser, semantic checks, HIR, and MIR are shared by interpreter and build paths. |
| Interpreter | `axc run` is the stable semantic reference for the current language mainline. |
| AOT | LLVM AOT v0 can emit IR and, with clang/linker support, native executables for the supported subset. |
| AOT parity | Default run-vs-exe parity covers `123` cases, including `26` `AX.toml` project cases. |
| Projects | All `26` repository `AX.toml` project examples are listed in the default AOT parity script. |
| Build manifest | `build-manifest.json` schema version is `10`. |
| AOT readiness | `aot_readiness.schema_version` is `3`. |
| Diagnostics | Text, `--json`, and `--json --ai` outputs exist. |
| AI repair contract | Diagnostics can carry stable rule ids, repair goals, fixits, context snippets, and validation guidance. |
| Context | `overview / boundaries / topology / flow / symbol / impact / evidence` are compiler-produced views. |
| Benchmarks | Repair benchmark export, run, score, compare, smoke, and deterministic replay assets are in-repo. |
| Package system | Local path package v0, `AX.lock` v0, registry metadata, `axc pkg`, and checksum-backed package install preview exist. |
| Package catalog | Registry catalog has `32` curated packages; stable pure-AX smoke covers `30` packages. |
| Package source | Preview package source lives in `https://github.com/AX-FDN/AX-PKG.git`. |
| Package maturity | Packages are classified as `stable_pure_ax`, `host_boundary_preview`, or `future_native_preview`. |
| Package native smoke | `scripts/smoke-package-registry-native-parity.ps1` verifies stable pure-AX registry packages through run-vs-exe parity, including `json_tools` plus `generic_tools` generic/method coverage. |
| Bytes ABI readiness | `scripts/smoke-bytes-runtime.ps1` verifies interpreter bytes behavior; `scripts/smoke-bytes-native-parity.ps1` verifies `std.bytes` run-vs-exe parity. |
| Host/network ABI readiness | `scripts/smoke-host-network-runtime.ps1` verifies local TCP-backed `std.http`/`std.net` behavior and the `AOT0301/runtime_abi` blocker. |
| 1.0 roadmap | `docs/release-1.0-backend-systems.md` is the active long-range roadmap. |
| Backend profile | `docs/backend-profile-v1.md` defines the draft native-build target. |
| Backend profile inventory | `docs/backend-profile-v1-inventory.md` maps current AOT parity to 1.0 candidates and gaps. |
| Backend profile promotion | `docs/backend-profile-v1-promotion.md` marks capabilities as candidate, profile-blocked, or future. |
| Standard library | `std.bytes`, `std.encoding`, `std.json`, `std.hash`, and `std.http` now provide package-facing foundations. |
| Platform | Windows has the fullest workflow. Linux has core support. macOS is not yet a committed support tier. |

## Current Language Shape

The current language mainline includes:

- explicit typed functions and local variables
- `i32`, `bool`, `f32`, `string`, `bytes`
- arrays, slices, structs, enums, payload enums
- `if`, `while`, `for`, `for in`, `break`, `continue`
- `match`, range patterns, or patterns, match guards, struct patterns
- generics for functions, structs, enums, impls, and methods in the current subset
- traits/interfaces and trait bounds in the current subset
- `module/import`, `AX.toml + sources`, and local path package modules
- official `std.option` and `std.result` conventions, including `Result`-style propagation

This is enough for tools, scripts, project examples, package experiments, and
backend-adjacent helper workloads. It is not yet a complete general-purpose
language surface.

## Current AOT Boundary

Safe factual statement:

```text
LLVM AOT v0 is an executable-capable subset.
It is not only an IR demo, and it is not yet a mature native backend.
```

AOT currently proves value by comparing:

```text
axc run
axc build
native executable
exit code / stdout / stderr
```

Unsupported native features should appear as AOT readiness blockers, lowering
diagnostics, runtime ABI blockers, or toolchain/link blockers. They should not be
misreported as user source errors.

## Current Package Boundary

AX now has a real package preview:

- curated registry metadata lives in this repository under `registry/`
- package source currently lives in `AX-FDN/AX-PKG`
- packages are source-only
- package metadata pins git URL, revision, path, module list, and checksum
- `axc pkg search/info/check/tree/add/install/hash` exists in the preview slice
- registry install can materialize packages into the local AX cache when metadata
  has real revisions and checksums

Current non-goals:

- no public upload server
- no account/login/token system
- no native binary package install
- no package install scripts
- no production-grade semver solver
- no automatic trust of arbitrary package code

## Current Standard Library Boundary

`std.*` is no longer just a placeholder, but it is still a preview foundation.

Current foundations include:

- `std.bytes`: interpreter-stable byte buffers
- `std.encoding`: hex/base64 helpers over bytes
- `std.json`: deterministic JSON string construction helpers
- `std.hash`: deterministic non-cryptographic checksum helpers
- `std.http`: pure request/status/header helpers plus host-boundary `get`

Important boundaries:

- `std.hash` is not cryptographic and must not be used for passwords, JWT signing, MACs, or security.
- `std.http` pure helpers are separate from real host networking.
- bytes/string/native HTTP runtime ABI work is still an AOT/backend task.

## What AX Should Be Evaluated On

AX should be judged by whether these loops keep getting stronger:

1. One frontend feeds checking, interpretation, context, build artifacts, and AOT.
2. Interpreter behavior is the semantic reference.
3. AOT expands by verified capability slices.
4. Every failure is classified by layer.
5. AI receives enough structured information to decide whether to edit source,
   explain a backend limit, verify a lockfile, install tooling, or report an
   internal compiler bug.
6. Benchmarks, snapshots, smokes, and parity scripts prove changes instead of
   relying on vague claims.

## What Not To Claim

Do not say:

- AX is production-ready.
- AX replaces Go, Rust, MoonBit, Python, or TypeScript.
- AX has a mature native backend.
- AX has a complete standard library.
- AX has a full public package registry.
- AX has production-grade HTTP, TLS, crypto, database drivers, FFI, async, or IDE support.

Accurate claim:

```text
AX is a credible AI-native language toolchain preview with a stable interpreter,
an executable-capable LLVM AOT v0 subset, structured diagnostics/context, repair
benchmarks, and an early curated package ecosystem.
```

## Update Rule

Update this file whenever any of these facts change:

- AOT parity count or project parity count
- manifest schema version
- AOT readiness schema version
- package catalog count or stable smoke count
- registry install behavior
- standard library foundation modules
- platform support tier
- public release boundary
