# Backend Profile v1 Executable Inventory

This inventory turns the Backend Profile v1 roadmap into an executable checklist.
The source of truth for the current native parity set is
`scripts/smoke-aot-parity.ps1`.

Current fixed baseline:

- default AOT parity cases: `123`
- project parity cases: `26`
- all repository `AX.toml` project examples are listed in default parity
- build manifest schema: `10`
- AOT readiness schema: `3`

## Already Covered By Default AOT Parity

These areas already have run-vs-native-exe parity coverage in the default smoke.
They are Backend Profile v1 candidates, not final 1.0 guarantees yet.

| Area | Representative cases | Current status |
| --- | --- | --- |
| Entry and basic functions | `aot_return.ax`, `aot_nested_calls.ax`, `factorial.ax` | Candidate |
| Integer arithmetic and guards | `aot_math.ax`, `modulo.ax`, `division_by_zero` runtime-error smoke | Candidate |
| Bool/control flow | `aot_bool_logic.ax`, `logical_ops.ax`, `aot_control_flow.ax`, `aot_loop.ax` | Candidate |
| Loops | `for_loop.ax`, `for_in.ax`, `break_loop.ax`, `continue.ax` | Candidate |
| `f32` core | `aot_f32_core.ax` | Candidate |
| Strings | `aot_string_values.ax`, `aot_string_runtime.ax`, `aot_string_predicates.ax`, `aot_string_replace.ax`, `aot_string_trim.ax` | Candidate, pending ABI v1 ownership rules |
| String slices/lists | `aot_string_split_lines.ax`, `aot_string_split_lines_for_in.ax`, `string_list.ax` | Candidate, pending list/slice ABI freeze |
| Arrays | `aot_array_read.ax`, `aot_array_write.ax`, `aot_array_to_string.ax`, `aot_array_equality.ax`, `empty_array.ax` | Candidate |
| Slices | `aot_slice_range.ax`, `aot_slice_for_in.ax`, `aot_slice_to_string.ax`, `aot_slice_equality.ax`, `slice_assignment.ax` | Candidate, pending cross-package ownership rules |
| Structs | `aot_struct_read.ax`, `aot_struct_write.ax`, `aot_struct_to_string.ax`, `aot_struct_equality.ax` | Candidate |
| Enums | `aot_enum_unit.ax`, `aot_enum_match.ax`, `aot_payload_enum.ax`, `aot_payload_enum_equality.ax` | Candidate |
| Enum formatting/payloads | `aot_enum_to_string.ax`, `aot_enum_print.ax`, `aot_enum_array_payload.ax`, `aot_enum_struct_slice_payload.ax` | Candidate, pending deeper payload rules |
| Match variants | `aot_match_expression.ax`, `match_range.ax`, `match_or.ax`, `match_guard.ax`, `match_struct_pattern.ax` | Candidate, richer destructuring still preview |
| Result/Option | `aot_result_option.ax`, `result_static_constructors.ax`, `result_propagation.ax`, `aot_result_try.ax` | Candidate, pending broader monomorphization |
| Generics/methods/traits slice | `generic_functions.ax`, `generic_impl.ax`, `generic_method.ax`, `trait_impl.ax`, `trait_bounds.ax` | Preview candidate, not frozen ABI |
| Project mode | `project_hello`, `project_split`, `project_module_smoke` | Candidate |
| Std tool modules | `project_text_normalize`, `project_config_validate`, `project_result_pipeline`, `project_job_runner` | Candidate for pure/host std subset |
| Host fs/path/env/process v0 | `aot_fs_read.ax`, `aot_fs_write.ax`, `aot_path_runtime.ax`, `aot_process_runtime.ax`, `project_process_result` | Candidate, pending Runtime ABI v1 |
| Local path packages | `project_package_math`, `project_package_config`, `project_job_runner` | Candidate, native package ABI not frozen |

## Package-Backed AOT Readiness

`scripts/smoke-package-registry-aot.ps1` is the focused package maturity smoke.

It verifies:

- `stable_pure_ax` registry packages do not produce package maturity blockers.
- `host_boundary_preview` registry packages produce `AOT0104`.
- `future_native_preview` registry packages produce `AOT0105`.
- build manifests preserve `registry_packages`.

This smoke is an IR/readiness check. It does not require clang or native
executable linking.

`scripts/smoke-package-registry-native-parity.ps1` is the first package-backed
native parity smoke. It installs `json_tools` from the curated registry, runs
`axc check`, compares `axc run` with the linked native executable, and requires
the registry package manifest entry to stay `stable_pure_ax`.

`scripts/smoke-aot-package-generics.ps1` verifies local path packages that
export generic structs/functions and non-generic methods/impls. It requires
`AX.lock`, checks the package graph readiness contract, and compares interpreter
output with the native executable.

## Runtime ABI Readiness

`scripts/smoke-bytes-runtime.ps1` is the current `std.bytes` ABI fixture. It
proves interpreter behavior for byte buffers and requires `axc build` to report
`bytes_runtime` with `AOT0303` at the `runtime_abi` AI layer. This keeps valid
byte-buffer source from being misreported as a user-code error while native
bytes lowering is still pending.

## Backend Profile v1 Gaps

These gaps block promotion from "candidate" to "Backend Profile v1 stable".

| Gap | Why it matters | Required next proof |
| --- | --- | --- |
| Native Runtime ABI v1 ownership | Backend services need predictable string/bytes/handle lifetime. | ABI doc plus runtime helper tests for release/ownership behavior. |
| `bytes` native ABI | HTTP/TLS/DB need binary-safe data. | Readiness fixture exists with `AOT0303/runtime_abi`; next proof is native byte-buffer layout and parity. |
| Registry package native parity | 1.0 packages must participate in native build, not only readiness. | First `stable_pure_ax` package fixture exists; expand to more packages and cross-package cases. |
| Cross-package monomorphization | Generic package APIs need concrete native instances. | Local path generic package smoke exists; add registry-backed generic package parity next. |
| Method/impl ABI freeze | Backend code will use methods heavily. | Local package method parity exists; freeze symbol/ABI rules and add registry-backed coverage next. |
| Trait ABI boundary | Traits exist but full dispatch is not frozen. | static dispatch subset documented; dynamic dispatch remains out of profile unless implemented. |
| HTTP/TLS runtime ABI | Backend language needs real network IO. | `std.http` client/server and `std.tls` readiness/ABI fixtures. |
| PostgreSQL runtime ABI | DB is a 1.0 backend target. | `std.db` PostgreSQL smoke with interpreter and AOT readiness first, parity later. |
| Async runtime model | Backend services need structured IO. | either `async fn/await` or explicit `std.async` API frozen with readiness blockers. |
| Linux executable parity | 1.0 target includes Linux server workflow. | Ubuntu CI runs Backend Profile v1 parity with clang installed. |

## Promotion Rule

A capability can move into Backend Profile v1 only when it has all four:

1. language or std contract documented
2. interpreter behavior covered
3. `axc build` manifest/readiness behavior covered
4. native parity or explicit blocker covered

If any item is missing, the capability stays preview.

## Immediate Test Additions

Next executable checks to add:

1. registry-backed generic/method helper fixture.
2. native byte-buffer layout and parity fixture.
3. host handle readiness fixture for future HTTP/TLS/DB/async handles.

These should be added before implementing production HTTP/TLS/DB/async APIs.
