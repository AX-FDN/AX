# AX Foundation Inventory

> 本文记录 P2 阶段 `foundation/` helper 的当前分类。
> 它不是正式标准库冻结文档；`foundation -> std.*` 的正式迁移属于 P3。

## 当前结论

`foundation/` 当前是 Std-0 孵化层。
它已经不是纯一次性 demo glue，但也还不是冻结后的官方 `std.*`。

当前 P2 结论：

- 保留所有现有 foundation 文件。
- 不在 P2 改名为 `std.*`。
- 不把 Rust crate 或宿主实现细节暴露成 AX 用户接口。
- 后续新增 helper 必须能被代表样例、宿主边界样例或 repair case 证明需要。

## Helper 分类

| 文件 | 分类 | 未来方向 | 当前理由 |
| --- | --- | --- | --- |
| [`../foundation/cli.ax`](../foundation/cli.ax) | 可复用 | `std.cli` 候选 | 多个 project-backed 样例复用参数数量、路径存在性和退出消息处理 |
| [`../foundation/report.ax`](../foundation/report.ax) | 可复用 | `std.report` 候选 | 多个样例复用 key/value 报告、section 和 path stat 输出 |
| [`../foundation/text.ax`](../foundation/text.ax) | 可复用 | `std.text` 候选 | 文本统计已被文本处理与命令捕获样例复用 |
| [`../foundation/file_kind.ax`](../foundation/file_kind.ax) | 继续孵化 | `std.path` / `std.workspace` 候选 | 当前主要服务搜索和目录索引，分类规则还偏项目经验，需要更多 workload 验证 |
| [`../foundation/workspace.ax`](../foundation/workspace.ax) | 继续孵化 | `std.workspace` 候选 | 工作区展示格式有复用价值，但还没到冻结 API 的程度 |
| [`../foundation/search.ax`](../foundation/search.ax) | 继续孵化 | `std.text` / `std.search` 候选 | 搜索统计可复用，但是否进入第一版标准库还要看 P3 命名空间边界 |

## 当前不删除的原因

目前没有明显的一次性 helper 值得在 P2 删除。
这些 helper 至少满足下面一个条件：

- 被两个以上样例复用。
- 承担代表样例的核心流程。
- 承担宿主边界样例的输入校验或报告输出。
- 能给未来 `std.*` 命名空间提供候选接口。

如果后续发现某个 helper 只服务单一样例，并且无法映射到 `std.cli / std.text / std.report / std.workspace / std.path`，再从对应样例私有 `lib/` 中承接，不继续留在 `foundation/`。

## P3 迁移前置

进入 P3 前，至少要完成：

- `foundation/cli.ax -> std.cli` 的接口边界草案。
- `foundation/report.ax -> std.report` 的接口边界草案。
- `foundation/text.ax -> std.text` 的接口边界草案。
- `foundation/workspace.ax` 与 `foundation/file_kind.ax` 是否合并进 `std.workspace` 的判断。
- `foundation/search.ax` 是否进入第一版标准库，还是继续留作样例库。

## 新 helper 准入规则

新增 `foundation/` helper 必须同时回答：

- 哪个固定代表样例或宿主边界样例需要它。
- 它未来更可能属于哪个 `std.*` 命名空间。
- 它是否能避免至少两个样例重复实现同一逻辑。
- 它的失败方式是否需要 diagnostics 或 runtime AI rule 跟进。
