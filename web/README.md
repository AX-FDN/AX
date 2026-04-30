# AX Web

This directory contains a standalone Vite + React frontend for AX.

It presents AX as a repair workbench rather than a generic landing page:

- benchmark metrics from `docs/benchmark-showcase.md`
- the `slice_assignment_read_only` sharp demo from `docs/killer-demo.md`
- cold/base/ai feedback mode comparison
- stable interface contract cards from `docs/interface-contracts.md`
- real workload and documentation entry points

## Development

```powershell
cd web
npm install
npm run dev
```

## Build

```powershell
cd web
npm run build
```

The app is intentionally isolated under `web/` so it does not affect the Rust
compiler crate or existing `cargo` workflows.
