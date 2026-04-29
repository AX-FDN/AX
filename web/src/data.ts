import {
  Activity,
  Boxes,
  Braces,
  FileCode2,
  FileJson,
  GitCompareArrows,
  Network,
  PackageCheck,
  Route,
  ShieldCheck,
  TerminalSquare,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

export type FeedbackMode = 'cold' | 'base' | 'ai';

export type Metric = {
  label: string;
  value: string;
  caption: string;
  tone: 'cyan' | 'green' | 'amber' | 'red';
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

export const brokenSliceSource = `fn main() -> i32 {
    let values: [i32; 3] = [1, 2, 3];
    let mut view: [i32] = values[0:2];
    view[0] = 9;
    return 0;
}`;

export const modePanels: Record<
  FeedbackMode,
  {
    label: string;
    command: string;
    status: string;
    result: string;
    payload: string;
    details: string[];
  }
> = {
  cold: {
    label: 'Cold prompt',
    command: 'repair without structured diagnostics',
    status: '23 / 30',
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
    status: '25 / 30',
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
    status: '30 / 30',
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

export const metrics: Metric[] = [
  {
    label: 'Full cases',
    value: '35',
    caption: '当前 full manifest',
    tone: 'cyan',
  },
  {
    label: 'Published replay',
    value: '30',
    caption: '已公开 deterministic snapshot',
    tone: 'green',
  },
  {
    label: 'Base -> AI lift',
    value: '+16.67pp',
    caption: '25/30 到 30/30',
    tone: 'amber',
  },
  {
    label: 'Smoke cases',
    value: '11',
    caption: '轻量回归入口',
    tone: 'red',
  },
];

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
