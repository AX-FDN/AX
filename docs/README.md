# AX Docs

- [`architecture.md`](./architecture.md)
  面向接手者的编译器架构地图：命令入口、前端流水线、解释执行、build/AOT、project/package、诊断/AI/context，以及后续加语法和做规划时应该优先看哪些层。

本目录放 AX 当前稳定、对外可引用的专题文档，服务于 AX 作为一门 AI-first 工具语言的对外说明、编译器护城河文档化和使用入口整理。
它不承担路线职责；路线只看 [`../执行路线.md`](../执行路线.md)。

## 先看哪几份

- [`../README.md`](../README.md)
  对外介绍、项目价值、入口命令和代表样例。
- [`../PROJECT_FACTS.md`](../PROJECT_FACTS.md)
  当前事实层：AX 已做到哪、哪些还没做到。
- [`../执行路线.md`](../执行路线.md)
  当前唯一执行路线，定义错误分层、AI 自修复、AOT 验证闭环和阶段出口。
- [`../曾经的计划/`](../曾经的计划/)
  已退役的旧版计划和施工单，仅作历史参考。
- [`../ARCHIVE.md`](../ARCHIVE.md)
  已完成事项归档。
- [`release-0.1-alpha.md`](./release-0.1-alpha.md)
  AX 0.1 Alpha 的收口边界、AOT parity 快照、发布前验证基线和不应夸大的 public claims。
- [`../CONTRIBUTING.md`](../CONTRIBUTING.md)
  贡献入口、推荐验证命令、AOT/标准库/包样例的提交方式。

## 当前专题文档

- [`why-not-language-subsets.md`](./why-not-language-subsets.md)
  说明为什么 AX 的价值必须由 canonical syntax、structured diagnostics、repair contract 和 benchmark evidence 一起成立。
- [`benchmark-showcase.md`](./benchmark-showcase.md)
  汇总当前已经验证过的 benchmark 结果，包括 `43` 个 full repair case、已发布的 `30` case deterministic replay、context-enabled export，并区分“仓库内可复现事实”和“尚未完成的外部对照”。
- [`repair-archaeology.md`](./repair-archaeology.md)
  定义下一轮 `Repair Archaeology v0`：把 repair replay、score、compare 和 context-enabled export 资产整理成 case 级 JSON / Markdown 修复证据对象。
- [`repair-archaeology-schema.md`](./repair-archaeology-schema.md)
  定义 Repair Archaeology v0 的 case 级 JSON artifact、Markdown 报告模板、字段来源和事实/解释边界。
- [`public-claims.md`](./public-claims.md)
  定义 AX 当前对外表述边界，避免把仓库内可复现事实说成尚未完成的跨语言或 live-model 结论。
- [`application-scenarios.md`](./application-scenarios.md)
  定义 AX 的 AI-first 先落在哪些真实场景，以及后端语言方向的实际推进顺序。
- [`llvm-aot.md`](./llvm-aot.md)
  记录 `axc build` 当前的 LLVM IR v0 后端原型、manifest schema version `10`、`--json` 输出、链接环境变量、AOT blocker AI 建议、run vs AOT exe parity smoke 和不应误读成发布级 native compiler 的边界。
- [`aot-native-abi.md`](./aot-native-abi.md)
  收口 AX Native ABI v1：string、slice、string_list、runtime error、内存策略和当前 LLVM layout 约定。
- [`package-registry-v0.md`](./package-registry-v0.md)
  定义 curated registry v0：先做源码包索引、下载/锁定/校验，不开放公共上传服务器。
- [`killer-demo.md`](./killer-demo.md)
  给对外演示用的短 demo 脚本。
- [`../web/`](../web/)
  Repair Workbench 前端，用可视化方式展示 AX 项目概览、same-case repair demo、cold/base/ai 对比、interface contract 卡片和 workload 入口。
- [`representative-samples.md`](./representative-samples.md)
  固定 P2 阶段的主代表样例、宿主边界样例和对应回归职责。
- [`quickstart.md`](./quickstart.md)
  Windows / Linux 快速开始总入口。
- [`quickstart-windows.md`](./quickstart-windows.md)
  Windows 完整工作流入口。
- [`quickstart-linux.md`](./quickstart-linux.md)
  Linux 核心 compiler/runtime 入口。
- [`platform-support.md`](./platform-support.md)
  定义 Windows / Linux / macOS 当前支持层级与边界。
- [`validation-matrix.md`](./validation-matrix.md)
  定义 Windows 本机、Windows CI、Ubuntu CI 分别应该跑什么，以及哪些链路仍是 Windows-only。
- [`interface-contracts.md`](./interface-contracts.md)
  说明 diagnostics、context、build manifest、repair export 等高价值外部契约及其 snapshot/regression 覆盖。
- [`host-runtime-boundary.md`](./host-runtime-boundary.md)
  解释 Rust 宿主原语、AX 接口层、project-private 库和未来包系统之间的边界。
- [`foundation-inventory.md`](./foundation-inventory.md)
  记录 `foundation/` helper 的分类、已下沉接口和继续孵化理由。
- [`stdlib-minimal-boundary.md`](./stdlib-minimal-boundary.md)
  定义 P3 第一版 `std.*` 命名空间、Std-1 冻结候选、继续孵化清单和宿主边界。
- [`import-module-minimal-design.md`](./import-module-minimal-design.md)
  固定第一阶段 `import / module` 设计。
- [`repair-benchmark.md`](./repair-benchmark.md)
  说明 benchmark manifest、导出链、运行链、评分和 compare 工作流。
- [`repair-adapter-spec.md`](./repair-adapter-spec.md)
  定义外部 repair adapter 的输入输出契约。
- [`diagnostics-schema.md`](./diagnostics-schema.md)
  说明 `axc check --json`、`axc run --json` 与 `--json --ai` 的稳定结构。
- [`diagnostics-benchmark-schema.md`](./diagnostics-benchmark-schema.md)
  说明 `benchmark-diagnostics.ps1` 输出 `summary.json` 的稳定结构。

## Repair Workbench Frontend

[`../web/`](../web/) 是当前独立的 Vite + React repair workbench 前端。它不改变 Rust 编译器 crate，也不影响 `cargo` 工作流；它负责把 benchmark 指标、`slice_assignment_read_only` sharp demo、`cold / base / ai` 反馈模式对比、稳定接口契约和 docs/workload 入口放到同一个可演示页面里。

前端验证入口：

```powershell
cd web
npm ci
npm run build
```

GitHub Actions 已有独立 `web` job 验证 `npm ci` 与 `npm run build`。

## 真实 workload 入口

当前更像真实工具的样例主要在 [`../examples/`](../examples/)：

- P2 固定代表样例与回归职责见 [`representative-samples.md`](./representative-samples.md)。
- [`../examples/workspace_audit.ax`](../examples/workspace_audit.ax)
  工作区审计与摘要报告。
- [`../examples/docs_release_snapshot.ax`](../examples/docs_release_snapshot.ax)
  文档快照、复制、收据与汇总。
- [`../examples/workspace_search_report.ax`](../examples/workspace_search_report.ax)
  关键字搜索与匹配报告。
- [`../examples/project_command_capture/`](../examples/project_command_capture/)
  命令执行与输出捕获。
- [`../examples/project_release_promote/`](../examples/project_release_promote/)
  发布产物提升与收据输出。
- [`../examples/project_directory_index/`](../examples/project_directory_index/)
  递归目录索引。
- [`../examples/project_command_batch/`](../examples/project_command_batch/)
  批量命令编排与环境变量读取。
- [`../examples/project_text_normalize/`](../examples/project_text_normalize/)
  文本读取、改写与报告输出。
- [`../examples/project_workspace_search_report/`](../examples/project_workspace_search_report/)
  多文件工作区搜索与报告。

## 文档边界

- 想知道 AX 是什么，看 [`../README.md`](../README.md)
- 想知道 AX 当前做到哪，看 [`../PROJECT_FACTS.md`](../PROJECT_FACTS.md)
- 想知道 AX 接下来按什么阶段推进，看 [`../执行路线.md`](../执行路线.md)
- 想看旧版计划和施工单，看 [`../曾经的计划/`](../曾经的计划/)
- 想知道哪些事已经做完，看 [`../ARCHIVE.md`](../ARCHIVE.md)
