# AX Quickstart: Windows

This is the current full-workflow source-install path for AX.

Current supported Windows path:

- `Windows x86_64`
- Rust stable GNU toolchain
- source build from this repository

This is still a source-first prototype workflow.

## 1. Install The Rust GNU Toolchain

AX uses [`../scripts/cargo-gnu.ps1`](../scripts/cargo-gnu.ps1) because some Windows environments do not have `link.exe` configured for the MSVC path.

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal -c rustfmt
```

## 2. Clone The Repository

```powershell
git clone https://github.com/AX-FDN/AX.git
cd AX
```

If your current shell blocks local `.ps1` execution, unlock it for this shell only:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
```

## 3. Build `axc`

```powershell
.\scripts\cargo-gnu.ps1 build
```

If the build succeeds, the compiler binary should be available at:

```text
.\target\debug\axc.exe
```

## 4. Run The Supported Local Test Path

The current supported local Windows validation path uses the same GNU wrapper.
Do not treat plain `cargo test` on an unconfigured MSVC shell as the reference path.

```powershell
.\scripts\cargo-gnu.ps1 test --lib
.\scripts\cargo-gnu.ps1 test --test interface_snapshots
```

If those commands pass, the local Windows Rust/test path is healthy.

## 5. Run The Smallest Sanity Check

```powershell
.\target\debug\axc.exe check examples\hello.ax
.\target\debug\axc.exe run examples\hello.ax
.\target\debug\axc.exe check examples\slice_assignment.ax --json --ai
.\target\debug\axc.exe run examples\extract_markdown_headings.ax -- README.md target\headings-demo.txt
Get-Content target\headings-demo.txt
```

If those commands work, the local Windows compiler path is healthy.

## 6. Optional: Run The Benchmark Smoke

Windows is still the only platform that officially covers the PowerShell benchmark/orchestration layer.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-compare-repair-feedback.ps1 -SkipBuild
```

## Where To Go Next

- [`quickstart.md`](./quickstart.md)
  Quickstart index for all platform entry points.
- [`platform-support.md`](./platform-support.md)
  Current platform support tiers and boundaries.
- [`repair-benchmark.md`](./repair-benchmark.md)
  Full benchmark/export/run/score workflow.
