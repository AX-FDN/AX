# AX 当前工作清单

> 本文件只放“当前还要做的小细节”。
> 已完成事项不留在这里，统一移到 [`ARCHIVE.md`](./ARCHIVE.md)。
> 所有条目都必须挂靠到 [`PLAN.md`](./PLAN.md) 的 `P0-P7` 阶段编号。
> `PLAN.md` 负责讲闭环路线，本文件负责把当前阶段拆成可执行施工单。

最后更新：2026-04-27

## 文档职责

- [`PLAN.md`](./PLAN.md)
  管唯一方向、阶段切换和前置条件。
- [`WORKLIST.md`](./WORKLIST.md)
  管当前施工项、顺序、依赖、阻塞和验收口径。
- [`ARCHIVE.md`](./ARCHIVE.md)
  管已经完成的事项、结果和日期。

一句话：

- 如果问题是“为什么现在做这个”，看 `PLAN.md`
- 如果问题是“接下来先做哪几件具体小事”，看 `WORKLIST.md`
- 如果问题是“已经做完了哪些事”，看 `ARCHIVE.md`

## 当前施工范围

当前 `WORKLIST` 分两层：

- `当前激活施工层`
  只展开 `P0-P3`，也就是现在真的要做、真的会排期、真的会进回归链的任务
- `已登记未激活层`
  记录 `PLAN` 里已经明确存在、但阶段还没到的后续队列，避免 `PLAN` 和 `WORKLIST` 脱节

也就是说：

- `P0-P3` 要拆成能执行的施工单
- `P4+` 以及未到激活时机的语法线，也要在本文件里登记，但不抢当前主优先级

当前激活主线判断固定为：

- `P0` 是地基修复层
- `P2` 是语言内核主施工层
- `P1` 是编译器护城河同步硬化层
- `P3` 是第一版标准库冻结准备层

## 状态说明

- `[~]` 进行中
- `[ ]` 待做
- `[!]` 明确阻塞，但尚未解除

## 当前总顺序

当前执行顺序是“优先级顺序”，不是“严格串行锁死顺序”。

也就是说：

- `P0` 仍然需要收口验证矩阵与外部契约，但本机 GNU 路径已经稳定
- `P2` 当前出口已经完成，语言内核主代表样例、宿主边界样例和第一项 `match` 语法线已经进入回归
- `P3` 前置边界已经完成，`foundation -> std.*` 的真正迁移暂不启动
- 当前主攻点回到 `P1`：把 `context` 从独立可读接口接进 repair / benchmark 消费链

当前优先级顺序固定为：

1. 先把 `P1` 的 context -> repair/benchmark 输入位接起来
2. 再定义 context 输入最小壳层：优先 `overview + boundaries + evidence`
3. 再给 context-enabled 路径补 smoke 或接口回归
4. 再把 benchmark 展示页做成公开可引用资产
5. 并行补 `P0` 的本机/CI 路径矩阵与外部契约说明
6. 暂不启动 `foundation -> std.*` 代码迁移，除非 P3 迁移试点明确开始

如果 `P1` 和新增语言能力临时抢资源，默认先保证当前护城河闭环：context 能被 repair/benchmark 消费。

## 阶段承接图

本文件不仅要写“现在做什么”，还要写“现在做完后把什么交给下一阶段”。

| 当前施工层 | 当前要交付的东西 | 直接服务下一阶段什么 |
| --- | --- | --- |
| `P0` | 本机可复跑路径、统一验证口径、稳定外部契约 | 让 `P1` 的 repair/context/benchmark 建在稳定接口上 |
| `P1` | 可复跑证据链、context 输入位、公开展示页 | 给 `P2/P3` 提供语言设计验证基线和编译器护城河输入 |
| `P2` | 固定代表样例、固定宿主边界样例、语法优先级、收紧后的 `foundation/` | 给 `P3` 提供真实工作负载、标准库冻结对象和语言主线收口依据 |
| `P3` | `foundation -> std.*` 映射、标准接口边界、迁移试点样例 | 让 `P4/P5` 的 AOT 和包接口建在冻结接口上 |

当前所有激活任务，都必须至少回答下面两个问题：

1. 它在解决当前阶段哪个出口条件？
2. 它做完以后会给下一阶段交什么？

## 当前阶段出口清单

### `P0` 出口还差什么

- [x] 至少一条正式支持的 Windows 本地构建/测试路径写清楚
- [x] README / quickstart / scripts 对本机验证路径的说法一致
- [ ] interface snapshots、context、build skeleton 的外部契约口径稳定

### `P1` 出口还差什么

- [ ] context 已进入至少一条 repair 或 benchmark 输入链
- [ ] benchmark 展示页已经区分“仓库内已复现结果”和“外部尚未完成对照”
- [ ] `base -> ai` 或 `cold -> base -> ai` 的差异能稳定复跑

### `P2` 出口还差什么

- [x] `3` 个主代表样例被正式固定
- [x] `2` 个宿主边界样例被正式固定
- [x] 主代表样例和宿主边界样例都有稳定 `check / run / build` 验证链
- [x] `foundation/` 不再依赖明显的一次性 glue helper
- [x] `P2` 语法优先级顺序已经冻结，不再一边补样例一边随意跳语法点
- [x] 至少一项 `P2` 语法缺口完成 scope freeze，并进入主线闭环准备

### `P3` 当前只做前置，不做全面启动

- [x] 第一版 `std.*` 命名空间边界先写清
- [x] `foundation/* -> std.*` 的映射先列清单
- [x] 至少有一组样例能作为标准库迁移试点

## 近期已解除阻塞

- [x] `B-02` `[P1]` context 视图已经存在，但还未完全进入 repair/benchmark 消费闭环
  - 影响：
    - `axc context` 还更像独立接口，而不是修复链输入层
  - 解除标准：
    - adapter、export 或 compare 链路里至少一条稳定消费 context
  - 当前状态：
    - 已通过 `export-repair-benchmark.ps1 -IncludeContext` 解除

- [x] `B-03` `[P1]` benchmark 展示层还不够硬
  - 影响：
    - 当前更像“仓库内脚本齐了”，还不够像“外部可引用证据页”
  - 解除标准：
    - 展示页、失败样例、方法说明、结果摘要同时成立
  - 当前状态：
    - 已通过 `docs/benchmark-showcase.md` 和 `docs/public-claims.md` 解除

## P0 施工项：环境与契约修复

- [x] `W-P0-02` 写清本机路径与 CI 路径矩阵
  - 目标：明确 Windows 本机、Windows CI、Ubuntu CI 分别跑什么，不再让验证口径混在一起。
  - 依赖：Windows GNU 本机基线已稳定，见 `ARCHIVE` 中 `A-2026-04-27-04`
  - 产物：
    - 一个清楚的本机/CI 路径说明段
    - 哪些链路是 Windows-only、哪些是 cross-platform 的边界描述
  - 完成标准：
    - 文档能回答“本机该跑什么、CI 在跑什么、为什么不完全相同”

- [x] `W-P0-03` 继续冻结高价值外部契约快照
  - 目标：让 diagnostics、context、build skeleton 的对外 JSON 口径继续稳定。
  - 依赖：无。
  - 产物：
    - 更清楚的 interface snapshot 覆盖边界
    - 契约字段的用途解释
  - 完成标准：
    - 新增或保留字段都能解释“为什么必须存在”
    - 文档与 snapshots 的说法一致

- [ ] `W-P0-04` 收口根目录与 docs 入口口径
  - 目标：避免 `README / PROJECT_FACTS / PLAN / WORKLIST / docs` 再各讲一套。
  - 依赖：`W-P0-02` 到 `W-P0-03` 至少基本稳定
  - 产物：
    - 统一入口导航
    - 统一阶段口径
  - 完成标准：
    - 不再出现第二套路线型文档口径

## P1 施工项：编译器护城河硬化

- [x] `W-P1-01` 把 context 协议接进 repair / benchmark 输入通道
  - 目标：让 `overview / boundaries / topology / flow / symbol / impact / evidence` 不只可读，还能被后续修复链消费。
  - 依赖：`P0` 契约字段不能继续漂移。
  - 产物：
    - repair adapter 或 benchmark/export 链路里的稳定 context 输入位
  - 完成标准：
    - 不开启 context 时维持当前行为
    - 开启后进入 smoke 或回归

- [x] `W-P1-02` 定义 context 输入最小壳层
  - 目标：明确 repair/export 链路到底吃哪些 context 视图，不搞“把所有视图全塞进去”的粗暴方案。
  - 依赖：`W-P1-01`
  - 产物：
    - context 输入位字段定义
    - 默认开启/关闭策略
  - 完成标准：
    - 能清楚回答“修复链最先消费哪几层 context”

- [x] `W-P1-03` 给 context-enabled 路径补 smoke 或回归
  - 目标：防止 context 接入后只存在于一次性实验中。
  - 依赖：`W-P1-01`、`W-P1-02`
  - 产物：
    - 对应 smoke 或接口回归
  - 完成标准：
    - context 进入修复链以后能被持续验证

- [x] `W-P1-04` 完成一版可公开引用的 benchmark 展示页
  - 目标：不只保留脚本，还要有方法说明、结果摘要、失败样例和“哪些结论已证实、哪些还没证实”的边界。
  - 依赖：`W-P1-01` 到 `W-P1-03`
  - 产物：
    - 更完整的 `docs/benchmark-showcase.md`
    - README / docs 入口指向
  - 完成标准：
    - `docs/benchmark-showcase.md` 与 `docs/repair-benchmark.md` 口径一致
    - 结果可回放、可重跑、可解释

- [x] `W-P1-05` 补 benchmark 失败样例说明层
  - 目标：不只展示 lift，也展示失败和退化 case，避免宣传化。
  - 依赖：`W-P1-04`
  - 产物：
    - 至少一组失败样例说明
    - 哪些结论尚未对外证实的边界说明
  - 完成标准：
    - 展示页不是“只报喜不报忧”

- [x] `W-P1-06` 收紧公开口径
  - 目标：所有对外表述都必须遵守“仓库内可复现事实”和“外部对照结论”分离。
  - 依赖：`W-P1-04`、`W-P1-05`
  - 产物：
    - README / docs / showcase 一致的措辞
  - 完成标准：
    - 不再出现“已经胜过某语言子集”式过界表述

## P2 施工项：语言内核与最小可写工具

- [x] `W-P2-01` 固定 `3` 个主代表样例
  - 当前优先候选：
    - `examples/project_directory_index/`
    - `examples/project_text_normalize/`
    - `examples/project_release_promote/`
  - 目标：让“代表样例”从口头说法变成正式主集合。
  - 依赖：`P1` 证据链至少稳定到可持续验证这些样例。
  - 完成标准：
    - 三个样例都能稳定 `check / run / build`
    - 文档明确它们是“主代表样例”

- [x] `W-P2-02` 给主代表样例接入更硬的回归链
  - 目标：主代表样例不能只存在于 examples，要进入 smoke 或固定 regression。
  - 依赖：`W-P2-01`
  - 产物：
    - `check / run / build` 回归入口
  - 完成标准：
    - 至少一层 smoke 或固定 regression

- [x] `W-P2-03` 固定 `2` 个宿主边界样例
  - 当前优先候选：
    - `examples/project_command_capture/`
    - `examples/project_command_batch/`
  - 目标：让它们承担 `process / env / path / fs` 验证职责，而不是继续泛化成更多样例品类。
  - 依赖：`W-P2-01`
  - 完成标准：
    - 验证目标写清楚
    - 对应宿主能力有固定回归

- [x] `W-P2-04` 收紧 `foundation/` helper，清理一次性辅助层
  - 目标：把当前 helper 分成“可复用、继续孵化、应该删除”三类。
  - 依赖：`W-P2-01`、`W-P2-03`
  - 产物：
    - 当前 helper 归类清单
  - 完成标准：
    - 主代表样例不再依赖一次性 glue helper
    - 下一批能力缺口来自真实样例和 repair case，而不是拍脑袋

- [x] `W-P2-05` 从样例与 repair case 反推下一批最值钱缺口
  - 目标：不是先扩语法面，而是先判断“下一刀最该补的是 `match` 第二刀、payload enum 深化，还是更小的表达/宿主缺口”。
  - 依赖：`W-P2-01` 到 `W-P2-04`
  - 当前结论：
    - 第一优先级仍是 `match` 第二刀，并已通过 bootstrap 样例迁移和回归启动
    - 第二优先级仍是 payload enum 深化，但只限 unit variant + 单 payload variant 做稳
    - 更小表达缺口当前不主动插队；宿主压力优先回到 `foundation/`、runtime diagnostics 或后续 `std.*` 路线
  - 完成标准：
    - 结论来自代表样例与 repair case
    - 一旦决定补能力，必须能立刻回写样例或修复链

- [x] `W-P2-06` 主代表样例与宿主边界样例的角色文档化
  - 目标：明确哪些样例承担“表达能力验证”，哪些承担“宿主边界验证”。
  - 依赖：`W-P2-01`、`W-P2-03`
  - 完成标准：
    - README / docs / facts 中不再混淆两类样例职责

## P2 语法施工项：最小可写工具内核的语法线

- [x] `W-P2-S01` 固定 `P2` 语法优先级顺序
  - 目标：把 `PLAN` 里已经列出的 `match` 第二刀、payload enum 深化和更小表达缺口，按当前真实价值排出先后。
  - 候选范围：
    - `match` 第二刀
    - payload enum 深化
    - 由主代表样例或 repair case 暴露出的更小表达缺口
  - 当前冻结结论：
    - 第一优先级：`match` 第二刀
    - 第二优先级：payload enum 深化
    - 第三优先级：仅在 `3` 个主代表样例或 `2` 个宿主边界样例被真实卡住时，才插入一个更小表达缺口；当前不主动立项新的语法糖
  - 冻结依据：
    - 现有主代表样例 `project_directory_index / project_text_normalize / project_release_promote` 主要压力在工程组织、宿主能力和共享 helper，不在新的表面语法糖
    - 现有工具样例已经大量稳定消费 `for`、slice、字符串处理、模块第一刀和宿主 builtin，这些能力当前不是 `P2` 语法主缺口
    - `match` 与 payload enum 目前已经进入单文件样例、bootstrap 风格样例、`SYNTAX.md` 和 repair 资产，但还更像“演示能力”而不是“可持续扩的工具语言能力”，因此应该先把这一条做完整
    - payload enum 的真实价值依赖 `match` 消费面继续变强；如果先扩 payload 形态而不先补 `match`，会扩大表面积但不能直接回写到真实工具 workload
  - 本轮明确不做：
    - 不在 `P2` 插入 `pub`、`const`、`import` 第二刀、泛型、trait、async、异常系统
    - 不为了缩短当前样例代码而主动发明新的小语法；若压力主要来自 helper 组织或宿主接口，优先在 `foundation/` 或后续 `std.*` 路线上解决
  - 完成标准：
    - 结论来自当前代表样例、宿主边界样例候选、现有 `match/payload enum` 样例和 repair case
    - 已明确“先做什么，不做什么”，后续 `P2` 不再随意跳语法点

- [x] `W-P2-S02` 冻结 `match` 第二刀 scope
  - 目标：把“想继续补 `match`”收成一份明确范围，而不是边写边长。
  - 当前冻结范围：
    - 本轮目标不是把 `match` 扩成完整 pattern matching 系统，而是把它收成“可稳定承载枚举驱动控制流和单 payload 枚举消费”的第二刀
    - 本轮只围绕现有单 scrutinee、单层 pattern 面继续补强，不引入第二套并行写法
    - 本轮允许继续打磨并回归的 pattern 只包括：
      - `true` / `false`
      - 整数字面量
      - `Enum.Variant`
      - `Enum.Variant(name)`
      - `Enum.Variant(_)`
      - 最终 `_`
      - 最终裸标识符 catch-all
  - 本轮必须直接服务的场景：
    - 让 `match` 能承担 bootstrap 风格样例里的枚举状态/标记分派，而不是继续只停留在 `if (value == Enum.Variant)` 梯子
    - 让 `match` 在 statement form 和 expression form 上都能稳定消费单 payload enum
    - 在不新增大语法面的前提下，把现有 pattern 集合的 duplicate / final catch-all / non-exhaustive / payload-shape 诊断继续打硬
  - 本轮明确不补：
    - 字符串字面量 pattern
    - tuple / array / struct destructuring
    - 多 payload 或命名 payload fields
    - 嵌套 payload destructuring
    - match guards
    - 多 pattern arm（如 `A | B`）
    - range pattern
    - block-valued match expression arm
  - 冻结依据：
    - `project_directory_index / project_text_normalize / project_release_promote` 的主压力不在新增表面 pattern，而在工程组织、宿主能力和共享 helper
    - `bootstrap_state_machine / bootstrap_token_scan / bootstrap_block_summary` 这类样例已经暴露出“枚举驱动控制流应该由 match 承担”的方向，但当前写法还更依赖相等判断梯子
    - repair 资产已经覆盖 enum variant 与 payload shape 误用，因此优先补强现有 enum-first `match` 面，比引入新的 pattern 家族更能直接回写到修复链
  - 依赖：`W-P2-S01`
  - 完成标准：
    - 能明确回答 `match` 第二刀到底是什么，不是什么

- [x] `W-P2-S03` 冻结 payload enum 深化 scope
  - 目标：把 payload enum 的下一刀写清楚，到底是补更稳的构造/匹配能力，还是补更深的 payload 形态。
  - 当前冻结范围：
    - 本轮不是把 enum 扩成完整代数数据类型系统，而是把“unit variant + 单 payload variant”这一路做稳
    - payload enum 深化的主目标是稳定三件事：
      - 构造：`Enum.Variant(value)` 的类型与 payload-shape 约束
      - 消费：`match` 里的 `Enum.Variant(name)` / `Enum.Variant(_)`
      - 诊断：缺 payload、错 payload、错 variant、把类型名当值等高频错误的稳定反馈
    - 本轮只允许继续补强：
      - unit variant
      - 单 payload variant
      - 单名字 payload binding
      - payload wildcard
      - 模块限定路径下的同一套构造/匹配/诊断行为
  - 本轮必须直接服务的样例与 repair 面：
    - `examples/payload_enum.ax`
    - `examples/type_name_as_value.ax`
    - `examples/unknown_enum_variant.ax`
    - 现有 enum/payload 相关 semantic tests 与 repair 资产
  - 本轮明确不补：
    - 多 payload variant
    - 命名 payload fields
    - 嵌套 payload pattern
    - enum generic parameter 化
    - 为 payload enum 单独引入新的构造语法
  - 冻结依据：
    - 当前 parser / semantic / interpreter / `SYNTAX.md` 已经围绕单 payload variant 建立了第一刀能力与边界说明
    - 当前真实 repair 面首先暴露的是 enum surface 与 payload-shape 误用，而不是“payload 形态不够花”
    - 如果在 `match` 第二刀还没站稳前就扩到多 payload 或命名字段，只会扩大实现面与 diagnostics 面，不会直接改善当前工具 workload
  - 依赖：`W-P2-S01`
  - 完成标准：
    - payload enum 深化不再是口头方向，而是可实施范围

- [x] `W-P2-S04` 建立 `P2` 语法施工同步清单
  - 目标：任何 `P2` 语法线一旦启动，必须同步补齐哪些层，先在 `WORKLIST` 里写死。
  - 必须同步：
    - lexer / parser / AST
    - formatter
    - semantic / diagnostics
    - `src/ai.rs`
    - HIR / MIR
    - interpreter
    - `SYNTAX.md`
    - `docs/feature-matrix.md`
    - tests / interface snapshots
    - 代表样例
    - 如适用则补 benchmark case
  - 依赖：无。
  - 完成标准：
    - 后续不会再出现“语法进了 parser，但没进修复链和样例”的半截能力

- [x] `W-P2-S05` 选择并启动第一项 `P2` 语法实现
  - 目标：按当前冻结顺序先启动 `match` 第二刀；只有代表样例被真实卡住时，才允许插入一个更小表达缺口。
  - 当前已选择：
    - 第一项启动对象固定为 `match` 第二刀
    - 第一组落点不是新增一组 pattern，而是先把 bootstrap 风格样例从 `if (value == Enum.Variant)` 梯子迁到 enum-first `match`
  - 已完成的最小实现面：
    - `examples/bootstrap_state_machine.ax` 使用 nested expression `match` 承担状态机分派
    - `examples/bootstrap_block_summary.ax` 使用 statement `match` 承担 token 分派
    - `examples/bootstrap_token_scan.ax` 使用 statement `match` 承担 token kind 分派
    - `tests/interface_snapshots.rs` 覆盖三个 bootstrap 样例的运行回归
    - `src/semantic.rs` 补充 enum `match` 非穷尽诊断回归
  - 依赖：`W-P2-S01`、`W-P2-S02`、`W-P2-S03`、`W-P2-S04`
  - 完成标准：
    - 第一项语法实现进入主线时，能立刻回写代表样例与回归链
    - 如果不是 `match` 第二刀，必须先在 `WORKLIST` 里补一条“为什么代表样例被它真实卡住”的说明
  - 验证：
    - `.\scripts\cargo-gnu.ps1 test --test interface_snapshots`
    - `.\scripts\cargo-gnu.ps1 test --lib`
    - `.\scripts\cargo-gnu.ps1 test --lib reports_non_exhaustive_enum_match`
    - `.\scripts\cargo-gnu.ps1 test --test interface_snapshots bootstrap_token_scan_example_runs`

## P3 前置语法与接口项：为包接口和标准库做准备

- [ ] `W-P3-S01` 登记 `pub / 模块边界` 为 `P5` 前置语法
  - 目标：虽然现在不启动实现，但要明确这是包接口前的硬前置，而不是临时想到再补。
  - 当前状态：已登记，未激活。

- [ ] `W-P3-S02` 登记 `import` 人体工学第二刀为 `P5` 前置语法
  - 目标：先明确它属于包接口前的组织性补丁，而不是当前 `P2` 表达性补丁。
  - 当前状态：已登记，未激活。

- [ ] `W-P3-S03` 登记 `const / 常量定义` 为标准库前置语法
  - 目标：先明确它服务的是标准库和包接口稳定性，不抢当前 `P2` 主优先级。
  - 当前状态：已登记，未激活。

## P3 施工项：官方最小标准库准备

- [x] `W-P3-01` 起草 `foundation -> std.*` 的最小迁移清单
  - 目标：先定义第一版官方命名空间和迁移边界，不急着一次性重写全部示例。
  - 依赖：`P2` 至少完成主代表样例与 helper 分类。
  - 完成标准：
    - `std.text / std.cli / std.fs / std.path / std.env / std.process / std.report / std.workspace / std.collections` 的接口边界先写清

- [x] `W-P3-02` 建立 `foundation` 文件到 `std.*` 模块的映射表
  - 目标：明确当前哪些 helper 未来对应哪块标准库接口。
  - 依赖：`W-P3-01`
  - 完成标准：
    - 至少能回答 `text / report / workspace / cli` 这几块怎么迁

- [x] `W-P3-03` 写清标准库接口的四件套要求
  - 目标：冻结前先明确每个官方接口都必须同步什么。
  - 依赖：`W-P3-01`
  - 四件套：
    - diagnostics
    - docs
    - examples
    - regression
  - 完成标准：
    - 标准库冻结前，不会漏掉这些配套环

- [x] `W-P3-04` 选择一组样例做标准库迁移试点
  - 目标：不是全仓一把梭，而是先找一组最适合试点的 project-backed 样例。
  - 当前优先候选：
    - `project_text_normalize`
    - `project_directory_index`
  - 依赖：`W-P3-01`、`W-P3-02`
  - 完成标准：
    - 至少有一组样例能作为将来 `foundation -> std.*` 的迁移样板

- [x] `W-P3-05` 写清宿主 Rust builtin 与 AX 标准接口的边界
  - 目标：为后续标准库冻结提前把“AX 接口”和“宿主实现细节”分开。
  - 依赖：`W-P3-01`
  - 完成标准：
    - 用户视角看到的是 AX 接口，不是 Rust crate 名单

## 当前明确不插队的方向

下面这些方向当前不进入 `WORKLIST` 主优先级：

- 大语法面扩张，例如复杂泛型、trait 体系、async、异常系统
- `AX import -> Cargo crate` 直通桥
- 通用 FFI、成熟包系统、网络库、并发库
- AOT、JIT、自举和生态扩张的实现性施工
- 为了“看起来完整”继续新增第二套路线文档
- 提前把 `build` 宣传成成熟 native backend

## 已登记但未激活的 `PLAN` 队列

下面这些方向已经在 [`PLAN.md`](./PLAN.md) 明确存在，所以在 `WORKLIST` 里显式登记，但当前不进入激活施工层：

### `P4` 后端线

- [ ] `Q-P4-01` `Build-1` 单文件 AOT
- [ ] `Q-P4-02` `Build-2` 多文件代表项目 AOT
- [ ] `Q-P4-03` `Build-3` 发布级 AOT
- [ ] `Q-P4-04` `JIT-Eval`

### `P5` 包接口线

- [ ] `Q-P5-01` path package
- [ ] `Q-P5-02` lockfile
- [ ] `Q-P5-03` registry package
- [ ] `Q-P5-04` package diagnostics / smoke

### `P6-P7` 后续语法线

- [ ] `Q-P6-01` methods / `impl`
- [ ] `Q-P6-02` 泛型
- [ ] `Q-P6-03` traits / interfaces
- [ ] `Q-P6-04` richer pattern matching
- [ ] `Q-P7-01` 闭包 / lambda
- [ ] `Q-P7-02` async / await

### `P6-P7` 自举与生态线

- [ ] `Q-P6-05` AX 自写 benchmark/report/context 工具
- [ ] `Q-P6-06` AX 自写项目辅助工具
- [ ] `Q-P7-03` host extension ABI
- [ ] `Q-P7-04` Linux 与 macOS 同级 core support

## 使用规则

1. 新增待做项时，先写它挂靠的 `PLAN` 阶段编号，再写任务本身。
2. 如果任务有依赖，直接写清依赖，不要靠人脑记。
3. 完成后，先从这里移除，再写入 [`ARCHIVE.md`](./ARCHIVE.md)。
4. 如果发现待做项已经不再服务 [`PLAN.md`](./PLAN.md) 当前阶段，先改 `PLAN.md`，再改这里。
