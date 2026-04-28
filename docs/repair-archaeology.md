# AX Repair Archaeology

> 本文定义 `Repair Archaeology v0` 的定位、边界和进入路线。
> 它不是新语法，也不是模型调用器；它是 AX repair benchmark 证据链的可回放、可解释、可导出展示层。

## 一句话定义

Repair Archaeology 是 AX 对修复证据链的“考古层”：

- 从已有 repair cases、adapter 输出、score 结果、compare 结果和 context bundle 中抽取事实
- 按 case 还原“初始错误 -> 修复候选 -> 验证结果 -> 模式差异 -> 失败原因”
- 输出稳定 JSON / Markdown，让外部读者能看懂 AX 的修复协议到底怎样工作

它解决的不是“让模型现场修代码”，而是：

- 这个错误怎么被修的
- 哪种反馈模式更有效
- 哪一步失败了
- context 有没有进入修复输入
- 这个 case 能不能被复现、比较和引用

## 为什么现在需要它

AX 已经有 repair benchmark 的骨架：

- manifest
- export
- run
- score
- compare
- smoke
- context-enabled export
- benchmark showcase

但这些资产对外仍偏“脚本和结果表”。
Repair Archaeology v0 的价值，是把它们升级成“可读的修复故事”和“可查询的证据对象”。

这比现在直接做 `axc generate`、真实模型交互或 UI 更适合当前阶段，因为它：

- 不引入模型供应商依赖
- 不新增 AX 语法
- 不扩大 package / runtime / stdlib 表面积
- 直接复用当前最强资产：diagnostics、repair contract、context protocol、benchmark evidence
- 能成为后续 Live Repair Stream 的数据基础

## v0 范围

`Repair Archaeology v0` 只做离线证据整理。

允许范围：

- 从现有 benchmark/export/score/compare 产物读取数据
- 按 case 输出 repair timeline
- 对比 `cold / base / ai` 或后续稳定模式
- 标注初始 diagnostic、rule_id、repair_goal、fixits、context bundle 是否使用
- 标注候选是否通过 check/run/score
- 导出 Markdown 报告
- 导出稳定 JSON artifact
- 给 README、benchmark showcase 和 demo 文档引用

不做范围：

- 不调用真实 LLM
- 不保存 API key
- 不做 `axc generate`
- 不做实时 UI
- 不做新 AX 语法
- 不做 `repair_contract as types`
- 不做 shadow execution policy
- 不把 replay 结果夸大成 live-model 结论

## 候选命令形态

v0 阶段优先选择脚本或独立工具入口，不急着扩 `axc` 命令面。

当前 v0 脚本入口：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\export-repair-archaeology.ps1 `
  -ComparisonPath .ax-ai\repair-comparisons\showcase-current\comparison.json `
  -OutputDir .ax-ai\repair-archaeology\showcase-current `
  -CaseIds missing_semicolon_basic,slice_assignment_read_only
```

候选长期入口：

```powershell
axc repair-log show --case non_bool_condition --format markdown
axc repair-log compare --case non_bool_condition --modes cold,base,ai
axc repair-log stream --case non_bool_condition --json-stream
```

长期入口必须等 artifact schema 和脚本形态稳定后再进入 `axc`，避免 P0-P3 阶段继续膨胀 CLI。

## v0 输出结构

v0 的具体 artifact 契约见 [`repair-archaeology-schema.md`](./repair-archaeology-schema.md)。

最小输出固定为：

- `index.json`
- `cases/<case-id>.json`
- `cases/<case-id>.md`

JSON artifact 是机器可消费的事实源；Markdown 报告只是展示层。
所有 pass/fail、remaining diagnostics、candidate status 都必须从现有 run / score / compare artifact 抽取，不能从 Markdown 文案反推。

## 与 Live Repair Stream 的关系

Live Repair Stream 不作为当前主实现。
它可以作为 Repair Archaeology 的展示层后续出现：

- v0：离线 repair archaeology，先把证据整理成 timeline
- v1：把 timeline 以 NDJSON 形式流式输出
- v2：再评估是否接真实 adapter / model loop

也就是说，当前最稳的路线不是直接做“实时协作式修复”，而是先把已有 replay 证据变成可流式展示的数据。

## 进入条件

启动 v0 前必须满足：

- repair benchmark full manifest 可跑
- score / compare 产物结构稳定
- benchmark showcase 已能解释当前 replay 结果
- public claims 文档已经区分“仓库内事实”和“外部 live-model 结论”
- 至少有一个通过 case 和一个失败/退化 case 可用于展示

## 退出条件

v0 完成时必须交付：

- 一份 schema 说明
- 一个最小导出脚本
- 至少 `3` 个 case 的 JSON / Markdown archaeology 报告
- 至少 `1` 个失败或退化 case 的解释报告
- 一个可复跑导出入口
- README / benchmark showcase / docs README 的入口链接
- 对应 smoke 或 interface regression，避免报告格式漂移

## 当前优先级

Repair Archaeology 是 `P1` 编译器护城河的展示与解释层。
它不抢 `P2/P3` 语言内核和最小标准库收口资源。

当前推荐顺序：

1. 先完成 `W-P3-15`，把 Std-1 冻结候选验证入口写清
2. 再启动 Repair Archaeology v0 的 artifact schema
3. 再做最小 JSON / Markdown export
4. 再补 smoke 或 interface regression
5. 最后再评估 `json-stream` 展示层
