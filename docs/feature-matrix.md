# AX Feature Matrix

> 阅读提示：本文件只回答“AX 当前到底做到哪了”。  
> 它不是愿景文档，也不是路线图；当前执行方向请看 [`../执行路线.md`](../执行路线.md)，旧版计划见 [`../曾经的计划/`](../曾经的计划/)。

状态说明：

- `[x]` 当前主链已成立
- `[~]` 已有第一版，但仍在补边界或回归
- `[ ]` 明确后置，当前不是主线

## 当前阶段定位

| 阶段 | 当前判断 |
| --- | --- |
| `P0` 环境与契约修复 | `[~]` 文档治理与契约治理已基本收口，Windows GNU 本地验证路径已固定，剩余工作在验证矩阵与快照契约 |
| `P1` 编译器护城河 | `[x]` repair/context/benchmark 已进入主线，context-enabled export、benchmark showcase 与公开口径边界已成立 |
| `P2` 语言内核 / 可写项目能力 | `[~]` 已进入后段，但不等于“工具语言完成”；当前仍在继续补齐支撑标准库、后端与 AI 生成稳定性的通用语言表面，payload enum 已开始进入 project-backed workload |
| `P3` 官方最小标准库 | `[~]` 已启动第一批 `std.*` AX 源码模块，已完成十三组 project-backed 迁移/压力试点，并已收口 Std-1 冻结候选清单；尚未全仓冻结 |
| `P4+` AOT / 包接口 / 自举 / 生态 | `[~]` 本地 path package v0 与 `AX.lock` v0 已启动；LLVM AOT v0 已能为极小单文件 MIR 子集生成文本 IR；registry、自举和生态仍按 `执行路线.md` 后续阶段推进 |

## 总览

| 领域 | 状态 | 当前结论 |
| --- | --- | --- |
| 前端主链 | `[x]` | Lexer / Parser / AST / HIR / MIR / Semantic 已进入主线 |
| 执行路径 | `[x]` | 解释执行是当前稳定主路径 |
| 结构化诊断 | `[x]` | 文本、`--json`、`--json --ai` 三层输出已成立 |
| AI 修复协议 | `[x]` | `rule_id / layer / ai_action / safe_to_edit / validation / repair_goal / fixits / context_snippets` 已进入输出层 |
| runtime AI 反馈 | `[~]` | 首批高价值 runtime 误用已接入，仍在扩完整覆盖 |
| context 协议 | `[x]` | 七个稳定视图已进入 CLI、快照回归与 repair export 输入链 |
| 多文件项目 | `[x]` | `AX.toml + sources` 已是当前项目组织主路径 |
| `import / module` | `[~]` | 第一刀已接入 parser/project/semantic/diagnostics，仍在补边界 |
| 共享 AX 基础层 | `[~]` | `foundation/` 已沉淀第一批 helper，`std/` 已启动第一批官方接口试点，并开始覆盖文本、报告、文件、路径、工作区、环境变量、进程边界、`Option` 和 `Result` 约定 |
| benchmark 证据链 | `[x]` | repair/export/score/compare/smoke/CI 已进入仓库主线，当前展示页可复现 `cold 23/30`、`base 25/30`、`ai 30/30` |
| Repair Archaeology | `[ ]` | 已进入规划，目标是把 replay / score / compare 产物整理成 case 级 JSON 与 Markdown 报告；尚未实现导出入口 |
| Linux core support | `[x]` | Ubuntu 上核心 `build / check / run / fmt` 与核心测试已进入 CI |
| macOS support | `[ ]` | 当前未进入主线承诺 |
| 原生后端 | `[~]` | `build` 已开始为极小单文件 MIR 子集生成 LLVM IR v0，但还不是成熟 native backend |
| 自举 | `[ ]` | 长期方向，不是当前主线 |
| FFI / 包管理 / IDE | `[ ]` | 当前未进入主线实现 |

## 详细矩阵

| 领域 | 状态 | 当前已支持 | 当前边界 / 不应误读成什么 | 代表入口 |
| --- | --- | --- | --- | --- |
| 编译器前端 | `[x]` | 词法、语法、AST、HIR、MIR、语义检查 | 已是工程主链，不是 parser demo | `src/lexer.rs` `src/parser.rs` `src/semantic/` |
| 执行模型 | `[x]` | 解释执行、基础运行期错误、部分 host builtin | 当前稳定路径是 interpreter，不是 AOT/JIT | `src/interpreter.rs` |
| CLI | `[x]` | `check / ast / hir / mir / build / run / fmt / context` | 当前优先稳契约，不继续膨胀命令面 | `src/cli.rs` |
| 诊断协议 | `[x]` | 文本诊断、`--json`、`--json --ai` | AI 增强层是增量，不应污染基础层 | `docs/diagnostics-schema.md` |
| AI 修复上下文 | `[x]` | `rule_id`、`layer`、`ai_action`、`safe_to_edit`、`validation`、`repair_goal`、`fixits`、`context_snippets`、teaching level | 当前目标是稳定 contract，不是供应商定制 prompt 仓库 | `src/ai.rs` |
| parser 高频错误稳定化 | `[x]` | 缺分号、缺括号、缺类型名、缺表达式等已接稳定 kind | 仍有少量 heuristic，但主方向已改为内部标签优先 | `src/parser.rs` |
| semantic 高频错误稳定化 | `[x]` | 模块误用与首批高价值 `S0022` 变体已接稳定 kind | 还会继续扩，但主框架已成立 | `src/diagnostics.rs` `src/ai.rs` |
| runtime 高频错误稳定化 | `[~]` | 数组越界、除零、可读文件/目录、argv/env/process 一批误用已接 AI 规则 | runtime 还在持续硬化，不代表 host boundary 已完全收口 | `src/interpreter.rs` `src/ai.rs` |
| context 协议 | `[x]` | `overview / boundaries / topology / flow / symbol / impact / evidence`，并可通过 `-IncludeContext` 进入 repair export | 当前完成的是输入链路，live-model A/B 收益仍需后续证明 | `src/context.rs` `docs/interface-contracts.md` |
| 语言表面 | `[~]` | 基础函数、显式类型、数组、slice、struct、enum、泛型结构体/函数/enum、trait/interface、trait bounds、静态方法、泛型方法、for、match、module/import 第一刀 | 不含 async、异常、宏、泛型 trait、闭包 | `SYNTAX.md` |
| 项目组织 | `[~]` | `AX.toml + sources`、project-backed 样例、共享 `foundation/` 与第一批 `std/` 试点、本地 path package v0、`AX.lock` v0 | 当前不是成熟 registry/版本求解系统 | `examples/project_*/` `examples/project_package_config/` `examples/project_job_runner/` |
| 模块系统 | `[~]` | support source 模块路径、重复模块 / import、缺 import 等诊断已存在 | 当前是 minimal module mode，不是完整 package/visibility 系统 | `docs/import-module-minimal-design.md` |
| 共享基础层 | `[~]` | `foundation/cli.ax`、`report.ax`、`search.ax`、`workspace.ax` 等，以及第一批 `std/cli.ax`、`env.ax`、`fs.ax`、`option.ax`、`path.ax`、`process.ax`、`report.ax`、`result.ax`、`text.ax`、`workspace.ax` | `std/` 仍是试点，不是全仓冻结后的完整标准库 | `foundation/` `std/` |
| benchmark 方法 | `[x]` | repair case、导出、评分、对比、smoke、CI、公开展示页 | 这不是“以后再补”的附件，而是语言主线的验证层；跨语言/live-model 对照仍是后续工作 | `docs/benchmark-showcase.md` `docs/repair-benchmark.md` |
| 修复证据展示层 | `[ ]` | `Repair Archaeology v0` 已定义方向 | 当前只是规划与边界，不是 live repair、不是模型客户端、不是新 CLI 契约 | `docs/repair-archaeology.md` |
| 对外平台支持 | `[~]` | Windows 路径已较完整，Linux 有 quickstart 与核心链路说明 | 仍应按文档与 CI 事实表述，不宜夸成“全平台成熟” | `docs/platform-support.md` |
| `build` | `[~]` | 可导出构建骨架产物，`context evidence` 会暴露 `build_readiness`；极小单文件 MIR 子集可额外生成 `generated/main.ll`；`axc build --json` 会打印同一个 `build-manifest.json` 对象 | 当前不是成熟 native compiler，更不是已完成后端 | `src/build.rs` `src/backend/llvm/` `src/context.rs` |
| AOT / JIT | `[~]` | AOT readiness v3 已进入 `build-manifest.json` schema v9 与 `context evidence`；blocker 已带 AI 建议对象；LLVM AOT v0 可为 `fn main() -> i32` 级别子集生成文本 IR，支持 `i32/bool/string` core 子集、`println(i32/bool)` 最小 stdout ABI、只读 string literal 直接 `println`、string 局部变量 / 参数 / 返回值、`string_len` / `len(string)`、字符串 `==` / `!=`、`to_string(i32/bool/string)`、`string + string`、固定长度数组 literal / 参数 by value / 索引读取 / 元素写入 / `len(array)`、非泛型 struct literal / 参数 by value / 返回值 / field read / field write、非泛型 unit enum tag 值 / 参数 / 返回值 / `== !=`、语句形态 unit enum `match` 测试，链接 exe 需显式设置 `AX_LLVM_AOT_LINK=1`；已有 run vs AOT exe parity smoke 覆盖 18 个 core/stdout/string/array-read-write/struct-read-write/enum-unit-match 样例 | 仍不是发布级 native executable output；JIT 仍不启动；复杂 runtime ABI、项目链接仍未 AOT；string 分配当前是 process-lifetime v0，暂不回收；slice、payload enum、表达式形态 match 和复杂 pattern 仍未进入 native layout/lowering | `src/build.rs` `src/backend/llvm/` `src/context.rs` `docs/llvm-aot.md` |
| 包接口 / 第三方库 | `[~]` | 本地 path package v0 已进入主线：`[dependencies] alias = { path = ... }` 会把本地 AX 包源码加载为 `alias.*` 模块；resolver 错误已有 `PX0001~PX0007` 稳定文本码和 `repair_rule / repair_goal / fixit`，context 与 build manifest 可暴露 `local_path_packages`，`axc lock` 可生成/校验 `AX.lock` v0；`axc lock --check` 已有 `LX0001~LX0004` 稳定文本码、package graph drift 详情和 AI-facing repair hints；`context evidence` 和 `build-manifest.json` 都会输出 `package_graph_readiness` 与 `aot_readiness` 说明包图是否可复现、是否阻塞 AOT 前置 | 仍不是 registry、版本求解、host extension ABI 或 `AX import -> Cargo crate` 直通桥 | `src/project.rs` `src/build.rs` `src/lockfile.rs` `src/context.rs` `examples/project_package_config/` `examples/project_job_runner/` |
| 自举准备 | `[ ]` | 已有长期路线与关卡条件 | 现在不应被当成当前 KPI 或宣传口径 | `执行路线.md` |

## 当前语法面矩阵

| 语法组 | 状态 | 当前已支持 | 当前未支持 / 未冻结 |
| --- | --- | --- | --- |
| 顶层声明 | `[~]` | `fn`、`struct`、`enum`、`module`、`import`、`pub`、`impl`、`trait`、`type` 别名、本地 path package manifest 声明、`AX.lock` v0 | registry 契约 |
| 语句 | `[x]` | `let`、`let mut`、赋值、`return`、`if/else`、`while`、`for`、`for in`、`break`、`continue`、语句 `match` | `defer`、异常传播、`switch` 类语法 |
| 表达式 | `[~]` | 调用、字段访问、索引、slice、结构体字面量、枚举值、表达式 `match`、block-valued match arm、`Point { x, y }` 这类结构体全字段 shorthand 解构 pattern、逻辑运算、余数、字符串拼接、值方法调用、静态方法调用 | 闭包、字段重命名 destructuring、partial struct pattern、嵌套/数组/tuple destructuring |
| 类型系统 | `[~]` | `bool / i32 / f32 / string / string_list`、固定长度数组、只读 slice、payload enum、泛型结构体/函数/enum、trait/interface、trait bounds、where 约束、泛型方法、官方 `Option/Result` 约定、`Result` 错误传播 `?` 第一刀 | 泛型 trait、关联类型、结构化错误层级 |
| 工程组织 | `[~]` | `AX.toml + sources`、最小模块模式、全限定跨模块引用、本地 path package v0、`AX.lock` v0 | registry、版本求解、完整可见性体系 |

## 现在最容易被误读的五件事

### 1. `build` 不等于成熟原生后端

- 当前 `axc build` 很重要，但它现在的意义是稳定骨架产物、后端前接口和最小 LLVM IR v0，不是“已经可以和成熟编译语言同台比性能”。

### 2. `import / module` 已经进入实现，但仍是最小方案

- 这条线已经不该再被表述成“还没做”。
- 但也不该被表述成“已经拥有完整包系统、可见性系统和生态分发能力”。

### 3. `std/` 已经开始试点，但还不是完整标准库

- 当前 `foundation/` 仍是 Std-0 孵化层。
- 当前 `std/` 已经有第一批 AX 源码模块，并由 `project_text_normalize`、`project_directory_index`、`project_release_promote`、`project_command_capture`、`project_command_batch`、`project_option_result`、`project_env_result`、`project_file_result`、`project_process_result` 与 `project_result_pipeline` 消费。
- `project_payload_event_report` 已补上语言能力压力样例：payload enum 跨模块进入数组、`match`、报告生成和 `check / run / build` 回归。
- `project_job_runner` 已补上 package-backed 后端 worker 样例：本地 path package、`AX.lock`、`Result`、`std.process`、`std.env` 与 build/context package readiness 进入同一条回归链。
- Std-1 冻结候选清单已经收口到 `std.cli / std.collections / std.env / std.fs / std.option / std.path / std.process / std.report / std.result / std.text / std.workspace`，但这不等于完整标准库已经冻结。
- 泛型 `std.collections`、`std.search`、网络、并发和第三方包接口仍然后置；当前 `std.collections` 只冻结 `string_list` 的最小包装。

### 4. Linux core support 不等于三平台同级成熟

- Ubuntu 已经能跑核心编译器/runtime 链路。
- 但 benchmark orchestration 仍以 Windows PowerShell 路径为准，macOS 也还没进入承诺范围。

### 5. 当前价值判断要先看语言内核和编译器护城河的同步成熟度

- 当前最该看的不是“AX 什么时候不用 Rust”，而是“AX 能不能继续把语言内核做成稳定可写工具的方向，并同时把 AI 修复协议和 benchmark 证据链做得够硬”。

## 现在最值得外部读者看的入口

1. [`../PROJECT_FACTS.md`](../PROJECT_FACTS.md)
2. [`repair-benchmark.md`](./repair-benchmark.md)
3. [`host-runtime-boundary.md`](./host-runtime-boundary.md)
4. [`import-module-minimal-design.md`](./import-module-minimal-design.md)
5. [`../执行路线.md`](../执行路线.md)
