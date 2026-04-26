# AX Feature Matrix

> 阅读提示：本文件只回答“AX 当前到底做到哪了”。  
> 它不是愿景文档，也不是路线图；未来方向请看 [`../PLAN.md`](../PLAN.md) 和 [`../规划.md`](../规划.md)。

状态说明：

- `[x]` 当前主链已成立
- `[~]` 已有第一版，但仍在补边界或回归
- `[ ]` 明确后置，当前不是主线

## 总览

| 领域 | 状态 | 当前结论 |
| --- | --- | --- |
| 前端主链 | `[x]` | Lexer / Parser / AST / HIR / MIR / Semantic 已进入主线 |
| 执行路径 | `[x]` | 解释执行是当前稳定主路径 |
| 结构化诊断 | `[x]` | 文本、`--json`、`--json --ai` 三层输出已成立 |
| AI 修复协议 | `[x]` | `rule_id / repair_goal / fixits / context_snippets` 已进入输出层 |
| runtime AI 反馈 | `[~]` | 首批高价值 runtime 误用已接入，仍在扩完整覆盖 |
| 多文件项目 | `[x]` | `AX.toml + sources` 已是当前项目组织主路径 |
| `import / module` | `[~]` | 第一刀已接入 parser/project/semantic/diagnostics，仍在补边界 |
| 共享 AX 基础层 | `[~]` | `foundation/` 已沉淀第一批 helper，但还不是完整标准库 |
| benchmark 证据链 | `[x]` | repair/export/score/compare/smoke/CI 已进入仓库主线 |
| 原生后端 | `[ ]` | `build` 当前仍是骨架，不是成熟 native backend |
| 自举 | `[ ]` | 长期方向，不是当前主线 |
| FFI / 包管理 / IDE | `[ ]` | 当前未进入主线实现 |

## 详细矩阵

| 领域 | 状态 | 当前已支持 | 当前边界 / 不应误读成什么 | 代表入口 |
| --- | --- | --- | --- | --- |
| 编译器前端 | `[x]` | 词法、语法、AST、HIR、MIR、语义检查 | 已是工程主链，不是 parser demo | `src/lexer.rs` `src/parser.rs` `src/semantic/` |
| 执行模型 | `[x]` | 解释执行、基础运行期错误、部分 host builtin | 当前稳定路径是 interpreter，不是 AOT/JIT | `src/interpreter.rs` |
| 诊断协议 | `[x]` | 文本诊断、`--json`、`--json --ai` | AI 增强层是增量，不应污染基础层 | `docs/diagnostics-schema.md` |
| AI 修复上下文 | `[x]` | `rule_id`、`repair_goal`、`fixits`、`context_snippets`、teaching level | 当前目标是稳定 contract，不是供应商定制 prompt 仓库 | `src/ai.rs` |
| parser 高频错误稳定化 | `[x]` | 缺分号、缺括号、缺类型名、缺表达式等已接稳定 kind | 仍有少量 heuristic，但主方向已改为内部标签优先 | `src/parser.rs` |
| semantic 高频错误稳定化 | `[x]` | 模块误用与首批高价值 `S0022` 变体已接稳定 kind | 还会继续扩，但主框架已成立 | `src/diagnostics.rs` `src/ai.rs` |
| runtime 高频错误稳定化 | `[~]` | 数组越界、除零、可读文件/目录、argv/env/process 一批误用已接 AI 规则 | runtime 还在持续硬化，不代表 host boundary 已完全收口 | `src/interpreter.rs` `src/ai.rs` |
| 语言表面 | `[~]` | 基础函数、显式类型、数组、slice、struct、enum、for、module/import 第一刀 | 不含复杂泛型、trait、async、异常、宏 | `SYNTAX.md` |
| 项目组织 | `[x]` | `AX.toml + sources`、project-backed 样例、共享 `foundation/` | 当前是最小工程模型，不是成熟包系统 | `examples/project_*/` |
| 模块系统 | `[~]` | support source 模块路径、重复模块 / import、缺 import 等诊断已存在 | 当前是 minimal module mode，不是完整 package/visibility 系统 | `docs/import-module-minimal-design.md` |
| 共享基础层 | `[~]` | `foundation/cli.ax`、`report.ax`、`search.ax`、`workspace.ax` 等 | 这是第一批 AX helper，不是完整标准库 | `foundation/` |
| benchmark 方法 | `[x]` | repair case、导出、评分、对比、smoke、CI | 这不是“以后再补”的附件，而是当前主线的一部分 | `docs/repair-benchmark.md` |
| 对外平台支持 | `[~]` | Windows 路径已较完整，Linux 有 quickstart 与核心链路说明 | 仍应按文档与 CI 事实表述，不宜夸成“全平台成熟” | `docs/platform-support.md` |
| `build` | `[~]` | 可导出构建骨架产物 | 当前不是成熟 native compiler，更不是已完成后端 | `src/build.rs` |
| 自举准备 | `[ ]` | 已有长期路线与关卡条件 | 现在不应被当成当前 KPI 或宣传口径 | `PLAN.md` `规划.md` |
| FFI / 包管理 / IDE | `[ ]` | 暂未进入主线 | 当前不应拿这些维度当项目成败标准 | `PLAN.md` |

## 现在最容易被误读的三件事

### 1. `build` 不等于成熟原生后端

- 当前 `axc build` 很重要，但它现在的意义是稳定骨架产物与后端前接口，不是“已经可以和成熟编译语言同台比性能”。

### 2. `import / module` 已经进入实现，但仍是最小方案

- 这条线已经不该再被表述成“还没做”。
- 但也不该被表述成“已经拥有完整包系统、可见性系统和生态分发能力”。

### 3. 自举是后段目标，不是当前价值判断

- 当前最该看的不是“AX 什么时候不用 Rust”，而是“AX 能不能把 AI 修复协议和 benchmark 证据链做得够硬”。

## 现在最值得外部读者看的入口

1. [`../PROJECT_FACTS.md`](../PROJECT_FACTS.md)
2. [`repair-benchmark.md`](./repair-benchmark.md)
3. [`host-runtime-boundary.md`](./host-runtime-boundary.md)
4. [`import-module-minimal-design.md`](./import-module-minimal-design.md)
