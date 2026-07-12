/// Inline CSS for the walking-skeleton shell + the built-in skins.
///
/// Inlining keeps the WASM bundle self-contained (no external stylesheet
/// fetch) and matches dnacalc-bench-host's pattern. The active `ThemeTokens`
/// object injects CSS custom properties into `.dtc-shell`; this stylesheet
/// consumes those variables so skins can share one presentation token surface.
pub const SHELL_CSS: &str = r#"
.dtc-shell {
    display: grid;
    grid-template-rows: auto 1fr auto;
    height: 100vh;
    width: 100%;
    font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
    color: var(--dtc-text);
    background: var(--dtc-surface-subtle);
}

.dtc-context-strip {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--dtc-border);
    background: var(--dtc-surface);
    font-size: 0.875rem;
}

.dtc-context-strip__title {
    font-weight: 600;
}

.dtc-context-strip__profile {
    padding: 0.125rem 0.5rem;
    background: var(--dtc-surface-muted);
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
    border: 1px solid var(--dtc-border-strong);
    border-radius: 4px;
    background: var(--dtc-surface);
    padding: 0.25rem 0.5rem;
    color: inherit;
    font: inherit;
}

.dtc-workspace-control__new {
    border: 1px solid var(--dtc-border-strong);
    border-radius: 4px;
    background: var(--dtc-surface);
    padding: 0.25rem 0.625rem;
    color: inherit;
    font: inherit;
    cursor: pointer;
}

.dtc-workspace-control__new:hover {
    background: var(--dtc-surface-muted);
}

.dtc-workspace-control__rename {
    min-width: 10rem;
    max-width: 16rem;
    border: 1px solid var(--dtc-border-strong);
    border-radius: 4px;
    background: var(--dtc-surface);
    padding: 0.25rem 0.5rem;
    color: inherit;
    font: inherit;
}

.dtc-workspace-control__rename-button {
    border: 1px solid var(--dtc-border-strong);
    border-radius: 4px;
    background: var(--dtc-surface);
    padding: 0.25rem 0.625rem;
    color: inherit;
    font: inherit;
    cursor: pointer;
}

.dtc-workspace-control__rename-button:hover {
    background: var(--dtc-surface-muted);
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
    background: var(--dtc-surface-muted);
}

.dtc-skin-switcher__tab--active {
    background: var(--dtc-accent);
    color: var(--dtc-surface);
    border-color: var(--dtc-accent-text);
}

.dtc-main-slot {
    overflow: auto;
    background: var(--dtc-surface);
    border-bottom: 1px solid var(--dtc-border);
}

.dtc-status-foot {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.375rem 0.75rem;
    background: var(--dtc-surface-muted);
    font-size: 0.75rem;
    color: var(--dtc-text-muted);
}

.dtc-recalc-mode {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
}

.dtc-recalc-mode button {
    border: 1px solid var(--dtc-border-strong);
    border-radius: 4px;
    background: var(--dtc-surface);
    padding: 0.1875rem 0.5rem;
    color: inherit;
    font: inherit;
    cursor: pointer;
}

.dtc-recalc-mode button:hover {
    background: var(--dtc-surface-muted);
}

.dtc-recalc-mode__button--active {
    border-color: var(--dtc-accent);
    background: var(--dtc-accent-surface) !important;
    color: var(--dtc-accent-text);
}

.dtc-recalc-mode__calculate--pending {
    border-color: var(--dtc-warning) !important;
    background: var(--dtc-warning-surface) !important;
    color: var(--dtc-warning-text) !important;
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
    border-right: 1px solid var(--dtc-border-muted);
    background: var(--dtc-surface-panel);
}

.dtc-triple-editor__editor {
    border-right: 1px solid var(--dtc-border-muted);
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
    background: var(--dtc-surface-muted);
}

.dtc-tree-row--selected {
    background: var(--dtc-accent-surface);
    color: var(--dtc-accent-text);
}

.dtc-tree-row--meta {
    opacity: 0.6;
    font-style: italic;
}

.dtc-formula-display {
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    background: var(--dtc-surface-subtle);
    padding: 0.5rem;
    border: 1px solid var(--dtc-border-muted);
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
    color: var(--dtc-danger);
}

.dtc-section-label {
    font-size: 0.6875rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--dtc-text-subtle);
    margin-bottom: 0.375rem;
}

.dtc-node-management {
    display: grid;
    gap: 0.5rem;
    padding: 0.625rem;
    margin-bottom: 0.75rem;
    border: 1px solid var(--dtc-border-muted);
    border-radius: 6px;
    background: var(--dtc-surface-subtle);
}

.dtc-node-management__row,
.dtc-node-management__commands {
    display: flex;
    gap: 0.375rem;
    min-width: 0;
}

.dtc-node-management__hints {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
    color: var(--dtc-text-subtle);
    font-size: 0.75rem;
}

.dtc-node-management__hints span {
    display: inline-flex;
    gap: 0.25rem;
    align-items: center;
}

.dtc-node-management__hints kbd {
    border: 1px solid var(--dtc-border-muted);
    border-radius: 3px;
    padding: 0 0.25rem;
    background: var(--dtc-surface);
    color: var(--dtc-text);
    font: inherit;
}

.dtc-command-hint--disabled {
    opacity: 0.45;
}

.dtc-node-management input,
.dtc-node-management select {
    min-width: 0;
    border: 1px solid var(--dtc-border-strong);
    border-radius: 4px;
    background: var(--dtc-surface);
    padding: 0.375rem 0.5rem;
    font: 0.8125rem ui-monospace, SFMono-Regular, Menlo, monospace;
}

.dtc-node-management__symbol {
    width: 9rem;
}

.dtc-node-management__content {
    flex: 1;
}

.dtc-node-management__template {
    flex: 1;
}

.dtc-node-management button {
    border: 1px solid var(--dtc-border-strong);
    border-radius: 4px;
    background: var(--dtc-surface);
    padding: 0.375rem 0.5rem;
    font: 0.8125rem ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
    cursor: pointer;
    white-space: nowrap;
}

.dtc-node-management button:hover {
    background: var(--dtc-surface-muted);
}

.dtc-node-management button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
    background: var(--dtc-surface);
}

.dtc-node-management__impact {
    margin: 0;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.75rem;
    line-height: 1.35;
}

.dtc-node-management__impact--blocked {
    list-style: none;
    color: var(--dtc-danger-text);
    background: var(--dtc-danger-surface);
    border: 1px solid var(--dtc-danger-border);
}

.dtc-node-management__impact--warn {
    color: var(--dtc-warning-text);
    background: var(--dtc-warning-surface);
    border: 1px solid var(--dtc-warning);
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
    background: var(--dtc-surface-muted);
    border-bottom: 1px solid var(--dtc-border);
    font-weight: 600;
    color: var(--dtc-text-muted);
}

.dtc-outline-table tbody td {
    padding: 0.25rem 0.5rem;
    border-bottom: 1px solid var(--dtc-surface-muted);
    vertical-align: top;
}

.dtc-outline-table tbody tr {
    cursor: pointer;
}

.dtc-outline-table tbody tr:hover {
    background: var(--dtc-surface-muted);
}

.dtc-outline-table tbody tr.dtc-outline-row--selected {
    background: var(--dtc-accent-surface);
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
    border-color: var(--dtc-border-strong);
    background: var(--dtc-surface);
}

.dtc-outline-table__content-input:focus {
    outline: 2px solid var(--dtc-focus);
    outline-offset: 1px;
    border-color: var(--dtc-accent);
    background: var(--dtc-surface);
}

.dtc-formula-tree {
    display: grid;
    grid-template-columns: minmax(220px, 320px) minmax(320px, 1fr);
    min-height: 100%;
}

.dtc-formula-tree__nav {
    padding: 0.75rem;
    overflow: auto;
    border-right: 1px solid var(--dtc-border-muted);
    background: var(--dtc-surface-panel);
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
    border: 1px solid var(--dtc-border-strong);
    border-radius: 6px;
    padding: 0.625rem;
    font: 0.875rem ui-monospace, SFMono-Regular, Menlo, monospace;
}

.dtc-formula-tree__commands {
    display: flex;
    gap: 0.5rem;
}

.dtc-formula-tree__commands button {
    border: 1px solid var(--dtc-border-strong);
    border-radius: 6px;
    background: var(--dtc-surface);
    padding: 0.375rem 0.75rem;
    font: inherit;
    cursor: pointer;
}

.dtc-formula-tree__commands button:hover {
    background: var(--dtc-surface-muted);
}

.dtc-array-value {
    display: inline-grid;
    gap: 1px;
    border: 1px solid var(--dtc-border-strong);
    background: var(--dtc-border-strong);
    max-width: 100%;
    min-width: 0;
    max-height: 18rem;
    overflow: auto;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
}

.dtc-array-value-shell {
    display: grid;
    gap: 0.5rem;
    max-width: 100%;
    min-width: 0;
}

.dtc-array-value-shell__summary {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 0.5rem;
    font: 0.75rem ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
    color: var(--dtc-text-muted);
}

.dtc-array-value-shell__omitted {
    color: var(--dtc-text-subtle);
}

.dtc-array-value-shell__toggle {
    border: 1px solid var(--dtc-border-strong);
    border-radius: 4px;
    background: var(--dtc-surface);
    padding: 0.1875rem 0.5rem;
    color: var(--dtc-accent);
    font: inherit;
    cursor: pointer;
}

.dtc-array-value-shell__toggle:hover {
    background: var(--dtc-surface-muted);
}

.dtc-array-value__row {
    display: flex;
    gap: 1px;
}

.dtc-array-value__cell {
    min-width: 4rem;
    padding: 0.25rem 0.375rem;
    background: var(--dtc-surface);
}

.dtc-value-board {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 0.75rem;
    padding: 0.75rem;
}

.dtc-value-board__active-selection {
    grid-column: 1 / -1;
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem 0.75rem;
    align-items: center;
    margin: 0;
    padding: 0.625rem 0.75rem;
    border: 1px solid var(--dtc-info-border);
    border-radius: 6px;
    background: var(--dtc-info-surface);
    font-size: 0.8125rem;
}

.dtc-value-board__active-selection dt {
    color: var(--dtc-text-muted);
}

.dtc-value-board__active-selection dd {
    margin: 0;
    font-weight: 700;
    color: var(--dtc-info-text);
}

.dtc-value-board__active-detail {
    grid-column: 1 / -1;
    padding: 0.75rem;
    border: 1px solid var(--dtc-border);
    border-radius: 6px;
    background: var(--dtc-surface);
}

.dtc-value-board__active-detail dl {
    display: grid;
    grid-template-columns: max-content minmax(0, 1fr) max-content minmax(0, 1fr);
    gap: 0.375rem 0.75rem;
    margin: 0;
    font-size: 0.8125rem;
}

.dtc-value-board__active-detail dt {
    color: var(--dtc-text-subtle);
}

.dtc-value-board__active-detail dd {
    min-width: 0;
    margin: 0;
    color: var(--dtc-text);
    font-weight: 600;
    overflow-wrap: anywhere;
}

@media (max-width: 700px) {
    .dtc-value-board__active-detail dl {
        grid-template-columns: max-content minmax(0, 1fr);
    }
}

.dtc-value-card {
    border: 1px solid var(--dtc-border-muted);
    border-radius: 8px;
    padding: 0.75rem;
    background: var(--dtc-surface);
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
    color: var(--dtc-text-subtle);
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
    color: var(--dtc-text-subtle);
}

.dtc-table-summary dd {
    margin: 0;
}

.dtc-table-card__active-cell {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem 0.75rem;
    align-items: center;
    margin: 0;
    padding: 0.5rem 0.625rem;
    border: 1px solid var(--dtc-accent-border);
    border-radius: 6px;
    background: var(--dtc-accent-surface-muted);
    font-size: 0.75rem;
}

.dtc-table-card__active-cell dt {
    color: var(--dtc-text-muted);
}

.dtc-table-card__active-cell dd {
    margin: 0;
    font-weight: 700;
    color: var(--dtc-accent-text);
}

.dtc-table-card {
    margin-top: 0.75rem;
    display: grid;
    gap: 0.5rem;
}

.dtc-table-card__header-toggle,
.dtc-table-card__totals-toggle,
.dtc-table-card__table-rename {
    display: inline-flex;
    flex-wrap: wrap;
    gap: 0.375rem;
    align-items: center;
    font-size: 0.75rem;
    color: var(--dtc-text-muted);
}

.dtc-table-card__table-name {
    min-width: 8rem;
}

.dtc-table-card__grid {
    display: grid;
    gap: 1px;
    overflow-x: auto;
    border: 1px solid var(--dtc-border);
    border-radius: 6px;
    background: var(--dtc-border);
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
    background: var(--dtc-surface);
    font: 0.8125rem ui-monospace, SFMono-Regular, Menlo, monospace;
    color: var(--dtc-text);
}

.dtc-table-card__row-action {
    background: var(--dtc-danger-surface);
    color: var(--dtc-danger-text);
    font-weight: 700;
    cursor: pointer;
}

.dtc-table-card__row-action:hover {
    background: var(--dtc-danger-surface);
}

.dtc-table-card__row-action:disabled {
    cursor: default;
    color: var(--dtc-text-subtle);
    background: var(--dtc-surface-subtle);
}

.dtc-table-card__header-cell {
    background: var(--dtc-surface-muted);
    font-weight: 700;
    color: var(--dtc-text-muted);
}

.dtc-table-card__row--totals .dtc-table-card__formula-cell {
    background: var(--dtc-surface-subtle);
    font-weight: 700;
}

.dtc-table-card__cell-input:focus {
    outline: 2px solid var(--dtc-accent);
    outline-offset: -2px;
}

.dtc-table-card__cell--selected {
    box-shadow: inset 0 0 0 2px var(--dtc-accent);
}

.dtc-table-card__add-row,
.dtc-table-card__row-reorder,
.dtc-table-card__add-column,
.dtc-table-card__add-formula-column,
.dtc-table-card__column-reorder,
.dtc-table-card__delete-column {
    display: flex;
    flex-wrap: wrap;
    gap: 0.375rem;
    align-items: center;
}

.dtc-table-card__add-row input,
.dtc-table-card__row-reorder input,
.dtc-table-card__row-reorder select,
.dtc-table-card__add-column input,
.dtc-table-card__add-formula-column input,
.dtc-table-card__column-reorder input,
.dtc-table-card__column-reorder select,
.dtc-table-card__delete-column select {
    flex: 1 1 7rem;
    min-width: 0;
    border: 1px solid var(--dtc-border);
    border-radius: 4px;
    padding: 0.375rem 0.5rem;
    font: 0.8125rem ui-monospace, SFMono-Regular, Menlo, monospace;
}

.dtc-table-card__add-row button,
.dtc-table-card__row-metadata button,
.dtc-table-card__add-column button,
.dtc-table-card__add-formula-column button,
.dtc-table-card__column-metadata button,
.dtc-table-card__totals-formulas button,
.dtc-table-card__delete-column button {
    border: 1px solid var(--dtc-accent-border);
    border-radius: 4px;
    padding: 0.375rem 0.625rem;
    background: var(--dtc-accent-surface-muted);
    color: var(--dtc-accent-text);
    font-weight: 700;
}

.dtc-table-card__delete-column button {
    border-color: var(--dtc-danger-border);
    background: var(--dtc-danger-surface);
    color: var(--dtc-danger-text);
}

.dtc-table-card__totals-toggle {
    display: inline-flex;
    gap: 0.375rem;
    align-items: center;
    width: fit-content;
    font-size: 0.8125rem;
    color: var(--dtc-text-muted);
}

.dtc-table-card__formula-edits {
    display: grid;
    gap: 0.375rem;
}

.dtc-table-card__totals-formulas {
    display: grid;
    gap: 0.375rem;
}

.dtc-table-card__column-metadata {
    display: grid;
    gap: 0.375rem;
}

.dtc-table-card__row-metadata {
    display: grid;
    gap: 0.375rem;
}

.dtc-table-card__row-rename {
    display: grid;
    grid-template-columns: minmax(7rem, 10rem) minmax(10rem, 1fr) auto;
    gap: 0.375rem;
    align-items: center;
    font-size: 0.8125rem;
    color: var(--dtc-text-muted);
}

.dtc-table-card__row-rename input {
    min-width: 0;
    border: 1px solid var(--dtc-border);
    border-radius: 4px;
    padding: 0.375rem 0.5rem;
    font: 0.8125rem ui-monospace, SFMono-Regular, Menlo, monospace;
}

.dtc-table-card__column-rename {
    display: grid;
    grid-template-columns: minmax(5rem, 8rem) minmax(10rem, 1fr) auto;
    gap: 0.375rem;
    align-items: center;
    font-size: 0.8125rem;
    color: var(--dtc-text-muted);
}

.dtc-table-card__column-rename input {
    min-width: 0;
    border: 1px solid var(--dtc-border);
    border-radius: 4px;
    padding: 0.375rem 0.5rem;
    font: 0.8125rem ui-monospace, SFMono-Regular, Menlo, monospace;
}

.dtc-table-card__formula-edit {
    display: grid;
    grid-template-columns: minmax(5rem, 8rem) minmax(12rem, 1fr);
    gap: 0.375rem;
    align-items: center;
    font-size: 0.8125rem;
    color: var(--dtc-text-muted);
}

.dtc-table-card__totals-formula {
    display: grid;
    grid-template-columns: minmax(5rem, 8rem) minmax(12rem, 1fr) auto auto;
    gap: 0.375rem;
    align-items: center;
    font-size: 0.8125rem;
    color: var(--dtc-text-muted);
}

.dtc-dependency-inspector {
    display: grid;
    grid-template-columns: minmax(260px, 360px) minmax(320px, 1fr);
    min-height: 100%;
}

.dtc-dependency-inspector__list {
    padding: 0.75rem;
    overflow: auto;
    border-right: 1px solid var(--dtc-border-muted);
    background: var(--dtc-surface-panel);
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
    background: var(--dtc-surface-muted);
}

.dtc-dependency-row--selected {
    background: var(--dtc-accent-surface);
    color: var(--dtc-accent-text);
}

.dtc-dependency-card {
    border: 1px solid var(--dtc-border-muted);
    border-radius: 8px;
    padding: 0.75rem;
    margin-bottom: 0.625rem;
    background: var(--dtc-surface);
}

.dtc-dependency-card code {
    display: block;
    margin-top: 0.375rem;
    color: var(--dtc-text-muted);
    overflow-wrap: anywhere;
}

.dtc-dependency-card__collection {
    display: flex;
    gap: 0.75rem;
    margin-top: 0.5rem;
    color: var(--dtc-text-subtle);
    font-size: 0.75rem;
}

.dtc-empty-detail {
    color: var(--dtc-text-subtle);
}

/* UX polish pass: denser, clearer, keyboard-friendly chrome and skins. */
.dtc-shell {
    grid-template-rows: auto auto 1fr auto;
    color: var(--dtc-text);
    background: var(--dtc-surface-muted);
}

.dtc-shell:focus {
    outline: none;
}

.dtc-context-strip {
    gap: 0.75rem;
    padding: 0.625rem 0.875rem;
    border-bottom-color: var(--dtc-border);
    background: var(--dtc-surface-panel);
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
    border: 1px solid var(--dtc-border);
    border-radius: 6px;
    background: var(--dtc-surface-subtle);
    color: var(--dtc-text-muted);
    letter-spacing: 0;
}

.dtc-workspace-control__select {
    border-color: var(--dtc-border-strong);
}

.dtc-workspace-control__new,
.dtc-recalc-mode button,
.dtc-skin-switcher__tab {
    display: inline-flex;
    align-items: center;
    gap: 0.375rem;
}

.dtc-workspace-control__new {
    border-color: var(--dtc-success-border);
    background: var(--dtc-success-surface);
    color: var(--dtc-success);
}

.dtc-workspace-control__new:hover {
    background: var(--dtc-success-surface);
}

.dtc-skin-switcher {
    gap: 0.1875rem;
    padding: 0.1875rem;
    border: 1px solid var(--dtc-border);
    border-radius: 8px;
    background: var(--dtc-surface-subtle);
}

.dtc-skin-switcher__tab {
    padding: 0.3125rem 0.625rem;
}

.dtc-skin-switcher__tab:hover {
    background: var(--dtc-surface);
}

.dtc-skin-switcher__tab--active {
    border-color: var(--dtc-accent-hover);
    background: var(--dtc-accent);
    box-shadow: 0 1px 2px var(--dtc-shadow-medium);
}

.dtc-shortcut-bar {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    min-height: 2rem;
    padding: 0.3125rem 0.875rem;
    border-bottom: 1px solid var(--dtc-border);
    background: var(--dtc-surface-subtle);
    color: var(--dtc-text-muted);
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
    border: 1px solid var(--dtc-kbd-border);
    border-bottom-color: var(--dtc-kbd-border-strong);
    border-radius: 4px;
    background: var(--dtc-surface);
    color: var(--dtc-kbd-text);
    font: 0.6875rem ui-monospace, SFMono-Regular, Menlo, monospace;
    white-space: nowrap;
}

.dtc-main-slot {
    background: var(--dtc-surface-panel);
    border-bottom-color: var(--dtc-border);
}

.dtc-status-foot {
    border-top: 1px solid var(--dtc-border);
    background: var(--dtc-surface-subtle);
}

.dtc-status-pill {
    padding: 0.125rem 0.5rem;
    border: 1px solid var(--dtc-success-border);
    border-radius: 999px;
    background: var(--dtc-success-surface);
    color: var(--dtc-success-text);
}

.dtc-status-pill--saved {
    border-color: var(--dtc-success-border);
    background: var(--dtc-success-surface);
    color: var(--dtc-success-text);
}

.dtc-status-pill--error {
    border-color: var(--dtc-danger, currentColor);
    background: transparent;
    color: var(--dtc-danger-text, var(--dtc-danger));
}

.dtc-recalc-mode__button--active {
    border-color: var(--dtc-accent);
    background: var(--dtc-accent-surface) !important;
    color: var(--dtc-accent);
}

.dtc-recalc-mode__calculate--pending {
    border-color: var(--dtc-warning) !important;
    background: var(--dtc-warning-surface) !important;
    color: var(--dtc-warning-text) !important;
}

.dtc-triple-editor {
    grid-template-columns: minmax(240px, 300px) minmax(300px, 1fr) minmax(300px, 1fr);
}

.dtc-triple-editor__nav,
.dtc-formula-tree__nav,
.dtc-dependency-inspector__list {
    border-right-color: var(--dtc-border);
    background: var(--dtc-surface-subtle);
}

.dtc-triple-editor__editor,
.dtc-formula-tree__workbench {
    background: var(--dtc-surface);
}

.dtc-triple-editor__value {
    background: var(--dtc-surface-panel);
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
    background: var(--dtc-accent-surface-muted);
}

.dtc-tree-row--selected,
.dtc-dependency-row--selected,
.dtc-outline-table tbody tr.dtc-outline-row--selected {
    border-color: var(--dtc-accent-border);
    background: var(--dtc-accent-surface);
    color: var(--dtc-accent-text);
}

.dtc-tree-row:focus-visible,
.dtc-skin-switcher__tab:focus-visible,
.dtc-workspace-control__new:focus-visible,
.dtc-recalc-mode button:focus-visible,
.dtc-node-management button:focus-visible,
.dtc-formula-tree__commands button:focus-visible,
.dtc-outline-table__content-input:focus-visible,
.dtc-dependency-row:focus-visible {
    outline: 2px solid var(--dtc-focus);
    outline-offset: 2px;
}

.dtc-formula-tree {
    grid-template-columns: minmax(240px, 320px) minmax(340px, 1fr);
}

.dtc-formula-tree__input {
    border-color: var(--dtc-border-strong);
    background: var(--dtc-surface);
    color: var(--dtc-text);
    box-shadow: inset 0 1px 2px var(--dtc-shadow-inset);
}

.dtc-node-management,
.dtc-value-card,
.dtc-dependency-card {
    border-color: var(--dtc-border);
    box-shadow: 0 1px 2px var(--dtc-shadow-soft);
}

.dtc-node-management {
    background: var(--dtc-surface-subtle);
}

.dtc-node-management input,
.dtc-node-management button,
.dtc-formula-tree__commands button {
    border-color: var(--dtc-border-strong);
}

.dtc-outline-table {
    border-collapse: separate;
    border-spacing: 0;
    background: var(--dtc-surface);
}

.dtc-outline-table thead th {
    position: sticky;
    top: 0;
    z-index: 1;
    padding: 0.5rem 0.625rem;
    border-bottom-color: var(--dtc-border);
    background: var(--dtc-accent-surface-muted);
    color: var(--dtc-text-muted);
}

.dtc-outline-table tbody td {
    padding: 0.3125rem 0.625rem;
    border-bottom-color: var(--dtc-surface-muted);
}

.dtc-outline-table tbody tr:hover {
    background: var(--dtc-surface-muted);
}

.dtc-array-value {
    border-color: var(--dtc-border-strong);
    background: var(--dtc-border-strong);
}

.dtc-array-value__cell {
    text-align: right;
}

.dtc-value-display {
    overflow-wrap: anywhere;
}

.dtc-value-card,
.dtc-dependency-card {
    background: var(--dtc-surface);
}

.dtc-dependency-row {
    color: inherit;
}
"#;
