# AX 0.2 Package Preview

This file is the execution anchor for the current 0.2 line. It narrows the
project after 0.1 Alpha: do not scatter into random AOT, HTTP, TLS, database, or
crypto work. Package Preview is the mainline.

## Goal

0.2 should prove that AX can support a small, curated, verifiable package
ecosystem without pretending it already has a mature public registry.

The goal is:

```text
std foundations
  -> AX-PKG source packages
  -> registry metadata
  -> checksum-backed install
  -> lock/check/run/build validation
  -> AOT readiness can explain package/std/runtime blockers
```

## Fixed Mainline

0.2 work should stay inside these six lanes:

1. Fix the `0.2 Package Preview` line as the active project direction.
2. Clarify the relationship between `std.*` and AX-PKG.
3. Classify every package by maturity:
   `stable_pure_ax`, `host_boundary_preview`, or `future_native_preview`.
4. Keep package smoke and registry smoke stable.
5. Make AOT readiness more accurate for packages and `std.*`.
6. Do not rush real network/TLS/database/crypto; design runtime ABI boundaries first.

## Current Implemented Slice

Already present:

- `registry/index.json`
- `registry/packages/*.json`
- `axc pkg search`
- `axc pkg info`
- `axc pkg tree`
- `axc pkg check`
- `axc pkg add`
- `axc pkg install`
- `axc pkg hash`
- registry-only `AX.lock` schema v2 preview entries
- git/cache materialization for metadata with real rev/checksum
- AX-PKG source monorepo
- package registry smoke
- first package AOT smoke entry
- focused package-backed AOT readiness smoke for package maturity blockers

Current fixed facts:

- catalog: `32` curated packages
- stable pure-AX smoke: `30` packages
- host-boundary preview packages: `http_tools`, `net_tools`
- source repo: `https://github.com/AX-FDN/AX-PKG.git`

## Std And AX-PKG Rule

`std.*` owns stable language-facing foundations. AX-PKG packages compose them.

Do this:

- put low-level shared primitives in `std.*`
- keep package-level domain helpers in AX-PKG
- avoid duplicating `std.*` behavior inside packages
- keep host/runtime calls explicit
- add focused examples when a package is host-boundary and cannot join stable
  pure-AX smoke

Do not do this:

- hide Rust/native behavior behind package source
- call a package production-grade when it is only a shape/helper preview
- add package APIs that require secrets, accounts, external paid services, or
  machine-specific tools
- treat `std.*` preview modules as a complete frozen standard library

## Maturity Rule

Package maturity is defined in [`package-maturity.md`](./package-maturity.md).

Summary:

| Maturity | Meaning |
| --- | --- |
| `stable_pure_ax` | Deterministic source-only package; no live host IO. |
| `host_boundary_preview` | Wraps explicit host runtime APIs such as HTTP/TCP. |
| `future_native_preview` | Models a future native/security/runtime domain without implementing it yet. |

Every new package should be classified before registry metadata is promoted.

## Smoke Rule

The stable baseline remains:

```powershell
.\scripts\smoke-package-registry.ps1
```

Focused package smokes are preferred over full validation when touching one
package family. Examples:

```powershell
.\scripts\smoke-http-helpers.ps1
.\scripts\smoke-json-runtime.ps1
.\scripts\smoke-hash-runtime.ps1
```

When AOT/package readiness changes, add:

```powershell
.\scripts\cargo-gnu.ps1 test --lib build::
.\scripts\smoke-package-registry-aot.ps1
```

The package AOT smoke is an IR/readiness smoke. It checks package maturity
behavior without requiring native executable linking:

- `stable_pure_ax` does not produce package maturity blockers.
- `host_boundary_preview` produces `AOT0104`.
- `future_native_preview` produces `AOT0105`.

## AOT Readiness Rule

Packages and `std.*` must not confuse source validity with backend maturity.

Correct examples:

```text
valid AX source + bytes runtime usage
  -> bytes_runtime and IR/native parity proof when the current lowering path supports it

valid AX source + host HTTP call
  -> host_http / AOT0301

registry dependency missing lockfile
  -> package-layer blocker such as PX0112
```

Incorrect behavior:

```text
valid source fails build
  -> report a syntax/type/source error
```

## Runtime ABI Before Real Backend Libraries

Do not rush these as normal packages:

- real HTTP client
- TLS
- crypto signing/hashing
- JWT signing/verification
- native database drivers
- async sockets
- FFI/native extensions

Before those become real package capabilities, AX needs clearer runtime ABI for:

- bytes/native byte buffers
- string ownership and allocation
- host error representation
- TLS/certificate behavior
- socket handles or async/concurrency model
- native package linking
- package-backed AOT readiness and parity

## Exit Criteria

0.2 Package Preview is in good shape when:

- package maturity is documented for every package
- registry metadata points to real rev/checksum for promoted packages
- `axc pkg add/install/hash/check/info/search/tree` are documented and stable
- package registry smoke is reliable
- package-backed AOT readiness smoke is reliable
- host-boundary packages have focused examples
- AOT readiness distinguishes pure helpers, host calls, bytes/string/runtime
  ABI blockers, package lock blockers, and toolchain/link blockers
- README, `PROJECT_FACTS.md`, `public-claims.md`, and package docs agree
