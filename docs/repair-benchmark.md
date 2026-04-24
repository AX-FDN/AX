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
- [`../examples`](../examples)
  Broken AX source files referenced by the manifests.

The full manifest schema is:

```json
{
  "version": 1,
  "description": "Stable broken AX programs for single-round repair experiments.",
  "cases": [
    {
      "id": "missing_semicolon_basic",
      "file": "examples/missing_semicolon.ax",
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
  Repository-relative path to the broken AX source.
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
  Broken source copied from the manifest input.
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

## Score Step

Use [`../scripts/score-repair-benchmark.ps1`](../scripts/score-repair-benchmark.ps1) to validate repaired candidates:

```powershell
.\scripts\score-repair-benchmark.ps1 -CandidatesDir .ax-ai\repair-candidates\demo
```

Candidate lookup supports two layouts:

- `.ax-ai\repair-candidates\demo\<case-id>.ax`
- `.ax-ai\repair-candidates\demo\<case-id>\repaired.ax`

The scorer runs:

- `axc check <candidate> --json`
- `axc run <candidate> --json` for cases whose `diagnostic_command` is `run`
- optionally `axc run <candidate>` when `-RunPrograms` is enabled for `check`-based cases that already passed `check`

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
  Relative gain over the base passed-case count when base is non-zero.
- `improved_cases`
  Case ids that failed in base and passed in AI.
- `regressed_cases`
  Case ids that passed in base and failed in AI.

## Smoke Workflow

For CI and fast local sanity checks, use [`../scripts/smoke-repair-benchmark.ps1`](../scripts/smoke-repair-benchmark.ps1):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-repair-benchmark.ps1
```

This script uses the smoke manifest plus replay candidates committed in the repository. It is intended to prove that:

- the benchmark manifests still export cleanly
- the runner contract still works
- scoring still works end to end

It also asserts the stable `run-summary.json` and `score/summary.json` contracts for the current 10-case smoke subset, including the `run --json` validation path for the two runtime repair cases.

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

For CI contract checks of that three-mode ladder, use [`../scripts/smoke-compare-repair-modes.ps1`](../scripts/smoke-compare-repair-modes.ps1). It replays the committed `cold`, `base`, and shared candidate sets and asserts the stable 10-case `comparison.json` contract, including pairwise lift totals and runtime category counts.

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
