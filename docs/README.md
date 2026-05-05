# AX Docs

This directory contains the stable documentation surface for AX. Planning files
can change, but these docs should describe the current project accurately.

## Start Here

| Document | Purpose |
| --- | --- |
| [`../README.md`](../README.md) | Project overview, current capability, quick commands, and contribution entry. |
| [`../PROJECT_FACTS.md`](../PROJECT_FACTS.md) | Current factual baseline. Keep this synchronized with public claims. |
| [`release-0.1-alpha.md`](./release-0.1-alpha.md) | AX 0.1 Alpha / Developer Preview release boundary. |
| [`public-claims.md`](./public-claims.md) | What AX can and cannot safely claim in public. |
| [`package-registry-v0.md`](./package-registry-v0.md) | Curated package registry and package install preview contract. |
| [`llvm-aot.md`](./llvm-aot.md) | LLVM AOT v0 architecture, build artifacts, parity, and limitations. |
| [`aot-native-abi.md`](./aot-native-abi.md) | Native ABI v1 notes for strings, slices, runtime helpers, and memory policy. |
| [`validation-matrix.md`](./validation-matrix.md) | What to run locally and in CI. |
| [`feature-matrix.md`](./feature-matrix.md) | Current language/toolchain feature support matrix. |
| [`language-support-status.md`](./language-support-status.md) | Language support status and remaining gaps. |
| [`benchmark-showcase.md`](./benchmark-showcase.md) | Repair benchmark evidence and current benchmark narrative. |
| [`repair-benchmark.md`](./repair-benchmark.md) | Benchmark workflow and artifacts. |
| [`host-runtime-boundary.md`](./host-runtime-boundary.md) | Host boundary rules for fs/env/process/network/runtime behavior. |
| [`host-runtime-packages.md`](./host-runtime-packages.md) | How package experiments should treat host-runtime capabilities. |
| [`stdlib-minimal-boundary.md`](./stdlib-minimal-boundary.md) | Standard library preview boundary and freeze discipline. |
| [`architecture.md`](./architecture.md) | Compiler architecture overview for maintainers. |
| [`quickstart.md`](./quickstart.md) | Quickstart index. |
| [`quickstart-windows.md`](./quickstart-windows.md) | Windows workflow. |
| [`quickstart-linux.md`](./quickstart-linux.md) | Linux core workflow. |
| [`platform-support.md`](./platform-support.md) | Platform support levels. |

## Current Direction

The project is converging on this line:

```text
AX 0.1 Alpha:
  interpreter-stable
  LLVM AOT v0 executable-capable subset
  structured diagnostics/context/repair
  benchmark evidence

AX 0.2 Package Preview:
  curated registry
  AX-PKG source packages
  checksum-backed installs
  standard-library package foundations
  package-backed validation and AOT readiness
```

Do not split the project into unrelated goals. New work should strengthen one of
these loops:

- shared frontend -> interpreter/build/context
- diagnostics -> AI repair contract
- run -> AOT exe parity
- std foundation -> curated package ecosystem
- package install -> lock/check/run/build validation

## Documentation Rules

- `README.md` is the public front door.
- `PROJECT_FACTS.md` is the factual anchor.
- `public-claims.md` is the wording boundary.
- `release-0.1-alpha.md` is the 0.1 release boundary.
- `package-registry-v0.md` is the 0.2 package preview contract.
- Keep old or retired plans in the archived plans folder, not in the active docs path.

When facts change, update the small set of anchor docs first, then update deeper
topic docs only where the detail actually changed.
