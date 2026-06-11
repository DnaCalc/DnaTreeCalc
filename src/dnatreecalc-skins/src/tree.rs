//! TREE — the hierarchy lens (ATLAS Phase A).
//!
//! Tree makes the model's hierarchy spatial: an indented, foldable outline for
//! folding, reshaping, and refactoring structure. It conforms to the shared
//! spine the Flow reference lens realizes:
//!
//! - **one grammar** — `h`/`l` fold/unfold the selected node, `/` Name-Box,
//!   Enter/Esc modeless edit (resolved through `KeybindingRegistry`);
//! - **one styling** — `calc_state` is the only saturated channel
//!   (`calc_state_class`), a 1-bit SELECTED/EDITING border
//!   (`selection_mode_class`), and meta structure rendered deliberately faint;
//! - **one continuity** — the fold set lives in `SharedSkinState.collapsed_keys`
//!   so collapsing a branch here collapses it in every later lens.
//!
//! Everything Tree shows is read from the published projection; every change it
//! makes goes through the closed `WorkspaceIntent` surface. It never parses
//! formula text, fabricates a value, or uses grid coordinates. Structural verbs
//! (duplicate/copy subtree, add/rename/move/reorder/delete) dispatch typed
//! intents and surface typed rejection receipts in a dismissible strip — the
//! honest feedback channel until legality previews reach the Skin IR.
//!
//! The embedded **Lens** inspector, **Console** strip, and Name-Box are the
//! shared [`crate::spine_widgets`] components; the structure workbench reuses
//! [`crate::node_management::NodeManagementPanel`].

use std::collections::HashSet;
use std::sync::Arc;

use dnatreecalc_skin_framework::{
    ATLAS_SPINE_CSS, AuthoringScope, ClipboardPayloadKind, Dispatcher, IntentReceipt, KeyChord,
    KeybindingRegistry, NodeCalcStateProjection, NodeId, NodeKey, NodeView, SelectionState,
    SharedSkinStateHandle, SharedStateChange, SharedStateOrigin, SkinCapabilities, SkinCategory,
    SkinContext, SkinHandle, SkinId, SkinManifest, SkinMountSlot, SkinState, SkinVerb,
    WorkspaceIntent, WorkspaceSkin, WorkspaceState, calc_state_class, selection_mode_class,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::node_management::NodeManagementPanel;
use crate::spine_widgets::{
    ConsoleBar, NameBoxBar, NodeInspector, SPINE_WIDGETS_CSS, commit_content_edit,
    event_target_is_interactive, event_target_is_text_entry, focus_edit_buffer,
};

pub const TREE_ID: SkinId = SkinId::new("tree");

/// Indentation per tree level, in pixels.
const INDENT_PX: u32 = 18;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeState {
    /// Whether effectively-meta structure is shown (faint) or hidden. Tree is
    /// the structure lens, so meta scaffolding is visible by default.
    pub show_meta: bool,
}

impl Default for TreeState {
    fn default() -> Self {
        Self { show_meta: true }
    }
}

impl SkinState for TreeState {
    fn schema_version() -> u32 {
        1
    }
}

#[derive(Default)]
pub struct TreeLens;

impl TreeLens {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSkin for TreeLens {
    type State = TreeState;

    fn id(&self) -> SkinId {
        TREE_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Tree",
            description: "Hierarchy lens: fold, reshape, and refactor the model structure.",
            category: SkinCategory::Editor,
            version: "0.1.0",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities {
            supports_multi_select: false,
            supports_inline_formula_edit: false,
            supports_meta_node_display: true,
            renders_arrays_inline: false,
            renders_table_values: false,
            allowed_slots: None,
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        crate::spine_widgets::stamp_active_lens(cx.shared, TREE_ID, cx.slot);
        SkinHandle::new(view! { <TreeView cx=cx /> }.into_any())
    }
}

// ---------------------------------------------------------------------------
// Pure helpers (row building / fold continuity / intent builders)
// ---------------------------------------------------------------------------

/// One visible outline row, projected from published truth only.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeRow {
    id: NodeId,
    key: NodeKey,
    label: String,
    depth: u32,
    is_meta_effective: bool,
    has_children: bool,
    collapsed: bool,
    /// Dependents badge: how many edges point AT this node (projection fact).
    incoming_count: usize,
    calc_state: Option<NodeCalcStateProjection>,
}

/// Depth-first walk from `root_paths`, children in `NodeView.children` order
/// (engine-ordered — never sorted here), skipping the descendants of collapsed
/// nodes. When `show_meta` is false, effectively-meta rows are skipped
/// entirely; the contagion rule lives in [`WorkspaceState::is_effective_meta`],
/// never re-derived in the lens.
fn visible_rows(
    workspace: &WorkspaceState,
    collapsed: &HashSet<NodeKey>,
    show_meta: bool,
) -> Vec<TreeRow> {
    let mut rows = Vec::new();
    for root in &workspace.root_paths {
        push_visible_rows(workspace, root, 0, collapsed, show_meta, &mut rows);
    }
    rows
}

fn push_visible_rows(
    workspace: &WorkspaceState,
    id: &NodeId,
    depth: u32,
    collapsed: &HashSet<NodeKey>,
    show_meta: bool,
    rows: &mut Vec<TreeRow>,
) {
    let Some(node) = workspace.node(id) else {
        return; // projection lists a child that is not published; skip it
    };
    let is_meta_effective = workspace.is_effective_meta(&node.key);
    if !show_meta && is_meta_effective {
        // Effective meta is contagious, so the whole branch is hidden.
        return;
    }
    let has_children = !node.children.is_empty();
    let is_collapsed = has_children && collapsed.contains(&node.key);
    rows.push(TreeRow {
        id: node.id.clone(),
        key: node.key.clone(),
        label: node.display_name.clone(),
        depth,
        is_meta_effective,
        has_children,
        collapsed: is_collapsed,
        incoming_count: workspace.dependencies.incoming_count_by_key(&node.key),
        calc_state: node.calc_state,
    });
    if is_collapsed {
        return;
    }
    for child in &node.children {
        push_visible_rows(workspace, child, depth + 1, collapsed, show_meta, rows);
    }
}

/// Apply a fold verb to the SHARED collapse set: `fold` inserts the key,
/// `!fold` removes it. Idempotent in both directions.
///
/// The UI reaches these semantics through the audited chokepoint
/// (`SharedStateChange::Fold`/`Unfold`); this pure form documents them and is
/// exercised by the unit tests.
#[cfg(test)]
fn apply_fold(collapsed: &mut HashSet<NodeKey>, key: &NodeKey, fold: bool) {
    if fold {
        collapsed.insert(key.clone());
    } else {
        collapsed.remove(key);
    }
}

/// Chevron toggle over the SHARED collapse set.
///
/// The UI reaches these semantics through the audited chokepoint (read the
/// collapsed set, then `Fold`/`Unfold`); this pure form documents them and is
/// exercised by the unit tests.
#[cfg(test)]
fn toggle_fold(collapsed: &mut HashSet<NodeKey>, key: &NodeKey) {
    if !collapsed.remove(key) {
        collapsed.insert(key.clone());
    }
}

/// Duplicate the selected subtree beside its source: stable-key source,
/// path-addressed destination parent (the intent payload mixes identities
/// deliberately), and a `_copy` symbol. Legality (formula-free subtree, name
/// collisions) is engine truth — the receipt reports rejections.
fn duplicate_subtree_intent(node: &NodeView) -> WorkspaceIntent {
    WorkspaceIntent::DuplicateSubtree {
        source: node.key.clone(),
        destination_parent: node.parent.clone(),
        new_symbol: format!("{}_copy", node.display_name),
    }
}

/// Copy the selected subtree into the host-owned clipboard carrier. Scope
/// expansion (subtree membership) is host/projection-owned.
fn copy_subtree_intent(node: &NodeView) -> WorkspaceIntent {
    WorkspaceIntent::CopyToClipboard {
        scope: AuthoringScope::Subtree(node.key.clone()),
        payload: ClipboardPayloadKind::Subtree,
    }
}

/// The typed rejection channel: `Some(message)` exactly when the dispatcher
/// refused the intent.
fn receipt_error_message(receipt: &IntentReceipt) -> Option<String> {
    if receipt.accepted {
        return None;
    }
    Some(
        receipt
            .error
            .as_ref()
            .map_or_else(|| "intent rejected".to_string(), ToString::to_string),
    )
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

#[component]
fn TreeView(cx: SkinContext<TreeState>) -> impl IntoView {
    let workspace = cx.workspace;
    let selection = cx.selection;
    let shared = cx.shared;
    let dispatch = cx.dispatch.clone();
    let state = cx.state.clone();
    let shared_origin = SharedStateOrigin::lens(TREE_ID, cx.slot);

    let editing = RwSignal::new(false);
    let editor_text = RwSignal::new(String::new());
    let name_box = RwSignal::new(Option::<String>::None);
    let last_error = RwSignal::new(Option::<String>::None);
    let edit_ref = NodeRef::<leptos::html::Textarea>::new();
    let stage_ref = NodeRef::<leptos::html::Section>::new();

    let selected = Memo::new(move |_| {
        let primary = selection.with(|s| s.primary.clone());
        workspace.with(|ws| primary.as_ref().and_then(|id| ws.node(id).cloned()))
    });
    // Selection *identity*, distinct from node contents — the edit buffer must
    // reset when the user selects a different node, but never when the same
    // node merely republishes (recalc, undo elsewhere, epoch bumps).
    let selected_id = Memo::new(move |_| selection.with(|s| s.primary.clone()));

    // Reset the buffer and leave edit mode only when the selection target
    // changes.
    Effect::new(move |_| {
        let _identity = selected_id.get();
        if let Some(node) = selected.get_untracked() {
            editor_text.set(node.content_text);
        } else {
            editor_text.set(String::new());
        }
        editing.set(false);
    });
    // While NOT editing, keep the buffer synced to the published content (so
    // undo/redo and external commits refresh it); while editing, never clobber.
    Effect::new(move |_| {
        if let Some(node) = selected.get()
            && !editing.get_untracked()
        {
            editor_text.set(node.content_text);
        }
    });

    // The one commit path, shared with every lens.
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
        } else if let Some(message) = receipt_error_message(&receipt) {
            last_error.set(Some(message));
        }
    };

    // Lens-local grammar, resolved through the one registry. Global verbs
    // (F9, Ctrl+Z/Y, Ctrl+N, lens switch, arrows) bubble to the shell.
    let grammar = KeybindingRegistry::universal();
    let grammar_origin = shared_origin.clone();
    let root_keydown = move |ev: leptos::ev::KeyboardEvent| {
        // Bare-key grammar never fires while typing — the contract. (The
        // editing flag deliberately does NOT gate here: while the buffer has
        // focus its own handler stops propagation, and when editing is set
        // with no mounted buffer — e.g. after a companion stand-down —
        // Escape must still be able to clear it.)
        if event_target_is_text_entry(&ev) {
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
                // While a Lens companion is mounted the embedded buffer is
                // stood down — the companion owns editing; never set a local
                // editing flag with no buffer to land in.
                if shared
                    .get_untracked()
                    .companion_slots_active
                    .contains(&SkinMountSlot::RightInspector)
                {
                    return;
                }
                if selected.get_untracked().is_some() {
                    ev.prevent_default();
                    editing.set(true);
                    focus_edit_buffer(edit_ref);
                }
            }
            SkinVerb::Fold => {
                // Fold the SELECTED node in the SHARED collapse set — the
                // continuity every other lens reads.
                if let Some(node) = selected.get_untracked()
                    && !node.children.is_empty()
                {
                    ev.prevent_default();
                    shared.apply(
                        SharedStateChange::Fold(node.key.clone()),
                        grammar_origin.clone(),
                    );
                }
            }
            SkinVerb::Unfold => {
                if let Some(node) = selected.get_untracked() {
                    ev.prevent_default();
                    shared.apply(
                        SharedStateChange::Unfold(node.key.clone()),
                        grammar_origin.clone(),
                    );
                }
            }
            SkinVerb::NameBox => {
                ev.prevent_default();
                name_box.set(Some(String::new()));
            }
            SkinVerb::Escape => {
                name_box.set(None);
                editing.set(false);
                last_error.set(None);
            }
            _ => {} // global verb: leave for the shell
        }
    };

    // Land keyboard focus on the stage when the lens mounts, so the grammar
    // works without a preparatory click.
    Effect::new(move |_| {
        if let Some(section) = stage_ref.get() {
            let _ = section.focus();
        }
    });

    let error_strip = move || {
        last_error
            .get()
            .map(|message| {
                view! {
                    <div class="dtc-tree-error" role="alert">
                        <span class="dtc-tree-error__text">{message}</span>
                        <button
                            type="button"
                            class="dtc-tree-error__dismiss"
                            on:click=move |_| last_error.set(None)
                        >
                            "Dismiss"
                        </button>
                    </div>
                }
                .into_any()
            })
            .unwrap_or_else(|| ().into_any())
    };

    let rows_workspace = workspace;
    let rows_selection = selection;
    let rows_state = state.clone();
    let rows_dispatch = dispatch.clone();
    let rows_origin = shared_origin.clone();
    let rows_view = move || {
        let ws = rows_workspace.get();
        let collapsed = shared.with(|s| s.collapsed_keys.clone());
        let show_meta = rows_state.with(|s| s.show_meta);
        let sel_key = rows_selection.with(|s| {
            s.primary
                .as_ref()
                .and_then(|id| ws.node(id))
                .map(|node| node.key.clone())
        });
        let editing_now = editing.get();
        let rows = visible_rows(&ws, &collapsed, show_meta);
        if rows.is_empty() {
            return view! {
                <p class="dtc-tree__empty">
                    "No visible nodes — add one on the right, or show meta structure."
                </p>
            }
            .into_any();
        }
        let row_views = rows
            .into_iter()
            .map(|row| {
                let is_selected = sel_key.as_ref() == Some(&row.key);
                tree_row_view(
                    row,
                    is_selected,
                    editing_now && is_selected,
                    shared,
                    &rows_origin,
                    rows_dispatch.clone(),
                    last_error,
                    stage_ref,
                )
            })
            .collect::<Vec<_>>();
        view! { <div class="dtc-tree-rows" aria-label="Model hierarchy">{row_views}</div> }
            .into_any()
    };

    let css = format!("{ATLAS_SPINE_CSS}\n{SPINE_WIDGETS_CSS}\n{TREE_CSS}");
    let bar_dispatch = dispatch.clone();
    let panel_dispatch = dispatch.clone();
    let console_dispatch = dispatch.clone();

    view! {
        <style>{css}</style>
        <section class="dtc-tree" tabindex="0" node_ref=stage_ref on:keydown=root_keydown>
            <div class="dtc-tree__body">
                <div class="dtc-tree__main">
                    <NameBoxBar name_box=name_box workspace=workspace dispatch=dispatch.clone() />
                    <TreeHeaderBar
                        workspace=workspace
                        selection=selection
                        state=state.clone()
                        dispatch=bar_dispatch
                        last_error=last_error
                    />
                    {error_strip}
                    <div class="dtc-tree__scroll">{rows_view}</div>
                </div>
                <div class="dtc-tree__side">
                    <NodeManagementPanel
                        workspace=workspace
                        selection=selection
                        dispatch=panel_dispatch
                    />
                    <NodeInspector
                        workspace=workspace
                        selection=selection
                        editing=editing
                        editor_text=editor_text
                        edit_ref=edit_ref
                        commit=Arc::new(commit)
                        shared=shared
                    preview=cx.preview.clone()
                    />
                </div>
            </div>
            <ConsoleBar workspace=workspace dispatch=console_dispatch shared=shared />
        </section>
    }
}

/// One outline row: chevron (shared fold continuity), calc-state dot, label,
/// and the dependents badge. Rows never take keyboard focus — focus stays on
/// the stage so the grammar keeps working (the Flow chip pattern).
#[allow(clippy::too_many_arguments)]
fn tree_row_view(
    row: TreeRow,
    is_selected: bool,
    is_editing: bool,
    shared: SharedSkinStateHandle,
    origin: &SharedStateOrigin,
    dispatch: Arc<dyn Dispatcher>,
    last_error: RwSignal<Option<String>>,
    stage_ref: NodeRef<leptos::html::Section>,
) -> AnyView {
    let calc_class = calc_state_class(row.calc_state);
    let sel_class = selection_mode_class(is_selected, is_editing);
    let mut classes: Vec<&str> = vec!["dtc-tree-row", calc_class];
    if row.is_meta_effective {
        classes.push("dtc-tree-row--meta");
    }
    if !sel_class.is_empty() {
        classes.push(sel_class);
    }
    let class_attr = classes.join(" ");
    let indent = format!("padding-left:{}px;", 6 + row.depth * INDENT_PX);
    let key_for_toggle = row.key.clone();
    let id_for_click = row.id.clone();
    let dependents_badge =
        (row.incoming_count > 0).then(|| format!("← {}", row.incoming_count));

    let chevron = if row.has_children {
        let glyph = if row.collapsed { "▸" } else { "▾" };
        let label = if row.collapsed { "Expand" } else { "Collapse" };
        let toggle_origin = origin.clone();
        view! {
            <button
                type="button"
                class="dtc-tree-row__chevron"
                tabindex="-1"
                aria-label=label
                on:mousedown=move |ev| ev.prevent_default()
                on:click=move |_| {
                    // Chevron toggle: read the shared collapse set, then fold
                    // or unfold through the audited chokepoint.
                    let folded = shared
                        .get_untracked()
                        .collapsed_keys
                        .contains(&key_for_toggle);
                    let change = if folded {
                        SharedStateChange::Unfold(key_for_toggle.clone())
                    } else {
                        SharedStateChange::Fold(key_for_toggle.clone())
                    };
                    shared.apply(change, toggle_origin.clone());
                    if let Some(section) = stage_ref.get_untracked() {
                        let _ = section.focus();
                    }
                }
            >
                {glyph}
            </button>
        }
        .into_any()
    } else {
        view! { <span class="dtc-tree-row__chevron dtc-tree-row__chevron--leaf"></span> }
            .into_any()
    };

    view! {
        <div class=class_attr style=indent data-node-key=row.key.to_string()>
            {chevron}
            <button
                type="button"
                class="dtc-tree-row__label"
                tabindex="-1"
                title=row.id.to_string()
                on:mousedown=move |ev| ev.prevent_default()
                on:click=move |_| {
                    let receipt =
                        dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id_for_click.clone())));
                    if let Some(message) = receipt_error_message(&receipt) {
                        last_error.set(Some(message));
                    }
                    if let Some(section) = stage_ref.get_untracked() {
                        let _ = section.focus();
                    }
                }
            >
                <span class="dtc-calc-dot"></span>
                <span class="dtc-tree-row__name">{row.label.clone()}</span>
            </button>
            {dependents_badge.map(|badge| view! {
                <span class="dtc-tree-row__deps" title="Dependents pointing at this node">
                    {badge}
                </span>
            })}
        </div>
    }
    .into_any()
}

/// Structure affordances over the outline: duplicate/copy the selected subtree
/// (typed intents; rejections land in the error strip) and the lens-local
/// show-meta toggle.
#[component]
fn TreeHeaderBar(
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<SelectionState>,
    state: dnatreecalc_skin_framework::SkinStateHandle<TreeState>,
    dispatch: Arc<dyn Dispatcher>,
    last_error: RwSignal<Option<String>>,
) -> impl IntoView {
    let selected = Memo::new(move |_| {
        let primary = selection.with(|s| s.primary.clone());
        workspace.with(|ws| primary.as_ref().and_then(|id| ws.node(id).cloned()))
    });
    let show_meta = {
        let state = state.clone();
        Memo::new(move |_| state.with(|s| s.show_meta))
    };
    let duplicate_dispatch = dispatch.clone();
    let copy_dispatch = dispatch.clone();
    let toggle_state = state.clone();

    view! {
        <div class="dtc-tree-bar" aria-label="Tree structure actions">
            <span class="dtc-tree-bar__label">"structure"</span>
            <button
                type="button"
                disabled=move || selected.with(Option::is_none)
                title="Duplicate the selected subtree beside its source"
                on:click=move |_| {
                    if let Some(node) = selected.get_untracked() {
                        let receipt = duplicate_dispatch.dispatch(duplicate_subtree_intent(&node));
                        if let Some(message) = receipt_error_message(&receipt) {
                            last_error.set(Some(message));
                        }
                    }
                }
            >
                "Duplicate subtree"
            </button>
            <button
                type="button"
                disabled=move || selected.with(Option::is_none)
                title="Copy the selected subtree to the model clipboard"
                on:click=move |_| {
                    if let Some(node) = selected.get_untracked() {
                        let receipt = copy_dispatch.dispatch(copy_subtree_intent(&node));
                        if let Some(message) = receipt_error_message(&receipt) {
                            last_error.set(Some(message));
                        }
                    }
                }
            >
                "Copy subtree"
            </button>
            <span class="dtc-tree-bar__spacer"></span>
            <span class="dtc-tree-bar__group">
                <span class="dtc-tree-bar__label">"fold"</span>
                <kbd>"h"</kbd>
                <kbd>"l"</kbd>
            </span>
            <button
                type="button"
                aria-pressed=move || show_meta.get().to_string()
                title="Show or hide meta structure (lens-local)"
                on:click=move |_| toggle_state.update(|s| s.show_meta = !s.show_meta)
            >
                {move || if show_meta.get() { "Meta: shown" } else { "Meta: hidden" }}
            </button>
        </div>
    }
}

const TREE_CSS: &str = r#"
.dtc-tree {
  display: flex; flex-direction: column; height: 100%;
  background: var(--dtc-surface); color: var(--dtc-text);
  font: 13px/1.4 var(--dtc-font, system-ui, sans-serif);
  outline: none;
}
.dtc-tree__body { display: flex; flex: 1; min-height: 0; }
.dtc-tree__main { flex: 1; display: flex; flex-direction: column; min-width: 0; padding: 8px 12px; }
.dtc-tree__scroll { flex: 1; overflow: auto; }
.dtc-tree__side {
  width: 340px; flex-shrink: 0; display: flex; flex-direction: column; min-height: 0;
  overflow: auto; border-left: 1px solid var(--dtc-border-muted); background: var(--dtc-surface-panel);
}
.dtc-tree__side .dtc-inspector { width: auto; border-left: none; flex-shrink: 0; overflow: visible; }
.dtc-tree__empty { color: var(--dtc-text-subtle); font-style: italic; }

.dtc-tree-bar {
  display: flex; align-items: center; gap: 8px; padding: 6px 8px; margin-bottom: 8px;
  background: var(--dtc-surface-subtle); border: 1px solid var(--dtc-border-muted); border-radius: 6px;
}
.dtc-tree-bar__group { display: inline-flex; align-items: center; gap: 6px; }
.dtc-tree-bar__label { color: var(--dtc-text-subtle); text-transform: uppercase; font-size: 11px; letter-spacing: 0.04em; }
.dtc-tree-bar__spacer { flex: 1; }
.dtc-tree-bar button {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 2px 8px; cursor: pointer; color: var(--dtc-text);
}
.dtc-tree-bar button:disabled { opacity: 0.45; cursor: default; }

.dtc-tree-error {
  display: flex; align-items: center; gap: 8px; padding: 5px 8px; margin-bottom: 8px;
  border: 1px solid var(--dtc-danger); border-radius: 6px;
  color: var(--dtc-danger-text); background: var(--dtc-surface-subtle);
}
.dtc-tree-error__text { flex: 1; }
.dtc-tree-error__dismiss {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 1px 7px; cursor: pointer; color: var(--dtc-text);
}

.dtc-tree .dtc-tree-row {
  display: flex; align-items: center; gap: 6px; padding: 2px 6px; border-radius: 5px;
}
.dtc-tree .dtc-tree-row--meta { opacity: 0.55; font-style: italic; }
.dtc-tree .dtc-tree-row__chevron {
  display: inline-flex; align-items: center; justify-content: center;
  width: 18px; height: 18px; padding: 0; flex-shrink: 0;
  background: transparent; border: none; cursor: pointer; color: var(--dtc-text-subtle);
}
.dtc-tree .dtc-tree-row__chevron--leaf { cursor: default; }
.dtc-tree .dtc-tree-row__label {
  display: inline-flex; align-items: center; gap: 6px; min-width: 0;
  background: transparent; border: none; cursor: pointer; color: inherit;
  padding: 2px 4px; border-radius: 4px; text-align: left; font: inherit;
}
.dtc-tree .dtc-tree-row__label:hover { background: var(--dtc-surface-subtle); }
.dtc-tree .dtc-tree-row__name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.dtc-tree .dtc-tree-row__deps {
  font-size: 11px; color: var(--dtc-text-muted); border: 1px solid var(--dtc-border-muted);
  border-radius: 9px; padding: 0 6px; font-variant-numeric: tabular-nums; flex-shrink: 0;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::{
        DependencyEdgeProjection, DependencyKindProjection, IntentError, NodeContentKind,
        NodeValueProjection,
    };
    use std::collections::BTreeMap;

    fn node(key: &str, path: &str) -> NodeView {
        NodeView {
            key: NodeKey::new(key),
            id: NodeId::new(path),
            display_name: path.rsplit('.').next().unwrap_or(path).to_string(),
            parent: path
                .rsplit_once('.')
                .map(|(parent, _)| NodeId::new(parent)),
            children: Vec::new(),
            depth: path.matches('.').count() as u32,
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

    fn edge(owner: &str, target: &str) -> DependencyEdgeProjection {
        DependencyEdgeProjection {
            edge_id: format!("{owner}->{target}"),
            descriptor_id: "d".to_string(),
            owner: NodeId::new(owner),
            owner_key: NodeKey::new(format!("k:{owner}")),
            target: NodeId::new(target),
            target_key: NodeKey::new(format!("k:{target}")),
            kind: DependencyKindProjection::StaticDirect,
        }
    }

    fn workspace_with(nodes: Vec<NodeView>) -> WorkspaceState {
        let mut ws = WorkspaceState::default();
        for node in nodes {
            if node.parent.is_none() {
                ws.root_paths.push(node.id.clone());
                ws.root_keys.push(node.key.clone());
            }
            ws.node_order.push(node.id.clone());
            ws.key_order.push(node.key.clone());
            ws.paths_by_key.insert(node.key.clone(), node.id.clone());
            ws.nodes.insert(node.id.clone(), node.clone());
            ws.nodes_by_key.insert(node.key.clone(), node);
        }
        ws
    }

    /// A: root with children [A.C, A.B] (engine order, deliberately not
    /// alphabetical); A.B has child A.B.D; M is a meta root with a
    /// regular-flagged child M.X (effectively meta by contagion). A.C has one
    /// incoming dependency edge.
    fn sample_workspace() -> WorkspaceState {
        let mut a = node("k:A", "A");
        a.children = vec![NodeId::new("A.C"), NodeId::new("A.B")];
        let mut ab = node("k:A.B", "A.B");
        ab.children = vec![NodeId::new("A.B.D")];
        let abd = node("k:A.B.D", "A.B.D");
        let ac = node("k:A.C", "A.C");
        let mut m = node("k:M", "M");
        m.is_meta = true;
        m.children = vec![NodeId::new("M.X")];
        let mx = node("k:M.X", "M.X");

        let mut ws = workspace_with(vec![a, ab, abd, ac, m, mx]);
        ws.dependencies
            .reverse_edges_by_key
            .insert(NodeKey::new("k:A.C"), vec![edge("A.B.D", "A.C")]);
        ws
    }

    #[test]
    fn visible_rows_walk_depth_first_in_engine_child_order() {
        let ws = sample_workspace();
        let rows = visible_rows(&ws, &HashSet::new(), true);
        let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
        // A's children stay in projection order (C before B) — never sorted.
        assert_eq!(ids, vec!["A", "A.C", "A.B", "A.B.D", "M", "M.X"]);
        let depths: Vec<u32> = rows.iter().map(|row| row.depth).collect();
        assert_eq!(depths, vec![0, 1, 1, 2, 0, 1]);
        assert!(rows[0].has_children);
        assert!(!rows[1].has_children);
        // The dependents badge reads the projection fact directly.
        assert_eq!(rows[1].incoming_count, 1, "A.C has one dependent");
        assert_eq!(rows[0].incoming_count, 0);
        // Meta contagion: M.X is effectively meta through its parent.
        assert!(rows[4].is_meta_effective);
        assert!(rows[5].is_meta_effective);
        assert!(!rows[0].is_meta_effective);
    }

    #[test]
    fn visible_rows_fold_skips_descendants() {
        let ws = sample_workspace();
        let mut collapsed = HashSet::new();
        collapsed.insert(NodeKey::new("k:A.B"));
        let rows = visible_rows(&ws, &collapsed, true);
        let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, vec!["A", "A.C", "A.B", "M", "M.X"]);
        let ab = rows.iter().find(|row| row.id.as_str() == "A.B").unwrap();
        assert!(ab.collapsed);

        // Collapsing a leaf is inert: nothing to hide, row not marked folded.
        let mut leaf_collapsed = HashSet::new();
        leaf_collapsed.insert(NodeKey::new("k:A.C"));
        let rows = visible_rows(&ws, &leaf_collapsed, true);
        assert_eq!(rows.len(), 6);
        let ac = rows.iter().find(|row| row.id.as_str() == "A.C").unwrap();
        assert!(!ac.collapsed);
    }

    #[test]
    fn visible_rows_hide_effective_meta_when_toggled_off() {
        let ws = sample_workspace();
        let rows = visible_rows(&ws, &HashSet::new(), false);
        let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
        // The meta root AND its regular-flagged child both leave the outline.
        assert_eq!(ids, vec!["A", "A.C", "A.B", "A.B.D"]);
    }

    #[test]
    fn duplicate_subtree_intent_lands_beside_the_source() {
        let ws = sample_workspace();
        let branch = ws.node(&NodeId::new("A.B")).unwrap();
        assert_eq!(
            duplicate_subtree_intent(branch),
            WorkspaceIntent::DuplicateSubtree {
                source: NodeKey::new("k:A.B"),
                destination_parent: Some(NodeId::new("A")),
                new_symbol: "B_copy".to_string(),
            }
        );
        // A root subtree duplicates to the workspace root (no parent).
        let root = ws.node(&NodeId::new("A")).unwrap();
        assert_eq!(
            duplicate_subtree_intent(root),
            WorkspaceIntent::DuplicateSubtree {
                source: NodeKey::new("k:A"),
                destination_parent: None,
                new_symbol: "A_copy".to_string(),
            }
        );
    }

    #[test]
    fn copy_subtree_intent_uses_subtree_scope() {
        let ws = sample_workspace();
        let branch = ws.node(&NodeId::new("A.B")).unwrap();
        assert_eq!(
            copy_subtree_intent(branch),
            WorkspaceIntent::CopyToClipboard {
                scope: AuthoringScope::Subtree(NodeKey::new("k:A.B")),
                payload: ClipboardPayloadKind::Subtree,
            }
        );
    }

    #[test]
    fn fold_helpers_update_the_shared_set() {
        let mut collapsed = HashSet::new();
        let key = NodeKey::new("k:A");
        apply_fold(&mut collapsed, &key, true);
        assert!(collapsed.contains(&key));
        apply_fold(&mut collapsed, &key, true);
        assert_eq!(collapsed.len(), 1, "fold is idempotent");
        apply_fold(&mut collapsed, &key, false);
        assert!(collapsed.is_empty());
        toggle_fold(&mut collapsed, &key);
        assert!(collapsed.contains(&key));
        toggle_fold(&mut collapsed, &key);
        assert!(!collapsed.contains(&key));
    }

    #[test]
    fn receipt_error_message_reports_only_rejections() {
        assert_eq!(receipt_error_message(&IntentReceipt::accepted()), None);
        let rejected = IntentReceipt::rejected(IntentError::UnknownNode {
            node: "A.B".to_string(),
        });
        assert_eq!(
            receipt_error_message(&rejected),
            Some("unknown node A.B".to_string())
        );
    }
}
