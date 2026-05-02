# AX Representative Samples

> 本文只定义当前 P2 阶段用于验证“最小可写工具内核”的样例集合。
> 它不是路线图；阶段目标和优先级仍以 [`../执行路线.md`](../执行路线.md) 为准。

## 为什么固定样例集合

AX 现在需要用少量稳定 workload 验证语言内核，而不是继续堆更多分散 demo。
P2/P3 交界阶段的样例分三类：

- 主代表样例：验证 AX 是否能写真实工具程序。
- 宿主边界样例：验证 AX 通过宿主能力访问 `process / env / path / fs` 时是否稳定。
- 包接口压力样例：验证本地 AX 包、`AX.lock`、标准库和 worker 风格入口能否组合成更接近后端任务的 workload。

新增语法、基础库 helper、宿主 builtin 或诊断规则时，优先看这些样例是否真的受益。

## 主代表样例

| 样例 | 主要职责 | 当前回归 |
| --- | --- | --- |
| [`../examples/project_directory_index/`](../examples/project_directory_index/) | 目录读取、条目分类、汇总报告、共享 helper 组合 | `check / run / build` |
| [`../examples/project_text_normalize/`](../examples/project_text_normalize/) | 文本读取、规范化、统计、报告输出 | `check / run / build` |
| [`../examples/project_release_promote/`](../examples/project_release_promote/) | 文件移动、发布目录准备、收据生成 | `check / run / build` |

这三组样例共同回答一个问题：AX 当前能不能写小型确定性工具，而不只是 `hello world` 或单文件语法展示。

## 宿主边界样例

| 样例 | 主要职责 | 当前回归 |
| --- | --- | --- |
| [`../examples/project_command_capture/`](../examples/project_command_capture/) | 工作目录校验、命令捕获、环境变量存在性、输出报告 | `check / run / build` |
| [`../examples/project_command_batch/`](../examples/project_command_batch/) | 批量命令执行、工作目录内执行、环境变量读取、路径写入 | `check / run / build` |

这两组样例专门承接宿主能力边界。
如果未来改 `process_*`、`env_*`、`path_*`、`fs_*` builtin，必须优先确认这两组样例没有回退。

## 包接口压力样例

| 样例 | 主要职责 | 当前回归 |
| --- | --- | --- |
| [`../examples/project_package_config/`](../examples/project_package_config/) | 本地 path package v0、包内规则模块、配置校验报告、build manifest package graph | `check / run / build / context` |
| [`../examples/project_job_runner/`](../examples/project_job_runner/) | 本地 path package v0、`AX.lock`、worker/job runner 入口、`std.process / std.env / std.result` 组合 | `check / run / build / lock / context` |

这两组样例承接“AX 包接口优先”的路线：用户依赖的是 AX 包模块，不是直接导入 Rust crate。
如果未来改 `[dependencies]`、`AX.lock`、package graph readiness、build manifest 或 context package facts，必须优先确认这两组样例没有回退。

## 当前 P2 使用规则

- 新增语言能力时，先判断它是否能让上述样例更稳定、更短、更可诊断。
- 新增 helper 时，先判断它是主代表样例复用出来的能力，还是一次性 glue。
- 新增宿主 builtin 时，必须能落到宿主边界样例或对应 runtime diagnostics。
- 新增包接口能力时，必须能落到包接口压力样例，并同步 `AX.lock`、context evidence 与 build manifest 契约。
- 新增 diagnostics / AI rule 时，优先让它能解释这些样例里的真实失败方式。

## 语言能力压力样例

这些样例不替代主代表样例和宿主边界样例；它们用于验证某条语言能力是否已经从单文件 smoke 进入 project-backed workload。

| 样例 | 主要职责 | 当前回归 |
| --- | --- | --- |
| [`../examples/project_payload_event_report/`](../examples/project_payload_event_report/) | 用 payload enum 表达 repair/event 事件，跨 support modules 进入数组、`match`、报告生成和文件输出 | `check / run / build` |

## 已接入的回归层

当前 `tests/interface_snapshots.rs` 已覆盖：

- 固定 project-backed 样例的 `axc check`
- 固定 project-backed 样例的 `axc run`
- 固定 project-backed 样例的 `axc build`

这意味着 P2 的代表样例不再只是文档名词，而是进入了固定回归链。
