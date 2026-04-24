# AX Repair Benchmark Guide

## Purpose

AX treats repair benchmark evidence as a first-class product asset. The benchmark pipeline exists to answer one concrete question:

- does a given feedback mode help an external agent repair broken AX programs more reliably?

For the diagnostics-cost baseline that sits alongside the repair benchmark, use [`../scripts/benchmark-diagnostics.ps1`](../scripts/benchmark-diagnostics.ps1). It measures `check`, `check --json`, and `check --json --ai` over stable broken programs and now emits a stable `summary.json` / `summary.md` report under `.ax-ai\diagnostics-benchmark\<timestamp>\`. The stable schema for that report is documented in [`diagnostics-benchmark-schema.md`](./diagnostics-benchmark-schema.md).

The current repository supports three layers of work:

1. export stable benchmark artifacts from manifest-defined broken programs
2. run a repair adapter against those artifacts
3. score repaired candidates and compare `cold`, `base`, and `ai` feedback branches

## Source Assets

The benchmark source of truth lives in the repository:

- [`../benchmarks/repair-cases.json`](../benchmarks/repair-cases.json)
  Full repair benchmark manifest.
- [`../benchmarks/repair-cases-smoke.json`](../benchmarks/repair-cases-smoke.json)
  Small CI-safe subset.
- [`../benchmarks/repair-candidates/smoke`](../benchmarks/repair-candidates/smoke)
  Replay candidates used by smoke tests.
- [`../benchmarks/repair-candidates/compare/shared`](../benchmarks/repair-candidates/compare/shared)
  Full-manifest shared replay candidates used as the deterministic passing baseline for compare runs.
- [`../benchmarks/repair-projects`](../benchmarks/repair-projects)
  Repository-owned AX project fixtures used by project-backed repair cases.
- [`../examples`](../examples)
  Broken single-file AX source files referenced by the manifests.

The full manifest schema is:

```json
{
  "version": 1,
  "description": "Stable broken AX programs for single-round repair experiments.",
  "cases": [
    {
      "id": "missing_semicolon_basic",
      "file": "examples/missing_semicolon.ax",
      "project": "examples/project_name",
      "category": "syntax",
      "diagnostic_command": "check",
      "expected_codes": ["P0001"],
      "expected_ai_rule_ids": ["statement_terminator_required"],
      "repair_goal": "Insert the missing semicolon after the let binding.",
      "notes": "Baseline parser recovery case."
    }
  ]
}
```

Field meanings:

- `version`
  Manifest schema version. The current scripts require `1`.
- `description`
  Human-readable description only.
- `cases[].id`
  Stable case identifier used in artifacts, candidate lookup, and reports.
- `cases[].file`
  Path to the broken AX source that the adapter should rewrite. Repository-relative paths are the normal form, but the current scripts also accept absolute paths for local contract tests.
- `cases[].project`
  Optional AX project root or `AX.toml` path. When present, diagnostics are exported from the whole project while `cases[].file` still names the specific AX source file that should be repaired.
- `cases[].category`
  Stable grouping key such as `syntax`, `semantic`, or `unsupported`.
- `cases[].diagnostic_command`
  Optional source of the exported diagnostics. Current values are `check` and `run`. When omitted, the scripts default to `check`.
- `cases[].expected_codes`
  Exact diagnostic code sequence expected from both base and AI-enhanced checks.
- `cases[].expected_ai_rule_ids`
  Exact AI rule sequence expected when `--json --ai` is enabled.
- `cases[].repair_goal`
  Human-facing repair target used in exported prompts and reports.
- `cases[].notes`
  Extra case-specific context for prompts and readers.

As of 2026-04-24, the committed manifests also include a repository-backed project-context case:
[`project_helper_missing_semicolon`](../benchmarks/repair-projects/helper_missing_semicolon).
It validates the "repair one target file while keeping the rest of the AX project read-only" path end to end.

## Export Step

Use [`../scripts/export-repair-benchmark.ps1`](../scripts/export-repair-benchmark.ps1) to freeze a benchmark snapshot:

```powershell
.\scripts\export-repair-benchmark.ps1
```

By default it writes to:

```text
.ax-ai\repair-benchmark\<timestamp>\
```

Each case directory contains:

- `source.ax`
  Broken target source copied from `cases[].file`.
- `project\`
  Present only for project-backed cases. Keeps a read-only AX project snapshot containing `AX.toml` plus the `.ax` files needed to re-run `check` or `run` against the whole project.
- `diagnostics.base.json`
  Output of `axc <diagnostic_command> <file> --json`.
- `diagnostics.ai.json`
  Output of `axc <diagnostic_command> <file> --json --ai`.
- `bundle.cold.json`
  Structured repair bundle for the prompt-only `cold` branch. Its `diagnostics` array is intentionally empty.
- `bundle.base.json`
  Structured repair bundle for base feedback.
- `bundle.ai.json`
  Structured repair bundle for AI feedback.
- `prompt.cold.md`
  Provider-neutral prompt for the prompt-only `cold` branch.
- `prompt.base.md`
  Provider-neutral prompt built from the base bundle.
- `prompt.ai.md`
  Provider-neutral prompt built from the AI bundle.
- `case.json`
  Per-case export summary and observed codes.

The export root also contains:

- `index.json`
  Manifest-derived list of all exported cases and artifact paths.

The export script validates the benchmark as it exports it:

- source file must exist
- `diagnostic_command` must be `check` or `run`
- `expected_codes` must match observed compiler output exactly
- `expected_ai_rule_ids` must match observed AI-enhanced output exactly

If any case drifts, export fails immediately.

For `run`-based cases, the exported prompts also include an explicit runtime-repair note so adapters know the failure already passed `check` and should be repaired without introducing new check-time diagnostics.
For project-backed cases, the exported prompts additionally include the project manifest and the other AX source files as read-only context, while keeping the broken target file as the only file the adapter is expected to rewrite.
For stable replay baselines, prefer repaired runtime candidates whose `main` returns `0` after the fix so benchmark evidence does not depend on the program's business result becoming the process exit code.

## Run Step

Use [`../scripts/run-repair-benchmark.ps1`](../scripts/run-repair-benchmark.ps1) to invoke a runner over one exported benchmark:

```powershell
.\scripts\run-repair-benchmark.ps1 `
  -RunnerScript .\scripts\replay-repair-adapter.ps1 `
  -RunnerExtraArgs @('-SourceDir', '.ax-ai\repair-candidates\smoke')
```

Key parameters:

- `-BenchmarkDir`
  Optional. Path to an exported benchmark root or its `index.json`.
- `-RunnerScript`
  Required. Script that consumes `PromptPath`, `BundlePath`, `OutputPath`, `CaseId`, and `FeedbackMode`.
- `-RunnerExtraArgs`
  Optional. Passed through to the runner unchanged.
- `-FeedbackMode`
  Either `base` or `ai`.
- `-RunPrograms`
  Optional. After successful `check`, also execute `axc run`.
- `-RefreshBenchmark`
  Optional. Force a fresh export before running.
- `-SkipBuild`
  Optional. Reuse an already-built `axc` binary instead of invoking `cargo build`.

Runner-specific extra args are passed through unchanged. The replay adapter uses this to support:

- `-SourceDir`
  Shared replay candidate root used by both modes.
- `-SourceDirCold`
  `cold` mode override root.
- `-SourceDirBase`
  Base-only override root searched before `-SourceDir`.
- `-SourceDirAi`
  AI-only override root searched before `-SourceDir`.

For a deterministic full-manifest compare baseline, point `-SourceDir` at
[`../benchmarks/repair-candidates/compare/shared`](../benchmarks/repair-candidates/compare/shared)
and then layer `-SourceDirCold` / `-SourceDirBase` overrides on top for the cases you intentionally keep broken in those branches.

Default output root:

```text
.ax-ai\repair-runs\<timestamp>\
```

Run output contains:

- `candidates\`
  Repaired AX source per case.
- `invocations\`
  Per-case `stdout.txt`, `stderr.txt`, and `invocation.json`.
- `run-summary.json`
  Top-level run result with case statuses.
- `score\`
  Embedded score output, unless `-SkipScore` is used.

For environments where `cargo` is unavailable but a compiled `axc` already exists, point `AXC_BINARY` at that executable and add `-SkipBuild`. `run-repair-benchmark.ps1` now forwards that flag to any nested export and score steps so the full run stays on the prebuilt binary path.
The runner also supports stdout-only adapters: if the child exits `0` and leaves no file at `OutputPath`, non-empty stdout is captured as the candidate source. A zero exit with no file and no stdout is recorded as `failed`.

`run-summary.json` uses these runner statuses:

- `ok`
  Runner produced a candidate successfully.
- `failed`
  Runner exited unsuccessfully or produced no usable candidate.
- `timed_out`
  Runner exceeded the configured timeout.

`-SkipScore` is useful when you want to validate only the runner contract itself, for example when checking whether a new adapter writes `OutputPath` correctly or returns the repaired source on stdout without hanging.

Current `run-summary.json` shape:

```json
{
  "schema_version": 1,
  "generated_at": "2026-04-24T12:34:56.0000000+08:00",
  "feedback_mode": "ai",
  "benchmark_index": "C:\\repo\\.ax-ai\\repair-benchmark\\20260424-123456\\index.json",
  "benchmark_root": "C:\\repo\\.ax-ai\\repair-benchmark\\20260424-123456",
  "runner_script": "C:\\repo\\scripts\\replay-repair-adapter.ps1",
  "runner_extra_args": [],
  "candidates_dir": "C:\\repo\\.ax-ai\\repair-runs\\20260424-123500\\candidates",
  "output_dir": "C:\\repo\\.ax-ai\\repair-runs\\20260424-123500",
  "totals": {
    "total": 11,
    "ok": 11,
    "failed": 0,
    "timed_out": 0
  },
  "score": {
    "skipped": false,
    "summary_path": "C:\\repo\\.ax-ai\\repair-runs\\20260424-123500\\score\\summary.json",
    "exit_code": 0
  },
  "cases": [
    {
      "id": "missing_semicolon_basic",
      "feedback_mode": "ai",
      "prompt_path": "C:\\repo\\.ax-ai\\repair-benchmark\\20260424-123456\\missing_semicolon_basic\\prompt.ai.md",
      "bundle_path": "C:\\repo\\.ax-ai\\repair-benchmark\\20260424-123456\\missing_semicolon_basic\\bundle.ai.json",
      "output_path": "C:\\repo\\.ax-ai\\repair-runs\\20260424-123500\\candidates\\missing_semicolon_basic.ax",
      "status": "ok",
      "timed_out": false,
      "exit_code": 0,
      "stdout_log": "C:\\repo\\.ax-ai\\repair-runs\\20260424-123500\\invocations\\missing_semicolon_basic\\stdout.txt",
      "stderr_log": "C:\\repo\\.ax-ai\\repair-runs\\20260424-123500\\invocations\\missing_semicolon_basic\\stderr.txt"
    }
  ]
}
```

Stable top-level fields:

- `schema_version: integer`
  Current run summary schema version. The repository currently emits `1`.
- `generated_at: string`
  ISO-8601 generation timestamp.
- `feedback_mode: "cold" | "base" | "ai"`
  Branch used when selecting prompts and bundles.
- `benchmark_index: string`
  Absolute path to the exported benchmark `index.json` used for this run.
- `benchmark_root: string`
  Absolute path to the parent directory of `benchmark_index`.
- `runner_script: string`
  Absolute path to the adapter script that was invoked for every case.
- `runner_extra_args: string[]`
  Extra adapter arguments appended after the required runner contract parameters.
- `candidates_dir: string`
  Absolute path to the directory where repaired candidates are written.
- `output_dir: string`
  Absolute path to the run root.
- `totals`
  Object with `total`, `ok`, `failed`, and `timed_out`.
- `score`
  Object with:
  `skipped: bool`, `summary_path: string | null`, `exit_code: integer | null`.
- `cases`
  Ordered per-case invocation records.

Stable per-case run fields:

- `id: string`
  Stable benchmark case id.
- `feedback_mode: "cold" | "base" | "ai"`
  Experiment branch used for this invocation.
- `prompt_path: string`
  Absolute path to the prompt passed to the runner.
- `bundle_path: string`
  Absolute path to the structured repair bundle passed to the runner.
- `output_path: string`
  Preferred candidate output path for this case.
- `status: "ok" | "failed" | "timed_out"`
  Final runner outcome for the case.
- `timed_out: bool`
  Convenience flag mirroring the timeout branch.
- `exit_code: integer | null`
  Runner exit code, or `null` when the process timed out before exit.
- `stdout_log: string`
- `stderr_log: string`
  Absolute paths to the recorded adapter logs for the case.

## Score Step

Use [`../scripts/score-repair-benchmark.ps1`](../scripts/score-repair-benchmark.ps1) to validate repaired candidates:

```powershell
.\scripts\score-repair-benchmark.ps1 -CandidatesDir .ax-ai\repair-candidates\demo
```

Candidate lookup supports two layouts:

- `.ax-ai\repair-candidates\demo\<case-id>.ax`
- `.ax-ai\repair-candidates\demo\<case-id>\repaired.ax`

For project-backed benchmark cases, the candidate is still one repaired AX file. The scorer reconstructs the exported `project\` snapshot, overwrites the target file named by the benchmark metadata, and then runs `axc check` / `axc run` against that working project root.

The scorer runs:

- `axc check <candidate> --json`
- `axc run <candidate> --json` for cases whose `diagnostic_command` is `run`
- optionally `axc run <candidate>` when `-RunPrograms` is enabled for `check`-based cases that already passed `check`

Before invoking `axc`, the scorer rewrites each candidate into its per-case output directory as BOM-free UTF-8.
That keeps Windows-authored replay files stable even when an adapter writes UTF-8 with a leading BOM or a BOM-marked UTF-16 file.

Per-case score status is:

- `passed`
  `axc check` succeeded and emitted no diagnostics, and for `run`-based cases the repaired program also produced no runtime diagnostics under `axc run --json`.
- `failed`
  Candidate existed but still failed `check`, or it still failed the required runtime validation for a `run`-based case.
- `missing`
  No candidate file was found for that case.

Standalone score output defaults to:

```text
.ax-ai\repair-results\<timestamp>\
```

Important files:

- `summary.json`
  Full score summary.
- `<case-id>\result.json`
  Per-case score result.
- `<case-id>\diagnostics.json`
  Remaining diagnostics for failed repairs.

Like export, scoring also honors `AXC_BINARY` and `CARGO_BIN_EXE_axc`. Pair either variable with `-SkipBuild` when you want to score against a prebuilt binary without rebuilding the workspace.

Current `summary.json` shape:

```json
{
  "schema_version": 1,
  "generated_at": "2026-04-24T12:35:10.0000000+08:00",
  "benchmark_dir": "C:\\repo\\.ax-ai\\repair-benchmark\\20260424-123456",
  "benchmark_index": "C:\\repo\\.ax-ai\\repair-benchmark\\20260424-123456\\index.json",
  "candidates_dir": "C:\\repo\\.ax-ai\\repair-runs\\20260424-123500\\candidates",
  "output_dir": "C:\\repo\\.ax-ai\\repair-runs\\20260424-123500\\score",
  "totals": {
    "total": 11,
    "passed": 11,
    "failed": 0,
    "missing": 0
  },
  "cases": [
    {
      "id": "index_out_of_bounds_runtime",
      "diagnostic_command": "run",
      "status": "passed",
      "success": true,
      "candidate_path": "C:\\repo\\.ax-ai\\repair-runs\\20260424-123500\\candidates\\index_out_of_bounds_runtime.ax",
      "remaining_codes": [],
      "diagnostics": [],
      "check_exit_code": 0,
      "run": {
        "command": "run --json",
        "command_exit_code": 0,
        "parsed_diagnostics": false,
        "diagnostics": [],
        "remaining_codes": []
      }
    }
  ]
}
```

Stable top-level score fields:

- `schema_version: integer`
  Current score summary schema version. The repository currently emits `1`.
- `generated_at: string`
  ISO-8601 generation timestamp.
- `benchmark_dir: string`
  Absolute path to the benchmark root that contains `index.json`.
- `benchmark_index: string`
  Absolute path to the benchmark `index.json`.
- `candidates_dir: string`
  Absolute path to the candidate directory being scored.
- `output_dir: string`
  Absolute path to the score output root.
- `totals`
  Object with `total`, `passed`, `failed`, and `missing`.
- `cases`
  Ordered per-case score results.

Stable per-case score fields:

- `id: string`
  Stable benchmark case id.
- `diagnostic_command: "check" | "run"`
  Validation path expected for that benchmark case.
- `status: "passed" | "failed" | "missing"`
  Final score outcome.
- `success: bool`
  Convenience success flag.
- `candidate_path: string | null`
  Absolute path to the candidate used for scoring, or `null` for missing cases.
- `benchmark_case: object`
  Embedded benchmark case metadata copied from `index.json`.
- `remaining_codes: string[]`
  Diagnostic codes still present after scoring.
- `diagnostics: object[]`
  Parsed `axc check --json` diagnostics that remain after repair.
- `check_exit_code: integer | null`
  Exit code returned by `axc check --json`.
- `run?: object`
  Present only for runtime validation cases or when `-RunPrograms` executes the repaired program.
  For runtime benchmark cases this object keeps `command`, `command_exit_code`, `parsed_diagnostics`, `diagnostics`, and `remaining_codes`.
  Runtime pass/fail is driven by parsed runtime diagnostics, not by `command_exit_code` alone, because AX currently reflects `main`'s integer return value into the process exit code.
  Clean `axc run --json` executions usually emit no JSON payload, so `parsed_diagnostics` will often be `false` while `diagnostics` and `remaining_codes` stay empty.

## Compare Step

Use [`../scripts/compare-repair-feedback.ps1`](../scripts/compare-repair-feedback.ps1) for the formal `base` versus `ai` experiment:

```powershell
.\scripts\compare-repair-feedback.ps1 `
  -RunnerScript .\scripts\codex-repair-adapter.ps1 `
  -RunnerExtraArgs @('-Model', 'gpt-5.4')
```

This script:

1. resolves or exports one benchmark snapshot
2. runs the same runner twice against the same snapshot
3. scores both runs
4. computes pass-rate lift and per-category deltas

`compare-repair-feedback.ps1` and `compare-repair-modes.ps1` also accept `-SkipBuild` and propagate it through their nested export and run stages, so comparison jobs can stay fully reproducible on machines that only have a prebuilt `axc`.

Default output root:

```text
.ax-ai\repair-comparisons\<timestamp>\
```

Comparison output contains:

- `comparison.json`
  Machine-readable summary of both modes, category splits, and per-case deltas.
- `comparison.md`
  Human-readable report.
- `base\`
  Full run and score outputs for base diagnostics.
- `ai\`
  Full run and score outputs for AI-enhanced diagnostics.

Comparison metrics:

- `base_pass_rate`
  Percentage of cases passed in base mode.
- `ai_pass_rate`
  Percentage of cases passed in AI mode.
- `absolute_lift_cases`
  `ai_passed - base_passed`.
- `absolute_lift_pp`
  Pass-rate gain in percentage points.
- `relative_lift_pct`
  Relative gain over the base passed-case count when base is non-zero, otherwise `null`.
- `improved_cases`
  Case ids that failed in base and passed in AI.
- `regressed_cases`
  Case ids that passed in base and failed in AI.

Current `comparison.json` shape for `compare-repair-feedback.ps1`:

```json
{
  "schema_version": 1,
  "generated_at": "2026-04-24T12:36:00.0000000+08:00",
  "benchmark_index": "C:\\repo\\.ax-ai\\repair-benchmark\\20260424-123456\\index.json",
  "runner_script": "C:\\repo\\scripts\\replay-repair-adapter.ps1",
  "runner_extra_args": [],
  "output_dir": "C:\\repo\\.ax-ai\\repair-comparisons\\20260424-123600",
  "modes": {
    "base": {
      "exit_code": 1,
      "timed_out": false,
      "stdout_log": "C:\\repo\\.ax-ai\\repair-comparisons\\20260424-123600\\base.stdout.txt",
      "stderr_log": "C:\\repo\\.ax-ai\\repair-comparisons\\20260424-123600\\base.stderr.txt",
      "run_summary_path": "C:\\repo\\.ax-ai\\repair-comparisons\\20260424-123600\\base\\run-summary.json",
      "score_summary_path": "C:\\repo\\.ax-ai\\repair-comparisons\\20260424-123600\\base\\score\\summary.json",
      "invocation_totals": { "total": 11, "ok": 11, "failed": 0, "timed_out": 0 },
      "score_totals": { "total": 11, "passed": 6, "failed": 5, "missing": 0 }
    },
    "ai": {
      "exit_code": 0,
      "timed_out": false,
      "stdout_log": "C:\\repo\\.ax-ai\\repair-comparisons\\20260424-123600\\ai.stdout.txt",
      "stderr_log": "C:\\repo\\.ax-ai\\repair-comparisons\\20260424-123600\\ai.stderr.txt",
      "run_summary_path": "C:\\repo\\.ax-ai\\repair-comparisons\\20260424-123600\\ai\\run-summary.json",
      "score_summary_path": "C:\\repo\\.ax-ai\\repair-comparisons\\20260424-123600\\ai\\score\\summary.json",
      "invocation_totals": { "total": 11, "ok": 11, "failed": 0, "timed_out": 0 },
      "score_totals": { "total": 11, "passed": 11, "failed": 0, "missing": 0 }
    }
  },
  "comparison": {
    "total_cases": 11,
    "base_passed": 6,
    "ai_passed": 11,
    "base_pass_rate": 54.55,
    "ai_pass_rate": 100,
    "absolute_lift_cases": 5,
    "absolute_lift_pp": 45.45,
    "relative_lift_pct": 83.33,
    "improved_cases": [],
    "regressed_cases": [],
    "unchanged_cases": []
  },
  "categories": [],
  "cases": []
}
```

Stable top-level comparison fields:

- `schema_version: integer`
  Current comparison schema version. The repository currently emits `1`.
- `generated_at: string`
  ISO-8601 generation timestamp.
- `benchmark_index: string`
  Absolute path to the benchmark `index.json` shared by both runs.
- `runner_script: string`
  Absolute path to the adapter script used in both modes.
- `runner_extra_args: string[]`
  Extra adapter arguments applied to both modes.
- `output_dir: string`
  Absolute path to the comparison root.
- `modes.base` and `modes.ai`
  Each keeps `exit_code`, `timed_out`, `stdout_log`, `stderr_log`, `run_summary_path`, `score_summary_path`, `invocation_totals`, and `score_totals`.
- `comparison`
  Summary object with `total_cases`, passed counts, pass rates, lift metrics, and `improved_cases` / `regressed_cases` / `unchanged_cases`. `relative_lift_pct` is `null` when the baseline passed-case count is zero.
- `categories`
  Per-category aggregates. Each category keeps counts, pass rates, lift, and improved/regressed case id arrays.
- `cases`
  Per-case deltas. Each case keeps `base_status`, `ai_status`, `base_success`, `ai_success`, `delta`, and remaining-code arrays for both modes.

## Smoke Workflow

For CI and fast local sanity checks, use [`../scripts/smoke-repair-benchmark.ps1`](../scripts/smoke-repair-benchmark.ps1):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-repair-benchmark.ps1
```

This script uses the smoke manifest plus replay candidates committed in the repository. It is intended to prove that:

- the benchmark manifests still export cleanly
- the runner contract still works
- scoring still works end to end

It also asserts the stable `run-summary.json` and `score/summary.json` contracts for the current 11-case smoke subset, including the `run --json` validation path for the two runtime repair cases and the committed project-backed helper repair case.

If your local environment does not expose `cargo`, the smoke entrypoints also accept `-SkipBuild`; combine that with `AXC_BINARY=<path-to-axc>` to replay the full smoke evidence chain against an existing compiler binary.

It is not intended to prove model quality.

For the diagnostics baseline path, use [`../scripts/smoke-benchmark-diagnostics.ps1`](../scripts/smoke-benchmark-diagnostics.ps1). It asserts the stable `summary.json` contract produced by `benchmark-diagnostics.ps1`, including schema version, mode order, case count, and per-mode row counts.

For the comparison path itself, use [`../scripts/smoke-compare-repair-feedback.ps1`](../scripts/smoke-compare-repair-feedback.ps1):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-compare-repair-feedback.ps1
```

This compare smoke intentionally replays:

- one shared repaired candidate set
- one `base`-only override set that leaves five cases still broken, including three semantic cases and two runtime cases

The committed `compare/shared` directory is broader than this smoke subset: it is intended to cover the full repair manifest so deterministic full-manifest compare runs can reuse one passing shared baseline instead of rebuilding ad hoc replay roots every time. That shared baseline now also includes the project-backed `project_helper_missing_semicolon` replay candidate.

It then asserts the stable `comparison.json` contract, including:

- schema version
- total case count
- `base_passed` and `ai_passed`
- absolute lift
- improved and regressed case ids
- runtime category totals and pass counts

This gives CI a deterministic proof that `compare-repair-feedback.ps1` still produces a comparable machine-readable report, not just two ad hoc benchmark runs.

For the three-mode report, use [`../scripts/compare-repair-modes.ps1`](../scripts/compare-repair-modes.ps1):

```powershell
.\scripts\compare-repair-modes.ps1 `
  -RunnerScript .\scripts\codex-repair-adapter.ps1 `
  -RunnerExtraArgs @('-Model', 'gpt-5.4')
```

This adds a fixed `cold` -> `base` -> `ai` ladder on top of the same exported benchmark snapshot and writes a three-mode `comparison.json` under `.ax-ai\repair-mode-comparisons\<timestamp>\`.

The three-mode `comparison.json` keeps the same outer contract style with these additions:

- `mode_order: string[]`
  Stable current mode order. The repository currently emits `["cold", "base", "ai"]`.
- `modes.cold`
  Same shape as `modes.base` and `modes.ai`.
- `summary`
  Three-mode aggregate object with pass counts, pass rates, and `pairwise_comparisons`.
- `summary.pairwise_comparisons.<pair>`
  Each pair currently keeps:
  `from_mode`, `to_mode`, `from_passed`, `to_passed`, `from_pass_rate`, `to_pass_rate`,
  `absolute_lift_cases`, `absolute_lift_pp`, `relative_lift_pct`, `improved_cases`,
  `regressed_cases`, and `unchanged_cases`. `relative_lift_pct` is `null` when `from_passed` is zero.
- `cases`
  Per-case deltas now keep `cold_to_base_delta`, `base_to_ai_delta`, and `cold_to_ai_delta`.
- `categories`
  Per-category aggregates now keep `cold_passed`, `base_passed`, `ai_passed`, the corresponding pass rates, and nested `pairwise_lifts`.

For CI contract checks of that three-mode ladder, use [`../scripts/smoke-compare-repair-modes.ps1`](../scripts/smoke-compare-repair-modes.ps1). It replays the committed `cold`, `base`, and shared candidate sets and asserts the stable 11-case `comparison.json` contract, including pairwise lift totals and runtime category counts.

## Stability Policy

The current repository treats these as stable external assets:

- manifest version `1`
- export `index.json` and per-case artifact naming
- runner parameter contract
- score and comparison summary roots
- diagnostic code sequences and AI rule ids pinned in the manifests

These are intentionally not promised as public stable contracts yet:

- the exact wording of provider-neutral prompts
- private session state files used by `--ai-session`
- the internal model-specific prompting strategy of a given adapter
