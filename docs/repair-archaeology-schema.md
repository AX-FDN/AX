# AX Repair Archaeology Artifact Schema

> 本文定义 `Repair Archaeology v0` 的 case 级 JSON artifact 与 Markdown 报告结构。
> 它是 [`repair-archaeology.md`](./repair-archaeology.md) 的实现契约，不是新的 `axc` 命令面。

## 目标

`Repair Archaeology v0` 的 artifact 要能从当前已有资产映射出来：

- repair manifest
- benchmark export `index.json`
- `bundle.cold/base/ai.json`
- `diagnostics.base/ai.json`
- run summary
- score summary
- compare summary
- optional `context_bundle`

v0 不重新定义修复流程，只把已有事实整理成单个 case 可读、可查询、可引用的证据对象。

## Artifact 文件布局

推荐第一版输出目录：

```text
.ax-ai/repair-archaeology/<run-id>/
  index.json
  cases/
    <case-id>.json
    <case-id>.md
```

`index.json` 只做目录和总览；每个 case 的完整证据放在 `cases/<case-id>.json`。

## Case JSON Schema

最小 case artifact 结构：

```json
{
  "schema_version": 1,
  "generated_at": "2026-04-28T00:00:00.0000000+08:00",
  "case": {
    "id": "slice_assignment_read_only",
    "category": "semantic",
    "diagnostic_command": "check",
    "repair_goal": "Rewrite the write to target the original mutable array instead of the slice view.",
    "notes": "Full benchmark semantic slice case."
  },
  "subject": {
    "kind": "file",
    "file": "examples/slice_assignment.ax",
    "project": null,
    "project_target_relative_path": null
  },
  "initial_diagnostic": {
    "expected_codes": ["S0035"],
    "observed_codes": ["S0035"],
    "expected_ai_rule_ids": ["slice_values_are_read_only"],
    "observed_ai_rule_ids": ["slice_values_are_read_only"],
    "primary_code": "S0035",
    "primary_rule_id": "slice_values_are_read_only",
    "primary_repair_goal": "Rewrite the write to target the original mutable array instead of the slice view."
  },
  "repair_contract": {
    "feedback_modes": ["cold", "base", "ai"],
    "pass_condition": "check has no diagnostics",
    "runtime_pass_condition": "run cases must not emit runtime diagnostics",
    "candidate_budget_per_mode": 1
  },
  "context": {
    "included": false,
    "symbol": null,
    "views": [],
    "artifact_path": null
  },
  "modes": [
    {
      "name": "base",
      "input": {
        "bundle_path": "slice_assignment_read_only/bundle.base.json",
        "prompt_path": "slice_assignment_read_only/prompt.base.md"
      },
      "candidate": {
        "status": "ok",
        "path": ".ax-ai/repair-runs/base/candidates/slice_assignment_read_only.ax",
        "invocation_exit_code": 0,
        "timed_out": false
      },
      "validation": {
        "status": "failed",
        "success": false,
        "check_exit_code": 1,
        "remaining_codes": ["S0035"],
        "diagnostics_path": ".ax-ai/repair-runs/base/score/slice_assignment_read_only/diagnostics.json",
        "run": null
      }
    },
    {
      "name": "ai",
      "input": {
        "bundle_path": "slice_assignment_read_only/bundle.ai.json",
        "prompt_path": "slice_assignment_read_only/prompt.ai.md"
      },
      "candidate": {
        "status": "ok",
        "path": ".ax-ai/repair-runs/ai/candidates/slice_assignment_read_only.ax",
        "invocation_exit_code": 0,
        "timed_out": false
      },
      "validation": {
        "status": "passed",
        "success": true,
        "check_exit_code": 0,
        "remaining_codes": [],
        "diagnostics_path": ".ax-ai/repair-runs/ai/score/slice_assignment_read_only/diagnostics.json",
        "run": null
      }
    }
  ],
  "comparison": {
    "delta": "improved",
    "base_success": false,
    "ai_success": true,
    "cold_success": null,
    "improved_modes": ["ai"],
    "regressed_modes": []
  },
  "archaeology_summary": {
    "classification": "ai_feedback_lift",
    "facts": [
      "base left S0035 diagnostics",
      "ai passed check with no remaining diagnostics"
    ],
    "interpretation": "AI-enhanced feedback narrowed the repair target to the read-only slice write."
  },
  "reproducibility": {
    "benchmark_index": ".ax-ai/repair-benchmark/showcase-current/index.json",
    "run_summary_paths": {
      "base": ".ax-ai/repair-comparisons/showcase-current/base/run-summary.json",
      "ai": ".ax-ai/repair-comparisons/showcase-current/ai/run-summary.json"
    },
    "score_summary_paths": {
      "base": ".ax-ai/repair-comparisons/showcase-current/base/score/summary.json",
      "ai": ".ax-ai/repair-comparisons/showcase-current/ai/score/summary.json"
    },
    "commands": [
      "powershell -NoProfile -ExecutionPolicy Bypass -File .\\scripts\\export-repair-benchmark.ps1 -ManifestPath benchmarks\\repair-cases.json -OutputDir .ax-ai\\repair-benchmark\\showcase-current -SkipBuild",
      "powershell -NoProfile -ExecutionPolicy Bypass -Command \"& { .\\scripts\\compare-repair-feedback.ps1 ... }\""
    ]
  },
  "provenance": {
    "repo_relative": true,
    "source_kind": "deterministic_replay",
    "live_model_claim": false
  }
}
```

## 字段来源

| Artifact 字段 | 来源 | 类型 |
| --- | --- | --- |
| `case.*` | repair manifest / export `index.json` | replay fact |
| `subject.*` | manifest `file/project` and export project fields | replay fact |
| `initial_diagnostic.expected_*` | manifest expected fields | replay fact |
| `initial_diagnostic.observed_*` | export `case.json.observed` and `diagnostics.ai.json` | compiler fact |
| `repair_contract.*` | benchmark method and current script limits | method fact |
| `context.*` | `bundle.*.json.context_bundle` when `-IncludeContext` is enabled | compiler fact |
| `modes[].input` | export `index.json.cases[].artifacts` | replay fact |
| `modes[].candidate` | run summary `cases[]` | runner fact |
| `modes[].validation` | score summary `cases[]` | validation fact |
| `comparison.*` | compare summary `cases[]` or derived from mode validation | derived fact |
| `archaeology_summary.facts` | derived from mode validation and remaining diagnostics | derived fact |
| `archaeology_summary.interpretation` | generated explanation | interpretation |
| `reproducibility.*` | command line and input artifact paths | method fact |
| `provenance.*` | exporter metadata | method fact |

Rule: fields marked as `interpretation` must never be used as benchmark pass/fail source of truth.

## Status Values

`modes[].candidate.status` follows `run-repair-benchmark.ps1`:

| Value | Meaning |
| --- | --- |
| `ok` | candidate file was produced by the adapter/runner |
| `failed` | runner exited unsuccessfully or produced no candidate |
| `timed_out` | runner exceeded timeout |

`modes[].validation.status` follows `score-repair-benchmark.ps1`:

| Value | Meaning |
| --- | --- |
| `passed` | candidate passed validation |
| `failed` | candidate was present but still produced diagnostics or runtime failure |
| `missing` | candidate was not available to score |

`comparison.delta` values:

| Value | Meaning |
| --- | --- |
| `improved` | weaker mode failed and stronger mode passed |
| `regressed` | weaker mode passed and stronger mode failed |
| `both_pass` | compared modes both passed |
| `both_fail` | compared modes both failed |
| `not_comparable` | one or more mode facts are missing |

## Markdown Report Template

Every `cases/<case-id>.md` report should follow this order:

````markdown
# Repair Archaeology: <case-id>

## Summary

- Category:
- Subject:
- Diagnostic command:
- Outcome:
- Claim boundary: deterministic replay, not live-model evidence

## Initial Diagnostic

- Expected codes:
- Observed codes:
- AI rule ids:
- Repair goal:

## Context

- Included:
- Views:
- Symbol:

## Timeline

| Mode | Candidate | Validation | Remaining diagnostics |
| --- | --- | --- | --- |
| cold | ... | ... | ... |
| base | ... | ... | ... |
| ai | ... | ... | ... |

## What Changed

- Replay facts:
- Interpretation:

## Failure / Regression Notes

- If failed:
- If regressed:
- If context missing:

## Reproduce

```powershell
<commands>
```

## Artifacts

- Benchmark index:
- Bundle paths:
- Candidate paths:
- Score paths:
````

Markdown reports are presentation artifacts.
The JSON artifact remains the source of truth.

## Index JSON

`index.json` should summarize a run without duplicating every case field:

```json
{
  "schema_version": 1,
  "generated_at": "2026-04-28T00:00:00.0000000+08:00",
  "source_kind": "deterministic_replay",
  "live_model_claim": false,
  "benchmark_index": ".ax-ai/repair-benchmark/showcase-current/index.json",
  "comparison_path": ".ax-ai/repair-comparisons/showcase-current/comparison.json",
  "totals": {
    "total": 30,
    "improved": 5,
    "regressed": 0,
    "both_pass": 25,
    "both_fail": 0
  },
  "cases": [
    {
      "id": "slice_assignment_read_only",
      "category": "semantic",
      "delta": "improved",
      "json": "cases/slice_assignment_read_only.json",
      "markdown": "cases/slice_assignment_read_only.md"
    }
  ]
}
```

## v0 Non-Goals

- 不调用真实模型。
- 不把 deterministic replay 解释成 live-model 结果。
- 不新增 `axc repair-log` 命令。
- 不新增 UI。
- 不改变 repair benchmark、diagnostics 或 context schema。
- 不把 Markdown 当成机器消费源。

## 下一步实现顺序

1. `export-repair-archaeology.ps1` 已作为 v0 最小脚本入口落地，只读取现有 artifact。
2. 当前先支持 `base -> ai` comparison，后续再扩到 `cold -> base -> ai`。
3. 当前先输出 JSON，再从 JSON 渲染 Markdown。
4. 当前 `smoke-repair-archaeology.ps1` 已覆盖两个 improved case 和一个 both-pass case；当 benchmark 出现 failed/regressed case 时，需要补对应 fixture。
5. 下一步再评估是否把该 smoke 接入 CI 或 interface snapshots。
