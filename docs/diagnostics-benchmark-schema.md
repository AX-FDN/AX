# AX Diagnostics Benchmark Schema

## Scope

This document covers the stable report shape emitted by:

- [`../scripts/benchmark-diagnostics.ps1`](../scripts/benchmark-diagnostics.ps1)
- [`../scripts/smoke-benchmark-diagnostics.ps1`](../scripts/smoke-benchmark-diagnostics.ps1)

The script measures the relative cost of:

- `axc check <file>`
- `axc check <file> --json`
- `axc check <file> --json --ai`

over a stable manifest of broken AX programs.

It does not define:

- exact timing numbers
- absolute performance targets
- CI machine-specific thresholds

## Output Files

By default the script writes to:

```text
.ax-ai\diagnostics-benchmark\<timestamp>\
```

Stable output files:

- `summary.json`
  Machine-readable benchmark report.
- `summary.md`
  Human-readable markdown summary built from the same report.

## Top-Level Contract

Current `summary.json` shape:

```json
{
  "schema_version": 1,
  "generated_at": "2026-04-23T12:34:56.0000000+08:00",
  "manifest_path": "C:\\repo\\benchmarks\\repair-cases.json",
  "output_dir": "C:\\repo\\.ax-ai\\diagnostics-benchmark\\20260423-123456",
  "iterations": 10,
  "total_cases": 5,
  "target_dir": "D:/CargoTarget/AX",
  "binary_path": "D:/CargoTarget/AX/debug/axc.exe",
  "mode_order": ["text", "json", "json_ai"],
  "per_case_timings": [],
  "mode_summary": [],
  "pairwise_overhead": []
}
```

Stable top-level fields:

- `schema_version: integer`
  Current report schema version. The current scripts write `1`.
- `generated_at: string`
  ISO-8601 timestamp for when the report was written.
- `manifest_path: string`
  Resolved benchmark manifest path.
- `output_dir: string`
  Resolved benchmark output directory.
- `iterations: integer`
  Number of repeated invocations per file/mode pair.
- `total_cases: integer`
  Number of benchmark files included in this run.
- `target_dir: string`
  Resolved Cargo target directory used to locate `axc.exe`.
- `binary_path: string`
  Resolved AX binary path used for measurement.
- `mode_order: string[]`
  Stable mode ordering. Current value is `["text", "json", "json_ai"]`.
- `per_case_timings: object[]`
  Per-file timings per mode.
- `mode_summary: object[]`
  Aggregated timing summary per mode.
- `pairwise_overhead: object[]`
  Relative overhead comparisons between the stable modes.

## `per_case_timings`

Each row contains:

```json
{
  "file": "examples/missing_semicolon.ax",
  "mode": "json_ai",
  "iterations": 10,
  "total_ms": 42.31,
  "avg_ms": 4.23
}
```

Stable fields:

- `file: string`
  Repository-relative AX input path from the manifest.
- `mode: "text" | "json" | "json_ai"`
- `iterations: integer`
- `total_ms: number`
- `avg_ms: number`

The report currently emits one row for every `file x mode` combination.

## `mode_summary`

Each row contains:

```json
{
  "mode": "json_ai",
  "files": 5,
  "avg_ms": 4.23,
  "min_ms": 2.11,
  "max_ms": 7.88,
  "total_ms": 21.15
}
```

Stable fields:

- `mode: "text" | "json" | "json_ai"`
- `files: integer`
  Number of benchmark files measured for the mode.
- `avg_ms: number`
  Mean of the per-file `avg_ms` values for that mode.
- `min_ms: number`
  Minimum per-file `avg_ms` value for that mode.
- `max_ms: number`
  Maximum per-file `avg_ms` value for that mode.
- `total_ms: number`
  Sum of the per-file `total_ms` values for that mode.

## `pairwise_overhead`

Each row contains:

```json
{
  "from_mode": "json",
  "to_mode": "json_ai",
  "avg_from_ms": 3.02,
  "avg_to_ms": 4.23,
  "avg_overhead_ms": 1.21,
  "relative_overhead_pct": 40.07
}
```

Stable rows:

- `text -> json`
- `json -> json_ai`
- `text -> json_ai`

Stable fields:

- `from_mode: string`
- `to_mode: string`
- `avg_from_ms: number`
- `avg_to_ms: number`
- `avg_overhead_ms: number`
  Mean extra cost of the `to_mode` relative to the `from_mode`.
- `relative_overhead_pct: number | null`
  Relative overhead percentage. May be `null` if the base mode average is zero.

## Compatibility Rules

The current repository treats these as stable behavior:

- `summary.json` exists
- `summary.md` exists
- `schema_version` exists and currently equals `1`
- `mode_order` is stable and ordered
- one `per_case_timings` row exists per `file x mode`
- one `mode_summary` row exists per mode
- `pairwise_overhead` includes the three stable comparisons

The following are intentionally unstable:

- exact timing values
- exact markdown wording in `summary.md`
- absolute machine performance across CI runners or local machines
