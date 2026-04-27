# AX Representative Samples

> 本文只定义当前 P2 阶段用于验证“最小可写工具内核”的样例集合。
> 它不是路线图；阶段目标和优先级仍以 [`../PLAN.md`](../PLAN.md) 与 [`../WORKLIST.md`](../WORKLIST.md) 为准。

## 为什么固定样例集合

AX 现在需要用少量稳定 workload 验证语言内核，而不是继续堆更多分散 demo。
P2 阶段的样例分两类：

- 主代表样例：验证 AX 是否能写真实工具程序。
- 宿主边界样例：验证 AX 通过宿主能力访问 `process / env / path / fs` 时是否稳定。

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

## 当前 P2 使用规则

- 新增语言能力时，先判断它是否能让上述样例更稳定、更短、更可诊断。
- 新增 helper 时，先判断它是主代表样例复用出来的能力，还是一次性 glue。
- 新增宿主 builtin 时，必须能落到宿主边界样例或对应 runtime diagnostics。
- 新增 diagnostics / AI rule 时，优先让它能解释这些样例里的真实失败方式。

## 已接入的回归层

当前 `tests/interface_snapshots.rs` 已覆盖：

- 五组 project-backed 样例的 `axc check`
- 五组 project-backed 样例的 `axc run`
- 五组 project-backed 样例的 `axc build`

这意味着 P2 的代表样例不再只是文档名词，而是进入了固定回归链。
