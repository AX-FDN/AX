# AX Interface Contracts

> 本文件说明 AX 当前哪些输出属于高价值外部契约，哪些测试或快照在保护这些契约。

AX 现在的外部契约不是只有 CLI 命令本身。对 agent 和工具链来说，更重要的是这些稳定 JSON / artifact 形态：

- diagnostics JSON
- AI-enhanced diagnostics JSON
- runtime diagnostics JSON
- context JSON
- build manifest JSON
- repair benchmark bundle / prompt / index

## Contract Map

| Contract | Producer | Consumer | Snapshot / Regression |
| --- | --- | --- | --- |
| check diagnostics | `axc check --json` | CLI users, repair export, adapters | `diagnostics_json_matches_snapshot` |
| AI diagnostics | `axc check --json --ai` | repair benchmark, adapters, AI agents | `diagnostics_json_with_ai_matches_snapshot` |
| AI session escalation | `axc check --json --ai --ai-session` | repeated agent repair loops | `diagnostics_ai_session_escalation_matches_snapshots` |
| runtime diagnostics | `axc run --json` | runtime repair benchmark | `run_runtime_error_json_matches_snapshot`, `run_runtime_division_by_zero_json_matches_snapshot` |
| runtime AI diagnostics | `axc run --json --ai` | runtime repair benchmark, adapters | `run_runtime_error_json_with_ai_matches_snapshot`, `run_runtime_division_by_zero_json_with_ai_matches_snapshot` |
| AST / HIR / MIR dumps | `axc ast/hir/mir --json` | compiler debugging, external tools | `ast_dump_matches_snapshot`, `hir_dump_matches_snapshot`, `mir_dump_matches_snapshot` |
| build skeleton manifest | `axc build` | future backend, build tooling | `build_manifest_matches_snapshot`, `project_build_manifest_matches_snapshot` |
| context overview | `axc context overview --json` | agents, docs, repair context | `context_overview_matches_snapshot` |
| context boundaries | `axc context boundaries --json` | host-boundary-aware agents | `context_boundaries_matches_snapshot` |
| context topology | `axc context topology --json` | project navigation agents | `context_topology_matches_snapshot` |
| context flow | `axc context flow --json` | workflow-aware agents | `context_flow_matches_snapshot` |
| context symbol | `axc context symbol --json` | local symbol repair planning | `context_symbol_matches_snapshot` |
| context impact | `axc context impact --json` | change-risk planning | `context_impact_matches_snapshot` |
| context evidence | `axc context evidence --json` | validation planning | `context_evidence_matches_snapshot` |
| repair export artifacts | `export-repair-benchmark.ps1` | repair adapters, score/compare scripts | `repair_benchmark_export_keeps_cold_base_ai_artifact_contracts` |
| project-backed repair export | `export-repair-benchmark.ps1` | multi-file repair adapters | `repair_benchmark_export_supports_project_context_cases` |
| context-enabled repair export | `export-repair-benchmark.ps1 -IncludeContext` | context-consuming repair adapters | `repair_benchmark_export_can_include_context_bundle` |
| Std-1 candidate source tree | `axc build examples/project_*` | future AOT/package/std consumers | `project_text_normalize_build_copies_real_example_source_tree`, `project_directory_index_build_copies_real_example_source_tree`, `project_release_promote_build_copies_real_example_source_tree`, `project_command_capture_build_copies_real_example_source_tree`, `project_command_batch_build_copies_real_example_source_tree` |
| Std-1 candidate runtime behavior | `axc run examples/project_*` | stdlib users, host-boundary examples | `project_text_normalize_runs_on_controlled_fixture`, `project_directory_index_runs_on_controlled_fixture`, `project_release_promote_runs_on_controlled_fixture`, `project_command_capture_runs_on_controlled_fixture`, `project_command_batch_runs_on_controlled_fixture` |

## Stability Rules

### Diagnostics

Stable:

- diagnostic array shape
- `code`
- `message`
- `file`
- `span.start`
- `span.end`
- `notes`
- `expected`
- `suggestion`
- optional `ai` object when `--json --ai` is used

Allowed to evolve carefully:

- richer `ai.rule_card`
- additional `context_snippets`
- new `rule_id` values for new diagnostics
- additional optional AI fields, if omitted cleanly when unavailable

Not allowed without explicit contract update:

- changing base field meaning
- changing `--json` success output away from `[]`
- requiring consumers to parse text diagnostics

### Context

Stable:

- `schema_version`
- `view`
- `subject`
- `facts`
- `hints`
- `validation`
- current view names: `overview / boundaries / topology / flow / symbol / impact / evidence`

Allowed to evolve carefully:

- adding fields under `facts`
- adding fields under `hints`
- adding recommended commands under `validation`

Not allowed without explicit contract update:

- renaming views
- moving `facts / hints / validation` to a different top-level shape
- making `symbol / impact / evidence` stop accepting explicit symbol queries

### Build Manifest

Stable:

- build emits a machine-readable manifest
- source/HIR/MIR/build metadata remain available as skeleton artifacts
- backend status is explicit while native executable output is not yet ready

Allowed to evolve carefully:

- adding backend metadata
- adding future AOT artifact fields
- adding platform-specific output metadata

Not allowed without explicit contract update:

- presenting skeleton output as a finished native executable contract
- removing manifest fields without a migration path

### Repair Export

Stable:

- `bundle.cold.json`
- `bundle.base.json`
- `bundle.ai.json`
- `prompt.cold.md`
- `prompt.base.md`
- `prompt.ai.md`
- `index.json`
- cold/base/ai feedback mode split
- project-backed read-only context fields

Stable optional extension:

- `context_bundle` appears only when `-IncludeContext` is enabled
- first context shell is `overview + boundaries + evidence`
- exports without `-IncludeContext` preserve the previous artifact contract

Not allowed without explicit contract update:

- making context mandatory for old adapters
- removing `BundlePath` or `PromptPath` from adapter workflows
- changing the full-source repair output contract into a patch-only contract

### Std-1 Candidate Interfaces

Stable for the current P3 freeze candidate:

- project-backed examples can import `std.*` through `AX.toml sources = ["../../std", ...]`
- build source snapshots include `external/std/cli.ax`
- build source snapshots include `external/std/env.ax`
- build source snapshots include `external/std/fs.ax`
- build source snapshots include `external/std/path.ax`
- build source snapshots include `external/std/process.ax`
- build source snapshots include `external/std/report.ax`
- build source snapshots include `external/std/text.ax`
- build source snapshots include `external/std/workspace.ax`

Allowed to evolve carefully:

- adding a new `std/` module after it has a real workload and interface snapshot coverage
- moving a helper from `foundation/` to `std/` after it satisfies docs、examples、regression and diagnostics/runtime boundary requirements

Not allowed without explicit contract update:

- removing a `std/` source from project build snapshots
- exposing Rust crate names as AX user-facing imports
- declaring `std.collections` frozen before a real `std/collections.ax` module and workload coverage exist

## Current Verification Commands

For broad contract coverage:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots
```

For diagnostics and core compiler changes:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --lib
```

For context-enabled repair export changes:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots repair_benchmark_export_can_include_context_bundle
```

For the full repair replay evidence path, see [`benchmark-showcase.md`](./benchmark-showcase.md).

## Update Rule

When adding or changing a public JSON/artifact field:

1. Update the producing code.
2. Update or add an interface snapshot/regression.
3. Update the corresponding docs.
4. Explain whether the field is stable, optional, or experimental.
5. Keep old consumers working unless a deliberate contract break is documented.
