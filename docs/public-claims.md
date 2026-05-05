# AX Public Claims Boundary

This document defines how AX should be described publicly. The goal is to sound
confident without overstating maturity.

## Recommended Short Description

Use this:

```text
AX is an AI-native language toolchain with a stable interpreter path, an LLVM
AOT v0 native compiler path, structured diagnostics, AI-readable context, repair
benchmarks, and an early curated package ecosystem.
```

Chinese version:

```text
AX 是一套 AI-native 语言工具链：解释器稳定执行，LLVM AOT v0 持续扩展，
结构化诊断、AI 上下文、修复 benchmark 和 curated package preview 都是一等能力。
```

## Safe Claims

These are safe to say now:

- AX has `axc check / run / fmt / build / context / pkg`.
- AX has a shared frontend for checking, interpretation, and AOT build artifacts.
- `axc run` is the stable semantic reference for the current language mainline.
- `axc build` has LLVM AOT v0 and can generate native executables for the supported subset when clang/linking is available.
- Default AOT parity covers `123` cases, including `26` project cases.
- All `26` repository `AX.toml` project examples are listed in default AOT parity.
- `build-manifest.json` schema version is `10`.
- `aot_readiness.schema_version` is `3`.
- AX has structured diagnostics and `--json --ai`.
- AX context has `overview / boundaries / topology / flow / symbol / impact / evidence`.
- AX has repair benchmark infrastructure and deterministic replay assets.
- AX has local path package v0, `AX.lock`, registry metadata, `axc pkg`, and a checksum-backed package install preview.
- AX has a curated package catalog with `31` packages and stable pure-AX smoke coverage for `29` packages.
- Package source currently lives in `https://github.com/AX-FDN/AX-PKG.git`.
- `std.bytes`, `std.encoding`, `std.json`, `std.hash`, and `std.http` provide current package-facing foundations.

## Claims That Need Careful Wording

Use careful wording for these:

| Topic | Accurate wording |
| --- | --- |
| AOT | "LLVM AOT v0 is executable-capable for the supported subset." |
| Backend maturity | "The native backend is growing by verified capability slices." |
| Package ecosystem | "AX has an early curated package preview." |
| Standard library | "AX has standard-library foundations, not a complete stdlib." |
| HTTP/network | "AX has pure HTTP helpers and interpreter-first host-boundary experiments." |
| AI repair | "AX has structured repair inputs and benchmark evidence loops." |
| Backend language direction | "AX is growing toward backend worker and service-support workloads." |

## Do Not Claim

Do not say:

- AX is production-ready 1.0.
- AX replaces Go, Rust, MoonBit, Python, or TypeScript.
- AX has a mature native backend comparable to established production languages.
- AX has a complete standard library.
- AX has a full public package registry with open upload.
- AX has production-grade HTTP, TLS, crypto, JWT signing, database drivers, async runtime, or FFI.
- AX has proven universal AI repair superiority over other languages.
- AX has completed cross-language or live-model benchmark conclusions unless those artifacts are explicitly added to the repository.

## Benchmark Wording

Safe wording:

```text
AX has repository-internal deterministic repair benchmark assets and a repair
evidence loop. These validate the internal repair workflow; cross-language and
live-model claims remain future work.
```

Avoid:

```text
AX has proven that all AI models repair AX better than other languages.
```

## AOT Wording

Safe wording:

```text
AX AOT is v0 but executable-capable. The default parity smoke compares
interpreter output and native executable output across 123 cases.
```

Avoid:

```text
AX already has a mature production native backend.
```

## Package Wording

Safe wording:

```text
AX has a curated package preview. The compiler repository owns registry metadata
and AX-PKG stores package source. Packages are checksum-backed and source-only.
```

Avoid:

```text
AX already has a complete public package ecosystem like npm/crates.io.
```

## Standard Library Wording

Safe wording:

```text
AX standard-library foundations now cover bytes, encoding, JSON string
construction, non-cryptographic checksums, and HTTP request/status helpers.
```

Avoid:

```text
AX has a complete production standard library.
```

## Update Rule

Update this file together with `README.md` and `PROJECT_FACTS.md` when:

- AOT parity numbers change
- package catalog numbers change
- manifest or readiness schema versions change
- package install/publish behavior changes
- standard library foundation modules change
- release scope changes
