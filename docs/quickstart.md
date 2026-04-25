# AX Quickstart

This page is now the quickstart index for AX platform entry points.

AX currently uses platform tiers instead of claiming full workflow parity everywhere.

## Supported Paths

- [`quickstart-windows.md`](./quickstart-windows.md)
  Full Windows source-install path, including the current PowerShell benchmark/orchestration workflow.
- [`quickstart-linux.md`](./quickstart-linux.md)
  Linux core compiler/runtime path for `axc build / check / run / fmt`.
- [`platform-support.md`](./platform-support.md)
  Current platform support tiers, boundaries, and what remains Windows-only.

## Current Boundary

Today the install story is still source-first.
The repo does **not** pretend that release packaging is already polished.

What exists now:

- a full Windows workflow
- a Linux core compiler/runtime path
- shared examples, tests, and Rust source on one mainline

What is still future productization work:

- Linux benchmark/orchestration parity
- macOS support
- smoother release binaries
- broader platform packaging

## Where To Go Next

- [`why-not-language-subsets.md`](./why-not-language-subsets.md)
  If you want the positioning argument.
- [`killer-demo.md`](./killer-demo.md)
  If you want the sharp same-case repair demo.
- [`benchmark-showcase.md`](./benchmark-showcase.md)
  If you want the current evidence summary.
- [`repair-benchmark.md`](./repair-benchmark.md)
  If you want the full benchmark/export/run/score workflow.
