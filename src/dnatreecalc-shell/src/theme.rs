/// Inline CSS for the walking-skeleton shell + the built-in skins.
///
/// Inlining keeps the WASM bundle self-contained (no external stylesheet
/// fetch) and matches DnaOneCalc's pattern. A real design-token system
/// (CSS variables, theme switching, per-skin overrides) layers in with
/// W006 hardening; the skeleton just needs the layout shape to read
/// honestly.
pub const SHELL_CSS: &str = r#"
.dtc-shell {
    display: grid;
    grid-template-rows: auto 1fr auto;
    height: 100vh;
    width: 100%;
    font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
    color: #1f2933;
    background: #f6f8fb;
}

.dtc-context-strip {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid #d6dde7;
    background: #ffffff;
    font-size: 0.875rem;
}

.dtc-context-strip__title {
    font-weight: 600;
}

.dtc-context-strip__profile {
    padding: 0.125rem 0.5rem;
    background: #eef2f7;
    border-radius: 999px;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
}

.dtc-context-strip__spacer {
    flex: 1;
}

.dtc-skin-switcher {
    display: inline-flex;
    gap: 0.25rem;
}

.dtc-skin-switcher__tab {
    appearance: none;
    border: 1px solid transparent;
    background: transparent;
    padding: 0.25rem 0.75rem;
    border-radius: 6px;
    font: inherit;
    cursor: pointer;
    color: inherit;
}

.dtc-skin-switcher__tab:hover {
    background: #eef2f7;
}

.dtc-skin-switcher__tab--active {
    background: #2563eb;
    color: #ffffff;
    border-color: #1d4ed8;
}

.dtc-main-slot {
    overflow: auto;
    background: #ffffff;
    border-bottom: 1px solid #d6dde7;
}

.dtc-status-foot {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.375rem 0.75rem;
    background: #f1f3f7;
    font-size: 0.75rem;
    color: #475569;
}

.dtc-recalc-mode {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
}

.dtc-recalc-mode button {
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    background: #ffffff;
    padding: 0.1875rem 0.5rem;
    color: inherit;
    font: inherit;
    cursor: pointer;
}

.dtc-recalc-mode button:hover {
    background: #eef2f7;
}

.dtc-recalc-mode__button--active {
    border-color: #2563eb;
    background: #dbeafe !important;
    color: #1d4ed8;
}

.dtc-recalc-mode__calculate--pending {
    border-color: #d97706 !important;
    background: #fffbeb !important;
    color: #92400e !important;
}

/* triple-editor minimal layout */
.dtc-triple-editor {
    display: grid;
    grid-template-columns: 260px 1fr 1fr;
    height: 100%;
}

.dtc-triple-editor__nav,
.dtc-triple-editor__editor,
.dtc-triple-editor__value {
    padding: 0.75rem;
    overflow: auto;
}

.dtc-triple-editor__nav {
    border-right: 1px solid #e2e8ef;
    background: #fafbfc;
}

.dtc-triple-editor__editor {
    border-right: 1px solid #e2e8ef;
}

.dtc-tree-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.125rem 0.25rem;
    border-radius: 4px;
    cursor: pointer;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.8125rem;
}

.dtc-tree-row:hover {
    background: #eef2f7;
}

.dtc-tree-row--selected {
    background: #dbeafe;
    color: #1d4ed8;
}

.dtc-tree-row--meta {
    opacity: 0.6;
    font-style: italic;
}

.dtc-formula-display {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    background: #f8fafc;
    padding: 0.5rem;
    border: 1px solid #e2e8ef;
    border-radius: 4px;
    min-height: 2.5rem;
    white-space: pre-wrap;
}

.dtc-triple-editor__input {
    min-height: 10rem;
    margin-bottom: 0.5rem;
}

.dtc-value-display {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.9rem;
}

.dtc-value-display--error {
    color: #b91c1c;
}

.dtc-section-label {
    font-size: 0.6875rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: #64748b;
    margin-bottom: 0.375rem;
}

.dtc-node-management {
    display: grid;
    gap: 0.5rem;
    padding: 0.625rem;
    margin-bottom: 0.75rem;
    border: 1px solid #e2e8ef;
    border-radius: 6px;
    background: #f8fafc;
}

.dtc-node-management__row,
.dtc-node-management__commands {
    display: flex;
    gap: 0.375rem;
    min-width: 0;
}

.dtc-node-management input {
    min-width: 0;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    background: #ffffff;
    padding: 0.375rem 0.5rem;
    font: 0.8125rem ui-monospace, SFMono-Regular, Menlo, monospace;
}

.dtc-node-management__symbol {
    width: 9rem;
}

.dtc-node-management__content {
    flex: 1;
}

.dtc-node-management button {
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    background: #ffffff;
    padding: 0.375rem 0.5rem;
    font: 0.8125rem ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
    cursor: pointer;
    white-space: nowrap;
}

.dtc-node-management button:hover {
    background: #eef2f7;
}

/* outline-table minimal layout */
.dtc-outline-table {
    width: 100%;
    border-collapse: collapse;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.8125rem;
}

.dtc-outline-table thead th {
    text-align: left;
    padding: 0.375rem 0.5rem;
    background: #f1f5f9;
    border-bottom: 1px solid #d6dde7;
    font-weight: 600;
    color: #475569;
}

.dtc-outline-table tbody td {
    padding: 0.25rem 0.5rem;
    border-bottom: 1px solid #f1f5f9;
    vertical-align: top;
}

.dtc-outline-table tbody tr {
    cursor: pointer;
}

.dtc-outline-table tbody tr:hover {
    background: #eef2f7;
}

.dtc-outline-table tbody tr.dtc-outline-row--selected {
    background: #dbeafe;
}

.dtc-formula-tree {
    display: grid;
    grid-template-columns: minmax(220px, 320px) minmax(320px, 1fr);
    min-height: 100%;
}

.dtc-formula-tree__nav {
    padding: 0.75rem;
    overflow: auto;
    border-right: 1px solid #e2e8ef;
    background: #fafbfc;
}

.dtc-formula-tree__workbench {
    display: grid;
    grid-template-rows: auto auto minmax(8rem, auto) auto auto 1fr;
    gap: 0.5rem;
    padding: 0.875rem;
    min-width: 0;
}

.dtc-formula-tree__input {
    width: 100%;
    min-height: 8rem;
    resize: vertical;
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    padding: 0.625rem;
    font: 0.875rem ui-monospace, SFMono-Regular, Menlo, monospace;
}

.dtc-formula-tree__commands {
    display: flex;
    gap: 0.5rem;
}

.dtc-formula-tree__commands button {
    border: 1px solid #cbd5e1;
    border-radius: 6px;
    background: #ffffff;
    padding: 0.375rem 0.75rem;
    font: inherit;
    cursor: pointer;
}

.dtc-formula-tree__commands button:hover {
    background: #eef2f7;
}

.dtc-array-value {
    display: inline-grid;
    gap: 1px;
    border: 1px solid #cbd5e1;
    background: #cbd5e1;
    max-width: 100%;
    overflow: auto;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.dtc-array-value-shell {
    display: grid;
    gap: 0.5rem;
    max-width: 100%;
}

.dtc-array-value-shell__summary {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem;
    font: 0.75rem ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
    color: #475569;
}

.dtc-array-value-shell__omitted {
    color: #64748b;
}

.dtc-array-value-shell__toggle {
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    background: #ffffff;
    padding: 0.1875rem 0.5rem;
    color: #2563eb;
    font: inherit;
    cursor: pointer;
}

.dtc-array-value-shell__toggle:hover {
    background: #eef2f7;
}

.dtc-array-value__row {
    display: flex;
    gap: 1px;
}

.dtc-array-value__cell {
    min-width: 4rem;
    padding: 0.25rem 0.375rem;
    background: #ffffff;
}

.dtc-value-board {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 0.75rem;
    padding: 0.75rem;
}

.dtc-value-card {
    border: 1px solid #e2e8ef;
    border-radius: 8px;
    padding: 0.75rem;
    background: #ffffff;
    min-width: 0;
}

.dtc-value-card header {
    display: flex;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 0.5rem;
    font-weight: 600;
}

.dtc-value-card code,
.dtc-value-card__formula {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.75rem;
    color: #64748b;
    overflow-wrap: anywhere;
}

.dtc-value-card__formula {
    margin-bottom: 0.5rem;
}

.dtc-table-summary {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.125rem 0.5rem;
    margin: 0.75rem 0 0;
    font-size: 0.75rem;
}

.dtc-table-summary dt {
    color: #64748b;
}

.dtc-table-summary dd {
    margin: 0;
}

.dtc-dependency-inspector {
    display: grid;
    grid-template-columns: minmax(260px, 360px) minmax(320px, 1fr);
    min-height: 100%;
}

.dtc-dependency-inspector__list {
    padding: 0.75rem;
    overflow: auto;
    border-right: 1px solid #e2e8ef;
    background: #fafbfc;
}

.dtc-dependency-inspector__detail {
    padding: 0.875rem;
    overflow: auto;
}

.dtc-dependency-row {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 0.5rem;
    width: 100%;
    border: 0;
    border-radius: 4px;
    background: transparent;
    padding: 0.25rem 0.375rem;
    text-align: left;
    font: 0.8125rem ui-monospace, SFMono-Regular, Menlo, monospace;
    cursor: pointer;
}

.dtc-dependency-row:hover {
    background: #eef2f7;
}

.dtc-dependency-row--selected {
    background: #dbeafe;
    color: #1d4ed8;
}

.dtc-dependency-card {
    border: 1px solid #e2e8ef;
    border-radius: 8px;
    padding: 0.75rem;
    margin-bottom: 0.625rem;
    background: #ffffff;
}

.dtc-dependency-card code {
    display: block;
    margin-top: 0.375rem;
    color: #475569;
    overflow-wrap: anywhere;
}

.dtc-dependency-card__collection {
    display: flex;
    gap: 0.75rem;
    margin-top: 0.5rem;
    color: #64748b;
    font-size: 0.75rem;
}

.dtc-empty-detail {
    color: #64748b;
}
"#;
