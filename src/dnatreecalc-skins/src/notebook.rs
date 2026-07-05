//! NOTEBOOK — the B1 read-only notebook skin (N1, route-map §B, §E.3).
//!
//! N1 ships the **entry-list derivation only**: a pure `derive_entries` fn
//! that turns a `WorkspaceState` into an ordered list of notebook entries
//! (names, uncovered cells, tables), a pure `entry_classification` fn that
//! badges each entry from its own authored metadata, and a read-only render
//! of that list plus the name rail (§B.2). This bead is READ-ONLY by
//! construction: zero mutation dispatches, no editor, no prose — those are
//! N2 (edit loop), N3 (name authoring), N4 (prose).
//!
//! **Grid-backed entries never mint synthetic `NodeKey`s** (coordinator
//! ruling 2026-07-05 after N1's escalation, §E.3 amended row): a name or an
//! uncovered cell is backed by a grid address (`grid` + `row`/`col`), not a
//! tree-model node, so [`entry_classification`] maps the C2 categories
//! (`workspace.rs:1184`) from the entry's own `GridAuthoredCellProjection`
//! and its covering [`DefinedNameProjection`] (if any) — never through
//! [`dnatreecalc_skin_framework::WorkspaceState::node_classification`], which
//! requires a real `NodeKey` in the dependency graph.
//!
//! Entry order (until H9's manifest lands, per the amended acceptance #1):
//! names in catalog order, then uncovered cells in sheet/row/col order, then
//! tables in grid/declaration order — matching B.1's "name-declaration order
//! + sheet/cell order for unlisted entries" default.

use std::sync::Arc;

use dnatreecalc_skin_framework::{
    CELL_ENTRY_CSS, CellEntryEditor, DefinedNameProjection, DefinedNameScopeProjection,
    DefinedNameTargetProjection, Dispatcher, EntryDiagnostics, EntryFeedback,
    GridAuthoredCellProjection, GridAuthoredKindProjection, GridCellProjection,
    GridEditabilityProjection, GridTableOverlayDescriptor, NAME_FORM_CSS, NameForm,
    NodeClassification, NodeId, NodeValueProjection, RenameNameForm, SkinCapabilities,
    SkinCategory, SkinContext, SkinHandle, SkinId, SkinManifest, SkinState, WorkspaceIntent,
    WorkspaceSkin, WorkspaceState,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::value_render::render_value;

pub const NOTEBOOK_ID: SkinId = SkinId::new("notebook");

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotebookState;

impl SkinState for NotebookState {
    fn schema_version() -> u32 {
        1
    }
}

#[derive(Default)]
pub struct NotebookLens;

impl NotebookLens {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSkin for NotebookLens {
    type State = NotebookState;

    fn id(&self) -> SkinId {
        NOTEBOOK_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Notebook",
            description: "Read-only notebook: entry list derived from names, cells, and tables.",
            category: SkinCategory::Presentation,
            version: "0.1.0",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities {
            supports_multi_select: false,
            supports_inline_formula_edit: false,
            supports_meta_node_display: false,
            renders_arrays_inline: true,
            renders_table_values: true,
            allowed_slots: None,
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        SkinHandle::new(view! { <NotebookView cx=cx /> }.into_any())
    }
}

// ---------------------------------------------------------------------------
// Pure entry model (unit-tested)
// ---------------------------------------------------------------------------

/// One row in the notebook's entry list (§B.1's four entry kinds, minus
/// prose — N1 is read-only and prose lives in the H9 manifest, N4's bead).
#[derive(Debug, Clone, PartialEq)]
pub enum NotebookEntryKind {
    /// A defined name (`rate = 0.065`); `backing_cell` is the full windowed
    /// cell projection at the name's static target (authored metadata +
    /// computed value), when the grid window covers it (`None` for a
    /// dynamic name, or a static name whose backing cell projection is
    /// outside the current window).
    Name {
        name: DefinedNameProjection,
        backing_cell: Option<GridCellProjection>,
    },
    /// An authored grid cell not covered by any defined name (the "Other
    /// cells" escape hatch, §B.1) — the notebook's total-surface guarantee.
    Cell {
        grid: NodeId,
        authored: GridAuthoredCellProjection,
        value: NodeValueProjection,
    },
    /// A structured-table overlay (§B.1's table entry).
    Table {
        grid: NodeId,
        table: GridTableOverlayDescriptor,
    },
}

/// One derived notebook entry: its kind plus the C2 badge
/// ([`entry_classification`]) and a display label.
#[derive(Debug, Clone, PartialEq)]
pub struct NotebookEntry {
    pub display_name: String,
    pub kind: NotebookEntryKind,
    pub classification: NodeClassification,
}

/// Classify one notebook entry's authored metadata into the same C2
/// categories the tree model uses (`NodeClassification`, `workspace.rs:1184`)
/// — **without** minting a synthetic `NodeKey`: this is a pure mapping from
/// the entry's own authored `kind` (+ the covering name's `is_dynamic`, when
/// present) to a classification, never a call through
/// `WorkspaceState::node_classification` (coordinator ruling 2026-07-05).
///
/// Axis mapping (mirrors the tree model's rule at the content-kind level;
/// grid entries carry no dependency-graph "consumed" signal today, so every
/// grid-backed entry classifies as the *unconsumed* arm of its content axis
/// — `FreeValue` for a literal, `Output` for a formula — until a grid-side
/// consumed signal exists (an engine ask, not a v1 concern) — **except** a
/// name's own target: a name is *definitionally* something other formulas
/// can reference, so a literal-backed name classifies as `Input` and a
/// dynamic (formula-backed) name classifies as `Intermediate`, matching how
/// the tree model treats a referenced node):
///
/// | authored kind         | name?           | classification |
/// |---|---|---|
/// | `Empty`                | —               | `Empty` |
/// | `Literal`               | name (static)   | `Input` |
/// | `Literal`               | no name         | `FreeValue` |
/// | `Formula`               | name (dynamic)  | `Intermediate` |
/// | `Formula`               | no name         | `Output` |
/// | `RichStub`              | —               | `Empty` (inert, no content) |
#[must_use]
pub fn entry_classification(
    authored: &GridAuthoredCellProjection,
    name: Option<&DefinedNameProjection>,
) -> NodeClassification {
    match authored.kind {
        GridAuthoredKindProjection::Empty | GridAuthoredKindProjection::RichStub => {
            NodeClassification::Empty
        }
        GridAuthoredKindProjection::Literal => {
            if name.is_some() {
                NodeClassification::Input
            } else {
                NodeClassification::FreeValue
            }
        }
        GridAuthoredKindProjection::Formula => {
            if name.is_some_and(|name| name.is_dynamic) {
                NodeClassification::Intermediate
            } else {
                NodeClassification::Output
            }
        }
    }
}

/// The left-gutter glyph for an entry kind (§B.2): `●` literal, `ƒ` formula,
/// `▦` table. (`¶` prose is N4's — no `NotebookEntryKind` arm produces it.)
#[must_use]
pub fn entry_glyph(entry: &NotebookEntry) -> &'static str {
    match &entry.kind {
        NotebookEntryKind::Table { .. } => "▦",
        NotebookEntryKind::Name { backing_cell, .. } => backing_cell
            .as_ref()
            .and_then(|cell| cell.authored.as_ref())
            .map_or("●", |authored| glyph_for_authored_kind(authored.kind)),
        NotebookEntryKind::Cell { authored, .. } => glyph_for_authored_kind(authored.kind),
    }
}

fn glyph_for_authored_kind(kind: GridAuthoredKindProjection) -> &'static str {
    match kind {
        GridAuthoredKindProjection::Formula => "ƒ",
        _ => "●",
    }
}

/// Resolve a defined name's authored backing cell from the covering grid's
/// windowed projection, when both the name is `Static` and the grid window
/// currently covers the target's anchor (top-left) cell. `None` for a
/// dynamic name or when the window does not (yet) cover the backing cell —
/// the caller must not treat `None` as "no content", only as "not resolved
/// from the current window".
fn resolve_backing_cell<'a>(
    workspace: &'a WorkspaceState,
    name: &DefinedNameProjection,
) -> Option<&'a GridCellProjection> {
    let DefinedNameTargetProjection::Static(rect) = &name.target else {
        return None;
    };
    let grid_id = name_scope_grid(workspace, name)?;
    let grid = workspace.grids.get(&grid_id)?;
    grid.cells
        .iter()
        .find(|cell| cell.row == rect.top_row && cell.col == rect.left_col)
}

/// The grid a name's static target resolves against: the sheet the scope
/// names when `Sheet`-scoped, or the sole grid in the workspace for a
/// `Workbook`-scoped name (the notebook's v1 single-sheet assumption; a
/// multi-sheet workbook-scoped name is a K-track/engine-ask concern, not N1's
/// — N1 never dispatches, so an ambiguous multi-grid workspace simply yields
/// no resolved backing cell rather than guessing).
fn name_scope_grid(workspace: &WorkspaceState, name: &DefinedNameProjection) -> Option<NodeId> {
    match &name.scope {
        DefinedNameScopeProjection::Sheet(id) => Some(id.clone()),
        DefinedNameScopeProjection::Workbook => {
            if workspace.grids.len() == 1 {
                workspace.grids.keys().next().cloned()
            } else {
                None
            }
        }
    }
}

/// True when a static name's target rect covers the given cell address (used
/// to exclude name-covered cells from the "Other cells" uncovered-cell
/// section, §B.1).
fn name_covers_cell(name: &DefinedNameProjection, grid: &NodeId, row: u32, col: u32) -> bool {
    let DefinedNameTargetProjection::Static(rect) = &name.target else {
        return false;
    };
    let Some(scope_grid) = name_scope_matches(name, grid) else {
        return false;
    };
    scope_grid
        && row >= rect.top_row
        && row <= rect.bottom_row
        && col >= rect.left_col
        && col <= rect.right_col
}

/// Whether `name`'s scope could possibly cover `grid` — `Sheet` scope must
/// match the grid exactly; `Workbook` scope covers every grid (single-sheet
/// v1 assumption, as `name_scope_grid`).
fn name_scope_matches(name: &DefinedNameProjection, grid: &NodeId) -> Option<bool> {
    match &name.scope {
        DefinedNameScopeProjection::Sheet(id) => Some(id == grid),
        DefinedNameScopeProjection::Workbook => Some(true),
    }
}

/// True when `(row, col)` falls inside any table overlay's `table_range` for
/// the given grid (table cells render inside the table entry, never doubled
/// into "Other cells").
fn cell_in_any_table(table: &GridTableOverlayDescriptor, row: u32, col: u32) -> bool {
    let rect = &table.table_range;
    row >= rect.top_row && row <= rect.bottom_row && col >= rect.left_col && col <= rect.right_col
}

/// Derive the notebook's ordered entry list from the published workspace
/// projection (§B.1 IA, N1 acceptance #1): every defined name, then every
/// authored grid cell not covered by a name or a table ("Other cells"), then
/// every table overlay — in that grouped order (manifest ordering is H9's, a
/// post-H9 test moves the ordering assertion once the manifest lands).
#[must_use]
pub fn derive_entries(workspace: &WorkspaceState) -> Vec<NotebookEntry> {
    let mut entries = Vec::new();

    // 1. Names, in catalog order.
    for name in &workspace.defined_names.entries {
        let backing_cell = resolve_backing_cell(workspace, name);
        let backing_authored = backing_cell.and_then(|cell| cell.authored.as_ref());
        let classification = backing_authored.map_or(
            // A name with no resolved authored backing cell yet (dynamic, or
            // window does not cover it): classify from the name's own
            // dynamic-ness alone, matching the Formula/no-authored-cell arm's
            // intent.
            if name.is_dynamic {
                NodeClassification::Intermediate
            } else {
                NodeClassification::Input
            },
            |authored| entry_classification(authored, Some(name)),
        );
        entries.push(NotebookEntry {
            display_name: name.name.clone(),
            kind: NotebookEntryKind::Name {
                name: name.clone(),
                backing_cell: backing_cell.cloned(),
            },
            classification,
        });
    }

    // 2. Uncovered cells, grid order then sheet/row/col order.
    for (grid_id, grid) in &workspace.grids {
        for cell in &grid.cells {
            let Some(authored) = &cell.authored else {
                continue;
            };
            if matches!(authored.kind, GridAuthoredKindProjection::Empty) {
                continue;
            }
            let covered_by_name = workspace
                .defined_names
                .entries
                .iter()
                .any(|name| name_covers_cell(name, grid_id, cell.row, cell.col));
            if covered_by_name {
                continue;
            }
            let covered_by_table = grid
                .overlays
                .tables
                .iter()
                .any(|table| cell_in_any_table(table, cell.row, cell.col));
            if covered_by_table {
                continue;
            }
            entries.push(NotebookEntry {
                display_name: cell_display_name(grid_id, cell.row, cell.col),
                classification: entry_classification(authored, None),
                kind: NotebookEntryKind::Cell {
                    grid: grid_id.clone(),
                    authored: authored.clone(),
                    value: cell.value.clone(),
                },
            });
        }
    }

    // 3. Tables, grid order then declaration order.
    for (grid_id, grid) in &workspace.grids {
        for table in &grid.overlays.tables {
            entries.push(NotebookEntry {
                display_name: table.table_name.clone(),
                classification: NodeClassification::Intermediate,
                kind: NotebookEntryKind::Table {
                    grid: grid_id.clone(),
                    table: table.clone(),
                },
            });
        }
    }

    entries
}

/// A stable, human-readable label for an uncovered cell entry: the grid's
/// display path plus its A1-style address, without hand-rolling a full A1
/// column-letter codec (the notebook's v1 needs a readable label, not a
/// second address-format authority — the row/col numeric form is
/// unambiguous and already what every other read-only surface here shows,
/// e.g. `sheet.rs`'s table-cell titles).
fn cell_display_name(grid: &NodeId, row: u32, col: u32) -> String {
    format!("{grid}!R{row}C{col}")
}

/// The editable backing cell of a notebook entry (§B.3 edit-commit loop): the
/// `(grid, row, col)` an `EnterGridCell` commit targets, plus the committed
/// text the editor's buffer seeds from (source text for a formula, literal
/// text for a literal, empty for a cleared cell).
///
/// `None` for entries N2 does not make editable: a table entry (structural,
/// K-track), a dynamic name (its body is a `SetDefinedName` re-bind, N3's), a
/// static name whose backing cell is outside the window (nothing to seed from
/// or address), and any cell whose `editability != Editable` (a spill/merged/
/// repeated/table-structural follower renders read-only — the classifier is
/// the backstop, §B.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryEditTarget {
    pub grid: NodeId,
    pub row: u32,
    pub col: u32,
    pub initial_text: String,
}

/// The committed text an authored cell's editor seeds from: a formula's source
/// text, a literal's display text, or empty (an empty/rich-stub cell has no
/// authored text to edit).
fn authored_initial_text(authored: &GridAuthoredCellProjection) -> String {
    match authored.kind {
        GridAuthoredKindProjection::Formula => authored.source_text.clone().unwrap_or_default(),
        GridAuthoredKindProjection::Literal => authored.literal_text.clone().unwrap_or_default(),
        GridAuthoredKindProjection::Empty | GridAuthoredKindProjection::RichStub => String::new(),
    }
}

/// Resolve the editable backing cell of an entry, or `None` when N2 does not
/// author it (see [`EntryEditTarget`]). A cell is editable only when its
/// authored projection classifies `Editable` — the notebook never guesses past
/// the classifier (§B.8, §C.3's affordance table).
#[must_use]
pub fn entry_edit_target(entry: &NotebookEntry) -> Option<EntryEditTarget> {
    match &entry.kind {
        NotebookEntryKind::Table { .. } => None,
        NotebookEntryKind::Name { name, backing_cell } => {
            // A dynamic name's body is a formula re-bind (N3), not a cell edit.
            if name.is_dynamic {
                return None;
            }
            let cell = backing_cell.as_ref()?;
            let authored = cell.authored.as_ref()?;
            if !matches!(authored.editability, GridEditabilityProjection::Editable) {
                return None;
            }
            let DefinedNameTargetProjection::Static(rect) = &name.target else {
                return None;
            };
            // Address the edit at the backing cell's own coordinates — the same
            // cell `resolve_backing_cell` matched (rect anchor). Its grid is
            // already known: the backing cell was resolved from exactly one
            // grid's window, so re-use the authored cell's coordinates and the
            // name's scope grid, keeping the read and edit paths identical.
            let grid = match &name.scope {
                DefinedNameScopeProjection::Sheet(id) => id.clone(),
                // A workbook-scoped name has no single grid identity on the
                // intent surface here; N2 leaves it read-only (multi-grid
                // resolution is a K-track/engine-ask concern, as N1's
                // `name_scope_grid` already documents).
                DefinedNameScopeProjection::Workbook => return None,
            };
            Some(EntryEditTarget {
                grid,
                row: rect.top_row,
                col: rect.left_col,
                initial_text: authored_initial_text(authored),
            })
        }
        NotebookEntryKind::Cell { grid, authored, .. } => {
            if !matches!(authored.editability, GridEditabilityProjection::Editable) {
                return None;
            }
            Some(EntryEditTarget {
                grid: grid.clone(),
                row: authored.row,
                col: authored.col,
                initial_text: authored_initial_text(authored),
            })
        }
    }
}

// ---------------------------------------------------------------------------
// View (read-only render: entry list + name rail, §B.2)
// ---------------------------------------------------------------------------

#[component]
fn NotebookView(cx: SkinContext<NotebookState>) -> impl IntoView {
    let workspace = cx.workspace;
    let dispatch = cx.dispatch.clone();

    let entries = {
        let dispatch = dispatch.clone();
        move || {
            let ws = workspace.get();
            let dispatch = dispatch.clone();
            derive_entries(&ws)
                .into_iter()
                .map(move |entry| {
                    view! { <EntryRow entry=entry dispatch=dispatch.clone() /> }.into_any()
                })
                .collect::<Vec<_>>()
        }
    };

    let name_rail = {
        let dispatch = dispatch.clone();
        move || {
            let ws = workspace.get();
            let dispatch = dispatch.clone();
            ws.defined_names
                .entries
                .iter()
                .cloned()
                .map(move |name| {
                    view! { <NameRailRow name=name dispatch=dispatch.clone() /> }.into_any()
                })
                .collect::<Vec<_>>()
        }
    };

    let name_form_open = RwSignal::new(false);
    let open_name_form = move |_| name_form_open.set(true);
    let name_form_view = {
        let dispatch = dispatch.clone();
        move || {
            if !name_form_open.get() {
                return ().into_any();
            }
            let ws = workspace.get();
            view! {
                <NameForm
                    defined_names=ws.defined_names.clone()
                    dispatch=dispatch.clone()
                    open=name_form_open
                />
            }
            .into_any()
        }
    };

    view! {
        <style>{NOTEBOOK_CSS}</style>
        <style>{CELL_ENTRY_CSS}</style>
        <style>{NAME_FORM_CSS}</style>
        <section class="dtc-notebook" aria-label="Notebook">
            <div class="dtc-notebook__entries">
                {entries}
                <div class="dtc-notebook__new-name">
                    <button
                        type="button"
                        class="dtc-notebook__new-name-button"
                        on:click=open_name_form
                    >
                        "+ name"
                    </button>
                    {name_form_view}
                </div>
            </div>
            <aside class="dtc-notebook__rail" aria-label="Names">
                <div class="dtc-notebook__rail-title">"NAMES"</div>
                {name_rail}
            </aside>
        </section>
    }
}

fn classification_class(classification: NodeClassification) -> &'static str {
    match classification {
        NodeClassification::Input => "dtc-notebook-badge--input",
        NodeClassification::FreeValue => "dtc-notebook-badge--free",
        NodeClassification::Intermediate => "dtc-notebook-badge--intermediate",
        NodeClassification::Output => "dtc-notebook-badge--output",
        NodeClassification::Empty => "dtc-notebook-badge--empty",
    }
}

fn classification_label(classification: NodeClassification) -> &'static str {
    match classification {
        NodeClassification::Input => "Input",
        NodeClassification::FreeValue => "Free",
        NodeClassification::Intermediate => "Intermediate",
        NodeClassification::Output => "Output",
        NodeClassification::Empty => "Empty",
    }
}

/// One entry row (§B.2 anatomy, §B.3 edit-commit loop): gutter glyph, display
/// name, C2 badge, and either the rendered value (read state) or the shared
/// [`CellEntryEditor`] (edit state). Array results render through the shared
/// [`render_value`], which collapses to the `{R×C array}` chip — never
/// editable, spill results are read-only by classifier (§B.8).
///
/// An entry that [`entry_edit_target`] resolves as editable becomes an
/// affordance: clicking its body (or pressing `Enter` while it holds focus)
/// opens the editor; committing dispatches exactly one
/// [`WorkspaceIntent::EnterGridCell`]; `Esc` reverts without dispatch; a typed
/// rejection keeps the editor open and lists the diagnostics under it (§B.3
/// step 3, §B.5). A non-editable entry (table, dynamic name, read-only cell)
/// renders exactly as N1's read-only row.
#[component]
fn EntryRow(entry: NotebookEntry, dispatch: Arc<dyn Dispatcher>) -> impl IntoView {
    let glyph = entry_glyph(&entry);
    let badge_class = classification_class(entry.classification);
    let badge_label = classification_label(entry.classification);
    let display_name = entry.display_name.clone();
    let kind_label = entry_kind_label(&entry.kind);
    // The value projection (Clone) an entry renders in its read state; `None`
    // for entries that show no value row (a name with no resolved backing cell,
    // a table). Kept as the projection — not a rendered `AnyView` (not `Clone`)
    // — so the reactive body can re-render it on each read/edit toggle.
    let value_projection: Option<NodeValueProjection> = match &entry.kind {
        NotebookEntryKind::Name {
            backing_cell: Some(cell),
            ..
        } => Some(cell.value.clone()),
        NotebookEntryKind::Name {
            backing_cell: None, ..
        } => None,
        NotebookEntryKind::Cell { value, .. } => Some(value.clone()),
        NotebookEntryKind::Table { .. } => None,
    };

    let edit_target = entry_edit_target(&entry);
    let editing = RwSignal::new(false);
    let feedback = RwSignal::new(EntryFeedback::None);

    // Read-only entries (no edit target) render exactly as N1 did.
    let Some(target) = edit_target else {
        let value_view = value_projection.as_ref().map(|value| {
            view! {
                <div class="dtc-notebook-entry__value">{render_value(value)}</div>
            }
            .into_any()
        });
        return view! {
            <div class="dtc-notebook-entry" data-entry-kind=kind_label>
                <div class="dtc-notebook-entry__head">
                    <span class="dtc-notebook-entry__glyph">{glyph}</span>
                    <span class="dtc-notebook-entry__name">{display_name}</span>
                    <span class=format!("dtc-notebook-badge {badge_class}")>{badge_label}</span>
                </div>
                {value_view}
            </div>
        }
        .into_any();
    };

    let open_editor = move |_| {
        // Reopening from a clean read state starts with no stale feedback; a
        // rejection that reopened the editor already set its own feedback.
        if !editing.get_untracked() {
            feedback.set(EntryFeedback::None);
            editing.set(true);
        }
    };
    let open_on_enter = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Enter" && !editing.get_untracked() {
            ev.prevent_default();
            feedback.set(EntryFeedback::None);
            editing.set(true);
        }
    };

    let editor_target = target.clone();
    let editor_dispatch = dispatch.clone();
    let body = move || {
        if editing.get() {
            let target = editor_target.clone();
            view! {
                <CellEntryEditor
                    grid=target.grid
                    row=target.row
                    col=target.col
                    initial_text=target.initial_text
                    dispatch=editor_dispatch.clone()
                    editing=editing
                    feedback=feedback
                />
            }
            .into_any()
        } else {
            value_projection
                .as_ref()
                .map(|value| {
                    view! {
                        <div class="dtc-notebook-entry__value">{render_value(value)}</div>
                    }
                    .into_any()
                })
                .unwrap_or_else(|| ().into_any())
        }
    };

    let feedback_view = move || feedback_below_editor(feedback.get());

    view! {
        <div class="dtc-notebook-entry dtc-notebook-entry--editable" data-entry-kind=kind_label>
            <div
                class="dtc-notebook-entry__head"
                role="button"
                tabindex="0"
                on:click=open_editor
                on:keydown=open_on_enter
            >
                <span class="dtc-notebook-entry__glyph">{glyph}</span>
                <span class="dtc-notebook-entry__name">{display_name}</span>
                <span class=format!("dtc-notebook-badge {badge_class}")>{badge_label}</span>
            </div>
            {body}
            {feedback_view}
        </div>
    }
    .into_any()
}

/// Render the post-commit feedback beneath the editor (§B.3 step 3, §B.5): a
/// typed rejection lists its diagnostics through the shared [`EntryDiagnostics`]
/// component; an unresolved-name success shows the `#NAME?` note; a generic
/// engine error shows its message; a clean commit shows nothing.
fn feedback_below_editor(feedback: EntryFeedback) -> AnyView {
    match feedback {
        EntryFeedback::None => ().into_any(),
        EntryFeedback::Rejected { diagnostics } => {
            view! { <EntryDiagnostics diagnostics=diagnostics /> }.into_any()
        }
        EntryFeedback::UnresolvedNames { note } => view! {
            <div class="dtc-entry-note" role="status">{note}</div>
        }
        .into_any(),
        EntryFeedback::OtherError { message } => view! {
            <div class="dtc-entry-note dtc-entry-note--error" role="alert">{message}</div>
        }
        .into_any(),
    }
}

fn entry_kind_label(kind: &NotebookEntryKind) -> &'static str {
    match kind {
        NotebookEntryKind::Name { .. } => "name",
        NotebookEntryKind::Cell { .. } => "cell",
        NotebookEntryKind::Table { .. } => "table",
    }
}

/// One name-rail row (§B.2 anatomy, §B.3 Name loop's rename/delete actions):
/// name text, static value or dynamic formula chip, plus N3's rename-inline
/// and delete-with-confirm actions.
///
/// Rename (acceptance 3): clicking the name's own text swaps it for
/// [`RenameNameForm`], which dispatches **only** `RenameDefinedName` on
/// commit (`components/name_form.rs`'s own scope; this component never
/// constructs the rename intent itself).
///
/// Delete-with-name confirm (§B.3 step 5: "deleting the *entry* ... additionally
/// deletes its covering name (`DeleteDefinedName`) after a confirm"): the
/// gutter's delete action opens an inline confirm naming the target; accepting
/// dispatches exactly one `DeleteDefinedName`.
#[component]
fn NameRailRow(name: DefinedNameProjection, dispatch: Arc<dyn Dispatcher>) -> impl IntoView {
    let target_text = match &name.target {
        DefinedNameTargetProjection::Static(rect) => {
            format!("R{}C{}", rect.top_row, rect.left_col)
        }
        DefinedNameTargetProjection::Dynamic { source_text } => source_text.clone(),
    };
    let renaming = RwSignal::new(false);
    let confirming_delete = RwSignal::new(false);
    let scope = name.scope.clone();
    let name_text = name.name.clone();

    let start_rename = move |_| renaming.set(true);
    let start_delete_confirm = move |_| confirming_delete.set(true);
    let cancel_delete_confirm = move |_| confirming_delete.set(false);

    let confirm_delete = {
        let dispatch = dispatch.clone();
        let scope = scope.clone();
        let name_text = name_text.clone();
        move |_| {
            let _ = dispatch.dispatch(WorkspaceIntent::DeleteDefinedName {
                scope: scope.clone(),
                name: name_text.clone(),
            });
            confirming_delete.set(false);
        }
    };

    let body = {
        let scope = scope.clone();
        let name_text = name_text.clone();
        let dispatch = dispatch.clone();
        let target_text = target_text.clone();
        move || {
            if renaming.get() {
                view! {
                    <RenameNameForm
                        scope=scope.clone()
                        current_name=name_text.clone()
                        dispatch=dispatch.clone()
                        editing=renaming
                    />
                }
                .into_any()
            } else {
                view! {
                    <div class="dtc-notebook-rail-row">
                        <span
                            class="dtc-notebook-rail-row__name"
                            role="button"
                            tabindex="0"
                            on:click=start_rename
                        >
                            {name_text.clone()}
                        </span>
                        <span class="dtc-notebook-rail-row__target">{target_text.clone()}</span>
                        <button
                            type="button"
                            class="dtc-notebook-rail-row__delete"
                            aria-label="Delete this name"
                            on:click=start_delete_confirm
                        >
                            "\u{2715}"
                        </button>
                    </div>
                }
                .into_any()
            }
        }
    };

    let confirm_view = {
        let name_text = name_text.clone();
        move || {
            if !confirming_delete.get() {
                return ().into_any();
            }
            view! {
                <div class="dtc-notebook-rail-row__confirm" role="alertdialog">
                    <span>{format!("Delete '{name_text}'?")}</span>
                    <button type="button" on:click=confirm_delete.clone()>"Delete"</button>
                    <button type="button" on:click=cancel_delete_confirm>"Cancel"</button>
                </div>
            }
            .into_any()
        }
    };

    view! {
        {body}
        {confirm_view}
    }
}

const NOTEBOOK_CSS: &str = r#"
.dtc-notebook {
  display: flex; height: 100%; min-height: 0;
  background: var(--dtc-surface); color: var(--dtc-text);
  font: 13px/1.4 var(--dtc-font, system-ui, sans-serif);
}
.dtc-notebook__entries { flex: 1; overflow: auto; padding: 8px 12px; display: flex; flex-direction: column; gap: 8px; }
.dtc-notebook__rail {
  width: 220px; flex-shrink: 0; overflow: auto; padding: 8px 12px;
  border-left: 1px solid var(--dtc-border-muted); background: var(--dtc-surface-subtle);
}
.dtc-notebook__rail-title { font-size: 11px; font-weight: 700; letter-spacing: 0.04em; color: var(--dtc-text-subtle); margin-bottom: 6px; }
.dtc-notebook-rail-row { display: flex; align-items: center; justify-content: space-between; gap: 8px; padding: 2px 0; }
.dtc-notebook-rail-row__name { font-weight: 600; cursor: pointer; }
.dtc-notebook-rail-row__target { color: var(--dtc-text-muted); font-family: ui-monospace, monospace; font-size: 11px; }
.dtc-notebook-rail-row__delete {
  border: none; background: none; color: var(--dtc-text-subtle); cursor: pointer;
  font-size: 11px; padding: 0 2px;
}
.dtc-notebook-rail-row__confirm {
  display: flex; align-items: center; gap: 6px; padding: 4px 6px; margin: 2px 0;
  background: var(--dtc-warning-surface, #fff4e0); color: var(--dtc-warning-text, #8a5a00);
  border-radius: 5px; font-size: 12px;
}
.dtc-notebook__new-name { margin-top: 4px; }
.dtc-notebook__new-name-button {
  padding: 4px 10px; border-radius: 6px; border: 1px dashed var(--dtc-border);
  background: var(--dtc-surface); color: var(--dtc-text-subtle); cursor: pointer;
  font-size: 12px;
}

.dtc-notebook-entry {
  padding: 6px 10px; background: var(--dtc-surface-panel);
  border: 1px solid var(--dtc-border); border-radius: 7px;
}
.dtc-notebook-entry__head { display: flex; align-items: center; gap: 8px; }
.dtc-notebook-entry__glyph { width: 1.2em; text-align: center; color: var(--dtc-text-subtle); }
.dtc-notebook-entry__name { flex: 1; font-weight: 600; }
.dtc-notebook-entry__value { margin-top: 4px; font-variant-numeric: tabular-nums; }
.dtc-notebook-badge {
  padding: 1px 8px; border-radius: 9px; font-size: 11px; border: 1px solid transparent;
}
.dtc-notebook-badge--input { background: var(--dtc-accent-surface, #e6f0ff); color: var(--dtc-accent-text, #1a4fa0); }
.dtc-notebook-badge--free { background: var(--dtc-surface-subtle); color: var(--dtc-text-subtle); }
.dtc-notebook-badge--intermediate { background: var(--dtc-warning-surface, #fff4e0); color: var(--dtc-warning-text, #8a5a00); }
.dtc-notebook-badge--output { background: var(--dtc-success-surface, #e6f7ec); color: var(--dtc-success-text, #17703c); }
.dtc-notebook-badge--empty { background: var(--dtc-surface-subtle); color: var(--dtc-text-subtle); }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::{
        GridEditabilityProjection, GridOverlayBundle, GridOverlayRect, GridProjection,
        GridRectProjection,
    };

    fn name(
        name_text: &str,
        target: DefinedNameTargetProjection,
        is_dynamic: bool,
    ) -> DefinedNameProjection {
        DefinedNameProjection {
            scope: DefinedNameScopeProjection::Sheet(NodeId::new("Sheet1")),
            name: name_text.to_string(),
            target,
            is_dynamic,
        }
    }

    fn static_rect(row: u32, col: u32) -> GridRectProjection {
        GridRectProjection {
            top_row: row,
            left_col: col,
            bottom_row: row,
            right_col: col,
        }
    }

    fn authored_cell(
        row: u32,
        col: u32,
        kind: GridAuthoredKindProjection,
        literal_text: Option<&str>,
        source_text: Option<&str>,
    ) -> GridAuthoredCellProjection {
        GridAuthoredCellProjection {
            row,
            col,
            kind,
            literal_text: literal_text.map(str::to_string),
            source_text: source_text.map(str::to_string),
            editability: GridEditabilityProjection::Editable,
        }
    }

    fn value_number(display: &str) -> NodeValueProjection {
        NodeValueProjection::Number {
            raw: display.to_string(),
            display: display.to_string(),
        }
    }

    fn cell_projection(
        row: u32,
        col: u32,
        value: NodeValueProjection,
        authored: Option<GridAuthoredCellProjection>,
    ) -> GridCellProjection {
        GridCellProjection {
            row,
            col,
            value,
            value_epoch: 1,
            authored,
            provenance: None,
        }
    }

    fn table_overlay(
        name: &str,
        top_row: u32,
        left_col: u32,
        bottom_row: u32,
        right_col: u32,
    ) -> GridTableOverlayDescriptor {
        GridTableOverlayDescriptor {
            table_id: format!("tbl:{name}"),
            table_name: name.to_string(),
            table_node_key: None,
            table_range: GridOverlayRect {
                top_row,
                left_col,
                bottom_row,
                right_col,
                clipped_top: false,
                clipped_left: false,
                clipped_bottom: false,
                clipped_right: false,
            },
            header_rect: None,
            totals_rect: None,
            columns: Vec::new(),
        }
    }

    fn grid_projection(
        cells: Vec<GridCellProjection>,
        tables: Vec<GridTableOverlayDescriptor>,
    ) -> GridProjection {
        GridProjection {
            grid_node_key: dnatreecalc_skin_framework::NodeKey::new("k:Sheet1"),
            grid_node_id: NodeId::new("Sheet1"),
            grid_id: "grid:Sheet1".to_string(),
            max_rows: 1000,
            max_cols: 100,
            cells,
            projection_epoch: 1,
            overlays: GridOverlayBundle {
                tables,
                spills: Vec::new(),
                merged: Vec::new(),
            },
            overlay_epoch: 1,
            differential_clean: true,
            authored_epoch: 1,
        }
    }

    /// N1 acceptance #1: 2 names + 1 uncovered cell + 1 table -> 4 entries,
    /// in manifest-then-default (names, then cells, then tables) order.
    #[test]
    fn derive_entries_returns_four_entries_in_default_order() {
        let mut ws = WorkspaceState::default();

        // Two names: `rate` (literal, static, row 1 col 1) and `monthly`
        // (dynamic formula, no backing cell in this fixture).
        ws.defined_names.entries.push(name(
            "rate",
            DefinedNameTargetProjection::Static(static_rect(1, 1)),
            false,
        ));
        ws.defined_names.entries.push(name(
            "monthly",
            DefinedNameTargetProjection::Dynamic {
                source_text: "=PMT(rate/12,360,-1)".to_string(),
            },
            true,
        ));

        // Grid: row 1 col 1 backs `rate` (covered -> not an "Other cells"
        // entry); row 2 col 1 is an uncovered authored formula cell; rows
        // 3..4 col 1..2 sit under a table overlay (also excluded).
        let rate_cell = authored_cell(
            1,
            1,
            GridAuthoredKindProjection::Literal,
            Some("0.065"),
            None,
        );
        let uncovered_cell = authored_cell(
            2,
            1,
            GridAuthoredKindProjection::Formula,
            None,
            Some("=1+1"),
        );
        let table_cell = authored_cell(
            3,
            1,
            GridAuthoredKindProjection::Literal,
            Some("base"),
            None,
        );
        let cells = vec![
            cell_projection(1, 1, value_number("0.065"), Some(rate_cell)),
            cell_projection(2, 1, value_number("2"), Some(uncovered_cell)),
            cell_projection(
                3,
                1,
                NodeValueProjection::Text("base".to_string()),
                Some(table_cell),
            ),
        ];
        let tables = vec![table_overlay("Scenarios", 3, 1, 4, 2)];
        ws.grids
            .insert(NodeId::new("Sheet1"), grid_projection(cells, tables));

        let entries = derive_entries(&ws);
        assert_eq!(entries.len(), 4, "2 names + 1 uncovered cell + 1 table");

        // Default order: names first (catalog order), then uncovered cells,
        // then tables.
        assert_eq!(entries[0].display_name, "rate");
        assert!(matches!(entries[0].kind, NotebookEntryKind::Name { .. }));
        assert_eq!(entries[1].display_name, "monthly");
        assert!(matches!(entries[1].kind, NotebookEntryKind::Name { .. }));
        assert!(matches!(entries[2].kind, NotebookEntryKind::Cell { .. }));
        assert_eq!(entries[2].display_name, "Sheet1!R2C1");
        assert!(matches!(entries[3].kind, NotebookEntryKind::Table { .. }));
        assert_eq!(entries[3].display_name, "Scenarios");
    }

    /// N1 acceptance #2 (amended): the badge is `entry_classification`'s
    /// output from the entry's own authored metadata, asserted per entry
    /// kind — never `WorkspaceState::node_classification` (no `NodeKey`
    /// exists for a grid-backed entry).
    #[test]
    fn entry_classification_maps_authored_metadata_per_kind() {
        // Empty / RichStub -> Empty, regardless of a covering name.
        let empty = authored_cell(1, 1, GridAuthoredKindProjection::Empty, None, None);
        assert_eq!(
            entry_classification(&empty, None),
            NodeClassification::Empty
        );
        let rich_stub = authored_cell(1, 1, GridAuthoredKindProjection::RichStub, None, None);
        assert_eq!(
            entry_classification(&rich_stub, None),
            NodeClassification::Empty
        );

        // Literal, no name -> FreeValue (an unconsumed constant).
        let literal = authored_cell(1, 1, GridAuthoredKindProjection::Literal, Some("1"), None);
        assert_eq!(
            entry_classification(&literal, None),
            NodeClassification::FreeValue
        );

        // Literal, backed by a (static) name -> Input.
        let static_name = name(
            "rate",
            DefinedNameTargetProjection::Static(static_rect(1, 1)),
            false,
        );
        assert_eq!(
            entry_classification(&literal, Some(&static_name)),
            NodeClassification::Input
        );

        // Formula, no name -> Output (an unconsumed terminal formula).
        let formula = authored_cell(
            2,
            1,
            GridAuthoredKindProjection::Formula,
            None,
            Some("=1+1"),
        );
        assert_eq!(
            entry_classification(&formula, None),
            NodeClassification::Output
        );

        // Formula, backed by a dynamic name -> Intermediate.
        let dynamic_name = name(
            "monthly",
            DefinedNameTargetProjection::Dynamic {
                source_text: "=1+1".to_string(),
            },
            true,
        );
        assert_eq!(
            entry_classification(&formula, Some(&dynamic_name)),
            NodeClassification::Intermediate
        );

        // Formula, backed by a NON-dynamic name (e.g. a static name whose
        // resolved backing cell happens to be a formula) -> still Output:
        // only `is_dynamic` flips the axis, per the mapping table.
        let static_name_over_formula = name(
            "static_over_formula",
            DefinedNameTargetProjection::Static(static_rect(2, 1)),
            false,
        );
        assert_eq!(
            entry_classification(&formula, Some(&static_name_over_formula)),
            NodeClassification::Output
        );
    }

    /// N1 acceptance #3: a spill result (an `Array` value) renders the
    /// collapsed `{R×C array}` chip via the shared `render_value`, never an
    /// editable surface — asserted at the entry-model level (no dispatcher
    /// exists in this skin at all, so "never editable" is structural, not a
    /// behavior to probe at runtime).
    #[test]
    fn spill_entry_carries_array_value_for_chip_render() {
        let mut ws = WorkspaceState::default();
        let spill_cell = authored_cell(
            1,
            1,
            GridAuthoredKindProjection::Formula,
            None,
            Some("=SEQUENCE(2,2)"),
        );
        let array_value = NodeValueProjection::Array {
            rows: 2,
            cols: 2,
            cells: vec![
                vec![value_number("1"), value_number("2")],
                vec![value_number("3"), value_number("4")],
            ],
        };
        let cells = vec![cell_projection(1, 1, array_value.clone(), Some(spill_cell))];
        ws.grids
            .insert(NodeId::new("Sheet1"), grid_projection(cells, Vec::new()));

        let entries = derive_entries(&ws);
        assert_eq!(entries.len(), 1);
        let NotebookEntryKind::Cell { value, .. } = &entries[0].kind else {
            panic!("expected a Cell entry");
        };
        assert_eq!(
            value, &array_value,
            "the entry model carries the Array value verbatim for render_value's {{R×C array}} chip"
        );
        // NotebookEntryKind has no field through which a value could be
        // written back — the entry model is read-only by construction.
    }

    #[test]
    fn derive_entries_excludes_cells_covered_by_a_workbook_scoped_name() {
        let mut ws = WorkspaceState::default();
        ws.defined_names.entries.push(DefinedNameProjection {
            scope: DefinedNameScopeProjection::Workbook,
            name: "total".to_string(),
            target: DefinedNameTargetProjection::Static(static_rect(5, 5)),
            is_dynamic: false,
        });
        let covered = authored_cell(5, 5, GridAuthoredKindProjection::Literal, Some("1"), None);
        let cells = vec![cell_projection(5, 5, value_number("1"), Some(covered))];
        ws.grids
            .insert(NodeId::new("Sheet1"), grid_projection(cells, Vec::new()));

        let entries = derive_entries(&ws);
        // Only the name entry — the covered cell must not double as an
        // "Other cells" entry.
        assert_eq!(entries.len(), 1);
        assert!(matches!(entries[0].kind, NotebookEntryKind::Name { .. }));
    }

    #[test]
    fn derive_entries_skips_empty_authored_cells() {
        let mut ws = WorkspaceState::default();
        let empty = authored_cell(1, 1, GridAuthoredKindProjection::Empty, None, None);
        let cells = vec![cell_projection(
            1,
            1,
            NodeValueProjection::Empty,
            Some(empty),
        )];
        ws.grids
            .insert(NodeId::new("Sheet1"), grid_projection(cells, Vec::new()));

        assert_eq!(derive_entries(&ws).len(), 0);
    }

    /// N2: an editable literal-backed static name resolves an edit target at
    /// its backing cell, seeded from the literal text — the `EnterGridCell`
    /// commit addresses that cell.
    #[test]
    fn entry_edit_target_for_literal_name_addresses_backing_cell() {
        let mut ws = WorkspaceState::default();
        ws.defined_names.entries.push(name(
            "rate",
            DefinedNameTargetProjection::Static(static_rect(4, 2)),
            false,
        ));
        let backing = authored_cell(
            4,
            2,
            GridAuthoredKindProjection::Literal,
            Some("0.065"),
            None,
        );
        let cells = vec![cell_projection(4, 2, value_number("0.065"), Some(backing))];
        ws.grids
            .insert(NodeId::new("Sheet1"), grid_projection(cells, Vec::new()));

        let entries = derive_entries(&ws);
        let target = entry_edit_target(&entries[0]).expect("a literal name is editable");
        assert_eq!(target.grid, NodeId::new("Sheet1"));
        assert_eq!((target.row, target.col), (4, 2));
        assert_eq!(target.initial_text, "0.065");
    }

    /// N2: an uncovered formula cell resolves an edit target seeded from its
    /// source text.
    #[test]
    fn entry_edit_target_for_uncovered_formula_cell_seeds_source_text() {
        let mut ws = WorkspaceState::default();
        let cell = authored_cell(
            2,
            1,
            GridAuthoredKindProjection::Formula,
            None,
            Some("=A1*3"),
        );
        let cells = vec![cell_projection(2, 1, value_number("30"), Some(cell))];
        ws.grids
            .insert(NodeId::new("Sheet1"), grid_projection(cells, Vec::new()));

        let entries = derive_entries(&ws);
        let target = entry_edit_target(&entries[0]).expect("an uncovered formula cell is editable");
        assert_eq!((target.row, target.col), (2, 1));
        assert_eq!(target.initial_text, "=A1*3");
    }

    /// N2: a dynamic name has no cell-edit target (its body is a
    /// `SetDefinedName` re-bind, N3), and a table entry is never editable here.
    #[test]
    fn entry_edit_target_is_none_for_dynamic_name_and_table() {
        let mut ws = WorkspaceState::default();
        ws.defined_names.entries.push(name(
            "monthly",
            DefinedNameTargetProjection::Dynamic {
                source_text: "=PMT(rate/12,360,-1)".to_string(),
            },
            true,
        ));
        let cells = vec![cell_projection(
            3,
            1,
            NodeValueProjection::Text("base".to_string()),
            Some(authored_cell(
                3,
                1,
                GridAuthoredKindProjection::Literal,
                Some("base"),
                None,
            )),
        )];
        let tables = vec![table_overlay("Scenarios", 3, 1, 4, 2)];
        ws.grids
            .insert(NodeId::new("Sheet1"), grid_projection(cells, tables));

        let entries = derive_entries(&ws);
        let dynamic = entries
            .iter()
            .find(|e| matches!(e.kind, NotebookEntryKind::Name { .. }))
            .expect("a name entry");
        assert_eq!(
            entry_edit_target(dynamic),
            None,
            "dynamic name: no cell edit"
        );
        let table = entries
            .iter()
            .find(|e| matches!(e.kind, NotebookEntryKind::Table { .. }))
            .expect("a table entry");
        assert_eq!(
            entry_edit_target(table),
            None,
            "table: never editable in N2"
        );
    }

    /// N2: a non-`Editable` cell (e.g. a spill display member) renders
    /// read-only — the classifier is the backstop, no edit target (§B.8).
    #[test]
    fn entry_edit_target_is_none_for_non_editable_cell() {
        let mut ws = WorkspaceState::default();
        let spill = GridAuthoredCellProjection {
            row: 4,
            col: 2,
            kind: GridAuthoredKindProjection::Formula,
            literal_text: None,
            source_text: Some("=SEQUENCE(2)".to_string()),
            editability: GridEditabilityProjection::SpillDisplay {
                anchor: dnatreecalc_skin_framework::GridCellRefProjection { row: 3, col: 2 },
            },
        };
        let cells = vec![cell_projection(4, 2, value_number("2"), Some(spill))];
        ws.grids
            .insert(NodeId::new("Sheet1"), grid_projection(cells, Vec::new()));

        let entries = derive_entries(&ws);
        assert_eq!(
            entry_edit_target(&entries[0]),
            None,
            "a spill-display cell is read-only by classifier"
        );
    }

    #[test]
    fn entry_glyph_reflects_authored_kind() {
        let literal_cell = NotebookEntry {
            display_name: "x".to_string(),
            kind: NotebookEntryKind::Cell {
                grid: NodeId::new("Sheet1"),
                authored: authored_cell(1, 1, GridAuthoredKindProjection::Literal, Some("1"), None),
                value: value_number("1"),
            },
            classification: NodeClassification::FreeValue,
        };
        assert_eq!(entry_glyph(&literal_cell), "●");

        let formula_cell = NotebookEntry {
            display_name: "y".to_string(),
            kind: NotebookEntryKind::Cell {
                grid: NodeId::new("Sheet1"),
                authored: authored_cell(
                    1,
                    1,
                    GridAuthoredKindProjection::Formula,
                    None,
                    Some("=1"),
                ),
                value: value_number("1"),
            },
            classification: NodeClassification::Output,
        };
        assert_eq!(entry_glyph(&formula_cell), "ƒ");

        let table_entry = NotebookEntry {
            display_name: "Scenarios".to_string(),
            kind: NotebookEntryKind::Table {
                grid: NodeId::new("Sheet1"),
                table: table_overlay("Scenarios", 1, 1, 2, 2),
            },
            classification: NodeClassification::Intermediate,
        };
        assert_eq!(entry_glyph(&table_entry), "▦");
    }
}
