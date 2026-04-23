# AX Worklist

最后更新：2026-04-23

状态说明：

- `[x]` 已完成
- `[~]` 进行中
- `[ ]` 待做

维护规则：

1. 这个文件放在项目根目录，作为当前施工清单。
2. 每完成一项，立即修改本文件状态、日期和备注。
3. 优先级以 `P0 > P1 > P2` 为准；除非有新证据，否则先做高优先级。

## P0

- [x] `P0-1` 实现 `axc fmt`，给当前原型语法提供唯一官方格式，并保证对已格式化输入幂等。
  - 完成于：2026-04-22
  - 备注：`axc fmt <file>` 已实现为原地格式化；当前覆盖已落地语法子集，并已通过 Rust 测试与真实命令 smoke test。
- [x] `P0-2` 写一个真实可用的模型 adapter。
  - 目标：不再只靠 replay adapter，至少能接一个真实 `Codex` 或 `Claude Code` CLI。
  - 完成于：2026-04-22
  - 备注：新增 `scripts/codex-repair-adapter.ps1`；已完成单 case 真机修复 smoke test，并通过 `run-repair-benchmark.ps1` + `score-repair-benchmark.ps1` 的单 case 闭环验证。
- [x] `P0-3` 引入 HIR 层，并提供稳定 HIR dump。
  - 目标：把 `AST -> 语义 -> 后端` 之间的中间表示真正落到代码里。
  - 完成于：2026-04-22
  - 备注：新增 `src/hir.rs` 与 `axc hir <file>`；`for` 已 lowering 为更核心的 HIR 控制流，解释器已改为直接执行 HIR，并通过测试与真实命令 smoke test。
- [x] `P0-4` 补 CI 与外部接口快照。
  - 目标：至少覆盖 `fmt` 幂等、diagnostics JSON、AST/HIR dump、repair benchmark smoke run。
  - 完成于：2026-04-22
  - 备注：新增 `tests/interface_snapshots.rs`、`tests/snapshots/`、`scripts/smoke-repair-benchmark.ps1` 与 `.github/workflows/ci.yml`；已通过 `cargo-gnu.ps1 test` 和 replay smoke benchmark 验证。

## P1

- [x] `P1-1` 扩大 AI 规则卡覆盖范围。
  - 目标：加入 unsupported feature guidance、更多类型错误、更多语法恢复规则。
  - 完成于：2026-04-22
  - 备注：已补齐首批 lexer/parser/semantic 规则卡扩展，并接入 `import` / `module` / `match` / 数组相关 guidance；当前固定长度数组与元素赋值已支持，空数组字面量仍保留为稳定 unsupported guidance。
- [x] `P1-2` 扩大 repair benchmark 数据集。
  - 目标：从当前高频坏例子扩展到更多真实小任务和多轮样例。
  - 完成于：2026-04-22
  - 备注：已把完整 repair case 扩到 17 个，并把 CI smoke replay 扩到 5 个稳定 case，覆盖 unknown type、missing struct field 和 unsupported import。
- [x] `P1-3` 做正式的模型对照实验。
  - 目标：跑出 `base diagnostics` vs `ai diagnostics` 的真实 repair lift。
  - 完成于：2026-04-22
  - 备注：新增 `scripts/compare-repair-feedback.ps1`，可导出 `comparison.json` / `comparison.md`，并已验证 0 lift 与正 lift 两种场景。
- [x] `P1-4` 稳定文档出口。
  - 目标：补 benchmark 使用说明、adapter 规范、diagnostics schema 文档。
  - 完成于：2026-04-22
  - 备注：已新增 `docs/README.md`、repair benchmark 指南、repair adapter 规范和 diagnostics schema 文档。

## P2

- [x] `P2-1` 引入 `axc build` 的最小骨架。
  - 目标：为原生后端预留真实命令入口和编译产物流程。
  - 完成于：2026-04-22
  - 备注：已新增 `axc build <file> [--out-dir <path>]`，当前会产出 `source.ax`、`program.hir.json` 与 `build-manifest.json`，并通过接口快照测试固定第一版输出形状。
- [x] `P2-2` 开始原生后端路线。
  - 完成于：2026-04-22
  - 备注：新增 `src/mir.rs` 与 `axc mir <file>`；当前已打通 `HIR -> MIR` lowering，MIR 会输出基本块 CFG 与 resolved locals，`axc build` 也开始产出 `program.mir.json` 并纳入接口快照。
  - 目标：按计划进入 `HIR -> MIR(or lower IR) -> Native Backend`。
- [x] `P2-3` 推进模块 / manifest 设计。
  - 完成于：2026-04-22
  - 备注：已接入最小项目 manifest `AX.toml`，当前支持单包单入口工程目录；`axc check/run/build/fmt/ast/hir/mir` 都可直接接受项目目录或 manifest 路径，`build` 也会复制项目 manifest 并固定接口快照。
- [~] `P2-4` 扩展更丰富的类型与语法能力。
  - 目标：在 benchmark 证据支持下，继续推进 `match`、切片、更多类型表面。
  - 进展：已完成第一批固定长度数组支持：`[Type; N]`、数组字面量、索引读取、数组元素赋值、HIR/MIR/解释器贯通；切片与空数组字面量推导待后续轮次。
