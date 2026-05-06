# AX Validation Matrix

> 本文件回答一个问题：当前本机和 CI 分别应该跑什么，哪些验证属于全量 Windows workflow，哪些只属于跨平台 core support。

AX 当前不是所有平台跑完全相同的验证链。仓库采用分层验证：

- Windows 本机：开发者可复跑的 full workflow 主路径。
- Windows CI：全量回归与 PowerShell benchmark/orchestration 主路径。
- Ubuntu CI：Linux core compiler/runtime 主路径。
- Web CI：Repair Workbench 前端构建验证路径。

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
| Windows CI | Repair archaeology smoke | `.\scripts\smoke-repair-archaeology.ps1 -SkipBuild` | 无 |
| Ubuntu CI | Linux core support | `cargo +stable fmt --check` | 不跑 PowerShell benchmark/orchestration |
| Ubuntu CI | Build | `cargo build --locked` | 不发布 Linux binary |
| Ubuntu CI | Rust unit tests | `cargo test --locked --lib` | 不跑 Windows-only script smoke |
| Ubuntu CI | Cross-platform interface tests | `cargo test --locked --test interface_snapshots` | PowerShell benchmark tests 在非 Windows 上保持 ignore |
| Ubuntu CI | LLVM AOT parity smoke | `pwsh ./scripts/smoke-aot-parity.ps1` after installing `clang` | 验证 core subset 与 project-backed AOT，不发布 Linux binary |
| Ubuntu CI | Core CLI smoke | `axc fmt/check/run/build` 最小链 | 不覆盖 repair/export/compare `.ps1` 链 |
| Web CI | Repair Workbench frontend | `cd web && npm ci && npm run build` | 不验证 Rust 编译器、benchmark 脚本或部署发布 |

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

### AOT Pivot / LLVM AOT v0 Change

改到 `src/backend/llvm/*`、`src/build/*`、`src/context/evidence.rs`、AOT 样例或 build manifest 契约时，至少跑：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --lib build::
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --lib backend::llvm
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots build_snapshots
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots context_
```

Package-backed AOT readiness changes should also run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-package-registry-aot.ps1
```

This smoke does not require native executable linking. It checks registry
package maturity behavior through `build-manifest.json`: stable pure-AX package
fixtures must not produce `AOT0104` or `AOT0105`, host-boundary fixtures must
produce `AOT0104`, and future-native fixtures must produce `AOT0105`.

Package-backed native linking changes should run the focused stable package
parity smoke:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-package-registry-native-parity.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-aot-package-generics.ps1
```

These smokes require clang. On Windows, `axc build` can also use the Rust
`x86_64-pc-windows-gnu` self-contained runtime libraries as a fallback when the
default clang/MSVC link path is missing system libraries.

Bytes ABI/readiness changes should run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-bytes-runtime.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-bytes-native-parity.ps1
```

The runtime smoke does not require clang. It verifies interpreter byte-buffer
behavior and the `bytes_runtime` readiness feature while requiring LLVM IR
generation to stay available for the current native bytes helper path. The
native parity smoke requires clang and compares `axc run` with the linked
native executable for the same `std.bytes` source.

Host/network ABI readiness changes should run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-host-network-runtime.ps1
```

This smoke does not require clang. It verifies local TCP-backed interpreter
behavior for `std.http`/`std.net` and the `AOT0301`/`runtime_abi` blocker
contract while native host handles are still pending.

如果本机或 CI 已安装 clang，优先跑 run vs AOT executable parity smoke：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-aot-parity.ps1
```

这条脚本会对 123 个 AOT 样例比较解释器和 native executable 的 `exit code / stdout / stderr`，完整清单由 `scripts/smoke-aot-parity.ps1` 维护。当前包含 97 个单文件/直接样例和 26 个 `AX.toml` project 样例，仓库内全部 project 示例都已经进入默认清单；覆盖 core/control-flow/consts/f32-core/stdout/string/string-runtime/string-predicate/string-replace/string-split-lines/string-trim/string-list-runtime/std-collections/std-env/std-fs/std-path/std-process/string-pattern/argv/fixed-array-read-write-format-equality/zero-length-array/for-in-readonly-slice/runtime-string-slice-for-in/slice-range-read/slice-range-for-in/slice-formatter-equality/struct-read-write-format-equality/struct-pattern/enum-unit/payload-enum/payload-enum-equality/enum-formatter/enum-print/enum-complex-payload-formatter/enum-array-slice-payload-equality/concrete-generic-enum-print/expression-match/range-pattern/or-pattern/match-guard/concrete-Result-Option/result-static-constructors/result-try/project-backed/local-path-package。如果只想验证单个 executable 链接与退出码，仍可跑 `.\scripts\smoke-aot-link.ps1`。没有 clang 的本机不要求这两条通过；必须保证默认 IR-only 路径和缺 clang 的 `AOT1001` blocker 路径稳定。

改到 AOT runtime error ABI 或 native guard 时，追加跑：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-aot-runtime-errors.ps1
```

这条脚本专门验证 runtime error，不要求 stderr 与解释器逐字相同，只要求 native executable 构建成功、退出码为 `1`，并且 stderr 以对应 AX 错误码开头；当前覆盖 `R0012 / R0018 / R0019 / R0020 / R0021 / R0022 / R0024 / R0031 / R0032 / R0053`。

### Std-1 Candidate Change

改到 `std/`、十三组 Std-1 试点/压力样例、`AX.toml sources`、本地 path package worker 样例或标准库边界文档时，至少跑：

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
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_option_result
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_env_result
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_file_result
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_process_result
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_result_pipeline
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_config_validate
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_collections_report
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_job_runner
```

改到 package resolver JSON diagnostics、`PX****` repair hints 或 package repair benchmark case 时，至少追加：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_path_package_manifest_errors_have_json_ai_diagnostics
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-repair-benchmark.ps1 -SkipBuild
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

当前 smoke 入口，已接入 Windows CI full workflow：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-repair-archaeology.ps1 -SkipBuild
```

这个 smoke 会重新导出 smoke benchmark、重新跑 deterministic `base -> ai` compare，再导出 `3` 个 Repair Archaeology case 报告，因此不依赖旧 `.ax-ai` 历史产物。

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
- `web` job 是独立 frontend build support。
- PowerShell benchmark/orchestration 仍是 Windows-only。
- macOS 不在当前 CI 承诺范围。

这意味着：

- Windows CI 失败通常说明主工作流不能交付。
- Ubuntu CI 失败通常说明 core compiler/runtime 不再跨平台。
- Web CI 失败通常说明 `web/` Repair Workbench 不能构建，不代表 Rust compiler/runtime 失败。
- Linux 不跑 `.ps1` benchmark / Repair Archaeology 链不是缺口，而是当前阶段边界。

## When To Expand This Matrix

只有满足下面条件之一时，才扩验证矩阵：

- Linux benchmark/orchestration 开始有非 PowerShell 实现。
- macOS 进入 core support。
- AOT backend 从 LLVM IR prototype 进入更宽的真实 executable output。
- 包系统或 stdlib contract 进入稳定 public interface。
- live-model benchmark 开始常驻 CI 或 nightly workflow。
