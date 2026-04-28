# AX 已完成归档

> 本文件只记录“已经做完的东西”。  
> 它不是路线图，也不是当前待做清单。  
> 未来完成的新事项，一律从 [`WORKLIST.md`](./WORKLIST.md) 移到这里归档。

最后更新：2026-04-28

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

### 2026-04-28

- `A-2026-04-28-01` `[Docs/P3]` AI-first 应用场景与 Std-1 验证入口收口
  - 结果：
    - 新增 `docs/application-scenarios.md`，把 AX 的 AI-first 场景固定为 agent 生成 CLI 工具、可修复自动化脚本、后端 worker 辅助工具和 compiler-guided repair benchmark
    - `README.md`、`PLAN.md`、`PROJECT_FACTS.md`、`docs/public-claims.md` 和 `docs/README.md` 已接入应用场景入口
    - `docs/stdlib-minimal-boundary.md` 已写清 Std-1 候选接口当前由哪些 interface snapshots 覆盖
    - `docs/validation-matrix.md` 已新增 Std-1 candidate change 的推荐验证命令
    - `docs/interface-contracts.md` 已把 Std-1 candidate source tree 与 runtime behavior 纳入契约地图
    - `WORKLIST.md` 已把 `W-P3-15` 标记完成，并把下一步推进到 `W-P1-08` Repair Archaeology artifact schema

- `A-2026-04-28-02` `[P1]` Repair Archaeology v0 artifact schema 定义完成
  - 结果：
    - 新增 `docs/repair-archaeology-schema.md`
    - 明确 case 级 JSON artifact、Markdown 报告模板、index 结构、字段来源和 status 枚举
    - 明确 replay fact、compiler fact、runner fact、validation fact、derived fact 与 interpretation 的边界
    - `docs/repair-archaeology.md` 已改为引用 schema 专文，避免在定位文档里维护重复结构
    - `docs/interface-contracts.md` 与 `docs/validation-matrix.md` 已登记 Repair Archaeology artifact 的契约边界和后续 smoke 要求
    - `WORKLIST.md` 已把 `W-P1-08` 标记完成，并新增 `W-P1-09` 最小导出脚本

- `A-2026-04-28-03` `[P1]` Repair Archaeology v0 最小导出脚本落地
  - 结果：
    - 新增 `scripts/export-repair-archaeology.ps1`
    - 脚本读取现有 deterministic replay 的 `comparison.json`、benchmark index、run summary 与 score summary
    - 脚本输出 `index.json` 与 `cases/<case-id>.json/.md`
    - 当前支持 `base -> ai` comparison，不调用真实 LLM，不新增 `axc` 命令
    - 本地 smoke 已用 `showcase-20260424` 产物导出 `missing_semicolon_basic`、`missing_paren_condition`、`slice_assignment_read_only` 三个 case
    - `WORKLIST.md` 已把下一步推进到 `W-P1-10` 固定 smoke，避免长期依赖本地历史 `.ax-ai` 产物

- `A-2026-04-28-04` `[P1]` Repair Archaeology v0 固定 smoke 落地
  - 结果：
    - 新增 `scripts/smoke-repair-archaeology.ps1`
    - smoke 会重新导出 smoke benchmark、重新跑 deterministic `base -> ai` compare、再导出 Repair Archaeology case JSON / Markdown
    - smoke 当前覆盖 `missing_semicolon_basic`、`type_mismatch_bool_from_int`、`slice_assignment_read_only`
    - smoke 验证 `index.json`、case JSON 和 case Markdown 均存在且 JSON 可解析
    - `docs/interface-contracts.md`、`docs/validation-matrix.md`、`docs/repair-archaeology.md` 已改为引用固定 smoke
    - `WORKLIST.md` 已新增 `W-P1-11`，下一步评估是否把该 smoke 接入 Windows CI

### 2026-04-27

- `A-2026-04-27-22` `[P1]` `Repair Archaeology v0` 增长点进入规划
  - 结果：
    - 新增 `docs/repair-archaeology.md`
    - `PLAN.md` 已把 Repair Archaeology 定义为 P1 证据链展示与解释层
    - `WORKLIST.md` 已登记 `W-P1-07`，并明确排在 `W-P3-15` 之后
    - README、PROJECT_FACTS、docs README、benchmark showcase、repair benchmark、public claims、feature matrix 已加入入口或边界说明
    - 明确 v0 不调用真实 LLM、不做 `axc generate`、不新增 AX 语法、不把离线 replay 结果说成 live-model 结论

- `A-2026-04-27-21` `[P3]` Std-1 冻结候选清单收口
  - 结果：
    - `docs/stdlib-minimal-boundary.md` 已新增 Std-1 冻结候选清单
    - 冻结候选覆盖 `std.cli / std.env / std.fs / std.path / std.process / std.report / std.text / std.workspace`
    - `std.collections` 明确暂不进入冻结候选，因为当前还没有 `std/collections.ax` 源码模块
    - `foundation/search.ax`、`foundation/file_kind.ax` 的 searchable/markdown 分类、`foundation/workspace.ax` 的 `append_named_line` 和目录重建策略继续孵化
    - `docs/foundation-inventory.md` 已更新为 P3 迁移后的事实：部分 helper 已下沉到 `std.*`，未验证充分的接口继续留在 Std-0
    - `WORKLIST.md` 已把下一步推进到 Std-1 候选验证入口，而不是继续默认迁移更多样例

- `A-2026-04-27-20` `[P3]` 第五组 `std.*` 迁移试点 command batch 落地
  - 结果：
    - `examples/project_command_batch/` 已从 `../../foundation + lib` 迁移到 `../../std + lib`
    - `src/main.ax` 已通过 `std.cli / std.fs / std.path / std.process / std.env` 承载入口校验、输出目录创建、命令执行和环境变量读取
    - `lib.report` 已显式声明 `module lib.report`，并通过 `std.report / std.fs / std.text / std.workspace` 构造 batch 报告
    - `std.text` 新增 `trim` 薄包装，补齐文档已声明的文本接口
    - `tests/interface_snapshots.rs` 已更新 command batch 的 build source 回归
    - `WORKLIST.md` 已把下一步推进到第一版 `std.*` 冻结候选清单收口，而不是继续默认迁移全仓样例

- `A-2026-04-27-19` `[P3]` `std.process / std.env` 第一刀与 command capture 试点落地
  - 结果：
    - 新增 `std/process.ax`，第一版只暴露 `run / run_in / capture_in`
    - 新增 `std/env.ax`，第一版只暴露 `has / get`
    - `examples/project_command_capture/` 已从 `../../foundation` 迁移到 `../../std`
    - `std.process.capture_in` 与 `std.env.has` 已进入真实命令捕获样例和 interface snapshots
    - `WORKLIST.md` 已把下一步推进到 `project_command_batch` 评估，而不是继续无边界扩 process API

- `A-2026-04-27-18` `[P3]` 第三组 `std.*` 迁移试点落地
  - 结果：
    - 扩展 `std/fs.ax` 的 `exists / remove_file / rename`
    - 扩展 `std/path.ax` 的 `extension`
    - `examples/project_release_promote/` 已从 `../../foundation` 迁移到 `../../std + lib`
    - `lib.receipt` 保留项目私有 receipt 逻辑，通用 fs/path/report/cli 能力下沉到 `std.*`
    - `tests/interface_snapshots.rs` 已更新 release promote 的运行和 build source 回归
    - `WORKLIST.md` 已把下一步推进到 `std.process / std.env` 宿主边界评估

- `A-2026-04-27-17` `[P3]` 第二组 `std.*` 迁移试点落地并修复递归调用栈问题
  - 结果：
    - 新增 `std/workspace.ax`
    - 扩展 `std/fs.ax` 的 `read_dir / file_size`
    - 扩展 `std/path.ax` 的 `parent / file_name / classify_file_kind / is_text_file`
    - `examples/project_directory_index/` 已从 `../../foundation` 迁移到 `../../std + lib`
    - 目录索引样例继续保留项目私有 `lib.index_totals / lib.report / lib.scan`，通用 workspace/path/fs/report 能力下沉到 `std.*`
    - `src/interpreter.rs` 新增轻量 declared-function 调用路径，避免递归用户函数每层都经过完整 builtin dispatch 栈帧
    - `tests/interface_snapshots.rs` 已更新 directory index 的运行和 build source 回归

- `A-2026-04-27-16` `[P3]` 第一组 `std.*` 源码模块与 text normalize 迁移试点落地
  - 结果：
    - 新增 `std/cli.ax`、`std/fs.ax`、`std/path.ax`、`std/report.ax`、`std/text.ax` 五个 AX 源码模块
    - `examples/project_text_normalize/` 已从 `../../foundation` 迁移到 `../../std + lib`
    - `lib.normalize` 与 `lib.report` 保留项目私有业务层，`src/main.ax` 通过 `std.*` 完成通用 CLI、文件、路径、文本和报告能力调用
    - `tests/interface_snapshots.rs` 已更新 build source tree 回归，运行夹具继续覆盖 text normalize 行为
    - `WORKLIST.md` 的当前主线已切到第二迁移试点评估，不再停留在“是否启动 P3”判断

- `A-2026-04-27-15` `[P0]` 根目录与 docs 入口口径收口
  - 结果：
    - `WORKLIST.md` 顶部当前主线已从旧的 P1 主攻口径改为 P0 收尾完成后的下一轮选择口径
    - `P0` 与 `P1` 出口清单已按当前事实标记完成
    - README 导航新增 validation matrix 与 interface contracts 入口
    - `W-P0-04` 已完成

- `A-2026-04-27-14` `[P0]` 外部接口契约与快照覆盖写清
  - 结果：
    - 新增 `docs/interface-contracts.md`
    - 把 diagnostics、AI diagnostics、runtime diagnostics、context、build manifest、repair export 的契约和对应 regression 写清
    - `docs/feature-matrix.md` 已更新为当前事实：context 已进入 repair export 输入链，P1 展示层已成立
    - `W-P0-03` 已完成

- `A-2026-04-27-13` `[P0]` 本机与 CI 验证矩阵写清
  - 结果：
    - 新增 `docs/validation-matrix.md`
    - 写清 Windows local、Windows CI、Ubuntu CI 分别跑哪些命令
    - 明确 PowerShell benchmark/orchestration 仍是 Windows-only，Ubuntu 只承诺 core compiler/runtime
    - quickstart、platform support、docs 入口已指向验证矩阵

- `A-2026-04-27-12` `[P1]` 公开口径边界收紧
  - 结果：
    - 新增 `docs/public-claims.md`，把“可直接说的仓库内事实”和“只能作为后续验证目标的外部结论”分开
    - README、PROJECT_FACTS、docs 入口和 benchmark showcase 已引用该边界
    - `W-P1-06` 已完成，公开表述不再把仓库内 replay 结果说成跨语言或 live-model 胜负结论

- `A-2026-04-27-11` `[P1]` benchmark 展示页升级为当前可引用事实页
  - 结果：
    - `docs/benchmark-showcase.md` 已更新到当前 `30` 个 full repair case
    - 展示页写清 `cold 23/30`、`base 25/30`、`ai 30/30` 的 deterministic replay 结果
    - 展示页补充 `-IncludeContext` 的 context-enabled export 链路
    - 展示页明确区分“仓库内已复现事实”和“跨语言 / live model 尚未完成对照”
    - README、PROJECT_FACTS、docs 入口已指向展示页

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
