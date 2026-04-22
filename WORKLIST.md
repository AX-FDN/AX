# AX Worklist

最后更新：2026-04-22

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
  - 备注：`axc fmt <file>` 已实现为原地格式化；当前覆盖已落地语法子集。
- [ ] `P0-2` 写一个真实可用的模型 adapter。
  - 目标：不再只靠 replay adapter，至少能接一个真实 `Codex` 或 `Claude Code` CLI。
- [ ] `P0-3` 引入 HIR 层，并提供稳定 HIR dump。
  - 目标：把 `AST -> 语义 -> 后端` 之间的中间表示真正落到代码里。
- [ ] `P0-4` 补 CI 与外部接口快照。
  - 目标：至少覆盖 `fmt` 幂等、diagnostics JSON、AST/HIR dump、repair benchmark smoke run。

## P1

- [ ] `P1-1` 扩大 AI 规则卡覆盖范围。
  - 目标：加入 unsupported feature guidance、更多类型错误、更多语法恢复规则。
- [ ] `P1-2` 扩大 repair benchmark 数据集。
  - 目标：从当前高频坏例子扩展到更多真实小任务和多轮样例。
- [ ] `P1-3` 做正式的模型对照实验。
  - 目标：跑出 `base diagnostics` vs `ai diagnostics` 的真实 repair lift。
- [ ] `P1-4` 稳定文档出口。
  - 目标：补 benchmark 使用说明、adapter 规范、diagnostics schema 文档。

## P2

- [ ] `P2-1` 引入 `axc build` 的最小骨架。
  - 目标：为原生后端预留真实命令入口和编译产物流程。
- [ ] `P2-2` 开始原生后端路线。
  - 目标：按计划进入 `HIR -> MIR(or lower IR) -> Native Backend`。
- [ ] `P2-3` 推进模块 / manifest 设计。
  - 目标：让 AX 从单文件原型走向最小工程化。
- [ ] `P2-4` 扩展更丰富的类型与语法能力。
  - 目标：在 benchmark 证据支持下，再讨论 `match`、数组/切片、更多类型表面。
