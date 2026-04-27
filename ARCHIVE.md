# AX 已完成归档

> 本文件只记录“已经做完的东西”。  
> 它不是路线图，也不是当前待做清单。  
> 未来完成的新事项，一律从 [`WORKLIST.md`](./WORKLIST.md) 移到这里归档。

最后更新：2026-04-27

## 文档职责

- [`PLAN.md`](./PLAN.md)
  管方向、阶段和切换条件。
- [`WORKLIST.md`](./WORKLIST.md)
  管当前还要做的小细节。
- [`ARCHIVE.md`](./ARCHIVE.md)
  管已经完成的事项、产物和日期。

## 归档规则

1. 这里只写“已完成”，不写“计划做”。
2. 每条归档尽量标注完成日期和挂靠的 `PLAN` 阶段编号。
3. 这里记录的是里程碑，不是逐提交 changelog；更细的改动以 Git 历史为准。

## 已完成里程碑

### 2026-04-27

- `A-2026-04-27-01` `[P1]` 上下文协议补齐 `evidence`
  - 结果：
    - `axc context evidence <path> <symbol> --json` 已进入主线
    - `related_examples / related_tests / related_docs / related_benchmarks / expected_artifacts` 已有稳定输出
    - `tests/interface_snapshots.rs` 已补对应快照回归

- `A-2026-04-27-02` `[P0]` 根目录规划文档收口为三层
  - 结果：
    - `PLAN.md` 成为唯一方向基线
    - `WORKLIST.md` 只保留当前待做细节
    - `ARCHIVE.md` 独立承接已完成事项
    - 并行规划文档已清理，根目录引用口径统一

- `A-2026-04-27-03` `[P0]` 唯一路线基线升级为闭环计划书 v3
  - 结果：
    - 主计划从三阶段总纲升级为 `P0-P7` 闭环路线
    - 语法、标准库、包接口、AOT、自举、平台、context 闭环的启动条件和退出条件被写清
    - `WORKLIST.md` 与 `ARCHIVE.md` 全部切换到 `P0-P7` 挂靠口径

### 2026-04-26

- `A-2026-04-26-01` `[P1]` 上下文协议第一批视图进入主线
  - 结果：
    - `overview / boundaries / topology / flow / symbol / impact` 已进入 `axc context`
    - 第一批稳定 schema、hints 和 validation 壳层已经形成
    - context 相关 interface snapshots 已落地

### 2026-04-25

- `A-2026-04-25-01` `[P2]` project-backed 真实工具样例成组落地
  - 结果：
    - `project_command_capture`
    - `project_release_promote`
    - `project_directory_index`
    - `project_command_batch`
    - `project_text_normalize`
    - `project_workspace_search_report`
  - 说明：
    - AX 已经不只会跑玩具样例，而是能承载多文件、带 `foundation` 的工具型样例

- `A-2026-04-25-02` `[P2]` 共享 `foundation/` 第一版成形
  - 结果：
    - `foundation/cli.ax`
    - `foundation/file_kind.ax`
    - `foundation/report.ax`
    - `foundation/search.ax`
    - `foundation/text.ax`
    - `foundation/workspace.ax`

- `A-2026-04-25-03` `[P2]` 第一批 minimal collections 以 `string_list` 进入主线
  - 结果：
    - `string_list_new`
    - `string_list_push`
    - `string_list_join`
    - `len(string_list)`
  - 说明：
    - 这不是为了“像标准库”，而是直接服务真实工具样例里的动态字符串聚合

- `A-2026-04-25-04` `[P0]` Linux 核心支持第一阶段完成
  - 结果：
    - Linux 上核心 `build / check / run / fmt`
    - 纯核心 Rust 测试与关键 smoke 路径可跑
    - Windows 仍保持全量支持平台角色

### 2026-04-24

- `A-2026-04-24-01` `[P2]` 只读切片全链路落地
  - 结果：
    - `[Type]`
    - `values[start:end]`
    - 数组作 slice 形参
    - 对应语义、解释执行、示例与测试补齐

- `A-2026-04-24-02` `[P2]` 更实用的字符串与最小格式化能力落地
  - 结果：
    - `string + string`
    - `string_len(text)`
    - `len(value)`
    - `to_string(value)`

- `A-2026-04-24-03` `[P2]` 空数组字面量策略固定
  - 结果：
    - 只在显式零长度数组上下文中通过
    - 非法情形有稳定 diagnostics 和 AI 规则

- `A-2026-04-24-04` `[P2]` 嵌套可写路径与循环控制继续补齐
  - 结果：
    - 嵌套字段/索引写入进入主线
    - `break;`、`continue;` 等高价值控制流能力进入解释执行与回归面

- `A-2026-04-24-05` `[P1]` 工具风格坏例子与修复样例扩充
  - 结果：
    - repair cases、replay candidates、compare smoke 覆盖更多真实误用
    - `cold / base / ai` 比较路径继续稳定

### 2026-04-23

- `A-2026-04-23-01` `[P0]` 第一条可运行原型链打通
  - 结果：
    - `axc check / run / ast / hir / mir / fmt / build`
    - `Lexer -> Parser -> AST -> HIR -> MIR -> Semantic -> Interpreter`

- `A-2026-04-23-02` `[P1]` 结构化 diagnostics 与 `--json --ai` 固定
  - 结果：
    - 基础 JSON diagnostics 稳定
    - `rule_id / repair_goal / fixits / context_snippets` 进入主链
    - session 版本化与回归资产建立

- `A-2026-04-23-03` `[P1]` repair benchmark、comparison、smoke 与 CI 骨架完成
  - 结果：
    - export / run / score / compare / smoke 主链跑通
    - repair asset 不再只是零散脚本

- `A-2026-04-23-04` `[P1]` `benchmark-diagnostics.ps1` 与性能基线成形
  - 结果：
    - `check`
    - `check --json`
    - `check --json --ai`
    - 三模式相对开销可重复测量

- `A-2026-04-23-05` `[P1]` 运行期结构化错误开始进入主线
  - 结果：
    - `axc run --json`
    - 首批 `axc run --json --ai`
    - 高频 runtime diagnostics 已可被修复链消费

- `A-2026-04-23-06` `[P2]` 第一批工具风格样例建立
  - 结果：
    - `bootstrap_token_scan.ax`
    - `bootstrap_state_machine.ax`
    - `bootstrap_block_summary.ax`
  - 说明：
    - 这批样例让 AX 脱离“只有 hello world 和 type mismatch”的阶段
