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
- [`validation-matrix.md`](./validation-matrix.md)
  Current local/CI validation matrix for Windows local, Windows CI, and Ubuntu CI.
- [`../web/`](../web/)
  Optional Repair Workbench frontend for viewing the AX repair/demo story in a browser.

## Current Boundary

Today the install story is still source-first.
The repo does **not** pretend that release packaging is already polished.

What exists now:

- a full Windows workflow
- a Linux core compiler/runtime path
- shared examples, tests, and Rust source on one mainline
- an isolated Vite + React Repair Workbench frontend under `web/`

Current Windows local validation contract:

- install the Rust GNU toolchain
- if the shell blocks local scripts, run `Set-ExecutionPolicy -Scope Process Bypass`
- use `.\scripts\cargo-gnu.ps1 build`
- use `.\scripts\cargo-gnu.ps1 test --lib` and `.\scripts\cargo-gnu.ps1 test --test interface_snapshots`
- then run `axc` smoke commands from `docs/quickstart-windows.md`

What is still future productization work:

- Linux benchmark/orchestration parity
- macOS support
- smoother release binaries
- broader platform packaging

## Optional Web Workbench

The `web/` directory is a standalone frontend for presenting AX as a repair workbench. It is not required to build or run `axc`.

```powershell
cd web
npm ci
npm run dev
```

For production build verification:

```powershell
cd web
npm run build
```

GitHub Actions runs a dedicated `web` job with Node.js 22, `npm ci`, and `npm run build`.

## Where To Go Next

- [`why-not-language-subsets.md`](./why-not-language-subsets.md)
  If you want the positioning argument.
- [`killer-demo.md`](./killer-demo.md)
  If you want the sharp same-case repair demo.
- [`benchmark-showcase.md`](./benchmark-showcase.md)
  If you want the current evidence summary.
- [`../web/README.md`](../web/README.md)
  If you want to run the Repair Workbench frontend locally.
- [`repair-benchmark.md`](./repair-benchmark.md)
  If you want the full benchmark/export/run/score workflow.
