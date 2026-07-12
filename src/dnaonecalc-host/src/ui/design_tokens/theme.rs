use leptos::prelude::*;

pub const ONECALC_THEME_CSS: &str = r#"
:root {
  --oc-color-bg: #f4ede2;
  --oc-color-surface: #fffaf4;
  --oc-color-panel: #eee4d6;
  --oc-color-ink: #1f2a2c;
  --oc-color-muted: #6b695f;
  --oc-color-border: #d9cfbe;
  --oc-color-accent: #245d5a;
  --oc-color-accent-soft: #d7ebe7;
  --oc-color-night: #183132;
  --oc-color-card-edge: rgba(36, 93, 90, 0.18);
  --oc-color-warm: #b76545;
  --oc-color-warning: #c58b2f;
  --oc-color-success: #4f7b57;
  --oc-space-1: 0.25rem;
  --oc-space-2: 0.5rem;
  --oc-space-3: 0.75rem;
  --oc-space-4: 1rem;
  --oc-space-5: 1.5rem;
  --oc-radius-panel: 14px;
  --oc-radius-pill: 999px;
  --oc-shadow-panel: 0 12px 28px rgba(31, 42, 44, 0.10);
  --oc-shadow-strong: 0 18px 42px rgba(24, 49, 50, 0.14);
  --oc-font-ui: \"Segoe UI\", \"Inter\", sans-serif;
  --oc-font-mono: \"Cascadia Code\", \"Consolas\", monospace;
}

.onecalc-app {
  font-family: var(--oc-font-ui);
  color: var(--oc-color-ink);
  background: var(--oc-color-bg);
}

.onecalc-shell-frame {
  display: grid;
  grid-template-columns: 18rem 1fr;
  min-height: 100vh;
  background:
    radial-gradient(circle at top left, rgba(255, 255, 255, 0.45), transparent 32rem),
    linear-gradient(180deg, #f8f1e7 0%, #f0e6d7 100%);
}

.onecalc-shell-frame__rail {
  padding: var(--oc-space-5);
  background:
    linear-gradient(180deg, rgba(36, 93, 90, 0.06), transparent 16rem),
    var(--oc-color-panel);
  border-right: 1px solid var(--oc-color-border);
  display: grid;
  align-content: start;
  gap: var(--oc-space-4);
}

.onecalc-shell-frame__content {
  display: grid;
  grid-template-rows: auto 1fr auto;
  min-height: 0;
  min-width: 0;
}

.onecalc-shell-frame__context-bar {
  display: grid;
  grid-template-columns: minmax(13rem, 0.8fr) minmax(0, 1.4fr) auto;
  align-items: start;
  gap: var(--oc-space-3);
  padding: var(--oc-space-4) var(--oc-space-5);
  border-bottom: 1px solid var(--oc-color-border);
  background: rgba(255, 250, 242, 0.92);
  backdrop-filter: blur(10px);
}

.onecalc-shell-frame__brand-block,
.onecalc-shell-frame__active-card,
.onecalc-shell-frame__context-copy {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-shell-frame__brand-copy {
  margin: 0;
  color: var(--oc-color-muted);
  line-height: 1.5;
}

.onecalc-shell-frame__active-card {
  padding: var(--oc-space-4);
  border-radius: 18px;
  border: 1px solid rgba(36, 93, 90, 0.16);
  background: linear-gradient(160deg, rgba(36, 93, 90, 0.08), rgba(255, 252, 246, 0.96));
  box-shadow: var(--oc-shadow-panel);
}

.onecalc-shell-frame__workspace-manifest {
  display: grid;
  gap: var(--oc-space-3);
  padding: var(--oc-space-4);
  border-radius: 18px;
  border: 1px solid rgba(31, 28, 23, 0.1);
  background: linear-gradient(180deg, rgba(255, 253, 248, 0.98), rgba(245, 237, 225, 0.94));
  box-shadow: 0 10px 24px rgba(31, 28, 23, 0.06);
}

.onecalc-shell-frame__workspace-manifest-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--oc-space-2);
}

.onecalc-shell-frame__workspace-manifest-metric {
  display: grid;
  gap: 0.2rem;
  padding: 0.65rem 0.75rem;
  border-radius: 12px;
  border: 1px solid rgba(31, 28, 23, 0.08);
  background: rgba(255, 255, 255, 0.78);
}

.onecalc-shell-frame__workspace-manifest-metric > span {
  color: var(--oc-color-muted);
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-size: 0.64rem;
  font-weight: 700;
}

.onecalc-shell-frame__workspace-manifest-metric > strong {
  font-family: var(--oc-font-mono);
  font-size: 1.15rem;
  color: var(--oc-color-night);
}

.onecalc-shell-frame__workspace-manifest-note {
  margin: 0;
  color: var(--oc-color-muted);
  font-size: 0.82rem;
  line-height: 1.5;
}

.onecalc-shell-frame__active-meta,
.onecalc-shell-frame__active-capability,
.onecalc-shell-frame__context-copy {
  color: var(--oc-color-muted);
}

.onecalc-shell-frame__workspace-summary,
.onecalc-shell-frame__context-subtitle {
  color: var(--oc-color-muted);
  font-size: 0.9rem;
}

.onecalc-shell-frame__active-meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
}

.onecalc-shell-frame__active-meta > span,
.onecalc-shell-frame__active-capability {
  padding: 0.25rem 0.6rem;
  border-radius: var(--oc-radius-pill);
  border: 1px solid var(--oc-color-border);
  background: rgba(255, 255, 255, 0.7);
}

.onecalc-shell-frame__context-title {
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--oc-color-night);
}

.onecalc-shell-frame__context-facts,
.onecalc-shell-frame__footer {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-shell-frame__context-facts {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.onecalc-shell-frame__context-fact,
.onecalc-shell-frame__footer-fact {
  display: grid;
  gap: 0.2rem;
  padding: 0.65rem 0.8rem;
  border-radius: 12px;
  border: 1px solid var(--oc-color-border);
  background: rgba(255, 255, 255, 0.74);
}

.onecalc-shell-frame__context-fact[data-tone="accent"] {
  background: linear-gradient(180deg, rgba(36, 93, 90, 0.12), rgba(255, 255, 255, 0.82));
  border-color: rgba(36, 93, 90, 0.2);
}

.onecalc-shell-frame__footer-fact[data-tone="warning"] {
  background: linear-gradient(180deg, rgba(197, 139, 47, 0.14), rgba(255, 255, 255, 0.9));
  border-color: rgba(197, 139, 47, 0.28);
}

.onecalc-shell-frame__context-fact-label,
.onecalc-shell-frame__footer-fact-label {
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-size: 0.68rem;
  color: var(--oc-color-muted);
  font-weight: 700;
}

.onecalc-shell-frame__context-fact-value,
.onecalc-shell-frame__footer-fact-value {
  color: var(--oc-color-night);
  font-size: 0.92rem;
}

.onecalc-shell-frame__mode-switch {
  display: flex;
  gap: var(--oc-space-2);
  align-items: center;
}

.onecalc-shell-frame__configure-action {
  margin-left: var(--oc-space-2);
  padding: 0.4rem 0.9rem;
  border: 1px solid var(--oc-shell-mode-accent, var(--oc-color-accent));
  border-radius: var(--oc-radius-pill);
  background: rgba(255, 252, 246, 0.95);
  color: var(--oc-shell-mode-accent, var(--oc-color-accent));
  font-size: 0.78rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  cursor: pointer;
}

.onecalc-shell-frame__configure-action[data-open="true"] {
  background: var(--oc-shell-mode-accent, var(--oc-color-accent));
  color: var(--oc-shell-mode-accent-ink, #fff);
}

.onecalc-shell-frame {
  --oc-shell-mode-accent: var(--oc-color-accent);
  --oc-shell-mode-accent-soft: var(--oc-color-accent-soft);
  --oc-shell-mode-accent-ink: #ffffff;
}

.onecalc-shell-frame[data-active-mode="explore"] {
  --oc-shell-mode-accent: #1e4d4a;
  --oc-shell-mode-accent-soft: rgba(30, 77, 74, 0.12);
  --oc-shell-mode-accent-ink: #ffffff;
}

.onecalc-shell-frame[data-active-mode="inspect"] {
  --oc-shell-mode-accent: #3e5238;
  --oc-shell-mode-accent-soft: rgba(62, 82, 56, 0.14);
  --oc-shell-mode-accent-ink: #ffffff;
}

.onecalc-shell-frame[data-active-mode="workbench"] {
  --oc-shell-mode-accent: #b84532;
  --oc-shell-mode-accent-soft: rgba(184, 69, 50, 0.14);
  --oc-shell-mode-accent-ink: #ffffff;
}

.onecalc-shell-frame__mode-button {
  border: 1px solid var(--oc-color-border);
  border-radius: var(--oc-radius-pill);
  background: transparent;
  padding: 0.4rem 0.8rem;
  color: var(--oc-color-ink);
}

.onecalc-shell-frame__mode-button--active {
  background: var(--oc-shell-mode-accent);
  color: var(--oc-shell-mode-accent-ink);
  border-color: var(--oc-shell-mode-accent);
  box-shadow: 0 8px 18px rgba(31, 28, 23, 0.18);
}

.onecalc-shell-frame__mode-body {
  padding: var(--oc-space-4) var(--oc-space-5);
  min-height: 0;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.onecalc-shell-frame__mode-body > * {
  flex: 1 1 auto;
  min-height: 0;
  min-width: 0;
}

.onecalc-shell-frame__content-main {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  min-height: 0;
  min-width: 0;
  overflow: hidden;
}

.onecalc-shell-frame__content-main[data-drawer-open="true"] {
  background: linear-gradient(180deg, rgba(255, 252, 246, 0.7), rgba(243, 236, 223, 0.7));
}

.onecalc-shell-frame__footer {
  grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
  padding: var(--oc-space-3) var(--oc-space-5) var(--oc-space-4);
  border-top: 1px solid var(--oc-color-border);
  background: rgba(255, 251, 244, 0.92);
}

.onecalc-shell-frame__rail h1,
.onecalc-inspect-shell__header h1,
.onecalc-workbench-shell__header h1 {
  margin: 0;
  font-size: 1.7rem;
  letter-spacing: -0.03em;
}

.onecalc-inspect-shell__eyebrow,
.onecalc-workbench-shell__eyebrow,
.onecalc-shell-frame__eyebrow {
  text-transform: uppercase;
  letter-spacing: 0.14em;
  font-size: 0.72rem;
  color: var(--oc-color-muted);
  font-weight: 700;
}

.onecalc-inspect-shell__lead,
.onecalc-workbench-shell__lead {
  margin: 0;
  max-width: 58rem;
  color: var(--oc-color-muted);
  line-height: 1.65;
  font-size: 1rem;
}

.onecalc-shell-frame__space-list {
  list-style: none;
  padding: 0;
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-shell-frame__space-item {
  padding: var(--oc-space-3);
  border-radius: var(--oc-radius-panel);
  border: 1px solid var(--oc-color-border);
  background: var(--oc-color-surface);
}

.onecalc-shell-frame__space-item[data-state="recent"] {
  border-style: dashed;
  background: linear-gradient(180deg, rgba(255, 252, 246, 0.94), rgba(244, 237, 226, 0.9));
}

.onecalc-shell-frame__space-button {
  width: 100%;
  text-align: left;
  border: none;
  background: transparent;
  color: inherit;
  font: inherit;
  padding: 0;
  display: grid;
  gap: 0.2rem;
}

.onecalc-shell-frame__space-button-label {
  font-weight: 700;
}

.onecalc-shell-frame__space-button-meta,
.onecalc-shell-frame__space-button-packet {
  color: var(--oc-color-muted);
  font-size: 0.82rem;
}

.onecalc-shell-frame__space-reopen-tag {
  display: inline-flex;
  width: fit-content;
  margin-top: 0.25rem;
  padding: 0.15rem 0.45rem;
  border-radius: 999px;
  background: rgba(36, 93, 90, 0.1);
  color: var(--oc-color-accent);
  font-size: 0.66rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.onecalc-shell-frame__space-item--active {
  border-left: 4px solid var(--oc-shell-mode-accent);
  border-top: 1px solid var(--oc-shell-mode-accent-soft);
  border-right: 1px solid var(--oc-shell-mode-accent-soft);
  border-bottom: 1px solid var(--oc-shell-mode-accent-soft);
  background: linear-gradient(90deg, var(--oc-shell-mode-accent-soft), var(--oc-color-surface) 60%);
  box-shadow: var(--oc-shadow-panel);
}

.onecalc-shell-frame__rail-section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--oc-space-2);
  margin-top: var(--oc-space-3);
}

.onecalc-shell-frame__rail-action-buttons {
  display: flex;
  gap: 0.3rem;
}

.onecalc-shell-frame__rail-action-button {
  width: 1.8rem;
  height: 1.8rem;
  border-radius: 999px;
  border: 1px solid var(--oc-shell-mode-accent, var(--oc-color-accent));
  background: rgba(255, 252, 246, 0.95);
  color: var(--oc-shell-mode-accent, var(--oc-color-accent));
  font-size: 1.1rem;
  font-weight: 700;
  cursor: pointer;
  line-height: 1;
}

.onecalc-shell-frame__rail-action-button:hover {
  background: var(--oc-shell-mode-accent, var(--oc-color-accent));
  color: var(--oc-shell-mode-accent-ink, #fff);
}

.onecalc-shell-frame__rail-section {
  display: grid;
  gap: var(--oc-space-2);
  margin-top: var(--oc-space-2);
}

.onecalc-shell-frame__rail-section-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--oc-space-2);
  font-size: 0.68rem;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--oc-color-muted);
  font-weight: 700;
}

.onecalc-shell-frame__rail-section-count {
  padding: 0.1rem 0.45rem;
  border-radius: 999px;
  background: rgba(31, 28, 23, 0.08);
  color: var(--oc-color-muted);
  font-size: 0.66rem;
}

.onecalc-shell-frame__rail-section-empty {
  margin: 0;
  padding: 0.5rem 0.75rem;
  border-radius: 10px;
  border: 1px dashed rgba(31, 28, 23, 0.14);
  background: rgba(255, 252, 246, 0.6);
  color: var(--oc-color-muted);
  font-size: 0.78rem;
  font-style: italic;
}

.onecalc-shell-frame__space-button-header {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}

.onecalc-shell-frame__space-dirty-dot {
  display: inline-block;
  width: 0.55rem;
  height: 0.55rem;
  border-radius: 999px;
  background: var(--oc-color-warning, #c88d2e);
  box-shadow: 0 0 0 2px rgba(200, 141, 46, 0.18);
}

.onecalc-shell-frame__space-verdicts {
  display: flex;
  gap: 0.25rem;
  align-items: center;
  margin-top: 0.35rem;
}

.onecalc-shell-frame__space-verdict {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.2rem;
  height: 1.2rem;
  border-radius: 999px;
  font-family: var(--oc-font-mono);
  font-size: 0.6rem;
  font-weight: 800;
  border: 1px solid rgba(31, 28, 23, 0.15);
  background: rgba(255, 252, 246, 0.9);
  color: var(--oc-color-muted);
}

.onecalc-shell-frame__space-verdict[data-verdict="pass"] {
  border-color: rgba(79, 123, 87, 0.55);
  background: rgba(79, 123, 87, 0.18);
  color: #33532c;
}

.onecalc-shell-frame__space-verdict[data-verdict="fail"] {
  border-color: rgba(184, 69, 50, 0.55);
  background: rgba(184, 69, 50, 0.14);
  color: #a75842;
}

.onecalc-shell-frame__space-verdict[data-verdict="unobserved"] {
  border-style: dashed;
  color: var(--oc-color-muted);
}

.onecalc-shell-frame__space-verdict-lane {
  margin-left: 0.25rem;
  font-size: 0.64rem;
  color: var(--oc-color-muted);
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.onecalc-shell-frame__space-affordances {
  display: flex;
  gap: 0.25rem;
  margin-top: 0.35rem;
  opacity: 0;
  transition: opacity 120ms ease-out;
}

.onecalc-shell-frame__space-item:hover .onecalc-shell-frame__space-affordances,
.onecalc-shell-frame__space-item--active .onecalc-shell-frame__space-affordances {
  opacity: 1;
}

.onecalc-shell-frame__space-affordance {
  padding: 0.2rem 0.5rem;
  border-radius: 999px;
  border: 1px solid rgba(31, 28, 23, 0.14);
  background: rgba(255, 252, 246, 0.95);
  color: var(--oc-color-muted);
  font-size: 0.66rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  cursor: pointer;
}

.onecalc-shell-frame__space-affordance:hover {
  color: var(--oc-shell-mode-accent, var(--oc-color-accent));
  border-color: var(--oc-shell-mode-accent, var(--oc-color-accent));
}

.onecalc-shell-frame[data-active-mode] .onecalc-shell-frame__context-title {
  color: var(--oc-shell-mode-accent);
}

.onecalc-shell-frame[data-active-mode] .onecalc-shell-frame__active-card {
  border-left: 3px solid var(--oc-shell-mode-accent);
  padding-left: calc(var(--oc-space-3) - 2px);
}

.onecalc-shell-frame__context-bar {
  display: flex;
  flex-direction: column;
  gap: var(--oc-space-2);
  padding: 0.85rem 1.25rem 0.9rem;
  border-bottom: 1px solid rgba(31, 28, 23, 0.08);
  background: linear-gradient(180deg, rgba(255, 252, 246, 0.95), rgba(246, 239, 225, 0.9));
  backdrop-filter: blur(8px);
}

.onecalc-shell-frame__breadcrumb {
  display: flex;
  align-items: center;
  gap: 0.45rem;
  flex-wrap: wrap;
  font-size: 0.8rem;
  color: var(--oc-color-muted);
}

.onecalc-shell-frame__breadcrumb-segment {
  padding: 0.2rem 0.4rem;
  border-radius: 6px;
  color: var(--oc-color-ink);
  font-weight: 600;
}

.onecalc-shell-frame__breadcrumb-segment--space {
  color: var(--oc-color-ink);
  font-weight: 700;
}

.onecalc-shell-frame__breadcrumb-segment--mode {
  color: var(--oc-shell-mode-accent, var(--oc-color-accent));
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  font-size: 0.72rem;
  padding: 0.2rem 0.55rem;
  background: var(--oc-shell-mode-accent-soft, rgba(36, 93, 90, 0.12));
  border-radius: 999px;
}

.onecalc-shell-frame__breadcrumb-separator {
  color: var(--oc-color-muted);
  opacity: 0.6;
}

.onecalc-shell-frame__scope-strip {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.onecalc-shell-frame__scope-segment {
  display: inline-flex;
  align-items: baseline;
  gap: 0.35rem;
  padding: 0.25rem 0.55rem;
  border-radius: 8px;
  border: 1px solid rgba(31, 28, 23, 0.1);
  background: rgba(255, 252, 246, 0.92);
  font-size: 0.78rem;
  color: var(--oc-color-ink);
}

.onecalc-shell-frame__scope-segment[data-status="live"] {
  border-color: rgba(79, 123, 87, 0.35);
  background: rgba(79, 123, 87, 0.08);
}

.onecalc-shell-frame__scope-segment[data-status="not-implemented"] {
  border-style: dashed;
  border-color: rgba(183, 101, 69, 0.4);
  background: rgba(183, 101, 69, 0.06);
  color: #a75842;
}

.onecalc-shell-frame__scope-segment[data-status="not-implemented"] .onecalc-shell-frame__scope-segment-value {
  text-decoration: underline dotted rgba(183, 101, 69, 0.55);
  text-underline-offset: 3px;
}

.onecalc-shell-frame__scope-segment-label {
  color: var(--oc-color-muted);
  text-transform: uppercase;
  font-size: 0.64rem;
  letter-spacing: 0.06em;
  font-weight: 700;
}

.onecalc-shell-frame__scope-segment-value {
  font-family: var(--oc-font-mono);
  font-weight: 700;
}

.onecalc-shell-frame__space-pin {
  margin-left: var(--oc-space-2);
  color: var(--oc-color-warm);
  font-size: 0.85rem;
}

.onecalc-explore-shell__body,
.onecalc-inspect-shell__body,
.onecalc-workbench-shell__body {
  display: grid;
  gap: var(--oc-space-4);
}

.onecalc-explore-shell__body {
  grid-template-columns: minmax(0, 2fr) minmax(18rem, 1.5fr) minmax(16rem, 1fr);
  align-items: stretch;
  min-height: 0;
  flex: 1 1 auto;
}

.onecalc-explore-shell {
  display: flex;
  flex-direction: column;
  min-height: 0;
  flex: 1 1 auto;
}

.onecalc-explore-shell__body-column {
  display: flex;
  flex-direction: column;
  gap: var(--oc-space-3);
  min-height: 0;
  min-width: 0;
  overflow: hidden;
}

.onecalc-explore-shell__body-column > * {
  min-height: 0;
}

.onecalc-inspect-shell__column,
.onecalc-workbench-shell__column {
  display: grid;
  gap: var(--oc-space-4);
  align-content: start;
}

.onecalc-inspect-shell__body {
  grid-template-columns: minmax(16rem, 0.72fr) minmax(0, 1.28fr) minmax(18rem, 0.9fr);
}

.onecalc-workbench-shell__body {
  grid-template-columns: minmax(0, 1.18fr) minmax(18rem, 0.86fr) minmax(18rem, 0.86fr);
}

.onecalc-explore-shell__editor-panel,
.onecalc-explore-shell__result-panel,
.onecalc-explore-shell__help-panel,
.onecalc-inspect-shell__walk-cluster,
.onecalc-inspect-shell__summary-cluster,
.onecalc-inspect-shell__summary-card,
.onecalc-workbench-shell__outcome-card,
.onecalc-workbench-shell__lineage-card,
.onecalc-workbench-shell__actions-card,
.onecalc-workbench-shell__evidence-card,
.onecalc-workbench-shell__catalog-card,
.onecalc-workbench-shell__compare-card,
.onecalc-workbench-shell__replay-card {
  background: var(--oc-color-surface);
  border: 1px solid var(--oc-color-border);
  border-radius: var(--oc-radius-panel);
  padding: var(--oc-space-4);
  box-shadow: var(--oc-shadow-panel);
}

.onecalc-explore-shell__editor-panel,
.onecalc-inspect-shell__walk-cluster {
  border-color: var(--oc-color-card-edge);
}

.onecalc-explore-shell__panel-header,
.onecalc-explore-shell__editor-summary-row,
.onecalc-explore-shell__result-metric,
.onecalc-explore-shell__array-preview-header,
.onecalc-inspect-shell__meta,
.onecalc-workbench-shell__meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
  align-items: center;
}

.onecalc-inspect-shell__header,
.onecalc-workbench-shell__header {
  display: grid;
  gap: var(--oc-space-3);
  margin-bottom: var(--oc-space-5);
}

.onecalc-inspect-shell__overview-deck,
.onecalc-workbench-shell__overview-deck {
  display: grid;
  gap: var(--oc-space-3);
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.onecalc-inspect-shell__overview-card,
.onecalc-workbench-shell__overview-card {
  display: grid;
  gap: var(--oc-space-2);
  padding: var(--oc-space-4);
  border-radius: 18px;
  border: 1px solid rgba(31, 28, 23, 0.1);
  background: linear-gradient(180deg, rgba(255, 253, 248, 0.98), rgba(244, 236, 223, 0.96));
  box-shadow: 0 16px 34px rgba(31, 28, 23, 0.06);
}

.onecalc-inspect-shell__overview-card strong,
.onecalc-workbench-shell__overview-card strong {
  font-size: 1.08rem;
  color: var(--oc-color-night);
}

.onecalc-inspect-shell__overview-card p,
.onecalc-workbench-shell__overview-card p {
  margin: 0;
  color: var(--oc-color-muted);
  line-height: 1.55;
}

.onecalc-explore-shell__panel-intro {
  display: grid;
  gap: var(--oc-space-1);
  margin-bottom: var(--oc-space-3);
}

.onecalc-explore-shell__panel-intro p {
  margin: 0;
  color: var(--oc-color-muted);
  line-height: 1.6;
}

.onecalc-explore-shell__editor-summary-row,
.onecalc-explore-shell__assist-meta {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
  gap: var(--oc-space-2);
  margin-bottom: var(--oc-space-3);
}

.onecalc-explore-shell__status-card,
.onecalc-explore-shell__assist-metric,
.onecalc-inspect-shell__source-card {
  display: grid;
  gap: var(--oc-space-1);
  padding: var(--oc-space-3);
  border-radius: 12px;
  border: 1px solid var(--oc-color-border);
  background: linear-gradient(180deg, #fffdf8, #fbf4e7);
}

.onecalc-explore-shell__status-label,
.onecalc-explore-shell__metric-label,
.onecalc-inspect-shell__source-label,
.onecalc-inspect-shell__walk-node-value-label,
.onecalc-explore-shell__hero-result-label {
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-size: 0.72rem;
  color: var(--oc-color-muted);
  font-weight: 700;
}

.onecalc-explore-shell__scenario-label,
.onecalc-explore-shell__truth-chip,
.onecalc-inspect-shell__meta > span,
.onecalc-workbench-shell__meta > span,
.onecalc-explore-shell__array-preview-badge {
  padding: 0.25rem 0.6rem;
  border-radius: var(--oc-radius-pill);
  background: #fbf6ed;
  border: 1px solid var(--oc-color-border);
  font-size: 0.85rem;
}

.onecalc-explore-shell__truth-chip,
.onecalc-explore-shell__array-preview-badge {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
}

.onecalc-explore-shell__trace-summary,
.onecalc-explore-shell__blocked-reason,
.onecalc-inspect-shell__context-card,
.onecalc-workbench-shell__import-surface,
.onecalc-workbench-shell__catalog-card,
.onecalc-workbench-shell__evidence-card {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-explore-shell__editor-note {
  display: grid;
  gap: 0.2rem;
  margin-bottom: var(--oc-space-3);
  padding: var(--oc-space-3);
  border-radius: 12px;
  border: 1px solid rgba(36, 93, 90, 0.16);
  background: linear-gradient(180deg, rgba(36, 93, 90, 0.08), rgba(255, 252, 246, 0.92));
}

.onecalc-explore-shell__editor-note-detail {
  color: var(--oc-color-muted);
  font-size: 0.88rem;
  line-height: 1.5;
}

.onecalc-explore-shell__blocked-reason {
  color: #8a5f19;
  background: #f4e4c6;
  border: 1px solid #e1c48c;
  border-radius: 12px;
  padding: var(--oc-space-2) var(--oc-space-3);
}

.onecalc-explore-shell__result-metric {
  justify-content: space-between;
  padding: var(--oc-space-2) var(--oc-space-3);
  border: 1px solid var(--oc-color-border);
  border-radius: 10px;
  background: #fbf6ed;
}

.onecalc-explore-shell__hero-result {
  display: grid;
  gap: var(--oc-space-1);
  margin-bottom: var(--oc-space-3);
  padding: var(--oc-space-4);
  border-radius: 14px;
  border: 1px solid var(--oc-color-card-edge);
  background: linear-gradient(160deg, rgba(36, 93, 90, 0.08), rgba(255, 255, 255, 0.92));
  box-shadow: var(--oc-shadow-panel);
}

.onecalc-explore-shell__hero-result-value {
  font-family: var(--oc-font-mono);
  font-size: 1.5rem;
  line-height: 1.2;
  color: var(--oc-color-night);
}

.onecalc-explore-shell__result-state-chip {
  padding: 0.35rem 0.8rem;
  border-radius: var(--oc-radius-pill);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  font-weight: 700;
}

.onecalc-explore-shell__array-preview {
  display: grid;
  gap: var(--oc-space-2);
  padding-top: var(--oc-space-3);
  border-top: 1px solid var(--oc-color-border);
}

.onecalc-explore-shell__array-grid {
  display: grid;
  gap: var(--oc-space-1);
}

.onecalc-explore-shell__array-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(3rem, 1fr));
  gap: var(--oc-space-1);
}

.onecalc-explore-shell__array-cell {
  padding: var(--oc-space-2);
  border-radius: 8px;
  background: #fbf6ed;
  border: 1px solid var(--oc-color-border);
  font-family: var(--oc-font-mono);
  text-align: center;
}

.onecalc-explore-shell__editor-text,
.onecalc-inspect-shell__source,
.onecalc-workbench-shell__evidence-source {
  font-family: var(--oc-font-mono);
  white-space: pre-wrap;
  background: #fbf6ed;
  border-radius: 10px;
  padding: var(--oc-space-3);
}

.onecalc-explore-shell__syntax-runs {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-1);
  font-family: var(--oc-font-mono);
}

.onecalc-formula-editor-surface__body {
  position: relative;
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-formula-editor-surface__native-input-layer,
.onecalc-formula-editor-surface__overlay-layer {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-formula-editor-surface__textarea {
  min-height: 10rem;
  width: 100%;
  padding: var(--oc-space-3);
  border-radius: 10px;
  border: 1px solid var(--oc-color-border);
  background: linear-gradient(180deg, #fffefb, #fef8ef);
  color: var(--oc-color-ink);
  font-family: var(--oc-font-mono);
  box-shadow: inset 0 1px 2px rgba(31, 42, 44, 0.06);
}

.onecalc-formula-editor-surface__toolbar,
.onecalc-formula-editor-surface__footer,
.onecalc-formula-editor-surface__editor-state,
.onecalc-formula-editor-surface__diagnostic-markers,
.onecalc-formula-editor-surface__inline-diagnostic-spans {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
}

.onecalc-formula-editor-surface__syntax-layer,
.onecalc-formula-editor-surface__selection-indicator,
.onecalc-formula-editor-surface__caret-indicator,
.onecalc-formula-editor-surface__scroll-indicator,
.onecalc-formula-editor-surface__diagnostic-markers {
  font-family: var(--oc-font-mono);
  font-size: 0.9rem;
}

.onecalc-formula-editor-surface__selection-indicator,
.onecalc-formula-editor-surface__caret-indicator,
.onecalc-formula-editor-surface__scroll-indicator,
.onecalc-formula-editor-surface__diagnostic-markers {
  color: var(--oc-color-muted);
}

.onecalc-formula-editor-surface__diagnostic-marker {
  display: inline-block;
  margin-right: var(--oc-space-2);
  color: var(--oc-color-warm);
}

.onecalc-formula-editor-surface__assist-indicator,
.onecalc-formula-editor-surface__inline-diagnostic {
  border: 1px solid var(--oc-color-border);
  border-radius: 10px;
  background: rgba(255, 250, 242, 0.95);
  padding: var(--oc-space-2);
}

.onecalc-formula-editor-surface__completion-popup,
.onecalc-formula-editor-surface__signature-help-popup {
  display: grid;
  gap: var(--oc-space-2);
  margin-top: var(--oc-space-2);
  min-width: 16rem;
  padding: var(--oc-space-2);
  border: 1px solid var(--oc-color-border);
  border-radius: 12px;
  background: rgba(255, 250, 242, 0.98);
  box-shadow: var(--oc-shadow-panel);
}

.onecalc-formula-editor-surface__completion-popup {
  max-height: calc(1.9rem * 8 + 1rem);
  overflow-y: auto;
  overscroll-behavior: contain;
}

.onecalc-formula-editor-surface__popup-container {
  z-index: 4;
  pointer-events: auto;
}

.onecalc-formula-editor-surface__popup-container--signature {
  pointer-events: none;
}

.onecalc-formula-editor-surface__popup-container--signature .onecalc-formula-editor-surface__signature-help-popup {
  min-width: 12rem;
  max-width: 28rem;
  margin-top: 0;
  padding: 0.5rem 0.75rem;
  border-color: rgba(255, 255, 255, 0.15);
  background: var(--oc-color-night);
  color: #faf7f1;
  box-shadow: 0 14px 28px rgba(24, 49, 50, 0.32);
  font-family: var(--oc-font-mono);
  font-size: 0.82rem;
}

.onecalc-formula-editor-surface__popup-container--signature [data-role="signature-help-callee"] {
  color: var(--oc-color-amber, #c88d2e);
  font-weight: 700;
  margin-right: 0.35rem;
}

.onecalc-formula-editor-surface__popup-container--signature .onecalc-signature-argument {
  color: rgba(250, 247, 241, 0.72);
}

.onecalc-formula-editor-surface__popup-container--signature .onecalc-signature-argument--active {
  color: #fff;
  font-weight: 700;
  border-bottom: 1px solid #c88d2e;
}

.onecalc-formula-editor-surface__popup-container[data-focused-assist="completion"] .onecalc-formula-editor-surface__completion-popup {
  border-color: var(--oc-color-accent);
}

.onecalc-formula-editor-surface__completion-item {
  text-align: left;
  border: 1px solid var(--oc-color-border);
  border-radius: 10px;
  background: var(--oc-color-surface);
  padding: var(--oc-space-2) var(--oc-space-3);
  color: var(--oc-color-ink);
}

.onecalc-formula-editor-surface__completion-item[data-selected="true"],
.onecalc-formula-editor-surface__completion-item[data-active-row="true"] {
  border-color: var(--oc-color-accent);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  font-weight: 600;
}

.onecalc-formula-editor-surface__completion-item:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: 2px;
}

.onecalc-signature-form {
  display: inline-flex;
  flex-wrap: wrap;
  gap: 0;
}

.onecalc-signature-argument {
  padding: 0 0.1rem;
  border-radius: 6px;
}

.onecalc-signature-argument--active {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  font-weight: 700;
}

.onecalc-explore-shell__function-help {
  display: grid;
  gap: var(--oc-space-3);
  margin-top: var(--oc-space-3);
  padding: var(--oc-space-3);
  border-radius: 14px;
  border: 1px solid var(--oc-color-card-edge);
  background: linear-gradient(180deg, #fffdf9, #f8efe3);
}

.onecalc-explore-shell__function-help-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--oc-space-3);
}

.onecalc-explore-shell__function-help-status {
  padding: 0.2rem 0.5rem;
  border-radius: var(--oc-radius-pill);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  font-size: 0.85rem;
}

.onecalc-explore-shell__function-help-status--limited {
  background: #f4e4c6;
  color: #8a5f19;
}

.onecalc-explore-shell__function-help-description {
  margin: 0;
  color: var(--oc-color-muted);
}

.onecalc-explore-shell__function-help-signatures,
.onecalc-explore-shell__function-help-arguments,
.onecalc-explore-shell__selected-proposal {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-explore-shell__function-help-signature,
.onecalc-explore-shell__function-help-argument {
  padding: var(--oc-space-2) var(--oc-space-3);
  border-radius: 10px;
  background: #fbf6ed;
  border: 1px solid var(--oc-color-border);
}

.onecalc-explore-shell__selected-proposal {
  padding: var(--oc-space-3);
  border-radius: 14px;
  border: 1px solid var(--oc-color-card-edge);
  background: linear-gradient(180deg, #fffef9, #f4ecdf);
}

.onecalc-explore-shell__selected-proposal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--oc-space-2);
}

.onecalc-explore-shell__function-help-argument--active {
  border-color: var(--oc-color-accent);
  background: var(--oc-color-accent-soft);
}

.onecalc-token[data-token-role=\"operator\"] { color: var(--oc-color-warm); }
.onecalc-token[data-token-role=\"function\"] { color: var(--oc-color-accent); font-weight: 600; }
.onecalc-token[data-token-role=\"number\"] { color: var(--oc-color-warning); }
.onecalc-token[data-token-role=\"delimiter\"] { color: var(--oc-color-muted); }
.onecalc-token[data-token-role=\"identifier\"] { color: var(--oc-color-success); }

.onecalc-inspect-shell__walk,
.onecalc-inspect-shell__walk-node-children {
  list-style: none;
  padding-left: var(--oc-space-4);
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-inspect-shell__walk-node-header {
  display: flex;
  justify-content: space-between;
  gap: var(--oc-space-2);
}

.onecalc-inspect-shell__walk-node {
  padding: var(--oc-space-3);
  border-radius: 12px;
  border: 1px solid var(--oc-color-border);
  background: linear-gradient(180deg, #fffef9, #f5ede2);
}

.onecalc-inspect-shell__walk-node-state {
  padding: 0.25rem 0.6rem;
  border-radius: var(--oc-radius-pill);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  font-size: 0.82rem;
  font-weight: 700;
}

.onecalc-inspect-shell__walk-node-value {
  display: grid;
  gap: var(--oc-space-1);
  margin-top: var(--oc-space-2);
}

.onecalc-inspect-shell__retained-context,
.onecalc-workbench-shell__import-field-group,
.onecalc-workbench-shell__import-outcome-guide {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-inspect-shell__source-stack,
.onecalc-inspect-shell__gap-board {
  display: grid;
  gap: var(--oc-space-3);
}

.onecalc-inspect-shell__retained-context-header,
.onecalc-workbench-shell__catalog-item-header,
.onecalc-inspect-shell__comparison-record-header,
.onecalc-workbench-shell__comparison-record-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--oc-space-3);
}

.onecalc-inspect-shell__retained-context-badges,
.onecalc-inspect-shell__comparison-record-badges,
.onecalc-workbench-shell__comparison-record-badges,
.onecalc-workbench-shell__catalog-item-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
}

.onecalc-inspect-shell__retained-context-badges > span,
.onecalc-inspect-shell__comparison-record-badges > span,
.onecalc-workbench-shell__comparison-record-badges > span,
.onecalc-workbench-shell__catalog-item-header > span,
.onecalc-workbench-shell__outcome-chip {
  padding: 0.3rem 0.75rem;
  border-radius: var(--oc-radius-pill);
  border: 1px solid var(--oc-color-border);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  font-weight: 700;
  font-size: 0.82rem;
}

.onecalc-inspect-shell__comparison-board,
.onecalc-inspect-shell__explain-board,
.onecalc-workbench-shell__compare-card,
.onecalc-workbench-shell__replay-card,
.onecalc-workbench-shell__actions-card {
  display: grid;
  gap: var(--oc-space-3);
}

.onecalc-inspect-shell__summary-grid,
.onecalc-inspect-shell__comparison-grid,
.onecalc-inspect-shell__explain-grid,
.onecalc-workbench-shell__comparison-grid,
.onecalc-workbench-shell__score-grid {
  display: grid;
  gap: var(--oc-space-3);
}

.onecalc-inspect-shell__comparison-grid,
.onecalc-inspect-shell__summary-grid,
.onecalc-workbench-shell__comparison-grid,
.onecalc-workbench-shell__score-grid {
  grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr));
}

.onecalc-inspect-shell__comparison-record,
.onecalc-inspect-shell__explain-record,
.onecalc-workbench-shell__comparison-record,
.onecalc-workbench-shell__explain-record,
.onecalc-workbench-shell__score-card,
.onecalc-workbench-shell__catalog-item {
  display: grid;
  gap: var(--oc-space-2);
  padding: var(--oc-space-3);
  border-radius: 14px;
  border: 1px solid var(--oc-color-border);
  background: linear-gradient(180deg, #fffef9, #f5ede2);
}

.onecalc-inspect-shell__comparison-record[data-projection-gap="true"],
.onecalc-workbench-shell__comparison-record[data-projection-gap="true"] {
  border-color: #d59b5b;
  background: linear-gradient(180deg, #fff9ed, #f7ead3);
}

.onecalc-inspect-shell__comparison-lane,
.onecalc-workbench-shell__comparison-lane {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--oc-space-2);
}

.onecalc-inspect-shell__comparison-lane-card,
.onecalc-workbench-shell__comparison-lane-card {
  display: grid;
  gap: var(--oc-space-1);
  padding: var(--oc-space-2);
  border-radius: 10px;
  border: 1px solid var(--oc-color-border);
  background: rgba(255, 255, 255, 0.72);
}

.onecalc-inspect-shell__comparison-detail,
.onecalc-workbench-shell__comparison-label,
.onecalc-workbench-shell__catalog-item,
.onecalc-inspect-shell__retained-context {
  color: var(--oc-color-muted);
}

.onecalc-workbench-shell__hero-outcome {
  display: grid;
  gap: var(--oc-space-1);
  padding: var(--oc-space-4);
  border-radius: 14px;
  border: 1px solid rgba(36, 93, 90, 0.18);
  background: linear-gradient(160deg, rgba(36, 93, 90, 0.09), rgba(255, 255, 255, 0.94));
}

.onecalc-workbench-shell__catalog-item-actions button,
.onecalc-shell-frame__mode-button,
.onecalc-workbench-shell__import-buttons button,
.onecalc-workbench-shell__catalog-card button,
.onecalc-workbench-shell__bundle-import-surface button {
  cursor: pointer;
}

.onecalc-workbench-shell__catalog-item-actions,
.onecalc-workbench-shell__import-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
}

.onecalc-workbench-shell__catalog-item-actions button,
.onecalc-workbench-shell__import-buttons button,
.onecalc-workbench-shell__bundle-import-surface button {
  border: 1px solid var(--oc-color-border);
  border-radius: 10px;
  background: #fffdf8;
  color: var(--oc-color-ink);
  padding: 0.55rem 0.9rem;
  font: inherit;
}

.onecalc-inspect-shell__context-card {
  margin-bottom: var(--oc-space-3);
  padding: var(--oc-space-3);
  border-radius: 12px;
  background: #fbf6ed;
  border: 1px solid var(--oc-color-border);
}

.onecalc-inspect-shell__walk-intro,
.onecalc-workbench-shell__outcome-ledger {
  display: grid;
  gap: var(--oc-space-2);
  padding: var(--oc-space-3);
  border-radius: 12px;
  border: 1px solid rgba(36, 93, 90, 0.15);
  background: linear-gradient(180deg, rgba(36, 93, 90, 0.08), rgba(255, 252, 246, 0.9));
  color: var(--oc-color-muted);
}

.onecalc-inspect-shell__summary-cluster {
  align-content: start;
}

.onecalc-inspect-shell__retained-context {
  margin-bottom: var(--oc-space-3);
  padding: var(--oc-space-3);
  border-radius: 12px;
  border: 1px solid var(--oc-color-border);
  background: #fbf6ed;
}

.onecalc-workbench-shell__import-surface {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-inspect-shell__column--left .onecalc-inspect-shell__source-stack,
.onecalc-inspect-shell__column--summary .onecalc-inspect-shell__summary-cluster,
.onecalc-workbench-shell__column--support .onecalc-workbench-shell__replay-card,
.onecalc-workbench-shell__column--actions .onecalc-workbench-shell__actions-card {
  position: sticky;
  top: 0;
}

.onecalc-workbench-shell__bundle-import-textarea {
  min-height: 12rem;
  width: 100%;
  padding: var(--oc-space-3);
  border-radius: 12px;
  border: 1px solid rgba(46, 62, 82, 0.18);
  background: rgba(255, 250, 243, 0.92);
  color: var(--oc-color-ink);
  font: 0.9rem/1.45 "Consolas", "SFMono-Regular", monospace;
  resize: vertical;
}

.onecalc-workbench-shell__import-label {
  font-weight: 600;
}

.onecalc-workbench-shell__import-help {
  color: var(--oc-color-muted);
  font-size: 0.9rem;
}

.onecalc-workbench-shell__import-outcome-guide {
  margin-top: var(--oc-space-2);
  padding: var(--oc-space-3);
  border-radius: 12px;
  background: #fbf6ed;
  border: 1px solid var(--oc-color-border);
}

.onecalc-workbench-shell__import-buttons {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
}

[data-role="retained-import-upstream-gap-summary"],
[data-role="inspect-retained-upstream-gap-summary"] {
  display: grid;
  gap: var(--oc-space-2);
  margin: 0;
  padding-left: 1.25rem;
  color: #8d4f2f;
}

.onecalc-explore-shell__header-copy,
.onecalc-workbench-shell__header-copy,
.onecalc-inspect-shell__header-copy {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--oc-space-4);
}

.onecalc-explore-shell__hero-badges {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
}

.onecalc-explore-shell__hero-badges > span,
.onecalc-workbench-shell__meta > span,
.onecalc-inspect-shell__meta > span {
  padding: 0.35rem 0.8rem;
  border-radius: var(--oc-radius-pill);
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 251, 245, 0.92);
  box-shadow: 0 6px 18px rgba(31, 28, 23, 0.05);
}

.onecalc-workbench-shell__overview-card[data-role="workbench-overview-outcome"],
.onecalc-inspect-shell__overview-card[data-role="inspect-overview-source"] {
  background: linear-gradient(155deg, rgba(36, 93, 90, 0.11), rgba(255, 252, 246, 0.96));
}

.onecalc-explore-shell__section-accent,
.onecalc-workbench-shell__section-accent,
.onecalc-inspect-shell__section-accent {
  width: 0.3rem;
  height: 2rem;
  border-radius: 999px;
  background: linear-gradient(180deg, var(--oc-color-warm), var(--oc-color-warning));
  box-shadow: 0 6px 14px rgba(183, 101, 69, 0.25);
  margin-bottom: var(--oc-space-2);
}

.onecalc-explore-shell__panel-header,
.onecalc-workbench-shell__panel-header,
.onecalc-inspect-shell__panel-header {
  align-items: flex-start;
  justify-content: space-between;
  padding-bottom: var(--oc-space-3);
  margin-bottom: var(--oc-space-3);
  border-bottom: 1px solid rgba(31, 28, 23, 0.08);
}

.onecalc-explore-shell__editor-panel,
.onecalc-explore-shell__result-panel,
.onecalc-explore-shell__help-panel {
  overflow: hidden;
  background:
    linear-gradient(180deg, rgba(255, 253, 248, 0.98), rgba(248, 239, 227, 0.96)),
    var(--oc-color-surface);
  border: 1px solid rgba(31, 28, 23, 0.1);
  box-shadow: 0 22px 48px rgba(31, 28, 23, 0.08);
}

.onecalc-explore-shell__editor-panel {
  position: relative;
}

.onecalc-explore-shell__editor-panel::before,
.onecalc-explore-shell__result-panel::before,
.onecalc-explore-shell__help-panel::before,
.onecalc-workbench-shell__outcome-card::before,
.onecalc-workbench-shell__evidence-card::before,
.onecalc-workbench-shell__compare-card::before,
.onecalc-workbench-shell__replay-card::before,
.onecalc-workbench-shell__lineage-card::before,
.onecalc-workbench-shell__actions-card::before,
.onecalc-workbench-shell__catalog-card::before,
.onecalc-inspect-shell__source-stack::before,
.onecalc-inspect-shell__walk-cluster::before,
.onecalc-inspect-shell__summary-cluster::before {
  content: "";
  display: block;
  height: 0.24rem;
  margin: calc(-1 * var(--oc-space-4)) calc(-1 * var(--oc-space-4)) var(--oc-space-4);
  background: linear-gradient(90deg, var(--oc-color-accent), rgba(200, 141, 46, 0.7), rgba(184, 69, 50, 0.6));
}

.onecalc-explore-shell__context-strip {
  padding: var(--oc-space-2) var(--oc-space-3);
  border-radius: 14px;
  background: linear-gradient(180deg, rgba(36, 93, 90, 0.07), rgba(255, 255, 255, 0.7));
  border: 1px solid rgba(36, 93, 90, 0.14);
}

.onecalc-explore-shell__status-card,
.onecalc-explore-shell__assist-metric,
.onecalc-inspect-shell__source-card,
.onecalc-workbench-shell__score-card,
.onecalc-workbench-shell__observation-card {
  background: linear-gradient(180deg, rgba(255, 253, 248, 0.98), rgba(248, 239, 227, 0.96));
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.8);
}

.onecalc-explore-shell__assist-intro {
  margin-bottom: var(--oc-space-3);
  padding: var(--oc-space-3);
  border-radius: 14px;
  border: 1px solid rgba(31, 28, 23, 0.08);
  background: rgba(255, 251, 245, 0.88);
  color: var(--oc-color-muted);
  line-height: 1.5;
}

.onecalc-explore-shell__overview-deck {
  display: grid;
  grid-template-columns: 1.15fr 0.85fr 0.95fr;
  gap: var(--oc-space-3);
}

.onecalc-explore-shell__overview-card {
  display: grid;
  gap: var(--oc-space-2);
  padding: var(--oc-space-4);
  border-radius: 18px;
  border: 1px solid rgba(31, 28, 23, 0.1);
  background: linear-gradient(180deg, rgba(255, 253, 248, 0.98), rgba(244, 236, 223, 0.96));
  box-shadow: 0 16px 34px rgba(31, 28, 23, 0.06);
}

.onecalc-explore-shell__overview-card strong {
  font-size: 1.08rem;
  color: var(--oc-color-night);
}

.onecalc-explore-shell__overview-card p {
  margin: 0;
  color: var(--oc-color-muted);
  line-height: 1.55;
}

.onecalc-explore-shell__overview-card[data-role="explore-overview-primary"] {
  background: linear-gradient(155deg, rgba(36, 93, 90, 0.11), rgba(255, 252, 246, 0.96));
}

.onecalc-explore-shell__overview-card[data-role="explore-overview-result"] strong {
  font-family: var(--oc-font-mono);
  font-size: 1.45rem;
}

.onecalc-explore-shell__body-column--result .onecalc-explore-shell__result-panel,
.onecalc-explore-shell__body-column--help .onecalc-explore-shell__help-panel {
  position: sticky;
  top: 0;
}

.onecalc-explore-shell__hero-result {
  grid-template-columns: minmax(0, 1.4fr) minmax(14rem, 0.9fr);
  align-items: stretch;
  padding: var(--oc-space-5);
}

.onecalc-explore-shell__hero-result-copy,
.onecalc-explore-shell__hero-result-sidecar {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-explore-shell__hero-result-sidecar {
  align-content: start;
}

.onecalc-explore-shell__hero-result-value {
  font-size: clamp(1.8rem, 3vw, 2.6rem);
}

.onecalc-explore-shell__hero-result-caption {
  margin: 0;
  color: var(--oc-color-muted);
  line-height: 1.45;
}

.onecalc-explore-shell__hero-pill {
  display: grid;
  gap: 0.2rem;
  padding: var(--oc-space-3);
  border-radius: 14px;
  border: 1px solid rgba(31, 28, 23, 0.1);
  background: rgba(255, 255, 255, 0.78);
}

.onecalc-explore-shell__hero-pill > span {
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-size: 0.72rem;
  color: var(--oc-color-muted);
  font-weight: 700;
}

.onecalc-explore-shell__result-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--oc-space-2);
  margin-bottom: var(--oc-space-3);
}

.onecalc-explore-shell__assist-callout {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--oc-space-3);
  margin-bottom: var(--oc-space-3);
  padding: var(--oc-space-3);
  border-radius: 16px;
  border: 1px solid rgba(36, 93, 90, 0.15);
  background: linear-gradient(180deg, rgba(36, 93, 90, 0.08), rgba(255, 255, 255, 0.78));
}

.onecalc-explore-shell__assist-callout > div {
  display: grid;
  gap: var(--oc-space-1);
}

.onecalc-explore-shell__assist-callout-state {
  padding: 0.35rem 0.8rem;
  border-radius: var(--oc-radius-pill);
  background: rgba(36, 93, 90, 0.14);
  color: var(--oc-color-accent);
  font-size: 0.82rem;
  font-weight: 700;
}

.onecalc-explore-shell__selected-proposal,
.onecalc-explore-shell__function-help {
  box-shadow: 0 14px 28px rgba(31, 28, 23, 0.06);
}

.onecalc-formula-editor-surface {
  display: grid;
  position: relative;
  gap: 0;
  border-radius: 18px;
  overflow: hidden;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: linear-gradient(180deg, #fffdf9, #f8f1e4);
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.85);
}

.onecalc-formula-editor-surface__toolbar {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  align-items: center;
  gap: var(--oc-space-3);
  padding: 1rem 1.25rem;
  border-bottom: 1px solid rgba(31, 28, 23, 0.1);
  background: linear-gradient(180deg, #f7f3ea, #eee4d6);
}

.onecalc-formula-editor-surface__toolbar-copy {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--oc-space-4);
  min-width: 0;
  flex: 1;
}

.onecalc-formula-editor-surface__toolbar-meta {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--oc-space-2);
  min-width: 0;
}

.onecalc-formula-editor-surface__toolbar-title {
  font-size: 0.95rem;
  font-weight: 700;
  color: #1f1c17;
}

.onecalc-formula-editor-surface__toolbar-subtitle {
  color: var(--oc-color-muted);
  font-size: 0.82rem;
  margin-top: 0.1rem;
}

.onecalc-formula-editor-surface__toolbar-metrics {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
  color: var(--oc-color-muted);
  font-size: 0.8rem;
}

.onecalc-formula-editor-surface__toolbar-state {
  white-space: nowrap;
  padding: 0.45rem 0.9rem;
  border-radius: 12px;
  background: var(--oc-color-accent);
  color: #fff;
  font-size: 0.82rem;
  font-weight: 700;
  box-shadow: 0 8px 18px rgba(36, 93, 90, 0.22);
}

.onecalc-formula-editor-surface__toolbar-pills {
  display: flex;
  align-items: center;
  gap: var(--oc-space-2);
  flex-wrap: nowrap;
  min-width: 0;
}

.onecalc-formula-editor-surface__toolbar-actions {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--oc-space-2);
  flex: 0 0 auto;
}

.onecalc-formula-editor-surface__entry-mode-pill,
.onecalc-formula-editor-surface__result-class-pill,
.onecalc-formula-editor-surface__live-state-pill {
  min-width: 0;
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.3rem 0.7rem;
  white-space: nowrap;
  border-radius: 999px;
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.02em;
  text-transform: uppercase;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 252, 246, 0.95);
  color: var(--oc-color-ink);
}

.onecalc-formula-editor-surface__entry-mode-pill[data-entry-mode="formula"] {
  border-color: rgba(36, 93, 90, 0.35);
  background: rgba(36, 93, 90, 0.1);
  color: var(--oc-color-accent);
}

.onecalc-formula-editor-surface__entry-mode-pill[data-entry-mode="value"] {
  border-color: rgba(200, 141, 46, 0.4);
  background: rgba(200, 141, 46, 0.12);
  color: #8d5f1e;
}

.onecalc-formula-editor-surface__entry-mode-pill[data-entry-mode="text"] {
  border-color: rgba(62, 82, 56, 0.4);
  background: rgba(62, 82, 56, 0.12);
  color: #3e5238;
}

.onecalc-formula-editor-surface__entry-mode-pill[data-entry-mode="empty"] {
  color: var(--oc-color-muted);
  font-style: italic;
}

.onecalc-formula-editor-surface__result-class-pill[data-has-result="false"] {
  color: var(--oc-color-muted);
  font-style: italic;
}

.onecalc-formula-editor-surface__live-state-pill[data-live-state="idle"] {
  color: var(--oc-color-muted);
}

.onecalc-formula-editor-surface__live-state-pill[data-live-state="editing-live"] {
  border-color: rgba(200, 141, 46, 0.45);
  background: rgba(200, 141, 46, 0.14);
  color: #8d5f1e;
}

.onecalc-formula-editor-surface__live-state-pill[data-live-state="proofed-scratch"] {
  border-color: rgba(36, 93, 90, 0.45);
  background: rgba(36, 93, 90, 0.12);
  color: var(--oc-color-accent);
}

.onecalc-formula-editor-surface__live-state-pill[data-live-state="committed"] {
  border-color: rgba(79, 123, 87, 0.5);
  background: rgba(79, 123, 87, 0.16);
  color: #33532c;
}

.onecalc-formula-editor-surface[data-expanded-editor="true"] .onecalc-formula-editor-surface__textarea {
  min-height: 32rem;
}

.onecalc-formula-editor-surface[data-fallback-mode="true"] .onecalc-formula-editor-surface__overlay-layer {
  display: none;
}

.onecalc-formula-editor-surface[data-fallback-mode="true"] .onecalc-formula-editor-surface__bracket-pair-layer,
.onecalc-formula-editor-surface[data-fallback-mode="true"] .onecalc-formula-editor-surface__popup-container,
.onecalc-formula-editor-surface[data-fallback-mode="true"] .onecalc-formula-editor-surface__toolbar-pills {
  display: none;
}

.onecalc-formula-editor-surface[data-fallback-mode="true"] .onecalc-formula-editor-surface__toolbar::after {
  content: "Plain mode";
  margin-left: var(--oc-space-2);
  padding: 0.3rem 0.7rem;
  border-radius: 999px;
  background: rgba(31, 28, 23, 0.06);
  color: var(--oc-color-muted);
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.onecalc-formula-editor-surface[data-fallback-mode="true"] .onecalc-formula-editor-surface__textarea {
  font: 0.95rem/1.6rem var(--oc-font-mono);
  padding: 0.85rem 1rem;
  background: #fff;
}

.onecalc-formula-editor-surface__inline-diagnostic {
  position: relative;
  color: var(--oc-color-warm);
  text-decoration-line: underline;
  text-decoration-style: wavy;
  text-decoration-color: var(--oc-color-warm);
  text-underline-offset: 3px;
}

.onecalc-formula-editor-surface__bracket-pair-layer {
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.onecalc-formula-editor-surface__bracket-pair {
  box-sizing: border-box;
  border: 1px solid rgba(36, 93, 90, 0.6);
  background: rgba(36, 93, 90, 0.1);
  border-radius: 3px;
}

.onecalc-formula-editor-surface__settings-gear {
  width: 2rem;
  height: 2rem;
  border-radius: 999px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 252, 246, 0.95);
  color: var(--oc-color-ink);
  font-size: 1rem;
  cursor: pointer;
}

.onecalc-formula-editor-surface__settings-gear[data-open="true"] {
  border-color: var(--oc-color-accent);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
}

.onecalc-formula-editor-surface__settings-popover {
  position: absolute;
  top: calc(100% + 0.65rem);
  right: 0;
  width: min(36rem, calc(100vw - 4rem));
  max-width: min(36rem, calc(100vw - 4rem));
  border: 1px solid rgba(31, 28, 23, 0.12);
  border-radius: 16px;
  background: linear-gradient(180deg, #fffdf9, #f8f1e4);
  box-shadow: 0 18px 34px rgba(31, 28, 23, 0.12);
  padding: 0.9rem 1rem;
  z-index: 5;
}

.onecalc-formula-editor-surface__settings-popover-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
  gap: var(--oc-space-2);
}

.onecalc-formula-editor-surface__settings-toggle {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--oc-space-2);
  padding: 0.55rem 0.8rem;
  border-radius: 10px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 255, 255, 0.9);
  color: var(--oc-color-ink);
  font-size: 0.82rem;
  cursor: pointer;
}

.onecalc-formula-editor-surface__settings-toggle[data-checked="true"] {
  border-color: var(--oc-color-accent);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  font-weight: 700;
}

.onecalc-formula-editor-surface__settings-choice {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  padding: 0.55rem 0.8rem;
  border-radius: 10px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 255, 255, 0.9);
  font-size: 0.82rem;
  color: var(--oc-color-muted);
}

.onecalc-formula-editor-surface__settings-choice-buttons {
  display: flex;
  gap: 0.35rem;
  flex-wrap: wrap;
}

.onecalc-formula-editor-surface__settings-choice-button {
  padding: 0.3rem 0.55rem;
  border-radius: 999px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 255, 255, 0.85);
  color: var(--oc-color-ink);
  font-size: 0.72rem;
  font-weight: 700;
  cursor: pointer;
}

.onecalc-formula-editor-surface__settings-choice-button[data-active="true"] {
  border-color: var(--oc-color-accent);
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
}

.onecalc-formula-editor-surface__body {
  display: grid;
  grid-template-columns: 4.2rem minmax(0, 1fr);
  min-height: 20rem;
  background: #fff;
}

.onecalc-formula-editor-surface__line-rail {
  display: grid;
  align-content: start;
  gap: 0;
  padding: 1rem 0;
  background: linear-gradient(180deg, #faf7f1, #f0e8dc);
  border-right: 1px solid rgba(31, 28, 23, 0.08);
}

.onecalc-formula-editor-surface__line-number {
  height: 1.9rem;
  line-height: 1.9rem;
  padding: 0 0.85rem;
  text-align: right;
  font-family: var(--oc-font-mono);
  font-size: 0.82rem;
  color: var(--oc-color-muted);
}

.onecalc-formula-editor-surface__line-number--active {
  color: var(--oc-color-accent);
  font-weight: 700;
}

.onecalc-formula-editor-surface__editor-stage {
  position: relative;
  min-width: 0;
}

.onecalc-formula-editor-surface__native-input-layer,
.onecalc-formula-editor-surface__overlay-layer {
  inset: 0;
  padding: 1rem 1.1rem;
}

.onecalc-formula-editor-surface__textarea {
  width: 100%;
  min-height: 20rem;
  padding: 0;
  border: none;
  outline: none;
  resize: vertical;
  background: transparent;
  color: #1f1c17;
  font: 0.95rem/1.9rem var(--oc-font-mono);
}

.onecalc-formula-editor-surface__overlay-layer {
  pointer-events: none;
}

.onecalc-formula-editor-surface__completion-popup,
.onecalc-formula-editor-surface__signature-help-popup {
  min-width: 15rem;
  border-radius: 14px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 252, 246, 0.97);
  box-shadow: 0 18px 36px rgba(31, 28, 23, 0.14);
  backdrop-filter: blur(10px);
}

.onecalc-formula-editor-surface__completion-item {
  width: 100%;
  text-align: left;
  padding: 0.7rem 0.85rem;
  border: none;
  background: transparent;
  color: var(--oc-color-ink);
  font: inherit;
}

.onecalc-formula-editor-surface__completion-item[data-active-row="true"] {
  background: linear-gradient(90deg, rgba(36, 93, 90, 0.12), rgba(255, 255, 255, 0.65));
  color: var(--oc-color-accent);
  font-weight: 700;
}

.onecalc-formula-editor-surface__diagnostic-band {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--oc-space-3);
  padding: 0.9rem 1.25rem;
  border-top: 1px solid rgba(31, 28, 23, 0.08);
  background: linear-gradient(180deg, rgba(36, 93, 90, 0.06), rgba(255, 255, 255, 0.84));
}

.onecalc-formula-editor-surface__diagnostic-band-state {
  display: flex;
  align-items: flex-start;
  gap: var(--oc-space-3);
}

.onecalc-formula-editor-surface__diagnostic-band-state > div {
  display: grid;
  gap: 0.2rem;
  color: var(--oc-color-muted);
  font-size: 0.84rem;
}

.onecalc-formula-editor-surface__diagnostic-band-state strong {
  color: var(--oc-color-ink);
}

.onecalc-formula-editor-surface__diagnostic-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1.75rem;
  height: 1.75rem;
  border-radius: 999px;
  background: var(--oc-color-accent);
  color: white;
  font-size: 0.72rem;
  font-weight: 700;
}

.onecalc-formula-editor-surface__diagnostic-band-action {
  padding: 0.35rem 0.75rem;
  border-radius: var(--oc-radius-pill);
  border: 1px solid rgba(31, 28, 23, 0.1);
  background: rgba(255, 255, 255, 0.75);
  color: var(--oc-color-ink);
  font-size: 0.82rem;
  font-weight: 700;
}

.onecalc-formula-editor-surface__footer {
  border-top: 1px solid rgba(31, 28, 23, 0.08);
  background: #fcf8f1;
}

.onecalc-workbench-shell__hero-outcome {
  padding: var(--oc-space-5);
  border: 1px solid rgba(184, 69, 50, 0.18);
  background: linear-gradient(145deg, rgba(184, 69, 50, 0.06), rgba(255, 255, 255, 0.94));
}

.onecalc-workbench-shell__score-grid {
  grid-template-columns: repeat(3, minmax(0, 1fr));
}

.onecalc-workbench-shell__score-card {
  border: 1px solid rgba(31, 28, 23, 0.1);
  box-shadow: 0 12px 26px rgba(31, 28, 23, 0.06);
}

.onecalc-workbench-shell__observation-envelope {
  display: grid;
  gap: var(--oc-space-3);
}

.onecalc-workbench-shell__observation-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(13rem, 1fr));
  gap: var(--oc-space-2);
}

.onecalc-workbench-shell__observation-card {
  padding: var(--oc-space-3);
  border-radius: 14px;
  border: 1px solid rgba(31, 28, 23, 0.1);
  color: var(--oc-color-muted);
}

.onecalc-workbench-shell__timeline {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-workbench-shell__timeline-item {
  display: grid;
  grid-template-columns: 1.25rem minmax(0, 1fr);
  gap: var(--oc-space-3);
  align-items: start;
}

.onecalc-workbench-shell__timeline-dot {
  width: 0.9rem;
  height: 0.9rem;
  margin-top: 0.55rem;
  border-radius: 999px;
  background: linear-gradient(180deg, var(--oc-color-accent), var(--oc-color-night));
  box-shadow: 0 0 0 0.22rem rgba(36, 93, 90, 0.12);
}

.onecalc-workbench-shell__timeline-card,
.onecalc-workbench-shell__action-item {
  padding: var(--oc-space-3);
  border-radius: 14px;
  border: 1px solid rgba(31, 28, 23, 0.1);
  background: linear-gradient(180deg, #fffef9, #f5ede2);
}

.onecalc-workbench-shell__action-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-inspect-shell__walk-node {
  position: relative;
  overflow: hidden;
}

.onecalc-inspect-shell__walk-node::before {
  content: "";
  position: absolute;
  inset: 0 auto 0 0;
  width: 0.3rem;
  background: linear-gradient(180deg, rgba(36, 93, 90, 0.7), rgba(184, 69, 50, 0.55));
}

.onecalc-inspect-shell__summary-card,
.onecalc-inspect-shell__comparison-record,
.onecalc-inspect-shell__explain-record {
  box-shadow: 0 14px 28px rgba(31, 28, 23, 0.06);
}

@media (max-width: 980px) {
  .onecalc-shell-frame {
    grid-template-columns: 1fr;
  }

  .onecalc-shell-frame__rail {
    border-right: none;
    border-bottom: 1px solid var(--oc-color-border);
  }

  .onecalc-shell-frame__context-bar {
    grid-template-columns: 1fr;
  }

  .onecalc-shell-frame__context-facts {
    grid-template-columns: 1fr;
  }

  .onecalc-shell-frame__workspace-manifest-grid {
    grid-template-columns: 1fr;
  }

  .onecalc-shell-frame__content-main {
    grid-template-columns: 1fr;
  }

  .onecalc-capability-center__summary {
    grid-template-columns: 1fr;
  }

  .onecalc-capability-center__diff-header {
    align-items: stretch;
    flex-direction: column;
  }

  .onecalc-capability-center__diff-target {
    min-width: 0;
    width: 100%;
  }

  .onecalc-explore-shell__body,
  .onecalc-inspect-shell__body,
  .onecalc-workbench-shell__body {
    grid-template-columns: 1fr;
  }

  .onecalc-explore-shell__hero-result,
  .onecalc-explore-shell__overview-deck,
  .onecalc-inspect-shell__overview-deck,
  .onecalc-workbench-shell__overview-deck,
  .onecalc-workbench-shell__score-grid,
  .onecalc-explore-shell__result-grid {
    grid-template-columns: 1fr;
  }

  .onecalc-explore-shell__header-copy,
  .onecalc-workbench-shell__header-copy,
  .onecalc-inspect-shell__header-copy,
  .onecalc-formula-editor-surface__toolbar,
  .onecalc-formula-editor-surface__toolbar-copy,
  .onecalc-formula-editor-surface__diagnostic-band {
    grid-template-columns: 1fr;
    display: grid;
  }

  .onecalc-formula-editor-surface__toolbar-meta {
    justify-content: space-between;
    flex-wrap: wrap;
  }

  .onecalc-formula-editor-surface__toolbar-pills {
    flex-wrap: wrap;
  }

  .onecalc-formula-editor-surface__settings-popover {
    position: static;
    width: auto;
    max-width: none;
    margin-top: var(--oc-space-2);
  }

  .onecalc-formula-editor-surface__body {
    grid-template-columns: 3.2rem minmax(0, 1fr);
  }

  .onecalc-inspect-shell__column--left .onecalc-inspect-shell__source-stack,
  .onecalc-inspect-shell__column--summary .onecalc-inspect-shell__summary-cluster,
  .onecalc-workbench-shell__column--support .onecalc-workbench-shell__replay-card,
  .onecalc-workbench-shell__column--actions .onecalc-workbench-shell__actions-card {
    position: static;
  }
}

.onecalc-explore-shell__configure-button {
  padding: 0.45rem 0.9rem;
  border-radius: var(--oc-radius-pill);
  border: 1px solid var(--oc-shell-mode-accent, var(--oc-color-accent));
  background: rgba(255, 252, 246, 0.9);
  color: var(--oc-shell-mode-accent, var(--oc-color-accent));
  font-size: 0.78rem;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  cursor: pointer;
  margin-left: var(--oc-space-2);
}

.onecalc-explore-shell__configure-button[data-open="true"] {
  background: var(--oc-shell-mode-accent, var(--oc-color-accent));
  color: var(--oc-shell-mode-accent-ink, #fff);
}

.onecalc-explore-shell__body-column[data-drawer-open="true"] {
  padding: 0;
}

.onecalc-explore-shell__configure-placeholder {
  display: grid;
  gap: var(--oc-space-2);
  color: var(--oc-color-ink);
  font-size: 0.85rem;
  line-height: 1.55;
}

.onecalc-explore-shell__configure-placeholder p {
  margin: 0;
  color: var(--oc-color-muted);
}

.onecalc-explore-shell__configure-placeholder ul {
  margin: 0;
  padding-left: 1.1rem;
}

.onecalc-shell-drawer {
  display: grid;
  grid-template-rows: auto 1fr;
  min-width: 20rem;
  max-width: 32rem;
  background: linear-gradient(180deg, #fffdf9, #f5efe1);
  border-left: 4px solid var(--oc-shell-mode-accent, var(--oc-color-accent));
  border-top: 1px solid rgba(31, 28, 23, 0.1);
  border-bottom: 1px solid rgba(31, 28, 23, 0.1);
  box-shadow: -18px 0 36px rgba(31, 28, 23, 0.14);
  color: var(--oc-color-ink);
  animation: onecalc-shell-drawer-slide-in 180ms ease-out both;
}

@keyframes onecalc-shell-drawer-slide-in {
  from {
    transform: translateX(12px);
    opacity: 0;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

.onecalc-shell-drawer__header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--oc-space-2);
  padding: 1rem 1.25rem 0.85rem;
  border-bottom: 1px solid rgba(31, 28, 23, 0.08);
  background: rgba(255, 252, 246, 0.75);
}

.onecalc-shell-drawer__titles {
  display: grid;
  gap: 0.2rem;
  min-width: 0;
}

.onecalc-shell-drawer__eyebrow {
  font-size: 0.66rem;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: var(--oc-shell-mode-accent, var(--oc-color-accent));
  font-weight: 700;
}

.onecalc-shell-drawer__title {
  font-size: 1.05rem;
  font-weight: 700;
  color: var(--oc-color-ink);
  line-height: 1.25;
}

.onecalc-shell-drawer__subtitle {
  font-size: 0.82rem;
  color: var(--oc-color-muted);
}

.onecalc-shell-drawer__close {
  width: 2rem;
  height: 2rem;
  border-radius: 999px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 252, 246, 0.95);
  color: var(--oc-color-muted);
  font-size: 1.2rem;
  line-height: 1;
  cursor: pointer;
  flex-shrink: 0;
}

.onecalc-shell-drawer__close:hover {
  color: var(--oc-color-warm);
  border-color: var(--oc-color-warm);
}

.onecalc-shell-drawer__body {
  padding: 1rem 1.25rem 1.25rem;
  overflow-y: auto;
  min-height: 0;
}

.onecalc-shell-frame__content-main .onecalc-shell-drawer {
  min-height: 0;
  height: 100%;
}

.onecalc-capability-center {
  display: grid;
  gap: var(--oc-space-4);
}

.onecalc-capability-center__summary {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--oc-space-2);
}

.onecalc-capability-center__summary-card {
  display: grid;
  gap: 0.2rem;
  padding: 0.7rem 0.8rem;
  border-radius: 12px;
  border: 1px solid rgba(31, 28, 23, 0.08);
  background: rgba(255, 255, 255, 0.78);
}

.onecalc-capability-center__summary-card[data-tone="accent"] {
  border-color: rgba(36, 93, 90, 0.24);
  background: rgba(36, 93, 90, 0.1);
}

.onecalc-capability-center__summary-label,
.onecalc-capability-center__diff-row-label {
  color: var(--oc-color-muted);
  text-transform: uppercase;
  letter-spacing: 0.08em;
  font-size: 0.66rem;
  font-weight: 700;
}

.onecalc-capability-center__summary-value {
  color: var(--oc-color-night);
  font-size: 0.92rem;
}

.onecalc-capability-center__actions {
  display: flex;
  gap: var(--oc-space-2);
  flex-wrap: wrap;
}

.onecalc-capability-center__action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.45rem 0.75rem;
  border-radius: 999px;
  border: 1px solid rgba(31, 28, 23, 0.14);
  background: rgba(255, 252, 246, 0.95);
  color: var(--oc-color-accent);
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  text-decoration: none;
  cursor: pointer;
}

.onecalc-capability-center__action--link {
  color: var(--oc-color-warm);
}

.onecalc-capability-center__diff {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-capability-center__diff-header {
  display: flex;
  align-items: end;
  justify-content: space-between;
  gap: var(--oc-space-2);
}

.onecalc-capability-center__diff-target {
  min-width: 14rem;
  padding: 0.45rem 0.65rem;
  border-radius: 10px;
  border: 1px solid rgba(31, 28, 23, 0.14);
  background: rgba(255, 255, 255, 0.92);
  color: var(--oc-color-ink);
}

.onecalc-capability-center__diff-rows {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-capability-center__diff-row {
  display: grid;
  gap: 0.35rem;
  padding: 0.7rem 0.8rem;
  border-radius: 12px;
  border: 1px solid rgba(31, 28, 23, 0.08);
  background: rgba(255, 255, 255, 0.76);
}

.onecalc-capability-center__diff-row[data-status="changed"] {
  border-color: rgba(184, 69, 50, 0.24);
  background: rgba(184, 69, 50, 0.06);
}

.onecalc-capability-center__diff-row-values {
  display: grid;
  gap: 0.2rem;
  font-family: var(--oc-font-mono);
  font-size: 0.78rem;
  color: var(--oc-color-muted);
}

.onecalc-capability-center__payload {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-capability-center__payload-text {
  width: 100%;
  min-height: 16rem;
  padding: 0.8rem 0.9rem;
  border-radius: 12px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 255, 255, 0.92);
  color: var(--oc-color-ink);
  font: 0.78rem/1.5rem var(--oc-font-mono);
  resize: vertical;
}

.onecalc-value-panel {
  display: grid;
  gap: var(--oc-space-2);
  padding: 1rem 1.25rem;
  border-radius: 16px;
  border: 2px solid rgba(36, 93, 90, 0.14);
  background: linear-gradient(180deg, #fffdf9, #f7f3ea);
  color: var(--oc-color-ink);
}

.onecalc-explore-shell__result-panel {
  display: grid;
  gap: var(--oc-space-3);
}

.onecalc-explore-shell__result-panel > .onecalc-value-panel {
  padding: 0;
  border: none;
  border-radius: 0;
  background: transparent;
  box-shadow: none;
}

.onecalc-value-panel__header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--oc-space-2);
}

.onecalc-value-panel__header-label {
  font-size: 1rem;
  font-weight: 700;
  color: var(--oc-color-ink);
}

.onecalc-value-panel__header-kind-tag {
  font-family: var(--oc-font-mono);
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--oc-color-muted);
}

.onecalc-value-panel__primary {
  display: grid;
  gap: var(--oc-space-2);
}

.onecalc-value-panel__number-main,
.onecalc-value-panel__logical,
.onecalc-value-panel__error-code {
  font-family: var(--oc-font-mono);
  font-size: 2rem;
  font-weight: 700;
  color: var(--oc-color-accent);
}

.onecalc-value-panel__error-code {
  color: var(--oc-color-warm);
}

.onecalc-value-panel__text-content {
  font-family: var(--oc-font-mono);
  font-size: 1.1rem;
  padding: 0.6rem 0.8rem;
  background: rgba(255, 255, 255, 0.8);
  border-radius: 8px;
  border: 1px solid rgba(31, 28, 23, 0.08);
  white-space: pre-wrap;
  word-break: break-word;
}

.onecalc-value-panel__text-meta,
.onecalc-value-panel__reference code,
.onecalc-value-panel__lambda dd code {
  font-family: var(--oc-font-mono);
  font-size: 0.8rem;
  color: var(--oc-color-muted);
}

.onecalc-value-panel__text-warning,
.onecalc-value-panel__reference-warning {
  color: var(--oc-color-warm);
  font-weight: 700;
}

.onecalc-value-panel__error-surface {
  display: inline-block;
  padding: 0.2rem 0.5rem;
  border-radius: 999px;
  background: rgba(200, 141, 46, 0.16);
  color: #8d5f1e;
  font-size: 0.72rem;
  font-weight: 700;
}

.onecalc-value-panel__array-header {
  font-family: var(--oc-font-mono);
  font-size: 0.85rem;
  color: var(--oc-color-muted);
}

.onecalc-value-panel__array-grid {
  border-collapse: collapse;
  font-family: var(--oc-font-mono);
  font-size: 0.85rem;
}

.onecalc-value-panel__array-cell {
  padding: 0.25rem 0.5rem;
  border: 1px solid rgba(31, 28, 23, 0.08);
  background: rgba(255, 255, 255, 0.8);
}

.onecalc-value-panel__kv-list,
.onecalc-value-panel__lambda {
  display: grid;
  gap: 0.25rem;
  margin: 0;
}

.onecalc-value-panel__kv-row {
  display: grid;
  grid-template-columns: 8rem 1fr;
  gap: var(--oc-space-2);
  font-size: 0.82rem;
}

.onecalc-value-panel__kv-row dt {
  color: var(--oc-color-muted);
  font-weight: 700;
  text-transform: uppercase;
  font-size: 0.7rem;
  letter-spacing: 0.04em;
}

.onecalc-value-panel__kv-row dd {
  margin: 0;
  color: var(--oc-color-ink);
  font-family: var(--oc-font-mono);
}

.onecalc-value-panel__rich-kvp-flag {
  margin-left: 0.35rem;
  padding: 0.1rem 0.35rem;
  border-radius: 999px;
  background: rgba(183, 101, 69, 0.12);
  color: #a75842;
  font-family: var(--oc-font-ui);
  font-size: 0.65rem;
  font-weight: 700;
  text-transform: uppercase;
}

.onecalc-value-panel__presentation {
  display: flex;
  gap: 0.35rem;
  align-items: center;
  flex-wrap: wrap;
}

.onecalc-value-panel__presentation-badge {
  padding: 0.25rem 0.55rem;
  border-radius: 999px;
  background: rgba(36, 93, 90, 0.1);
  color: var(--oc-color-accent);
  font-size: 0.7rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.onecalc-value-panel__presentation-format-code {
  font-family: var(--oc-font-mono);
  font-size: 0.78rem;
  color: var(--oc-color-muted);
}

.onecalc-value-panel__effective-display {
  display: grid;
  gap: 0.35rem;
  padding: 0.6rem 0.85rem;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.7);
  border: 1px solid rgba(31, 28, 23, 0.08);
}

.onecalc-value-panel__effective-display-line {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
}

.onecalc-value-panel__effective-display-label {
  color: var(--oc-color-muted);
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.onecalc-value-panel__effective-display-value {
  font-family: var(--oc-font-mono);
  font-size: 1rem;
}

.onecalc-value-panel__effective-display-value[data-has-display="false"] {
  color: var(--oc-color-muted);
  font-style: italic;
}

.onecalc-value-panel__effective-display-pipeline {
  display: flex;
  gap: 0.35rem;
  flex-wrap: wrap;
  min-width: 0;
}

.onecalc-value-panel__pipeline-chip {
  max-width: 100%;
  padding: 0.15rem 0.5rem;
  border-radius: 999px;
  border: 1px solid rgba(31, 28, 23, 0.12);
  background: rgba(255, 255, 255, 0.85);
  color: var(--oc-color-muted);
  font-size: 0.68rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  white-space: normal;
  overflow-wrap: anywhere;
  text-align: center;
}

.onecalc-value-panel__pipeline-chip[data-status="live"] {
  border-color: rgba(79, 123, 87, 0.4);
  background: rgba(79, 123, 87, 0.14);
  color: #33532c;
}

.onecalc-value-panel__pipeline-chip[data-status="not-implemented"] {
  border-color: rgba(183, 101, 69, 0.35);
  background: rgba(183, 101, 69, 0.1);
  color: #a75842;
  text-decoration: line-through;
}

.onecalc-value-panel__provenance {
  display: flex;
  gap: var(--oc-space-2);
  font-family: var(--oc-font-mono);
  font-size: 0.7rem;
  color: var(--oc-color-muted);
}

.onecalc-value-panel__unevaluated {
  padding: 0.6rem 0.85rem;
  border-radius: 10px;
  border: 1px dashed rgba(31, 28, 23, 0.18);
  background: rgba(255, 255, 255, 0.6);
  color: var(--oc-color-muted);
  font-style: italic;
}

/* ============================================================================
   WS-14 Pre-MVP home shell — minimal editor + result + status foot.
   No drill-downs, no breadcrumb, no compare in this slice.
   ============================================================================ */

.onecalc-home-shell {
  display: grid;
  /* Four rows: titlebar (auto), tab strip (auto), body (1fr —
     the 1fr lives on the body so the tab strip stays its
     natural height even when the strip's `align-items: stretch`
     would otherwise inflate it), status foot (auto). When the
     tab strip is hidden the renderer emits an empty fragment
     and the grid collapses that row to 0px automatically. */
  grid-template-rows: auto auto 1fr auto;
  min-height: 100vh;
  font-family: var(--oc-font-ui);
  color: var(--oc-color-ink);
  background: var(--oc-color-bg);
}

.onecalc-home-shell__titlebar {
  display: flex;
  align-items: center;
  gap: var(--oc-space-3);
  height: 36px;
  padding: 0 var(--oc-space-5);
  background: var(--oc-color-night);
  color: #e6efef;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}

.onecalc-home-shell__brand {
  font-weight: 600;
  letter-spacing: 0.02em;
}

.onecalc-home-shell__titlebar-hint {
  margin-left: auto;
  color: rgba(230, 239, 239, 0.6);
  font-family: var(--oc-font-mono);
  font-size: 12px;
}

.onecalc-home-shell__titlebar-action {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px;
  border: 1px solid rgba(255, 255, 255, 0.18);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.06);
  color: #e6efef;
  font-family: var(--oc-font-ui);
  font-size: 0.78rem;
  cursor: pointer;
}

.onecalc-home-shell__titlebar-action:hover {
  background: rgba(255, 255, 255, 0.12);
  border-color: rgba(255, 255, 255, 0.3);
}

.onecalc-home-shell__titlebar-action:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: 1px;
}

.onecalc-home-shell__titlebar-action-glyph {
  font-size: 0.95rem;
  line-height: 1;
}

/* Command palette — Ctrl+K overlay surface. Modal centered on
   the viewport with a dimmed backdrop; closing on Esc / outside-
   click is wired in the renderer. The list is keyboard-driven
   (ArrowUp/Down + Enter), the input is autofocused on open. */
.onecalc-home-shell__palette-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 12vh;
  z-index: 60;
}

.onecalc-home-shell__palette {
  width: min(640px, 96vw);
  background: var(--oc-color-surface);
  border-radius: 12px;
  border: 1px solid var(--oc-color-border);
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.25);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.onecalc-home-shell__palette-input {
  border: none;
  border-bottom: 1px solid var(--oc-color-border);
  padding: 12px 16px;
  font-family: var(--oc-font-ui);
  font-size: 1rem;
  background: transparent;
  outline: none;
}

.onecalc-home-shell__palette-list {
  max-height: 60vh;
  overflow-y: auto;
}

.onecalc-home-shell__palette-row {
  display: grid;
  grid-template-columns: 84px 1fr auto auto;
  align-items: center;
  gap: 12px;
  padding: 8px 16px;
  width: 100%;
  border: none;
  background: transparent;
  font: inherit;
  text-align: left;
  cursor: pointer;
  color: inherit;
}

.onecalc-home-shell__palette-row:hover,
.onecalc-home-shell__palette-row[data-is-selected="true"] {
  background: var(--oc-color-accent-soft);
}

.onecalc-home-shell__palette-row-section {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--oc-color-muted);
}

.onecalc-home-shell__palette-row-label {
  font-weight: 500;
}

.onecalc-home-shell__palette-row-detail {
  color: var(--oc-color-muted);
  font-size: 0.85rem;
}

.onecalc-home-shell__vba-association {
  display: grid;
  grid-template-columns: minmax(88px, auto) minmax(140px, 1fr) auto auto minmax(120px, auto) auto;
  align-items: center;
  gap: 8px 12px;
  padding: 8px 0;
  border-top: 1px solid var(--oc-color-border);
}

.onecalc-home-shell__vba-source-kind {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--oc-color-muted);
}

.onecalc-home-shell__vba-source-name {
  min-width: 0;
  overflow-wrap: anywhere;
  font-weight: 500;
}

.onecalc-home-shell__vba-source-status,
.onecalc-home-shell__vba-source-detail,
.onecalc-home-shell__vba-source-ref,
.onecalc-home-shell__vba-source-rejected {
  color: var(--oc-color-muted);
  font-size: 0.85rem;
}

.onecalc-home-shell__vba-source-ref {
  min-width: 0;
  overflow-wrap: anywhere;
  font-family: var(--oc-font-mono);
}

.onecalc-home-shell__vba-source-rejected {
  grid-column: 3 / -1;
  overflow-wrap: anywhere;
}

.onecalc-home-shell__vba-association > .onecalc-home-shell__formatting-preset {
  justify-self: end;
}

@media (max-width: 900px) {
  .onecalc-home-shell__vba-association {
    grid-template-columns: minmax(76px, auto) minmax(0, 1fr) auto;
  }

  .onecalc-home-shell__vba-source-status,
  .onecalc-home-shell__vba-source-detail,
  .onecalc-home-shell__vba-source-ref,
  .onecalc-home-shell__vba-source-rejected {
    grid-column: 2 / -1;
  }

  .onecalc-home-shell__vba-association > .onecalc-home-shell__formatting-preset {
    grid-column: 3;
    grid-row: 1;
  }
}

.onecalc-home-shell__palette-row-chord {
  color: var(--oc-color-muted);
  font-family: var(--oc-font-mono);
  font-size: 0.78rem;
  padding: 2px 6px;
  border: 1px solid var(--oc-color-border);
  border-radius: 4px;
}

.onecalc-home-shell__palette-empty {
  padding: 18px 16px;
  color: var(--oc-color-muted);
  font-style: italic;
  text-align: center;
}

/* Manage-formulas overlay — searchable list of every formula in the
   workspace. Sits at the same modal level as the command palette;
   only one of the two is visible at a time. The overlay deliberately
   centers higher than the palette (8vh vs 12vh) since its rows are
   taller and the user benefits from extra scroll height. */
.onecalc-home-shell__manage-formulas-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: flex-start;
  justify-content: center;
  padding-top: 8vh;
  z-index: 60;
}

.onecalc-home-shell__manage-formulas {
  width: min(720px, 96vw);
  max-height: 80vh;
  background: var(--oc-color-surface);
  border-radius: 12px;
  border: 1px solid var(--oc-color-border);
  box-shadow: 0 24px 80px rgba(0, 0, 0, 0.25);
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

.onecalc-home-shell__manage-formulas-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px 6px;
  gap: 12px;
  border-bottom: 1px solid var(--oc-color-border);
}

.onecalc-home-shell__manage-formulas-title {
  margin: 0;
  font-family: var(--oc-font-ui);
  font-size: 1rem;
  font-weight: 600;
  color: var(--oc-color-warm);
}

.onecalc-home-shell__manage-formulas-close {
  border: none;
  background: transparent;
  color: var(--oc-color-muted);
  font: inherit;
  font-size: 1rem;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
}

.onecalc-home-shell__manage-formulas-close:hover {
  color: var(--oc-color-warm);
  background: rgba(0, 0, 0, 0.05);
}

.onecalc-home-shell__manage-formulas-search {
  padding: 8px 16px 12px;
  border-bottom: 1px solid var(--oc-color-border);
}

.onecalc-home-shell__manage-formulas-search-input {
  width: 100%;
  border: 1px solid var(--oc-color-border);
  border-radius: 6px;
  padding: 8px 12px;
  font-family: var(--oc-font-ui);
  font-size: 0.95rem;
  background: var(--oc-color-bg);
  color: inherit;
  outline: none;
}

.onecalc-home-shell__manage-formulas-search-input:focus-visible {
  border-color: var(--oc-color-accent);
  box-shadow: 0 0 0 2px var(--oc-color-accent-soft);
}

.onecalc-home-shell__manage-formulas-rows {
  flex: 1 1 auto;
  overflow-y: auto;
  padding: 4px 0;
}

.onecalc-home-shell__manage-formulas-row {
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: center;
  gap: 16px;
  padding: 10px 16px;
  border-bottom: 1px solid var(--oc-color-border);
}

.onecalc-home-shell__manage-formulas-row:last-of-type {
  border-bottom: none;
}

.onecalc-home-shell__manage-formulas-row[data-is-active="true"] {
  background: var(--oc-color-accent-soft);
}

.onecalc-home-shell__manage-formulas-row-info {
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.onecalc-home-shell__manage-formulas-row-name {
  display: flex;
  align-items: center;
  gap: 6px;
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.onecalc-home-shell__manage-formulas-row-name-text {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.onecalc-home-shell__manage-formulas-row-pin {
  color: var(--oc-color-warm);
  font-size: 0.85rem;
}

.onecalc-home-shell__manage-formulas-row-dirty {
  color: var(--oc-color-warm);
  font-size: 0.6rem;
}

.onecalc-home-shell__manage-formulas-row-preview {
  color: var(--oc-color-muted);
  font-family: var(--oc-font-mono);
  font-size: 0.78rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.onecalc-home-shell__manage-formulas-row-preview[data-empty="true"] {
  font-style: italic;
}

.onecalc-home-shell__manage-formulas-row-actions {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}

.onecalc-home-shell__manage-formulas-row-action {
  border: 1px solid var(--oc-color-border);
  border-radius: 4px;
  background: var(--oc-color-surface);
  font-family: var(--oc-font-ui);
  font-size: 0.78rem;
  padding: 4px 8px;
  cursor: pointer;
  color: inherit;
}

.onecalc-home-shell__manage-formulas-row-action:hover:not(:disabled) {
  background: var(--oc-color-accent-soft);
  border-color: var(--oc-color-accent);
}

.onecalc-home-shell__manage-formulas-row-action:disabled {
  color: var(--oc-color-muted);
  cursor: default;
  opacity: 0.65;
}

.onecalc-home-shell__manage-formulas-empty {
  padding: 18px 16px;
  color: var(--oc-color-muted);
  font-style: italic;
  text-align: center;
}

/* Tab strip — WS-14 §1 minimum-viable surface for switching between
   open formulas. Sits between the titlebar and the editor caption.
   Hidden when only one formula is open (the breadcrumb names it). */
.onecalc-home-shell__tab-strip {
  display: flex;
  align-items: stretch;
  gap: 2px;
  padding: 4px var(--oc-space-5);
  background: var(--oc-color-bg);
  border-bottom: 1px solid var(--oc-color-border);
  overflow-x: auto;
}

.onecalc-home-shell__tab-strip-chip {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding: 0;
  border: 1px solid var(--oc-color-border);
  border-radius: 6px;
  background: var(--oc-color-surface);
  font-family: var(--oc-font-ui);
  font-size: 0.78rem;
  font-weight: 500;
  max-width: 240px;
  min-width: 72px;
  /* Fixed height across active and inactive states. The strip's
     parent uses `align-items: stretch`, so any chip whose
     intrinsic content height differs from its peers (e.g. when
     `font-weight` shifts glyph metrics, when the dirty / pinned
     markers appear, or when the close button line-height drags
     the cross-axis up) would otherwise pull the whole strip
     to that taller height when activated. Locking height here
     keeps the strip visually stable across tab switches. */
  min-height: 28px;
  box-sizing: border-box;
}

.onecalc-home-shell__tab-strip-chip[data-is-active="true"] {
  background: var(--oc-color-accent-soft);
  border-color: var(--oc-color-accent);
}

.onecalc-home-shell__tab-strip-chip-label {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  flex: 1 1 auto;
  padding: 4px 8px;
  border: none;
  background: transparent;
  font: inherit;
  text-align: left;
  cursor: pointer;
  color: inherit;
  overflow: hidden;
}

.onecalc-home-shell__tab-strip-chip-name {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.onecalc-home-shell__tab-strip-rename-input {
  flex: 1 1 auto;
  min-width: 0;
  border: none;
  outline: none;
  background: var(--oc-color-bg);
  font: inherit;
  color: inherit;
  padding: 0 4px;
  border-radius: 3px;
}

.onecalc-home-shell__tab-strip-rename-input:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: -2px;
}

.onecalc-home-shell__tab-strip-pin {
  color: var(--oc-color-warm);
  font-size: 0.7rem;
  flex-shrink: 0;
}

.onecalc-home-shell__tab-strip-dirty {
  color: var(--oc-color-warm);
  font-size: 0.55rem;
  margin-left: 2px;
  flex-shrink: 0;
}

.onecalc-home-shell__tab-strip-chip-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  border: none;
  background: transparent;
  color: var(--oc-color-muted);
  cursor: pointer;
  font-size: 0.85rem;
  line-height: 1;
  border-radius: 4px;
}

.onecalc-home-shell__tab-strip-chip-close:hover {
  color: var(--oc-color-warm);
  background: rgba(0, 0, 0, 0.05);
}

.onecalc-home-shell__tab-strip-chip-label:focus-visible,
.onecalc-home-shell__tab-strip-chip-close:focus-visible,
.onecalc-home-shell__tab-strip-new:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: -2px;
}

.onecalc-home-shell__tab-strip-new {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  border: 1px dashed var(--oc-color-border);
  border-radius: 6px;
  background: transparent;
  color: var(--oc-color-muted);
  cursor: pointer;
  font-size: 0.95rem;
  line-height: 1;
}

.onecalc-home-shell__tab-strip-new:hover {
  color: var(--oc-color-accent);
  border-color: var(--oc-color-accent);
  background: var(--oc-color-accent-soft);
}

/* ----- breadcrumb wrap + button ----------------------------------------- */
.onecalc-home-shell__breadcrumb-wrap {
  position: relative;
  display: inline-flex;
}

.onecalc-home-shell__breadcrumb-button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  color: #e6efef;
  border: 1px solid rgba(255, 255, 255, 0.14);
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.04);
  font: inherit;
  font-weight: 500;
  cursor: pointer;
}

.onecalc-home-shell__breadcrumb-button:hover {
  background: rgba(255, 255, 255, 0.08);
}

.onecalc-home-shell__breadcrumb-button:focus-visible {
  outline: 2px solid var(--oc-color-accent-soft);
  outline-offset: 1px;
}

.onecalc-home-shell__breadcrumb-dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: transparent;
}

.onecalc-home-shell__breadcrumb-button[data-dirty="true"] .onecalc-home-shell__breadcrumb-dot {
  background: var(--oc-color-warning);
}

.onecalc-home-shell__breadcrumb-caret {
  font-size: 10px;
  opacity: 0.7;
}

/* ----- scenario menu (dropdown body) ------------------------------------ */
.onecalc-home-shell__scenario-menu {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  min-width: 320px;
  background: var(--oc-color-surface);
  color: var(--oc-color-ink);
  border: 1px solid var(--oc-color-border);
  border-radius: var(--oc-radius-panel);
  box-shadow: var(--oc-shadow-strong);
  padding: var(--oc-space-3);
  display: none;
  z-index: 40;
}

.onecalc-home-shell__scenario-menu[data-open="true"] {
  display: block;
}

.onecalc-home-shell__scenario-menu-section {
  padding: var(--oc-space-1) var(--oc-space-2);
}

.onecalc-home-shell__scenario-menu-section + .onecalc-home-shell__scenario-menu-section {
  border-top: 1px solid var(--oc-color-border);
  margin-top: var(--oc-space-2);
  padding-top: var(--oc-space-2);
}

.onecalc-home-shell__scenario-menu-heading {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--oc-color-muted);
  margin-bottom: var(--oc-space-1);
}

.onecalc-home-shell__scenario-menu-item {
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: center;
  gap: var(--oc-space-3);
  padding: 6px 8px;
  width: 100%;
  border: none;
  border-radius: 6px;
  background: transparent;
  font: inherit;
  text-align: left;
  cursor: pointer;
  color: inherit;
}

.onecalc-home-shell__scenario-menu-item:hover {
  background: var(--oc-color-accent-soft);
}

.onecalc-home-shell__scenario-menu-item:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: -2px;
}

.onecalc-home-shell__scenario-menu-item[data-is-active="true"] {
  background: rgba(79, 123, 87, 0.12);
}

/* Pin glyph is now rendered as a sibling button so the row click
   target stays distinct from the pin toggle. The legacy ::after
   indicator is suppressed when the row is rendered through the
   `<div class="onecalc-home-shell__scenario-menu-row">` wrapper.
   Kept on bare items (not in a row) for minor surfaces that still
   render a flat list. */

.onecalc-home-shell__scenario-menu-row {
  display: flex;
  align-items: stretch;
  gap: 2px;
  width: 100%;
}

.onecalc-home-shell__scenario-menu-row .onecalc-home-shell__scenario-menu-item {
  flex: 1 1 auto;
}

/* Inside the row the per-item ::after pin indicator is replaced by
   the pin-toggle button — suppress the indicator to avoid a
   double-pin visual. */
.onecalc-home-shell__scenario-menu-row
  .onecalc-home-shell__scenario-menu-item[data-is-pinned="true"]::after {
  content: none;
}

.onecalc-home-shell__scenario-menu-pin {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--oc-color-muted);
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
}

.onecalc-home-shell__scenario-menu-pin:hover {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-warm);
}

.onecalc-home-shell__scenario-menu-pin[data-is-pinned="true"] {
  color: var(--oc-color-warm);
}

.onecalc-home-shell__scenario-menu-pin:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: -2px;
}

.onecalc-home-shell__scenario-menu-item[data-is-pinned="true"]::after {
  content: "📌";
  font-size: 11px;
  opacity: 0.6;
}

.onecalc-home-shell__scenario-menu-item-name {
  font-weight: 500;
}

.onecalc-home-shell__scenario-menu-item-meta {
  color: var(--oc-color-muted);
  font-size: 12px;
}

.onecalc-home-shell__scenario-menu-empty {
  color: var(--oc-color-muted);
  font-size: 12px;
  font-style: italic;
  padding: 6px 8px;
}

.onecalc-home-shell__body {
  max-width: 880px;
  width: 100%;
  margin: 0 auto;
  padding: var(--oc-space-5);
  display: grid;
  gap: var(--oc-space-5);
  align-content: start;
}

.onecalc-home-shell__no-formula-space {
  color: var(--oc-color-muted);
  font-style: italic;
}

.onecalc-home-shell__caption-row {
  display: flex;
  align-items: center;
  gap: var(--oc-space-2);
  margin-bottom: var(--oc-space-2);
}

.onecalc-home-shell__caption {
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--oc-color-ink);
  letter-spacing: 0.01em;
}

.onecalc-home-shell__caption-pill {
  display: inline-flex;
  align-items: center;
  height: 1.2rem;
  padding: 0 var(--oc-space-2);
  border-radius: var(--oc-radius-pill);
  font-family: var(--oc-font-ui);
  font-size: 0.72rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  border: 1px solid var(--oc-color-border);
  background: var(--oc-color-panel);
  color: var(--oc-color-muted);
}

.onecalc-home-shell__caption-pill--entry[data-mode="formula"] {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  border-color: var(--oc-color-card-edge);
}

.onecalc-home-shell__caption-pill--entry[data-mode="value"] {
  background: #eef0e6;
  color: var(--oc-color-success);
  border-color: rgba(79, 123, 87, 0.25);
}

.onecalc-home-shell__caption-pill--entry[data-mode="text"] {
  background: #f1ece1;
  color: var(--oc-color-ink);
}

.onecalc-home-shell__caption-pill--entry[data-mode="empty"] {
  background: var(--oc-color-panel);
  color: var(--oc-color-muted);
  font-style: italic;
}

.onecalc-home-shell__caption-pill--result[data-class="number"] {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  border-color: var(--oc-color-card-edge);
}

.onecalc-home-shell__caption-pill--result[data-class="text"] {
  background: #f1ece1;
  color: var(--oc-color-ink);
}

.onecalc-home-shell__caption-pill--result[data-class="logical"] {
  background: #eef0e6;
  color: var(--oc-color-success);
  border-color: rgba(79, 123, 87, 0.25);
}

.onecalc-home-shell__caption-pill--result[data-class="error"] {
  background: #fff0ea;
  color: var(--oc-color-warm);
  border-color: rgba(183, 101, 69, 0.3);
}

.onecalc-home-shell__caption-pill--result[data-class="array"] {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  border-color: var(--oc-color-card-edge);
}

.onecalc-home-shell__caption-pill--result[data-class="other"] {
  background: var(--oc-color-panel);
  color: var(--oc-color-muted);
}

.onecalc-home-shell__editor {
  display: grid;
}

/* Layered editor frame: the overlay (syntax colouring) stacks underneath
   the textarea with identical box geometry so character offsets align.
   The textarea has transparent text + visible caret; what the user sees
   is the overlay's coloured tokens. */
.onecalc-home-shell__editor-frame {
  position: relative;
  background: var(--oc-color-surface);
  border: 1px solid var(--oc-color-border);
  border-radius: var(--oc-radius-panel);
  box-shadow: var(--oc-shadow-panel);
}

.onecalc-home-shell__editor-frame:focus-within {
  border-color: var(--oc-color-accent);
  box-shadow:
    var(--oc-shadow-panel),
    0 0 0 2px rgba(36, 93, 90, 0.35);
}

.onecalc-home-shell__editor-overlay {
  position: absolute;
  inset: 0;
  padding: var(--oc-space-4);
  font-family: var(--oc-font-mono);
  font-size: 0.95rem;
  line-height: 1.55;
  white-space: pre-wrap;
  word-break: break-word;
  pointer-events: none;
  user-select: none;
  color: var(--oc-color-ink);
  overflow: hidden;
}

.onecalc-home-shell__editor-overlay .syn-op {
  color: var(--oc-color-accent);
  font-weight: 600;
}

.onecalc-home-shell__editor-overlay .syn-fn {
  color: var(--oc-color-accent);
  font-weight: 700;
}

.onecalc-home-shell__editor-overlay .syn-num {
  color: var(--oc-color-success);
}

.onecalc-home-shell__editor-overlay .syn-delim {
  color: var(--oc-color-muted);
}

/* Rotating-colour bracket nesting (WS-14 §2.3 item 4). The renderer
   tags each `(` `)` `[` `]` `{` `}` span with `syn-bracket` and
   `syn-bracket--depth-N` (N modulo 4). Four colours pulled from the
   existing accent rhythm so the brackets read as part of the warm
   palette rather than a separate ink layer. */
.onecalc-home-shell__editor-overlay .syn-bracket {
  font-weight: 600;
}

.onecalc-home-shell__editor-overlay .syn-bracket--depth-0 {
  color: var(--oc-color-accent);
}

.onecalc-home-shell__editor-overlay .syn-bracket--depth-1 {
  color: var(--oc-color-warm);
}

.onecalc-home-shell__editor-overlay .syn-bracket--depth-2 {
  color: var(--oc-color-warning);
}

.onecalc-home-shell__editor-overlay .syn-bracket--depth-3 {
  color: var(--oc-color-success);
}

/* Active-pair emphasis: the bracket pair surrounding the caret gets
   a soft accent box + bold weight. Keeps the rotating colour intact
   so the user can still see the depth, but pulls the eye to the pair
   under inspection. */
.onecalc-home-shell__editor-overlay .syn-bracket--active {
  font-weight: 800;
  background: rgba(36, 93, 90, 0.15);
  border-radius: 3px;
}

.onecalc-home-shell__editor-overlay .syn-id {
  color: var(--oc-color-ink);
}

.onecalc-home-shell__editor-overlay .syn-text {
  color: var(--oc-color-ink);
}

/* Diagnostic squiggle overlay: shares geometry with the syntax overlay
   and sits in the same stacking position. Text is invisible; only the
   wavy underline shows, drawing attention to the diagnostic span. */
.onecalc-home-shell__editor-squiggles {
  position: absolute;
  inset: 0;
  padding: var(--oc-space-4);
  font-family: var(--oc-font-mono);
  font-size: 0.95rem;
  line-height: 1.55;
  white-space: pre-wrap;
  word-break: break-word;
  pointer-events: none;
  user-select: none;
  color: transparent;
  -webkit-text-fill-color: transparent;
  overflow: hidden;
}

.onecalc-home-shell__editor-squiggles .squiggle {
  /* `auto` allows hover tooltips even though the overlay is
     pointer-events:none; squiggle re-enables pointer events for itself. */
  pointer-events: auto;
  text-decoration-line: underline;
  text-decoration-style: wavy;
  text-decoration-skip-ink: none;
  text-underline-offset: 2px;
}

.onecalc-home-shell__editor-squiggles .squiggle--error {
  text-decoration-color: var(--oc-color-warm);
}

.onecalc-home-shell__editor-squiggles .squiggle--warning {
  text-decoration-color: var(--oc-color-warning);
}

.onecalc-home-shell__editor-squiggles .squiggle--info {
  text-decoration-color: var(--oc-color-muted);
}

/* Foot rows: small caption row under each section carrying live counts
   and active-context summaries. */
.onecalc-home-shell__foot-row {
  display: flex;
  align-items: center;
  gap: var(--oc-space-2);
  margin-top: var(--oc-space-2);
  font-size: 0.78rem;
  font-family: var(--oc-font-ui);
  color: var(--oc-color-muted);
  letter-spacing: 0.02em;
}

.onecalc-home-shell__chip {
  display: inline-flex;
  align-items: center;
  gap: var(--oc-space-1);
  padding: 2px var(--oc-space-2);
  border-radius: var(--oc-radius-pill);
  background: var(--oc-color-panel);
  border: 1px solid var(--oc-color-border);
  color: var(--oc-color-muted);
  font-family: var(--oc-font-mono);
}

.onecalc-home-shell__chip--metrics {
  font-variant-numeric: tabular-nums;
}

.onecalc-home-shell__chip-sep {
  color: var(--oc-color-border);
}

.onecalc-home-shell__chip-field {
  color: var(--oc-color-ink);
}

.onecalc-home-shell__chip-field--seam {
  color: var(--oc-color-warm);
}

.onecalc-home-shell__chip-seam {
  margin-left: var(--oc-space-1);
  font-size: 0.72rem;
  color: var(--oc-color-warm);
  background: rgba(183, 101, 69, 0.10);
  padding: 0 var(--oc-space-1);
  border-radius: 3px;
}

/* User-mode editor-foot chip variants. The "ready" state is
   muted-sage to subtly confirm the formula parses, and the
   "warning" state surfaces the first diagnostic message in the
   warm error palette. */
.onecalc-home-shell__chip--ready {
  background: rgba(79, 123, 87, 0.12);
  color: var(--oc-color-success);
  border-color: rgba(79, 123, 87, 0.25);
  font-style: italic;
}

.onecalc-home-shell__chip--warning {
  background: rgba(183, 101, 69, 0.10);
  color: var(--oc-color-warm);
  border-color: rgba(183, 101, 69, 0.30);
  font-weight: 500;
  /* Allow the message to wrap onto multiple lines if long. */
  white-space: normal;
  word-break: break-word;
}

/* Completion popup: absolute-positioned inside the editor frame at
   the caret anchor. The wrapper itself does not capture pointer
   events so background clicks fall through to the textarea; each row
   re-enables pointer events for its own click handler. */
.onecalc-completion-popup {
  position: absolute;
  z-index: 10;
  width: 220px;
  max-height: 180px;
  overflow-y: auto;
  background: var(--oc-color-surface);
  border: 1px solid var(--oc-color-card-edge);
  border-radius: 6px;
  /* Lighter shadow than --oc-shadow-strong: this is a dropdown,
     not a dialog. */
  box-shadow: 0 6px 14px rgba(31, 42, 44, 0.10);
  font-family: var(--oc-font-ui);
  pointer-events: none;
  padding: 2px 0;
}

.onecalc-completion-popup__item {
  display: grid;
  grid-template-columns: 0.9rem 1fr auto;
  align-items: center;
  gap: 6px;
  /* Tighter row padding so the popup reads as a quiet dropdown
     rather than a card. */
  padding: 1px var(--oc-space-2);
  cursor: pointer;
  pointer-events: auto;
  font-size: 0.78rem;
  line-height: 1.35;
  color: var(--oc-color-ink);
}

.onecalc-completion-popup__item:hover {
  background: rgba(36, 93, 90, 0.06);
}

.onecalc-completion-popup__item[data-selected="true"] {
  background: var(--oc-color-accent);
  color: #fffaf4;
}

.onecalc-completion-popup__glyph {
  font-family: var(--oc-font-mono);
  font-weight: 500;
  font-size: 0.72rem;
  text-align: center;
  color: var(--oc-color-accent);
}

.onecalc-completion-popup__item[data-selected="true"] .onecalc-completion-popup__glyph {
  color: #fffaf4;
}

.onecalc-completion-popup__text {
  font-family: var(--oc-font-mono);
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.onecalc-completion-popup__kind {
  font-size: 0.6rem;
  color: var(--oc-color-muted);
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.onecalc-completion-popup__item[data-selected="true"] .onecalc-completion-popup__kind {
  color: rgba(255, 250, 244, 0.78);
}

/* ===== Signature-help line ====================================
   A non-interactive one-line tooltip rendered ABOVE the caret while
   the caret sits inside an open function call (`=SUM(`). Re-uses the
   completion-popup anchoring scheme but transforms upward so the
   tooltip sits on the line above the caret rather than below. The
   completion popup wins when both want the same caret — see the
   view-model projector's suppression rule. */
.onecalc-signature-help {
  position: absolute;
  z-index: 9; /* one below the completion popup (z=10) */
  /* Anchor the BOTTOM-left of the tooltip at the caret-box top, so
     the help renders entirely above the caret. The 6px gap keeps
     the tooltip from kissing the caret. */
  transform: translateY(calc(-100% - 6px));
  /* Match the completion popup chrome: same surface, border, radius,
     and shadow weight. The signature-help is the popup's *partner*
     — a quiet, non-interactive sibling — so the two should read as a
     pair, not as two unrelated tooltips. */
  max-width: 380px;
  padding: 1px var(--oc-space-2);
  background: var(--oc-color-surface);
  border: 1px solid var(--oc-color-card-edge);
  border-radius: 6px;
  box-shadow: 0 6px 14px rgba(31, 42, 44, 0.10);
  font-family: var(--oc-font-mono);
  font-size: 0.78rem;
  line-height: 1.35;
  color: var(--oc-color-ink);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  pointer-events: none;
  /* Inline-flex so the callee, parens, and parameters share the row
     baseline cleanly — matches the completion-popup item's grid
     density without making the help a grid (its content is variable
     width and a flow layout reads as a function-signature line). */
  display: inline-flex;
  align-items: baseline;
  gap: 0;
}

/* Callee gets the same accent treatment as the completion popup's
   selected-state glyph: bold, accent-coloured, monospace. */
.onecalc-signature-help__callee {
  font-weight: 600;
  color: var(--oc-color-accent);
}

.onecalc-signature-help__paren {
  color: var(--oc-color-muted);
}

.onecalc-signature-help__parameter {
  color: var(--oc-color-ink);
}

/* Active-parameter treatment mirrors the completion popup's selected
   item: bold + accent fill, but without the underline (the popup uses
   background, the signature line uses inline emphasis since it has no
   row to fill). */
.onecalc-signature-help__parameter--active {
  font-weight: 700;
  color: var(--oc-color-accent);
}

.onecalc-signature-help__separator {
  color: var(--oc-color-muted);
}

/* ===== Function-help hover tooltip ============================
   Multi-line tooltip rendered below a `.syn-fn` span when the
   pointer is over the function token AND the bridge has
   function-help for it. Wider than the signature-help line
   (multi-line content); lower z-index than the completion
   popup so the popup wins when both could appear. */
.onecalc-function-help {
  position: absolute;
  z-index: 8;
  max-width: 380px;
  padding: var(--oc-space-3);
  background: var(--oc-color-surface);
  border: 1px solid var(--oc-color-card-edge);
  border-radius: var(--oc-radius-panel);
  box-shadow: var(--oc-shadow-panel);
  font-family: var(--oc-font-ui);
  font-size: 0.85rem;
  line-height: 1.4;
  color: var(--oc-color-ink);
  pointer-events: none;
}

.onecalc-function-help__heading {
  font-weight: 700;
  color: var(--oc-color-accent);
  font-size: 0.9rem;
  margin-bottom: 0.15rem;
}

.onecalc-function-help__signature {
  font-family: var(--oc-font-mono);
  color: var(--oc-color-ink);
  margin-bottom: 0.35rem;
}

.onecalc-function-help__description {
  color: var(--oc-color-muted);
  margin-bottom: 0.25rem;
}

.onecalc-function-help__availability {
  font-size: 0.72rem;
  color: var(--oc-color-muted);
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.onecalc-function-help[data-deferred="true"] {
  border-color: var(--oc-color-warning);
}

/* ===== Formula walk-tree drill-down ============================
   First progressive-disclosure surface. The toggle row sits in
   the editor-foot next to the live-metrics chip; the panel
   itself is a SECTION between the editor and the result so the
   result hero pushes down (rather than overlaying or being
   covered) when expanded. */

.onecalc-home-shell__formula-drill-toggle {
  display: inline-flex;
  align-items: center;
  height: 1.25rem;
  padding: 0 var(--oc-space-2);
  background: var(--oc-color-panel);
  border: 1px solid var(--oc-color-border);
  border-radius: var(--oc-radius-pill);
  cursor: pointer;
  font-family: var(--oc-font-ui);
  font-size: 0.78rem;
  color: var(--oc-color-muted);
}

.onecalc-home-shell__formula-drill-toggle:hover {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  border-color: var(--oc-color-card-edge);
}

.onecalc-home-shell__formula-drill-toggle[data-expanded="true"] {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  border-color: var(--oc-color-card-edge);
}

.onecalc-home-shell__formula-drill-section {
  display: grid;
}

.onecalc-home-shell__formula-drill-panel {
  background: rgba(36, 93, 90, 0.04);
  border: 1px solid var(--oc-color-border);
  border-radius: var(--oc-radius-panel);
  padding: 0;
  max-height: 0;
  overflow: hidden;
  /* Default: zero animation. The @media block below adds a
     200 ms transition only when the user has not requested
     reduced motion. */
}

.onecalc-home-shell__formula-drill-panel[data-expanded="true"] {
  max-height: 320px;
  overflow-y: auto;
  padding: var(--oc-space-3);
}

@media (prefers-reduced-motion: no-preference) {
  .onecalc-home-shell__formula-drill-panel {
    transition: max-height 200ms ease, padding 200ms ease;
  }
}

.onecalc-home-shell__formula-drill-loading {
  font-family: var(--oc-font-ui);
  color: var(--oc-color-muted);
  font-style: italic;
  font-size: 0.85rem;
}

.onecalc-home-shell__formula-drill-tree {
  display: grid;
  gap: 2px;
}

.onecalc-home-shell__formula-drill-row {
  font-family: var(--oc-font-mono);
  font-size: 0.8rem;
  color: var(--oc-color-ink);
}

/* Leaf row — terminal node with no children. Same grid layout
   as the summary row inside a branch so leaves and summaries
   align horizontally. */
.onecalc-home-shell__formula-drill-row--leaf {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: var(--oc-space-2);
  padding: 2px var(--oc-space-2);
  border-radius: 4px;
}

.onecalc-home-shell__formula-drill-row--leaf:hover {
  background: rgba(36, 93, 90, 0.06);
}

/* Branch row — `<details>` element that holds the summary plus
   nested children. The default disclosure marker is suppressed in
   favour of a chevron that rotates with the open / closed state. */
.onecalc-home-shell__formula-drill-row--branch {
  border-left: 1px solid var(--oc-color-border);
  margin-left: 4px;
  padding-left: 2px;
}

.onecalc-home-shell__formula-drill-row--branch > summary {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: var(--oc-space-2);
  padding: 2px var(--oc-space-2);
  border-radius: 4px;
  cursor: pointer;
  list-style: none;
}

.onecalc-home-shell__formula-drill-row--branch > summary::before {
  content: "▾";
  font-family: var(--oc-font-ui);
  font-size: 0.7rem;
  color: var(--oc-color-muted);
  width: 0.7rem;
  display: inline-block;
}

.onecalc-home-shell__formula-drill-row--branch:not([open]) > summary::before {
  content: "▸";
}

.onecalc-home-shell__formula-drill-row--branch > summary::-webkit-details-marker {
  display: none;
}

.onecalc-home-shell__formula-drill-row--branch > summary:hover {
  background: rgba(36, 93, 90, 0.06);
}

.onecalc-home-shell__formula-drill-row-children {
  margin-left: 0.8rem;
  padding-left: var(--oc-space-2);
  border-left: 1px dotted var(--oc-color-border);
}

/* Diagnostics list inside the drill-down panel. Vertical stack
   of severity-coloured rows; each row carries the diagnostic's
   message + (Developer mode) stage and code. */
.onecalc-home-shell__formula-drill-diagnostics {
  list-style: none;
  margin: 0 0 var(--oc-space-3) 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.onecalc-home-shell__formula-drill-diagnostic {
  display: grid;
  grid-template-columns: auto 1fr;
  align-items: baseline;
  gap: var(--oc-space-2);
  padding: 4px var(--oc-space-2);
  border-radius: 4px;
  font-family: var(--oc-font-ui);
  font-size: 0.8rem;
}

.onecalc-home-shell__formula-drill-diagnostic[data-severity="error"] {
  background: rgba(183, 101, 69, 0.10);
}

.onecalc-home-shell__formula-drill-diagnostic[data-severity="warning"] {
  background: rgba(197, 139, 47, 0.10);
}

.onecalc-home-shell__formula-drill-diagnostic[data-severity="info"] {
  background: rgba(36, 93, 90, 0.06);
}

.onecalc-home-shell__formula-drill-diagnostic-severity {
  font-size: 0.65rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.onecalc-home-shell__formula-drill-diagnostic-severity[data-severity="error"] {
  color: var(--oc-color-warm);
}

.onecalc-home-shell__formula-drill-diagnostic-severity[data-severity="warning"] {
  color: var(--oc-color-warning);
}

.onecalc-home-shell__formula-drill-diagnostic-severity[data-severity="info"] {
  color: var(--oc-color-accent);
}

.onecalc-home-shell__formula-drill-diagnostic-message {
  font-family: var(--oc-font-mono);
  font-size: 0.78rem;
  color: var(--oc-color-ink);
}

/* Developer-view toggle inside the drill-down panel header.
   Right-aligned, small, opt-in surface for power users.
   `aria-pressed` reflects the active mode. */
.onecalc-home-shell__formula-drill-mode-toggle {
  display: flex;
  justify-content: flex-end;
  margin-bottom: var(--oc-space-2);
}

.onecalc-home-shell__formula-drill-mode-button {
  font-family: var(--oc-font-ui);
  font-size: 0.72rem;
  padding: 2px 10px;
  border: 1px solid var(--oc-color-border);
  border-radius: 999px;
  background: transparent;
  color: var(--oc-color-muted);
  cursor: pointer;
}

.onecalc-home-shell__formula-drill-mode-button[aria-pressed="true"] {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  border-color: var(--oc-color-card-edge);
}

.onecalc-home-shell__formula-drill-mode-button:hover {
  color: var(--oc-color-accent);
  border-color: var(--oc-color-accent);
}

.onecalc-home-shell__formula-drill-state {
  display: inline-flex;
  align-items: center;
  height: 1rem;
  padding: 0 var(--oc-space-2);
  border-radius: var(--oc-radius-pill);
  font-family: var(--oc-font-ui);
  font-size: 0.65rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.onecalc-home-shell__formula-drill-state[data-state="evaluated"] {
  background: rgba(79, 123, 87, 0.18);
  color: var(--oc-color-success);
}

.onecalc-home-shell__formula-drill-state[data-state="bound"] {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
}

.onecalc-home-shell__formula-drill-state[data-state="opaque"] {
  background: var(--oc-color-panel);
  color: var(--oc-color-muted);
}

.onecalc-home-shell__formula-drill-state[data-state="blocked"] {
  background: rgba(183, 101, 69, 0.15);
  color: var(--oc-color-warm);
}

.onecalc-home-shell__formula-drill-label {
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.onecalc-home-shell__formula-drill-value {
  color: var(--oc-color-muted);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 18rem;
  text-align: right;
}

.onecalc-home-shell__formula-drill-array {
  grid-column: 1 / -1;
  margin: 4px 0 6px 18px;
  padding: 4px 6px;
  border: 1px solid var(--oc-color-border);
  border-radius: 4px;
  background: color-mix(in srgb, var(--oc-color-surface) 92%, var(--oc-color-accent-soft));
}

.onecalc-home-shell__formula-drill-array-summary {
  cursor: pointer;
  color: var(--oc-color-muted);
  font-family: var(--oc-font-mono);
  font-size: 0.72rem;
  line-height: 1.4;
}

.onecalc-home-shell__formula-drill-array-hidden {
  margin-left: 8px;
  color: var(--oc-color-subtle);
}

.onecalc-home-shell__formula-drill-array-grid {
  display: grid;
  gap: 0;
  margin-top: 5px;
  width: max-content;
  max-width: 100%;
  overflow: auto;
  border-top: 1px solid var(--oc-color-border);
  border-left: 1px solid var(--oc-color-border);
}

.onecalc-home-shell__formula-drill-array-cell {
  min-width: 2.75rem;
  min-height: 1.45rem;
  padding: 3px 7px;
  border-right: 1px solid var(--oc-color-border);
  border-bottom: 1px solid var(--oc-color-border);
  background: var(--oc-color-surface);
  color: var(--oc-color-ink);
  font-family: var(--oc-font-mono);
  font-size: 0.74rem;
  line-height: 1.25;
  white-space: pre;
  overflow: hidden;
  text-overflow: ellipsis;
}

.onecalc-home-shell__formula-drill-phase-strip {
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
  margin-top: var(--oc-space-3);
  padding-top: var(--oc-space-2);
  border-top: 1px dashed var(--oc-color-border);
}

.onecalc-home-shell__formula-drill-phase {
  display: inline-flex;
  align-items: center;
  gap: var(--oc-space-1);
  padding: 2px var(--oc-space-2);
  border-radius: var(--oc-radius-pill);
  font-family: var(--oc-font-mono);
  font-size: 0.75rem;
  background: var(--oc-color-panel);
  color: var(--oc-color-muted);
}

.onecalc-home-shell__formula-drill-phase[data-state="ok"] strong {
  color: var(--oc-color-success);
}

.onecalc-home-shell__formula-drill-phase[data-state="blocked"] {
  background: rgba(183, 101, 69, 0.10);
  color: var(--oc-color-warm);
}

.onecalc-home-shell__formula-drill-phase[data-state="blocked"] strong {
  color: var(--oc-color-warm);
}

/* ===== Formula drill — User-mode rendering ====================
   In User mode the row is laid out equation-style:
     label = value
   The state chip is dropped, the equals sign and value column
   are emphasised. Blocked rows replace the equals + value with a
   small terracotta "blocked" tag — the only state an Excel user
   genuinely needs to see.
   The data-mode attribute on each row + the tree wrapper drives
   the layout switch. */

.onecalc-home-shell__formula-drill-row[data-mode="user"] {
  /* Two columns: label flexes, value right-aligns. The equals
     sign sits between as a fixed-width hairline column. */
  grid-template-columns: 1fr auto 1fr;
}

.onecalc-home-shell__formula-drill-row[data-mode="user"]
  .onecalc-home-shell__formula-drill-state {
  /* Hide the developer-mode state chip in User mode without
     dropping it from the DOM (the corpus reads `data-state` on
     the row itself). */
  display: none;
}

.onecalc-home-shell__formula-drill-equals {
  color: var(--oc-color-muted);
  font-family: var(--oc-font-mono);
  padding: 0 var(--oc-space-1);
}

.onecalc-home-shell__formula-drill-blocked-tag {
  display: inline-flex;
  align-items: center;
  height: 1rem;
  padding: 0 var(--oc-space-2);
  border-radius: var(--oc-radius-pill);
  background: rgba(183, 101, 69, 0.15);
  color: var(--oc-color-warm);
  font-family: var(--oc-font-ui);
  font-size: 0.65rem;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.onecalc-home-shell__formula-drill-row[data-mode="user"][data-state="blocked"] {
  /* Slight tint on blocked rows so they stand out from the
     equation list. */
  background: rgba(183, 101, 69, 0.05);
}

/* User-mode phase strip: a single status line replacing the
   per-phase chip strip. */
.onecalc-home-shell__formula-drill-status {
  display: inline-flex;
  align-items: center;
  gap: var(--oc-space-1);
  padding: 2px var(--oc-space-2);
  border-radius: var(--oc-radius-pill);
  font-family: var(--oc-font-ui);
  font-size: 0.78rem;
  letter-spacing: 0.02em;
}

.onecalc-home-shell__formula-drill-status--ok {
  background: rgba(79, 123, 87, 0.10);
  color: var(--oc-color-success);
}

.onecalc-home-shell__formula-drill-status--blocked {
  background: rgba(183, 101, 69, 0.12);
  color: var(--oc-color-warm);
}

.onecalc-home-shell__textarea {
  position: relative;
  width: 100%;
  min-height: 5.5rem;
  padding: var(--oc-space-4);
  background: transparent;
  border: 0;
  border-radius: var(--oc-radius-panel);
  font-family: var(--oc-font-mono);
  font-size: 0.95rem;
  line-height: 1.55;
  color: transparent;
  caret-color: var(--oc-color-ink);
  -webkit-text-fill-color: transparent;
  resize: vertical;
  outline: none;
}

.onecalc-home-shell__textarea::selection {
  background: rgba(36, 93, 90, 0.18);
  color: transparent;
  -webkit-text-fill-color: transparent;
}

.onecalc-home-shell__result-section {
  display: grid;
}

/* ----- formatting controls (slice 5) ----------------------------------- */
.onecalc-home-shell__formatting-section {
  display: block;
}

/* WS-14 §5.3 item 8: collapsible formatting panel that lives between
   the formula drill-down and the result section. Default state is
   collapsed — only the toggle chip with the one-line summary is
   visible; clicking the chip expands the body which holds the full
   `.onecalc-home-shell__formatting-row` controls. */
.onecalc-home-shell__formatting-panel {
  display: flex;
  flex-direction: column;
  gap: var(--oc-space-2);
}

.onecalc-home-shell__formatting-toggle-button {
  align-self: flex-start;
  font-family: var(--oc-font-ui);
  font-size: 0.85rem;
  font-weight: 500;
  padding: 4px 12px;
  border: 1px solid var(--oc-color-border);
  border-radius: 999px;
  background: var(--oc-color-panel);
  color: var(--oc-color-ink);
  cursor: pointer;
  transition: background 120ms ease;
}

.onecalc-home-shell__formatting-toggle-button:hover {
  background: var(--oc-color-surface);
}

.onecalc-home-shell__formatting-toggle-button:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: 2px;
}

.onecalc-home-shell__formatting-toggle-button[data-expanded="true"] {
  background: var(--oc-color-surface);
  border-color: var(--oc-color-accent);
}

.onecalc-home-shell__formatting-panel-body[data-expanded="false"] {
  display: none;
}

.onecalc-home-shell__formatting-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--oc-space-3);
  padding: var(--oc-space-3) var(--oc-space-4);
  background: var(--oc-color-panel);
  border: 1px solid var(--oc-color-border);
  border-radius: var(--oc-radius-panel);
  font-family: var(--oc-font-ui);
  font-size: 0.85rem;
}

.onecalc-home-shell__formatting-caption {
  font-weight: 600;
  color: var(--oc-color-ink);
  letter-spacing: 0.01em;
  margin-right: var(--oc-space-2);
}

.onecalc-home-shell__formatting-field {
  display: flex;
  align-items: center;
  gap: var(--oc-space-2);
}

.onecalc-home-shell__formatting-field-label {
  color: var(--oc-color-muted);
  font-size: 0.78rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.onecalc-home-shell__formatting-input {
  font-family: var(--oc-font-mono);
  font-size: 0.85rem;
  padding: 4px 8px;
  border: 1px solid var(--oc-color-border);
  border-radius: 4px;
  background: var(--oc-color-surface);
  /* Wide enough for custom Excel format codes like
     `_($* #,##0.00_);_($* (#,##0.00);_($* "-"??_);_(@_)` to be
     visible without horizontal scrolling — power users paste long
     codes here; tripling the prior `min-width: 8rem` is the
     ergonomic default. */
  min-width: 24rem;
}

.onecalc-home-shell__formatting-input:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: 0;
}

.onecalc-home-shell__formatting-color {
  width: 2rem;
  height: 1.6rem;
  border: 1px solid var(--oc-color-border);
  border-radius: 4px;
  padding: 1px;
  background: var(--oc-color-surface);
  cursor: pointer;
}

.onecalc-home-shell__formatting-toggle {
  width: 1rem;
  height: 1rem;
  cursor: pointer;
}

/* Multi-row formatting body: format / calc-options / CF rules.
   Each row is the same flex strip as the original single-row layout,
   stacked vertically inside the panel body. */
.onecalc-home-shell__formatting-rows {
  display: flex;
  flex-direction: column;
  gap: var(--oc-space-2);
}

/* SEAM marker on preset chips and CF rules — inline ⚠ glyph in the
   warning palette colour. Hover reveals the SEAM id via title /
   aria-describedby; the badge is small enough not to dominate the
   chip but visible enough to flag the gap. */
.onecalc-home-shell__formatting-preset-seam {
  display: inline-block;
  margin-left: 4px;
  font-size: 0.7rem;
  color: var(--oc-color-warning);
  cursor: help;
}

/* Calc-options two-button segmented control. The active button gets
   the accent fill; the inactive button stays panel-coloured. */
.onecalc-home-shell__formatting-policy-toggle {
  display: inline-flex;
  border: 1px solid var(--oc-color-border);
  border-radius: 999px;
  overflow: hidden;
}

.onecalc-home-shell__formatting-policy-button {
  font-family: var(--oc-font-ui);
  font-size: 0.78rem;
  padding: 4px 12px;
  border: none;
  background: var(--oc-color-panel);
  color: var(--oc-color-ink);
  cursor: pointer;
  transition: background 120ms ease, color 120ms ease;
}

.onecalc-home-shell__formatting-policy-button[data-active="true"] {
  background: var(--oc-color-accent);
  color: #fffaf4;
}

/* Locale picker inside the formatting panel's calc-options row.
   The `<select>` lets the user switch the workspace's ambient
   format-code triple immediately. The SEAM badge stays alongside
   to flag that runtime locale tables (month names, separators,
   currency) are pending upstream — picking a locale only swaps
   the format-code defaults today. */
.onecalc-home-shell__formatting-locale {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 10px;
  font-family: var(--oc-font-ui);
  font-size: 0.78rem;
  border: 1px dashed var(--oc-color-border);
  border-radius: 999px;
  color: var(--oc-color-muted);
}

.onecalc-home-shell__formatting-locale-select {
  font-family: var(--oc-font-ui);
  font-size: 0.78rem;
  padding: 2px 6px;
  border: 1px solid var(--oc-color-border);
  border-radius: 4px;
  background: var(--oc-color-bg);
  color: var(--oc-color-ink);
}

.onecalc-home-shell__formatting-locale-select:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: 0;
}

.onecalc-home-shell__formatting-locale-seam {
  color: var(--oc-color-warning);
  font-size: 0.7rem;
  cursor: help;
}

.onecalc-home-shell__formatting-policy-button:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: 2px;
}

/* CF rules list: stacked rule cards + an `+ add rule` button at the
   bottom. Each card is its own flex row of small fields. */
.onecalc-home-shell__cf-rules {
  display: flex;
  flex-direction: column;
  gap: var(--oc-space-2);
  align-items: flex-start;
}

.onecalc-home-shell__cf-rule {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--oc-space-2);
  padding: 4px var(--oc-space-2);
  background: var(--oc-color-surface);
  border: 1px solid var(--oc-color-border);
  border-radius: var(--oc-radius-panel);
  font-family: var(--oc-font-ui);
  font-size: 0.78rem;
}

.onecalc-home-shell__cf-rule-field {
  display: flex;
  align-items: center;
  gap: 4px;
}

.onecalc-home-shell__cf-rule-field-label {
  color: var(--oc-color-muted);
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}

.onecalc-home-shell__cf-rule-input {
  font-family: var(--oc-font-mono);
  font-size: 0.78rem;
  padding: 2px 6px;
  border: 1px solid var(--oc-color-border);
  border-radius: 4px;
  background: var(--oc-color-bg);
  min-width: 4rem;
}

.onecalc-home-shell__cf-rule-input:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: 0;
}

.onecalc-home-shell__cf-rule-seam {
  color: var(--oc-color-warning);
  font-size: 0.7rem;
  font-weight: 600;
  cursor: help;
}

.onecalc-home-shell__cf-rule-remove {
  margin-left: auto;
  font-size: 0.85rem;
  border: none;
  background: transparent;
  color: var(--oc-color-muted);
  cursor: pointer;
  padding: 2px 4px;
}

.onecalc-home-shell__cf-rule-remove:hover {
  color: var(--oc-color-warm);
}

/* Typed CF rule sub-form. Sits as a full-width row below the
   single-line rule header; flex-basis: 100% forces a wrap inside
   .onecalc-home-shell__cf-rule's wrap container. */
.onecalc-home-shell__cf-rule-typed-subform {
  flex-basis: 100%;
  display: flex;
  flex-wrap: wrap;
  gap: var(--oc-space-2);
  padding: 6px 8px;
  margin-top: 4px;
  background: var(--oc-color-bg);
  border: 1px dashed var(--oc-color-border);
  border-radius: var(--oc-radius-panel);
}

.onecalc-home-shell__cf-rule-typed-stop {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 2px 4px;
  background: var(--oc-color-surface);
  border: 1px solid var(--oc-color-border);
  border-radius: 4px;
}

.onecalc-home-shell__cf-rule-typed-stop-remove {
  font-size: 0.75rem;
  border: none;
  background: transparent;
  color: var(--oc-color-muted);
  cursor: pointer;
  padding: 2px 4px;
}

.onecalc-home-shell__cf-rule-typed-stop-remove:hover {
  color: var(--oc-color-warm);
}

.onecalc-home-shell__cf-rule-typed-add {
  font-family: var(--oc-font-ui);
  font-size: 0.72rem;
  padding: 2px 10px;
  border: 1px dashed var(--oc-color-border);
  border-radius: 999px;
  background: transparent;
  color: var(--oc-color-muted);
  cursor: pointer;
}

.onecalc-home-shell__cf-rule-typed-add:hover {
  color: var(--oc-color-accent);
  border-color: var(--oc-color-accent);
}

.onecalc-home-shell__cf-rule-typed-checkbox {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 0.72rem;
  color: var(--oc-color-muted);
}

.onecalc-home-shell__cf-add-button {
  font-family: var(--oc-font-ui);
  font-size: 0.78rem;
  padding: 4px 12px;
  border: 1px dashed var(--oc-color-border);
  border-radius: 999px;
  background: transparent;
  color: var(--oc-color-muted);
  cursor: pointer;
}

.onecalc-home-shell__cf-add-button:hover {
  color: var(--oc-color-accent);
  border-color: var(--oc-color-accent);
}

/* Editor-foot recalculate trigger. Mirrors the F9 keystroke and
   sits alongside the formula drill-down toggle in the foot row. */
.onecalc-home-shell__recalculate-button {
  font-family: var(--oc-font-ui);
  font-size: 0.78rem;
  padding: 4px 12px;
  border: 1px solid var(--oc-color-border);
  border-radius: 999px;
  background: var(--oc-color-panel);
  color: var(--oc-color-ink);
  cursor: pointer;
  margin-left: auto;
  transition: background 120ms ease, color 120ms ease;
}

.onecalc-home-shell__recalculate-button:hover {
  background: var(--oc-color-surface);
  color: var(--oc-color-accent);
}

.onecalc-home-shell__recalculate-button:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: 2px;
}

.onecalc-home-shell__formatting-presets {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.onecalc-home-shell__formatting-preset {
  padding: 2px 8px;
  font-size: 0.75rem;
  font-family: var(--oc-font-ui);
  border: 1px solid var(--oc-color-border);
  border-radius: var(--oc-radius-pill);
  background: var(--oc-color-surface);
  color: var(--oc-color-muted);
  cursor: pointer;
}

.onecalc-home-shell__formatting-preset:hover {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  border-color: var(--oc-color-card-edge);
}

.onecalc-home-shell__formatting-preset:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: 1px;
}

.onecalc-home-shell__result-block {
  min-height: 4.5rem;
  padding: var(--oc-space-4) var(--oc-space-5);
  background: var(--oc-color-surface);
  border: 1px solid var(--oc-color-border);
  border-radius: var(--oc-radius-panel);
  box-shadow: var(--oc-shadow-panel);
  display: grid;
  place-items: center;
  font-family: var(--oc-font-mono);
}

.onecalc-home-shell__result-block .muted {
  color: var(--oc-color-muted);
  font-family: var(--oc-font-ui);
  font-style: italic;
  font-size: 0.95rem;
}

.onecalc-home-shell__result-block .value {
  font-size: 2rem;
  font-weight: 600;
  color: var(--oc-color-ink);
  letter-spacing: -0.005em;
  line-height: 1.1;
  text-align: center;
}

.onecalc-home-shell__result-block .value.array {
  font-size: 1.25rem;
  font-weight: 500;
  color: var(--oc-color-accent);
}

.onecalc-home-shell__result-block[data-kind="error"] {
  background: #fff0ea;
  border-color: rgba(183, 101, 69, 0.3);
}

.onecalc-home-shell__result-block .value.error {
  display: grid;
  gap: var(--oc-space-1);
  text-align: center;
}

.onecalc-home-shell__result-block .value.error .value__code {
  color: var(--oc-color-warm);
  font-size: 1.5rem;
  font-weight: 700;
  letter-spacing: 0.02em;
}

.onecalc-home-shell__result-block .value.error .value__surface {
  color: var(--oc-color-muted);
  font-family: var(--oc-font-ui);
  font-size: 0.9rem;
  font-weight: 400;
}

.onecalc-home-shell__statusfoot {
  height: 28px;
  display: flex;
  align-items: center;
  gap: var(--oc-space-2);
  padding: 0 var(--oc-space-5);
  background: var(--oc-color-panel);
  color: var(--oc-color-muted);
  border-top: 1px solid var(--oc-color-border);
  font-family: var(--oc-font-mono);
  font-size: 0.74rem;
  letter-spacing: 0.02em;
}

.onecalc-home-shell__statusfoot-dot {
  display: inline-block;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--oc-color-muted);
  margin-right: var(--oc-space-1);
}

.onecalc-home-shell__statusfoot-dot[data-health="live"] {
  background: var(--oc-color-success);
}

.onecalc-home-shell__statusfoot-dot[data-health="stale"] {
  background: var(--oc-color-warning);
}

.onecalc-home-shell__statusfoot-sep {
  color: var(--oc-color-border);
}

.onecalc-home-shell__statusfoot-scenario {
  display: inline-flex;
  gap: 0.25rem;
}

.onecalc-home-shell__statusfoot-scenario-name {
  color: var(--oc-color-ink);
  font-weight: 500;
}

/* Status-foot view-mode toggle button. Always rendered so users
   can opt into Developer view without learning a chord. Two
   visual states driven by `data-view-mode`:
     - user: muted, low-contrast — present but unobtrusive,
     - developer: accent-soft palette, high-contrast — actively
       signalling the workspace is in Developer mode. */
.onecalc-home-shell__statusfoot-mode-button {
  margin-left: auto;
  display: inline-flex;
  align-items: center;
  height: 1rem;
  padding: 0 var(--oc-space-2);
  border-radius: var(--oc-radius-pill);
  font-family: var(--oc-font-ui);
  font-size: 0.65rem;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  cursor: pointer;
  border: 1px solid var(--oc-color-border);
  background: transparent;
  color: var(--oc-color-muted);
  transition: background 120ms ease, color 120ms ease, border-color 120ms ease;
}

.onecalc-home-shell__statusfoot-mode-button:hover {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  border-color: var(--oc-color-card-edge);
}

.onecalc-home-shell__statusfoot-mode-button:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: 1px;
}

.onecalc-home-shell__statusfoot-mode-button[data-view-mode="developer"] {
  background: var(--oc-color-accent-soft);
  color: var(--oc-color-accent);
  border-color: var(--oc-color-card-edge);
}

/* ----- Array result browser (WS-14 §3 item 6) -----------------------------
   The result hero renders array results as a scrollable, resizable grid.
   Outer container sets the resize handle; inner scroll container hosts the
   CSS grid. Headers are sticky (top + left) so addresses stay visible as
   the user scrolls through large arrays. */
.onecalc-array-browser {
  display: flex;
  flex-direction: column;
  gap: var(--oc-space-2);
  width: 100%;
  align-items: stretch;
}

.onecalc-array-browser__caption {
  font-family: var(--oc-font-mono);
  font-size: 0.85rem;
  color: var(--oc-color-muted);
}

.onecalc-array-browser__scroll {
  display: grid;
  /* grid-template-columns is set inline by the renderer (depends on
     preview-col count). */
  align-content: start;
  /* User-resizable + scrollable browser. The eye stays on the formula
     while the user inspects array results; the panel grows / shrinks
     to match how much they want to see. */
  resize: both;
  overflow: auto;
  min-height: 8rem;
  min-width: 12rem;
  max-height: 60vh;
  max-width: 100%;
  background: var(--oc-color-surface);
  border: 1px solid var(--oc-color-card-edge);
  border-radius: var(--oc-radius-panel);
  font-family: var(--oc-font-mono);
  font-size: 0.85rem;
  /* Default browser height: ~12rem so a small-enough array doesn't
     dominate the result hero, but large arrays that need browsing get
     enough vertical real estate. */
  height: 14rem;
}

.onecalc-array-browser__header {
  position: sticky;
  background: var(--oc-color-panel);
  color: var(--oc-color-muted);
  font-size: 0.72rem;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  padding: 4px 6px;
  border-bottom: 1px solid var(--oc-color-card-edge);
  z-index: 1;
  text-align: center;
  user-select: none;
}

.onecalc-array-browser__column-header {
  top: 0;
  /* Vertical separator between column headers — matches the
     `border-right` on cells so a column boundary draws as a
     single continuous line from header through every cell. */
  border-right: 1px solid var(--oc-color-card-edge);
  cursor: pointer;
}

.onecalc-array-browser__row-header {
  left: 0;
  border-right: 1px solid var(--oc-color-card-edge);
  border-bottom: 1px solid var(--oc-color-border);
  cursor: pointer;
}

.onecalc-array-browser__corner {
  top: 0;
  left: 0;
  z-index: 2;
  border-right: 1px solid var(--oc-color-card-edge);
  border-bottom: 1px solid var(--oc-color-border);
  cursor: pointer;
}

.onecalc-array-browser__column-header:hover,
.onecalc-array-browser__row-header:hover,
.onecalc-array-browser__corner:hover {
  background: var(--oc-color-accent-soft);
}

.onecalc-array-browser__cell {
  display: flex;
  align-items: center;
  box-sizing: border-box;
  min-height: 1.6rem;
  line-height: 1.25;
  padding: 4px 8px;
  /* Excel-style grid: each cell paints its bottom + right edge.
     The container's outer border supplies the very top + left
     edges. With this shape the grid renders correctly whether
     row/column headers are visible or not — without `border-right`
     the headers-off case collapsed into horizontal stripes with
     no visible column separation. */
  border-bottom: 1px solid var(--oc-color-border);
  border-right: 1px solid var(--oc-color-border);
  /* Long strings wrap inside the cell up to a max-width — prevents a
     single 10k-char cell from blowing out the grid layout. */
  white-space: pre-wrap;
  word-break: break-word;
  max-width: 24rem;
  color: var(--oc-color-ink);
  overflow: hidden;
  text-overflow: ellipsis;
}

.onecalc-array-browser__cell--empty {
  background: rgba(36, 93, 90, 0.02);
}

/* Block selection (Excel-style rectangular selection). Cells in
   the selection get a translucent accent fill + a subtle border
   on the outer edge of the rectangle. The cell's own background
   (CF fill etc.) shows through via opacity. */
.onecalc-array-browser__cell--selected {
  background: rgba(91, 138, 196, 0.18);
  outline: 1px solid rgba(91, 138, 196, 0.55);
  outline-offset: -1px;
}

/* Toolbar above the grid — zoom buttons, display toggles,
   selection summary, copy. */
.onecalc-array-browser__toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--oc-color-border);
  background: var(--oc-color-bg);
  font-family: var(--oc-font-ui);
  font-size: 0.78rem;
}

.onecalc-array-browser__toolbar-group {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.onecalc-array-browser__toolbar-group--info {
  margin-left: auto;
  gap: 8px;
}

.onecalc-array-browser__toolbar-button {
  padding: 4px 10px;
  font: inherit;
  border: 1px solid var(--oc-color-border);
  border-radius: 4px;
  background: var(--oc-color-surface);
  color: var(--oc-color-ink);
  cursor: pointer;
}

.onecalc-array-browser__toolbar-button:hover {
  background: var(--oc-color-accent-soft);
}

.onecalc-array-browser__toolbar-button[data-active="true"] {
  background: var(--oc-color-accent-soft);
  border-color: var(--oc-color-accent);
  color: var(--oc-color-accent);
}

.onecalc-array-browser__toolbar-button:focus-visible {
  outline: 2px solid var(--oc-color-accent);
  outline-offset: 2px;
}

.onecalc-array-browser__toolbar-zoom-label {
  display: inline-block;
  min-width: 3.5rem;
  text-align: center;
  font-family: var(--oc-font-mono);
  color: var(--oc-color-muted);
}

.onecalc-array-browser__toolbar-info {
  font-family: var(--oc-font-mono);
  color: var(--oc-color-muted);
  font-size: 0.72rem;
}

/* Resize handles — small drag affordances on the trailing edge
   of column / row headers. Excel shows a thin separator that
   thickens on hover; we mirror with a 4px-wide hit zone that
   only paints (border-color) on hover. */
.onecalc-array-browser__resize-handle {
  position: absolute;
  background: transparent;
  cursor: col-resize;
  user-select: none;
  z-index: 3;
}

.onecalc-array-browser__resize-handle--col {
  top: 0;
  right: 0;
  width: 4px;
  height: 100%;
}

.onecalc-array-browser__resize-handle--row {
  left: 0;
  bottom: 0;
  width: 100%;
  height: 4px;
  cursor: row-resize;
}

.onecalc-array-browser__resize-handle:hover {
  background: var(--oc-color-accent);
  opacity: 0.4;
}

/* Anchor the resize handle inside the header cell. */
.onecalc-array-browser__column-header,
.onecalc-array-browser__row-header {
  position: relative;
}

/* Display options — reactive to the toolbar toggles. The grid
   lines toggle hides cell borders; alternating rows tints odd
   rows (every cell on a row carries `data-row-parity="even|odd"`
   so the rule scopes per-row regardless of how many columns the
   array has). Headers-off is handled by the renderer skipping
   the header DOM + the leading row-number track entirely — no
   CSS shim needed. */
.onecalc-array-browser[data-show-grid-lines="false"] .onecalc-array-browser__cell {
  border-bottom: none;
  border-right: none;
}

.onecalc-array-browser[data-show-alternating-rows="true"]
  .onecalc-array-browser__cell[data-row-parity="odd"]:not(.onecalc-array-browser__cell--selected) {
  background-color: rgba(36, 93, 90, 0.05);
}

/* Per-cell CF carrier (W071 + W072): cells become positioned
   containers so the data-bar overlay can stack underneath the
   value text. The cell's font / fill colour comes from inline
   style; this rule only sets up positioning + value layout. */
.onecalc-array-browser__cell--cf {
  position: relative;
  align-items: center;
  gap: 4px;
}

.onecalc-array-browser__cell-value {
  position: relative;
  z-index: 1;
}

.onecalc-array-browser__cell-value--hidden {
  visibility: hidden;
}

/* Data bar (W072): positioned absolutely behind the cell value,
   sized via inline `width: N%` from `fill_ratio`. Direction
   `right` means the bar grows from the right edge (used for
   negative axes); direction `left` (default) grows from the
   left edge. Colour comes from inline style. */
.onecalc-array-browser__data-bar {
  position: absolute;
  top: 4px;
  bottom: 4px;
  left: 4px;
  z-index: 0;
  border-radius: 2px;
  opacity: 0.5;
  pointer-events: none;
}

.onecalc-array-browser__data-bar[data-direction="right"] {
  left: auto;
  right: 4px;
}

/* Icon glyph (W072 icon-set rule). Sits ahead of the value
   in flex order. */
.onecalc-array-browser__icon {
  position: relative;
  z-index: 1;
  font-size: 0.95rem;
  line-height: 1;
  flex-shrink: 0;
}

.onecalc-array-browser__truncation {
  align-self: flex-start;
  font-family: var(--oc-font-ui);
  font-size: 0.72rem;
  color: var(--oc-color-muted);
  padding: 2px 8px;
  border: 1px dashed var(--oc-color-border);
  border-radius: 999px;
  background: var(--oc-color-panel);
}
"#;

#[component]
pub fn ThemeStyleTag() -> impl IntoView {
    view! { <style data-theme="onecalc-theme">{ONECALC_THEME_CSS}</style> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_style_tag_renders_css_tokens() {
        let html = view! { <ThemeStyleTag /> }.to_html();
        assert!(html.contains("--oc-color-bg"));
        assert!(html.contains("onecalc-shell-frame"));
    }
}
