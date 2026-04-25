# AX Worklist

最后更新：2026-04-24

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
5. 用最小工具宿主能力和真实工具样例验证 AX 的实际可写性
6. 在证据支持下再推进更大的语言面和后端

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
  - 不做范围：本轮不引入 `continue;`，因为当前 `for` lowering 下如果硬上会把 step 语义做错。
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

- [ ] `P1-10` 最小工具宿主能力第一批
  - 目标：给 AX 补上写真实 CLI / 文本处理 / 构建辅助程序的最低可用宿主能力，而不是继续优先补大语法面。
  - 输入：[`PLAN.md`](C:/Users/xiaoy/Desktop/A语言/AX/PLAN.md) 的任务族定义、现有工具风格样例、[`能力缺口排序.md`](C:/Users/xiaoy/Desktop/A语言/AX/能力缺口排序.md) 的最新排序。
  - 输出：围绕 `process / env / path / fs / string` 的第一批稳定接口、对应 diagnostics / AI guidance / 示例 / 文档 / 测试。
  - 通过条件：至少一批真实工具风格程序不再只依赖纯玩具输入；误用这些接口时仍能给出稳定结构化反馈。
  - 回归保障：语义 / 解释器 / 接口测试、真实样例 smoke、repair benchmark 资产。
  - 不做范围：不一口气做大而全标准库，不先做网络、GUI、并发运行时或包生态。

- [~] `P1-11` 第二组真实工具样例与 backend 验证目标
  - 目标：让后续 `axc build` 与更大语法决策建立在真实程序上，而不是建立在玩具样例上。
  - 输入：第一批宿主能力、现有 examples、benchmark 观察与 build 骨架现状。
  - 输出：2-4 个更接近真实任务的样例，覆盖 CLI、文本处理、批处理或构建辅助中的至少两类，并形成后续 native build 验证目标。
  - 通过条件：这些样例可 `check / run`，且能明确暴露“下一步补标准库、backend 还是语法”。
  - 回归保障：`examples/`、`tests/interface_snapshots.rs`、必要的 smoke。
  - 不做范围：不把样例扩成完整产品，不为了样例强行引入大表面积语法。
  - 进展：2026-04-24 已新增 [`examples/workspace_audit.ax`](C:/Users/xwh/Desktop/AX-main-git/examples/workspace_audit.ax)、[`examples/docs_release_snapshot.ax`](C:/Users/xwh/Desktop/AX-main-git/examples/docs_release_snapshot.ax) 与 [`examples/workspace_search_report.ax`](C:/Users/xwh/Desktop/AX-main-git/examples/workspace_search_report.ax)，三者均已完成 `check / run` 验证。
  - 进展：2026-04-24 已新增项目化样例 [`examples/project_split/`](C:/Users/xwh/Desktop/AX-main-git/examples/project_split)、[`examples/project_foundation_report/`](C:/Users/xwh/Desktop/AX-main-git/examples/project_foundation_report)、[`examples/project_docs_release/`](C:/Users/xwh/Desktop/AX-main-git/examples/project_docs_release)、[`examples/project_workspace_audit/`](C:/Users/xwh/Desktop/AX-main-git/examples/project_workspace_audit) 与 [`examples/project_workspace_search_report/`](C:/Users/xwh/Desktop/AX-main-git/examples/project_workspace_search_report)，并把 `AX.toml` 的 `sources` 扩到可指向 `lib/` 这类支持目录、把 `axc build` 扩到导出 `project-sources/` 原始项目源树快照、把 repair benchmark export/score 扩到支持“项目上下文 + 单文件修复目标”的 project-backed case。最新已把首个公开仓库用例 [`benchmarks/repair-projects/helper_missing_semicolon/`](C:/Users/xwh/Desktop/AX-main-git/benchmarks/repair-projects/helper_missing_semicolon) 接进 full/smoke manifest 与 shared/smoke replay，用于验证目录级多文件装载和 AX 侧 helper 库组织。
  - 进展：2026-04-25 已把共享 AX 基础层扩到 [`examples/foundation/`](C:/Users/xiaoy/Desktop/A语言/AX/examples/foundation/) 下的 `file_kind.ax` 与 `workspace.ax`，并把多个 project 样例中重复的本地文件分类、workspace 标签与报告明细行渲染 helper 回收到共享层，开始形成真正可复用的 AX-side foundation。
  - 进展：2026-04-25 已继续补强共享 [`examples/foundation/cli.ax`](C:/Users/xiaoy/Desktop/A语言/AX/examples/foundation/cli.ax)，新增最小入口骨架 helper：`require_min_args`、`require_directory`、`require_file`、`require_non_empty_text`、`ensure_directory` 与 `recreate_directory`；`project_foundation_report / project_docs_release / project_workspace_audit / project_workspace_search_report` 已改为直接复用这层 CLI 校验与目录准备逻辑。
  - 当前暴露的真实缺口：不是先继续堆大语法，而是模块/导入/库组织、更深目录遍历、可扩展集合能力，以及把这些样例接入 smoke 或后续 backend 验证目标。

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
