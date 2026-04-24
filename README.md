<div align="center">
  <img src="./assets/ax-logo.svg" alt="AX logo" width="132" height="132" />

# AX

### AX — AI-first Source Protocol and Execution Language Prototype

[![CI](https://img.shields.io/github/actions/workflow/status/AX-FDN/AX/ci.yml?branch=main&label=CI)](https://github.com/AX-FDN/AX/actions/workflows/ci.yml)
[![License](https://img.shields.io/github/license/AX-FDN/AX)](./LICENSE)
[![Prototype](https://img.shields.io/badge/status-prototype-0ea5e9)](./规划.md)
[![Diagnostics](https://img.shields.io/badge/diagnostics-structured-111827)](./docs/diagnostics-schema.md)
[![Benchmark](https://img.shields.io/badge/repair%20benchmark-included-2563eb)](./docs/repair-benchmark.md)
[![Syntax](https://img.shields.io/badge/syntax-frozen%20prototype-1d4ed8)](./SYNTAX.md)

</div>

AX 不是一个“再造通用语言”的口号项目。

它更准确的定位是：一个面向 Coding AI 的源码协议实验。AX 把受约束的语言表面、结构化诊断、修复反馈契约和 benchmark 证据链一起工程化，用来回答一个更现实的问题：

> 当越来越多的代码由 Codex、Claude 这类 Coding AI 生成时，什么样的源码形式、错误反馈和修复上下文，能让它们生成得更稳定、修得更稳、比较得更清楚？

AX 想做的，不只是“发明一种新语法”，而是把这件事做成可验证的工程系统。

## What AX Is Building

AX 当前同时在建设三层东西：

- 一套低歧义、可规范化的源码形式，主动收敛等价写法和高漂移语法
- 一套面向 agent 的编译器反馈协议，围绕 `rule_id`、`repair_goal`、`fixits`、`context_snippets` 输出结构化修复上下文
- 一条可重复的 repair benchmark 证据链，用真实坏例子、候选修复、评分脚本、smoke 和对比报告验证这些设计是否真的有效

这也是 AX 最有价值的地方：它不只讨论“AI 更适合什么语言”，而是把“怎样让 Coding AI 在真实任务上更稳定”拆成可实现、可测试、可比较的部件。

## What AX Is Not

AX 不把自己建立在“贴合某个隐藏 tokenizer”这种无法验证的前提上。

我们能控制的是：

- 源码的规范化形式是否更低熵
- 诊断结构是否更稳定、更容易被模型消费
- 修复上下文是否更聚焦、更少漂移
- benchmark 结果是否在真实模型任务上更有说服力

所以 AX 更应该被理解成：

- AI-first constrained language
- compiler diagnostics and repair contract
- benchmark methodology for coding agents

而不是一句“这是下一代通用语言”。

## Why This Direction

传统语言默认服务的是“人类自由书写”。
AX 更在意的是“模型稳定生成、稳定理解、稳定修复”。

一旦代码开始大量由模型产出，真正影响结果的往往不是语法看起来多先进，而是下面这些基础设施是否扎实：

- 一个语义是否能尽量收敛到更少的主写法
- 编译器是否能给出稳定、机器可消费的 diagnostics
- 修复接口是否能明确告诉 agent 错在哪、改什么、哪些上下文相关
- benchmark 是否能把“感觉更好”变成可比较的数据

AX 追求的不是最大化写法自由，而是：

- 生成更确定
- 诊断更结构化
- 修复更可控
- 对比更可复现

## What Exists Today

AX 已经不是纸上概念。当前仓库里已经有一条可以真实跑通的原型链路：

- `axc check / run / ast / hir / mir / fmt / build`
- `Lexer -> Parser -> AST -> HIR -> Semantic Check -> Interpreter`
- 结构化 diagnostics，与 `--json --ai` AI 增强反馈
- repair benchmark、adapter、comparison、smoke 脚本和 CI
- 项目 manifest、接口快照测试、稳定文档入口

这意味着 AX 现在更像一个正在被工程化推进的编译器前端与协议实验，而不是单纯的“语言哲学”。

## How AX Should Be Evaluated

AX 最终不该靠口号成立，而要靠证据成立。

当前最关键的问题不是“还能再加多少新语法”，而是下面这些指标是否真的改善：

- 同一任务上，AI 首次生成成功率是否提升
- 同一坏例子上，单轮 repair 成功率是否提升
- 同等语义下，输入输出 token 和上下文消耗是否更可控
- 同一 benchmark 上，不同模型和不同版本是否都能稳定受益
- 结构化反馈是否真的比普通报错更能指导修复

如果这些问题没有数据支撑，AX 就只是一个有想法的原型。
如果这些问题能持续拿出证据，AX 才有资格被看成一套成立的 AI-oriented source protocol。

## What Makes AX Different

| 传统语言默认假设 | AX 的设计选择 |
| --- | --- |
| 代码主要由人写 | 代码越来越多地由模型生成 |
| 编译器主要负责报错 | 编译器同时负责提供可修复反馈契约 |
| 报错给人看懂就够了 | 诊断既要给人看，也要给 agent 稳定消费 |
| 同一个意思允许多种写法 | 尽量收敛到更少、更稳的表达形式 |
| 修复靠自由发挥 | 修复围绕固定字段、规则卡和上下文切片收敛 |

所以 AX 不是只在设计语法，而是在同时建设：

- 语言本体
- 编译器前中端
- 结构化诊断协议
- AI 修复反馈层
- benchmark 验证闭环

## Quick Look

```ax
struct Point {
    x: i32,
    y: i32,
}

fn total(point: Point) -> i32 {
    return point.x + point.y;
}

fn main() -> i32 {
    let mut point: Point = Point { x: 2, y: 3 };
    point.x = point.x + 1;
    println(total(point));
    return 0;
}
```

```powershell
.\scripts\cargo-gnu.ps1 run -- check examples\syntax_overview.ax
.\scripts\cargo-gnu.ps1 run -- run examples\hello.ax
.\scripts\cargo-gnu.ps1 run -- check examples\missing_semicolon.ax --json --ai
```

更多命令、示例、benchmark 工作流和 Windows 使用方式，请看 [`详细介绍.md`](./详细介绍.md)。

## Repository Guide

- [`详细介绍.md`](./详细介绍.md)
  当前原型已经支持的命令、示例、repair benchmark、adapter 和实践说明。
- [`docs/repair-benchmark.md`](./docs/repair-benchmark.md)
  benchmark 资产、导出链路、评分方法和对比方式。
- [`docs/repair-adapter-spec.md`](./docs/repair-adapter-spec.md)
  外部 repair adapter 的输入输出契约。
- [`docs/diagnostics-schema.md`](./docs/diagnostics-schema.md)
  结构化 diagnostics 与 AI 修复字段定义。
- [`SYNTAX.md`](./SYNTAX.md)
  当前 AX 原型语法、支持范围与 EBNF。
- [`PLAN.md`](./PLAN.md)
  项目路线、边界、核心原则与阶段目标。
- [`规划.md`](./规划.md)
  从当前原型收口到 benchmark、后端与产品化的执行顺序。
- [`WORKLIST.md`](./WORKLIST.md)
  当前施工项、优先级和已完成记录。
- [`docs/README.md`](./docs/README.md)
  稳定外部文档入口。

## Current Position

AX 现在的状态很明确：

- 它已经不是概念项目
- 它也还不是一个已经被大规模证据证明成立的成熟语言系统
- 它当前更接近“编译器前端 + 解释执行 + AI 修复协议”的原型，而不是完整的成熟后端生态

当前阶段最重要的，不是继续堆叠宏大叙事，而是把三件事做硬：

- 低歧义源码形式
- 可消费的修复协议
- 可重复的 benchmark 证据链

AX 接下来要赢的，不是“特性数量”，而是“证据质量”。

## Read This First If You Are An Agent

- 只使用仓库当前已经实现的原型语法
- 不要擅自发明 `match`、泛型、模块系统、异常或 `async`
- 切片目前已支持，写法是 `[Type]` 和 `values[start:end]`
- 字符串与遍历辅助目前已支持 `string + string`、`string_len(text)`、统一长度查询 `len(value)` 和最小格式化能力 `to_string(value)`
- 数据结构写入目前已支持可变路径赋值，包括 `outer.inner.value = expr;` 和 `tokens[index].value = expr;`
- 循环控制目前已支持 `break;`，可直接退出最近一层 `while` 或 `for`
- `main` 必须写成 `fn main() -> i32 { ... }`
- 局部变量、参数、返回值都应显式标注类型
- 先参考 [`SYNTAX.md`](./SYNTAX.md)，再生成 AX 代码
- 如果要消费结构化修复反馈，请优先参考 [`docs/diagnostics-schema.md`](./docs/diagnostics-schema.md)

## Closing

AX 要做的，不是把“新语言”三个字喊得更响。

AX 要做的，是把一套面向 Coding AI 的源码约束、编译器反馈协议和 benchmark 方法学做扎实。

如果这条路走通，AX 的价值不会只是“又一个语言项目”，而会是一套真正可验证、可复用、可对比的 AI 时代编程协议。
