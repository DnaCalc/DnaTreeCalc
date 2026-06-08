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

.dtc-workspace-control {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
}

.dtc-workspace-control__select {
    min-width: 10rem;
    max-width: 16rem;
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    background: #ffffff;
    padding: 0.25rem 0.5rem;
    color: inherit;
    font: inherit;
}

.dtc-workspace-control__new {
    border: 1px solid #cbd5e1;
    border-radius: 4px;
    background: #ffffff;
    padding: 0.25rem 0.625rem;
    color: inherit;
    font: inherit;
    cursor: pointer;
}

.dtc-workspace-control__new:hover {
    background: #eef2f7;
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

.dtc-outline-table__content-input {
    width: 100%;
    min-width: 10rem;
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    padding: 0.1875rem 0.25rem;
    font: inherit;
    color: inherit;
}

.dtc-outline-table__content-input:hover {
    border-color: #cbd5e1;
    background: #ffffff;
}

.dtc-outline-table__content-input:focus {
    outline: 2px solid #93c5fd;
    outline-offset: 1px;
    border-color: #2563eb;
    background: #ffffff;
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

.dtc-table-card {
    margin-top: 0.75rem;
    display: grid;
    gap: 0.5rem;
}

.dtc-table-card__grid {
    display: grid;
    gap: 1px;
    overflow-x: auto;
    border: 1px solid #d8dee8;
    border-radius: 6px;
    background: #d8dee8;
}

.dtc-table-card__row {
    display: grid;
    grid-template-columns: repeat(var(--dtc-table-cols, 3), minmax(5rem, 1fr));
    gap: 1px;
}

.dtc-table-card__header-cell,
.dtc-table-card__formula-cell,
.dtc-table-card__cell-input,
.dtc-table-card__row-action {
    min-width: 0;
    min-height: 2rem;
    border: 0;
    border-radius: 0;
    padding: 0.375rem 0.5rem;
    background: #ffffff;
    font: 0.8125rem ui-monospace, SFMono-Regular, Menlo, monospace;
    color: #1f2937;
}

.dtc-table-card__row-action {
    background: #fff7ed;
    color: #9a3412;
    font-weight: 700;
    cursor: pointer;
}

.dtc-table-card__row-action:hover {
    background: #ffedd5;
}

.dtc-table-card__row-action:disabled {
    cursor: default;
    color: #94a3b8;
    background: #f8fafc;
}

.dtc-table-card__header-cell {
    background: #f3f6f9;
    font-weight: 700;
    color: #475569;
}

.dtc-table-card__row--totals .dtc-table-card__formula-cell {
    background: #f8fafc;
    font-weight: 700;
}

.dtc-table-card__cell-input:focus {
    outline: 2px solid #2563eb;
    outline-offset: -2px;
}

.dtc-table-card__add-row,
.dtc-table-card__add-column,
.dtc-table-card__delete-column {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
    align-items: center;
}

.dtc-table-card__add-row input,
.dtc-table-card__add-column input,
.dtc-table-card__delete-column select {
    flex: 1 1 7rem;
    min-width: 0;
    border: 1px solid #d8dee8;
    border-radius: 4px;
    padding: 0.375rem 0.5rem;
    font: 0.8125rem ui-monospace, SFMono-Regular, Menlo, monospace;
}

.dtc-table-card__add-row button,
.dtc-table-card__add-column button,
.dtc-table-card__delete-column button {
    border: 1px solid #bfdbfe;
    border-radius: 4px;
    padding: 0.375rem 0.625rem;
    background: #eff6ff;
    color: #1d4ed8;
    font-weight: 700;
}

.dtc-table-card__delete-column button {
    border-color: #fed7aa;
    background: #fff7ed;
    color: #9a3412;
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

/* UX polish pass: denser, clearer, keyboard-friendly chrome and skins. */
.dtc-shell {
    grid-template-rows: auto auto 1fr auto;
    color: #18212f;
    background: #eef3f8;
}

.dtc-shell:focus {
    outline: none;
}

.dtc-context-strip {
    gap: 0.75rem;
    padding: 0.625rem 0.875rem;
    border-bottom-color: #ccd6e2;
    background: #fbfcfe;
}

.dtc-brand-block {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    min-width: 0;
}

.dtc-context-strip__title {
    max-width: 18rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

.dtc-context-strip__profile {
    border: 1px solid #d6dde7;
    border-radius: 6px;
    background: #f4f7fb;
    color: #4b647c;
    letter-spacing: 0;
}

.dtc-workspace-control__select {
    border-color: #bdc9d8;
}

.dtc-workspace-control__new,
.dtc-recalc-mode button,
.dtc-skin-switcher__tab {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
}

.dtc-workspace-control__new {
    border-color: #7aa37d;
    background: #f4fbf4;
    color: #245c2a;
}

.dtc-workspace-control__new:hover {
    background: #e8f5e8;
}

.dtc-skin-switcher {
    gap: 0.1875rem;
    padding: 0.1875rem;
    border: 1px solid #d6dde7;
    border-radius: 8px;
    background: #f4f7fb;
}

.dtc-skin-switcher__tab {
    padding: 0.3125rem 0.625rem;
}

.dtc-skin-switcher__tab:hover {
    background: #ffffff;
}

.dtc-skin-switcher__tab--active {
    border-color: #1f4f83;
    background: #245f9c;
    box-shadow: 0 1px 2px rgba(15, 23, 42, 0.16);
}

.dtc-shortcut-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-height: 2rem;
    padding: 0.3125rem 0.875rem;
    border-bottom: 1px solid #dce3ec;
    background: #f5f8fc;
    color: #52677d;
    font-size: 0.75rem;
}

.dtc-shortcut-hint {
    display: inline-flex;
    align-items: center;
    gap: 0.3125rem;
}

.dtc-shell kbd {
    display: inline-flex;
    align-items: center;
    min-height: 1.125rem;
    padding: 0 0.3125rem;
    border: 1px solid #c4cfdc;
    border-bottom-color: #aab7c7;
    border-radius: 4px;
    background: #ffffff;
    color: #33485f;
    font: 0.6875rem ui-monospace, SFMono-Regular, Menlo, monospace;
    white-space: nowrap;
}

.dtc-main-slot {
    background: #f9fbfd;
    border-bottom-color: #ccd6e2;
}

.dtc-status-foot {
    border-top: 1px solid #d6dde7;
    background: #f7f9fc;
}

.dtc-status-pill {
    padding: 0.125rem 0.5rem;
    border: 1px solid #b9d5bb;
    border-radius: 999px;
    background: #eef8ef;
    color: #2f6b35;
}

.dtc-recalc-mode__button--active {
    border-color: #245f9c;
    background: #e7f0f9 !important;
    color: #245f9c;
}

.dtc-recalc-mode__calculate--pending {
    border-color: #b7791f !important;
    background: #fff7e6 !important;
    color: #8a5a12 !important;
}

.dtc-triple-editor {
    grid-template-columns: minmax(240px, 300px) minmax(300px, 1fr) minmax(300px, 1fr);
}

.dtc-triple-editor__nav,
.dtc-formula-tree__nav,
.dtc-dependency-inspector__list {
    border-right-color: #dce3ec;
    background: #f6f9fc;
}

.dtc-triple-editor__editor,
.dtc-formula-tree__workbench {
    background: #ffffff;
}

.dtc-triple-editor__value {
    background: #fbfcfe;
}

.dtc-tree-row {
    width: 100%;
    border: 1px solid transparent;
    background: transparent;
    padding: 0.1875rem 0.375rem;
    color: inherit;
    text-align: left;
}

.dtc-tree-row:hover,
.dtc-node-management button:hover,
.dtc-formula-tree__commands button:hover,
.dtc-dependency-row:hover {
    background: #eef4fa;
}

.dtc-tree-row--selected,
.dtc-dependency-row--selected,
.dtc-outline-table tbody tr.dtc-outline-row--selected {
    border-color: #8bb7dd;
    background: #e5f1fb;
    color: #20537f;
}

.dtc-tree-row:focus-visible,
.dtc-skin-switcher__tab:focus-visible,
.dtc-workspace-control__new:focus-visible,
.dtc-recalc-mode button:focus-visible,
.dtc-node-management button:focus-visible,
.dtc-formula-tree__commands button:focus-visible,
.dtc-outline-table__content-input:focus-visible,
.dtc-dependency-row:focus-visible {
    outline: 2px solid #3c7fb1;
    outline-offset: 2px;
}

.dtc-formula-tree {
    grid-template-columns: minmax(240px, 320px) minmax(340px, 1fr);
}

.dtc-formula-tree__input {
    border-color: #bdc9d8;
    background: #ffffff;
    color: #172033;
    box-shadow: inset 0 1px 2px rgba(15, 23, 42, 0.04);
}

.dtc-node-management,
.dtc-value-card,
.dtc-dependency-card {
    border-color: #d4dde8;
    box-shadow: 0 1px 2px rgba(15, 23, 42, 0.05);
}

.dtc-node-management {
    background: #f7fafc;
}

.dtc-node-management input,
.dtc-node-management button,
.dtc-formula-tree__commands button {
    border-color: #bdc9d8;
}

.dtc-outline-table {
    border-collapse: separate;
    border-spacing: 0;
    background: #ffffff;
}

.dtc-outline-table thead th {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 0.5rem 0.625rem;
    border-bottom-color: #cbd7e5;
    background: #eef4fa;
    color: #3d5268;
}

.dtc-outline-table tbody td {
    padding: 0.3125rem 0.625rem;
    border-bottom-color: #edf2f7;
}

.dtc-outline-table tbody tr:hover {
    background: #f3f8fc;
}

.dtc-array-value {
    border-color: #bdc9d8;
    background: #bdc9d8;
}

.dtc-array-value__cell {
    text-align: right;
}

.dtc-value-display {
    overflow-wrap: anywhere;
}

.dtc-value-card,
.dtc-dependency-card {
    background: #ffffff;
}

.dtc-dependency-row {
    color: inherit;
}
"#;
