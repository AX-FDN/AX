# AX Foundation Inventory

> 本文记录 P2 阶段 `foundation/` helper 的当前分类。
> 它不是正式标准库冻结文档；`foundation -> std.*` 的正式迁移属于 P3。

## 当前结论

`foundation/` 当前是 Std-0 孵化层。
它已经不是纯一次性 demo glue，但也还不是冻结后的官方 `std.*`。

当前 P3 结论：

- 保留所有现有 foundation 文件。
- 已经有九组样例迁移到 `std.*`，但不继续全仓改名。
- 不把 Rust crate 或宿主实现细节暴露成 AX 用户接口。
- 后续新增或迁移 helper 必须能被代表样例、宿主边界样例或 repair case 证明需要。
- `foundation/search.ax`、`foundation/file_kind.ax` 和部分 workspace 展示 helper 继续孵化，不进入 Std-1 冻结候选。

## Helper 分类

| 文件 | 分类 | 未来方向 | 当前理由 |
| --- | --- | --- | --- |
| [`../foundation/cli.ax`](../foundation/cli.ax) | 已部分下沉 | `std.cli` 已有冻结候选 | 入口校验函数已进入 `std.cli`；`ensure_directory / recreate_directory` 继续留在孵化层 |
| [`../foundation/report.ax`](../foundation/report.ax) | 已大部分下沉 | `std.report` 已有冻结候选 | key/value 报告、section 和 path stat 已进入 `std.report`；旧 helper 保留给未迁移样例 |
| [`../foundation/text.ax`](../foundation/text.ax) | 已部分下沉 | `std.text` 已有冻结候选 | 文本统计和 trim/normalize 已进入 `std.text`；旧 `analyze_text` 命名保留给未迁移样例 |
| [`../foundation/file_kind.ax`](../foundation/file_kind.ax) | 继续孵化 | `std.path` / 后续搜索接口候选 | `std.path` 已吸收轻量分类；markdown/searchable 分类仍偏 workload 经验，需要更多验证 |
| [`../foundation/workspace.ax`](../foundation/workspace.ax) | 已部分下沉 | `std.workspace` 已有冻结候选 | workspace line 和 label 已进入 `std.workspace`；`append_named_line` 继续孵化 |
| [`../foundation/search.ax`](../foundation/search.ax) | 继续孵化 | 后续 `std.search` 候选 | 搜索统计可复用，但当前只有搜索类 workload 强依赖，不进入 Std-1 |

## 当前不删除的原因

目前没有明显的一次性 helper 值得在 P2 删除。
这些 helper 至少满足下面一个条件：

- 被两个以上样例复用。
- 承担代表样例的核心流程。
- 承担宿主边界样例的输入校验或报告输出。
- 能给未来 `std.*` 命名空间提供候选接口。

如果后续发现某个 helper 只服务单一样例，并且无法映射到 `std.cli / std.text / std.report / std.workspace / std.path`，再从对应样例私有 `lib/` 中承接，不继续留在 `foundation/`。

## P3 迁移结果

已完成：

- `foundation/cli.ax -> std.cli` 的核心入口校验接口下沉。
- `foundation/report.ax -> std.report` 的确定性报告接口下沉。
- `foundation/text.ax -> std.text` 的基础文本统计、trim 与 normalize 接口下沉。
- `foundation/workspace.ax` 的稳定展示辅助下沉到 `std.workspace`。
- `foundation/file_kind.ax` 的轻量分类下沉到 `std.path`，但 searchable/markdown 经验规则继续孵化。
- `foundation/search.ax` 不进入 Std-1，继续等待搜索 workload 或 repair case 证明。

详细冻结候选见 [`stdlib-minimal-boundary.md`](./stdlib-minimal-boundary.md)。

## 新 helper 准入规则

新增 `foundation/` helper 必须同时回答：

- 哪个固定代表样例或宿主边界样例需要它。
- 它未来更可能属于哪个 `std.*` 命名空间。
- 它是否能避免至少两个样例重复实现同一逻辑。
- 它的失败方式是否需要 diagnostics 或 runtime AI rule 跟进。
