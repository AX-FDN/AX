# AX Quickstart: Linux

This is the current Linux core path for AX.

Current supported Linux scope:

- source build from this repository
- `axc build / check / run / fmt`
- core Rust tests and core examples

This is not yet the full benchmark/orchestration workflow.

## 1. Install Rust Stable

```bash
curl https://sh.rustup.rs -sSf | sh
rustup toolchain install stable --profile minimal -c rustfmt
```

## 2. Clone The Repository

```bash
git clone https://github.com/AX-FDN/AX.git
cd AX
```

## 3. Build `axc`

```bash
cargo build
```

If the build succeeds, the compiler binary should be available at:

```text
./target/debug/axc
```

## 4. Run Core Sanity Checks

```bash
./target/debug/axc check examples/hello.ax
./target/debug/axc run examples/hello.ax
./target/debug/axc check examples/slice_assignment.ax --json --ai
./target/debug/axc build examples/project_hello --out-dir target/hello-build
```

If those commands work, the Linux core compiler/runtime path is healthy.

## Current Boundary

Linux support in this phase does **not** include:

- `scripts/*.ps1`
- repair benchmark export/run/score orchestration
- PowerShell smoke comparison workflows

Those remain Windows-only for now.

## Where To Go Next

- [`quickstart.md`](./quickstart.md)
  Quickstart index for all platform entry points.
- [`platform-support.md`](./platform-support.md)
  Current platform support tiers and boundaries.
- [`why-not-language-subsets.md`](./why-not-language-subsets.md)
  Positioning for the AX source protocol direction.
