# AX Worklist

最后更新：2026-04-23

状态说明：

- `[x]` 已完成
- `[~]` 进行中
- `[ ]` 待做

维护规则：

1. 这个文件放在项目根目录，作为当前施工清单。
2. 每完成一项，立即修改本文件状态、日期和备注，并同步检查 [`规划.md`](C:/Users/xiaoy/Desktop/A语言/AX/规划.md) 是否需要更新当前阶段、当前顺序或优先级判断。
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
- [x] `P0-5` 加固 AI session 版本策略与 AI diagnostics 快照。
  - 目标：固定 `axc check --json --ai` 的接口形状，并让 `--ai-session` 对不兼容版本显式失败。
  - 完成于：2026-04-23
  - 备注：已补 `check --json --ai` 接口快照、unsupported session version CLI 测试、session schema version 单元测试、首批高价值错误码映射测试，并同步更新 diagnostics schema 文档。
- [x] `P0-6` 打通 `axc run --json` 运行期结构化诊断。
  - 目标：让 `run` 在前端失败和解释器运行失败时都能输出结构化 diagnostics，而不只输出文本。
  - 完成于：2026-04-23
  - 备注：已为 `axc run` 接入 `--json / --ai / --ai-session` 参数解析，补齐运行期越界错误的基础 / AI 接口快照与 CLI 测试，并同步更新 README 与 diagnostics schema 文档。
- [x] `P0-7` 收口除法执行一致性与除零 AI 规则。
  - 目标：修复 `/` 词法断层，并把运行期 `R0021` 纳入稳定 AI 反馈覆盖。
  - 完成于：2026-04-23
  - 备注：已补 `/` 词法回归测试、解释器整除回归测试、`division_by_zero_must_be_avoided` 规则、CLI AI 快照测试与 `examples/division_by_zero.ax` 示例。
- [x] `P0-8` 细化高频类型诊断的专门 AI 规则。
  - 目标：把“条件必须是 `bool`”和“数组索引必须是 `i32`”从泛型 `type_match_required` 中拆出来，给更具体的修复 guidance。
  - 完成于：2026-04-23
  - 备注：已新增 `condition_expression_must_be_bool` 与 `array_index_must_be_i32`，补齐单元测试、稳定 `rule_id` 覆盖、CLI 快照，以及 `examples/non_bool_condition.ax`、`examples/array_index_type_mismatch.ax` 示例。
- [x] `P0-9` 继续拆分高频 `S0022` 类型错位。
  - 目标：把“函数参数类型不匹配”和“return 类型不匹配”从泛型 `type_match_required` 中拆出来，让 AI 反馈更接近真实修复动作。
  - 完成于：2026-04-23
  - 备注：已新增 `function_argument_type_must_match` 与 `return_value_must_match_declared_type`，补齐单元测试、稳定 `rule_id` 覆盖、函数参数类型错的 CLI 快照，以及 `examples/function_argument_type_mismatch.ax`、`examples/return_type_mismatch.ax` 示例。
- [x] `P0-10` 固定 compare benchmark 的 smoke 回归。
  - 目标：让 `base diagnostics` vs `ai diagnostics` 的 `comparison.json` 不再只是一次性产物，而是有稳定回归保护的外部契约。
  - 完成于：2026-04-23
  - 备注：已扩展 `scripts/replay-repair-adapter.ps1` 支持 `-SourceDirBase / -SourceDirAi` 覆盖目录；新增 `scripts/smoke-compare-repair-feedback.ps1`、`benchmarks/repair-candidates/compare/base/` 稳定样本，并将 compare smoke 接入 CI 与 benchmark 文档。
- [x] `P0-11` 固定三模式 benchmark 报告。
  - 目标：把 `cold / base / ai` 三层反馈做成稳定、可比较、可回归的固定报告，而不是只停留在两模式对比。
  - 完成于：2026-04-23
  - 备注：已为 export / run / replay adapter 接入 `cold` 模式，新增 `scripts/compare-repair-modes.ps1`、`scripts/smoke-compare-repair-modes.ps1` 与 `benchmarks/repair-candidates/compare/cold/` 稳定样本，并将三模式 smoke 接入 CI 与 benchmark 文档。
- [x] `P0-12` 启动 `semantic.rs` 拆层的第一步。
  - 目标：先把语义层里的类型定义、顶层收集器和诊断 helper 拆出去，减少 `semantic.rs` 的“大厨房”压力，同时保持外部行为不变。
  - 完成于：2026-04-23
  - 备注：已新增 `src/semantic/types.rs`、`src/semantic/program_info.rs`、`src/semantic/helpers.rs`，让 `semantic.rs` 开始退回到“入口 + checker + tests”；`cargo-gnu.ps1 test` 已通过。
- [x] `P0-13` 完成 `semantic.rs` 拆层的第二步。
  - 目标：把 `TypeChecker` 主体和局部绑定逻辑搬进独立模块，让 `semantic.rs` 进一步收口为“入口编排 + 测试”。
  - 完成于：2026-04-23
  - 备注：已新增 `src/semantic/checker.rs`，并把 `TypeChecker` / `Binding` / 检查规则实现整体迁出；`semantic.rs` 现在只保留模块装配、`check_program` 与测试；`cargo-gnu.ps1 test` 已通过。
- [x] `P0-14` 继续细化 `checker` 的职责边界。
  - 目标：先把赋值目标检查从 `checker.rs` 主体里拆出去，给后续继续拆 `resolver / type checker / assignment rules` 打出稳定边界。
  - 完成于：2026-04-23
  - 备注：已新增 `src/semantic/checker/assignment.rs`，迁出变量赋值、结构体字段赋值、数组元素赋值与非法赋值目标诊断；`cargo-gnu.ps1 test` 已通过。
- [x] `P0-15` 继续拆名字解析与作用域层。
  - 目标：把绑定声明、作用域查找、可见变量提示和未定义变量诊断从 `checker.rs` 主体里拆出去，继续收紧 `resolver / type checker` 边界。
  - 完成于：2026-04-23
  - 备注：已新增 `src/semantic/checker/names.rs`，迁出 `Binding`、`declare`、`lookup`、未定义变量诊断与作用域可见性辅助逻辑；`cargo-gnu.ps1 test` 已通过。
- [x] `P0-16` 继续拆表达式类型检查层。
  - 目标：把 `check_expr` 与表达式级类型/调用/字面量校验从 `checker.rs` 主体里拆出去，继续把 `TypeChecker` 主干收口到更清晰的语句与控制流边界。
  - 完成于：2026-04-23
  - 备注：已新增 `src/semantic/checker/expr.rs`，迁出表达式类型检查与相关诊断逻辑；`checker.rs` 现在主要保留语句分发、`for`/块控制流检查和共享的 `expect_type_match`；`cargo-gnu.ps1 test` 已通过。
- [x] `P0-17` 继续拆控制流语义规则。
  - 目标：把 `return / if / while / for` 的语义检查从 `checker.rs` 主体里拆出去，让后续控制流返回分析和语句分发继续解耦。
  - 完成于：2026-04-23
  - 备注：已新增 `src/semantic/checker/control_flow.rs`，迁出返回值校验、条件为 `bool` 的检查、`for` header 校验与块级控制流入口；`checker.rs` 进一步收口为语句分发与共享类型匹配辅助；`cargo-gnu.ps1 test` 已通过。
- [x] `P0-18` 继续压缩 `checker.rs` 的共享调度层。
  - 目标：把语句分发与共享类型匹配从 `checker.rs` 再拆出去，让 `checker.rs` 基本只保留 `TypeChecker` 结构、初始化与 block 级编排。
  - 完成于：2026-04-23
  - 备注：已新增 `src/semantic/checker/statements.rs` 与 `src/semantic/checker/type_rules.rs`，迁出 `check_statement` 与 `expect_type_match`；`checker.rs` 现在主要承担模块装配、`TypeChecker` 定义与 `check_block` 编排；`cargo-gnu.ps1 test` 已通过。
- [x] `P0-19` 继续拆返回路径分析层。
  - 目标：把缺少 `return` 的控制流分析与 `S0023` 诊断构造从 `semantic.rs` 和通用 helper 里拆出去，让语义入口进一步只负责编排。
  - 完成于：2026-04-23
  - 备注：已新增 `src/semantic/return_analysis.rs`，迁出返回路径判断、缺少 `return` 的 note/suggestion 与 `S0023` 诊断构造；`semantic.rs` 现在直接复用 `missing_return_diagnostic(...)`；`cargo-gnu.ps1 test` 已通过。

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
