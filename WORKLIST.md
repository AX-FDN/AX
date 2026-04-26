# AX Worklist

> 阅读提示：本文件是当前施工清单，不是项目概览，也不是能力宣传页。  
> 如果你想先看“AX 现在到底做到了什么”，请先看 [`PROJECT_FACTS.md`](./PROJECT_FACTS.md) 和 [`docs/feature-matrix.md`](./docs/feature-matrix.md)；本文件主要回答“我们此刻正在做什么”。

最后更新：2026-04-25

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
4. 把 full repair benchmark 与 compare replay 资产做厚
5. 把现有五类 project-backed 样例固化成代表集，并接进更硬的验证链
6. 用代表样例决定下一批缺口，再推进宿主能力补口、更大的语言面或后端

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
- [x] 已补对外事实层文档：
  [`PROJECT_FACTS.md`](C:/Users/xiaoy/Desktop/A语言/AX/PROJECT_FACTS.md) 与 [`docs/feature-matrix.md`](C:/Users/xiaoy/Desktop/A语言/AX/docs/feature-matrix.md)
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

- [x] `P0-23` AI 规则覆盖与 session 版本策略
  - 目标：把高频错误的 AI 修复反馈变成可回归资产，而不是临时文案。
  - 输入：现有 `ai.rs`、规则卡、session 文件、examples 与 benchmark cases。
  - 输出：规则覆盖测试、session version 策略、更多稳定 `rule_id` 映射。
  - 通过条件：高频错误能稳定命中规则；session 行为可预期；相同输入 + 相同 session 输出一致。
  - 回归保障：接口快照、AI 规则测试、repair smoke。
  - 不做范围：不追求一次性覆盖全部错误码。
  - 完成于：2026-04-23
  - 备注：已新增 `tests/interface_snapshots.rs` 的 manifest 驱动 CLI 级规则回归测试，直接校验 [`benchmarks/repair-cases.json`](C:/Users/xiaoy/Desktop/A语言/AX/benchmarks/repair-cases.json) 中的 `expected_codes` 与 `expected_ai_rule_ids`；同时补了 `--ai-session` 落盘状态的版本化断言，确保外部接口层也固定 `version=1`、`repeat_count` 与 `last_teaching_level` 的基本语义。

- [x] `P0-24` 文档-实现一致性清理
  - 目标：让 `README`、[`SYNTAX.md`](C:/Users/xiaoy/Desktop/A语言/AX/SYNTAX.md)、[`PLAN.md`](C:/Users/xiaoy/Desktop/A语言/AX/PLAN.md) 和当前实现不互相打架。
  - 输入：现有命令行为、示例、文档。
  - 输出：一致性清单与修正文档。
  - 通过条件：已支持 / 未支持 / guidance-only 条目明确且一致。
  - 回归保障：文档人工审计与示例 smoke。
  - 不做范围：不在文档中提前承诺未落地能力。
  - 完成于：2026-04-23
  - 备注：已统一 `docs/repair-adapter-spec.md`、`docs/README.md` 与当前三模式 benchmark 事实，明确 runner 合约覆盖 `cold/base/ai`；同时补齐 [`详细介绍.md`](C:/Users/xiaoy/Desktop/A语言/AX/详细介绍.md) 对 `axc mir` 与 `axc build` 当前阶段含义的说明，避免把 build 骨架产物误读成成熟原生后端。

- [x] `P0-25` 建立首批工具风格样例集合
  - 目标：不只做一个示例，而是形成一小组能反映“AX 是否适合写工具”的样例。
  - 输入：当前语法能力、现有 examples、benchmark 观察。
  - 输出：2-4 个工具风格样例与对应 smoke。
  - 通过条件：至少覆盖 token 扫描、状态机、计数或简单格式化中的两类。
  - 回归保障：样例可 `check`、可 `run`，后续可纳入 smoke。
  - 不做范围：不要求现在就上文件 IO 或完整编译器模块。
  - 完成于：2026-04-23
  - 备注：已落地 [`examples/bootstrap_token_scan.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/bootstrap_token_scan.ax)、[`examples/bootstrap_state_machine.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/bootstrap_state_machine.ax) 与 [`examples/bootstrap_block_summary.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/bootstrap_block_summary.ax)，覆盖 token 扫描/计数、状态机、以及接近简单格式化/结构遍历的 block 总结三类工具风格逻辑；后两者都已加入 [`tests/interface_snapshots.rs`](C:/Users/xiaoy/Desktop/A语言/AX/tests/interface_snapshots.rs) 的运行回归。

- [x] `P0-26` 根据样例与 benchmark 排能力缺口
  - 目标：把“接下来补什么”从感觉变成证据。
  - 输入：工具风格样例、benchmark 结果、AI 生成修复体验。
  - 输出：明确的下一批能力缺口排序。
  - 通过条件：能回答“先补什么最值”。
  - 回归保障：同步更新 [`规划.md`](C:/Users/xiaoy/Desktop/A语言/AX/规划.md)。
  - 不做范围：不立即把所有缺口都实现。
  - 完成于：2026-04-23
  - 备注：已新增 [`能力缺口排序.md`](C:/Users/xiaoy/Desktop/A语言/AX/能力缺口排序.md)，先把工具风格样例、repair benchmark 与当前实现边界放到同一张表里做排序；当时结论是优先推进切片、更实用的字符串处理、空数组字面量策略与更贴近工具代码的遍历能力。这一轮能力已在 2026-04-24 基本落地，当前主线已顺延为 benchmark 代表性、最小工具宿主能力与真实工具样例。

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

- [x] `P1-1` 切片路线
  - 目标：把只读 slice 从语法、语义、解释执行和文档一路打通，解除固定长度数组对真实工具样例的硬绑定。
  - 输入：[`能力缺口排序.md`](C:/Users/xiaoy/Desktop/A语言/AX/能力缺口排序.md) 的第一优先级结论、现有数组链路、tool-style examples。
  - 输出：`[Type]` 切片类型、`values[start:end]` 切片表达式、数组到切片形参的兼容、解释器切片读取、示例与测试。
  - 通过条件：slice 全链路可 `check` / `run` / `fmt` / `hir` / `mir`；数组可传给 slice 参数；slice 保持只读；接口与示例测试稳定。
  - 回归保障：parser / semantic / hir / mir / interpreter / formatter 单测，`tests/interface_snapshots.rs` 新增 `examples/slices.ax` 运行回归。
  - 不做范围：不引入可变 slice、borrow 系统、可省略边界的切片语法或集合泛型。
  - 完成于：2026-04-24
  - 备注：第一版收敛为只读切片；当前已支持 `[Type]`、`values[start:end]`、slice 索引读取和数组传 slice 形参，新增 [`examples/slices.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/slices.ax)。
- [x] `P1-1a` 空数组字面量策略
  - 目标：给 `[]` 一个稳定、可诊断、可被 AI 理解的实现边界，避免它长期悬空在 unsupported 与半套推导之间。
  - 输入：[`能力缺口排序.md`](C:/Users/xiaoy/Desktop/A语言/AX/能力缺口排序.md) 的第三优先级结论、现有数组类型系统、repair benchmark case。
  - 输出：空数组字面量的明确策略、语义检查、AI 规则更新、示例与回归测试。
  - 通过条件：`[]` 只在显式零长度数组上下文中通过；非零长度上下文稳定报 `S0032`；AI 规则与 benchmark 口径一致。
  - 回归保障：semantic 单测、`tests/interface_snapshots.rs` 的 `examples/empty_array.ax` 运行回归、repair cases 规则断言。
  - 不做范围：不引入通用空数组推导、泛型集合字面量或隐式长度补全。
  - 完成于：2026-04-24
  - 备注：当前策略已收敛为 `let values: [i32; 0] = [];` 合法，`let values: [i32; 1] = [];` 稳定报 `S0032`；已新增 [`examples/empty_array.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/empty_array.ax) 并同步 AI 规则文案。
- [x] `P1-2` 更实用的字符串处理
  - 目标：补上不重设计但马上有用的字符串能力，让 AX 能更自然地拼消息和做最小文本统计。
  - 输入：[`能力缺口排序.md`](C:/Users/xiaoy/Desktop/A语言/AX/能力缺口排序.md) 的第二优先级结论、现有字符串字面量与 `println`。
  - 输出：`string + string`、内置 `string_len(text)`、字符串工具示例与回归测试。
  - 通过条件：字符串拼接与长度查询可 `check` / `run`；语义与运行期行为一致；示例与接口测试稳定。
  - 回归保障：semantic / interpreter 单测，`tests/interface_snapshots.rs` 新增 `examples/string_tools.ax` 运行回归。
  - 不做范围：不引入字符串索引、Unicode 语义设计、格式化模板或完整文本库。
  - 完成于：2026-04-24
  - 备注：第一版收敛为“拼接 + 长度”；当前已支持 `string + string` 和 `string_len(text)`，新增 [`examples/string_tools.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/string_tools.ax)。
- [x] `P1-3` 更贴近工具链代码的遍历能力
  - 目标：在不引入新循环语法的前提下，先把数组 / 切片 / 字符串的统一长度查询打通，让真实工具逻辑能用现有 `for` 写出稳定遍历。
  - 输入：已落地的固定长度数组、只读切片、字符串能力，以及 [`能力缺口排序.md`](C:/Users/xiaoy/Desktop/A语言/AX/能力缺口排序.md) 的下一优先级结论。
  - 输出：统一内置 `len(value)`、遍历示例、语义/解释器/接口回归测试。
  - 通过条件：`len` 可作用于 `string`、固定长度数组与切片；现有 C 风格 `for` 可自然遍历 slice；示例与测试稳定通过。
  - 回归保障：semantic / interpreter 单测，`tests/interface_snapshots.rs` 的 [`examples/traversal.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/traversal.ax) 运行回归。
  - 不做范围：不引入 `for item in values`、迭代器协议、可变切片或完整集合库。
  - 完成于：2026-04-24
  - 备注：第一版收敛为统一 `len(value)`；当前已支持 `len(string)`、`len([T; N])` 与 `len([T])`，新增 [`examples/traversal.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/traversal.ax) 展示基于切片的遍历模式。
- [x] `P1-4` 更多高价值错误的 AI 教学规则
  - 目标：把切片与统一长度 helper 相关的高频错误也纳入稳定 `rule_id`，让新补的语言能力同样具备可回归的 AI 修复反馈。
  - 输入：现有 `src/ai.rs` 规则表、切片/遍历能力、repair benchmark manifest。
  - 输出：新增高价值 AI 规则、对应坏例子、repair benchmark case 与稳定性测试。
  - 通过条件：`len` 参数类型错误、非序列切片、切片写入三类错误都能稳定映射到专用 `rule_id`；manifest 回归测试通过。
  - 回归保障：`src/ai.rs` 单测、`tests/interface_snapshots.rs` 的 repair manifest 断言、`benchmarks/repair-cases.json`。
  - 不做范围：不引入厂商特定 prompt 文案，不追求一次覆盖所有剩余诊断码。
  - 完成于：2026-04-24
  - 备注：已新增 `len_builtin_requires_countable_value`、`slice_base_must_be_array_or_slice`、`slice_values_are_read_only` 三个稳定规则，并补充 [`examples/len_non_countable.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/len_non_countable.ax)、[`examples/non_slice_base.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/non_slice_base.ax)、[`examples/slice_assignment.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/slice_assignment.ax) 进入 repair case 集。
- [x] `P1-5` 更多工具风格坏例子与修复样例
  - 目标：把新落地的切片、`len(value)` 和只读视图规则补进 repair benchmark 资产，而不是只停留在单个示例文件。
  - 输入：现有工具风格样例、repair manifest、replay candidate 目录和 smoke 脚本。
  - 输出：更多工具风格坏例子、对应修复候选、更新后的 smoke manifest 与 compare smoke 基线。
  - 通过条件：新 case 能进入 full/smoke manifest；shared/base/cold replay 资产齐全；repair smoke 与 compare smoke 稳定通过。
  - 回归保障：`benchmarks/repair-cases*.json`、`benchmarks/repair-candidates/**`、`scripts/smoke-*.ps1`、`.\scripts\cargo-gnu.ps1 test`。
  - 不做范围：不接入新的模型供应商，不引入新的语言特性，只补 benchmark 与修复资产。
  - 完成于：2026-04-24
  - 备注：已把 [`examples/len_non_countable.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/len_non_countable.ax)、[`examples/non_slice_base.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/non_slice_base.ax)、[`examples/slice_assignment.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/slice_assignment.ax) 接入 full/smoke manifest，并补齐 [`benchmarks/repair-candidates/smoke`](C:/Users/xiaoy/Desktop/A语言/AX/benchmarks/repair-candidates/smoke)、[`benchmarks/repair-candidates/compare/base`](C:/Users/xiaoy/Desktop/A语言/AX/benchmarks/repair-candidates/compare/base) 与 [`benchmarks/repair-candidates/compare/cold`](C:/Users/xiaoy/Desktop/A语言/AX/benchmarks/repair-candidates/compare/cold) 的 replay 资产；compare smoke 现稳定验证 8 个 case、`cold -> base -> ai` 三模式差异。
- [x] `P1-6` 最小格式化能力
  - 目标：在不引入完整格式化 DSL 的前提下，先让 AX 能把运行时值稳定转成字符串，支撑工具式报告和摘要拼接。
  - 输入：现有 `string + string`、`len(value)`、运行时 `display` 逻辑与工具风格样例需求。
  - 输出：统一内置 `to_string(value)`、工具风格示例、语义/解释器/接口回归测试。
  - 通过条件：`to_string` 可用于具体运行时值并返回 `string`；工具示例能拼出结构化报告；现有链路不回退。
  - 回归保障：`src/semantic.rs`、`src/interpreter.rs`、`tests/interface_snapshots.rs`、[`examples/format_report.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/format_report.ax)。
  - 不做范围：不引入格式化模板、占位符语法、字符串插值或完整文本库。
  - 完成于：2026-04-24
  - 备注：当前已支持 `to_string(value)` 把数字、布尔、枚举、结构体、数组和切片等具体运行时值转为 `string`；新增 [`examples/format_report.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/format_report.ax) 作为最小工具报告样例。

- [x] `P1-7` 嵌套可写路径
  - 目标：把“可变变量 / 字段 / 数组元素”这三种分散写法收敛成真正可用的可写路径，让 AX 能表达 `outer.inner.value = expr;` 和 `tokens[index].value = expr;` 这类工具代码常见更新逻辑。
  - 输入：现有结构体、固定长度数组、只读切片、HIR/MIR place lowering 与解释器赋值链路。
  - 输出：递归 place lowering、嵌套赋值语义检查、解释器嵌套写入支持、工具风格样例与回归测试。
  - 通过条件：嵌套字段路径和“数组元素中的字段”路径都可 `check` / `run`；只读切片上的嵌套写入继续稳定拒绝；现有 AI 规则与接口测试不回退。
  - 回归保障：`src/hir.rs`、`src/mir.rs`、`src/semantic.rs`、`src/interpreter.rs` 单测与 `.\scripts\cargo-gnu.ps1 test` 全量回归。
  - 不做范围：不引入可变切片、借用系统、引用类型或更大集合抽象。
  - 完成于：2026-04-24
  - 备注：当前已支持 `name.field.other = expr;`、`name[index] = expr;` 与 `name[index].field = expr;` 三类递归可写路径；新增 [`examples/token_rewrite.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/token_rewrite.ax) 作为最小工具式数据重写样例。
- [x] `P1-8` 最小循环提前退出
  - 目标：补上最小但高频的循环控制，让工具代码不再依赖 `index = limit;` 这类机械退出写法。
  - 输入：现有 `while` / `for`、解释器控制流、HIR/MIR lowering 与工具风格样例中的循环退出痛点。
  - 输出：`break;` 语法、循环内语义检查、HIR/MIR/解释器支持、示例与回归测试。
  - 通过条件：`break;` 可在 `while` 和 `for` 中稳定提前退出；循环外使用稳定报错；现有 `for -> while` lowering 与接口测试不回退。
  - 回归保障：`src/lexer.rs`、`src/parser.rs`、`src/formatter.rs`、`src/semantic.rs`、`src/hir.rs`、`src/mir.rs`、`src/interpreter.rs` 单测与 `.\scripts\cargo-gnu.ps1 test` 全量回归。
  - 不做范围：本轮只收 `break;`，`continue;` 的 `for` step 语义改在后续独立条目里处理。
  - 完成于：2026-04-24
  - 备注：当前已支持 `break;` 提前退出最近一层 `while` 或 `for`；新增 [`examples/break_loop.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/break_loop.ax)，并把 [`examples/bootstrap_block_summary.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/bootstrap_block_summary.ax) 的机械退出写法替换为真实 `break;`。

## P1（当前待做）

- [x] `P1-9` 扩 full repair benchmark 资产与 compare replay 基线
  - 目标：把 repair evidence 从“10-case smoke 子集稳定”推进到“full manifest 更完整、compare replay 更接近真实对比基线”。
  - 输入：现有 [`benchmarks/repair-cases.json`](C:/Users/xiaoy/Desktop/A语言/AX/benchmarks/repair-cases.json)、[`benchmarks/repair-cases-smoke.json`](C:/Users/xiaoy/Desktop/A语言/AX/benchmarks/repair-cases-smoke.json)、[`benchmarks/repair-candidates/`](C:/Users/xiaoy/Desktop/A语言/AX/benchmarks/repair-candidates/) 与 compare/smoke 脚本。
  - 输出：扩容后的 full manifest、可重放的 full compare shared replay 资产、必要的 docs 与回归更新。
  - 通过条件：full export 稳定；新增 case 的 `expected_codes / expected_ai_rule_ids` 固定；compare replay 不再只围绕 smoke 子集组织。
  - 回归保障：`tests/interface_snapshots.rs`、`scripts/smoke-repair-benchmark.ps1`、`scripts/smoke-compare-repair-feedback.ps1`、`scripts/smoke-compare-repair-modes.ps1`。
  - 不做范围：不在这一项里引入新的模型供应商，不把 smoke 子集和 full 基线混为一谈。
  - 完成于：2026-04-24
  - 备注：已把 full manifest 从 24 case 扩到 27 case，先新增 `array_index_type_mismatch` 与 `return_type_mismatch_main`，随后再补入首个仓库内 project-backed repair case `project_helper_missing_semicolon`；同时补齐 [`benchmarks/repair-candidates/compare/shared`](C:/Users/xiaoy/Desktop/A语言/AX/benchmarks/repair-candidates/compare/shared) 的 full replay 基线、[`benchmarks/repair-candidates/smoke`](C:/Users/xiaoy/Desktop/A语言/AX/benchmarks/repair-candidates/smoke) 的 project-backed replay 资产，并新增 full shared score 回归与 compare 说明文档。

- [~] `P1-10` 最小工具宿主能力第一批
  - 目标：给 AX 补上写真实 CLI / 文本处理 / 构建辅助程序的最低可用宿主能力，而不是继续优先补大语法面。
  - 输入：[`PLAN.md`](C:/Users/xiaoy/Desktop/A语言/AX/PLAN.md) 的任务族定义、现有工具风格样例、[`能力缺口排序.md`](C:/Users/xiaoy/Desktop/A语言/AX/能力缺口排序.md) 的最新排序。
  - 输出：围绕 `process / env / path / fs / string` 的第一批稳定接口、对应 diagnostics / AI guidance / 示例 / 文档 / 测试。
  - 通过条件：至少一批真实工具风格程序不再只依赖纯玩具输入；误用这些接口时仍能给出稳定结构化反馈。
  - 回归保障：语义 / 解释器 / 接口测试、真实样例 smoke、repair benchmark 资产。
  - 不做范围：不一口气做大而全标准库，不先做网络、GUI、并发运行时或包生态。
  - 进展：2026-04-25 已通过 [`examples/project_command_capture/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_command_capture/)、[`examples/project_release_promote/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_release_promote/)、[`examples/project_directory_index/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_directory_index/)、[`examples/project_command_batch/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_command_batch/) 与 [`examples/project_text_normalize/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_text_normalize/) 把 `process / env / path / fs / string` 这批接口压过第一轮。
  - 进展：2026-04-25 共享 AX 基础层 [`foundation/`](C:/Users/xiaoy/Desktop/A语言/AX/foundation/) 已沉淀 `cli / report / text / search / file_kind / workspace`，说明这一批能力不再只是零散样例调用，而是开始形成可复用边界。
  - 进展：2026-04-25 已为首批宿主运行期误用补上稳定 AI 规则卡与接口验证，当前覆盖 `fs_read_to_string` / `fs_file_size` 的可读文件要求、`fs_read_dir` 的可读目录要求，以及 `process_run_in` / `process_capture` 的常见启动失败与非零退出反馈。
  - 进展：2026-04-25 已把缺可读文件的宿主运行期坏例子接入 full repair benchmark，并为其补上共享修复候选与 full-manifest 回归测试，说明这批宿主反馈已经进入可重复证据链。
  - 进展：2026-04-25 已继续把 `process_capture` 非零退出坏例子接入 full repair benchmark，并补上共享修复候选；当前 full manifest 已能回归两类宿主 runtime 修复：缺可读文件与 capture 非零退出。
  - 进展：2026-04-25 已继续把 `fs_read_dir` 缺可读目录坏例子接入 full repair benchmark，并补上共享修复候选；当前 full manifest 已能回归三类宿主 runtime 修复：缺可读文件、缺可读目录与 capture 非零退出。
  - 进展：2026-04-25 已修复 Windows 下 `env_has/env_get` 对宿主环境变量大小写敏感导致的误判；解释器现在会在 Windows 上按大小写不敏感方式查询环境变量，并补上 `src/interpreter.rs` 单测与 [`tests/interface_snapshots.rs`](C:/Users/xiaoy/Desktop/A语言/AX/tests/interface_snapshots.rs) 的 `project_command_batch` 回归验证。
  - 进展：2026-04-25 已补强 Windows 下 benchmark / interface snapshot 的 PowerShell 运行链：临时 runner 脚本现在按 BOM 方式落盘，benchmark 导出/打分脚本也改为用可清理的临时 `axc` 副本执行，避免中文路径和可执行文件锁导致的伪失败。
  - 当前剩余收口：把接口边界、误用 diagnostics、AI guidance 与代表样例验证链一起收稳，而不是继续无上限加新宿主 API。

- [x] `P1-11` 固化代表样例集并接入更硬验证链
  - 目标：把现有五类 project-backed 真实工具样例收成“可验收的代表资产”，让后续 `axc build` 和下一批能力决策都建立在这些样例上，而不是继续按启动期逻辑新增样例品类。
  - 输入：现有五类 project-backed 样例、共享 [`foundation/`](C:/Users/xiaoy/Desktop/A语言/AX/foundation/)、`axc build` 骨架、interface snapshots 与当前 smoke。
  - 输出：核心代表集、宿主能力验证集、更硬的 `check / run / build / smoke` 验证链，以及一份由代表样例驱动的下一缺口判断。
  - 通过条件：核心代表样例可稳定 `check / run` 并进入更硬的 `build / smoke`；宿主能力验证样例可继续压 `process / env / path / fs`；能明确说出下一步更值钱的是更深目录遍历、最小 collections，还是 `import / module`。
  - 回归保障：`examples/`、[`tests/interface_snapshots.rs`](C:/Users/xiaoy/Desktop/A语言/AX/tests/interface_snapshots.rs)、必要的 smoke 与 build 回归。
  - 不做范围：不把样例扩成完整产品，不为了凑样例继续发明第六类、第七类 project 品类。
  - 进展：2026-04-24 已新增 [`examples/workspace_audit.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/workspace_audit.ax)、[`examples/docs_release_snapshot.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/docs_release_snapshot.ax) 与 [`examples/workspace_search_report.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/workspace_search_report.ax)，三者均已完成 `check / run` 验证。
  - 进展：2026-04-24 已新增项目化样例 [`examples/project_split/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_split/)、[`examples/project_foundation_report/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_foundation_report/)、[`examples/project_docs_release/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_docs_release/)、[`examples/project_workspace_audit/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_workspace_audit/) 与 [`examples/project_workspace_search_report/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_workspace_search_report/)，并把 `AX.toml` 的 `sources` 扩到支持目录，把 `axc build` 扩到导出 `project-sources/` 源树快照，把 repair benchmark export/score 扩到支持“项目上下文 + 单文件修复目标”的 project-backed case。
  - 进展：2026-04-25 已完成共享库目录迁移：`examples/project_*` 的 `AX.toml` 全部改为引用 repo 根目录 `../../foundation`，旧 `examples/foundation/*.ax` 已退场；共享 [`foundation/cli.ax`](C:/Users/xiaoy/Desktop/A语言/AX/foundation/cli.ax)、[`foundation/report.ax`](C:/Users/xiaoy/Desktop/A语言/AX/foundation/report.ax)、[`foundation/search.ax`](C:/Users/xiaoy/Desktop/A语言/AX/foundation/search.ax)、[`foundation/file_kind.ax`](C:/Users/xiaoy/Desktop/A语言/AX/foundation/file_kind.ax) 与 [`foundation/workspace.ax`](C:/Users/xiaoy/Desktop/A语言/AX/foundation/workspace.ax) 已开始承接重复 helper。
  - 进展：2026-04-25 已形成五类 project-backed 真实工具样例：
    - [`examples/project_command_capture/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_command_capture/)
    - [`examples/project_release_promote/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_release_promote/)
    - [`examples/project_directory_index/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_directory_index/)
    - [`examples/project_command_batch/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_command_batch/)
    - [`examples/project_text_normalize/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_text_normalize/)
  - 进展：2026-04-25 已把五类 project-backed 样例全部接进 `build` 级接口回归，新增 [`tests/interface_snapshots.rs`](C:/Users/xiaoy/Desktop/A语言/AX/tests/interface_snapshots.rs) 中的五个真实 example build 测试；同时新增 [`scripts/smoke-project-representatives.ps1`](C:/Users/xiaoy/Desktop/A语言/AX/scripts/smoke-project-representatives.ps1)，把 `project_directory_index / project_text_normalize / project_release_promote` 作为核心代表集、`project_command_capture / project_command_batch` 作为宿主能力验证集，固定成一条可复跑的代表项目 build smoke。
  - 完成于：2026-04-25

- [x] `P1-12` 用代表样例重排下一缺口
  - 目标：不靠感觉决定下一步，而是让代表样例和共享 foundation 边界来回答“下一步到底该补什么”。
  - 输入：代表样例运行结果、现有 benchmark 资产、[`能力缺口排序.md`](C:/Users/xiaoy/Desktop/A语言/AX/能力缺口排序.md)、当前 foundation 边界。
  - 输出：更新后的缺口排序结论，以及同步后的 [`规划.md`](C:/Users/xiaoy/Desktop/A语言/AX/规划.md) / [`WORKLIST.md`](C:/Users/xiaoy/Desktop/A语言/AX/WORKLIST.md)。
  - 通过条件：能明确指出是更深目录遍历、最小 collections，还是 `import / module` 更值；并能列出是哪几个代表样例暴露了该缺口。
  - 回归保障：文档同步审计、代表样例 smoke、必要的 interface snapshot 更新。
  - 不做范围：不在排序阶段顺手把下一批大能力一起实现。
  - 完成于：2026-04-25
  - 备注：已把 [`examples/project_directory_index/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_directory_index/) 与 [`examples/project_workspace_search_report/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_workspace_search_report/) 推到递归目录遍历，证明“更深目录遍历”在当前 AX 上可以用现有函数递归 + 共享 foundation helper 落地，不再是下一编译器主缺口。当前新排序已收敛为：`最小 collections > import / module > 继续样例侧递归遍历增强`。

- [x] `P1-13` 最小 collections 第一批
  - 目标：补上代表样例已经开始真实需要、但当前只能靠“边遍历边拼字符串”勉强绕开的最小集合能力。
  - 输入：[`examples/project_directory_index/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_directory_index/)、[`examples/project_text_normalize/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_text_normalize/)、[`examples/project_command_batch/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_command_batch/) 与最新的 [`能力缺口排序.md`](C:/Users/xiaoy/Desktop/A语言/AX/能力缺口排序.md)。
  - 输出：最小 collections 方案边界、第一版实现、误用 diagnostics / AI guidance、代表样例验证与文档同步。
  - 通过条件：至少一类真实工具样例可以不再依赖“纯流式字符串累积”来表达动态条目聚合；新能力不破坏 `check/json/ai`、build 打包与代表样例 smoke。
  - 回归保障：语义 / 解释器 / interface snapshots、代表样例 smoke、必要时补 repair case。
  - 不做范围：不直接扩成大而全容器库，不一次引入 map/set/iterator 全家桶。
  - 完成于：2026-04-25
  - 备注：第一版刻意收敛为内建 `string_list`，只解决“动态收集字符串条目”这一类最常见工具场景；当前已落地 `string_list_new / string_list_push / string_list_join`、`len(string_list)`、[`examples/string_list.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/string_list.ax) 与 [`examples/project_workspace_search_report/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_workspace_search_report/) 的真实接入，证明代表样例不再只能靠纯流式字符串累积来维护匹配明细。

- [x] `P1-14` `import / module` 进入条件复查
  - 目标：在不急着实现模块系统的前提下，先回答“`import / module` 现在是不是已经到了该进主线的时候”。
  - 输入：共享 [`foundation/`](C:/Users/xiaoy/Desktop/A语言/AX/foundation/)、现有 project-backed 代表样例、[`能力缺口排序.md`](C:/Users/xiaoy/Desktop/A语言/AX/能力缺口排序.md)、当前 `AX.toml + sources` 组织方式。
  - 输出：一份明确的进入条件判断，以及同步后的 [`规划.md`](C:/Users/xiaoy/Desktop/A语言/AX/规划.md) / [`WORKLIST.md`](C:/Users/xiaoy/Desktop/A语言/AX/WORKLIST.md)。
  - 通过条件：能明确判断 `import / module` 是否已经压过“第二批 minimal collections”，并列出支持这一判断的实际项目证据。
  - 回归保障：文档同步审计；代表样例与共享 foundation 的人工复核。
  - 不做范围：不在这一项里直接实现模块语法、包系统或可见性规则。
  - 完成于：2026-04-25
  - 备注：复查结论已从“继续观察”升级为“下一主阻塞”。当前共享 foundation 已沉淀 `cli / report / text / search / file_kind / workspace` 六块 helper，七个 project-backed 工程稳定采用 `sources = ["../../foundation", "lib"]`；同时 [`examples/project_directory_index/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_directory_index/)、[`examples/project_workspace_audit/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_workspace_audit/)、[`examples/project_docs_release/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_docs_release/) 与 [`examples/project_workspace_search_report/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_workspace_search_report/) 已进入 3-4 个 `.ax` 文件、约 107-122 行的量级，`build_summary / build_receipt / build_report / display_label / search_text` 等重复 helper 名称说明扁平全局命名空间已经开始主要靠命名纪律维持。

- [x] `P1-15` `import / module` 最小方案与迁移边界
  - 目标：在不引入完整包系统的前提下，先冻结足以支撑共享 foundation 与多文件 project 的最小模块组织方案。
  - 输入：[`foundation/`](C:/Users/xiaoy/Desktop/A语言/AX/foundation/)、现有 `AX.toml + sources` 组织方式、project-backed 代表样例、[`能力缺口排序.md`](C:/Users/xiaoy/Desktop/A语言/AX/能力缺口排序.md) 的复查结论。
  - 输出：模块最小语法草案、文件到模块的映射规则、限定名 / 导入规则、与 `AX.toml + sources` 的兼容迁移边界，以及明确的不做范围。
  - 通过条件：能解释当前如何消解共享 foundation 与 project 私有库之间的命名冲突；现有 project-backed 工程有清晰迁移路径；不会把范围直接膨胀成完整包系统。
  - 回归保障：同步更新 [`规划.md`](C:/Users/xiaoy/Desktop/A语言/AX/规划.md)、[`能力缺口排序.md`](C:/Users/xiaoy/Desktop/A语言/AX/能力缺口排序.md) 与后续 `SYNTAX.md` 设计说明。
  - 不做范围：本项不直接落 parser / semantic / interpreter 实现，不引入包管理、远程依赖或复杂可见性系统。
  - 完成于：2026-04-25
  - 备注：已冻结第一版最小模块设计，详见 [`docs/import-module-minimal-design.md`](C:/Users/xiaoy/Desktop/A语言/AX/docs/import-module-minimal-design.md)。当前结论是：继续保留 `AX.toml + sources` 做文件发现，支持文件走“一文件一模块”，入口文件显式 `import`，跨模块名称默认走全限定路径；同时明确第一刀不做 alias / wildcard / `pub` / 包管理，并把该设计同步回 [`SYNTAX.md`](C:/Users/xiaoy/Desktop/A语言/AX/SYNTAX.md)。

- [~] `P1-16` `import / module` parser / resolver / diagnostics 第一刀
  - 目标：把已经冻结的最小模块方案真正落到 parser / resolver / 诊断主链上，并迁一个代表 project 样例验证。
  - 输入：[`docs/import-module-minimal-design.md`](C:/Users/xiaoy/Desktop/A语言/AX/docs/import-module-minimal-design.md)、现有 `AX.toml + sources` 工程模型、共享 [`foundation/`](C:/Users/xiaoy/Desktop/A语言/AX/foundation/) 与代表 project 样例。
  - 输出：`module` / `import` 语法支持、源文件到模块路径映射、模块注册与首批专用 diagnostics、一条迁移后的 project-backed 验证样例。
  - 通过条件：最小模块项目可稳定 `check`；模块路径与文件路径错配、重复模块、导入缺失等错误有稳定 diagnostics；现有非模块项目保持兼容。
  - 回归保障：parser / resolver / interface snapshots、迁移样例 `check / run`、必要的 AI rule 与 benchmark 资产补充。
  - 不做范围：本项不扩成完整包系统，不做 alias / wildcard / `pub` / 远程依赖，不顺手重写整套工程模型。
- 进展：2026-04-25 已接上 `module` / `import` 词法与 parser，`Program.source_units` 现可保留每个源文件的 header 元信息；同时 `Project` 已能从 `AX.toml + sources` 推导 support source 的期望模块路径并拒绝重复根别名。
- 进展：2026-04-25 已补第一批模块语义诊断：support source 缺少 `module`、模块路径与文件路径错配、重复模块、重复 `import`、导入不存在模块、跨模块引用缺少 `import`；并新增迁移样例 [`examples/project_module_smoke/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/project_module_smoke/) 与 CLI `check` 回归。
- 进展：2026-04-25 已补 qualified function/type/enum 的 HIR lowering 兼容，`examples/project_module_smoke/` 现已可稳定走到 `axc hir` 与 `axc run`；剩余收尾重点转为 AI rule/benchmark 资产同步，以及更完整的模块模式回归覆盖。
- 进展：2026-04-25 已把 AI 反馈层对齐当前模块实现，移除旧的 “import/module 尚未支持” guidance，改为稳定覆盖 `S0037`-`S0043` 模块诊断；并补上 project 级 AI rule 测试，确认缺少 `module` 声明、缺少 `import`、导入不存在模块等场景都有稳定 `rule_id`。
- 进展：2026-04-25 已把 repair benchmark 的旧模块占位 case 同步到当前实现：`examples/import_unsupported.ax` / `examples/module_unsupported.ax` 现改为最小模块模式误用样例，manifest 预期也切到 `S0042` / `S0037` 与对应 `rule_id`；同时更新了 [`docs/import-module-minimal-design.md`](C:/Users/xiaoy/Desktop/A语言/AX/docs/import-module-minimal-design.md) 和 benchmark prompt 文案，去掉“modules/imports 尚未实现”的过时说法。
- 进展：2026-04-25 已给这批模块诊断和首批高价值 `S0022` 变体补上稳定 `DiagnosticKind`，`src/ai.rs` 现会优先按内部语义标签映射 `rule_id`，而不是继续把规则绑定在 `message.contains(...)` 上；新增 `stable_diagnostic_kinds_drive_rule_matching_without_old_message_text` 回归测试，并已通过 `cargo +stable-x86_64-pc-windows-gnu test --lib`。
- 进展：2026-04-25 已继续把 parser 侧高频 `P0001` 诊断接入稳定 `DiagnosticKind`，当前覆盖缺分号、缺右括号、缺右花括号与顶层声明错误；`src/ai.rs` 对这批规则也已优先走 kind 映射，不再继续把高价值修复入口绑死在文案特判上。
- 进展：2026-04-25 已让 parser 的详细提示生成也优先消费 `DiagnosticKind`，不再只靠 `message.contains(...)` 决定补充 note/suggestion；新增 “文案改掉但 kind 不变时，帮助信息仍保留” 的回归测试，并复跑 `cargo +stable-x86_64-pc-windows-gnu test --lib` 与两条 diagnostics interface snapshot。
- 进展：2026-04-25 又把缺右中括号、类型名缺失、表达式缺失也接进稳定 `DiagnosticKind`，并补上 `close_bracketed_construct` 规则卡；这说明 parser 高频基础错误现在已经不只是“能分类”，而是开始形成更完整的稳定 AI 修复入口。

- [x] `P1-17` 最小 `continue;` 语法全链路落地
  - 目标：在不破坏现有 `for -> while` lowering 语义的前提下，把 `continue;` 从缺失能力补成可执行能力。
  - 输入：现有 `while` / `for`、`break;` 控制流实现、HIR `for` lowering、AI kind-based rule 映射与工具风格样例。
  - 输出：`continue;` 词法 / parser / formatter / semantic / HIR / MIR / interpreter 支持，`for` 场景下的 step-before-continue 重写，AI 规则映射、样例、文档与接口回归。
  - 通过条件：`continue;` 可在 `while` 与 `for` 中稳定运行；循环外使用稳定报 `S0044`；`for` 中命中 `continue;` 时 step 仍会先执行；现有 `break;` 与模块/benchmark 回归不退化。
  - 回归保障：`cargo +stable-x86_64-pc-windows-gnu test --lib -j 1`、`cargo +stable-x86_64-pc-windows-gnu test --test interface_snapshots continue_example_runs -j 1`、`examples/continue.ax`。
  - 不做范围：本轮不顺手引入 `match`、`loop`、标签循环或更高层集合遍历语法。
  - 完成于：2026-04-26
  - 备注：这一轮的关键不是“把关键字塞进 parser”，而是把 `continue;` 在 `for` lowering 下的 step 语义补对；当前实现会在 lowered `for` body 内对 `continue;` 做局部重写，保证进入下一轮前仍先跑 step。
- [x] `P1-18` 最小 `match` 语法全链路落地
  - 目标：补一版真正可执行、可诊断、可给 AI 稳定反馈的最小 `match`，而不是只把关键字塞进 parser。
  - 输入：现有 `if / else` 控制流、`break/continue` 后的 control-flow 检查、HIR lowering、AI kind-based rule 映射、模块模式与当前 examples / benchmark prompt。
  - 输出：`match` 词法 / parser / formatter / semantic / return analysis / HIR lowering / interpreter 支持，最小模式规则（`bool` / `i32` / enum / `_`）、AI 规则卡、示例、文档与接口回归。
  - 通过条件：`match` 可在 `bool`、`i32`、enum 输入上稳定 `check / run`；不穷尽、重复 pattern、wildcard 位置错误、pattern 类型不匹配、缺少 concrete pattern 都有稳定诊断与 `rule_id`；文档与 benchmark prompt 不再把 `match` 说成“不支持”。
  - 回归保障：`cargo +stable-x86_64-pc-windows-gnu test --lib -j 1`、`cargo +stable-x86_64-pc-windows-gnu test --test interface_snapshots match_example_runs -j 1`、`examples/match.ax`。
  - 不做范围：本轮不做 binding pattern、解构、guard、表达式形态 `match`、多模式合并或更高级 pattern DSL。
  - 完成于：2026-04-26
  - 备注：这一轮先把 `match` 固定成“语句级、穷尽式、最小 pattern 集”的版本，并通过 lowering 复用现有 `if` / `else` 执行链，避免为第一版引入新的 HIR/MIR 语义面。
- [x] `P1-19` 逻辑与 / 或 `&&` `||` 全链路落地
  - 目标：补齐工具代码高频缺口，让 AX 能写更自然的条件组合，同时保持确定的布尔规则与短路语义。
  - 输入：现有表达式 parser、布尔类型检查、HIR/MIR 二元表达式链路、解释器表达式求值和当前文档中的语法面清单。
  - 输出：`&&` / `||` 的 token / lexer / parser / formatter / semantic / interpreter 支持，短路求值行为、示例、文档与定向回归。
  - 通过条件：`&&` / `||` 可稳定解析、格式化、检查与运行；两边非 `bool` 时会给出明确诊断；运行时不会无意义求值被短路的一侧。
  - 回归保障：`cargo +stable-x86_64-pc-windows-gnu test --lib -j 1`、`cargo +stable-x86_64-pc-windows-gnu test --test interface_snapshots logical_ops_example_runs -j 1`、`examples/logical_ops.ax`。
  - 不做范围：本轮不顺手引入 `%`、复合赋值、`for in`、布尔表达式常量折叠或新的 AI 规则家族。
  - 完成于：2026-04-26
  - 备注：这一轮刻意不把 `&&` / `||` lower 成更复杂的控制流节点，而是先复用现有二元表达式链路，并在解释器层保证真正的短路执行。
- [x] `P1-20` 余数运算 `%` 全链路落地
  - 目标：补齐基础整数运算缺口，让 AX 能更自然地表达分桶、奇偶判断、轮转与索引归类逻辑。
  - 输入：现有算术表达式 parser、`i32` 数值规则、解释器整数运算链路、逻辑运算补完后的表达式文档面。
  - 输出：`%` 的 token / lexer / parser / formatter / semantic / interpreter 支持，`i32` 约束、零除检查、示例、文档与定向回归。
  - 通过条件：`%` 可稳定解析、格式化、检查与运行；非 `i32` 操作数会稳定报错；运行时会检查 `% 0`；文档与缺失语法清单不再把 `%` 记成未支持。
  - 回归保障：`cargo test --lib modulo -j 1`、`cargo test --test interface_snapshots modulo_example_runs -j 1`、`examples/modulo.ax`。
  - 不做范围：本轮不顺手引入 `%=`, 浮点 `%`、常量折叠、`for in` 或新的复杂数值类型。
  - 完成于：2026-04-26
  - 备注：这一轮把 `%` 收敛为“仅 `i32`、有零除检查、优先服务工具代码”的版本，没有为了表面完整度去扩成更大的数值系统。
- [x] `P1-21` 第一版 `for in` 全链路落地
  - 目标：补上真实工具代码最常见的顺序遍历缺口，让 AX 不必所有集合遍历都手写索引型 `for`。
  - 输入：现有数组 / slice、`for` lowering、`continue;` 语义、`len(value)` 统一 helper、缺失语法排序。
  - 输出：`for (let value: T in values) { ... }` 的 token / lexer / parser / formatter / semantic / HIR lowering / interpreter 执行链、示例、文档与 AI 规则映射。
  - 通过条件：第一版 `for in` 可稳定 `check / run / format`；当前只接受数组 / slice；loop variable 类型必须与元素类型一致；`continue;` 在 `for in` 中不会跳过索引推进。
  - 回归保障：`cargo test --lib for_in -j 1`、`cargo test --lib stable_diagnostic_kinds_drive_rule_matching_without_old_message_text -j 1`、`cargo test --test interface_snapshots for_in_example_runs -j 1`、`examples/for_in.ax`。
  - 不做范围：本轮不顺手引入隐式类型推断式 `for in`、iterator trait、`for in` over `string_list` / map / set，也不补 destructuring pattern。
  - 完成于：2026-04-26
  - 备注：第一版刻意保持 AX 的显式类型风格，只支持 `for (let value: T in values)` 这一条轨道；lowering 走“slice temp + index while”以复用现有执行面，并显式保证 `continue;` 前仍会推进索引。
- 进展：2026-04-25 又把运行时高价值 host diagnostics 接进稳定 `DiagnosticKind`，当前覆盖 `argv_get` 负索引/越界、缺失环境变量、不可读文件/目录、`process` 启动失败与 `capture` 非零退出；`src/ai.rs` 对这批宿主误用现也优先按 kind 映射 `rule_id`，并已补上 env/argv runtime AI 回归测试与 diagnostics snapshot 复跑。

## P2

- [ ] `P2-1` 更大语法扩展
  - 进入条件：当前工具风格样例和 repair benchmark 已经能稳定指出“下一批最值钱的语法缺口”，而不是靠感觉扩语法。
  - 目标：只扩那些能明确提升工具代码表达力、benchmark 修复率或后续自举准备效率的语法能力。
  - 必须产物：每个新特性的 parser / semantic / interpreter 或 backend / diagnostics / AI guidance / docs / tests 全链路补齐包。
  - 通过条件：新特性带来可证明收益；`check/json/ai`、AST/HIR/MIR 快照和 repair benchmark 不回退。
  - 不做范围：不为了“像完整语言”而堆 `match`、泛型、async、宏系统等大表面积特性。

- [ ] `P2-2` 原生后端深化
  - 进入条件：语言前中端、diagnostics 和 benchmark 主线已基本稳定，`axc build` 骨架不再频繁变形。
  - 目标：让 `axc build` 从导出骨架产物走向真实可执行的 native backend 路线。
  - 必须产物：更稳定的 MIR 语义、后端接入点、构建失败的结构化诊断、一组真实样例的 native build 结果。
  - 通过条件：至少一组工具风格样例可原生编译并运行；构建失败有结构化错误；耗时与资源开销进入可接受范围。
  - 不做范围：不把当前 `build` 骨架包装成“已经成熟的编译后端”。

- [ ] `P2-3` 更完整标准库
  - 进入条件：已有样例和 benchmark 已经明确暴露出 `io/fs/path/env/process/string` 这类库缺口，且补库比继续补语法更值。
  - 目标：形成一套足以支撑真实工具程序的最小标准库面，而不是一次做大而全运行时。
  - 必须产物：稳定的库接口设计、语义与执行支持、错误与 AI 反馈、示例和文档。
  - 通过条件：新用户可用标准库写出一批真实工具风格程序；repair benchmark 和样例能覆盖这些 API 的常见误用。
  - 不做范围：不引入庞大生态叙事，不先做网络、GUI、并发运行时等高膨胀方向。

- [ ] `P2-4` AX 局部重写前端子集
  - 进入条件：Rust 种子编译器已经足够稳定，AX 语言面和工具链能力已能承载一部分编译器风格逻辑。
  - 目标：用 AX 重写编译器的局部前端子集，验证它是否真的适合承载自身工具链。
  - 必须产物：AX 版子模块、双实现对照流程、输入输出一致性验证、能力缺口清单。
  - 通过条件：AX 子实现可稳定运行，且与 Rust 基线对照结果可重复；不会压垮当前主线开发效率。
  - 不做范围：不提前追求“一口气自举整个编译器”，不为了自举重排当前所有工作。
