# AX Project Facts

> 阅读提示：本文件是 AX 的“当前事实清单”，给外部读者、协作者和 AI 工具快速建立同一套项目认知用。  
> 它不是路线图，不替代 [`PLAN.md`](./PLAN.md)、[`WORKLIST.md`](./WORKLIST.md) 或 [`规划.md`](./规划.md)。

## 一句话定义

AX 是一个面向 Coding AI 的源码协议与执行语言原型，目标不是做“又一门通用语言”，而是把源码形态、结构化诊断、修复协议和 benchmark 证据链放进同一个可运行仓库里。

## AX 现在是什么

- 一个已经具备 `axc check / run / fmt / ast / hir / mir / build` 的语言原型
- 一个以解释执行为当前主路径的前中端系统
- 一个已经输出文本诊断、`--json` 与 `--json --ai` 三层反馈的编译器接口
- 一个已经把 `rule_id / repair_goal / fixits / context_snippets` 接入主链的 AI 修复协议原型
- 一个已经内置 repair benchmark、compare、smoke 与 CI 的证据链仓库

## AX 现在不是什么

- 不是要立刻取代 `Python / Rust / Go` 的通用语言产品
- 不是已经成熟的原生编译后端；当前 `build` 仍是骨架产物导出路径
- 不是已经完成自举的编译器；当前编译器仍明确是 `Rust` 种子实现
- 不是已经拥有包管理、FFI、成熟 IDE、完整标准库的生态型项目

## 当前已经成立的事实

### 1. 编译器主链已经打通

- 词法、语法、AST、HIR、MIR、语义检查、解释执行都已进入同一仓库主线
- `axc check`、`axc run`、`axc fmt`、`axc ast`、`axc hir`、`axc mir`、`axc build` 都已有稳定入口

### 2. 结构化诊断已经是主产品面

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

### 5. benchmark 证据链是主线能力，不是附属脚本

- 仓库内已有 repair cases、adapter spec、导出、评分、对比、smoke 与 CI
- AX 的主张不是“语法更酷”，而是“对 AI 更稳定、更可修复”，所以 benchmark 是继续条件的一部分

## 当前仍处于“第一版 / 在硬化”的部分

- `import / module`：第一刀已落地，但还在补更多接口回归和边界覆盖
- host runtime boundary：已开始收紧，但仍是当前最重要的持续硬化方向之一
- `src/ai.rs` 的规则触发：正在从文案匹配继续迁移到更稳定的内部语义标签
- `build`：当前仍是 backend 前的构建骨架，不应被表述成成熟 native compiler

## 明确后置、不是当前主线的方向

- 完整 native backend
- 自举与 AX 重写编译器核心
- FFI 与包管理系统
- 成熟 IDE / 调试器生态
- async、宏系统、复杂泛型、庞大标准库

## 当前最该用什么来评价 AX

不要优先问“它有没有泛型 / FFI / JIT / 包管理”，而应优先问：

1. 它是否已经形成稳定的 `check/json/ai` 修复协议？
2. 它是否能让真实坏例子进入可回放、可评分、可比较的 benchmark？
3. 它是否已经能承载一批真实工具风格样例，而不是只会跑玩具例子？
4. 它的 host boundary 是否在持续收紧，而不是越来越松？

## 推荐的阅读顺序

1. [`README.md`](./README.md)：项目介绍与入口
2. [`docs/feature-matrix.md`](./docs/feature-matrix.md)：当前能力面、边界和非目标
3. [`docs/repair-benchmark.md`](./docs/repair-benchmark.md)：benchmark 证据链
4. [`PLAN.md`](./PLAN.md)：长期设计基线
5. [`规划.md`](./规划.md)：阶段顺序与切换条件
6. [`WORKLIST.md`](./WORKLIST.md)：当前施工项

## 当前一句话判断

AX 现在最值得关注的，不是“它像不像一门完整语言”，而是它是否能持续把“源码协议 + 结构化诊断 + AI 修复协议 + benchmark 证据链”这条链做得比普通语言项目更硬。
