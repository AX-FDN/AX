# AX Minimal Standard Library Boundary

> 本文定义 P3 阶段的 `foundation -> std.*` 最小边界和第一迁移试点。
> 当前仓库已经新增第一批 `std/` AX 源码模块；`foundation/` 仍保留为未迁移样例的 Std-0 孵化层。

## P3 目标

P3 的目标不是一次性做完整标准库，而是先把第一版官方标准接口的边界写清楚，再用一个真实 project-backed 样例验证迁移成本。

第一版标准库只服务“可写工具语言”：

- 文本处理
- 命令行输入校验
- 文件系统与路径
- 环境变量与进程
- 报告输出
- 工作区扫描
- 最小集合类型

## 第一版命名空间边界

| 命名空间 | 范围 | 当前来源 | P3 边界 |
| --- | --- | --- | --- |
| `std.text` | 字符串统计、trim、split、contains、starts/ends with、简单 normalize | `std/text.ax`、`foundation/text.ax` 与宿主 string builtin | 只放纯文本处理，不放文件读取 |
| `std.cli` | 参数数量校验、usage、退出消息、输入路径校验 | `std/cli.ax` 与 `foundation/cli.ax` | 只放 CLI 程序入口常用约束，不放业务流程 |
| `std.fs` | 文件/目录存在性、读写、复制、移动、删除、重命名、目录创建、目录枚举、文件大小 | `std/fs.ax` 与宿主 `fs_*` builtin | AX 接口稳定，宿主实现细节隐藏 |
| `std.option` | 显式可能缺失值：`Option<T>`、`Some(T)`、`None`、基础查询与 fallback | `std/option.ax` | 只定义低熵返回值约定，不引入隐式空值或异常 |
| `std.path` | join、parent、file name、stem、extension、resolve、文件类型辅助 | `std/path.ax`、宿主 `path_*` builtin 与 `foundation/file_kind.ax` | 路径操作和轻量分类放这里，工作区递归逻辑不放这里 |
| `std.env` | 环境变量存在性与读取 | `std/env.ax` 与宿主 `env_*` builtin | 只暴露 AX 函数，不暴露宿主环境实现 |
| `std.process` | 命令执行、工作目录内执行、输出捕获 | `std/process.ax` 与宿主 `process_*` builtin | 保持小而明确，不引入 async 或流式进程 API |
| `std.report` | key/value 行、section、path stat、bool/int/string stat | `std/report.ax` 与 `foundation/report.ax` | 只放确定性文本报告构造，不放展示主题系统 |
| `std.result` | 显式可能失败值：`Result<T,E>`、`Ok(T)`、`Err(E)`、基础查询与 fallback | `std/result.ax` | 先作为官方返回值约定，不引入 `?` 或异常系统 |
| `std.workspace` | 工作区条目显示、深度前缀、递归扫描辅助 | `std/workspace.ax`、`foundation/workspace.ax` 与部分样例私有逻辑 | 只承接小型工具常用 workspace 输出，不做完整项目索引器 |
| `std.collections` | `string_list` 和后续最小集合 helper | 宿主 `string_list_*` builtin | 第一版只承认已有最小集合，不提前上泛型 collections |

## Foundation 映射表

| 当前文件 | P3 目标 | 迁移策略 |
| --- | --- | --- |
| [`../foundation/cli.ax`](../foundation/cli.ax) | `std.cli` | 第一批迁移候选 |
| [`../foundation/report.ax`](../foundation/report.ax) | `std.report` | 第一批迁移候选 |
| [`../foundation/text.ax`](../foundation/text.ax) | `std.text` | 第一批迁移候选 |
| [`../foundation/file_kind.ax`](../foundation/file_kind.ax) | `std.path` 或 `std.workspace` | 先继续孵化，等路径分类边界稳定再迁 |
| [`../foundation/workspace.ax`](../foundation/workspace.ax) | `std.workspace` | 先继续孵化，等目录/工作区样例再稳定一轮 |
| [`../foundation/search.ax`](../foundation/search.ax) | `std.text` 或后续 `std.search` | 暂不进入 Std-1，等搜索 workload 增多再决定 |

## 当前已落地的 `std/` 试点模块

| 文件 | 模块 | 当前职责 | 迁移状态 |
| --- | --- | --- | --- |
| [`../std/cli.ax`](../std/cli.ax) | `std.cli` | CLI 参数校验、usage、错误退出消息、输入文本校验 | 第一、第三、第四、第五试点已用 |
| [`../std/env.ax`](../std/env.ax) | `std.env` | 环境变量存在性判断与读取 | 第四、第五试点已用 |
| [`../std/fs.ax`](../std/fs.ax) | `std.fs` | 文件读取、文件写入、目录创建、目录枚举、文件大小、文件/目录存在性判断、删除文件、重命名 | 第一、第二、第三、第四、第五试点已用 |
| [`../std/option.ax`](../std/option.ax) | `std.option` | `Option<T>`、`Some(T)`、`None`、静态构造 `Option.some`、`is_some / is_none / unwrap_or` | `project_option_result` 已用 |
| [`../std/path.ax`](../std/path.ax) | `std.path` | `join / parent / file_name / stem / extension / resolve / classify_file_kind / is_text_file` | 第一、第二、第三、第五试点已用 |
| [`../std/process.ax`](../std/process.ax) | `std.process` | 命令执行、工作目录内执行与输出捕获 | 第四、第五试点已用 |
| [`../std/report.ax`](../std/report.ax) | `std.report` | key/value 报告、路径报告、section 片段 | 第一、第二、第三、第四、第五试点已用 |
| [`../std/result.ax`](../std/result.ax) | `std.result` | `Result<T,E>`、`Ok(T)`、`Err(E)`、`is_ok / is_err / unwrap_or / error_or` | `project_option_result` 已用 |
| [`../std/text.ax`](../std/text.ax) | `std.text` | trim、文本统计、基础 normalize | 第一、第四、第五试点已用 |
| [`../std/workspace.ax`](../std/workspace.ax) | `std.workspace` | workspace 条目行、深度前缀、展示 label | 第二试点已用 |

## Std-1 冻结候选清单

五组迁移试点完成后，第一版 Std-1 不再继续按“看到 helper 就迁移”的方式扩张。
当前冻结候选只包括已经被 project-backed workload 消费、并进入 `check / run / build` 或 interface snapshots 回归的接口。

| 模块 | 冻结候选接口 | 已验证 workload | 冻结口径 |
| --- | --- | --- | --- |
| `std.cli` | `usage_error / require_min_args / exit_with_message / require_file / require_directory / require_non_empty_text / ensure_output_parent` | text normalize、directory index、release promote、command capture、command batch | 只冻结入口校验与输出父目录准备，不冻结目录重建策略 |
| `std.env` | `has / get` | command capture、command batch | `get` 必须优先配合 `has` 使用；本轮不设计默认值、optional 或错误传播语法 |
| `std.fs` | `read_to_string / write_string / create_dir_all / exists / remove_file / rename / read_dir / file_size / is_file / is_dir` | 五组迁移试点 | 只冻结同步文件系统薄接口，不引入权限模型、流式 IO、watcher 或平台细节 |
| `std.option` | `Option<T> / Some(T) / None / Option.some / is_some / is_none / unwrap_or` | option/result smoke | 冻结显式缺失值约定；不引入隐式 null |
| `std.path` | `join / parent / file_name / stem / extension / resolve / classify_file_kind / is_text_file` | text normalize、directory index、release promote、command batch | 路径拼接与轻量分类可冻结；分类规则是工具语言默认策略，不等于完整 MIME / 文件类型系统 |
| `std.process` | `run / run_in / capture_in` | command capture、command batch | 只冻结同步命令执行与输出捕获；不冻结 stdout/stderr/exit-code 结构体、shell contract、async 或 streaming |
| `std.report` | `append_line / append_string_stat / append_int_stat / append_bool_stat / append_path_stat / begin_section / append_section_details_or_none` | 五组迁移试点 | 只冻结确定性 key/value 与 section 文本报告，不做表格、主题、颜色或富文本 |
| `std.result` | `Result<T,E> / Ok(T) / Err(E) / is_ok / is_err / unwrap_or / error_or` | option/result smoke | 冻结显式失败值约定；错误传播语法与结构化错误类型后置 |
| `std.text` | `TextStats / zero_text_stats / trim / analyze / normalize_content` | text normalize、command capture、command batch | 只冻结纯字符串处理与基础统计，不放搜索语义或文件读取 |
| `std.workspace` | `display_label / depth_prefix / append_workspace_line` | directory index、command batch | 只冻结 workspace 展示辅助，不冻结递归扫描、索引策略或搜索策略 |

当前 `std.collections` 只作为命名空间方向保留，不进入 Std-1 冻结候选。
原因是仓库目前依赖的是宿主 `string_list_*` builtin，而不是 `std/collections.ax` 源码模块；它需要等更多集合 workload 或泛型路线明确后再启动。

## Std-1 候选验证入口

Std-1 当前不是靠“文档声明”冻结，而是靠五组 project-backed 样例和 interface snapshots 共同保护。
验证入口固定为下面三层：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots representative_project_examples_check_cleanly
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots project_text_normalize
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\cargo-gnu.ps1 test --test interface_snapshots
```

三层含义分别是：

- `representative_project_examples_check_cleanly`
  快速确认五组 Std-1 试点项目都还能 `check`。
- 单个 `project_*` filter
  局部确认某个试点的 `run` 夹具和 `build` source tree 快照。
- 完整 `interface_snapshots`
  提交前或接口变更时的全量契约回归。

当前覆盖关系如下：

| Std-1 候选模块 | 主要覆盖样例 | interface snapshots 覆盖点 |
| --- | --- | --- |
| `std.cli` | text normalize、directory index、release promote、command capture、command batch | 五组样例的 `check`，对应运行夹具，以及五组 `*_build_copies_real_example_source_tree` |
| `std.env` | command capture、command batch | `project_command_capture_runs_on_controlled_fixture`、`project_command_batch_runs_on_controlled_fixture`、两组 build source tree |
| `std.fs` | 五组迁移试点 | 五组运行夹具覆盖读写、目录创建、目录枚举、文件大小、存在性、删除和重命名；五组 build source tree 确认 `std/fs.ax` 被复制 |
| `std.option` | option/result smoke | `project_option_result_runs` 覆盖 `Option.some / None / is_none / unwrap_or`；build source tree 确认 `std/option.ax` 被复制 |
| `std.path` | text normalize、directory index、release promote、command batch | 对应运行夹具覆盖 join、parent、file name、extension、resolve 与轻量分类；build source tree 确认 `std/path.ax` 被复制 |
| `std.process` | command capture、command batch | 两组命令类运行夹具覆盖 `capture_in / run / run_in`；build source tree 确认 `std/process.ax` 被复制 |
| `std.report` | 五组迁移试点 | 五组运行夹具覆盖 deterministic 文本报告构造；build source tree 确认 `std/report.ax` 被复制 |
| `std.result` | option/result smoke | `project_option_result_runs` 覆盖 `Ok / Err / is_ok / unwrap_or / error_or`；build source tree 确认 `std/result.ax` 被复制 |
| `std.text` | text normalize、command capture、command batch | 文本归一化、命令输出统计、batch 报告运行夹具覆盖 `trim / analyze / normalize_content` |
| `std.workspace` | directory index、command batch | 目录索引和 batch 报告运行夹具覆盖 workspace 行输出与深度展示；build source tree 确认 `std/workspace.ax` 被复制 |

`tests/interface_snapshots.rs` 里的 `SHARED_STD_PROJECT_SOURCES` 是当前 Std-1 build source tree 的最小契约集合。
新增或移除 `std/` 源码模块时，必须同步更新该集合、本文、[`validation-matrix.md`](./validation-matrix.md) 和 [`interface-contracts.md`](./interface-contracts.md)。

## 继续孵化清单

下面这些接口或 helper 仍然保留在 `foundation/` 或样例私有 `lib/`，不进入 Std-1：

| 位置 | 当前职责 | 不冻结原因 | 重新评估触发条件 |
| --- | --- | --- | --- |
| [`../foundation/search.ax`](../foundation/search.ax) | `SearchStats / search_text` | 搜索语义还没有跨多个 `std.*` 迁移试点验证；直接建 `std.search` 会扩大命名空间 | `project_workspace_search_report` 或新的 repair case 明确需要可复用搜索接口 |
| [`../foundation/file_kind.ax`](../foundation/file_kind.ax) | markdown/text/searchable 文件分类 | `std.path` 已吸收轻量分类，但 markdown/searchable 策略仍偏 workload 经验 | 至少两个 project-backed 样例共同需要 markdown/searchable 分类 |
| [`../foundation/workspace.ax`](../foundation/workspace.ax) 的 `append_named_line` | 简单文件名详情行 | 当前只有旧 foundation 样例和迁移过渡需要，`std.workspace.append_workspace_line` 已覆盖更稳定形态 | 新 workload 证明 named-line 比 workspace-line 更适合作为通用接口 |
| [`../foundation/cli.ax`](../foundation/cli.ax) 的 `ensure_directory / recreate_directory` | 输出目录确保与重建 | `std.fs.create_dir_all / remove_file / rename` 已覆盖基础操作；目录重建策略更危险，应留给项目私有逻辑 | 发布/构建类样例反复需要同一套安全重建策略，并补齐诊断边界 |
| 样例私有 `lib.*` | 业务报告、扫描、发布收据、搜索汇总 | 这些是 workload 逻辑，不是标准库逻辑 | 两个以上代表样例复制同一业务逻辑，且接口能保持低熵 |

下一轮如果要继续扩标准库，必须由下面至少一个来源触发：

- 固定代表样例被当前 `std.*` 明确卡住。
- 宿主边界样例暴露出无法用现有 `std.process / std.env / std.fs / std.path` 表达的重复模式。
- repair case 或 context evidence 指向同一类高频误用，并需要标准接口承接。
- 两个以上 project-backed 样例重复实现同一段通用逻辑。

## 标准库接口四件套

任何 `std.*` 接口冻结前，必须同时具备四件套：

- diagnostics：误用时能产生稳定诊断或 runtime AI rule。
- docs：有语义、参数、返回值和失败边界说明。
- examples：至少一个固定样例使用该接口。
- regression：进入 `check / run / build` 或对应 unit/interface 回归。

如果缺任一项，该接口只能留在 `foundation/` 或样例私有 `lib/`，不能宣称为官方标准库接口。

## 迁移试点

第一迁移试点：[`../examples/project_text_normalize/`](../examples/project_text_normalize/)

理由：

- 同时消费 `std.cli`、`std.text`、`std.report`、`std.fs`、`std.path`。
- 业务逻辑相对清晰，失败面小，适合验证命名空间迁移成本。
- 输出报告稳定，便于做回归。

当前迁移结果：

- `AX.toml` 使用 `sources = ["../../std", "lib"]`。
- `src/main.ax` 通过 `import std.cli / std.fs / std.path / std.text` 调用标准接口。
- `lib.normalize` 与 `lib.report` 保留项目私有逻辑，验证“标准库 + 项目私有模块”的组合方式。
- interface snapshots 已覆盖该项目的运行夹具和 build source tree。

第二迁移试点：[`../examples/project_directory_index/`](../examples/project_directory_index/)

理由：

- 能验证 `std.workspace`、`std.path`、`std.report` 的组合边界。
- 更接近真实工作区扫描工具，适合在第一试点稳定后推进。

当前迁移结果：

- `AX.toml` 使用 `sources = ["../../std", "lib"]`。
- `src/main.ax` 通过 `import std.cli / std.fs` 调用入口约束、目录读取和输出写入。
- `lib.scan` 通过 `std.fs / std.path / std.workspace` 承载递归扫描、文件分类和 workspace 行输出。
- `lib.report` 通过 `std.report` 构造 summary。
- interface snapshots 已覆盖该项目的运行夹具和 build source tree。

暂不选 [`../examples/project_command_batch/`](../examples/project_command_batch/) 作为第一试点，因为它触碰 `std.process / std.env`，宿主边界更多，适合在文本与报告接口先稳定后再迁。

第三迁移试点：[`../examples/project_release_promote/`](../examples/project_release_promote/)

理由：

- 能验证发布型工具常见的 rename、覆盖已有文件、receipt report 和路径 extension。
- 继续使用 `std.cli / std.fs / std.path / std.report`，不提前触碰 `std.process / std.env`。
- 是 P2 主代表样例之一，迁移价值高于继续迁移 toy example。

当前迁移结果：

- `AX.toml` 使用 `sources = ["../../std", "lib"]`。
- `src/main.ax` 通过 `std.cli / std.fs / std.path` 完成入口校验、发布目录处理、文件提升和 receipt 输出。
- `lib.receipt` 通过 `std.report / std.path / std.fs` 构造发布收据。
- interface snapshots 已覆盖该项目的运行夹具和 build source tree。

第四迁移试点：[`../examples/project_command_capture/`](../examples/project_command_capture/)

理由：

- 能验证最薄的宿主命令捕获路径是否可以下沉到 `std.process`。
- 能验证环境变量存在性检查是否可以通过 `std.env` 暴露为 AX 侧标准接口。
- 它是 P2 宿主边界样例之一，比继续迁移纯文本样例更能暴露真实 host boundary 压力。

当前迁移结果：

- `AX.toml` 使用 `sources = ["../../std"]`。
- `src/main.ax` 通过 `std.cli / std.process / std.env / std.text / std.report / std.fs` 完成参数校验、命令输出捕获、环境变量检查、文本统计、报告构造和输出写入。
- 第一版 `std.process` 只暴露 `run / run_in / capture_in`，不引入结构化 stdout/stderr/exit-code 结果体。
- 第一版 `std.env` 只暴露 `has / get`，不提前设计默认值或错误传播语法。
- interface snapshots 已覆盖该项目的 build source tree，命令捕获路径继续作为宿主边界回归样例。

第五迁移试点：[`../examples/project_command_batch/`](../examples/project_command_batch/)

理由：

- 能验证 `std.process.run / std.process.run_in` 是否足够承载 batch 类工具。
- 能验证 `std.env.get` 是否可以在有 `std.env.has` 护栏时安全进入真实样例。
- 能验证 `std.* + lib.*` 组合在宿主边界更重的项目里仍然保持清晰分层。

当前迁移结果：

- `AX.toml` 使用 `sources = ["../../std", "lib"]`。
- `src/main.ax` 通过 `std.cli / std.fs / std.path / std.process / std.env` 完成入口校验、输出目录创建、命令执行和环境变量读取。
- `lib.report` 通过 `std.report / std.fs / std.text / std.workspace` 构造 batch 报告。
- `std.text` 新增 `trim` 薄包装，用来替代样例层直接调用 `string_trim`。
- 本轮没有引入结构化 process result、跨平台 shell contract、async/streaming process 或 Linux benchmark/orchestration 承诺。
- interface snapshots 已覆盖该项目的运行夹具和 build source tree。

第六迁移试点：[`../examples/project_option_result/`](../examples/project_option_result/)

理由：

- 将泛型 enum 从单文件样例推进到 `std/` 官方约定层。
- 验证 `std.option.Option<T>` 与 `std.result.Result<T,E>` 可以跨模块声明、静态构造、variant 构造、match、fallback 和进入 build source tree。
- 为后续错误传播语法、标准库失败返回值和 repair contract 统一返回形态提供前置接口。

当前迁移结果：

- `AX.toml` 使用 `sources = ["../../std"]`。
- `src/main.ax` 通过 `import std.option / std.result` 消费官方约定类型。
- semantic test 已覆盖泛型 enum unit variant 在期望实例类型下的归入，例如 `let missing: Option<i32> = Option.None;`。
- interface snapshots 已覆盖该项目的 `check / run / build source tree`。
- 本轮没有引入 `?`、异常系统、泛型 trait 或完整错误模型。

## Rust 宿主边界

用户视角只能看到 AX 标准接口：

- `std.fs.*`
- `std.path.*`
- `std.env.*`
- `std.process.*`
- `std.option.*`
- `std.result.*`
- `std.text.*`
- `std.report.*`
- `std.cli.*`

用户不应该看到：

- Rust crate 名称
- Cargo dependency 名称
- Rust 模块路径
- 平台分支实现细节

宿主 Rust 只负责提供不可避免的系统能力，例如文件系统、进程、环境变量和路径解析。
AX 标准库负责定义稳定接口、输入输出形状、错误边界和示例。

## 当前不启动的能力

P3 前置不启动：

- 通用 FFI
- `AX import -> Cargo crate`
- registry package
- async process API
- network 标准库
- 泛型 collections
- 标准库全仓重命名

这些能力必须等 P4/P5 之后再重新评估。
