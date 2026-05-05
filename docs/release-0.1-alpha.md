# AX 0.1 Alpha Release Scope

> This file is the release-scope contract for the first public AX alpha. It is not a claim that AX is a mature production language.

## Release Goal

AX 0.1 Alpha should prove this core idea:

```text
same AX source
  -> interpreter can run it
  -> AOT can compile the supported subset
  -> failures are layered
  -> AI/tools can decide whether to edit source, explain a backend gap, or fix tooling
  -> benchmark and parity scripts can verify the result
```

The release is a **Developer Preview / Agent-Native Preview**, not AX 1.0.

## What Is In Scope

- `axc check / run / fmt / build / context`.
- Shared frontend: lexer, parser, semantic checks, HIR, MIR.
- Interpreter as the stable semantic reference.
- LLVM AOT v0 as the native compiler path for the current supported subset.
- `build-manifest.json` schema version `10`.
- `aot_readiness` schema version `3`.
- `AX.toml + sources` project mode.
- First-stage `module/import`.
- Local path package v0 and `AX.lock` v0.
- First `std.*` source modules used by project-backed examples.
- Structured diagnostics and `--json --ai` repair contract.
- Context protocol: `overview / boundaries / topology / flow / symbol / impact / evidence`.
- Repair benchmark evidence chain.
- AOT run-vs-exe parity smoke.
- Curated package registry v0 design, but not public upload.

## AOT Snapshot

At the time of this scope document:

- Default AOT parity smoke covers `123` cases.
- `26` default parity cases are `AX.toml` projects.
- All `26` repository `AX.toml` project examples are listed in `scripts/smoke-aot-parity.ps1`.
- All project examples are expected to be at least AOT IR-ready.
- Most project examples are executable parity cases; side-effect-heavy cases may use controlled failure paths until fixture isolation is upgraded.

The parity smoke runs:

```text
axc check
axc run
axc build --json
native executable
compare exit code / stdout / stderr
```

## Release Validation Baseline

Before cutting an alpha tag, run at least:

```powershell
.\scripts\cargo-gnu.ps1 fmt --check
.\scripts\cargo-gnu.ps1 test --lib backend::llvm
.\scripts\cargo-gnu.ps1 test --lib build::
.\scripts\cargo-gnu.ps1 build --quiet
git diff --check
```

With clang available, also run a representative AOT parity set:

```powershell
$env:AX_LLVM_CLANG = "C:\path\to\clang.exe"
.\scripts\smoke-aot-parity.ps1 -SourcePath @(
  'examples/aot_return.ax',
  'examples/aot_string_runtime.ax',
  'examples/aot_result_try.ax',
  'examples/project_package_math',
  'examples/project_package_config',
  'examples/project_job_runner',
  'examples/project_process_result'
) -OutputRoot 'build\aot-parity-release-0.1-alpha'
```

Full local validation can run:

```powershell
.\scripts\smoke-aot-parity.ps1
```

## Public Claims

Safe claims:

- AX is an AI-native language toolchain.
- AX has a stable interpreter path for the current language mainline.
- AX has an LLVM AOT v0 path for a growing native subset.
- AX shares frontend semantics between interpreter and AOT.
- AX uses structured diagnostics, context, repair contracts, and benchmarks as first-class compiler surfaces.
- AX has local path package v0 and a curated registry plan.

Do not claim:

- AX is production-ready 1.0.
- AX has a mature native backend comparable to Go/Rust/MoonBit.
- AX has a complete package registry.
- AX has a complete standard library.
- AX has stable FFI/native extension ABI.
- AX has complete cross-platform binary distribution.

## Known Alpha Boundaries

- AOT is v0 and still grows by capability package.
- Native memory policy is process-lifetime allocation v0 for strings/lists/runtime helpers.
- More complete package native linking remains future work.
- Full generic trait/impl/method ABI remains future work.
- Registry download/upload is not released in 0.1.
- macOS is not yet a committed support tier.
- Debug info, optimization, incremental compilation, and object/static/dynamic library outputs remain future work.

## Exit Criteria For 0.1 Alpha

- README and docs reflect current parity/project numbers.
- `CONTRIBUTING.md` exists and explains validation.
- Package registry v0 spec exists.
- Release validation baseline passes.
- Known limitations are explicit.
- No user-facing claim says AX is a mature production language.

## Next Phase After 0.1

Recommended next phase:

```text
AX 0.2 Package Preview
  -> in-repo curated package index prototype
  -> axc pkg search/info/check/tree first cut
  -> axc pkg add/install next slice
  -> checksum lock entries
  -> package-backed AOT parity examples
  -> stronger per-case fixture isolation in AOT parity
```
