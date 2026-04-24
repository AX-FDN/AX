# AX Docs

This directory holds the stable external documentation for the current AX prototype.

- [`why-not-language-subsets.md`](./why-not-language-subsets.md)
  Explains why AX only makes sense if canonical syntax, structured diagnostics, repair contract, and benchmark evidence all land together.
- [`benchmark-showcase.md`](./benchmark-showcase.md)
  Summarizes the current verified AX benchmark evidence and separates reproduced internal results from next public comparison targets.
- [`killer-demo.md`](./killer-demo.md)
  Gives a short external-facing demo sequence for showing AX's repair contract and tool-script direction.
- [`quickstart.md`](./quickstart.md)
  Covers the current source-install path and a minimal sanity-check sequence for new users.
- [`host-runtime-boundary.md`](./host-runtime-boundary.md)
  Explains the current boundary between Rust host primitives, AX-facing library interfaces, project libraries, and future package-system expectations.
- [`repair-benchmark.md`](./repair-benchmark.md)
  Explains the benchmark manifests, export pipeline, runner flow, scoring, and comparison workflow.
- [`repair-adapter-spec.md`](./repair-adapter-spec.md)
  Defines the runner script contract used by `run-repair-benchmark.ps1`, `compare-repair-feedback.ps1`, and `compare-repair-modes.ps1`.
- [`diagnostics-schema.md`](./diagnostics-schema.md)
  Documents the stable JSON shape of `axc check --json`, `axc run --json`, and the optional AI extension used by `--json --ai`.
- [`diagnostics-benchmark-schema.md`](./diagnostics-benchmark-schema.md)
  Documents the stable `summary.json` shape emitted by `benchmark-diagnostics.ps1`.

Real workload examples currently live in [`../examples/`](../examples/):

- [`../examples/workspace_audit.ax`](../examples/workspace_audit.ax)
  Workspace audit report with directory, text, and action-item counts.
- [`../examples/docs_release_snapshot.ax`](../examples/docs_release_snapshot.ax)
  Docs snapshot and receipt generation workflow.
- [`../examples/workspace_search_report.ax`](../examples/workspace_search_report.ax)
  Keyword search report over a workspace slice.
- [`../examples/project_split/`](../examples/project_split/)
  Minimal multi-file AX project using `AX.toml` `sources = [...]` support.
- [`../examples/project_foundation_report/`](../examples/project_foundation_report/)
  AX-side helper files under `lib/` plus a real reporting entrypoint.
- [`../examples/project_docs_release/`](../examples/project_docs_release/)
  Multi-file AX project for docs snapshot and receipt generation.
- [`../examples/project_workspace_audit/`](../examples/project_workspace_audit/)
  Multi-file AX project for workspace auditing, with AX-side helper files for file typing, text stats, totals, report rendering, and file-level auditing.
- [`../examples/project_workspace_search_report/`](../examples/project_workspace_search_report/)
  Multi-file AX project for workspace search reporting, with AX-side helper files for searchable file selection, line matching, totals, report rendering, and file-level search aggregation.

Read [`../README.md`](../README.md) for the AX design statement and project entry, [`../详细介绍.md`](../详细介绍.md) for practical commands and benchmark workflow, [`../PLAN.md`](../PLAN.md) for roadmap and project policy, and [`../SYNTAX.md`](../SYNTAX.md) for the current prototype grammar.
