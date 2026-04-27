# AX Sharp Demo

This is the short external-facing demo for AX.
It is intentionally not a generic `hello world`.

The sharp version of the pitch is:

> keep the same bad example, keep the same single-round repair budget, keep the same model, and change only the feedback contract

If AX is worth attention, the repair chain should become more stable.

## Demo Target

Use one concrete failure:

- case id: `slice_assignment_read_only`
- source: [`../examples/slice_assignment.ax`](../examples/slice_assignment.ax)
- single-case manifest: [`../benchmarks/repair-cases-demo.json`](../benchmarks/repair-cases-demo.json)

Broken source:

```ax
fn main() -> i32 {
    let values: [i32; 3] = [1, 2, 3];
    let mut view: [i32] = values[0:2];
    view[0] = 9;
    return 0;
}
```

Why this case is good:

- it is short enough to understand in seconds
- it is not just punctuation
- the AX repair contract says something specific: slices are read-only views

## Demo Path

Use this order:

1. show the broken file
2. show base vs AI-enhanced diagnostics on the exact same file
3. run a deterministic one-case compare
4. optionally run the same one-case compare with a live model
5. close with one real tool-style AX script

## Step 1: Same File, Better Repair Payload

Base structured diagnostics:

```powershell
.\target\debug\axc.exe check examples\slice_assignment.ax --json
```

AI-enhanced structured diagnostics:

```powershell
.\target\debug\axc.exe check examples\slice_assignment.ax --json --ai
```

What to point out:

| Base diagnostics | AI-enhanced diagnostics |
| --- | --- |
| `code: "S0035"` | same stable code |
| readable error message | same message plus `rule_id` |
| one suggestion | explicit `repair_goal`, `focus_item`, `relevant_spans`, `rule_card`, `fixits` |

That is the core AX pitch in one screen:

- not only "the program is wrong"
- but also "what kind of wrong this is"
- and "what the agent should repair without wandering"

## Step 2: Reproducible One-Case Compare

This is the deterministic version.
It uses the committed replay assets on exactly the same single-case manifest.

Export the one-case benchmark:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\export-repair-benchmark.ps1 `
  -ManifestPath .\benchmarks\repair-cases-demo.json `
  -OutputDir .ax-ai\repair-benchmark\demo-sharp `
  -SkipBuild
```

Compare `base` vs `ai` on that one case:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "& { `
  .\scripts\compare-repair-feedback.ps1 `
    -BenchmarkDir '.ax-ai\repair-benchmark\demo-sharp' `
    -RunnerScript '.\scripts\replay-repair-adapter.ps1' `
    -RunnerExtraArgs @('-SourceDir', '.\benchmarks\repair-candidates\compare\shared', '-SourceDirBase', '.\benchmarks\repair-candidates\compare\base') `
    -OutputDir '.ax-ai\repair-comparisons\demo-sharp' `
    -SkipBuild `
}"
```

Expected deterministic result for the current repository state:

- `base`: `0/1`
- `ai`: `1/1`

This is the cleanest "same bad example, only feedback mode changes" demo in the repo today.

## Step 3: Same Model, Same Single-Round Budget, Same Case

This is the live-model version.
Use it when you want the sharper external demo instead of the deterministic replay proof.

Prerequisites:

- Codex CLI is installed and authenticated
- `axc` is already built

Run the exact same one-case manifest through the Codex adapter:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -Command "& { `
  .\scripts\compare-repair-feedback.ps1 `
    -BenchmarkDir '.ax-ai\repair-benchmark\demo-sharp' `
    -RunnerScript '.\scripts\codex-repair-adapter.ps1' `
    -RunnerExtraArgs @('-Model', 'gpt-5.4') `
    -OutputDir '.ax-ai\repair-comparisons\demo-sharp-live' `
    -SkipBuild `
}"
```

What stays fixed here:

- same broken source
- same one-case manifest
- same single repair attempt per mode
- same runner contract
- same model

What changes:

- only the feedback bundle: `base` vs `ai`

Important honesty line:

- live-model results may vary by model version and service state
- the replay compare above is the stable reference proof that the protocol difference is encoded correctly

## Step 4: Close With A Real Tool Script

The demo should not end on diagnostics alone.
Show that AX already writes small host-facing tools.

Recommended example:

- program: [`../examples/extract_markdown_headings.ax`](../examples/extract_markdown_headings.ax)

```powershell
.\target\debug\axc.exe run examples\extract_markdown_headings.ax -- README.md target\headings-demo.txt
Get-Content target\headings-demo.txt
```

What this shows:

- AX can read a real file
- AX can do line-oriented string processing
- AX can emit a useful artifact

If you want a filesystem example instead, use [`../examples/index_directory.ax`](../examples/index_directory.ax).

## Suggested Spoken Close

Use this if you want one short spoken summary:

> AX is not trying to win by adding more syntax.
> It is trying to make source form, compiler diagnostics, and repair feedback easier for coding agents to consume.
> Here is the same bad example with the same single-round budget.
> The only thing that changed is the feedback contract.
> And here is a small AX script already doing real tool work against the filesystem and text files.

## What Not To Claim

Do not say:

- AX has already beaten existing languages in public cross-language benchmarks
- AX is a finished general-purpose language
- AX is optimized for a hidden tokenizer

Do say:

- AX is an AI-first tool language designed for coding-agent generation, repair, and project understanding
- AX owns its source form, structured diagnostics, context protocol, and repair feedback as one compiler surface
- AX already has a structured repair contract
- AX already has a reproducible internal benchmark loop
- AX can already express small tool-style workloads
