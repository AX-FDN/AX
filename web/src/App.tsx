import { useMemo, useState } from 'react';
import {
  ArrowRight,
  BookOpen,
  CheckCircle2,
  ClipboardList,
  Code2,
  Copy,
  ExternalLink,
  FileJson,
  Gauge,
  Github,
  Layers3,
  Play,
  TerminalSquare,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import logoUrl from './assets/ax-logo.svg';
import {
  brokenSliceSource,
  comparisonRows,
  contracts,
  metrics,
  modePanels,
  routeItems,
  sampleTracks,
  type FeedbackMode,
} from './data';

const modes: FeedbackMode[] = ['cold', 'base', 'ai'];

function App() {
  const [mode, setMode] = useState<FeedbackMode>('ai');
  const [copied, setCopied] = useState(false);
  const activePanel = modePanels[mode];

  const progressSummary = useMemo(() => {
    const selected = comparisonRows.find((row) => row.mode === mode);
    if (!selected) {
      return '0%';
    }

    return `${Math.round((selected.passed / selected.total) * 100)}%`;
  }, [mode]);

  const handleCopySource = async () => {
    await navigator.clipboard.writeText(brokenSliceSource);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1600);
  };

  return (
    <main className="app-shell">
      <header className="topbar">
        <a className="brand" href="https://github.com/AX-FDN/AX" aria-label="AX GitHub repository">
          <img src={logoUrl} alt="" />
          <span>
            <strong>AX</strong>
            <small>Repair Workbench</small>
          </span>
        </a>

        <nav className="topnav" aria-label="Primary">
          <a href="#workbench">Workbench</a>
          <a href="#contracts">Contracts</a>
          <a href="#samples">Samples</a>
        </nav>

        <a className="repo-link" href="https://github.com/AX-FDN/AX">
          <Github size={18} />
          <span>GitHub</span>
        </a>
      </header>

      <section className="command-strip" aria-label="AX commands">
        <CommandPill icon={TerminalSquare} label="check" command="axc check --json --ai" />
        <CommandPill icon={Play} label="run" command="axc run examples/extract_markdown_headings.ax" />
        <CommandPill icon={Gauge} label="compare" command="compare-repair-feedback.ps1" />
      </section>

      <section className="dashboard" id="workbench">
        <div className="intro-panel">
          <div className="eyebrow">
            <Layers3 size={16} />
            <span>AI-first tool language</span>
          </div>
          <h1>把 diagnostics、repair contract 和 benchmark 证据放到同一个工作台。</h1>
          <p>
            AX 当前的核心卖点不是单个 hello world，而是可复跑的修复链：
            同一个坏例子、同一轮预算、同一套评分脚本，只改变反馈协议。
          </p>
          <div className="intro-actions">
            <a href="#repair-demo" className="primary-action">
              <FileJson size={18} />
              <span>查看 repair payload</span>
            </a>
            <a href="#docs" className="secondary-action">
              <BookOpen size={18} />
              <span>文档入口</span>
            </a>
          </div>
        </div>

        <div className="metric-grid" aria-label="Benchmark metrics">
          {metrics.map((metric) => (
            <article className={`metric-card tone-${metric.tone}`} key={metric.label}>
              <span>{metric.label}</span>
              <strong>{metric.value}</strong>
              <small>{metric.caption}</small>
            </article>
          ))}
        </div>
      </section>

      <section className="workbench-grid" id="repair-demo">
        <div className="code-panel">
          <div className="panel-heading">
            <div>
              <span className="section-kicker">case</span>
              <h2>slice_assignment_read_only</h2>
            </div>
            <IconButton
              label={copied ? 'Copied source' : 'Copy source'}
              icon={copied ? CheckCircle2 : Copy}
              onClick={handleCopySource}
            />
          </div>
          <pre className="code-block" aria-label="Broken AX source">
            <code>{brokenSliceSource}</code>
          </pre>
        </div>

        <div className="repair-panel">
          <div className="panel-heading">
            <div>
              <span className="section-kicker">feedback mode</span>
              <h2>{activePanel.label}</h2>
            </div>
            <span className="progress-pill">{progressSummary}</span>
          </div>

          <div className="mode-switch" role="tablist" aria-label="Feedback modes">
            {modes.map((item) => (
              <button
                className={item === mode ? 'active' : ''}
                key={item}
                onClick={() => setMode(item)}
                role="tab"
                aria-selected={item === mode}
              >
                {modePanels[item].label}
              </button>
            ))}
          </div>

          <div className="command-line">
            <Code2 size={16} />
            <span>{activePanel.command}</span>
          </div>

          <p className="result-copy">{activePanel.result}</p>

          <pre className="payload-block" aria-label={`${activePanel.label} payload`}>
            <code>{activePanel.payload}</code>
          </pre>

          <ul className="detail-list">
            {activePanel.details.map((detail) => (
              <li key={detail}>
                <CheckCircle2 size={16} />
                <span>{detail}</span>
              </li>
            ))}
          </ul>
        </div>
      </section>

      <section className="compare-section" aria-label="Repair comparison">
        <div className="section-title-row">
          <div>
            <span className="section-kicker">deterministic replay</span>
            <h2>同一批 case，三档反馈对比</h2>
          </div>
          <span className="snapshot-pill">snapshot 2026-04-27</span>
        </div>

        <div className="compare-grid">
          {comparisonRows.map((row) => {
            const width = `${(row.passed / row.total) * 100}%`;
            return (
              <button
                className={`compare-row ${row.mode === mode ? 'active' : ''}`}
                key={row.mode}
                onClick={() => setMode(row.mode)}
              >
                <span className="compare-mode">{modePanels[row.mode].label}</span>
                <span className="compare-summary">{row.summary}</span>
                <span className="compare-score">
                  {row.passed}/{row.total}
                </span>
                <span className="bar-track" aria-hidden="true">
                  <span style={{ width }} />
                </span>
              </button>
            );
          })}
        </div>
      </section>

      <section className="contract-section" id="contracts">
        <div className="section-title-row">
          <div>
            <span className="section-kicker">contracts</span>
            <h2>前端默认展示的稳定接口</h2>
          </div>
          <ClipboardList size={22} />
        </div>

        <div className="contract-grid">
          {contracts.map((contract) => (
            <article className="contract-card" key={contract.title}>
              <contract.icon size={22} />
              <h3>{contract.title}</h3>
              <dl>
                <div>
                  <dt>Producer</dt>
                  <dd>{contract.producer}</dd>
                </div>
                <div>
                  <dt>Consumer</dt>
                  <dd>{contract.consumer}</dd>
                </div>
              </dl>
              <p>{contract.guarantee}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="sample-section" id="samples">
        <div className="section-title-row">
          <div>
            <span className="section-kicker">workloads</span>
            <h2>真实工具型样例</h2>
          </div>
          <ArrowRight size={22} />
        </div>

        <div className="sample-grid">
          {sampleTracks.map((track) => (
            <article className="sample-card" key={track.title}>
              <track.icon size={24} />
              <h3>{track.title}</h3>
              <p>{track.description}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="docs-section" id="docs">
        <div className="section-title-row">
          <div>
            <span className="section-kicker">docs</span>
            <h2>提交说明里可以引用的文档入口</h2>
          </div>
          <BookOpen size={22} />
        </div>

        <div className="route-list">
          {routeItems.map((item) => (
            <a className="route-item" href={`https://github.com/AX-FDN/AX/blob/main/${item.path}`} key={item.path}>
              <item.icon size={20} />
              <span>
                <strong>{item.title}</strong>
                <small>{item.summary}</small>
              </span>
              <ExternalLink size={16} />
            </a>
          ))}
        </div>
      </section>
    </main>
  );
}

function CommandPill({
  icon: Icon,
  label,
  command,
}: {
  icon: LucideIcon;
  label: string;
  command: string;
}) {
  return (
    <div className="command-pill">
      <Icon size={17} />
      <span>{label}</span>
      <code>{command}</code>
    </div>
  );
}

function IconButton({
  label,
  icon: Icon,
  onClick,
}: {
  label: string;
  icon: LucideIcon;
  onClick: () => void;
}) {
  return (
    <button className="icon-button" type="button" title={label} aria-label={label} onClick={onClick}>
      <Icon size={18} />
    </button>
  );
}

export default App;
