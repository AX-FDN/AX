<div align="center">
  <img src="./assets/ax-logo.svg" alt="AX logo" width="132" height="132" />

# AX

### AX — The AI Execution Language

[![CI](https://img.shields.io/github/actions/workflow/status/AX-FDN/AX/ci.yml?branch=main&label=CI)](https://github.com/AX-FDN/AX/actions/workflows/ci.yml)
[![License](https://img.shields.io/github/license/AX-FDN/AX)](./LICENSE)
[![Prototype](https://img.shields.io/badge/status-prototype-0ea5e9)](./规划.md)
[![Diagnostics](https://img.shields.io/badge/diagnostics-structured-111827)](./docs/diagnostics-schema.md)
[![Benchmark](https://img.shields.io/badge/repair%20benchmark-included-2563eb)](./docs/repair-benchmark.md)
[![Syntax](https://img.shields.io/badge/syntax-frozen%20prototype-1d4ed8)](./SYNTAX.md)

</div>

AX 不是“又一门通用语言”的复刻项目，也不是一个只会把源码 parse 掉的玩具编译器。

AX 的目标更直接，也更适合 AI 时代：

> 当越来越多的代码由大模型生成时，语言本身和编译器能不能主动为 AI 的稳定生成、稳定理解、稳定修复而设计？

我们的回答是：可以，而且值得认真做。

## Why AX

传统语言默认服务的是“人类自由书写”。  
AX 更在意的是“模型稳定输出”。

这意味着 AX 会主动压缩很多对大模型并不友好的空间：

- 减少等价写法、模糊语法和高歧义结构
- 避免隐式转换、隐式控制流和漂移很大的错误表达
- 让编译器输出稳定、可比较、机器可消费的诊断结果
- 让 agent 可以围绕 `rule_id`、`repair_goal`、`fixits` 和上下文切片收敛修复

AX 要追求的不是“写法自由度最大化”，而是：

- 生成更确定
- 诊断更结构化
- 修复更可控
- benchmark 更可比较

## What Exists Today

AX 已经不是纸上概念。当前仓库里已经有一条真实可运行的原型链路：

- `axc check / run / ast / hir / mir / fmt / build`
- `Lexer -> Parser -> AST -> HIR -> Semantic Check -> Interpreter`
- 结构化 diagnostics，与 `--json --ai` AI 增强反馈
- repair benchmark、adapter、comparison、smoke 脚本和 CI
- 项目 manifest、接口快照测试、稳定文档入口

这意味着 AX 现在更像一个正在被工程化推进的语言系统，而不是一句“我想造门语言”的口号。

## What Makes AX Different

| 传统语言默认假设 | AX 的设计选择 |
| --- | --- |
| 代码主要由人写 | 代码越来越多地由模型生成 |
| 编译器主要负责报错 | 编译器同时负责提供可修复协议 |
| 报错给人看懂就够了 | 诊断既要给人看，也要给 agent 稳定消费 |
| 同一个意思允许多种写法 | 尽量收敛到更少、更稳的表达形式 |
| 修复靠自由发挥 | 修复围绕固定字段和规则卡收敛 |

所以 AX 不是只在设计语法，而是在同时建设：

- 语言本体
- 编译器前中端
- 结构化诊断协议
- AI 修复反馈层
- benchmark 验证闭环

## Design Statement

AX 正在尝试把编程语言从“给人类高手使用的自然型工具”，推进成“适合 AI 消费的工业化标准件”。

这并不意味着 AX 要做得僵硬，而是要让整条链路都更稳定：

- 生成时少乱写
- 报错时少含糊
- 修复时少漂移
- 对比时少靠感觉

如果这条路线成立，AX 的价值不会只是“有一套语法”，而会是一整套 AI 原生的编程协议与工具链。

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
- [`SYNTAX.md`](./SYNTAX.md)
  当前 AX 原型语法、支持范围与 EBNF。
- [`PLAN.md`](./PLAN.md)
  项目路线、边界、核心原则与阶段目标。
- [`规划.md`](./规划.md)
  从当前原型收口到 benchmark、后端与产品化的执行顺序。
- [`WORKLIST.md`](./WORKLIST.md)
  当前施工项、优先级和已完成记录。
- [`docs/README.md`](./docs/README.md)
  稳定外部文档入口，包括 diagnostics schema 与 repair adapter 契约。

## Current Position

AX 现在的状态很明确：

- 它已经不是概念项目
- 它也还不是“已经证明自己成立”的成熟语言

当前最关键的，不是无止境地堆新特性，而是持续验证三件事：

- AX 是否真的提升 AI 首次生成成功率
- AX 是否真的提升单轮修复成功率
- AX 的结构化反馈是否真的比普通报错更有价值

AX 接下来要赢的，不是“特性数量”，而是证据。

## Read This First If You Are An Agent

- 只使用仓库当前已经实现的原型语法
- 不要擅自发明 `match`、泛型、模块系统、异常或 `async`
- 只读切片目前已支持，写法是 `[Type]` 和 `values[start:end]`
- 字符串与遍历辅助目前已支持 `string + string`、`string_len(text)`、统一长度查询 `len(value)` 和最小格式化能力 `to_string(value)`
- 数据结构写入目前已支持可变路径赋值，包括 `outer.inner.value = expr;` 和 `tokens[index].value = expr;`
- `main` 必须写成 `fn main() -> i32 { ... }`
- 局部变量、参数、返回值都应显式标注类型
- 先参考 [`SYNTAX.md`](./SYNTAX.md)，再生成 AX 代码
- 如果要消费结构化修复反馈，请优先参考 [`docs/diagnostics-schema.md`](./docs/diagnostics-schema.md)

## Closing

AX 要做的，不是再造一门“更自由”的语言。

AX 要做的，是一门对 AI 更稳定、对编译器更友好、对错误更可诊断、对修复更可控的语言。

如果这条路走通，AX 不会只是“又一个语言项目”，而会是一套真正属于 AI 时代的编程协议。
