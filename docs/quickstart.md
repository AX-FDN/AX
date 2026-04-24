# AX Quickstart

This is the fastest honest path to get AX running locally today.

Current tested install path:

- `Windows x86_64`
- Rust stable GNU toolchain
- source build from this repository

This is a source-first prototype.
Treat this page as the current install entry, not as a polished release installer.

## 1. Install The Rust GNU Toolchain

AX uses [`../scripts/cargo-gnu.ps1`](../scripts/cargo-gnu.ps1) because some Windows environments do not have `link.exe` configured for the MSVC path.

Install the tested Rust toolchain:

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal -c rustfmt
```

## 2. Clone The Repository

```powershell
git clone https://github.com/AX-FDN/AX.git
cd AX
```

## 3. Build `axc`

```powershell
.\scripts\cargo-gnu.ps1 build
```

If the build succeeds, the compiler binary should be available at:

```text
.\target\debug\axc.exe
```

## 4. Run The Smallest Sanity Check

Check a valid example:

```powershell
.\target\debug\axc.exe check examples\hello.ax
```

Run a program:

```powershell
.\target\debug\axc.exe run examples\hello.ax
```

Inspect AI-enhanced diagnostics:

```powershell
.\target\debug\axc.exe check examples\slice_assignment.ax --json --ai
```

Run one small tool-style script:

```powershell
.\target\debug\axc.exe run examples\extract_markdown_headings.ax -- README.md target\headings-demo.txt
Get-Content target\headings-demo.txt
```

If all four commands work, your local prototype path is healthy.

## 5. Optional: Run The Benchmark Smoke

If you want to verify the repair-evidence path instead of only the compiler binary:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-compare-repair-feedback.ps1 -SkipBuild
```

That checks the stable `base` versus `ai` comparison contract on the committed smoke benchmark assets.

## Where To Go Next

- [`why-not-language-subsets.md`](./why-not-language-subsets.md)
  If you want the positioning argument.
- [`killer-demo.md`](./killer-demo.md)
  If you want the sharp same-case repair demo.
- [`benchmark-showcase.md`](./benchmark-showcase.md)
  If you want the current evidence summary.
- [`repair-benchmark.md`](./repair-benchmark.md)
  If you want the full benchmark/export/run/score workflow.

## Current Boundary

Today the install story is still source-first.
The repo does **not** pretend that release packaging is already polished.

What exists now:

- a working source build path
- a working compiler/interpreter prototype
- stable benchmark scripts and smoke checks

What is still future productization work:

- smoother release binaries
- faster first-run onboarding
- broader platform packaging
