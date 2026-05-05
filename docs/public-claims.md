# AX Public Claims Boundary

> 本文件定义 AX 当前对外表述边界。它的目的不是削弱项目定位，而是防止把仓库内可复现事实说成尚未完成的外部结论。

## 一句话口径

推荐对外这样介绍 AX：

> AX is an AI-first tool language and compiler/runtime prototype that makes source syntax, structured diagnostics, repair contracts, architecture context, and benchmark evidence part of one language system.

中文口径：

> AX 是一门面向 Coding AI 的 AI-first 工具语言与编译器/运行时原型，把显式语法、结构化诊断、修复协议、架构上下文和 benchmark 证据链放进同一套语言系统。

## 应用场景口径

推荐把 AX 的 AI-first 场景说具体：

> AX first targets agent-generated CLI tools, repairable automation scripts, backend worker utilities, and compiler-guided repair benchmarks.

中文：

> AX 当前优先服务 agent 生成 CLI 工具、可修复自动化脚本、后端 worker 辅助工具，以及由编译器事实驱动的修复 benchmark。

可以说：

- AX 正在先把 CLI / worker tools 做成稳定语言内核。
- AX 的后端路线已经先启动本地 path package v0，后续会经过 AOT、JSON/config/log 和 worker workload，再评估 HTTP client/server 与 async。
- AX 的 AI-first 指的是生成、修复、项目理解和验证链路对 Coding AI 更友好。

不建议说：

- “AX 已经是完整后端语言。”
- “AX 已经能替代现有 Web 框架。”
- “AX 的 AI-first 等于只给 AI 看，不需要人类审阅。”

## 已经可以说的事实

这些是当前仓库内已经成立、可以对外引用的事实：

- AX 已经有可运行的 `axc check / run / fmt / build / context` 命令面。
- AX 已经有 `--json` 与 `--json --ai` 诊断输出。
- AI 增强诊断已经包含 `rule_id / repair_goal / fixits / context_snippets`。
- `axc context` 已经提供 `overview / boundaries / topology / flow / symbol / impact / evidence` 七个视图。
- repair benchmark 已经有 manifest、export、adapter spec、run、score、compare、smoke 链路。
- 当前 full repair manifest 有 `43` 个 case。
- 当前 deterministic replay 可复现 `cold 23/30`、`base 25/30`、`ai 30/30`。
- `export-repair-benchmark.ps1 -IncludeContext` 已能把 `overview / boundaries / evidence` 写入 repair bundle 与 prompt。
- `Repair Archaeology v0` 已有 artifact schema、最小导出脚本和固定 smoke；后续目标是把仓库内 replay / score / compare 事实继续整理成更完整的 case 级 JSON / Markdown 报告。
- 本地 path package v0 已进入主线，项目可以通过 `[dependencies] alias = { path = ... }` 复用本地 AX 源码包。
- 本地 path package v0 已有 `PX0001~PX0007` 稳定 resolver 错误码，并能在 context 中暴露 `local_path_packages`。
- `AX.lock` v0 已进入主线，可通过 `axc lock <project> [--check]` 生成或校验本地 path package 图。
- context `overview/topology/evidence` 已能暴露 `local_package_lock` 状态，供 agent 判断本地包锁文件是否缺失、当前有效、过期或不可读。
- `axc build` 已进入 LLVM AOT v0，`build-manifest.json` 当前 schema version 为 `10`，`aot_readiness.schema_version` 为 `3`。
- 当前默认 AOT parity smoke 覆盖 `123` 个 run-vs-exe 样例，其中 `26` 个是 `AX.toml` project 样例，仓库内全部 project 示例都已列入默认清单。
- AX Native ABI v1 已有文档收口：`string = ptr`、`slice = { ptr, i32 }`、`string_list = opaque ptr`，当前内存策略是 process-lifetime allocation v0。
- curated package registry v0 已有设计文档；当前是计划和契约，不是公共上传服务。
- Windows 是 full workflow 平台，Linux 是 core support 平台，macOS 尚未进入承诺范围。

## 可以作为目标说，但不能作为结论说的内容

这些是 AX 的目标或假设，需要明确使用“目标、正在验证、用于验证、下一步证明”等措辞：

- 提高 Coding AI 的首轮生成通过率。
- 提高结构化修复成功率。
- 降低多文件项目中的架构理解成本。
- 证明 AX 比受限 Rust / Go / Python / TypeScript 子集更适合指定 agent 任务。
- 证明 context bundle 对 live model 有稳定收益。
- 证明 AX 的低熵源码表面能跨模型、跨版本持续有效。

推荐写法：

- “AX 的目标是提高……”
- “当前仓库已经建立了验证这件事的 benchmark 链路……”
- “外部 cross-language / live-model 对照仍是下一阶段工作……”

避免写法：

- “AX 已经证明比 Rust / Go / Python 更适合 AI。”
- “AX 已经让所有模型生成更稳定。”
- “AX 已经完成 AI 时代语言的最终答案。”
- “AX 的 tokenizer 设计直接匹配 Codex / Claude 内部 tokenizer。”
- “Repair Archaeology 展示的是模型在线协商全过程。” 当前 v0 只计划整理离线可复现证据。

## Benchmark 引用口径

引用当前 benchmark 时，使用这句话：

> In the repository-internal deterministic replay benchmark, AX currently reproduces `cold 23/30`, `base 25/30`, and `ai 30/30` over 30 repair cases. This validates the internal repair evidence loop; cross-language and live-model claims remain future work.

中文：

> 在仓库内 deterministic replay benchmark 中，AX 当前在 30 个 repair case 上可复现 `cold 23/30`、`base 25/30`、`ai 30/30`。这证明内部修复证据链已经成立；跨语言和 live model 结论仍是后续工作。

## Repair Archaeology 引用口径

引用 Repair Archaeology 时，使用这句话：

> Repair Archaeology v0 is the first evidence display layer over AX's existing repair benchmark artifacts. Its artifact schema and minimal export/smoke are already in-repo, and it will keep growing into richer JSON and Markdown case reports. It is not a live-model claim and not an agent runtime.

中文：

> Repair Archaeology v0 是 AX 现有 repair benchmark 产物之上的第一层证据展示面。它的 artifact schema 与最小导出/smoke 已经进入仓库，后续会继续长成更完整的 JSON 和 Markdown case 报告；它不是 live-model 结论，也不是 agent 运行时。

## Context 引用口径

引用 context 协议时，使用这句话：

> AX context is already a compiler-produced JSON interface for project overview, boundaries, topology, flow, symbol, impact, and evidence. The first repair-consumable context shell is `overview + boundaries + evidence` via `-IncludeContext`.

中文：

> AX context 已经是编译器生成的 JSON 接口，覆盖 overview、boundaries、topology、flow、symbol、impact、evidence。第一条进入 repair 输入链的最小壳层是通过 `-IncludeContext` 导出的 `overview + boundaries + evidence`。

## 语言定位边界

AX 应该被表述为语言项目，而不是纯 research harness：

- `source protocol` 是 AX 语言与编译器接口的一部分，不是 AX 的全部。
- `diagnostics / context / repair / benchmark` 是 AX 的编译器护城河，不是取代语言本体。
- 语言内核、标准库、包接口、AOT、自举仍然是长期路线的一部分。

更准确的表达是：

> AX 是 AI-first 工具语言；它的护城河是显式语法、结构化诊断、架构上下文、修复协议和 benchmark 闭环共同组成的编译器系统。

不建议表达成：

> AX 只是一个 source protocol 实验。

## 更新规则

当下面任一事实变化时，需要同步更新本文件、[`README.md`](../README.md)、[`PROJECT_FACTS.md`](../PROJECT_FACTS.md) 和 [`benchmark-showcase.md`](./benchmark-showcase.md)：

- repair case 总数变化
- deterministic replay 结果变化
- context bundle 默认视图变化
- live-model benchmark 结果进入仓库
- cross-language benchmark 结果进入仓库
- Repair Archaeology artifact schema 或报告入口进入仓库
- AOT parity 默认样例数量或 project 样例数量变化
- `build-manifest.json` / `aot_readiness` schema version 变化
- package registry 从设计进入可安装/可上传实现
- AX 0.1 Alpha release scope 变化
- 平台支持等级变化
