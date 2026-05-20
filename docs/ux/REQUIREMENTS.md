# DNA TreeCalc — UX Requirements (Conceptual)

This is the conceptual UX requirements document. It pins down what the user-facing application must present and let the user do. It does not commit to layout, styling, or technical implementation — those are covered by [`prototypes/`](prototypes/) (visual prototypes) and [`TECHNICAL.md`](TECHNICAL.md) (implementation/integration).

This document cross-references [`CORE_MODEL_SPEC.md`](../model/CORE_MODEL_SPEC.md) for the underlying language and engine model.

---

## 1. Product framing and personas

### 1.1 Product framing

DNA TreeCalc is the multi-node host built on the OxCalc tree substrate, analogous to how DNA OneCalc is the single-formula host on OxFml. Its conceptual model is "Excel with named nodes, nesting deeper than sheet/cell, plus templates as reusable subtrees, plus per-node formatting, plus reference-collection navigation operators."

The UX must honor three baseline expectations:
- A spreadsheet-fluent user (Excel power user) recognizes the core editing affordances and keybindings.
- A modeler / data analyst sees a tree structure that maps directly to how they think about hierarchical models (accounts, time periods, regions, scenarios).
- A casual / reader / consumer can navigate a published workspace without editing.

### 1.2 Personas

**P1. The modeler (primary persona for v1).** Builds multi-node calculation graphs — financial models, scientific computations, data pipelines, what-if analyses. Wants fast editing, refactor support, navigation, drill, templates. Comes from Excel; expects keyboard fluency.

**P2. The reader / reviewer.** Looking at someone else's published workspace, drilling into how values are derived. Wants navigation and drill, not editing. Read-only mode of the same shell.

**P3. The casual / migration user.** Brings an Excel workbook (sheets + defined names), expects it to "just work" after import. Wants familiarity above novelty.

The shell must accommodate all three. P1 is the design driver; P2 is a view-only variant; P3 is an entry path validated by the Excel-defined-name import contract (spec §10).

### 1.3 Non-goals (for v1)

- Multi-user collaborative editing (single-user only).
- Mobile / touch-primary UX (desktop / laptop with keyboard is the design target).
- Charting and visualization (deferred; basic value-shape rendering only).
- Locale-customized rendering beyond what OxFml's existing locale machinery provides.

**Notable in-scope (clarification 2026-05-19):** Real-time / async-streaming external data values (RTD-style pushes from streaming sources) are explicitly in scope. The mechanism lives in OxCalc — the engine supports values that arrive asynchronously and propagate through the dependency graph as invalidations. The host's job is to provide a bridge adapter that feeds those updates into the engine and to render the resulting value changes; skins observe workspace state and re-render normally. No skin-specific accommodation needed beyond the standard signal subscription model.

---

## 2. Presentation areas

The UX consists of distinct presentation areas. Each is a named region with specific content and interactions. The shell composes them in a layout; multiple layouts are supported (§4).

### 2.1 Workspace shell chrome

**2.1.1 Workspace context bar (top).** Persistent strip showing:
- Workspace filename and unsaved-changes indicator.
- Active capability profile (e.g., `treecalc-v1`).
- Recalc status: idle / evaluating / errored / cycle-blocked, with timing for the last recalc.
- Workspace-level menu (file operations, view modes, settings).
- Search affordance (Ctrl+F equivalent, see §3.8).

**2.1.2 Navigation rail (left).** The primary tree outline. Renders the workspace tree as a hierarchical, virtualized list. Each row shows:
- Indentation indicating depth.
- Expand/collapse triangle for non-leaf nodes.
- Node name (with bracket-escape decoration where the literal name has special chars).
- Value summary — scalar value, `[Array N×M]` for arrays, `[Table]` for table-typed, status icon for errors / cycles / pending.
- Selection / hover affordances.
- Drag handle for reordering.
- Meta-node toggle (small icon by default hides meta-nodes; toggle reveals).

The rail virtualizes for large trees (tens of thousands of nodes). Filter / search narrows visible rows.

**2.1.3 Working area (center / right).** The variable-content region whose layout depends on the selected node and the chosen presentation mode (§4). Default is the three-pane "node editor" layout:
- Formula editor on top.
- Value detail / array view / drill in the middle.
- Diagnostics / dependency map / structural-edit context at bottom.

**2.1.4 Status foot (bottom).** Per-node and per-workspace status:
- Selected node's calc state (clean / dirty / evaluating / error / cycle-blocked).
- Time since last published value.
- Dependency direction indicators (count of dependencies / dependents).
- Quick-action shortcuts for current node.

### 2.2 Tree outline (navigation rail)

The tree outline is the workspace's index of truth. Requirements:

- **Hierarchical rendering.** Standard outline: triangles expand/collapse, indentation indicates depth.
- **Virtualization.** Performant at tens of thousands of nodes; only visible rows render.
- **Search / filter.** Type-ahead narrows rows to matching nodes (case-insensitive name match; future: formula-text match, value match).
- **Selection.** Single-select primary; multi-select with Shift/Ctrl for batch operations.
- **Drag-and-drop reordering.** Drop indicator shows where the node will land; drop confirms the move (subject to the rename-propagation prompt; §3.4).
- **Inline rename.** F2 on a selected node opens an inline rename input.
- **Context menu.** Right-click on a node reveals operations (insert sibling, insert child, rename, move, duplicate, delete, promote to template, attach format, etc.).
- **Meta-node visibility toggle.** A workspace-level toggle reveals or hides meta-nodes in the outline; default hides them. When revealed, meta-nodes render with distinct visual treatment.
- **Status badges.** Per-row status icons (error, cycle, dirty, computed-from-template, has-format-meta, etc.).
- **Breadcrumb path.** When a deep node is selected, a breadcrumb shows the path from root. Clicking any breadcrumb segment selects that ancestor.

### 2.3 Formula editor

The formula editor is reused from DnaOneCalc (the existing `OxfmlEditorBridge` surface) with no semantic changes. Requirements:

- **Textarea-based input** with syntax overlay (reusing DnaOneCalc's `SyntaxRun` / `SyntaxTokenRole` machinery).
- **Live diagnostics.** Red squiggles for parse / bind errors; hover for diagnostic detail.
- **Completion / autocomplete** for function names, defined names visible in scope, and node names resolved via walk-up. Tab to accept; Esc to dismiss.
- **Signature help** showing function parameter hints as the user types arguments.
- **Reference resolution hover.** When the cursor sits on a reference token, a hover popup shows what the reference resolves to (absolute path / value).
- **Bracket-pair highlighting.** Matching brackets visually paired.
- **Reference-form cycling** (F4 equivalent) for relative / absolute mode where applicable.
- **Excel-aligned keybindings.** F2 enter edit mode, Enter commit, Esc cancel, Tab accept completion, Shift+Enter newline within formula text. See §3.10 for the full table.
- **Multi-line entry support.** Formulas can be long; line wrap and scrollable input.
- **Function help panel** alongside (DnaOneCalc's `FunctionHelpPacket` rendering).

The formula editor is bound to the currently-selected node. Switching nodes commits the previous formula (or prompts on unsaved edits per §3.5).

### 2.4 Value detail panel

Displays the selected node's computed value. Adapts to the value's shape:

- **Scalar value:** large primary display with effective formatted text. Optional sub-displays: raw underlying value, current format-code being applied.
- **Array value:** virtualized grid view. Each cell shows formatted text + optional CF outcomes (data bars, icon sets, font/fill color from `ArrayCellFormat`). Supports scrolling for large arrays; "load more" / lazy-fetch for arrays exceeding a memory budget.
- **Reference value:** displays "Reference to [path]" with a click-through to navigate.
- **Lambda value:** displays the lambda signature; not invokable directly from this panel.
- **Error value:** the error code (#REF!, #VALUE!, #CALC!, #NAME?, #DIV/0!, etc.) with diagnostic context.
- **Table value:** structured-table view (see §2.7 below).

Side affordances:
- "Pin this value" — keep the value display visible even when another node is selected.
- "Copy value as text" — clipboard support for scalars and rendered arrays.
- "Show in canvas" — promotes the node to the free-canvas layout (§4.3).

### 2.5 Drill panel (walk-tree)

Reuses DnaOneCalc's drill panel directly. Requirements:

- **Tree-of-prepared-calls rendering** (recursive `FormulaWalkNode`).
- Per-node state badges: Evaluated, Bound, Blocked, Opaque, Skipped.
- Expand / collapse subtrees.
- Click a sub-expression to navigate to the referenced node (when the sub-expression is a reference).
- "Snapshot trace" — capture the current walk-tree as an artifact for sharing / debugging.
- Toggle-able trace mode: ValueOnly (cheap) vs. PreparedCalls (expensive, full walk-tree).

Drill panel opens by default for the selected node; collapsible to reclaim space.

### 2.6 Dependency map

A panel showing the selected node's relationship to others. Two flavors:

**2.6.1 Local dependency map.** For the selected node:
- "Depends on" — list of nodes this formula references (incoming dependencies).
- "Depended on by" — list of nodes whose formulas reference this node (outgoing dependents).
- Click any entry to navigate.
- Filter by type (direct vs. transitive).

**2.6.2 Graph view (optional, on-demand).** A graph-visualization of dependencies within a subtree. Nodes as boxes, edges as arrows. Useful for moderate-size subtrees; impractical for full workspaces. Triggered explicitly ("Show dependency graph for this subtree").

### 2.7 Table value editor

When a node's value is a Table (§2.4 of spec — note: tables are deferred to a future spec section), the value detail panel renders a structured-table editor:

- Column headers with names, optional totals row.
- Data rows displayed as a grid.
- Cell-level editing (constant values) or column-level formula editing.
- Add/remove rows; add/remove columns.
- Structured-reference autocomplete inside column formulas (`[@Col]`, `[#Headers]`, `[#Data]`, etc.).
- Column format inheritance from `Format` meta-children.

### 2.8 Format editor

When the user invokes "Edit Format" on a node, a format editor panel appears (modal or dockable):

- Number-format code editor with live preview.
- Font: family, size, weight, italic, color.
- Fill: solid color, gradient, pattern.
- Conditional formatting: rule list with editing for each.
- Data bars: enable, color, direction.
- Icon sets: choose from standard sets.
- Per-property toggle: literal vs. computed (a property's value can be a formula returning the format value).

Reads and writes the node's `Format` meta-child (per the meta-node model). When inherited from an ancestor, the source is shown ("inherited from .Accounts.Format"); user can override.

### 2.9 Template editor

When the user opens a template (a meta-flagged subtree), the template editor appears:

- The template's structure as a sub-tree outline (similar to the navigation rail but scoped).
- Editing operations on the template's nodes: insert, rename, move, delete, edit formula.
- Per-template metadata: name, description, version (host-bumped).
- Instance list: which workspace positions are currently instances of this template; click to navigate.
- "Validate / Sync" action: compare instances against the current template using the stored template-id mapping, then apply accepted changes.
- "Fit-check" (future) — given a selected subtree elsewhere, check whether it could be adopted as an instance.

### 2.10 Diagnostics panel

Per-node diagnostic surface. Renders:
- Parse errors with positions.
- Bind errors (unresolved references, type mismatches).
- Calc errors (#REF!, #DIV/0!, etc.) with originating context.
- Warnings: cycle-detected, capability-profile mismatch, performance hints.

Each diagnostic links to the position in the formula or to the offending dependency.

### 2.11 Workspace-level views

Beyond per-node detail, the shell offers workspace-wide views:

**2.11.1 Calc status overview.** A heatmap or list of all nodes' calc states; click to navigate to dirty/errored/cycle-blocked nodes.

**2.11.2 Templates registry.** Lists all templates in the workspace with their instance counts; click to open the template editor.

**2.11.3 Workspace settings / config.** Cross-workspace alias manifest, UI preferences, capability profile (read-only display).

**2.11.4 Import / export status.** When importing from Excel or another workspace, shows progress and any items that didn't translate.

---

## 3. Editing and manipulation actions

### 3.1 Node creation

- **Insert sibling** — create a new node at the same level as the selected node, immediately after it. Default name "NewNode", incremented if collision. Enters inline-rename mode.
- **Insert child** — create a new node as the first or last child of the selected node. Enters inline-rename.
- **Insert at root** — create a new top-level node ("sheet equivalent").
- **Duplicate node** — create a copy of the selected node and all its descendants at the same level (with name suffix).
- **Paste node(s)** — paste previously-cut/copied nodes at the current position.

All creations are committed atomically and trigger recalc as needed.

### 3.2 Node deletion

- **Delete selected node(s)** — confirm prompt if any other formula references the node(s) being deleted.
- **Cascade behavior** — descendants are deleted with the parent.
- **Reference handling** — references from non-deleted nodes to deleted ones become `Unresolved`. The confirm prompt shows affected references.
- Undo restores everything.

### 3.3 Rename

- F2 on selected node opens inline-rename.
- New name validated for uniqueness within parent (case-insensitive).
- On commit, prompt with the list of referencing formulas: "N formulas reference this node. Propagate the rename?"
  - **Propagate** — all references are rewritten.
  - **Don't propagate** — references break to `Unresolved` for tracking.
  - **Cancel** — rename reverted.
- Undo restores both name and references.

### 3.4 Move and reorder

- **Drag-to-move** — drag a node onto another node's position. Drop indicator shows the target.
  - Onto another node's "before/after" zone: move as sibling at the new position.
  - Onto a node body: move as a child of the target.
- **Keyboard move** — Ctrl+Up/Down to reorder among siblings; Ctrl+Left/Right to outdent/indent.
- **Move semantics across parents** — references involving the moved node may need rebinding:
  - Relative references (`^.X`, etc.) inside the moved subtree rebind to their new structural context.
  - Absolute references (`[]Foo.Bar`) to the moved node may break.
- **Prompt** — analogous to rename, list affected references, offer propagate vs. break.

### 3.5 Formula editing

- F2 enters formula edit mode for the selected node.
- The formula editor takes focus.
- Live re-evaluation as the user types (debounced).
- Enter commits the formula; Esc reverts to the prior text.
- Tab accepts completion suggestion.
- Switching nodes (clicking another, arrow keys in tree) prompts on unsaved edits: "Commit / Discard / Cancel".

### 3.6 Value entry (constant-content node)

When the node's content is a constant (no leading `=`, not formula-derived):
- Single value: the editor accepts one content string; `""` is the empty node value, a leading `=` is a formula, and any other entry is a literal constant (number/logical/text per Excel cell-entry rules — CORE_MODEL_SPEC §6).
- Array constant: the value detail panel renders an editable grid. Paste from clipboard (CSV/TSV) supported. Resizing the array is by adding/removing rows or columns.
- Switching between constant and formula: the user can toggle ("convert to formula" / "freeze to constant").

### 3.7 Selection and multi-select

- **Single-select** — clicking a tree row or in the canvas selects that node.
- **Shift-click range** — selects a range of siblings in the tree.
- **Ctrl-click toggle** — adds/removes nodes from the selection set.
- **Selection-set operations** — bulk delete, bulk rename (templated), bulk format, bulk export.

### 3.8 Search / find

- Ctrl+F opens a workspace search field.
- **Name search** — type-ahead matches node names (substring, case-insensitive).
- **Formula text search** — find nodes whose formula contains a substring.
- **Value search** — find nodes whose current value matches a pattern.
- **Combined filters** — name OR formula OR value, with checkboxes.
- Results render as a list with click-to-navigate. Selected results highlight in the tree.

### 3.9 Undo / redo

- Ctrl+Z undoes the last operation (single atomic step from the user's perspective, even if internally composed of N engine edits).
- Ctrl+Y / Ctrl+Shift+Z redoes.
- Undo history persists for the session (not across workspace close).
- Operations grouped per logical action — e.g., template sync is one undoable step, not N.

### 3.10 Keyboard shortcuts

| Key | Action |
|---|---|
| F2 | Edit node name (inline) or formula (when focused on editor) |
| Enter | Commit edit |
| Esc | Cancel edit / dismiss popup |
| Tab | Accept completion / signature item |
| Shift+Tab | Reject completion or move backward |
| Arrow Up/Down | Navigate sibling rows in tree |
| Arrow Left/Right | Collapse/expand subtree (when on tree row) |
| Ctrl+Up/Down | Reorder selected node among siblings |
| Ctrl+Left/Right | Outdent / indent selected node |
| Ctrl+F | Open search |
| Ctrl+N | Insert new sibling node |
| Ctrl+Shift+N | Insert new child node |
| Delete | Delete selected node(s) |
| Ctrl+D | Toggle formula drill panel |
| Ctrl+Alt+D | Toggle developer mode |
| F3 | Jump to definition (when on a reference) |
| Shift+F3 | Find all references to selected node |
| F4 | Cycle reference form (relative ↔ absolute) when reference selected |
| Ctrl+Z / Ctrl+Y | Undo / redo |
| Ctrl+S | Save workspace |
| Ctrl+O | Open workspace |
| Ctrl+B | Toggle navigation rail |
| Ctrl+. | Open command palette |

Excel-familiar bindings preserved where the action maps directly. TreeCalc-specific bindings (structural editing, drill, search) added.

### 3.11 Command palette

Ctrl+. opens a command palette with fuzzy-search over all available commands. Each command shows its keyboard shortcut. Reduces dependence on menu/context-menu discovery for power users.

### 3.12 Workspace-level actions

- **New workspace** — empty workspace with a default root.
- **Open workspace** — load from a `.dnatree` (or whatever extension) file.
- **Save / Save As** — persist to disk.
- **Import Excel** — bring in an Excel workbook restricted to sheets + defined names (per spec §10). Shows a preview of the import mapping; lets user adjust before confirming.
- **Export to Excel** — flatten back to Excel defined names (with the bidirectional fidelity caveats per spec §10.7).
- **Settings** — UI preferences, capability profile, autosave interval, font size, etc.

---

## 4. Layout modes — reframed as skins

> **Updated 2026-05-19:** the modes below are not alternative layouts the user picks one of; they are **skins** — parallel front-ends to the same core, switchable at runtime, each with its own persisted state in dedicated meta-namespaces. See [`SKINS.md`](SKINS.md) for the architecture. The functional requirements per skin below remain valid; only their relationship to each other has changed (parallel, not exclusive).

### 4.1 Three-pane node editor (default)

The default editing layout: formula editor + value detail + diagnostics/drill. Reused from DnaOneCalc, extended for tree context.

**Strengths:** familiar (Excel-adjacent, OneCalc-style), keyboard-friendly, focused.

**Use cases:** building or debugging a single node's formula.

### 4.2 Outline-table (tree-table hybrid)

Rows are nodes (regular only; meta hidden), columns are attributes (name, formula, value, format-summary, status). Dense, scannable.

**Strengths:** lets users see many nodes at once; column sorting/filtering.

**Constraints:** array-valued nodes can't be edited inline in a cell — they overflow. The value column shows a summary; double-click drills into the array.

**Use cases:** workspace overview, bulk editing, comparing many nodes at once.

### 4.3 Free canvas

Nodes can be positioned anywhere on a 2D canvas. Each node is a card showing name, formula (collapsible), value. Drag-positioning. Connections drawn between nodes that reference each other.

**Strengths:** visual organization, dashboard-like presentation, custom layouts.

**Constraints:** position is host-level metadata, not part of structural tree. Persisted per node in the workspace file.

**Use cases:** dashboards, narrative presentations, building visual mental models.

### 4.4 Notebook (linear with indentation)

A vertical scroll of nodes in tree order. Non-leaf nodes precede their children; indentation indicates nesting. Each node is a "cell" with editable formula and value.

**Strengths:** narrative top-to-bottom flow, good for presentation and reports.

**Constraints:** scales poorly for wide trees; awkward for deep trees.

**Use cases:** report-style presentations, tutorials, exported documents.

### 4.5 Adaptive / auto-layout

The system chooses a layout per top-level subtree based on its shape:
- Templated uniform subtrees (Q1/Q2/Q3/Q4 with parallel inner structure) → grid view, one column per instance.
- Tabular data (table-valued nodes) → table view.
- Mostly-leaf flat nodes → list or notebook.
- Mostly-deep narrow nodes → outline.

User can override with explicit layout choice.

### 4.6 Drill-down

The shell supports "summary-at-parent, expand-to-children" interactions. A parent node's formula commonly summarizes its children (`SUM(.*)`, `AVG(.*)`, etc.). The display:
- Shows the parent's summary value prominently.
- Renders children as nested rows / cards visible on expansion.
- Supports expand-all / collapse-all per subtree.
- Allows drill from a parent summary directly to the contributing children (click the summary, jump to the children's values).

### 4.7 Multi-pane / split view

The user can split the working area into multiple panes, each showing a different node. Useful for comparison, side-by-side editing, or watching a value while editing a dependency.

### 4.8 Read-only / presentation mode

A "viewer" mode hides editor surfaces. Tree is navigable; values and arrays are visible; no editing. Used for sharing workspaces with reviewers or for "publish to readers" workflow.

---

## 5. Specific interaction patterns

### 5.1 Array values that change size

An array-valued node may grow or shrink as the formula re-evaluates. The UX requirements:
- **Stable view position** — when the array grows, the user's scroll position is preserved (the value detail panel doesn't jump).
- **Size indicator** — `[Array N×M]` summary in the tree outline always reflects the current shape; clearly visible.
- **Auto-fit option** — the value-detail grid can auto-resize to fit the data, with a max-cells cap to prevent runaway.
- **Lazy load** — for very large arrays (millions of cells), the grid virtualizes; only visible cells fetch.
- **Diff highlighting** — when an array changes (between recalcs), the changed cells briefly highlight to draw attention.

### 5.2 Nodes with both formula and children

Every node has a formula AND can have children. The UX shows both:
- Node row in tree shows the value computed from the formula.
- Expanding the row reveals child nodes with their own values.
- The user can edit the parent's formula independently of the children.
- If the parent's formula explicitly references children (`SUM(.*)`), the relationship is reflected in the dependency map.

### 5.3 Cross-workspace references

A formula `=[reports]Q1.Revenue` references another workspace. UX requirements:
- The external workspace's status (loaded / not loaded / error / stale) is visible inline at the reference site.
- Clicking the bracket portion offers "Open referenced workspace" or "Edit alias mapping."
- Workspace-level "External workspaces" panel lists all cross-workspace aliases and their current status.
- Stale-data warning when a referenced workspace has been republished since the local workspace last read.

### 5.4 Templates and instances

- An instance subtree is visually marked in the tree outline (badge, slightly different background, icon).
- Hovering an instance shows "Instance of template `QuarterShape` v7."
- Template-link status is visible: current mapping, stale/needs-validation, detached, or manually changed since the last validation.
- Right-click on an instance offers: "Show template," "Validate against template," "Sync from template now," and "Detach from template."
- Editing the template opens its dedicated editor (§2.9); on save, the template version advances and the UI can offer a validate/sync action for existing instances.

### 5.5 Formatting

- A node with a `Format` meta-child shows a small format indicator in the tree outline.
- Per-node format applied to the value display.
- Inherited formats indicated: "Format inherited from `.Accounts.Format`" shown when applicable.
- The format editor (§2.8) is accessible via right-click → "Edit Format" or keyboard shortcut.

### 5.6 Meta-nodes (when revealed)

- By default, meta-nodes are hidden in the navigation rail.
- A toggle (eye-icon at the top of the rail or in workspace settings) reveals all meta-nodes.
- When revealed, meta-nodes render with distinct visual treatment (greyed text, italics, dedicated icon by role — template, format, config, draft, annotation).
- Meta-nodes are NOT addressable from formulas (per spec §3.1), so they don't appear in formula completion suggestions even when revealed.

### 5.7 Error rendering

- Errors at the node value level: red border on the value display, error code shown prominently, hover for context.
- Errors during typing: red squiggle under the offending token, hover for diagnostic.
- Cycle errors: distinct visual treatment (warning icon), with "Show cycle" action that highlights the cycle in the dependency map.
- Workspace-level error summary in the status foot ("3 errors, 1 cycle").

### 5.8 Calc state visualization

Per-node state from the engine's invalidation vocabulary, mapped to user-visible icons:
- Clean (no badge / green dot) — value is current.
- Dirty / pending — yellow dot.
- Evaluating — animated spinner.
- Error — red dot with error code.
- Cycle-blocked — orange warning icon.
- (Note: `verified_clean` is not distinguished from `clean` per spec §7.)

### 5.9 Drag-and-drop semantics

- Within tree: drag node row to reorder or reparent.
- From tree to canvas: drag a node onto the canvas to add it to the visual layout.
- From canvas to tree: drag a node from the canvas onto the tree to re-anchor (mostly a no-op since the tree is structural; useful for confirming structural position).
- External drag-in: drag a `.dnatree` file or `.xlsx` file onto the shell to open / import.

### 5.10 Copy and paste

- Copy node: clipboard carries node identity + structure + formula text.
- Paste as new node: instantiates the structure at the paste destination with new identity (renaming on collision).
- Paste as link (Ctrl+Shift+V): creates a reference-only node pointing to the original.
- Copy value: clipboard receives the formatted value text (scalar or array as CSV/TSV).
- Cross-workspace copy: tracked, prompts the user to register an alias if the source workspace is opened separately.

---

## 6. Adaptive behaviors

### 6.1 Per-node-shape rendering

The shell auto-selects rendering based on each node's value type:
- Scalar: hero value display.
- Array: virtualized grid.
- Table: structured-table editor.
- Reference: indirect-target display with click-through.
- Lambda: signature display.
- Error: prominent error rendering with diagnostic.

User can override the rendering choice for a specific node (e.g., "always show as grid even for scalar").

### 6.2 Performance tiers

Based on workspace size, the shell adapts:
- **Small** (< 100 nodes): everything renders; no virtualization needed; all panels eager.
- **Medium** (100 – 10K nodes): tree virtualized; drill panel lazy; value detail eager.
- **Large** (10K – 1M nodes): tree heavily virtualized; subtree lazy-loading; status overview shown as heatmap; recalc batched.
- **Very large** (> 1M nodes): same as large with stricter recalc batching, optional read-only mode for portions of the tree.

### 6.3 Accessibility

- Keyboard-complete: every action reachable without a mouse.
- Screen reader: ARIA labels on tree rows, formula tokens, value displays.
- High-contrast theme variant.
- Configurable font size and weights.
- Color choices not the sole indicator of state (icons + text accompany color).

---

## 7. Cross-cutting concerns

### 7.1 Save / persistence

- Autosave to a workspace file on each significant edit (debounced).
- localStorage fallback for browser-resident workspaces (same pattern as DnaOneCalc).
- Workspace file format records: tree structure, formulas, values (or recompute on load), formats (as meta-data), templates, instance links, cross-workspace alias manifest, capability profile, UI layout preferences per node.
- Explicit "Save As" for naming and choosing location.

### 7.2 Versioning

- Workspace file carries a schema version; loading mismatched version triggers migration (or refuses with error).
- Capability profile carries its own version (`treecalc-v1`).
- Templates carry their own version (host-bumped).

### 7.3 Multi-user (future, not v1)

Not in scope, but the workspace file format should not preclude future collaboration features. Each node's identity is stable (uses `TreeNodeId(u64)` underneath), so future merge / conflict resolution has a basis.

### 7.4 Profile and capability awareness

The UI consults the workspace's `capability_profile_id` (per spec §4). Under strict-Excel profile, TreeCalc-specific operators (`.*`, `**`, `@`-family, etc.) are unavailable in the editor; the autocomplete pool and syntax highlighter reflect the profile.

### 7.5 Help and documentation

- Inline function help in the editor (DnaOneCalc's existing `FunctionHelpPacket`).
- Hover diagnostics with explanation.
- Workspace-level help menu with topic browser.
- Tutorial walkthrough for first-time users.

### 7.6 Notifications

- Toast notifications for non-blocking events (save complete, recalc finished, template synced).
- Modal dialogs for blocking events (rename propagation, cross-workspace alias prompt, breaking-change warnings).

---

## 8. Coverage / coherence check

This section verifies the requirements cover all the spec's scope.

| Spec section | UX requirement coverage |
|---|---|
| §2 Core model — nodes, formula+value+format, tree structure | §2.2 tree outline, §2.3 formula editor, §2.4 value detail, §2.8 format editor |
| §3 Reference syntax (path, anchors, separators) | §2.3 editor with autocomplete + hover; §3.10 keybindings (F4 reference cycling) |
| §3.4 Bracket-escape rules | §2.3 editor handles transparently; tree outline renders bracket-decorated names |
| §3.5 Meta-accessor `@`-family | §2.3 editor recognizes; not a separate UX surface |
| §3.6 Recursive descent `**` | §2.5 drill panel shows expanded refs; §6.2 large-tree perf considerations |
| §3.7 Reference-to-engine mapping | Transparent to user; UX shows resolved bindings via §2.3 hover |
| §4 Capability profile | §7.4 profile awareness in editor |
| §5 Excel alignment | §3.10 keybindings; §3.12 Excel import |
| §6 Values / arrays / engine prereqs | §2.4 value detail; §5.1 array size changes |
| §6 calc-participation flag (is_meta) | §5.6 meta-node rendering |
| §7 Calc-state display | §5.8 calc state visualization |
| §7b Templates | §2.9 template editor; §5.4 template/instance UX |
| §8 Structural editing | §3 actions (create, delete, rename, move) |
| §10 Excel import | §3.12 workspace-level actions; §2.11.4 import status |
| Tables | §2.7 table editor (deferred but UX surface defined) |
| Meta-nodes (formatting use) | §2.8 format editor; §5.5 formatting UX |

Gaps flagged for future work:
- **Charting and visualization** — explicit non-goal for v1; eventual integration with the canvas layout (§4.3).
- **Multi-user collaboration** — out of v1 scope; persistence format leaves room.
- **External data connectors** — not addressed (but RTD / async-streaming value updates ARE in scope; mechanism in OxCalc, host pushes, skins observe).
- **Mobile / touch layout** — not in v1 scope.
- **UDF hosting (VBA + .xll)** — TreeCalc supports both VBA UDFs (via OxVba) and `.xll` native add-ins (Excel C API / Excel SDK), consumed via a shared UDF-hosting core extracted from DnaOneCalc-first work — not reimplemented per host. Not on v1 critical path; arrives when the shared core lands.

---

## 9. UX dial-by-dial summary

Decisions implicit in this requirements doc:

| Decision | Locked |
|---|---|
| Primary persona | Modeler (P1) |
| Default layout | Three-pane node editor (§4.1) |
| Tree outline visibility | Always visible (left rail); collapsible |
| Meta-nodes default visibility | Hidden in tree by default |
| Excel keybindings | Adopted where they map (F2, Enter, Esc, Tab, F4, etc.) |
| Read-only mode | Available via toggle (§4.8) |
| Persistence | Autosave + explicit save |
| Wow-moment for v1 | Templated forecast model (Accounts.YYYY.Qx pattern) |
| Tech reuse from DnaOneCalc | Formula editor surface, bridge pattern, drill panel, completion machinery |
| Adaptive layouts | Auto-chosen per subtree, with explicit override |

Dials worth surfacing for further user input:
- Default font / theme (DnaOneCalc's warm beige vs. alternatives).
- Default meta-node visibility (currently hidden; could default to "shown but greyed").
- Whether templates registry is a separate top-level workspace view or a section of the workspace settings panel.
- Free-canvas auto-layout details (force-directed graph initialization vs. user-positioned blank slate).
- Cross-workspace stale-data refresh policy (auto vs. prompt).

---

## 10. Status

This UX requirements document is comprehensive at the conceptual level. It covers all presentation areas, editing actions, layout modes, and interaction patterns identified through the design conversation. Cross-references to the design spec confirm coverage of all locked language and engine features.

Next:
- [`prototypes/`](prototypes/) — visual HTML mockups grounded in these requirements.
- [`TECHNICAL.md`](TECHNICAL.md) — the implementation/integration plan, tech stack, and component architecture.
