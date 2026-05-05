# AX Project Facts

> 阅读提示：本文件是 AX 的“当前事实清单”，给外部读者、协作者和 AI 工具快速建立同一套项目认知用。
> 它不是路线图，不替代 [`执行路线.md`](./执行路线.md) 或 [`ARCHIVE.md`](./ARCHIVE.md)。

## 一句话定义

AX 是一门面向自回归 Coding AI 的 AI-first 工具语言，目前处在从可运行原型向成熟工具语言推进的阶段。
它把 canonical syntax、structured diagnostics、repair contract、context protocol 和 benchmark evidence 放进同一套语言与编译器工具链里，目标是让模型在生成、修复和项目理解三个环节都更稳定。

## 当前应用场景

AX 当前优先服务四类能被仓库直接验证的场景：

| 场景 | 当前事实 |
| --- | --- |
| Agent-generated CLI tools | 已有多组 project-backed CLI / workspace 工具样例和第一批 `std.*` 试点模块 |
| Repairable automation scripts | `--json --ai`、repair cases、export、score、compare、smoke 已进入主线 |
| Backend worker utilities | 发布提升、命令批处理、文本处理、目录索引等后端外围工具样例已存在 |
| Compiler-guided repair benchmarks | deterministic replay、context-enabled export 和 Repair Archaeology v0 最小导出/smoke 已建立 |

更完整的应用场景边界见 [`docs/application-scenarios.md`](./docs/application-scenarios.md)。

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
| repair archaeology | v0 已有 artifact schema、最小导出脚本和固定 smoke，并已接入 Windows CI |
| web workbench | `web/` 已作为独立 Vite + React 前端进入主线，展示 benchmark 指标、same-case repair demo、反馈模式对比和接口契约 |
| build | AOT/native backend 正处在 LLVM v0 能力包扩展阶段；`build-manifest.json` schema version `10` 与 `context evidence` 已暴露结构化 `aot_readiness` 和 blocker-level AI 建议，`axc build --json` 会打印同一个 manifest 对象，并且 `axc build` 可为当前 AOT MIR 子集生成 `generated/main.ll` LLVM IR v0；当前 AOT parity 已覆盖 `123` 个样例，完整清单由 `scripts/smoke-aot-parity.ps1` 维护，其中 `26` 个是 `AX.toml` project 样例，仓库内全部 26 个 project 示例都已进入默认 parity 清单；覆盖范围包括 core/control-flow/consts/f32-core/stdout/string/string-runtime/string-predicate/string-replace/string-split-lines/string-trim/string-list-runtime/std-collections/std-env/std-fs/std-path/std-process/string-pattern/argv/fixed-array-read-write-format-equality/zero-length-array/for-in-readonly-slice/runtime-string-slice-for-in/slice-range-read/slice-range-for-in/slice-formatter-equality/struct-read-write-format-equality/struct-pattern/enum-unit/payload-enum/payload-enum-equality/enum-formatter/enum-print/enum-complex-payload-formatter/enum-array-slice-payload-equality/concrete-generic-enum-print/expression-match/range-pattern/or-pattern/match-guard/concrete-Result-Option/result-static-constructors/result-try/project-backed/local-path-package；AOT runtime error v0 已能在 native 侧为 `i32` negation/add/sub/mul/div/rem overflow、除零、数组/切片越界、argv 越界、env 缺失和部分 host runtime 失败输出最小 stderr 错误码消息；有 clang 时可对比解释器和 AOT exe 的 exit code / stdout / stderr |
| 平台 | Windows 为 full workflow，Linux 为 core support，macOS 尚未启动 |

## 当前语法完成度判断

| 层级 | 完成度 | 说明 |
| --- | --- | --- |
| 最小可写工具内核 | `93%~96%` | 已具备显式类型、数组/切片、`for/for in`、`break/continue`、`match` 多个高价值切片、字符串 pattern、payload enum、跨模块 payload enum 工具样例、泛型 enum、官方 `Option/Result` 约定、模块第一刀、`pub`、methods/impl、静态方法、返回上下文泛型推断、泛型结构体/函数/impl/方法、trait bounds、`where` 输入语法、泛型 `type` 别名、宿主 builtin |
| 通用语言表面 | `69%~75%` | 已补 methods/impl、静态方法、返回上下文泛型推断、泛型结构体/函数/enum/impl/method、traits/interfaces、trait bounds、`where` 输入语法、`pub`、泛型 `type` 别名、官方 `Option/Result` 约定与 `Result` 错误传播 `?` 第一刀；payload enum 已进入 project-backed 多文件工具验证；还缺泛型 trait、闭包、async、结构化错误层级、完整包系统等 |
| 生态支撑语法 | `34%~40%` | 已能组织 project-backed 样例并启动 `std.*` 试点，`Option/Result` 已成为标准错误/缺失值约定前置；仍缺稳定包接口、lockfile、host extension ABI、AOT 发布路径和第三方库契约 |

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
- 仓库内已有 project-backed 代表样例、共享 `foundation/` helper，以及第一批 `std/` 标准库试点模块
- `examples/project_payload_event_report/` 已验证 payload enum 可以跨 support modules 进入数组、`match`、报告生成和 `check / run / build` 回归
- P2 阶段主代表样例与宿主边界样例已在 [`docs/representative-samples.md`](./docs/representative-samples.md) 固定

### 5. benchmark 证据链是语言主线的验证层，不是附属脚本

- 仓库内已有 repair cases、adapter spec、导出、评分、对比、smoke 与 CI
- AX 的主张是“把语言本体和 AI 友好编译器一起做硬”，所以 benchmark 是继续条件的一部分
- 当前 full manifest 有 `43` 个 repair case；smoke subset 有 `13` 个 case；已发布 deterministic replay 快照覆盖 `30` 个 case，仓库内可复现 `cold 23/30`、`base 25/30`、`ai 30/30`
- 公开展示页见 [`docs/benchmark-showcase.md`](./docs/benchmark-showcase.md)
- [`docs/repair-archaeology.md`](./docs/repair-archaeology.md) 已定义 Repair Archaeology v0；`scripts/export-repair-archaeology.ps1` 与 `scripts/smoke-repair-archaeology.ps1` 已把 replay / score / compare 升级成按 case 可查询、可导出的修复证据对象

### 6. context 协议已经是对外接口，不是草图

- `overview / boundaries / topology / flow / symbol / impact / evidence` 都已经有命令和 JSON
- `export-repair-benchmark.ps1 -IncludeContext` 已能把 `overview / boundaries / evidence` 写入 repair bundle 与 prompt
- 当前欠缺的不是“有没有视图”或“能不能进入输入链”，而是 context-enabled benchmark 还需要继续做 live-model A/B 与公开对照结果

## 当前仍在硬化、还不应被误读成完成态的部分

- `import / module`：第一刀已落地，但还在补更多接口回归和边界覆盖
- host runtime boundary：已开始收紧，但仍是当前最重要的持续硬化方向之一
- `src/ai.rs` 的规则触发：正在从文案匹配继续迁移到更稳定的内部语义标签
- `build`：当前已启动 LLVM IR v0，但仍不应被表述成成熟 native compiler；默认只保证稳定构建资产和可选 `generated/main.ll`
- `std/`：当前已启动第一批标准库试点模块，并已由 `project_text_normalize`、`project_directory_index`、`project_release_promote`、`project_command_capture`、`project_command_batch`、`project_option_result`、`project_collections_core`、`project_env_result`、`project_file_result`、`project_process_result`、`project_result_pipeline`、`project_config_validate`、`project_collections_report`、`project_job_runner` 十四组真实样例消费；Std-1 冻结候选清单已在 [`docs/stdlib-minimal-boundary.md`](./docs/stdlib-minimal-boundary.md) 收口，但还不是完整官方标准库
- `foundation/`：当前仍是 Std-0 孵化层，负责承载尚未迁移的样例和未充分验证的 helper，尤其是搜索、markdown/searchable 文件分类和目录重建策略
- `Repair Archaeology v0`：artifact schema、最小导出脚本和固定 smoke 已落地；它仍不代表 live-model benchmark 已经完成，也暂不新增 `axc` 命令

## 当前明确后置、不是主线的方向

- 发布级 native backend
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
5. [`docs/repair-archaeology.md`](./docs/repair-archaeology.md)：修复证据可解释展示层
6. [`docs/interface-contracts.md`](./docs/interface-contracts.md)：外部契约与快照覆盖
7. [`docs/repair-benchmark.md`](./docs/repair-benchmark.md)：benchmark 证据链
8. [`web/README.md`](./web/README.md)：Repair Workbench 前端入口
9. [`执行路线.md`](./执行路线.md)：当前唯一执行路线与阶段出口
10. [`曾经的计划/`](./曾经的计划/)：已退役的旧版计划和施工单
11. [`ARCHIVE.md`](./ARCHIVE.md)：已完成事项归档

## 当前一句话判断

AX 现在最值得关注的，是它能否一边把语言内核继续推进到成熟工具语言的方向，一边把“结构化诊断 + AI 修复协议 + 上下文协议 + benchmark 证据链”做成比普通语言项目更硬的编译器护城河。
