# AX Application Scenarios

> 本文回答一个具体问题：AX 的 AI-first 到底先落在哪些真实场景里。
> 它不是第二份路线图；阶段门槛仍以 [`../执行路线.md`](../执行路线.md) 为准。

## 一句话定位

AX 的 AI-first 不是“只给 AI 看”，也不是“围绕某个隐藏 tokenizer 下注”。
AX 先服务一类很明确的程序：由 Coding AI 高频生成、修复、验证，并且需要人类审阅的确定性工具程序。

这类程序的共同点是：

- 输入输出边界清楚
- 执行路径相对短
- 失败需要可诊断、可修复、可回放
- 经常触碰文件、路径、环境变量、进程、报告输出等宿主能力
- 适合用编译器直接给 agent 提供结构化反馈

## 当前优先应用场景

| 场景 | AX 负责什么 | 当前仓库对应资产 |
| --- | --- | --- |
| Agent-generated CLI tools | 让 agent 生成小型命令行工具后，能立刻 `check / run / fmt / build`，并通过显式语法和标准库接口降低漂移 | `examples/project_text_normalize/`、`examples/project_directory_index/`、`std/` |
| Repairable automation scripts | 让自动化脚本出错后，编译器输出 `rule_id / repair_goal / fixits / context_snippets`，进入可复跑修复链 | `src/ai.rs`、`docs/diagnostics-schema.md`、`benchmarks/repair-cases.json` |
| Backend worker utilities | 先承载后端外围 worker、构建辅助、发布处理、日志/报告整理、批处理编排，而不是立刻承诺完整 Web framework | `examples/project_release_promote/`、`examples/project_command_batch/` |
| Compiler-guided repair benchmarks | 把错误、候选修复、评分、compare、context 输入和后续 Repair Archaeology 都做成可验证资产 | `docs/benchmark-showcase.md`、`docs/repair-archaeology.md` |

## 后端路线的真实顺序

AX 可以往后端语言方向走，但顺序必须现实：

1. `CLI / worker tools`
   先把命令行工具、文件处理、文本处理、发布辅助、批处理 worker 做稳。
2. `path packages v0`
   本地 AX 包复用先成立，让项目私有库和未来标准库/三方库边界有真实载体。
3. `AOT`
   在 Std-1 候选和 path package v0 稳定后启动真实可执行产物，让 `build` 从 skeleton 进入发布路径。
4. `JSON / config / log`
   补后端 worker 最常见的数据输入输出能力。
5. `backend workers`
   让 AX 可以稳定写队列任务、批处理、报告生成、内部自动化 worker。
6. `HTTP client/server`
   等 AOT、包接口、错误模型和并发模型更稳后，再进入网络库和服务端框架。

## 当前不抢的场景

下面这些方向不是否定，而是必须后置：

- 通用 Web 后端框架
- network 标准库
- async / await
- 通用 FFI
- `AX import -> Cargo crate` 直通桥
- 完整 IDE / 调试器生态
- Live Repair Stream 的真实模型协商与 UI

它们都依赖更稳的语言内核、Std-1、AOT、包接口和错误模型。

## AI-first 的工程含义

AX 的 AI-first 目前落实在五个层面：

| 层面 | 工程要求 |
| --- | --- |
| 语法 | 显式、低歧义、少同义写法，能被 formatter 收敛成 canonical form |
| 标准库 | 用户调用 AX 接口，不直接暴露 Rust crate 或宿主实现细节 |
| 诊断 | 文本、人类可读 JSON、AI 增强 JSON 共用同一套错误事实 |
| 上下文 | agent 能拿到 overview、boundaries、topology、flow、symbol、impact、evidence |
| 验证 | 每个高价值能力都要能进入样例、snapshot、repair 或 benchmark 链路 |

## 与成熟语言目标的关系

AX 的长期目标仍然是成熟语言，不是只做 benchmark harness。
区别在于：AX 不先追求“大而全”的语法面，而是优先选择能被 agent 生成、修复、验证和审阅的语言能力。

因此当前路线是：

- 先做稳定可写工具语言
- 再冻结最小标准库
- 再启动 AOT 和包接口
- 再扩到后端 worker 与生态
- 最后才进入网络、并发、泛型抽象和更完整语言生态
