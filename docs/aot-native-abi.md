# AX Native Runtime ABI v1 Draft

This document is the ABI anchor for the 1.0 backend systems language line.
It replaces the earlier v0 notes with a clean contract for LLVM AOT, runtime
helpers, package linking, and future backend standard-library work.

The current implementation is still a preview. This document defines the target
shape that new AOT/runtime work should converge toward.

## Positioning

AX keeps `axc run` as the semantic reference. AX 1.0, however, should make
`axc build --emit exe` reliable for Backend Profile v1 on Windows and Linux.

Native ABI v1 exists to keep these systems aligned:

- MIR-to-LLVM lowering.
- Runtime helper declarations and definitions.
- Standard library host/runtime modules.
- Registry and local path package native linking.
- AOT readiness blockers and AI repair guidance.

Do not add new host, HTTP, TLS, database, async, or native package behavior by
hiding rules inside one call-lowering path. Add the ABI rule here, then connect
the code through the existing backend entry points.

## Stable Scalar Layout

| AX type | Native representation | Status |
| --- | --- | --- |
| `bool` | `i1` | Stable for current AOT subset. |
| `i32` | `i32` | Stable for current AOT subset. |
| `f32` | `float` | Stable for current AOT subset. |
| `string` | pointer to UTF-8 bytes with NUL terminator | Preview; ownership rules below. |
| `bytes` | runtime-owned opaque byte-buffer handle | Native lowering target for ABI v1; the LLVM runtime helpers are in place and the remaining work is linker/toolchain parity. |
| fixed array `[T; N]` | LLVM array value `[N x T]` | Stable inside current AOT subset. |
| slice `[T]` | `{ ptr, i32 len }` | Preview; cross-package ownership not frozen. |
| `string_list` | runtime-owned list handle | Preview; must stay runtime-owned. |
| struct | LLVM struct layout | Stable for non-generic current subset. |
| unit enum | `i32` tag | Stable for current subset. |
| payload enum | `{ i32 tag, ptr payload }` | Preview; payload allocation needs release policy. |
| `Option<T>` / `Result<T,E>` | concrete enum instance layout | Preview; cross-package monomorphization remains open. |

## Ownership And Allocation

Current AOT code uses process-lifetime allocation in several runtime helpers.
That remains acceptable for v0 parity examples, but it is not enough for a
backend systems language.

ABI v1 should introduce one explicit memory policy:

- Runtime-created strings, byte buffers, lists, payload boxes, and host handles
  are runtime-owned values.
- Borrowed values may be read by a helper but must not be released by that
  helper.
- Process-lifetime values are allowed only for literals, compiler-owned
  constants, compatibility helpers, and current v0 parity fixtures.
- Runtime-owned values must have either a matching release helper or a documented
  arena/process-lifetime rule.
- Backend standard-library APIs must state whether the caller owns, borrows, or
  receives a runtime-owned value.
- AOT lowering must not invent allocation/free rules locally.

Default v1 direction:

```text
runtime-owned object
  -> opaque handle or pointer
  -> explicit runtime release helper when lifetime can end
  -> process-lifetime only for literals, compiler-owned constants, or documented
     compatibility helpers
```

GC is not required for 1.0. Reference counting is not required for 1.0. A clear
release/runtime-owned rule is required.

### Ownership Terms

| Term | Meaning | Current examples |
| --- | --- | --- |
| runtime-owned | The runtime allocated the value and owns the release policy. | `bytes`, `string_list`, future socket/TLS/DB/task handles. |
| borrowed | The helper can read the value but cannot release it. | string literals, function arguments, slice views. |
| process-lifetime | The value intentionally lives until process exit. | current string/bytes/list v0 compatibility helpers and constants. |

### Release Helpers

ABI v1 reserves release helper names even while the current v0 executable subset
continues to use process-lifetime allocation:

| Runtime value | Release helper | Current behavior |
| --- | --- | --- |
| owned string | `ax_string_release_owned` | reserved; string helpers still use process-lifetime allocation. |
| owned bytes | `ax_bytes_release` | emitted as a private no-op helper until the allocator policy is enabled. |
| owned string list | `ax_string_list_release` | emitted as a private no-op helper until nested value ownership is frozen. |

The no-op release helpers are intentional ABI anchors. They let lowering,
runtime helpers, package linking, and future host handles agree on names before
automatic lifetime insertion is implemented. A future allocator pass may replace
the no-op bodies with real `free` or runtime-owned release logic without changing
the ABI names.

## Host Result And Error ABI

Host/runtime failures must not appear as parser or semantic errors.

Runtime helpers should report failures through one of these routes:

- AX `Result<T, string>` for recoverable standard-library APIs.
- Structured runtime diagnostics for unrecoverable interpreter/AOT runtime
  failures.
- AOT readiness blockers when the backend does not support the required ABI.

Native host errors should preserve:

- stable error code or category
- human-readable message
- source operation, such as `http.get`, `tls.connect`, `db.query`, or
  `async.timeout`
- AI action: edit source, explain backend limit, install/configure toolchain,
  retry validation, or report compiler bug

## Resource Handles

Backend systems work needs handles. ABI v1 reserves these handle families:

| Handle | Intended owner | Release rule |
| --- | --- | --- |
| file handle | runtime | explicit close helper |
| tcp socket | runtime | explicit close helper |
| tls stream | runtime | explicit close helper |
| http client/server | runtime | explicit close/shutdown helper |
| db connection | runtime | explicit close helper |
| async task | runtime/event loop | join/cancel/drop helper |
| timer | runtime/event loop | cancel/drop helper |

Do not expose raw OS handles as stable AX values in 1.0. Keep them behind
runtime-owned handles so Windows and Linux can share the same language contract.

## Backend Standard Library ABI Targets

The 1.0 backend standard library should use the ABI in this order:

1. `std.bytes`: byte buffers and binary-safe conversion.
2. `std.net`: TCP sockets, timeouts, and error mapping.
3. `std.tls`: TLS stream over sockets with certificate policy.
4. `std.http`: request/response over TCP/TLS.
5. `std.db`: PostgreSQL client v1 over TCP/TLS.
6. `std.async`: task and event-loop integration for network/database IO.

`std.hash` and checksum helpers are not cryptography. Secure crypto must be a
separate `std.crypto` contract before password hashing, JWT signing, HMAC, or
TLS internals can be claimed.

Current `std.bytes` status:

- `axc run` supports `bytes_empty`, `bytes_from_string`, `bytes_push`,
  `bytes_get`, `bytes_to_hex`, `bytes_to_string_lossy`, and `len(bytes)`.
- `axc build` reports `bytes_runtime` while the byte-buffer ABI is in the AOT
  runtime path; current work focuses on executable linking and parity.
- `scripts/smoke-bytes-runtime.ps1` is the fixture that locks the runtime and
  build boundary for this contract.

Current host/network status:

- `axc run` supports host-boundary preview helpers for `std.http` and `std.net`
  over local TCP/HTTP test fixtures.
- `axc build` must report `host_http`, `host_net`, and `AOT0301` until
  runtime-owned host handles and native networking ABI rules are implemented.
- `AOT0301` belongs to the `runtime_abi` AI layer and is not a safe source-edit
  request.
- `scripts/smoke-host-network-runtime.ps1` is the fixture that locks this
  contract.

## Backend Code Boundaries

Keep ABI implementation concentrated in these backend areas:

- `src/backend/llvm/abi.rs`: type and helper-name constants.
- `src/backend/llvm/symbols.rs`: user, package, method, generic, and runtime
  symbol generation.
- `src/backend/llvm/runtime/*`: runtime helper declarations and definitions.
- `src/backend/llvm/linking.rs`: native link plan and future package/std object
  linking.
- `src/backend/llvm/monomorph.rs`: concrete instance planning.

Call lowering may use these APIs, but it should not become the source of truth
for ABI policy.

## AOT Readiness Contract

When ABI support is missing, `axc build` should report a blocker rather than a
source error.

Required blocker categories:

- `runtime_abi`
- `aot_readiness`
- `monomorphization`
- `llvm_lowering`
- `toolchain_link`
- `package_registry`
- `package_cache`
- `internal_compiler_error`

The AI-facing rule is simple:

```text
valid check/run behavior + missing native ABI
  -> explain backend/runtime gap
  -> do not rewrite user business logic
```

## 1.0 Exit Criteria

Native ABI v1 is ready for the 1.0 backend profile when:

- strings, bytes, slices, results, and runtime-owned handles have documented
  layout and ownership.
- Windows and Linux use the same language-level ABI contract.
- runtime helpers are declared through the backend ABI entry points.
- package and std native linking use the same symbol/mangling rules.
- AOT readiness can explain every unsupported ABI boundary with a structured
  blocker.
