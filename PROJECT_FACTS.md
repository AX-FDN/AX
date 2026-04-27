# AX Project Facts

> 阅读提示：本文件是 AX 的“当前事实清单”，给外部读者、协作者和 AI 工具快速建立同一套项目认知用。
> 它不是路线图，不替代 [`PLAN.md`](./PLAN.md)、[`WORKLIST.md`](./WORKLIST.md) 或 [`ARCHIVE.md`](./ARCHIVE.md)。

## 一句话定义

AX 是一门面向自回归 Coding AI 的 AI-first 工具语言，目前处在从可运行原型向成熟工具语言推进的阶段。
它把 canonical syntax、structured diagnostics、repair contract、context protocol 和 benchmark evidence 放进同一套语言与编译器工具链里，目标是让模型在生成、修复和项目理解三个环节都更稳定。

## 当前基线快照

| 维度 | 当前状态 |
| --- | --- |
| CLI | `axc check / ast / hir / mir / build / run / fmt / context` 已进入主线 |
| 编译链 | `Lexer -> Parser -> AST -> HIR -> MIR -> Semantic` 已打通 |
| 执行链 | 解释执行是当前稳定主路径 |
| 诊断协议 | 文本、`--json`、`--json --ai` 三层输出已成立 |
| 修复协议 | `rule_id / repair_goal / fixits / context_snippets` 已进入稳定输出 |
| 上下文协议 | `overview / boundaries / topology / flow / symbol / impact / evidence` 七个视图已进入主线 |
| 项目模式 | `AX.toml + sources` 与最小 `module/import` 已落地 |
| 代表样例 | 已固定 P2 主代表样例与宿主边界样例，并接入 `check / run / build` 回归 |
| benchmark | export / run / score / compare / smoke / CI 已落地，context-enabled export 已进入修复输入链 |
| build | 当前仍是 skeleton build，不是成熟 AOT/native backend |
| 平台 | Windows 为 full workflow，Linux 为 core support，macOS 尚未启动 |

## 当前语法完成度判断

| 层级 | 完成度 | 说明 |
| --- | --- | --- |
| 最小可写工具内核 | `75%~80%` | 已具备显式类型、数组/切片、`for/for in`、`break/continue`、最小 `match`、payload enum、模块第一刀、宿主 builtin |
| 通用语言表面 | `35%~40%` | 还缺泛型、`impl`、traits/interfaces、可见性、包系统、闭包、async 等 |
| 生态支撑语法 | `20%~30%` | 已能组织项目，但还不能支撑完整标准库与第三方包生态 |

## AX 现在已经成立的事实

### 1. 编译器主链已经打通

- 词法、语法、AST、HIR、MIR、语义检查、解释执行都已进入同一仓库主线
- `axc check`、`axc run`、`axc fmt`、`axc ast`、`axc hir`、`axc mir`、`axc build` 都已有稳定入口

### 2. 结构化诊断已经是编译器主能力的一部分

- 基础层：文本诊断、`--json`
- AI 增强层：`--json --ai`
- 当前 AI 增强输出已经包含稳定 `rule_id`、修复目标、fixits 和上下文切片

### 3. runtime 也开始进入 AI 修复链

- `axc run --json --ai` 已覆盖首批高价值运行时误用
- 当前已进入稳定协议的 runtime 家族包括数组越界、除零、部分文件/目录/进程/环境变量/argv 误用

### 4. 多文件项目不是概念，而是已在跑的路径

- 项目组织采用 `AX.toml + sources`
- 第一阶段 `import / module` 已接入 parser、project、semantic 与诊断主链
- 仓库内已有 project-backed 代表样例和共享 `foundation/` helper
- P2 阶段主代表样例与宿主边界样例已在 [`docs/representative-samples.md`](./docs/representative-samples.md) 固定

### 5. benchmark 证据链是语言主线的验证层，不是附属脚本

- 仓库内已有 repair cases、adapter spec、导出、评分、对比、smoke 与 CI
- AX 的主张是“把语言本体和 AI 友好编译器一起做硬”，所以 benchmark 是继续条件的一部分
- 当前 full manifest 有 `30` 个 repair case，仓库内 deterministic replay 当前可复现 `cold 23/30`、`base 25/30`、`ai 30/30`
- 公开展示页见 [`docs/benchmark-showcase.md`](./docs/benchmark-showcase.md)

### 6. context 协议已经是对外接口，不是草图

- `overview / boundaries / topology / flow / symbol / impact / evidence` 都已经有命令和 JSON
- `export-repair-benchmark.ps1 -IncludeContext` 已能把 `overview / boundaries / evidence` 写入 repair bundle 与 prompt
- 当前欠缺的不是“有没有视图”或“能不能进入输入链”，而是 context-enabled benchmark 还需要继续做 live-model A/B 与公开对照结果

## 当前仍在硬化、还不应被误读成完成态的部分

- `import / module`：第一刀已落地，但还在补更多接口回归和边界覆盖
- host runtime boundary：已开始收紧，但仍是当前最重要的持续硬化方向之一
- `src/ai.rs` 的规则触发：正在从文案匹配继续迁移到更稳定的内部语义标签
- `build`：当前仍是 backend 前的构建骨架，不应被表述成成熟 native compiler
- `foundation/`：当前是 Std-0 孵化层，还不是冻结后的官方标准库

## 当前明确后置、不是主线的方向

- 完整 native backend
- 自举与 AX 重写编译器核心
- FFI 与包管理系统
- 成熟 IDE / 调试器生态
- async、宏系统、复杂泛型、庞大标准库
- `AX import -> Cargo crate` 直接桥接

## 当前最该怎么评价 AX

不要只盯着“它有没有泛型 / FFI / JIT / 包管理”，更应该先问：

1. 它是否已经具备一条稳定向前推进的语言内核和工程组织路径？
2. 它是否已经形成稳定的 `check/json/ai` 修复协议？
3. 它是否能让真实坏例子进入可回放、可评分、可比较的 benchmark？
4. 它是否已经能承载一批真实工具风格样例，而不是只会跑玩具例子？
5. 它的 host boundary 和 context 协议是否在持续收紧并进入修复闭环？

## 推荐阅读顺序

1. [`README.md`](./README.md)：项目介绍与入口
2. [`docs/feature-matrix.md`](./docs/feature-matrix.md)：当前能力面、边界和非目标
3. [`docs/benchmark-showcase.md`](./docs/benchmark-showcase.md)：当前可复现 benchmark 展示页
4. [`docs/public-claims.md`](./docs/public-claims.md)：对外表述边界
5. [`docs/repair-benchmark.md`](./docs/repair-benchmark.md)：benchmark 证据链
6. [`PLAN.md`](./PLAN.md)：全项目闭环计划与阶段门槛
7. [`WORKLIST.md`](./WORKLIST.md)：当前施工项
8. [`ARCHIVE.md`](./ARCHIVE.md)：已完成事项归档

## 当前一句话判断

AX 现在最值得关注的，是它能否一边把语言内核继续推进到成熟工具语言的方向，一边把“结构化诊断 + AI 修复协议 + 上下文协议 + benchmark 证据链”做成比普通语言项目更硬的编译器护城河。
