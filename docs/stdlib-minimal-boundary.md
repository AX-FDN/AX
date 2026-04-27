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
| `std.fs` | 文件/目录存在性、读写、复制、移动、删除、目录创建、目录枚举、文件大小 | `std/fs.ax` 与宿主 `fs_*` builtin | AX 接口稳定，宿主实现细节隐藏 |
| `std.path` | join、parent、file name、stem、extension、resolve、文件类型辅助 | `std/path.ax`、宿主 `path_*` builtin 与 `foundation/file_kind.ax` | 路径操作和轻量分类放这里，工作区递归逻辑不放这里 |
| `std.env` | 环境变量存在性与读取 | 宿主 `env_*` builtin | 只暴露 AX 函数，不暴露宿主环境实现 |
| `std.process` | 命令执行、工作目录内执行、输出捕获 | 宿主 `process_*` builtin | 保持小而明确，不引入 async 或流式进程 API |
| `std.report` | key/value 行、section、path stat、bool/int/string stat | `std/report.ax` 与 `foundation/report.ax` | 只放确定性文本报告构造，不放展示主题系统 |
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
| [`../std/cli.ax`](../std/cli.ax) | `std.cli` | CLI 参数校验、usage、错误退出消息、输入文本校验 | 第一试点已用 |
| [`../std/fs.ax`](../std/fs.ax) | `std.fs` | 文件读取、文件写入、目录创建、目录枚举、文件大小、文件/目录存在性判断 | 第一、第二试点已用 |
| [`../std/path.ax`](../std/path.ax) | `std.path` | `join / parent / file_name / stem / resolve / classify_file_kind / is_text_file` | 第一、第二试点已用 |
| [`../std/report.ax`](../std/report.ax) | `std.report` | key/value 报告、路径报告、section 片段 | 第一试点已用 |
| [`../std/text.ax`](../std/text.ax) | `std.text` | 文本统计、基础 normalize | 第一试点已用 |
| [`../std/workspace.ax`](../std/workspace.ax) | `std.workspace` | workspace 条目行、深度前缀、展示 label | 第二试点已用 |

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

## Rust 宿主边界

用户视角只能看到 AX 标准接口：

- `std.fs.*`
- `std.path.*`
- `std.env.*`
- `std.process.*`
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
