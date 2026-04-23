# AX Worklist

最后更新：2026-04-23

状态说明：

- `[x]` 已完成
- `[~]` 进行中
- `[ ]` 待做

维护规则：

1. 这个文件放在项目根目录，作为当前施工清单。
2. 每完成一项，立刻修改本文件状态、日期和备注，并同步检查 [`规划.md`](C:/Users/xiaoy/Desktop/A语言/AX/规划.md) 是否需要更新主线或优先级。
3. 当前优先级以 `P0 > P1 > P2` 为准。

## 当前主线

当前主线不是“全面为自举让路”，而是：

1. 固定 benchmark 与性能基线
2. 固定 `check/json/ai` 接口与回归资产
3. 继续提高 AI 修复协议质量
4. 用工具风格样例倒逼下一批语言能力
5. 在证据支持下再推进更大的语言面和后端

说明：

- 自举仍然是长期方向，但不再作为当前 `P0` 的组织原则。
- Rust 种子实现的内部整理只在确实支撑主线时继续，不再单独作为主要推进目标。

## 已完成的关键里程碑

- [x] 已完成第一条可运行原型链：
  `axc check / run / ast / hir / mir / fmt / build`
- [x] 已完成结构化 diagnostics 与 `--json --ai`
- [x] 已完成 repair benchmark、comparison、smoke 与 CI 骨架
- [x] 已完成 `run --json` 与首批 runtime AI diagnostics
- [x] 已完成固定长度数组、结构体、枚举与解释器贯通
- [x] 已完成 `semantic` 的第一轮必要拆层，消除明显“大厨房”问题
- [x] 已新增首个工具风格 AX 示例：
  [`examples/bootstrap_token_scan.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/bootstrap_token_scan.ax)

## P0

- [x] `P0-21` 固定 benchmark 与性能基线
  - 目标：让三模式 benchmark 和最小性能测量成为稳定可重复的证据链。
  - 输入：现有 `compare-repair-modes.ps1`、`benchmark-diagnostics.ps1`、repair cases。
  - 输出：稳定报告节奏、最小性能基线、可复跑结果。
  - 通过条件：同一输入可重复跑出同结构报告；能看见 `check / check --json / check --json --ai` 的相对开销。
  - 回归保障：现有 benchmark smoke、CI。
  - 不做范围：不新增大语法特性。
  - 完成于：2026-04-23
  - 备注：`benchmark-diagnostics.ps1` 现在稳定落盘 `summary.json` / `summary.md`，包含三组固定 `pairwise_overhead`；已新增 `smoke-benchmark-diagnostics.ps1`、CI 入口与 [`docs/diagnostics-benchmark-schema.md`](C:/Users/xiaoy/Desktop/A语言/AX/docs/diagnostics-benchmark-schema.md)。

- [x] `P0-22` 固定 `check/json/ai` 接口与回归资产
  - 目标：让基础诊断层和 AI 增强层的外部接口可稳定消费、可快照、可比较。
  - 输入：现有 `Diagnostic` schema、`--json --ai` 输出、interface snapshots。
  - 输出：稳定 schema、快照、必要的接口说明。
  - 通过条件：基础层字段语义不漂移；AI 增强层只做增量扩展；快照稳定。
  - 回归保障：`tests/interface_snapshots.rs` 与相关示例。
  - 不做范围：不引入供应商定制 prompt 文案。
  - 完成于：2026-04-23
  - 备注：已补齐 `check --json` 成功路径快照、`check --json --ai` 成功路径快照，以及 `--ai-session` 从 `L1 -> L2` 的 CLI 级快照；[`docs/diagnostics-schema.md`](C:/Users/xiaoy/Desktop/A语言/AX/docs/diagnostics-schema.md) 也已同步说明成功输出与 session 升级语义。

- [~] `P0-23` AI 规则覆盖与 session 版本策略
  - 目标：把高频错误的 AI 修复反馈变成可回归资产，而不是临时文案。
  - 输入：现有 `ai.rs`、规则卡、session 文件、examples 与 benchmark cases。
  - 输出：规则覆盖测试、session version 策略、更多稳定 `rule_id` 映射。
  - 通过条件：高频错误能稳定命中规则；session 行为可预期；相同输入 + 相同 session 输出一致。
  - 回归保障：接口快照、AI 规则测试、repair smoke。
  - 不做范围：不追求一次性覆盖全部错误码。

- [~] `P0-24` 文档-实现一致性清理
  - 目标：让 `README`、[`SYNTAX.md`](C:/Users/xiaoy/Desktop/A语言/AX/SYNTAX.md)、[`PLAN.md`](C:/Users/xiaoy/Desktop/A语言/AX/PLAN.md) 和当前实现不互相打架。
  - 输入：现有命令行为、示例、文档。
  - 输出：一致性清单与修正文档。
  - 通过条件：已支持 / 未支持 / guidance-only 条目明确且一致。
  - 回归保障：文档人工审计与示例 smoke。
  - 不做范围：不在文档中提前承诺未落地能力。

- [~] `P0-25` 建立首批工具风格样例集合
  - 目标：不只做一个示例，而是形成一小组能反映“AX 是否适合写工具”的样例。
  - 输入：当前语法能力、现有 examples、benchmark 观察。
  - 输出：2-4 个工具风格样例与对应 smoke。
  - 通过条件：至少覆盖 token 扫描、状态机、计数或简单格式化中的两类。
  - 回归保障：样例可 `check`、可 `run`，后续可纳入 smoke。
  - 不做范围：不要求现在就上文件 IO 或完整编译器模块。

- [ ] `P0-26` 根据样例与 benchmark 排能力缺口
  - 目标：把“接下来补什么”从感觉变成证据。
  - 输入：工具风格样例、benchmark 结果、AI 生成修复体验。
  - 输出：明确的下一批能力缺口排序。
  - 通过条件：能回答“先补什么最值”。
  - 回归保障：同步更新 [`规划.md`](C:/Users/xiaoy/Desktop/A语言/AX/规划.md)。
  - 不做范围：不立即把所有缺口都实现。

- [x] `P0-27` 补 `run` 结构化错误输出
  - 目标：让运行期失败也能进入结构化诊断和后续 AI 修复链。
  - 输入：现有 `axc run` 文本错误、runtime diagnostics、`--json` 输出约定。
  - 输出：更稳定的 `run` 结构化错误输出与对应测试。
  - 通过条件：常见运行期失败可输出结构化字段，且不破坏基础快路径。
  - 回归保障：interface snapshots、runtime 相关示例与 smoke。
  - 不做范围：不追求一次性覆盖所有运行期异常。
  - 完成于：2026-04-23
  - 备注：已补强高频 runtime error 的基础 `notes/suggestion`，并新增 `run_division_by_zero.json` 快照覆盖基础 JSON 契约。

## P1

- [ ] `P1-1` 切片路线
- [ ] `P1-1a` 空数组字面量策略
- [ ] `P1-2` 更实用的字符串处理
- [ ] `P1-3` 更贴近工具链代码的遍历能力
- [ ] `P1-4` 更多高价值错误的 AI 教学规则
- [ ] `P1-5` 更多工具风格坏例子与修复样例

## P2

- [ ] `P2-1` 更大语法扩展
- [ ] `P2-2` 原生后端深化
- [ ] `P2-3` 更完整标准库
- [ ] `P2-4` AX 局部重写前端子集
