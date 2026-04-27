# AX Platform Support

AX currently uses platform tiers instead of pretending every workflow is equally supported everywhere.

## Current Status

| Platform | Status | Scope |
| --- | --- | --- |
| Windows | Full workflow support | `cargo-gnu` local build/test path, `axc` core commands, PowerShell benchmark/export/run/score scripts, CI smoke, and source quickstart |
| Linux | Core compiler/runtime support | `axc build/check/run/fmt`, core Rust tests, core examples, and Ubuntu CI |
| macOS | Planned later | Not yet part of the supported CI or quickstart path |

## What Linux Covers Today

Linux support in this phase means:

- `cargo build`
- `cargo test --lib`
- `cargo test --test interface_snapshots`
- `axc check`
- `axc run`
- `axc build`
- `axc fmt`

The Linux path is intentionally limited to the compiler/runtime core and the shared example/test surface.

## What Windows Local Validation Means Today

The current supported local Windows path is:

- install `stable-x86_64-pc-windows-gnu`
- if needed, run `Set-ExecutionPolicy -Scope Process Bypass` for the current shell
- run `.\scripts\cargo-gnu.ps1 build`
- run `.\scripts\cargo-gnu.ps1 test --lib`
- run `.\scripts\cargo-gnu.ps1 test --test interface_snapshots`
- then run `axc` smoke commands

This is the reference local path because some Windows shells still do not have a usable MSVC `link.exe` path.
Plain `cargo test` in those shells is not the supported baseline.

## What Remains Windows-only

The current repair benchmark orchestration layer remains Windows-only in this phase:

- `scripts/*.ps1`
- benchmark export/run/score automation
- repair comparison smoke workflows

These scripts are still part of the supported Windows workflow, but they are not part of the Linux support claim yet.

## Why macOS Is Deferred

macOS is likely to follow the same Unix-oriented path as Linux, but AX is not claiming macOS support until:

- Linux core support is stable
- Ubuntu CI is green over time
- the platform boundary is no longer drifting

Until then, macOS should be treated as future work rather than a tested target.
