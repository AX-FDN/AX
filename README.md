<div align="center">
  <img src="./assets/ax-logo.svg" alt="AX logo" width="132" height="132" />

# AX

### AI-native language toolchain

[![License](https://img.shields.io/github/license/AX-FDN/AX)](./LICENSE)
[![Status](https://img.shields.io/badge/status-0.1%20Alpha-0ea5e9)](./docs/release-0.1-alpha.md)
[![AOT](https://img.shields.io/badge/LLVM%20AOT-v0-2563eb)](./docs/llvm-aot.md)
[![Packages](https://img.shields.io/badge/packages-preview-16a34a)](./docs/package-registry-v0.md)

</div>

AX is an AI-native language toolchain. It combines a language frontend, stable
interpreter execution, LLVM AOT v0, structured diagnostics, AI-readable context,
repair benchmark evidence, and a curated package preview.

Current public boundary:

```text
AX 0.1 Alpha / Developer Preview
interpreter-stable + LLVM AOT v0 executable-capable subset + 0.2 Package Preview in progress
```

AX is not a mature production language yet. The honest claim is stronger than
that: AX already has a coherent toolchain shape, and the project is now focused
on making every capability verifiable, layered, and usable by humans and coding
agents.

## Why AX Exists

AX is designed around a simple idea:

```text
source code
  -> compiler understands the layer
  -> diagnostics explain the failure
  -> AI knows whether to edit source, explain a backend gap, verify a package, or fix tooling
  -> tests/smokes/parity prove the result
```

The goal is not only to run code. The goal is to make code generation, repair,
project understanding, and validation part of the same language system.

## Current Snapshot

| Area | Current status |
| --- | --- |
| Interpreter | `axc run` is the stable semantic reference for the current language mainline. |
| Compiler/AOT | LLVM AOT v0 can emit IR and native executables for the supported subset when clang/linking is available. |
| AOT parity | Default run-vs-exe parity covers `123` cases, including `26` project cases. |
| Projects | All `26` repository `AX.toml` project examples are listed in default AOT parity. |
| Build contract | `build-manifest.json` schema version `10`; `aot_readiness.schema_version = 3`. |
| Diagnostics | Text, `--json`, and `--json --ai` outputs exist. |
| Context | `overview / boundaries / topology / flow / symbol / impact / evidence` are compiler-produced views. |
| Packages | Curated registry preview with `31` packages; stable pure-AX smoke covers `29` packages. |
| Package source | Preview packages live in [AX-FDN/AX-PKG](https://github.com/AX-FDN/AX-PKG.git). |
| Std foundations | `std.bytes`, `std.encoding`, `std.json`, `std.hash`, and `std.http` are package-facing foundations. |

## Interpreter And Compiler Together

AX has two execution paths, but not two languages:

| Path | Command | Role |
| --- | --- | --- |
| Interpreter | `axc run <file-or-project>` | Stable semantic execution and reference behavior. |
| AOT compiler | `axc build <file-or-project>` | Emits build artifacts, LLVM IR, and native executable output for the supported subset. |

Both paths share the same lexer, parser, semantic layer, HIR, and MIR pipeline.
New language work should keep this invariant: checking, interpretation, build
artifacts, context, and AOT should all describe the same source-language fact.

## AOT Status

Safe description:

```text
LLVM AOT v0 is executable-capable for a growing native subset.
It is not just an IR demo.
It is not yet a mature native backend.
```

AOT validation compares:

```text
axc check
axc run
axc build --json
native executable
exit code / stdout / stderr
```

Current AOT covers a broad core subset: arithmetic, control flow, strings in the
current runtime subset, f32 core operations, arrays, slices, structs, enums,
match features, Result/Option cases, project-backed examples, local path package
cases, and selected host runtime boundaries such as argv/env/fs/path/process.

Unsupported features must be reported as AOT readiness blockers, lowering
diagnostics, runtime ABI blockers, toolchain issues, or linker issues. They
should not be disguised as user source errors.

## Package Ecosystem Preview

AX now has an early but usable package preview:

- registry metadata lives in this compiler repository under [`registry/`](./registry/)
- package source lives in [AX-PKG](https://github.com/AX-FDN/AX-PKG.git)
- packages are source-only
- metadata pins git URL, revision, path, modules, and checksum
- `axc pkg search/info/check/tree/add/install/hash` exists in the preview slice
- registry install can materialize real pinned packages into the local AX cache

Current registry facts:

- Registry catalog: `31` curated packages.
- Stable pure-AX smoke coverage: `29` packages.
- Host-boundary preview packages: `http_tools` and `net_tools`.
- Package source monorepo: `https://github.com/AX-FDN/AX-PKG.git`.
- Validation entry: [`scripts/smoke-package-registry.ps1`](./scripts/smoke-package-registry.ps1).

Package families include API helpers, auth previews, bytes/encoding/hash tools,
JSON/text/url helpers, cache/retry/pagination, queue/migration/schema workflow,
observability, rate limits, health checks, and host-boundary HTTP/TCP previews.

Important boundaries:

- `jwt_tools` does not sign tokens.
- `hash_tools` and `std.hash` are not cryptographic.
- `database_tools` is not a native database driver.
- `http_tools` and `net_tools` are interpreter-first host-boundary experiments.
- Production TLS, crypto, real DB drivers, and mature network runtime ABI are future work.

## Standard Library Foundation

The standard library is in preview, but it now carries real package-facing
foundation work:

| Module | Current role |
| --- | --- |
| `std.bytes` | Interpreter-stable byte buffers. |
| `std.encoding` | Hex/base64 helpers over bytes. |
| `std.json` | Deterministic JSON string construction. |
| `std.hash` | Deterministic non-cryptographic checksums and cache labels. |
| `std.http` | Pure HTTP request/status/header helpers plus host-boundary `get`. |

The important design rule is separation:

```text
pure helpers
  -> should stay usable without claiming native host runtime support

host/runtime calls
  -> must report explicit readiness blockers until native ABI support exists
```

## Quick Commands

Build the compiler:

```powershell
.\scripts\cargo-gnu.ps1 build --quiet
```

Check and run an AX file:

```powershell
D:\CargoTarget\AX\debug\axc.exe check examples\aot_return.ax
D:\CargoTarget\AX\debug\axc.exe run examples\aot_return.ax
```

Build IR or an executable:

```powershell
D:\CargoTarget\AX\debug\axc.exe build examples\aot_return.ax --emit ir --no-link
D:\CargoTarget\AX\debug\axc.exe build examples\aot_return.ax --emit exe
```

Use package registry preview:

```powershell
D:\CargoTarget\AX\debug\axc.exe pkg search text --registry registry
D:\CargoTarget\AX\debug\axc.exe pkg info http_tools --registry registry
D:\CargoTarget\AX\debug\axc.exe pkg check --registry registry
```

Run focused smokes:

```powershell
.\scripts\smoke-http-helpers.ps1
.\scripts\smoke-json-runtime.ps1
.\scripts\smoke-hash-runtime.ps1
.\scripts\smoke-package-registry.ps1
```

Run the core build test slice:

```powershell
.\scripts\cargo-gnu.ps1 fmt --check
.\scripts\cargo-gnu.ps1 test --lib build::
.\scripts\cargo-gnu.ps1 test --lib backend::llvm
```

## Current Direction

AX is currently converging around two release lines:

```text
0.1 Alpha:
  keep interpreter stable
  keep LLVM AOT v0 honest and executable-capable
  keep diagnostics/context/repair benchmark as first-class compiler surfaces

0.2 Package Preview:
  strengthen curated registry
  strengthen AX-PKG source packages
  strengthen checksum-backed installs
  make package-backed check/run/build/AOT readiness increasingly reliable

1.0 Backend Systems Language:
  Windows + Linux
  default reliable AOT for Backend Profile v1
  runtime ABI before production HTTP/TLS/DB/async
  keep AI-native diagnostics and repair guidance as a core differentiator
```

New work should strengthen one of these loops:

- shared frontend -> interpreter/build/context
- diagnostics -> AI repair contract
- run -> AOT executable parity
- std foundation -> curated package ecosystem
- package install -> lock/check/run/build validation

## Documentation Map

| Need | Document |
| --- | --- |
| Current facts | [`PROJECT_FACTS.md`](./PROJECT_FACTS.md) |
| Public wording boundary | [`docs/public-claims.md`](./docs/public-claims.md) |
| 0.1 Alpha release scope | [`docs/release-0.1-alpha.md`](./docs/release-0.1-alpha.md) |
| 0.2 Package Preview | [`docs/release-0.2-package-preview.md`](./docs/release-0.2-package-preview.md) |
| 1.0 Backend Systems roadmap | [`docs/release-1.0-backend-systems.md`](./docs/release-1.0-backend-systems.md) |
| Backend Profile v1 draft | [`docs/backend-profile-v1.md`](./docs/backend-profile-v1.md) |
| Backend Profile v1 inventory | [`docs/backend-profile-v1-inventory.md`](./docs/backend-profile-v1-inventory.md) |
| Language specification skeleton | [`docs/language-spec.md`](./docs/language-spec.md) |
| Error model skeleton | [`docs/error-model.md`](./docs/error-model.md) |
| Package preview contract | [`docs/package-registry-v0.md`](./docs/package-registry-v0.md) |
| Package maturity | [`docs/package-maturity.md`](./docs/package-maturity.md) |
| AOT details | [`docs/llvm-aot.md`](./docs/llvm-aot.md) |
| Native ABI notes | [`docs/aot-native-abi.md`](./docs/aot-native-abi.md) |
| Validation matrix | [`docs/validation-matrix.md`](./docs/validation-matrix.md) |
| Feature matrix | [`docs/feature-matrix.md`](./docs/feature-matrix.md) |
| Architecture overview | [`docs/architecture.md`](./docs/architecture.md) |
| Contribution guide | [`CONTRIBUTING.md`](./CONTRIBUTING.md) |
| Package source repo | [AX-PKG](https://github.com/AX-FDN/AX-PKG.git) |

## What Not To Claim Yet

AX should not currently claim:

- production-ready 1.0 status
- mature native backend parity with Go/Rust/MoonBit
- complete standard library
- complete public package registry
- production-grade HTTP/TLS/crypto/database/async/FFI support
- universal AI repair superiority over other languages

The confident claim is:

```text
AX is a real AI-native language toolchain preview with a stable interpreter,
an executable-capable LLVM AOT v0 subset, structured diagnostics/context,
repair benchmark evidence, and a growing curated package ecosystem.
```

## License

AX is licensed under [Apache-2.0](./LICENSE).
