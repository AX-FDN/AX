import {
  Activity,
  Binary,
  Boxes,
  Braces,
  BrainCircuit,
  Code2,
  FileCode2,
  FileJson,
  FolderTree,
  GitCompareArrows,
  Hammer,
  Layers3,
  Network,
  PackageCheck,
  Route,
  ShieldCheck,
  Sparkles,
  TerminalSquare,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

export type FeedbackMode = 'cold' | 'base' | 'ai';

export type PortalNavItem = {
  label: string;
  href: string;
};

export type StatCard = {
  label: string;
  value: string;
  caption: string;
  tone: 'cyan' | 'green' | 'amber' | 'red';
};

export type FeatureCard = {
  title: string;
  summary: string;
  icon: LucideIcon;
};

export type DocTrack = {
  title: string;
  summary: string;
  links: string[];
  icon: LucideIcon;
};

export type PackageEntry = {
  name: string;
  owner: string;
  status: string;
  summary: string;
  tags: string[];
};

export type BenchmarkCard = {
  title: string;
  metric: string;
  summary: string;
  command: string;
};

export type ContractCard = {
  title: string;
  producer: string;
  consumer: string;
  guarantee: string;
  icon: LucideIcon;
};

export type RouteItem = {
  title: string;
  path: string;
  summary: string;
  icon: LucideIcon;
};

export const portalNav: PortalNavItem[] = [
  { label: 'Docs', href: '#docs' },
  { label: 'Packages', href: '#packages' },
  { label: 'Benchmarks', href: '#benchmarks' },
  { label: 'Repair', href: '#repair' },
  { label: 'Context', href: '#context' },
  { label: 'Download', href: '#download' },
];

export const heroCode = `module main;

import std.cli;
import std.fs;
import std.path;
import std.report;

fn main() {
    let root: string = cli.arg_or(1, ".");
    let files: [string] = fs.list_files(root);
    let summary: string = report.workspace_summary(root, files);
    let output: string = path.join("target", "workspace-report.txt");

    fs.write_string(output, summary);
    print("report=" + output);
}`;

export const aiPayload = `{
  "rule_id": "path_join_expected_string",
  "repair_goal": "keep path construction explicit",
  "fixits": [
    "wrap path segments with path.join(...)",
    "write output through std.fs"
  ],
  "context_snippets": [
    "module: main",
    "flow: cli -> fs -> report"
  ]
}`;

export const brokenSliceSource = `fn main() -> i32 {
    let values: [i32; 3] = [1, 2, 3];
    let mut view: [i32] = values[0:2];
    view[0] = 9;
    return 0;
}`;

export const statCards: StatCard[] = [
  {
    label: 'Compiler surface',
    value: 'check/run/fmt/build/context',
    caption: '核心命令已经形成对外入口',
    tone: 'cyan',
  },
  {
    label: 'Repair cases',
    value: '35',
    caption: 'full manifest，含 smoke 子集',
    tone: 'green',
  },
  {
    label: 'AI lift snapshot',
    value: '+16.67pp',
    caption: 'base JSON 到 AI repair contract',
    tone: 'amber',
  },
  {
    label: 'Portal mode',
    value: 'v0',
    caption: '静态语言门户，registry 后续接入',
    tone: 'red',
  },
];

export const featureCards: FeatureCard[] = [
  {
    title: '显式语法',
    summary: 'AX 把类型、模块、错误边界和工具程序流程写得更窄、更稳定，减少模型猜测空间。',
    icon: Code2,
  },
  {
    title: '结构化诊断',
    summary: 'diagnostics JSON 为 CLI、编辑器、agent 和 benchmark 提供同一份可消费错误协议。',
    icon: FileJson,
  },
  {
    title: '架构上下文',
    summary: 'context view 输出 overview、boundaries、topology、flow、symbol、impact、evidence。',
    icon: Layers3,
  },
  {
    title: '修复证据链',
    summary: 'repair bundle、score、compare 和 Repair Archaeology 把修复过程变成可复盘资产。',
    icon: GitCompareArrows,
  },
];

export const docTracks: DocTrack[] = [
  {
    title: 'Getting started',
    summary: '从安装、hello.ax、项目模式到常用 axc 命令，服务第一次试用。',
    links: ['Quickstart', 'CLI commands', 'Project layout'],
    icon: TerminalSquare,
  },
  {
    title: 'Language guide',
    summary: '集中讲清楚 AX 语法、显式类型、模块、match、payload enum 和错误模型。',
    links: ['Syntax', 'Feature matrix', 'Representative samples'],
    icon: FileCode2,
  },
  {
    title: 'AI protocols',
    summary: '面向 agent 的 diagnostics、repair contract、context schema 和 evidence bundle。',
    links: ['Diagnostics schema', 'Repair adapter spec', 'Context protocol'],
    icon: BrainCircuit,
  },
  {
    title: 'Compiler internals',
    summary: '解释 frontend、AST/HIR/MIR、semantic、interpreter、build manifest 的稳定边界。',
    links: ['Architecture', 'Interface contracts', 'Validation matrix'],
    icon: Binary,
  },
];

export const packageEntries: PackageEntry[] = [
  {
    owner: 'axlang',
    name: 'std.text',
    status: 'candidate',
    summary: '字符串切分、归一化、连接和报告文本组装。',
    tags: ['stdlib', 'text', 'agent-output'],
  },
  {
    owner: 'axlang',
    name: 'std.cli',
    status: 'candidate',
    summary: 'argv、命令入口、参数默认值和小型 CLI 程序约定。',
    tags: ['stdlib', 'cli', 'tools'],
  },
  {
    owner: 'axlang',
    name: 'std.fs',
    status: 'candidate',
    summary: '文件读取、写入、目录扫描和 workspace 工具边界。',
    tags: ['stdlib', 'fs', 'host'],
  },
  {
    owner: 'axlang',
    name: 'std.report',
    status: 'candidate',
    summary: '把工具运行结果整理成稳定、可 diff、可审阅的报告格式。',
    tags: ['stdlib', 'report', 'benchmark'],
  },
  {
    owner: 'examples',
    name: 'project_payload_event_report',
    status: 'sample',
    summary: 'payload enum、match、跨模块类型和文件报告输出的项目级样例。',
    tags: ['enum', 'match', 'workload'],
  },
  {
    owner: 'examples',
    name: 'project_workspace_audit',
    status: 'sample',
    summary: '面向真实目录的 workspace 扫描、汇总和输出边界样例。',
    tags: ['workspace', 'fs', 'backend-tool'],
  },
];

export const benchmarkCards: BenchmarkCard[] = [
  {
    title: 'Cold / Base / AI feedback',
    metric: '23/30 -> 25/30 -> 30/30',
    summary: '同一批修复 case，只改变反馈协议，观察单轮修复稳定性。',
    command: 'scripts/compare-repair-feedback.ps1',
  },
  {
    title: 'Smoke repair loop',
    metric: '11 smoke cases',
    summary: '轻量回归入口，确保修复协议、评分脚本和样例输入没有漂移。',
    command: 'scripts/smoke-repair-benchmark.ps1',
  },
  {
    title: 'Repair Archaeology v0',
    metric: 'replayable history',
    summary: '把诊断、候选修复、评分和失败原因沉淀成可回放知识资产。',
    command: 'scripts/export-repair-archaeology.ps1',
  },
];

export const modePanels: Record<
  FeedbackMode,
  {
    label: string;
    command: string;
    result: string;
    payload: string;
    details: string[];
  }
> = {
  cold: {
    label: 'Cold prompt',
    command: 'repair without structured diagnostics',
    result: '源码与自然语言提示能修掉一部分问题，但缺少稳定修复目标。',
    payload: `{
  "mode": "cold",
  "input": "broken source only",
  "case_id": "slice_assignment_read_only",
  "attempt_budget": 1
}`,
    details: [
      '模型只能从源码里猜测 slice 语义',
      '候选修复容易改偏成数组可变性或索引问题',
      '评分仍回到 axc check/run 统一验证',
    ],
  },
  base: {
    label: 'Base diagnostics',
    command: 'axc check examples/slice_assignment.ax --json',
    result: '基础 JSON 把错误位置和消息稳定下来，但修复方向仍然偏薄。',
    payload: `{
  "code": "S0035",
  "message": "cannot assign through a slice view",
  "file": "examples/slice_assignment.ax",
  "span": { "start": 94, "end": 103 },
  "suggestion": "write to the original array or create a new value"
}`,
    details: [
      '错误码、文件与 span 可供工具链消费',
      '提示可以进入 adapter，但没有完整 repair contract',
      'base -> ai 的提升来自更明确的语义约束',
    ],
  },
  ai: {
    label: 'AI repair contract',
    command: 'axc check examples/slice_assignment.ax --json --ai',
    result: 'AI 增强 diagnostics 给出 rule_id、repair_goal、fixits 和上下文片段。',
    payload: `{
  "code": "S0035",
  "ai": {
    "rule_id": "slice_assignment_read_only",
    "repair_goal": "preserve slice as a read-only view",
    "focus_item": "view[0] assignment",
    "fixits": [
      "assign through values[0] before slicing",
      "construct a new array when mutation is required"
    ],
    "context_snippets": ["slices are read-only views"]
  }
}`,
    details: [
      'repair goal 直接约束候选方向',
      'fixits 把“怎么修”从文本提示升级为协议字段',
      '同一 case 在 deterministic replay 中通过',
    ],
  },
};

export const comparisonRows = [
  { mode: 'cold', passed: 23, total: 30, summary: '源码 + prompt' },
  { mode: 'base', passed: 25, total: 30, summary: '基础 JSON diagnostics' },
  { mode: 'ai', passed: 30, total: 30, summary: 'AI repair contract' },
] satisfies Array<{
  mode: FeedbackMode;
  passed: number;
  total: number;
  summary: string;
}>;

export const contracts: ContractCard[] = [
  {
    title: 'Diagnostics JSON',
    producer: 'axc check --json',
    consumer: 'CLI users, repair export, adapters',
    guarantee: '稳定 code/message/file/span/notes/suggestion 字段。',
    icon: FileJson,
  },
  {
    title: 'AI Diagnostics',
    producer: 'axc check --json --ai',
    consumer: 'repair benchmark, AI agents',
    guarantee: '可选 ai 对象承载 rule_id、repair_goal、fixits。',
    icon: Braces,
  },
  {
    title: 'Context JSON',
    producer: 'axc context',
    consumer: 'agents, docs, repair context',
    guarantee: 'overview/boundaries/evidence 等 view 共享稳定外壳。',
    icon: Network,
  },
  {
    title: 'Repair Bundle',
    producer: 'export-repair-benchmark.ps1',
    consumer: 'repair adapters, score scripts',
    guarantee: 'cold/base/ai bundle 与 prompt 形态保持可复跑。',
    icon: GitCompareArrows,
  },
];

export const routeItems: RouteItem[] = [
  {
    title: 'Sharp demo',
    path: 'docs/killer-demo.md',
    summary: '单个 slice_assignment_read_only case，演示同样坏例子在不同反馈模式下的修复差异。',
    icon: TerminalSquare,
  },
  {
    title: 'Benchmark showcase',
    path: 'docs/benchmark-showcase.md',
    summary: '30 case deterministic replay、cold/base/ai 对比和 context-enabled export 快照。',
    icon: Activity,
  },
  {
    title: 'Interface contracts',
    path: 'docs/interface-contracts.md',
    summary: 'diagnostics、context、build manifest、repair export 的稳定契约边界。',
    icon: ShieldCheck,
  },
  {
    title: 'Representative samples',
    path: 'docs/representative-samples.md',
    summary: '真实工具样例与 P2 阶段代表 workload。',
    icon: FileCode2,
  },
];

export const sampleTracks = [
  {
    title: 'Agent-generated CLI tools',
    description: '文本归一化、目录索引、发布快照等工具型 AX 程序。',
    icon: Boxes,
  },
  {
    title: 'Repairable automation scripts',
    description: '错误、候选修复、评分和对比被同一套协议固定。',
    icon: Route,
  },
  {
    title: 'Backend worker utilities',
    description: '先覆盖批处理、命令捕获、文件整理和构建辅助。',
    icon: PackageCheck,
  },
];

export const contextLayers = [
  'overview',
  'boundaries',
  'topology',
  'flow',
  'symbol',
  'impact',
  'evidence',
];

export const downloadRows = [
  {
    platform: 'Windows',
    level: 'full workflow support',
    command: 'cargo build --release',
  },
  {
    platform: 'Linux',
    level: 'core compiler/runtime support',
    command: 'cargo build --release && cargo test --lib',
  },
  {
    platform: 'macOS',
    level: 'planned after Linux core is stable',
    command: 'not committed yet',
  },
];

export const ecosystemMilestones = [
  {
    title: 'Portal v0',
    summary: '静态官网、文档入口、packages catalog、benchmark 展示和 Repair Workbench。',
    icon: Sparkles,
  },
  {
    title: 'Catalog v1',
    summary: '从 std/、examples/、AX.toml 自动生成模块说明和样例索引。',
    icon: FolderTree,
  },
  {
    title: 'Registry v2',
    summary: '在 AX 包系统、lockfile、AOT 和 publish contract 成型后接入社区包。',
    icon: Hammer,
  },
];
