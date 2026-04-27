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

- `A-2026-04-27-10` `[P1]` context 进入 repair benchmark 导出链
  - 结果：
    - `scripts/export-repair-benchmark.ps1` 新增可选 `-IncludeContext` 路径，默认导出契约保持不变
    - context-enabled bundle 新增 `context_bundle`，首批固定消费 `overview / boundaries / evidence`
    - `cases[].context_symbol` 可指定 evidence 视图符号，缺省回退到 `main`
    - prompt 新增 `AX context bundle` 段落，让 adapter 能同时消费源码、diagnostics、项目快照和架构上下文
    - `tests/interface_snapshots.rs` 已覆盖 context-enabled export 回归

- `A-2026-04-27-09` `[Docs]` 对外定位口径收回到 AI-first 语言主线
  - 结果：
    - `docs/why-not-language-subsets.md` 不再把 AX 表述成 source-protocol experiment，而是表述为拥有 source protocol、diagnostics contract、repair contract 与 benchmark loop 的 AI-first tool language
    - `docs/killer-demo.md` 和 `docs/quickstart-linux.md` 的话术已改成语言优先
    - `架构上下文文档.md` 已更新为当前事实：七个 context 视图已经存在，下一步是进入 repair / benchmark 消费链
    - `WORKLIST.md` 当前优先级已切换到 `P1 context -> repair/benchmark` 主攻

- `A-2026-04-27-08` `[P3]` 最小标准库边界前置完成
  - 结果：
    - `docs/stdlib-minimal-boundary.md` 已定义 `std.text / std.cli / std.fs / std.path / std.env / std.process / std.report / std.workspace / std.collections` 的第一版边界
    - 已建立 `foundation/* -> std.*` 映射表
    - 已写清标准库接口四件套：diagnostics、docs、examples、regression
    - 已选定 `project_text_normalize` 为第一迁移试点，`project_directory_index` 为第二迁移试点

- `A-2026-04-27-07` `[P2]` `foundation/` helper 分类收口
  - 结果：
    - `docs/foundation-inventory.md` 已记录当前 helper 分类、保留理由、P3 迁移前置和新增 helper 准入规则
    - 当前 `foundation/` 被定义为 Std-0 孵化层，不在 P2 直接改名为 `std.*`
    - `WORKLIST.md` 已把 `W-P2-04` 标记为完成，后续标准库冻结进入 P3 前置项

- `A-2026-04-27-06` `[P2]` 代表样例与宿主边界样例固定
  - 结果：
    - `docs/representative-samples.md` 已定义 `3` 个主代表样例与 `2` 个宿主边界样例
    - `README.md`、`PROJECT_FACTS.md`、`docs/README.md` 已指向固定样例集合
    - `tests/interface_snapshots.rs` 已补充五组 project-backed 样例的 `axc check` 回归，与既有 `run / build` 回归形成闭环

- `A-2026-04-27-05` `[P2]` `match` 第二刀启动并接入 bootstrap 回归
  - 结果：
    - `WORKLIST.md` 已冻结 `match` 第二刀与 payload enum 深化范围，并启动第一项 P2 语法实现
    - `examples/bootstrap_state_machine.ax`、`examples/bootstrap_block_summary.ax`、`examples/bootstrap_token_scan.ax` 已改成用 enum-first `match` 承担真实控制流分派
    - `tests/interface_snapshots.rs` 已覆盖三个 bootstrap 样例运行回归
    - `src/semantic.rs` 已补充 enum `match` 非穷尽诊断回归

- `A-2026-04-27-04` `[P0]` Windows 本机 GNU 验证路径固定并实测通过
  - 结果：
    - `scripts/cargo-gnu.ps1` 的 native cargo 参数转发收紧，`test --lib` 可稳定透传
    - `README.md`、`详细介绍.md`、`docs/quickstart.md`、`docs/quickstart-windows.md`、`docs/platform-support.md` 已统一写明 Windows 本机正式可复跑路径
    - 本机已按 GNU 路径实际通过 `build`、`test --lib` 和 `test --test interface_snapshots`

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
