# Contributing to AX

AX is an AI-native language toolchain. Contributions are welcome when they make the language easier to generate, check, run, compile, repair, or validate.

This repository is currently moving toward **AX 0.1 Alpha**. That means we value focused, verifiable changes over large speculative additions.

## Current Contribution Areas

Good early contributions:

- `std/` helpers that are consumed by real `examples/project_*` workloads.
- New project-backed examples that exercise a clear language, std, package, or AOT boundary.
- AOT parity cases in `scripts/smoke-aot-parity.ps1`.
- Diagnostics and AI repair-rule improvements with stable error-layer intent.
- Documentation that clarifies current behavior, limitations, or contribution flow.
- Package metadata experiments for the curated registry v0 plan.

Please avoid starting with:

- A full registry server.
- Async runtime, web framework, database drivers, or macro system.
- Large syntax additions without representative examples and diagnostics.
- Native FFI or install scripts for packages.
- Big rewrites that do not improve a current validation path.

## Local Setup

Install Rust stable. On Windows, AX's local baseline uses the repository cargo wrapper:

```powershell
.\scripts\cargo-gnu.ps1 build --quiet
```

For AOT executable smoke tests, install clang and set `AX_LLVM_CLANG` if clang is not on `PATH`:

```powershell
$env:AX_LLVM_CLANG = "C:\path\to\clang.exe"
```

## Useful Commands

Fast formatting check:

```powershell
.\scripts\cargo-gnu.ps1 fmt --check
```

Backend/AOT-focused tests:

```powershell
.\scripts\cargo-gnu.ps1 test --lib backend::llvm
.\scripts\cargo-gnu.ps1 test --lib build::
```

General library tests:

```powershell
.\scripts\cargo-gnu.ps1 test --lib
```

Interface snapshots:

```powershell
.\scripts\cargo-gnu.ps1 test --test interface_snapshots
```

AOT run-vs-exe parity, requires clang:

```powershell
.\scripts\smoke-aot-parity.ps1
```

Whitespace check before submitting:

```powershell
git diff --check
```

## Adding a Language or AOT Feature

A feature is not considered done just because code compiles. Prefer this checklist:

- Add or update an AX example that naturally uses the feature.
- Ensure `axc check` accepts valid code and rejects invalid code with a useful diagnostic.
- If the interpreter supports it, make sure `axc run` has a representative path.
- If AOT supports it, add or update run-vs-exe parity.
- If AOT does not support it yet, ensure the unsupported path is reported as an AOT/backend blocker, not as a user-code error.
- Update docs only with facts that are now covered by tests or smoke scripts.

## Adding a `std/` Helper

Prefer small helpers that are used by at least one project-backed workload. A `std/` helper should normally come with:

- Source in `std/`.
- A representative `examples/project_*` consumer.
- Interface snapshot or smoke coverage.
- AOT parity if the helper is inside the current native subset.

## Adding a Package Example

AX currently supports local path package v0. Good package examples should include:

- A package `AX.toml`.
- Source modules under `src/`.
- A consuming project with `[dependencies] alias = { path = "..." }`.
- `AX.lock` when the package graph is intended to be reproducible.
- A `check/run/build` validation path.

Registry packages are not open upload yet. See `docs/package-registry-v0.md` for the staged package plan.

## Pull Request Expectations

Keep pull requests narrow. Include:

- What changed.
- Why the change belongs in the current AX phase.
- Which commands were run.
- Any remaining limitation or intentionally unsupported case.

Do not revert unrelated changes in a dirty worktree. AX development often has active feature branches with overlapping docs, examples, and backend files.

