# AX Benchmark Showcase

This page is the shortest honest answer to one question:

> What has AX already proved, how was it measured, and what is still unproven?

## Executive Summary

Current reproduced snapshot on `2026-04-24`:

| Item | Current value |
| --- | --- |
| Manifest | [`../benchmarks/repair-cases.json`](../benchmarks/repair-cases.json) |
| Total cases | `26` |
| Compare ladder | `cold -> base -> ai` |
| `base` result | `21/26` |
| `ai` result | `26/26` |
| `base -> ai` lift | `+5` repaired cases |
| `base -> ai` lift | `+19.23` percentage points |

This is already a real evidence loop.
It is **not yet** the final public proof against Rust / Go / Python subsets.

## What This Page Shows

This page summarizes the current verified AX-internal evidence chain:

- a fixed repair manifest
- deterministic replay candidates
- stable export / run / score / compare scripts
- reproduced comparison results from the current repository state

It does **not** claim that AX has already beaten Rust, Go, or Python subsets in a public cross-language benchmark.
That is the next benchmark step, not a finished result.

## Case Set

The current full manifest lives in [`../benchmarks/repair-cases.json`](../benchmarks/repair-cases.json).

### Category Breakdown

| Category | Cases | Current `base` | Current `ai` | Representative cases |
| --- | ---: | ---: | ---: | --- |
| `syntax` | 2 | 2/2 | 2/2 | `missing_semicolon_basic`, `missing_paren_condition` |
| `semantic` | 19 | 16/19 | 19/19 | `type_mismatch_bool_from_int`, `missing_struct_literal_field`, `slice_assignment_read_only` |
| `runtime` | 2 | 0/2 | 2/2 | `index_out_of_bounds_runtime`, `division_by_zero_runtime` |
| `unsupported` | 3 | 3/3 | 3/3 | `import_declaration_unsupported`, `module_declaration_unsupported`, `empty_array_literal_unsupported` |

### Why This Case Mix Matters

The current manifest is already wider than trivial parser demos.
It covers:

- simple syntax recovery
- semantic mismatches around types and shapes
- runtime failures that passed `check`
- explicit unsupported-surface cases

That matters because AX is trying to prove a repair protocol, not only a prettier parser error.

## Method

The current reproduced result is a deterministic replay comparison.
That is deliberate.

| Dimension | Current setup |
| --- | --- |
| Snapshot date | `2026-04-24` |
| Export input | fixed manifest + broken AX source files |
| Feedback modes | `cold`, `base`, `ai` |
| Benchmark budget | one repair attempt per case per mode |
| Runner in reproduced report | [`../scripts/replay-repair-adapter.ps1`](../scripts/replay-repair-adapter.ps1) |
| Shared passing baseline | [`../benchmarks/repair-candidates/compare/shared`](../benchmarks/repair-candidates/compare/shared) |
| Mode-specific overrides | [`../benchmarks/repair-candidates/compare/cold`](../benchmarks/repair-candidates/compare/cold), [`../benchmarks/repair-candidates/compare/base`](../benchmarks/repair-candidates/compare/base) |
| Scoring | [`../scripts/score-repair-benchmark.ps1`](../scripts/score-repair-benchmark.ps1) |
| Pass condition | candidate finishes with no remaining check diagnostics; `run` cases also fail if runtime diagnostics remain |

This setup isolates protocol drift from live-model variance.
It answers:

- did the benchmark assets stay stable?
- did the feedback contract keep its intended shape?
- is the measured lift still present when only feedback mode changes?

## Results Summary

### Full Compare Replay

Using the committed shared replay baseline plus base-only overrides:

- `cold`: `19/26` passed
- `base`: `21/26` passed
- `ai`: `26/26` passed
- `base -> ai` lift: `+5` repaired cases
- `base -> ai` lift: `+19.23` percentage points
- `cold -> ai` lift: `+7` repaired cases

### Improved Cases In `base -> ai`

The current reproduced `base -> ai` lift comes from:

- `type_mismatch_bool_from_int`
- `missing_struct_literal_field`
- `index_out_of_bounds_runtime`
- `division_by_zero_runtime`
- `slice_assignment_read_only`

### Category-Level Lift

The strongest visible gains in the current snapshot are:

- runtime repair: `0/2 -> 2/2`
- semantic repair: `16/19 -> 19/19`

That is important because it means the lift is not concentrated only in punctuation mistakes.

## Failure Sample

One useful failure sample is [`../examples/slice_assignment.ax`](../examples/slice_assignment.ax):

```ax
fn main() -> i32 {
    let values: [i32; 3] = [1, 2, 3];
    let mut view: [i32] = values[0:2];
    view[0] = 9;
    return 0;
}
```

Base structured diagnostics on this file currently tell you:

- `code: S0035`
- message: cannot assign through slice variable `view` because slices are read-only
- suggestion: assign through the original mutable array instead of a slice view

AI-enhanced diagnostics on the same file add:

- `rule_id: slice_values_are_read_only`
- a concrete `repair_goal`
- `focus_item`
- `relevant_spans`
- `rule_card`
- `fixits`

That is the difference AX is trying to benchmark:

- not just whether the compiler says "wrong"
- but whether the repair payload narrows the fix well enough to improve a single repair attempt

## Why The Replay Baseline Matters

The replay comparison is intentionally deterministic.
It isolates the effect of the feedback contract and benchmark assets from live-model variance.

That makes it useful for:

- regression testing protocol changes
- checking whether benchmark assets still encode the intended differences between `cold`, `base`, and `ai`
- preventing benchmark drift from being hidden behind anecdotal model behavior

For AX, this matters because the project is trying to prove:

- canonical source surface
- structured diagnostics
- repair goals
- focused fixits and spans

Those are protocol properties.
They need a stable baseline before they can be fairly tested against live models.

## Reproduce The Current Snapshot

Prerequisite:

- a compiled `axc` binary is available, for example via:

```powershell
.\scripts\cargo-gnu.ps1 build
```

Then reproduce the exported benchmark and both comparison reports:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\export-repair-benchmark.ps1 `
  -OutputDir .ax-ai\repair-benchmark\showcase-20260424 `
  -SkipBuild

powershell -NoProfile -ExecutionPolicy Bypass -Command "& { `
  .\scripts\compare-repair-feedback.ps1 `
    -BenchmarkDir '.ax-ai\repair-benchmark\showcase-20260424' `
    -RunnerScript '.\scripts\replay-repair-adapter.ps1' `
    -RunnerExtraArgs @('-SourceDir', '.\benchmarks\repair-candidates\compare\shared', '-SourceDirBase', '.\benchmarks\repair-candidates\compare\base') `
    -OutputDir '.ax-ai\repair-comparisons\showcase-20260424' `
    -SkipBuild `
}"

powershell -NoProfile -ExecutionPolicy Bypass -Command "& { `
  .\scripts\compare-repair-modes.ps1 `
    -BenchmarkDir '.ax-ai\repair-benchmark\showcase-20260424' `
    -RunnerScript '.\scripts\replay-repair-adapter.ps1' `
    -RunnerExtraArgs @('-SourceDir', '.\benchmarks\repair-candidates\compare\shared', '-SourceDirCold', '.\benchmarks\repair-candidates\compare\cold', '-SourceDirBase', '.\benchmarks\repair-candidates\compare\base') `
    -OutputDir '.ax-ai\repair-mode-comparisons\showcase-20260424' `
    -SkipBuild `
}"
```

The resulting machine-readable reports are written to:

- `.ax-ai\repair-comparisons\showcase-20260424\comparison.json`
- `.ax-ai\repair-mode-comparisons\showcase-20260424\comparison.json`

For the underlying workflow details, see [`repair-benchmark.md`](./repair-benchmark.md).

## What This Evidence Proves

This benchmark does prove that AX already has a reproducible internal evidence chain for its repair protocol:

- the benchmark cases are fixed and versioned
- the diagnostics contract is stable enough to replay
- `--json --ai` adds measurable repair lift over base diagnostics on the same benchmark snapshot
- the lift is visible in runtime and semantic failures, not only trivial syntax cases

## What It Does Not Yet Prove

This benchmark does **not** yet prove the final external thesis:

- it does not compare AX against Rust / Go / Python subsets
- it does not yet measure live multi-model performance across providers
- it does not yet prove that AX is universally better for all coding tasks

So the current honest claim is:

> AX already has a hard, reproducible repair-evidence loop inside its own benchmark harness.

That is enough to justify the project as an engineering protocol experiment.
It is not yet the final proof that AX wins against existing language subsets.

## Next Public Proof To Add

The next benchmark step should be explicit and narrow:

1. Fix a small cross-language task set.
2. Compare AX against constrained Rust / Go / Python subsets.
3. Hold model, retry budget, and tool access constant.
4. Measure pass@1, single-round repair rate, token cost, and output stability.
