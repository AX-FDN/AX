# AX Docs

This directory holds the stable external documentation for the current AX prototype.

- [`repair-benchmark.md`](./repair-benchmark.md)
  Explains the benchmark manifests, export pipeline, runner flow, scoring, and comparison workflow.
- [`repair-adapter-spec.md`](./repair-adapter-spec.md)
  Defines the runner script contract used by `run-repair-benchmark.ps1` and `compare-repair-feedback.ps1`.
- [`diagnostics-schema.md`](./diagnostics-schema.md)
  Documents the stable JSON shape of `axc check --json`, `axc run --json`, and the optional AI extension used by `--json --ai`.

Read [`../README.md`](../README.md) for the AX design statement and project entry, [`../详细介绍.md`](../详细介绍.md) for practical commands and benchmark workflow, [`../PLAN.md`](../PLAN.md) for roadmap and project policy, and [`../SYNTAX.md`](../SYNTAX.md) for the current prototype grammar.
