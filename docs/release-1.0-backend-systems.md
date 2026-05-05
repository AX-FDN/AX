# AX 1.0 Backend Systems Language Roadmap

This document is the active long-range roadmap after 0.1 Alpha and 0.2 Package
Preview. It does not replace the current package-preview execution line; it
explains how those foundations grow into a mature backend systems language.

## 1.0 Target

AX 1.0 is targeted as:

```text
Windows + Linux
default reliable AOT for Backend Profile v1
backend systems language
HTTP / TLS / JSON / DB / async on the roadmap
AI-native diagnostics and repair contract preserved
```

The goal is not to win by adding the most syntax. The goal is to make backend
projects stable across checking, interpretation, native build, package
validation, and AI-assisted repair.

## Mainline Order

```text
0.2 Package Preview
  -> Language Spec Freeze
  -> Native Runtime ABI v1
  -> Reliable AOT Backend Profile v1
  -> Backend Standard Library v1
  -> Async/IO Runtime v1
  -> Package/Registry Stability
  -> LSP/VSCode
  -> 1.0 Release Candidate
```

## Milestones

| Milestone | Goal | Exit criteria |
| --- | --- | --- |
| A. Package Preview Done | Packages participate in check/run/build/readiness. | package-backed AOT smoke, registry manifest contract, maturity blockers. |
| B. Language Spec Freeze | Language behavior is a contract, not just code. | spec docs map to examples/tests and current feature matrix. |
| C. Runtime ABI v1 | Host/backend libraries share one ABI model. | string/bytes/slice/result/error/handle ownership documented and implemented through backend ABI entry points. |
| D. Backend Profile v1 | AOT has a default reliable subset. | profile examples pass check/run/build/exe parity on Windows and Linux. |
| E. Backend Std v1 | Backend services can be written in AX. | JSON, HTTP, TLS, TCP, PostgreSQL, and crypto boundaries are explicit. |
| F. Async Runtime v1 | IO can be structured without blocking-only scripts. | async model frozen and connected to HTTP/TCP/DB readiness. |
| G. Tooling/IDE v1 | AX is usable outside the terminal. | VSCode/LSP diagnostics, hover, definitions, symbols, run/build tasks. |
| H. 1.0 RC | Release candidate. | Windows + Linux CI, parity, package smoke, backend demo, repair benchmark baseline. |

## Immediate Work

The next implementation work should stay in this order:

1. Expand package-backed AOT fixture smoke beyond the first `stable_pure_ax`
   native parity case.
2. Backend Profile v1 draft.
3. Native Runtime ABI v1 draft.
4. Language Spec v1 skeleton.
5. Backend demo target.

The next profile-tracking document is
[`backend-profile-v1-inventory.md`](./backend-profile-v1-inventory.md), which
maps current executable parity coverage to Backend Profile v1 candidates and
remaining gaps.

Do not start production HTTP/TLS/DB/async implementation until the ABI and
Backend Profile documents are in place.

## 1.0 Non-Goals

AX 1.0 should not promise:

- macOS support as a first-tier platform.
- public upload registry.
- install scripts or binary native packages.
- complete semver dependency solving.
- full FFI/native extension ABI.
- all future language features in AOT.
- production crypto unless `std.crypto` has a reviewed contract.

## Evaluation Standard

AX 1.0 should be judged by these loops:

- one frontend feeds check, run, context, build artifacts, and AOT.
- interpreter behavior remains the semantic reference.
- Backend Profile v1 builds native executables by default.
- every failure has a layer, code, action, and validation path.
- packages are curated, checksum-backed, and maturity-classified.
- AI can decide whether to edit source, explain a backend gap, verify a package,
  configure tooling, or report a compiler bug.
