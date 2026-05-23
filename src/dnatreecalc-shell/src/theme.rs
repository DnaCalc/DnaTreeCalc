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
"#;
