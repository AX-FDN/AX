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
| AI diagnostics | `axc check --json --ai` | repair benchmark, adapters, AI agents | `diagnostics_json_with_ai_matches_snapshot`, `diagnostics_result_propagation_json_with_ai_matches_snapshot` |
| package resolver diagnostics | `axc check <project> --json --ai` | package-aware repair benchmark, adapters, AI agents | `project_path_package_manifest_errors_have_json_ai_diagnostics` |
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
| context evidence | `axc context evidence --json` | validation planning, build/AOT readiness planning | `context_evidence_matches_snapshot` |
| repair export artifacts | `export-repair-benchmark.ps1` | repair adapters, score/compare scripts | `repair_benchmark_export_keeps_cold_base_ai_artifact_contracts` |
| project-backed repair export | `export-repair-benchmark.ps1` | multi-file repair adapters | `repair_benchmark_export_supports_project_context_cases` |
| context-enabled repair export | `export-repair-benchmark.ps1 -IncludeContext` | context-consuming repair adapters | `repair_benchmark_export_can_include_context_bundle` |
| repair archaeology artifact | `export-repair-archaeology.ps1` | docs, demos, benchmark readers | `smoke-repair-archaeology.ps1` exports `index.json` and `cases/<case-id>.json/.md` from fresh deterministic replay artifacts |
| Std-1 candidate source tree | `axc build examples/project_*` | future AOT/package/std consumers | `project_text_normalize_build_copies_real_example_source_tree`, `project_directory_index_build_copies_real_example_source_tree`, `project_release_promote_build_copies_real_example_source_tree`, `project_command_capture_build_copies_real_example_source_tree`, `project_command_batch_build_copies_real_example_source_tree`, `project_option_result_build_copies_real_example_source_tree`, `project_env_result_build_copies_real_example_source_tree`, `project_file_result_build_copies_real_example_source_tree`, `project_process_result_build_copies_real_example_source_tree`, `project_result_pipeline_build_copies_real_example_source_tree`, `project_config_validate_build_copies_real_example_source_tree`, `project_collections_report_build_copies_real_example_source_tree`, `project_job_runner_build_copies_real_example_source_tree` |
| Std-1 candidate runtime behavior | `axc run examples/project_*` | stdlib users, host-boundary examples | `project_text_normalize_runs_on_controlled_fixture`, `project_directory_index_runs_on_controlled_fixture`, `project_release_promote_runs_on_controlled_fixture`, `project_command_capture_runs_on_controlled_fixture`, `project_command_batch_runs_on_controlled_fixture`, `project_option_result_runs`, `project_env_result_runs`, `project_file_result_runs_on_controlled_fixture`, `project_process_result_runs_on_controlled_fixture`, `project_result_pipeline_runs_on_controlled_fixture`, `project_config_validate_runs_on_controlled_fixture`, `project_collections_report_runs_on_controlled_fixture`, `project_job_runner_runs_on_controlled_fixture` |
| local path package v0 | `AX.toml [dependencies] alias = { path = ... }` | project organization, future package/AOT consumers | `project_package_config_runs_on_controlled_fixture`, `project_package_config_build_copies_real_example_source_tree`, `project_job_runner_runs_on_controlled_fixture`, `project_job_runner_build_copies_real_example_source_tree`, `project_job_runner_lock_and_context_expose_package_graph`, `project::tests::resolves_local_path_dependency_sources_under_dependency_alias` |
| `AX.lock` v0 | `axc lock <project> [--check]` | reproducible local package planning, future package/AOT consumers | `project_lock_generates_and_checks_local_path_packages`, `project_lock_check_reports_stale_package_graph_details`, `project_job_runner_lock_and_context_expose_package_graph` |

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
- adding read-only readiness facts, such as `facts.build_readiness`, when they describe current compiler contracts instead of creating new behavior

Not allowed without explicit contract update:

- renaming views
- moving `facts / hints / validation` to a different top-level shape
- making `symbol / impact / evidence` stop accepting explicit symbol queries

### Build Manifest

Stable:

- build emits a machine-readable manifest
- source/HIR/MIR/build metadata remain available as skeleton artifacts
- backend status is explicit while native executable output is not yet ready
- build manifest schema version `5` exposes `aot_readiness` so AOT blockers are machine-readable before native executable emission exists
- `aot_readiness` records required backend features, blocker codes such as `AOT0001`, `AOT0101`, `AOT0201`, and `AOT0301`, and the next backend stage that must resolve each blocker

Allowed to evolve carefully:

- adding backend metadata
- adding future AOT artifact fields
- adding platform-specific output metadata
- adding new `AOT****` blocker codes when new syntax, package, runtime, or ABI surfaces become visible to the backend

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

### Repair Archaeology Artifact

Current status:

- schema documented in [`repair-archaeology-schema.md`](./repair-archaeology-schema.md)
- producer script exists at [`../scripts/export-repair-archaeology.ps1`](../scripts/export-repair-archaeology.ps1)
- not part of `axc` CLI yet

Stable for v0 design:

- case JSON artifact has `schema_version`
- case JSON separates `case / subject / initial_diagnostic / repair_contract / context / modes / comparison / archaeology_summary / reproducibility / provenance`
- Markdown report is presentation only
- deterministic replay must be marked as `source_kind = deterministic_replay`
- live-model claims must remain `false` until a separate live benchmark exists

Not allowed without explicit contract update:

- using Markdown as source of truth
- generating pass/fail from interpretation text
- treating Repair Archaeology as evidence of live-model performance
- adding an `axc repair-log` command before script/artifact schema is stable

Current smoke, included in Windows CI full workflow:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-repair-archaeology.ps1 -SkipBuild
```

### Std-1 Candidate Interfaces

Stable for the current P3 freeze candidate:

- project-backed examples can import `std.*` through `AX.toml sources = ["../../std", ...]`
- build source snapshots include `external/std/cli.ax`
- build source snapshots include `external/std/collections.ax`
- build source snapshots include `external/std/env.ax`
- build source snapshots include `external/std/fs.ax`
- build source snapshots include `external/std/option.ax`
- build source snapshots include `external/std/path.ax`
- build source snapshots include `external/std/process.ax`
- build source snapshots include `external/std/report.ax`
- build source snapshots include `external/std/result.ax`
- build source snapshots include `external/std/text.ax`
- build source snapshots include `external/std/workspace.ax`

Allowed to evolve carefully:

- adding a new `std/` module after it has a real workload and interface snapshot coverage
- moving a helper from `foundation/` to `std/` after it satisfies docs、examples、regression and diagnostics/runtime boundary requirements

Not allowed without explicit contract update:

- removing a `std/` source from project build snapshots
- exposing Rust crate names as AX user-facing imports
- expanding `std.collections` beyond minimal `string_list` wrappers and queries before generic collection workload coverage exists

### Local Path Package v0

Stable for the current package-interface slice:

- main projects may declare local AX packages as `[dependencies] alias = { path = "relative/path" }`
- dependency aliases use AX identifier rules because they become module roots
- dependency package sources are loaded from the dependency manifest's `[package].sources`
- dependency modules must declare paths under the dependency alias, for example `module config_rules.validate;`
- `axc build` packages dependency sources under their project-relative paths when they live inside the project tree
- `axc build` packages dependency sources under `external/<package-root>/...` when the path package lives outside the project tree
- `build-manifest.json` uses schema version `5` and exposes `local_path_packages` for projects that declare path packages
- each build-manifest package entry includes `alias`, `root`, `manifest`, `source_count`, and sorted `modules`
- `build-manifest.json` exposes `package_graph_readiness` for local path package projects, including `package_mode`, `reproducible`, `aot_ready`, `lock_status`, `risk_level`, `blocking_reasons`, and `recommended_commands`
- build package graph readiness must keep `aot_ready = false` until native local package linking semantics exist, even when `AX.lock` is current
- `axc lock <project>` writes `AX.lock` as stable JSON with schema version `1`
- `axc lock <project> --check` validates that the checked-in `AX.lock` matches the current local path package graph
- `AX.lock` v0 records only local path packages: root package name, dependency `alias`, `kind = "path"`, dependency package name, declared path, manifest path, source count, and sorted modules
- `axc lock <project> --check` failures use stable `LX****` text codes:
  - `LX0001`: `AX.lock` is missing for a project with local path packages
  - `LX0002`: `AX.lock` is stale or no longer matches the current package graph
  - `LX0003`: `AX.lock` exists but cannot be read
  - `LX0004`: the compiler cannot render the expected lockfile from the current package graph
- `PX****` project resolver failures and `LX****` lock check failures include AI-facing repair hints in CLI stderr:
  - `repair_rule`
  - `repair_goal`
  - `fixit`
- stale lock reports include issue kinds such as `dependency_count_changed`, `dependency_source_count_changed`, `dependency_modules_changed`, `dependency_metadata_changed`, `dependency_missing`, and `dependency_removed`
- context `overview`、`topology` and `evidence` expose `local_package_lock` for projects that declare local path packages, with `status = missing/current/stale/unreadable/unavailable`
- context `local_package_lock.issues[]` exposes the same lock check issue code, kind, message, fixit, `repair_rule`, and `repair_goal` used by `axc lock --check`
- context `evidence` also exposes `package_graph_readiness` for local path package projects:
  - `package_mode = local_path_v0`
  - `reproducible = true` only when `AX.lock` is current
  - `aot_ready = false` until local package AOT linking semantics exist
  - `risk_level = medium/high` summarizes whether the package graph is reproducible
  - `blocking_reasons` explains stale/missing lock risk and future AOT linking blockers
- package resolver failures use stable `PX****` text codes before source diagnostics exist:
  - `PX0001`: dependency alias is not a valid AX module root
  - `PX0002`: dependency path is empty, missing, inaccessible, or not a directory
  - `PX0003`: dependency `AX.toml` is missing, invalid, or declares unsupported package metadata
  - `PX0004`: dependency sources are empty or invalid
  - `PX0005`: dependency module root or module path conflicts with another loaded source
  - `PX0006`: transitive path dependencies are not supported in v0
  - `PX0007`: dependency source expands to a duplicate or entry-overlapping source
- context `overview` and `topology` expose `local_path_packages` when a project uses path packages
- context `evidence` recommends `axc lock <project> --check` for local path package projects
- `axc check <project> --json --ai` emits `PX****` package resolver failures as normal JSON diagnostics when project loading fails before AX source analysis starts
- package resolver JSON diagnostics point at `AX.toml`, use `expected = ["valid local path package graph"]`, and expose the same package repair hint as `ai.rule_id`, `ai.repair_goal`, and `ai.fixits`
- repair benchmark package cases currently cover every local path package resolver code from `PX0001` through `PX0007`; they repair `AX.toml` as the target file and keep `LX****` lockfile repair out of the source-repair scorer until a separate artifact type is defined

Allowed to evolve carefully:

- richer package diagnostics
- package-aware context facts
- future lockfile fields that extend the existing local path package shape
- stricter public/private export checking

Not allowed without explicit contract update:

- direct `AX import -> Cargo crate` mapping
- registry package resolution
- registry lockfile semantics or version solving
- transitive path dependencies
- version solving

Current smoke coverage:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_path_package
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_package_config_context
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_package_config_build_manifest
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_job_runner
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_lock_generates_and_checks_local_path_packages
```

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
