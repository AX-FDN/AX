# AX Repair Adapter Spec

## Goal

A repair adapter is the glue between AX benchmark artifacts and an external repair engine such as:

- a replay source directory
- Codex CLI
- Claude Code CLI
- another custom automation tool

The repository-level contract is deliberately small. If your adapter obeys it, it can be used by:

- [`../scripts/run-repair-benchmark.ps1`](../scripts/run-repair-benchmark.ps1)
- [`../scripts/compare-repair-feedback.ps1`](../scripts/compare-repair-feedback.ps1)
- [`../scripts/compare-repair-modes.ps1`](../scripts/compare-repair-modes.ps1)

## Required Parameters

Every runner script must accept these PowerShell parameters:

- `-PromptPath`
- `-BundlePath`
- `-OutputPath`
- `-CaseId`
- `-FeedbackMode`

The benchmark runner passes them in this form:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File <runner.ps1> `
  -PromptPath <path> `
  -BundlePath <path> `
  -OutputPath <path> `
  -CaseId <case-id> `
  -FeedbackMode <cold|base|ai> `
  <runner-extra-args...>
```

`RunnerExtraArgs` from the caller are appended unchanged after the required parameters.

## Parameter Meanings

- `PromptPath`
  Path to the provider-neutral Markdown repair prompt exported for this case.
- `BundlePath`
  Path to the structured JSON repair bundle for this case.
- `OutputPath`
  Preferred destination for the repaired AX source file.
- `CaseId`
  Stable benchmark case id.
- `FeedbackMode`
  Experiment branch chosen by the caller. Current values are `cold`, `base`, and `ai`.

Important distinction:

- runner parameter `FeedbackMode` is `cold`, `base`, or `ai`
- bundle field `feedback_mode` inside exported JSON is `cold_prompt`, `base_json`, or `ai_json`

The first describes the experiment branch. The second describes the artifact flavor.

## Input Contract

The adapter may use either input artifact as its primary source of truth:

- `PromptPath`
  Better for tools that work from plain text prompts.
- `BundlePath`
  Better for tools that prefer structured JSON.

The stable bundle fields today are:

- `schema_version`
- `case_id`
- `feedback_mode`
- `diagnostic_command`
- `file`
- `category`
- `repair_goal`
- `notes`
- `expected_codes`
- `expected_ai_rule_ids`
- `source_file`
- `diagnostics`

`diagnostic_command` tells the adapter whether the exported failure came from `axc check --json` or `axc run --json`. Adapters do not need to execute that command themselves, but they may use it to tailor prompts or repair strategy.

Adapters should treat the prompt and bundle as read-only inputs.

## Output Contract

Success contract:

- exit code `0`
- produce the repaired AX source

Accepted success forms:

1. write repaired AX source to `OutputPath`
2. write repaired AX source to `stdout` and exit `0`

The benchmark runner prefers `OutputPath`, but if the file does not exist and `stdout` is non-empty, it will capture stdout into the candidate file.

Failure contract:

- non-zero exit code, or
- zero exit code with neither `OutputPath` nor non-empty stdout

The runner records stdout and stderr either way.

## Output Quality Requirements

The repository expects the final repair to be:

- the full AX source, not a patch
- plain source code, not Markdown fences
- compatible with the currently implemented AX prototype

Adapters may internally ask a model for JSON, fenced code, or another format, but they must normalize that into final AX source before returning.

## Recommended Behavior

Adapters should:

- prefer the smallest valid repair
- preserve explicit AX type annotations
- avoid inventing unsupported prototype features
- keep the process deterministic where practical
- log provider stderr without hiding it

Adapters should not:

- mutate benchmark artifacts in place
- assume `PromptPath` or `BundlePath` live under a fixed absolute directory
- rely on interactive input during benchmark runs

## Minimal Replay Example

[`../scripts/replay-repair-adapter.ps1`](../scripts/replay-repair-adapter.ps1) is the reference minimal adapter.

Its behavior is:

1. look up a prewritten candidate by `CaseId`
2. copy that candidate to `OutputPath`
3. exit `0`

It also supports optional replay roots:

- `-SourceDir`
  Shared replay candidate root.
- `-SourceDirCold`
  Cold-start replay override root.
- `-SourceDirBase`
  Base-only override root.
- `-SourceDirAi`
  AI-only override root.

Lookup order is:

1. mode-specific override root for the current `FeedbackMode`, if provided
2. shared `SourceDir`, if provided

This lets smoke tests replay deterministic `base` versus `ai` outcomes without changing the runner contract.

## Real Model Example

[`../scripts/codex-repair-adapter.ps1`](../scripts/codex-repair-adapter.ps1) is the reference model-backed adapter.

It additionally supports:

- `-CodexCommand <name-or-path>`
- `-Model <model-name>`
- `-Profile <profile-name>`
- `-ConfigOverride @('key=value', ...)`

It reads the exported prompt and bundle, invokes `codex exec`, and normalizes the result to a single `repaired_source` value before writing the final AX source.

## Compatibility Checklist

An adapter is compatible with AX benchmark tooling if all of the following are true:

- it accepts the five required parameters
- it can run non-interactively
- it returns repaired AX source on success
- it exits non-zero on real failure
- it does not require the caller to parse provider-specific output formats

If all five hold, `run-repair-benchmark.ps1` and `compare-repair-feedback.ps1` can use it directly.

The same runner contract is also used by `compare-repair-modes.ps1`, which exercises the three-mode `cold -> base -> ai` comparison ladder over one exported benchmark snapshot.
