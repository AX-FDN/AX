# AX Validation Matrix

> 本文件回答一个问题：当前本机和 CI 分别应该跑什么，哪些验证属于全量 Windows workflow，哪些只属于跨平台 core support。

AX 当前不是所有平台跑完全相同的验证链。仓库采用分层验证：

- Windows 本机：开发者可复跑的 full workflow 主路径。
- Windows CI：全量回归与 PowerShell benchmark/orchestration 主路径。
- Ubuntu CI：Linux core compiler/runtime 主路径。

## Matrix

| 环境 | 定位 | 必跑命令 | 不承诺内容 |
| --- | --- | --- | --- |
| Windows local | 当前主开发与本机复跑路径 | `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 build` | 不要求 plain `cargo test` 在缺 MSVC `link.exe` 的 shell 中可用 |
| Windows local | Rust unit tests | `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --lib` | 不把 MSVC linker 路径作为默认本机基线 |
| Windows local | CLI/interface regression | `powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots` | 不要求每次都跑完整 benchmark compare，除非改到 scripts/benchmark/diagnostics/context |
| Windows CI | Full workflow support | `cargo +stable fmt --check` | 无 |
| Windows CI | Full test suite | `.\scripts\cargo-gnu.ps1 test` | 无 |
| Windows CI | Diagnostics benchmark smoke | `.\scripts\smoke-benchmark-diagnostics.ps1` | 无 |
| Windows CI | Repair benchmark smoke | `.\scripts\smoke-repair-benchmark.ps1` | 无 |
| Windows CI | Repair compare smoke | `.\scripts\smoke-compare-repair-feedback.ps1` | 无 |
| Windows CI | Repair mode compare smoke | `.\scripts\smoke-compare-repair-modes.ps1` | 无 |
| Ubuntu CI | Linux core support | `cargo +stable fmt --check` | 不跑 PowerShell benchmark/orchestration |
| Ubuntu CI | Build | `cargo build --locked` | 不发布 Linux binary |
| Ubuntu CI | Rust unit tests | `cargo test --locked --lib` | 不跑 Windows-only script smoke |
| Ubuntu CI | Cross-platform interface tests | `cargo test --locked --test interface_snapshots` | PowerShell benchmark tests在非 Windows 上保持 ignore |
| Ubuntu CI | Core CLI smoke | `axc fmt/check/run/build` 最小链 | 不覆盖 repair/export/compare `.ps1` 链 |

## Recommended Local Paths

### Normal Documentation-Only Change

文档改动至少跑：

```powershell
git diff --check
```

如果改到了 README、docs 导航或 benchmark 数字，额外跑相关脚本或手动核对来源。

### Compiler / Runtime / Language Change

语言、语义、解释器、project、context 或 CLI 改动至少跑：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --lib
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots
```

如果改动影响 examples，还要至少跑对应 example 的 `check / run / build` 回归，或确保它已经在 `interface_snapshots` 覆盖。

### Std-1 Candidate Change

改到 `std/`、五组 Std-1 试点样例、`AX.toml sources` 或标准库边界文档时，至少跑：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots representative_project_examples_check_cleanly
```

如果只改某一组试点，可以追加对应 filter：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_text_normalize
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_directory_index
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_release_promote
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_command_capture
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_command_batch
```

如果改动会影响 `std/` source tree、build artifacts 或多个试点，跑完整 interface snapshots：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots
```

### Repair / Benchmark / Diagnostics Change

改到 `scripts/`、`benchmarks/`、`src/ai.rs`、`src/diagnostics.rs` 或 repair/context 文档时，优先跑：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots
```

如果改到 repair export、run、score、compare 链路，额外跑对应 smoke：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-repair-benchmark.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-compare-repair-feedback.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-compare-repair-modes.ps1
```

如果改到 diagnostics benchmark：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-benchmark-diagnostics.ps1
```

### Repair Archaeology Schema Change

只改 Repair Archaeology schema / docs 时，至少跑：

```powershell
git diff --check
```

如果开始新增 `export-repair-archaeology.ps1` 或修改 run / score / compare artifact 读取方式，必须补一条 smoke，至少覆盖当前 comparison artifact 里存在的高价值类别：

- 一个 `improved` case
- 一个 `both_pass` case
- 如果当前 deterministic replay 产物里存在 `failed / regressed / both_fail`，必须至少覆盖一个

该 smoke 在 v0 阶段应继续读取 deterministic replay artifact，不调用真实 LLM。

当前本地 smoke 入口：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\export-repair-archaeology.ps1 `
  -ComparisonPath .ax-ai\repair-comparisons\showcase-20260424\comparison.json `
  -OutputDir .ax-ai\repair-archaeology\local-smoke `
  -CaseIds missing_semicolon_basic,missing_paren_condition,slice_assignment_read_only
```

### Context-Enabled Repair Export

改到 `axc context` 或 `export-repair-benchmark.ps1 -IncludeContext` 时，至少跑：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots repair_benchmark_export_can_include_context_bundle
```

必要时再跑完整 smoke manifest：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\export-repair-benchmark.ps1 `
  -ManifestPath benchmarks\repair-cases-smoke.json `
  -OutputDir .ax-ai\repair-benchmark\context-smoke-local `
  -IncludeContext `
  -SkipBuild
```

## CI Contract

当前 CI 合同固定为：

- `windows-latest` 是 full workflow support。
- `ubuntu-latest` 是 core compiler/runtime support。
- PowerShell benchmark/orchestration 仍是 Windows-only。
- macOS 不在当前 CI 承诺范围。

这意味着：

- Windows CI 失败通常说明主工作流不能交付。
- Ubuntu CI 失败通常说明 core compiler/runtime 不再跨平台。
- Linux 不跑 `.ps1` benchmark 链不是缺口，而是当前阶段边界。

## When To Expand This Matrix

只有满足下面条件之一时，才扩验证矩阵：

- Linux benchmark/orchestration 开始有非 PowerShell 实现。
- macOS 进入 core support。
- AOT backend 从 skeleton 进入真实 executable output。
- 包系统或 stdlib contract 进入稳定 public interface。
- live-model benchmark 开始常驻 CI 或 nightly workflow。
