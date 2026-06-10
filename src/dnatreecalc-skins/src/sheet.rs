//! SHEET — the Excel edit loop on a tree.
//!
//! Sheet shows **values by default**: every visible node is a value-first row
//! (name + big rendered value), formulas appear small under the name only when
//! the `show_formulas` toggle is on or the row is selected. A real **formula
//! bar** sits at the top: it reads the active selection detail (node or table
//! cell), edits in a modeless buffer, and commits on Enter through the closed
//! intent surface. **Tables expand in place** as real grids with header, body,
//! and totals rows.
//!
//! Spine conformance:
//! - **one grammar** — Enter (outside inputs) focuses the formula bar, `/`
//!   opens the Name-Box, Escape unwinds (resolved through
//!   [`KeybindingRegistry`]); bare keys never fire while typing.
//! - **one styling** — `calc_state` is the only saturated channel
//!   (`calc_state_class` + `dtc-calc-dot`), provenance is structural
//!   (`provenance_tint`), SELECTED/EDITING is the 1-bit border
//!   (`selection_mode_class`).
//! - **one continuity** — selection rides the shared dispatcher path; the
//!   embedded Lens/Console/Name-Box are the shared [`crate::spine_widgets`].
//!
//! Everything Sheet shows is read from the published projection; every change
//! goes through `WorkspaceIntent`. Sheet never parses formula text — the `=`
//! classification of content belongs to the host — and never computes a value.

use std::sync::Arc;

use dnatreecalc_skin_framework::{
    ATLAS_SPINE_CSS, ActiveSelectionDetailProjection, Dispatcher,
    FormulaReferenceInsertionProjection, FormulaReferenceInsertionTarget, KeyChord,
    KeybindingRegistry, NodeId, NodeKey, NodeView, SharedStateOrigin, SkinCapabilities,
    SkinCategory, SkinContext, SkinHandle, SkinId, SkinManifest, SkinState, SkinVerb,
    TableCellEditabilityProjection, TableCellProjection, TableCellRegionProjection,
    TableCellSelection, TableProjection, WorkspaceDeltaChange, WorkspaceIntent, WorkspaceSkin,
    WorkspaceState, calc_state_class, provenance_tint, selection_mode_class,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::spine_widgets::{
    ConsoleBar, NameBoxBar, NodeInspector, SPINE_WIDGETS_CSS, commit_content_edit,
    event_target_is_interactive, event_target_is_text_entry,
};
use crate::value_render::{render_value, value_text};

pub const SHEET_ID: SkinId = SkinId::new("sheet");

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetState {
    /// Show authored formulas under every row (values stay primary).
    pub show_formulas: bool,
}

impl SkinState for SheetState {
    fn schema_version() -> u32 {
        1
    }
}

#[derive(Default)]
pub struct SheetLens;

impl SheetLens {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSkin for SheetLens {
    type State = SheetState;

    fn id(&self) -> SkinId {
        SHEET_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Sheet",
            description: "Excel edit loop: formula bar, values by default, tables in place.",
            category: SkinCategory::Editor,
            version: "0.1.0",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities {
            supports_multi_select: false,
            supports_inline_formula_edit: true,
            supports_meta_node_display: false,
            renders_arrays_inline: true,
            renders_table_values: true,
            allowed_slots: None,
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        crate::spine_widgets::stamp_active_lens(cx.shared, SHEET_ID, cx.slot);
        SkinHandle::new(view! { <SheetView cx=cx /> }.into_any())
    }
}

// ---------------------------------------------------------------------------
// Pure helpers (unit-tested)
// ---------------------------------------------------------------------------

/// Name-label for the formula bar: the node's dotted path, or
/// `Table!row:col` for table cells (`totals` when the totals row is active).
fn selection_label(detail: &ActiveSelectionDetailProjection) -> String {
    match detail {
        ActiveSelectionDetailProjection::Node(node) => node.node.to_string(),
        ActiveSelectionDetailProjection::TableCell(cell) => {
            let row = cell.row_id.as_deref().unwrap_or("totals");
            format!("{}!{}:{}", cell.table_name, row, cell.column_id)
        }
    }
}

/// What the formula bar buffer holds for the active selection. The projection
/// carries no per-cell content text for direct-input cells, so the bar shows
/// the rendered value text; on commit the typed text becomes the new content
/// (the host owns the `=` classification).
fn formula_bar_buffer(detail: &ActiveSelectionDetailProjection) -> String {
    match detail {
        ActiveSelectionDetailProjection::Node(node) => node.content_text.clone(),
        ActiveSelectionDetailProjection::TableCell(cell) => match cell.editability {
            TableCellEditabilityProjection::DirectInput
            | TableCellEditabilityProjection::ReadOnly => value_text(&cell.value),
            TableCellEditabilityProjection::FormulaBacked
            | TableCellEditabilityProjection::TotalsFormula => cell
                .formula
                .as_ref()
                .map(|formula| formula.formula_text.clone())
                .unwrap_or_default(),
        },
    }
}

/// Structural badge next to the bar so the user knows what an edit means.
fn editability_badge(detail: &ActiveSelectionDetailProjection) -> Option<&'static str> {
    match detail {
        ActiveSelectionDetailProjection::Node(_) => None,
        ActiveSelectionDetailProjection::TableCell(cell) => match cell.editability {
            TableCellEditabilityProjection::DirectInput => None,
            TableCellEditabilityProjection::FormulaBacked => Some("column formula"),
            TableCellEditabilityProjection::TotalsFormula => Some("totals"),
            TableCellEditabilityProjection::ReadOnly => Some("read-only"),
        },
    }
}

/// The bar input is disabled when nothing is selected or the active table
/// cell is read-only.
fn bar_is_read_only(detail: Option<&ActiveSelectionDetailProjection>) -> bool {
    match detail {
        None => true,
        Some(ActiveSelectionDetailProjection::Node(_)) => false,
        Some(ActiveSelectionDetailProjection::TableCell(cell)) => {
            cell.editability == TableCellEditabilityProjection::ReadOnly
        }
    }
}

/// Map an Enter in the formula bar to its table intent. Returns `None` for
/// node selections (the caller routes those through the shared
/// [`commit_content_edit`] recalc-mode path) and for read-only cells. The
/// typed text passes through VERBATIM — the host owns classification.
fn sheet_commit_intent(
    detail: &ActiveSelectionDetailProjection,
    content: String,
) -> Option<WorkspaceIntent> {
    let ActiveSelectionDetailProjection::TableCell(cell) = detail else {
        // Node edits stay on the shared recalc-mode commit path.
        return None;
    };
    match (cell.region, cell.editability) {
        (TableCellRegionProjection::Body, TableCellEditabilityProjection::DirectInput) => {
            cell.row_id
                .clone()
                .map(|row_id| WorkspaceIntent::EditTableCell {
                    table: cell.table.clone(),
                    row_id,
                    column_id: cell.column_id.clone(),
                    content,
                })
        }
        (TableCellRegionProjection::Body, TableCellEditabilityProjection::FormulaBacked) => {
            Some(WorkspaceIntent::EditTableColumnFormula {
                table: cell.table.clone(),
                column_id: cell.column_id.clone(),
                formula_text: content,
            })
        }
        (_, TableCellEditabilityProjection::TotalsFormula) => {
            Some(WorkspaceIntent::SetTableTotalsFormula {
                table: cell.table.clone(),
                column_id: cell.column_id.clone(),
                formula_text: content,
            })
        }
        _ => None,
    }
}

/// Clamp a raw DOM caret offset (`selection_start`) to the buffer's char
/// count. `None` (e.g. unmounted input) lands at the end of the buffer.
fn caret_char_offset(raw: Option<u32>, buffer: &str) -> usize {
    let len = buffer.chars().count();
    raw.map_or(len, |offset| (offset as usize).min(len))
}

/// Point-mode reference insert, armed-and-explicit.
///
/// NOTE: `InsertFormulaReference` COMMITS the recomposed formula through the
/// host — it is not a pure compose-text service. A compose-without-commit
/// OxFml seam is a logged follow-up; the armed mode makes the commit a
/// deliberate user action (arm, then click the target node).
fn insert_reference_intent(
    edited: NodeKey,
    buffer: String,
    caret: usize,
    clicked: NodeKey,
) -> WorkspaceIntent {
    WorkspaceIntent::InsertFormulaReference {
        node: edited,
        current_formula_text: buffer,
        replacement_start: caret,
        replacement_len: 0,
        target: FormulaReferenceInsertionTarget::Node(clicked),
    }
}

/// Find the host's recomposed-formula fact in an accepted receipt's delta.
fn first_inserted_reference(
    changes: &[WorkspaceDeltaChange],
) -> Option<&FormulaReferenceInsertionProjection> {
    changes.iter().find_map(|change| match change {
        WorkspaceDeltaChange::FormulaReferenceInserted(insertion) => Some(insertion),
        _ => None,
    })
}

/// Select a table cell (the closed intent carries the table's dotted path and
/// the cell's stable row/column ids; `row_id` is `None` for totals cells).
fn select_cell_intent(table: &NodeId, cell: &TableCellProjection) -> WorkspaceIntent {
    WorkspaceIntent::SelectTableCell {
        table: table.clone(),
        row_id: cell.row_id.clone(),
        column_id: cell.column_id.clone(),
    }
}

/// Append a row with a lens-generated id; values stay empty so the host
/// applies its own initial-content policy.
fn add_row_intent(table: &NodeId, projection: &TableProjection) -> WorkspaceIntent {
    WorkspaceIntent::AddTableRow {
        table: table.clone(),
        row_id: format!("row:new{}", projection.rows.len() + 1),
        values: Vec::new(),
    }
}

/// Append a constant column with a lens-generated id and display name.
fn add_column_intent(table: &NodeId, projection: &TableProjection) -> WorkspaceIntent {
    let next = projection.columns.len() + 1;
    WorkspaceIntent::AddTableColumn {
        table: table.clone(),
        column_id: format!("col:new{next}"),
        name: format!("Column {next}"),
        values: Vec::new(),
    }
}

/// The value-first row population: every node that is not effectively meta
/// and is not a table anchor (tables render as in-place grids instead).
fn sheet_row_keys(workspace: &WorkspaceState) -> Vec<NodeKey> {
    workspace
        .key_order
        .iter()
        .filter(|key| {
            let Some(node) = workspace.node_by_key(key) else {
                return false;
            };
            node.table.is_none()
                && !workspace.tables.contains_key(&node.id)
                && !workspace.is_effective_meta(key)
        })
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

#[component]
fn SheetView(cx: SkinContext<SheetState>) -> impl IntoView {
    let workspace = cx.workspace;
    let selection = cx.selection;
    let shared = cx.shared;
    let dispatch = cx.dispatch.clone();
    let state = cx.state.clone();
    let shared_origin = SharedStateOrigin::lens(SHEET_ID, cx.slot);

    // Inspector edit buffer (shared modeless pattern).
    let editing = RwSignal::new(false);
    let editor_text = RwSignal::new(String::new());
    let edit_ref = NodeRef::<leptos::html::Textarea>::new();
    // Formula bar.
    let bar_text = RwSignal::new(String::new());
    let bar_editing = RwSignal::new(false);
    let bar_ref = NodeRef::<leptos::html::Input>::new();
    let insert_armed = RwSignal::new(false);
    // Chrome.
    let name_box = RwSignal::new(Option::<String>::None);
    let rejection = RwSignal::new(Option::<String>::None);
    let section_ref = NodeRef::<leptos::html::Section>::new();

    let show_formulas = {
        let state = state.clone();
        Memo::new(move |_| state.with(|s| s.show_formulas))
    };

    // The active selection detail — node OR table cell — is the bar's truth.
    let detail = Memo::new(move |_| {
        selection.with(|sel| workspace.with(|ws| ws.active_selection_detail(sel)))
    });
    // Selection *identity*, distinct from projected contents — the bar buffer
    // must reset when the user selects a different node/cell, never when the
    // same target merely republishes.
    let selection_identity =
        Memo::new(move |_| selection.with(|sel| (sel.primary.clone(), sel.table_cell.clone())));

    // Reset the bar buffer (and disarm point-mode) only when the selection
    // target changes.
    Effect::new(move |_| {
        let _identity = selection_identity.get();
        let text = detail
            .get_untracked()
            .as_ref()
            .map(formula_bar_buffer)
            .unwrap_or_default();
        bar_text.set(text);
        bar_editing.set(false);
        insert_armed.set(false);
    });
    // While NOT editing, keep the bar synced to the published projection (so
    // undo/redo and external commits refresh it); while editing, never clobber.
    Effect::new(move |_| {
        if let Some(d) = detail.get()
            && !bar_editing.get_untracked()
        {
            bar_text.set(formula_bar_buffer(&d));
        }
    });

    // Inspector buffer: identical two-effect pattern over the primary node.
    let selected = Memo::new(move |_| {
        let primary = selection.with(|s| s.primary.clone());
        workspace.with(|ws| primary.as_ref().and_then(|id| ws.node(id).cloned()))
    });
    let selected_id = Memo::new(move |_| selection.with(|s| s.primary.clone()));
    Effect::new(move |_| {
        let _identity = selected_id.get();
        if let Some(node) = selected.get_untracked() {
            editor_text.set(node.content_text);
        } else {
            editor_text.set(String::new());
        }
        editing.set(false);
    });
    Effect::new(move |_| {
        if let Some(node) = selected.get()
            && !editing.get_untracked()
        {
            editor_text.set(node.content_text);
        }
    });

    // The one commit path for node content, shared with every lens.
    let commit_dispatch = dispatch.clone();
    let commit_origin = shared_origin.clone();
    let commit = move || {
        let Some(node) = selected.get_untracked() else {
            return;
        };
        let receipt = commit_content_edit(
            &commit_dispatch,
            shared,
            &commit_origin,
            node.id,
            editor_text.get_untracked(),
        );
        if receipt.accepted {
            editing.set(false);
        }
    };

    // Formula-bar commit: node selections route through the shared
    // recalc-mode path; table cells through their dedicated intents.
    let bar_dispatch = dispatch.clone();
    let bar_origin = shared_origin.clone();
    let bar_commit: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        let Some(detail_now) = detail.get_untracked() else {
            return;
        };
        let content = bar_text.get_untracked();
        if let ActiveSelectionDetailProjection::Node(node) = &detail_now {
            let receipt =
                commit_content_edit(&bar_dispatch, shared, &bar_origin, node.node.clone(), content);
            if receipt.accepted {
                bar_editing.set(false);
                rejection.set(None);
            } else {
                rejection.set(receipt.error.map(|error| error.to_string()));
            }
            return;
        }
        let Some(intent) = sheet_commit_intent(&detail_now, content) else {
            return; // read-only cell: nothing to commit
        };
        let receipt = bar_dispatch.dispatch(intent);
        if receipt.accepted {
            bar_editing.set(false);
            rejection.set(None);
        } else {
            rejection.set(receipt.error.map(|error| error.to_string()));
        }
    });

    let bar_commit_for_keys = bar_commit.clone();
    let bar_keydown = move |ev: leptos::ev::KeyboardEvent| {
        // Keys typed in the formula bar are the bar's alone (the contract).
        ev.stop_propagation();
        match ev.key().as_str() {
            "Enter" => {
                ev.prevent_default();
                (bar_commit_for_keys)();
            }
            "Escape" => {
                ev.prevent_default();
                bar_editing.set(false);
                insert_armed.set(false);
                let restored = detail
                    .get_untracked()
                    .as_ref()
                    .map(formula_bar_buffer)
                    .unwrap_or_default();
                bar_text.set(restored);
            }
            _ => {}
        }
    };

    let bar_label = Memo::new(move |_| {
        detail
            .get()
            .map(|d| selection_label(&d))
            .unwrap_or_else(|| "no selection".to_string())
    });
    let bar_badge = Memo::new(move |_| detail.get().as_ref().and_then(editability_badge));
    let bar_disabled = Memo::new(move |_| bar_is_read_only(detail.get().as_ref()));
    let arm_enabled = Memo::new(move |_| {
        matches!(detail.get(), Some(ActiveSelectionDetailProjection::Node(_)))
    });

    // Row clicks: plain select, or — when point-mode is armed — insert a
    // reference to the clicked node into the edited node's bar buffer at the
    // caret. See `insert_reference_intent` for the commit-semantics note.
    let click_dispatch = dispatch.clone();
    let on_node_click: Arc<dyn Fn(NodeKey, NodeId) + Send + Sync> =
        Arc::new(move |key: NodeKey, id: NodeId| {
            if !insert_armed.get_untracked() {
                click_dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id)));
                if let Some(section) = section_ref.get_untracked() {
                    let _ = section.focus();
                }
                return;
            }
            let Some(ActiveSelectionDetailProjection::Node(edited)) = detail.get_untracked()
            else {
                insert_armed.set(false);
                return;
            };
            let buffer = bar_text.get_untracked();
            let raw_caret = bar_ref
                .get_untracked()
                .and_then(|input| input.selection_start().ok().flatten());
            let caret = caret_char_offset(raw_caret, &buffer);
            let receipt = click_dispatch.dispatch(insert_reference_intent(
                edited.node_key.clone(),
                buffer,
                caret,
                key,
            ));
            if receipt.accepted {
                if let Some(insertion) = first_inserted_reference(&receipt.delta.changes) {
                    bar_text.set(insertion.updated_formula_text.clone());
                }
                rejection.set(None);
            } else {
                rejection.set(receipt.error.map(|error| error.to_string()));
            }
            insert_armed.set(false);
        });

    // Lens-local grammar, resolved through the one registry. Global verbs
    // (F9, Ctrl+Z/Y, Ctrl+N, lens switch, arrows) bubble to the shell.
    let grammar = KeybindingRegistry::universal();
    let root_keydown = move |ev: leptos::ev::KeyboardEvent| {
        // Bare-key grammar never fires while typing — the contract.
        if event_target_is_text_entry(&ev) || editing.get_untracked() || bar_editing.get_untracked()
        {
            return;
        }
        let command = ev.ctrl_key() || ev.meta_key();
        let chord = KeyChord::from_parts(ev.key(), command, ev.alt_key(), ev.shift_key());
        let Some(verb) = grammar.resolve(&chord) else {
            return;
        };
        match verb {
            SkinVerb::Commit => {
                // Enter on a focused button must activate the button.
                if event_target_is_interactive(&ev) {
                    return;
                }
                if detail.get_untracked().is_some() {
                    ev.prevent_default();
                    focus_formula_bar(bar_ref);
                }
            }
            SkinVerb::NameBox => {
                ev.prevent_default();
                name_box.set(Some(String::new()));
            }
            SkinVerb::Escape => {
                name_box.set(None);
                editing.set(false);
                bar_editing.set(false);
                insert_armed.set(false);
                let restored = detail
                    .get_untracked()
                    .as_ref()
                    .map(formula_bar_buffer)
                    .unwrap_or_default();
                bar_text.set(restored);
            }
            _ => {} // global verb: leave for the shell
        }
    };

    // Land keyboard focus on the sheet when the lens mounts, so the grammar
    // works without a preparatory click.
    Effect::new(move |_| {
        if let Some(section) = section_ref.get() {
            let _ = section.focus();
        }
    });

    let body_dispatch = dispatch.clone();
    let body_click = on_node_click.clone();
    let body = move || {
        let ws = workspace.get();
        let sel = selection.get();
        let show = show_formulas.get();
        let bar_editing_now = bar_editing.get();
        let armed = insert_armed.get();

        let rows = sheet_row_keys(&ws)
            .into_iter()
            .filter_map(|key| {
                let node = ws.node_by_key(&key)?.clone();
                let is_selected =
                    sel.table_cell.is_none() && sel.primary.as_ref() == Some(&node.id);
                Some(node_row_view(
                    node,
                    is_selected,
                    is_selected && bar_editing_now,
                    show,
                    armed,
                    &ws,
                    body_click.clone(),
                ))
            })
            .collect::<Vec<_>>();

        let tables = ws
            .tables
            .iter()
            .map(|(table_id, table)| {
                table_view(
                    table_id.clone(),
                    table,
                    sel.table_cell.as_ref(),
                    body_dispatch.clone(),
                    rejection,
                )
            })
            .collect::<Vec<_>>();

        view! {
            <div class="dtc-sheet-rows">{rows}</div>
            {tables}
        }
    };

    let css = format!("{ATLAS_SPINE_CSS}\n{SPINE_WIDGETS_CSS}\n{SHEET_CSS}");
    let toggle_state = state.clone();
    let console_dispatch = dispatch.clone();

    view! {
        <style>{css}</style>
        <section class="dtc-sheet" tabindex="0" node_ref=section_ref on:keydown=root_keydown>
            <div class="dtc-sheet-bar" aria-label="Formula bar">
                <span class="dtc-sheet-bar__label">{move || bar_label.get()}</span>
                {move || bar_badge.get().map(|badge| view! {
                    <span class="dtc-sheet-bar__badge">{badge}</span>
                }.into_any())}
                <input
                    class="dtc-sheet-bar__input"
                    node_ref=bar_ref
                    placeholder="content or =formula"
                    prop:value=move || bar_text.get()
                    prop:disabled=move || bar_disabled.get()
                    on:focus=move |_| bar_editing.set(true)
                    on:input=move |ev| {
                        bar_editing.set(true);
                        bar_text.set(event_target_value(&ev));
                    }
                    on:keydown=bar_keydown
                />
                <button
                    type="button"
                    class=move || if insert_armed.get() {
                        "dtc-sheet-bar__arm dtc-sheet-bar__arm--on"
                    } else {
                        "dtc-sheet-bar__arm"
                    }
                    prop:disabled=move || !arm_enabled.get()
                    title="Arm point-mode: click a node to insert a reference at the caret (commits the recomposed formula)"
                    on:click=move |_| {
                        if arm_enabled.get_untracked() {
                            insert_armed.update(|armed| *armed = !*armed);
                        } else {
                            insert_armed.set(false);
                        }
                    }
                >
                    {move || if insert_armed.get() { "Insert ref: click a node" } else { "Insert ref" }}
                </button>
            </div>
            <div class="dtc-sheet-toolbar">
                <button
                    type="button"
                    on:click=move |_| toggle_state.update(|s| s.show_formulas = !s.show_formulas)
                >
                    {move || if show_formulas.get() { "Hide formulas" } else { "Show formulas" }}
                </button>
            </div>
            {move || rejection.get().map(|message| view! {
                <div class="dtc-sheet-rejection" role="alert">
                    <span>{message}</span>
                    <button type="button" on:click=move |_| rejection.set(None)>"Dismiss"</button>
                </div>
            }.into_any())}
            <div class="dtc-sheet__body">
                <div class="dtc-sheet__scroll">
                    <NameBoxBar name_box=name_box workspace=workspace dispatch=dispatch.clone() />
                    {body}
                </div>
                <NodeInspector
                    workspace=workspace
                    selection=selection
                    editing=editing
                    editor_text=editor_text
                    edit_ref=edit_ref
                    commit=Arc::new(commit)
                />
            </div>
            <ConsoleBar workspace=workspace dispatch=console_dispatch />
        </section>
    }
}

/// One value-first row. A `div` (not a `button`) because the rendered value
/// may embed its own interactive control (the array expand toggle), and
/// nesting buttons is invalid; focus stays on the sheet section regardless
/// (mousedown is prevented), so the grammar keeps working.
fn node_row_view(
    node: NodeView,
    is_selected: bool,
    is_editing: bool,
    show_formula: bool,
    armed: bool,
    workspace: &WorkspaceState,
    on_click: Arc<dyn Fn(NodeKey, NodeId) + Send + Sync>,
) -> AnyView {
    let calc_class = calc_state_class(node.calc_state);
    let prov = provenance_tint(&node, workspace);
    let sel_class = selection_mode_class(is_selected, is_editing);
    let mut classes: Vec<&str> = vec!["dtc-sheet-row", calc_class, prov.css_class()];
    if armed {
        classes.push("dtc-sheet-row--armed");
    }
    if !sel_class.is_empty() {
        classes.push(sel_class);
    }
    let class_attr = classes.join(" ");
    let formula_line = (show_formula || is_selected) && !node.content_text.is_empty();
    let key_for_click = node.key.clone();
    let id_for_click = node.id.clone();
    let value_view = render_value(&node.computed_value);

    view! {
        <div
            class=class_attr
            // Rows never take keyboard focus: focus stays on the sheet so the
            // grammar keeps working after a click.
            on:mousedown=move |ev| ev.prevent_default()
            on:click=move |_| on_click(key_for_click.clone(), id_for_click.clone())
            title=node.id.to_string()
            data-node-key=node.key.to_string()
        >
            <div class="dtc-sheet-row__head">
                <span class="dtc-sheet-row__title">
                    <span class="dtc-calc-dot"></span>
                    {node.display_name.clone()}
                </span>
                {formula_line.then(|| view! {
                    <code class="dtc-sheet-row__formula">{node.content_text.clone()}</code>
                }.into_any())}
            </div>
            <div class="dtc-sheet-row__value">{value_view}</div>
        </div>
    }
    .into_any()
}

/// One in-place table grid: header row (when present), body rows, totals row
/// (when present), and the add-row/add-column affordances. The active cell —
/// the shared `selection.table_cell` — carries the 1-bit SELECTED border.
fn table_view(
    table_id: NodeId,
    table: &TableProjection,
    active_cell: Option<&TableCellSelection>,
    dispatch: Arc<dyn Dispatcher>,
    rejection: RwSignal<Option<String>>,
) -> AnyView {
    let header = table.header_row_present.then(|| {
        let heads = table
            .columns
            .iter()
            .map(|column| view! { <th>{column.name.clone()}</th> }.into_any())
            .collect::<Vec<_>>();
        view! { <tr class="dtc-sheet-table__head">{heads}</tr> }.into_any()
    });

    let body_rows = table
        .cells
        .as_ref()
        .map(|cells| {
            cells
                .body_rows
                .iter()
                .map(|row| {
                    let row_cells = row
                        .iter()
                        .map(|cell| {
                            table_cell_view(&table_id, cell.as_ref(), active_cell, dispatch.clone())
                        })
                        .collect::<Vec<_>>();
                    view! { <tr>{row_cells}</tr> }.into_any()
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let totals_row = if table.totals_row_present {
        table.cells.as_ref().map(|cells| {
            let row_cells = cells
                .totals_row
                .iter()
                .map(|cell| table_cell_view(&table_id, cell.as_ref(), active_cell, dispatch.clone()))
                .collect::<Vec<_>>();
            view! { <tr class="dtc-sheet-table__totals">{row_cells}</tr> }.into_any()
        })
    } else {
        None
    };

    let add_row = add_row_intent(&table_id, table);
    let add_column = add_column_intent(&table_id, table);
    let add_row_dispatch = dispatch.clone();
    let add_column_dispatch = dispatch;

    view! {
        <div class="dtc-sheet-table">
            <div class="dtc-sheet-table__name">
                {table.table_name.clone()}
                <code class="dtc-sheet-table__path">{table.display_path.clone()}</code>
            </div>
            <table>
                <tbody>
                    {header}
                    {body_rows}
                    {totals_row}
                </tbody>
            </table>
            <div class="dtc-sheet-table__actions">
                <button
                    type="button"
                    on:click=move |_| {
                        let receipt = add_row_dispatch.dispatch(add_row.clone());
                        if receipt.accepted {
                            rejection.set(None);
                        } else {
                            rejection.set(receipt.error.map(|error| error.to_string()));
                        }
                    }
                >
                    "Add row"
                </button>
                <button
                    type="button"
                    on:click=move |_| {
                        let receipt = add_column_dispatch.dispatch(add_column.clone());
                        if receipt.accepted {
                            rejection.set(None);
                        } else {
                            rejection.set(receipt.error.map(|error| error.to_string()));
                        }
                    }
                >
                    "Add column"
                </button>
            </div>
        </div>
    }
    .into_any()
}

/// One table cell. Renders the projected value text; click focuses the cell
/// through the closed `SelectTableCell` intent. Missing cell projections
/// render blank and inert.
fn table_cell_view(
    table_id: &NodeId,
    cell: Option<&TableCellProjection>,
    active_cell: Option<&TableCellSelection>,
    dispatch: Arc<dyn Dispatcher>,
) -> AnyView {
    let Some(cell) = cell else {
        return view! { <td class="dtc-sheet-cell dtc-sheet-cell--blank"></td> }.into_any();
    };
    let is_active = active_cell.is_some_and(|sel| {
        &sel.table == table_id
            && sel.row_id.as_deref() == cell.row_id.as_deref()
            && sel.column_id == cell.column_id
    });
    let mut class = String::from("dtc-sheet-cell");
    if is_active {
        class.push_str(" dtc-sel--selected");
    }
    let text = value_text(&cell.value);
    let intent = select_cell_intent(table_id, cell);

    view! {
        <td
            class=class
            on:click=move |_| {
                dispatch.dispatch(intent.clone());
            }
        >
            {text}
        </td>
    }
    .into_any()
}

/// Move keyboard focus to the formula bar with the caret at the end. No-op
/// when the input is not mounted or is disabled.
fn focus_formula_bar(bar_ref: NodeRef<leptos::html::Input>) {
    if let Some(input) = bar_ref.get_untracked() {
        let _ = input.focus();
        // Browsers clamp the range to the value length.
        let _ = input.set_selection_range(u32::MAX, u32::MAX);
    }
}

const SHEET_CSS: &str = r#"
.dtc-sheet {
  display: flex; flex-direction: column; height: 100%;
  background: var(--dtc-surface); color: var(--dtc-text);
  font: 13px/1.4 var(--dtc-font, system-ui, sans-serif);
  outline: none;
}
.dtc-sheet__body { display: flex; flex: 1; min-height: 0; }
.dtc-sheet__scroll { flex: 1; overflow: auto; padding: 8px 12px; min-width: 0; }

.dtc-sheet-bar {
  display: flex; align-items: center; gap: 8px; padding: 6px 12px;
  border-bottom: 1px solid var(--dtc-border-muted); background: var(--dtc-surface-subtle);
}
.dtc-sheet-bar__label {
  max-width: 220px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  font-family: ui-monospace, monospace; font-size: 12px; color: var(--dtc-text-muted);
}
.dtc-sheet-bar__badge {
  padding: 1px 8px; border: 1px solid var(--dtc-border); border-radius: 9px;
  font-size: 11px; color: var(--dtc-text-subtle); white-space: nowrap;
}
.dtc-sheet-bar__input {
  flex: 1; padding: 4px 8px; border: 1px solid var(--dtc-border); border-radius: 5px;
  background: var(--dtc-surface-input); color: var(--dtc-text);
  font-family: ui-monospace, monospace;
}
.dtc-sheet-bar__input:disabled { opacity: 0.55; }
.dtc-sheet-bar__arm {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 3px 8px; cursor: pointer; color: var(--dtc-text); white-space: nowrap;
}
.dtc-sheet-bar__arm--on { border-color: var(--dtc-accent-border); box-shadow: inset 0 0 0 1px var(--dtc-accent); }
.dtc-sheet-bar__arm:disabled { opacity: 0.5; cursor: default; }

.dtc-sheet-toolbar {
  display: flex; align-items: center; gap: 8px; padding: 4px 12px;
  border-bottom: 1px solid var(--dtc-border-muted);
}
.dtc-sheet-toolbar button {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 2px 8px; cursor: pointer; color: var(--dtc-text);
}

.dtc-sheet-rejection {
  display: flex; align-items: center; gap: 8px; padding: 4px 12px;
  color: var(--dtc-danger-text); background: var(--dtc-surface-subtle);
  border-bottom: 1px solid var(--dtc-border-muted);
}
.dtc-sheet-rejection button {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 1px 6px; cursor: pointer; color: var(--dtc-text);
}

.dtc-sheet-rows { display: flex; flex-direction: column; gap: 6px; margin-bottom: 12px; }
.dtc-sheet-row {
  display: flex; justify-content: space-between; align-items: flex-start; gap: 12px;
  padding: 6px 10px; background: var(--dtc-surface-panel);
  border: 1px solid var(--dtc-border); border-radius: 7px; cursor: pointer;
}
.dtc-sheet-row:hover { border-color: var(--dtc-border-strong); }
.dtc-sheet-row--armed { cursor: crosshair; }
.dtc-sheet-row__head { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.dtc-sheet-row__title { display: flex; align-items: center; gap: 6px; font-weight: 600; }
.dtc-sheet-row__formula { font-family: ui-monospace, monospace; font-size: 11px; color: var(--dtc-text-muted); }
.dtc-sheet-row__value { font-size: 15px; font-variant-numeric: tabular-nums; text-align: right; }

.dtc-sheet-table { margin: 0 0 14px; }
.dtc-sheet-table__name { display: flex; align-items: baseline; gap: 8px; font-weight: 700; margin-bottom: 4px; }
.dtc-sheet-table__path { font-size: 11px; color: var(--dtc-text-subtle); font-weight: 400; }
.dtc-sheet-table table { border-collapse: collapse; }
.dtc-sheet-table th {
  border: 1px solid var(--dtc-border); padding: 3px 10px; text-align: left;
  background: var(--dtc-surface-subtle);
}
.dtc-sheet-cell {
  border: 1px solid var(--dtc-border); padding: 3px 10px; cursor: pointer;
  text-align: right; font-variant-numeric: tabular-nums; min-width: 64px;
}
.dtc-sheet-cell--blank { cursor: default; }
.dtc-sheet-table__totals .dtc-sheet-cell { font-weight: 600; border-top: 2px solid var(--dtc-border-strong); }
.dtc-sheet-table__actions { display: flex; gap: 6px; margin-top: 4px; }
.dtc-sheet-table__actions button {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 2px 8px; cursor: pointer; color: var(--dtc-text);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::{
        ActiveTableCellDetailProjection, NodeContentKind, NodeValueProjection, SelectionState,
        TableAnchorProjection, TableCellsProjection, TableColumnBodyProjection,
        TableColumnProjection, TableFormulaMetadataProjection, TableRowProjection,
    };
    use std::collections::BTreeMap;

    fn node(key: &str, path: &str) -> NodeView {
        NodeView {
            key: NodeKey::new(key),
            id: NodeId::new(path),
            display_name: path.to_string(),
            parent: None,
            children: Vec::new(),
            depth: 0,
            content_kind: NodeContentKind::Constant,
            content_text: "0".to_string(),
            computed_value: NodeValueProjection::Number {
                raw: "0".to_string(),
                display: "0".to_string(),
            },
            scenario_override: None,
            literalized_value_input: None,
            value_epoch: None,
            calc_state: None,
            effective_format: None,
            note: None,
            attributes: BTreeMap::new(),
            binding_diagnostics: Vec::new(),
            is_meta: false,
            table: None,
        }
    }

    fn formula_meta(text: &str) -> TableFormulaMetadataProjection {
        TableFormulaMetadataProjection {
            formula_artifact_id: "fa".to_string(),
            bind_artifact_id: None,
            formula_text_version: "v1".to_string(),
            formula_text: text.to_string(),
        }
    }

    fn cell(row_id: Option<&str>, column_id: &str, key: &str, display: &str) -> TableCellProjection {
        TableCellProjection {
            row_id: row_id.map(str::to_string),
            column_id: column_id.to_string(),
            node_key: NodeKey::new(key),
            value: NodeValueProjection::Number {
                raw: display.to_string(),
                display: display.to_string(),
            },
        }
    }

    /// One table, two columns: `col:qty` (constant cells, has a totals
    /// formula) and `col:total` (column formula, no totals formula). One body
    /// row. Covers all four editability arms.
    fn sample_table() -> TableProjection {
        TableProjection {
            table_id: "tbl:items".to_string(),
            table_name: "Items".to_string(),
            display_path: "Model.Items".to_string(),
            canonical_path: "Model.Items".to_string(),
            virtual_anchor: TableAnchorProjection {
                workbook_scope_ref: "wb".to_string(),
                sheet_scope_ref: "sheet".to_string(),
                start_row: 0,
                start_col: 0,
            },
            rows: vec![TableRowProjection {
                row_id: "row:1".to_string(),
                ordinal: 0,
            }],
            columns: vec![
                TableColumnProjection {
                    column_id: "col:qty".to_string(),
                    name: "Qty".to_string(),
                    ordinal: 0,
                    body: TableColumnBodyProjection::ConstantCells,
                    totals_formula: Some(formula_meta("=SUM([Qty])")),
                },
                TableColumnProjection {
                    column_id: "col:total".to_string(),
                    name: "Total".to_string(),
                    ordinal: 1,
                    body: TableColumnBodyProjection::Formula(formula_meta("=[Qty]*2")),
                    totals_formula: None,
                },
            ],
            cells: Some(TableCellsProjection {
                body_rows: vec![vec![
                    Some(cell(Some("row:1"), "col:qty", "k:c1", "4")),
                    Some(cell(Some("row:1"), "col:total", "k:c2", "8")),
                ]],
                totals_row: vec![
                    Some(cell(None, "col:qty", "k:t1", "4")),
                    Some(cell(None, "col:total", "k:t2", "8")),
                ],
            }),
            row_count: 1,
            column_count: 2,
            header_row_present: true,
            totals_row_present: true,
            table_namespace_version: "v1".to_string(),
            row_membership_version: "v1".to_string(),
            row_order_version: "v1".to_string(),
            column_identity_version: "v1".to_string(),
            dependency_inventory: Vec::new(),
        }
    }

    /// Workspace with a plain node `A` (a formula), a meta node, a table
    /// anchor node, and the sample table projection.
    fn workspace() -> WorkspaceState {
        let mut ws = WorkspaceState::default();
        for (key, path) in [("k:A", "A"), ("k:M", "Meta"), ("k:T", "Model.Items")] {
            let n = node(key, path);
            ws.node_order.push(n.id.clone());
            ws.key_order.push(n.key.clone());
            ws.paths_by_key.insert(n.key.clone(), n.id.clone());
            ws.nodes.insert(n.id.clone(), n.clone());
            ws.nodes_by_key.insert(n.key.clone(), n);
        }
        ws.nodes.get_mut(&NodeId::new("A")).unwrap().content_text = "=1+1".to_string();
        ws.nodes_by_key
            .get_mut(&NodeKey::new("k:A"))
            .unwrap()
            .content_text = "=1+1".to_string();
        ws.nodes.get_mut(&NodeId::new("Meta")).unwrap().is_meta = true;
        ws.nodes_by_key
            .get_mut(&NodeKey::new("k:M"))
            .unwrap()
            .is_meta = true;
        ws.tables.insert(NodeId::new("Model.Items"), sample_table());
        ws
    }

    fn node_detail() -> ActiveSelectionDetailProjection {
        workspace()
            .active_selection_detail(&SelectionState::with_primary(Some(NodeId::new("A"))))
            .expect("node detail")
    }

    fn cell_detail(row_id: Option<&str>, column_id: &str) -> ActiveTableCellDetailProjection {
        let ws = workspace();
        let selection = SelectionState::with_table_cell(TableCellSelection {
            table: NodeId::new("Model.Items"),
            row_id: row_id.map(str::to_string),
            column_id: column_id.to_string(),
        });
        ws.active_table_cell_detail(&selection).expect("cell detail")
    }

    #[test]
    fn selection_label_names_nodes_and_cells() {
        assert_eq!(selection_label(&node_detail()), "A");
        let body = ActiveSelectionDetailProjection::TableCell(cell_detail(Some("row:1"), "col:qty"));
        assert_eq!(selection_label(&body), "Items!row:1:col:qty");
        let totals = ActiveSelectionDetailProjection::TableCell(cell_detail(None, "col:qty"));
        assert_eq!(selection_label(&totals), "Items!totals:col:qty");
    }

    #[test]
    fn formula_bar_buffer_per_editability() {
        // Node: the authored content text.
        assert_eq!(formula_bar_buffer(&node_detail()), "=1+1");
        // Direct-input cell: the rendered value text (no per-cell content in
        // the projection).
        let direct =
            ActiveSelectionDetailProjection::TableCell(cell_detail(Some("row:1"), "col:qty"));
        assert_eq!(formula_bar_buffer(&direct), "4");
        // Formula-backed cell: the COLUMN formula.
        let backed =
            ActiveSelectionDetailProjection::TableCell(cell_detail(Some("row:1"), "col:total"));
        assert_eq!(formula_bar_buffer(&backed), "=[Qty]*2");
        // Totals cell with a formula: the totals formula.
        let totals = ActiveSelectionDetailProjection::TableCell(cell_detail(None, "col:qty"));
        assert_eq!(formula_bar_buffer(&totals), "=SUM([Qty])");
        // Read-only totals cell: the value text (input is disabled).
        let read_only = ActiveSelectionDetailProjection::TableCell(cell_detail(None, "col:total"));
        assert_eq!(formula_bar_buffer(&read_only), "8");
    }

    #[test]
    fn editability_badge_is_structural() {
        assert_eq!(editability_badge(&node_detail()), None);
        let direct =
            ActiveSelectionDetailProjection::TableCell(cell_detail(Some("row:1"), "col:qty"));
        assert_eq!(editability_badge(&direct), None);
        let backed =
            ActiveSelectionDetailProjection::TableCell(cell_detail(Some("row:1"), "col:total"));
        assert_eq!(editability_badge(&backed), Some("column formula"));
        let totals = ActiveSelectionDetailProjection::TableCell(cell_detail(None, "col:qty"));
        assert_eq!(editability_badge(&totals), Some("totals"));
        let read_only = ActiveSelectionDetailProjection::TableCell(cell_detail(None, "col:total"));
        assert_eq!(editability_badge(&read_only), Some("read-only"));
    }

    #[test]
    fn bar_is_read_only_for_missing_or_read_only_selection() {
        assert!(bar_is_read_only(None));
        assert!(!bar_is_read_only(Some(&node_detail())));
        let direct =
            ActiveSelectionDetailProjection::TableCell(cell_detail(Some("row:1"), "col:qty"));
        assert!(!bar_is_read_only(Some(&direct)));
        let read_only = ActiveSelectionDetailProjection::TableCell(cell_detail(None, "col:total"));
        assert!(bar_is_read_only(Some(&read_only)));
    }

    #[test]
    fn commit_intent_node_stays_on_shared_path() {
        assert_eq!(sheet_commit_intent(&node_detail(), "=2".to_string()), None);
    }

    #[test]
    fn commit_intent_direct_input_edits_the_cell() {
        let detail =
            ActiveSelectionDetailProjection::TableCell(cell_detail(Some("row:1"), "col:qty"));
        assert_eq!(
            sheet_commit_intent(&detail, "5".to_string()),
            Some(WorkspaceIntent::EditTableCell {
                table: NodeId::new("Model.Items"),
                row_id: "row:1".to_string(),
                column_id: "col:qty".to_string(),
                content: "5".to_string(),
            })
        );
    }

    #[test]
    fn commit_intent_formula_backed_edits_the_column_formula() {
        let detail =
            ActiveSelectionDetailProjection::TableCell(cell_detail(Some("row:1"), "col:total"));
        assert_eq!(
            sheet_commit_intent(&detail, "=[Qty]*3".to_string()),
            Some(WorkspaceIntent::EditTableColumnFormula {
                table: NodeId::new("Model.Items"),
                column_id: "col:total".to_string(),
                formula_text: "=[Qty]*3".to_string(),
            })
        );
    }

    #[test]
    fn commit_intent_totals_formula_sets_the_totals_formula() {
        let detail = ActiveSelectionDetailProjection::TableCell(cell_detail(None, "col:qty"));
        assert_eq!(
            sheet_commit_intent(&detail, "=AVERAGE([Qty])".to_string()),
            Some(WorkspaceIntent::SetTableTotalsFormula {
                table: NodeId::new("Model.Items"),
                column_id: "col:qty".to_string(),
                formula_text: "=AVERAGE([Qty])".to_string(),
            })
        );
    }

    #[test]
    fn commit_intent_read_only_is_none() {
        let detail = ActiveSelectionDetailProjection::TableCell(cell_detail(None, "col:total"));
        assert_eq!(sheet_commit_intent(&detail, "9".to_string()), None);
    }

    #[test]
    fn caret_char_offset_clamps_to_char_count() {
        assert_eq!(caret_char_offset(Some(0), "abc"), 0);
        assert_eq!(caret_char_offset(Some(2), "abc"), 2);
        assert_eq!(caret_char_offset(Some(99), "abc"), 3);
        assert_eq!(caret_char_offset(None, "abc"), 3);
        // Char offsets, not bytes: multibyte buffers clamp to the char count.
        assert_eq!(caret_char_offset(Some(99), "né☃"), 3);
        assert_eq!(caret_char_offset(None, ""), 0);
    }

    #[test]
    fn add_row_intent_builds_next_row_payload() {
        let table = sample_table();
        let id = NodeId::new("Model.Items");
        assert_eq!(
            add_row_intent(&id, &table),
            WorkspaceIntent::AddTableRow {
                table: id.clone(),
                row_id: "row:new2".to_string(),
                values: Vec::new(),
            }
        );
    }

    #[test]
    fn add_column_intent_builds_next_column_payload() {
        let table = sample_table();
        let id = NodeId::new("Model.Items");
        assert_eq!(
            add_column_intent(&id, &table),
            WorkspaceIntent::AddTableColumn {
                table: id.clone(),
                column_id: "col:new3".to_string(),
                name: "Column 3".to_string(),
                values: Vec::new(),
            }
        );
    }

    #[test]
    fn select_cell_intent_uses_stable_ids() {
        let id = NodeId::new("Model.Items");
        let body = cell(Some("row:1"), "col:qty", "k:c1", "4");
        assert_eq!(
            select_cell_intent(&id, &body),
            WorkspaceIntent::SelectTableCell {
                table: id.clone(),
                row_id: Some("row:1".to_string()),
                column_id: "col:qty".to_string(),
            }
        );
        let totals = cell(None, "col:qty", "k:t1", "4");
        assert_eq!(
            select_cell_intent(&id, &totals),
            WorkspaceIntent::SelectTableCell {
                table: id,
                row_id: None,
                column_id: "col:qty".to_string(),
            }
        );
    }

    #[test]
    fn insert_reference_intent_targets_the_clicked_node() {
        assert_eq!(
            insert_reference_intent(
                NodeKey::new("k:A"),
                "=1+1".to_string(),
                2,
                NodeKey::new("k:B"),
            ),
            WorkspaceIntent::InsertFormulaReference {
                node: NodeKey::new("k:A"),
                current_formula_text: "=1+1".to_string(),
                replacement_start: 2,
                replacement_len: 0,
                target: FormulaReferenceInsertionTarget::Node(NodeKey::new("k:B")),
            }
        );
    }

    #[test]
    fn first_inserted_reference_finds_the_insertion() {
        let insertion = FormulaReferenceInsertionProjection {
            node: NodeKey::new("k:A"),
            target: FormulaReferenceInsertionTarget::Node(NodeKey::new("k:B")),
            inserted_text: "B".to_string(),
            updated_formula_text: "=B+1".to_string(),
            applied_start: 1,
            applied_len: 0,
        };
        let changes = vec![
            WorkspaceDeltaChange::NodesChanged(Vec::new()),
            WorkspaceDeltaChange::FormulaReferenceInserted(insertion.clone()),
        ];
        assert_eq!(first_inserted_reference(&changes), Some(&insertion));
        assert_eq!(first_inserted_reference(&[]), None);
    }

    #[test]
    fn sheet_row_keys_skips_meta_and_table_anchors() {
        let ws = workspace();
        assert_eq!(sheet_row_keys(&ws), vec![NodeKey::new("k:A")]);
    }
}
