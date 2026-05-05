# AX Package Maturity

This document defines how AX classifies packages during `0.2 Package Preview`.
It is intentionally conservative: package maturity describes what users and
agents may assume today, not what the package name may support someday.

## Maturity Levels

| Level | Meaning | User expectation | AOT expectation |
| --- | --- | --- | --- |
| `stable_pure_ax` | Source-only AX package with deterministic behavior and no live host IO. | Safe for normal `check/run` preview workflows. | May still hit std/runtime blockers, but should not require network, secrets, accounts, or local services. |
| `host_boundary_preview` | Package wraps explicit host runtime APIs such as HTTP or TCP. | Useful in interpreter-first workflows; must document host boundary. | Should report `host_*` readiness blockers until native runtime ABI exists. |
| `future_native_preview` | Package models a future native/runtime/security domain without implementing the real native behavior yet. | Useful for shapes, metadata, validation, and planning. Not production-grade for the named domain. | Should not claim AOT/native parity for the future capability. |

The key rule:

```text
pure helpers can be stable;
host IO must be explicit;
future-native names must not pretend the runtime already exists.
```

## Current Package Classification

| Package | Maturity | Why |
| --- | --- | --- |
| `api_tools` | `stable_pure_ax` | API response/status helper shapes only. |
| `auth_tools` | `future_native_preview` | Safe header preview/redaction helpers, but no real auth protocol or crypto. |
| `bytes_tools` | `stable_pure_ax` | Byte helper wrappers over `std.bytes`; interpreter-stable, no host IO. |
| `cache_tools` | `stable_pure_ax` | Cache key/freshness policy helpers only. |
| `collection_tools` | `stable_pure_ax` | Deterministic integer collection summaries. |
| `config_rules` | `stable_pure_ax` | Pure config validation helpers. |
| `database_tools` | `future_native_preview` | DSN/readiness/schema-adjacent helpers, not a native DB driver. |
| `encoding_tools` | `stable_pure_ax` | Hex/base64 wrappers over `std.encoding`. |
| `feature_flag_tools` | `stable_pure_ax` | Deterministic rollout/decision helpers. |
| `hash_tools` | `future_native_preview` | Non-cryptographic checksum helpers; not crypto/hash security. |
| `health_tools` | `stable_pure_ax` | Service health summary helpers. |
| `http_tools` | `host_boundary_preview` | Pure helpers plus interpreter-first `std.http.get` wrapper; real HTTP native ABI is future work. |
| `json_tools` | `stable_pure_ax` | Deterministic JSON string construction helpers. |
| `jwt_tools` | `future_native_preview` | Unsigned JWT shape helpers only; no signing or verification. |
| `log_tools` | `stable_pure_ax` | Deterministic log line formatting. |
| `markdown_tools` | `stable_pure_ax` | Markdown heading inspection only. |
| `math_rules` | `stable_pure_ax` | Small deterministic scoring helpers. |
| `migration_tools` | `stable_pure_ax` | Migration naming/planning metadata helpers. |
| `net_tools` | `host_boundary_preview` | Interpreter-first TCP wrapper; native socket/TLS ABI is future work. |
| `number_tools` | `stable_pure_ax` | Integer clamps, percentages, and range checks. |
| `observability_tools` | `stable_pure_ax` | Metric/span/latency string helpers, not an exporter. |
| `pagination_tools` | `stable_pure_ax` | Page/window/offset helpers. |
| `queue_tools` | `stable_pure_ax` | Queue job state and retry/dead-letter helpers. |
| `rate_limit_tools` | `stable_pure_ax` | Quota/window math helpers. |
| `report_tools` | `stable_pure_ax` | Plain-text report builders. |
| `result_tools` | `stable_pure_ax` | Helpers around status/result summaries. |
| `retry_tools` | `stable_pure_ax` | Retry classification and delay policy helpers. |
| `schema_tools` | `stable_pure_ax` | Schema/table description helpers. |
| `text_tools` | `stable_pure_ax` | Text normalization and simple metrics. |
| `url_tools` | `stable_pure_ax` | URL classification and query construction helpers. |
| `validation_tools` | `stable_pure_ax` | Reusable validation predicates/status messages. |

## Std And AX-PKG Relationship

`std.*` owns the stable language-facing foundation. AX-PKG packages should build
on top of `std.*` instead of duplicating behavior when a standard helper exists.

Current rule of thumb:

| Capability | Owner |
| --- | --- |
| low-level language-facing primitives | `std.*` |
| deterministic reusable domain helpers | `AX-PKG` |
| examples proving package composition | `AX-PKG/examples` and AX smoke scripts |
| registry metadata, checksums, install behavior | AX compiler repository |
| host/native runtime ABI | AX compiler/runtime backend |

Examples:

- `std.json` owns deterministic JSON string construction primitives.
- `json_tools` may wrap or compose `std.json` for package-facing workflows.
- `std.http` owns status/query/header helpers and the explicit host `get` wrapper.
- `http_tools` wraps `std.http` and documents host-boundary maturity.
- `std.hash` owns non-crypto checksum primitives.
- `hash_tools` must continue to say it is non-cryptographic.

## AOT Readiness Rule

Package maturity and AOT readiness must agree:

- `stable_pure_ax` packages should not introduce `host_http`, `host_net`, or
  `host_db` blockers unless they explicitly call host APIs.
- `host_boundary_preview` packages should produce `host_*` blockers when the
  host path is used.
- `future_native_preview` packages should not pretend production native support
  exists. If they require bytes, crypto, TLS, DB, or host ABI work, blockers must
  describe that backend/runtime gap.

## Upgrade Path

Before a package can move upward in maturity:

1. It must have deterministic examples.
2. It must pass package registry smoke or a focused package smoke.
3. It must document security/runtime boundaries.
4. It must not require secrets, private services, install scripts, or native extensions.
5. If it claims native support, it needs AOT readiness/parity evidence.

Registry metadata now includes a machine-readable field:

```json
{
  "maturity": "stable_pure_ax"
}
```

This document explains the meaning of the field and remains the source of truth
for package classification changes.
