# AX Web

This directory contains a standalone Vite + React frontend for AX. It is kept
inside `web/` so the public-facing site work does not mix with the Rust compiler
crate, benchmark scripts, or root planning documents.

The app now acts as an AX language portal and repair workbench:

- Home page positioning for AX as an AI-first tool/backend language.
- Docs, packages, benchmark, repair, context, download, and ecosystem sections.
- A static package catalog v0 for `std.*` modules and representative examples.
- The existing `slice_assignment_read_only` repair workbench with cold/base/ai comparison.
- Stable interface contract cards sourced from the current diagnostics/context/repair story.

The package catalog is intentionally static for now. It should become a real
registry only after AX has package dependencies, lockfiles, publish contracts,
and a stable AOT/build path.

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
