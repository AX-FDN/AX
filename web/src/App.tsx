import { useMemo, useState } from 'react';
import {
  ArrowRight,
  BookOpen,
  CheckCircle2,
  Code2,
  Copy,
  Download,
  ExternalLink,
  FileJson,
  Gauge,
  Github,
  Layers3,
  Package,
  Play,
  Search,
  ShieldCheck,
  Sparkles,
  TerminalSquare,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import logoUrl from './assets/ax-logo.svg';
import {
  aiPayload,
  benchmarkCards,
  brokenSliceSource,
  comparisonRows,
  contextLayers,
  contracts,
  docTracks,
  downloadRows,
  ecosystemMilestones,
  featureCards,
  heroCode,
  modePanels,
  packageEntries,
  portalNav,
  routeItems,
  sampleTracks,
  statCards,
  type FeedbackMode,
} from './data';

const modes: FeedbackMode[] = ['cold', 'base', 'ai'];

function App() {
  const [mode, setMode] = useState<FeedbackMode>('ai');
  const [copied, setCopied] = useState<'hero' | 'repair' | null>(null);
  const activePanel = modePanels[mode];

  const progressSummary = useMemo(() => {
    const selected = comparisonRows.find((row) => row.mode === mode);
    if (!selected) {
      return '0%';
    }

    return `${Math.round((selected.passed / selected.total) * 100)}%`;
  }, [mode]);

  const copyText = async (kind: 'hero' | 'repair', value: string) => {
    await navigator.clipboard.writeText(value);
    setCopied(kind);
    window.setTimeout(() => setCopied(null), 1600);
  };

  return (
    <main className="app-shell">
      <header className="topbar">
        <a className="brand" href="#home" aria-label="AX portal home">
          <img src={logoUrl} alt="" />
          <span>
            <strong>AX</strong>
            <small>AI-first language portal</small>
          </span>
        </a>

        <nav className="topnav" aria-label="Primary">
          {portalNav.map((item) => (
            <a href={item.href} key={item.href}>
              {item.label}
            </a>
          ))}
        </nav>

        <a className="repo-link" href="https://github.com/AX-FDN/AX">
          <Github size={18} />
          <span>GitHub</span>
        </a>
      </header>

      <section className="hero-section" id="home">
        <div className="hero-copy">
          <div className="eyebrow">
            <Code2 size={16} />
            <span>AI-written backend tools</span>
          </div>
          <div className="hero-pills" aria-label="AX status">
            <span>v0 language portal</span>
            <span>35 repair cases</span>
            <span>AI-readable docs</span>
          </div>
          <h1>AX 是面向 Coding AI 的显式工具语言。</h1>
          <p>
            它把低歧义语法、结构化诊断、架构上下文、修复协议和 benchmark
            证据链放进同一条工具链，让 agent 更稳定地生成、理解、修改和验证代码。
          </p>
          <div className="hero-actions">
            <a href="#docs" className="primary-action">
              <BookOpen size={18} />
              <span>Get Started</span>
            </a>
            <a href="#packages" className="secondary-action">
              <Package size={18} />
              <span>Browse Packages</span>
            </a>
          </div>
          <div className="command-deck" aria-label="AX command highlights">
            <CommandPill icon={TerminalSquare} label="check" command="axc check --json --ai" />
            <CommandPill icon={Play} label="run" command="axc run examples/project_workspace_audit" />
            <CommandPill icon={Layers3} label="context" command="axc context evidence" />
          </div>
        </div>

        <div className="hero-visual" aria-label="AX portal preview">
          <div className="floating-card floating-card-top">
            <Sparkles size={18} />
            <span>repair_goal + fixits + context</span>
          </div>
          <div className="hero-terminal">
            <div className="terminal-tabs">
              <span>workspace_tool.ax</span>
              <button
                className="copy-button"
                type="button"
                onClick={() => copyText('hero', heroCode)}
                aria-label="Copy AX sample"
              >
                {copied === 'hero' ? <CheckCircle2 size={16} /> : <Copy size={16} />}
                <span>{copied === 'hero' ? 'Copied' : 'Copy'}</span>
              </button>
            </div>
            <pre className="code-block hero-code" aria-label="AX source sample">
              <code>{heroCode}</code>
            </pre>
          </div>
          <div className="floating-card floating-card-bottom">
            <ShieldCheck size={18} />
            <span>deterministic validation loop</span>
          </div>
        </div>
      </section>

      <section className="stat-strip" aria-label="AX project status">
        {statCards.map((stat) => (
          <article className={`metric-card tone-${stat.tone}`} key={stat.label}>
            <span>{stat.label}</span>
            <strong>{stat.value}</strong>
            <small>{stat.caption}</small>
          </article>
        ))}
      </section>

      <section className="feature-section" aria-label="AX advantages">
        {featureCards.map((feature) => (
          <article className="feature-card" key={feature.title}>
            <feature.icon size={24} />
            <h3>{feature.title}</h3>
            <p>{feature.summary}</p>
          </article>
        ))}
      </section>

      <section className="docs-section" id="docs">
        <SectionTitle
          kicker="documentation"
          title="从入门到协议，按语言项目方式组织。"
          icon={BookOpen}
        />
        <div className="doc-grid">
          {docTracks.map((track) => (
            <article className="doc-card" key={track.title}>
              <track.icon size={24} />
              <h3>{track.title}</h3>
              <p>{track.summary}</p>
              <ul>
                {track.links.map((link) => (
                  <li key={link}>{link}</li>
                ))}
              </ul>
            </article>
          ))}
        </div>
      </section>

      <section className="packages-section" id="packages">
        <SectionTitle
          kicker="package catalog v0"
          title="先展示官方库和代表样例，后续接入真正 registry。"
          icon={Package}
        />
        <div className="search-shell" aria-label="Package search preview">
          <Search size={18} />
          <span>Search modules: std.fs, std.report, project_payload_event_report</span>
        </div>
        <div className="package-grid">
          {packageEntries.map((entry) => (
            <article className="package-card" key={`${entry.owner}/${entry.name}`}>
              <div className="package-heading">
                <span>
                  {entry.owner} / <strong>{entry.name}</strong>
                </span>
                <small>{entry.status}</small>
              </div>
              <p>{entry.summary}</p>
              <div className="tag-row">
                {entry.tags.map((tag) => (
                  <span key={tag}>{tag}</span>
                ))}
              </div>
            </article>
          ))}
        </div>
      </section>

      <section className="benchmarks-section" id="benchmarks">
        <SectionTitle
          kicker="benchmark evidence"
          title="AX 的证据页重点展示修复稳定性，不只展示脚本。"
          icon={Gauge}
        />
        <div className="benchmark-grid">
          {benchmarkCards.map((card) => (
            <article className="benchmark-card" key={card.title}>
              <span>{card.metric}</span>
              <h3>{card.title}</h3>
              <p>{card.summary}</p>
              <code>{card.command}</code>
            </article>
          ))}
        </div>
      </section>

      <section className="repair-section" id="repair">
        <SectionTitle
          kicker="repair workbench"
          title="同一个坏例子，展示 cold / base / ai 三档反馈。"
          icon={FileJson}
        />
        <div className="workbench-grid">
          <div className="code-panel">
            <div className="panel-heading">
              <div>
                <span className="section-kicker">case</span>
                <h2>slice_assignment_read_only</h2>
              </div>
              <IconButton
                label={copied === 'repair' ? 'Copied source' : 'Copy source'}
                icon={copied === 'repair' ? CheckCircle2 : Copy}
                onClick={() => copyText('repair', brokenSliceSource)}
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
        </div>

        <div className="compare-section" aria-label="Repair comparison">
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
        </div>
      </section>

      <section className="context-section" id="context">
        <SectionTitle
          kicker="architecture context"
          title="六层上下文让 agent 知道边界、影响面和验证路径。"
          icon={TerminalSquare}
        />
        <div className="context-layout">
          <div className="payload-card">
            <span className="section-kicker">ai-facing bundle</span>
            <pre className="payload-block">
              <code>{aiPayload}</code>
            </pre>
          </div>
          <div className="context-list">
            {contextLayers.map((layer, index) => (
              <div className="context-row" key={layer}>
                <span>{String(index + 1).padStart(2, '0')}</span>
                <strong>{layer}</strong>
                <small>axc context {layer}</small>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="contracts-section">
        <SectionTitle
          kicker="stable contracts"
          title="官网默认展示的是可被工具链消费的接口。"
          icon={FileJson}
        />
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

      <section className="samples-section">
        <SectionTitle kicker="workloads" title="真实工具型样例负责证明 AX 不是玩具。" icon={Play} />
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

      <section className="download-section" id="download">
        <SectionTitle
          kicker="download"
          title="平台支持按成熟度分级，不把未完成能力包装成承诺。"
          icon={Download}
        />
        <div className="download-grid">
          {downloadRows.map((row) => (
            <article className="download-card" key={row.platform}>
              <h3>{row.platform}</h3>
              <p>{row.level}</p>
              <code>{row.command}</code>
            </article>
          ))}
        </div>
      </section>

      <section className="ecosystem-section">
        <SectionTitle
          kicker="ecosystem path"
          title="先做门户，再做目录，最后做真正社区 registry。"
          icon={ArrowRight}
        />
        <div className="milestone-grid">
          {ecosystemMilestones.map((milestone) => (
            <article className="milestone-card" key={milestone.title}>
              <milestone.icon size={24} />
              <h3>{milestone.title}</h3>
              <p>{milestone.summary}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="docs-section">
        <SectionTitle kicker="repository docs" title="可继续引用的仓库文档入口。" icon={BookOpen} />
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
      <Icon size={16} />
      <span>{label}</span>
      <code>{command}</code>
    </div>
  );
}

function SectionTitle({
  kicker,
  title,
  icon: Icon,
}: {
  kicker: string;
  title: string;
  icon: LucideIcon;
}) {
  return (
    <div className="section-title-row">
      <div>
        <span className="section-kicker">{kicker}</span>
        <h2>{title}</h2>
      </div>
      <Icon size={22} />
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
