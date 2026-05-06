# Backend Profile v1 Promotion Table

This table is the promotion gate for AX 1.0's native backend profile. It sits
between the broad roadmap and the executable parity inventory:

- `backend-profile-v1.md` defines the target contract.
- `backend-profile-v1-inventory.md` lists current parity coverage and gaps.
- this file decides what can be promoted, what stays preview, and what proof is
  needed next.

Promotion status terms:

| Status | Meaning |
| --- | --- |
| `candidate` | Covered by current run-vs-exe parity, but still waiting on profile freeze or cross-platform proof. |
| `profile-blocked` | Valid language/std behavior exists, but AOT needs a specific ABI/linking/lowering proof. |
| `future` | Out of Backend Profile v1 unless explicitly pulled forward later. |

## Language And Core Runtime

| Capability | Status | Current proof | Main blocker | Next proof |
| --- | --- | --- | --- | --- |
| Entry/functions/returns | candidate | default AOT parity | Linux CI profile proof | Windows + Linux parity slice |
| `i32`, `bool`, `f32` | candidate | default AOT parity | profile freeze | include in Backend Profile v1 baseline |
| control flow and loops | candidate | default AOT parity | profile freeze | include in Backend Profile v1 baseline |
| strings | profile-blocked | parity exists for string runtime helpers | ownership/release policy | owned string release strategy or explicit process-lifetime carveout |
| bytes | profile-blocked | `std.bytes` runtime and native parity smokes | ownership/release policy | bytes release/layout follow-up tests |
| arrays and slices | profile-blocked | default AOT parity | cross-package ownership and slice lifetime | slice ABI note plus package-backed slice fixture |
| structs/enums/payload enums | candidate | default AOT parity | payload allocation/release policy | payload ownership note plus parity coverage retained |
| Result/Option and `?` | profile-blocked | default AOT parity | broader monomorphization | registry-backed generic Result/Option fixture |
| match pattern subset | candidate | default AOT parity | profile freeze | document stable pattern subset |
| full trait dispatch | future | static subset parity only | dispatch ABI not frozen | keep out of Backend Profile v1 unless dispatch ABI lands |
| full generic impl/method ABI | profile-blocked | local and registry package generic/method smokes | ABI freeze and broader monomorphization | Result/Option-style registry generic fixture |

## Standard Library

| Capability | Status | Current proof | Main blocker | Next proof |
| --- | --- | --- | --- | --- |
| pure std helpers (`text`, `json`, `hash`, `encoding`) | profile-blocked | registry and project smokes; bytes/encoding/hash runtime smokes; `json_tools` native parity | package-backed native parity breadth | expand stable registry package parity beyond JSON/generic helpers |
| `std.bytes` | profile-blocked | bytes runtime/native parity smoke; encoding smoke | ownership/release policy | bytes release/layout follow-up tests |
| `std.fs`, `std.path`, `std.env`, `std.process` | candidate | default AOT parity and runtime helpers | ABI v1 ownership notes | keep in host std candidate list |
| `std.net` | profile-blocked | interpreter host-network smoke and `AOT0301` | runtime-owned socket handle ABI implementation | native handle creation/release/error smoke |
| `std.http` | profile-blocked | interpreter host-network smoke and `AOT0301` | TCP/TLS handle ABI, request/response ownership | native HTTP readiness fixture after socket ABI |
| `std.tls` | profile-blocked | ABI placeholder only | TLS policy and stream handle ABI | TLS design doc plus readiness blocker |
| `std.db` PostgreSQL | profile-blocked | package/demo direction only | DB connection handle ABI and row/result model | `std.db` readiness smoke with `AOT0301`/DB-specific feature |
| `std.async` | profile-blocked | ABI placeholder only | async model not frozen | choose `async fn/await` or explicit runtime API |
| secure `std.crypto` | future | no reviewed contract | security review and implementation | keep non-crypto checksum separate |

## Packages And Linking

| Capability | Status | Current proof | Main blocker | Next proof |
| --- | --- | --- | --- | --- |
| local path packages | candidate | default parity and local package smokes | native package ABI freeze | keep in profile candidate list |
| registry metadata/install | profile-blocked | package registry smoke | 0.2 preview stability | snapshot `pkg` output and error codes |
| stable pure-AX registry native parity | profile-blocked | `json_tools` plus `generic_tools` registry native parity | package breadth and cross-package cases | expand stable registry native parity beyond JSON/generic helpers |
| host-boundary packages | profile-blocked | `AOT0104` maturity smoke | host handle ABI implementation | keep `AOT0104` until native handles exist |
| future-native packages | future | `AOT0105` maturity smoke | native ABI not designed | do not promote into Backend Profile v1 |
| native binary packages | future | non-goal | trust/install/linking model | keep out of 1.0 |

## Toolchain And Platforms

| Capability | Status | Current proof | Main blocker | Next proof |
| --- | --- | --- | --- | --- |
| Windows native executable path | candidate | local parity smokes | release baseline | keep focused parity smoke green |
| Linux native executable path | profile-blocked | Linux core support direction | CI parity proof | Ubuntu clang/lld Backend Profile smoke |
| macOS native path | future | no first-tier commitment | platform policy | keep out of 1.0 unless promoted |
| linker/toolchain diagnostics | candidate | `AOT1001`/toolchain blockers | profile freeze | keep structured toolchain blockers stable |

## Current Promotion Priorities

1. Runtime-owned socket handle creation/release/error smoke.
2. Broader registry generic Result/Option-style package fixture.
3. Backend Profile v1 stable pattern subset documentation.
4. Linux Backend Profile parity CI.
5. Bytes release/layout follow-up tests.

These priorities deliberately come before production HTTP/TLS/DB/async APIs.
