//! FLOW — the ATLAS reference lens.
//!
//! Flow renders the model as a dependency-flow "sentence": inputs on the left,
//! results on the right, laid out in structure-stable layered columns with
//! dependency wires between them. It is the lens every other lens conforms to,
//! so it is the canonical realization of the shared spine:
//!
//! - **one grammar** — F9 recalc, `]`/`[` trace depth, `E` explain, `/`
//!   Name-Box, Enter/Esc modeless edit (resolved through `KeybindingRegistry`);
//! - **one styling** — `calc_state` is the only saturated channel
//!   (`calc_state_class`), provenance is structural (`provenance_tint`), and a
//!   1-bit SELECTED/EDITING border (`selection_mode_class`);
//! - **one continuity** — selection + `focus_key` are shared view-state, so a
//!   later lens lands on the same node.
//!
//! Everything Flow shows is read from the published projection; every change it
//! makes goes through the closed `WorkspaceIntent` surface. It never parses
//! formula text, fabricates a value, or uses grid coordinates.
//!
//! The embedded **Lens** inspector and **Console** strip are the shared
//! [`crate::spine_widgets`] components — identical in every Phase-A mono-lens,
//! promoted to companion slots in the Phase-B cockpit.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use dnatreecalc_skin_framework::{
    ATLAS_SPINE_CSS, DerivationInvocationProjection, Dispatcher, KeyChord, KeybindingRegistry,
    NodeId, NodeKey, NodeView, SharedStateOrigin, SkinCapabilities, SkinCategory, SkinContext,
    SkinHandle, SkinId, SkinManifest, SkinMountSlot, SkinState, SkinVerb, WorkspaceIntent,
    WorkspaceSkin, WorkspaceState, calc_state_class, provenance_tint, selection_mode_class,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::spine_widgets::{
    ConsoleBar, NameBoxBar, NodeInspector, SPINE_WIDGETS_CSS, clear_content_edit,
    commit_content_edit, event_target_is_interactive, event_target_is_text_entry,
    focus_edit_buffer,
};

pub const FLOW_ID: SkinId = SkinId::new("flow");

// Layout geometry. Cells are a fixed grid so wire endpoints can be computed
// analytically and the SVG overlay maps 1:1 to chip pixels (the proto-09 fix).
const COL_W: f64 = 220.0;
const ROW_H: f64 = 76.0;
const CHIP_W: f64 = 176.0;
const CHIP_H: f64 = 52.0;
const PAD_X: f64 = 22.0;
const PAD_Y: f64 = 14.0;
const MAX_TRACE_DEPTH: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowState {
    /// How many dependency hops toward dependents (`]`) to highlight.
    pub forward_depth: u32,
    /// How many dependency hops toward precedents (`[`) to highlight.
    pub back_depth: u32,
    /// Whether the explain panel is open (`E`).
    pub explain_open: bool,
    /// Whether the inspector (embedded Lens) is open.
    pub inspector_open: bool,
}

impl Default for FlowState {
    fn default() -> Self {
        Self {
            forward_depth: 1,
            back_depth: 1,
            explain_open: false,
            inspector_open: true,
        }
    }
}

impl SkinState for FlowState {
    fn schema_version() -> u32 {
        1
    }
}

#[derive(Default)]
pub struct FlowLens;

impl FlowLens {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSkin for FlowLens {
    type State = FlowState;

    fn id(&self) -> SkinId {
        FLOW_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Flow",
            description: "Dependency-flow lens: layered calc theatre, trace, and explain.",
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
            renders_table_values: false,
            allowed_slots: None,
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        crate::spine_widgets::stamp_active_lens(cx.shared, FLOW_ID, cx.slot);
        SkinHandle::new(view! { <FlowView cx=cx /> }.into_any())
    }
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
struct FlowCell {
    col: usize,
    row: usize,
    x: f64,
    y: f64,
}

#[derive(Clone, PartialEq)]
struct FlowLayout {
    cells: BTreeMap<NodeKey, FlowCell>,
    width: f64,
    height: f64,
}

/// Structure-stable layered layout: column = longest precedent-chain rank, so
/// inputs sit left and results right. Depends only on topology, not values.
/// Effectively-meta nodes (self or any ancestor) stay off the stage.
fn compute_layout(workspace: &WorkspaceState) -> FlowLayout {
    let layout_keys: Vec<NodeKey> = workspace
        .key_order
        .iter()
        .filter(|key| {
            workspace.node_by_key(key).is_some() && !workspace.is_effective_meta(key)
        })
        .cloned()
        .collect();
    let key_set: BTreeSet<NodeKey> = layout_keys.iter().cloned().collect();
    let edges = &workspace.dependencies.edges_by_owner_key;

    let mut memo: BTreeMap<NodeKey, u32> = BTreeMap::new();
    for key in &layout_keys {
        let mut visiting = BTreeSet::new();
        let rank = rank_of(key, edges, &key_set, &mut memo, &mut visiting);
        memo.insert(key.clone(), rank);
    }

    let max_rank = layout_keys
        .iter()
        .map(|key| memo.get(key).copied().unwrap_or(0))
        .max()
        .unwrap_or(0);
    let mut columns: Vec<Vec<NodeKey>> = vec![Vec::new(); max_rank as usize + 1];
    for key in &layout_keys {
        let rank = memo.get(key).copied().unwrap_or(0) as usize;
        columns[rank].push(key.clone());
    }

    let mut cells = BTreeMap::new();
    let mut max_rows = 0usize;
    for (col, keys) in columns.iter().enumerate() {
        max_rows = max_rows.max(keys.len());
        for (row, key) in keys.iter().enumerate() {
            cells.insert(
                key.clone(),
                FlowCell {
                    col,
                    row,
                    x: col as f64 * COL_W + PAD_X,
                    y: row as f64 * ROW_H + PAD_Y,
                },
            );
        }
    }

    FlowLayout {
        cells,
        width: columns.len().max(1) as f64 * COL_W,
        height: max_rows.max(1) as f64 * ROW_H,
    }
}

fn rank_of(
    key: &NodeKey,
    edges: &BTreeMap<NodeKey, Vec<dnatreecalc_skin_framework::DependencyEdgeProjection>>,
    key_set: &BTreeSet<NodeKey>,
    memo: &mut BTreeMap<NodeKey, u32>,
    visiting: &mut BTreeSet<NodeKey>,
) -> u32 {
    if let Some(rank) = memo.get(key) {
        return *rank;
    }
    if !visiting.insert(key.clone()) {
        return 0; // cycle: break the recursion
    }
    let rank = edges.get(key).map_or(0, |outgoing| {
        outgoing
            .iter()
            .filter(|edge| key_set.contains(&edge.target_key))
            .map(|edge| 1 + rank_of(&edge.target_key, edges, key_set, memo, visiting))
            .max()
            .unwrap_or(0)
    });
    visiting.remove(key);
    memo.insert(key.clone(), rank);
    rank
}

/// The set of keys on the trace from `focus`: precedents up to `back_depth`
/// and dependents up to `forward_depth`.
fn trace_keys(
    workspace: &WorkspaceState,
    focus: &NodeKey,
    back_depth: u32,
    forward_depth: u32,
) -> BTreeSet<NodeKey> {
    let mut set = BTreeSet::new();
    set.insert(focus.clone());

    let mut frontier = vec![focus.clone()];
    for _ in 0..back_depth {
        let mut next = Vec::new();
        for key in &frontier {
            if let Some(edges) = workspace.dependencies.edges_by_owner_key.get(key) {
                for edge in edges {
                    if set.insert(edge.target_key.clone()) {
                        next.push(edge.target_key.clone());
                    }
                }
            }
        }
        frontier = next;
    }

    let mut frontier = vec![focus.clone()];
    for _ in 0..forward_depth {
        let mut next = Vec::new();
        for key in &frontier {
            if let Some(edges) = workspace.dependencies.reverse_edges_by_key.get(key) {
                for edge in edges {
                    if set.insert(edge.owner_key.clone()) {
                        next.push(edge.owner_key.clone());
                    }
                }
            }
        }
        frontier = next;
    }
    set
}

/// Build the dependency-wire SVG as markup (injected via `inner_html` to avoid
/// SVG-namespace handling). Endpoints are computed from the fixed cell grid, so
/// the overlay maps 1:1 onto the chip pixels. The markup contains only numeric
/// coordinates and fixed class names — never node-supplied text.
fn wires_svg(workspace: &WorkspaceState, layout: &FlowLayout, trace: &BTreeSet<NodeKey>) -> String {
    let mut paths = String::new();
    for (owner_key, owner_cell) in &layout.cells {
        let Some(edges) = workspace.dependencies.edges_by_owner_key.get(owner_key) else {
            continue;
        };
        for edge in edges {
            let Some(target_cell) = layout.cells.get(&edge.target_key) else {
                continue;
            };
            // Wire from precedent (target) right edge to dependent (owner) left edge.
            let x1 = target_cell.x + CHIP_W;
            let y1 = target_cell.y + CHIP_H / 2.0;
            let x2 = owner_cell.x;
            let y2 = owner_cell.y + CHIP_H / 2.0;
            let cx = (x1 + x2) / 2.0;
            let on_trace = trace.contains(owner_key) && trace.contains(&edge.target_key);
            let class = if on_trace {
                "dtc-flow-wire dtc-flow-wire--trace"
            } else {
                "dtc-flow-wire"
            };
            paths.push_str(&format!(
                "<path class=\"{class}\" d=\"M {x1:.1} {y1:.1} C {cx:.1} {y1:.1} {cx:.1} {y2:.1} {x2:.1} {y2:.1}\" />"
            ));
        }
    }
    format!(
        "<svg class=\"dtc-flow-wires\" width=\"{w:.0}\" height=\"{h:.0}\" viewBox=\"0 0 {w:.0} {h:.0}\" preserveAspectRatio=\"xMinYMin meet\">{paths}</svg>",
        w = layout.width,
        h = layout.height,
    )
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

#[component]
fn FlowView(cx: SkinContext<FlowState>) -> impl IntoView {
    let workspace = cx.workspace;
    let selection = cx.selection;
    let shared = cx.shared;
    let dispatch = cx.dispatch.clone();
    let state = cx.state.clone();
    let shared_origin = SharedStateOrigin::lens(FLOW_ID, cx.slot);

    let editing = RwSignal::new(false);
    let editor_text = RwSignal::new(String::new());
    let name_box = RwSignal::new(Option::<String>::None);
    let reading_head = RwSignal::new(Option::<usize>::None);
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
        }
    };

    // Lens-local grammar, resolved through the one registry. Global verbs
    // (F9, Ctrl+Z/Y, Ctrl+N, lens switch, arrows) bubble to the shell.
    let grammar = KeybindingRegistry::universal();
    let grammar_state = state.clone();
    let clear_dispatch = dispatch.clone();
    let clear_origin = shared_origin.clone();
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
            SkinVerb::Commit | SkinVerb::EditInPlace => {
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
            SkinVerb::ClearContents => {
                // Excel's Delete: clear the selected node's contents (safe,
                // non-structural — never a node removal).
                if event_target_is_interactive(&ev) {
                    return;
                }
                if let Some(node) = selected.get_untracked() {
                    ev.prevent_default();
                    clear_content_edit(&clear_dispatch, shared, &clear_origin, node.id.clone());
                }
            }
            SkinVerb::Explain => {
                ev.prevent_default();
                grammar_state.update(|s| s.explain_open = !s.explain_open);
            }
            SkinVerb::TraceForward => {
                ev.prevent_default();
                grammar_state
                    .update(|s| s.forward_depth = (s.forward_depth + 1) % (MAX_TRACE_DEPTH + 1));
            }
            SkinVerb::TraceBack => {
                ev.prevent_default();
                grammar_state
                    .update(|s| s.back_depth = (s.back_depth + 1) % (MAX_TRACE_DEPTH + 1));
            }
            SkinVerb::NameBox => {
                ev.prevent_default();
                name_box.set(Some(String::new()));
            }
            SkinVerb::Escape => {
                name_box.set(None);
                editing.set(false);
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

    let stage_workspace = workspace;
    let stage_selection = selection;
    let stage_state = state.clone();
    let stage_dispatch = dispatch.clone();
    let stage = move || {
        let ws = stage_workspace.get();
        let sel_key = stage_selection.with(|s| {
            s.primary
                .as_ref()
                .and_then(|id| ws.node(id))
                .map(|node| node.key.clone())
        });
        let editing_now = editing.get();
        let (back_depth, forward_depth) = stage_state.with(|s| (s.back_depth, s.forward_depth));
        let layout = compute_layout(&ws);
        let trace = match &sel_key {
            Some(focus) => trace_keys(&ws, focus, back_depth, forward_depth),
            None => BTreeSet::new(),
        };
        let svg = wires_svg(&ws, &layout, &trace);

        // Reading-head: map eval order to a sweep index.
        let head = reading_head.get();
        let eval_index: BTreeMap<NodeId, usize> = ws
            .last_run
            .as_ref()
            .map(|run| {
                run.evaluation_order
                    .iter()
                    .enumerate()
                    .map(|(index, id)| (id.clone(), index))
                    .collect()
            })
            .unwrap_or_default();

        let chips = layout
            .cells
            .iter()
            .filter_map(|(key, cell)| {
                let node = ws.node_by_key(key)?.clone();
                Some(chip_view(
                    node,
                    cell,
                    sel_key.as_ref() == Some(key),
                    editing_now && sel_key.as_ref() == Some(key),
                    trace.contains(key),
                    head,
                    &eval_index,
                    &ws,
                    stage_dispatch.clone(),
                    stage_ref,
                ))
            })
            .collect::<Vec<_>>();

        let style = format!("width:{:.0}px;height:{:.0}px;", layout.width, layout.height);
        view! {
            <div class="dtc-flow-stage" style=style>
                <div class="dtc-flow-wires-layer" inner_html=svg></div>
                {chips}
            </div>
        }
    };

    let css = format!("{ATLAS_SPINE_CSS}\n{SPINE_WIDGETS_CSS}\n{FLOW_CSS}");
    let explain_state = state.clone();
    let console_dispatch = dispatch.clone();

    // The explain stack renders as the lens-specific extra inside the shared
    // inspector.
    let explain_detail = Memo::new(move |_| {
        selection.with(|sel| {
            workspace.with(|ws| ws.active_node_detail(sel).map(|d| d.node_key))
        })
    });
    let traces = Memo::new(move |_| {
        let owner = explain_detail.get();
        workspace.with(|ws| {
            let Some(owner) = owner else {
                return Vec::new();
            };
            ws.last_run
                .as_ref()
                .map(|run| {
                    run.derivation_traces
                        .iter()
                        .filter(|trace| trace.owner_key == owner)
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
    });
    let explain_open = {
        let explain_state = explain_state.clone();
        Memo::new(move |_| explain_state.with(|s| s.explain_open))
    };

    let commit_arc: Arc<dyn Fn() + Send + Sync> = Arc::new(commit);

    view! {
        <style>{css}</style>
        <section class="dtc-flow" tabindex="0" node_ref=stage_ref on:keydown=root_keydown>
            <div class="dtc-flow__body">
                <div class="dtc-flow__stage-scroll">
                    <NameBoxBar name_box=name_box workspace=workspace dispatch=dispatch.clone() />
                    <TraceBar state=state.clone() reading_head=reading_head workspace=workspace dispatch=dispatch.clone() />
                    {stage}
                </div>
                <NodeInspector
                    workspace=workspace
                    selection=selection
                    editing=editing
                    editor_text=editor_text
                    edit_ref=edit_ref
                    commit=commit_arc
                    shared=shared
                    preview=cx.preview.clone()
                >
                    {move || {
                        view! {
                            <div class="dtc-flow-explain-block">
                                <div class="dtc-section-label">"Explain (E)"</div>
                                {move || explain_open.get().then(|| {
                                    let traces = traces.get();
                                    if traces.is_empty() {
                                        view! { <p class="dtc-inspector__empty">"No derivation trace for this node yet — recalculate."</p> }.into_any()
                                    } else {
                                        view! {
                                            <div class="dtc-flow-explain">
                                                {traces.into_iter().map(|trace| view! {
                                                    <div class="dtc-flow-explain__trace">
                                                        <div class="dtc-flow-explain__result">
                                                            "= "{trace.kernel_returned_value.clone()}
                                                        </div>
                                                        <ul class="dtc-flow-explain__tree">
                                                            {trace.sub_invocation_tree.iter().map(render_invocation).collect::<Vec<_>>()}
                                                        </ul>
                                                    </div>
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        }.into_any()
                                    }
                                })}
                            </div>
                        }
                        .into_any()
                    }}
                </NodeInspector>
            </div>
            <ConsoleBar workspace=workspace dispatch=console_dispatch shared=shared />
        </section>
    }
}

#[allow(clippy::too_many_arguments)]
fn chip_view(
    node: NodeView,
    cell: &FlowCell,
    is_selected: bool,
    is_editing: bool,
    on_trace: bool,
    head: Option<usize>,
    eval_index: &BTreeMap<NodeId, usize>,
    workspace: &WorkspaceState,
    dispatch: Arc<dyn Dispatcher>,
    stage_ref: NodeRef<leptos::html::Section>,
) -> AnyView {
    let calc_class = calc_state_class(node.calc_state);
    let prov = provenance_tint(&node, workspace);
    let sel_class = selection_mode_class(is_selected, is_editing);
    let value_text = node.computed_value.display_text();
    let index = eval_index.get(&node.id).copied();
    let (swept, is_head) = match (head, index) {
        (Some(h), Some(i)) => (i <= h, i == h),
        _ => (true, false),
    };
    let mut classes: Vec<&str> = vec!["dtc-flow-chip", calc_class, prov.css_class()];
    if on_trace {
        classes.push("dtc-flow-chip--trace");
    }
    if !swept {
        classes.push("dtc-flow-chip--dim");
    }
    if is_head {
        classes.push("dtc-flow-chip--head");
    }
    if !sel_class.is_empty() {
        classes.push(sel_class);
    }
    let class_attr = classes.join(" ");
    let style = format!(
        "left:{:.0}px;top:{:.0}px;width:{:.0}px;min-height:{:.0}px;",
        cell.x, cell.y, CHIP_W, CHIP_H
    );
    let id_for_click = node.id.clone();
    let key_for_click = node.key.clone();

    view! {
        <button
            type="button"
            class=class_attr
            style=style
            tabindex="-1"
            // Chips never take keyboard focus: focus stays on the stage so the
            // grammar keeps working and Space/Enter can never re-activate a
            // stale chip after arrow navigation.
            on:mousedown=move |ev| ev.prevent_default()
            on:click=move |_| {
                dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id_for_click.clone())));
                if let Some(section) = stage_ref.get_untracked() {
                    let _ = section.focus();
                }
            }
            title=node.key.to_string()
            data-node-key=key_for_click.to_string()
        >
            <span class="dtc-flow-chip__name">
                <span class="dtc-calc-dot"></span>
                {node.display_name.clone()}
            </span>
            <span class="dtc-flow-chip__value">{value_text}</span>
        </button>
    }
    .into_any()
}

#[component]
fn TraceBar(
    state: dnatreecalc_skin_framework::SkinStateHandle<FlowState>,
    reading_head: RwSignal<Option<usize>>,
    workspace: ReadSignal<WorkspaceState>,
    dispatch: Arc<dyn Dispatcher>,
) -> impl IntoView {
    let back = {
        let state = state.clone();
        Memo::new(move |_| state.with(|s| s.back_depth))
    };
    let forward = {
        let state = state.clone();
        Memo::new(move |_| state.with(|s| s.forward_depth))
    };
    let eval_len = Memo::new(move |_| {
        workspace.with(|ws| {
            ws.last_run
                .as_ref()
                .map_or(0, |run| run.evaluation_order.len())
        })
    });
    let recalc_dispatch = dispatch.clone();
    let sweep_reset = reading_head;

    view! {
        <div class="dtc-flow-tracebar" aria-label="Trace and reading head">
            <span class="dtc-flow-tracebar__group">
                <span class="dtc-flow-tracebar__label">"trace"</span>
                <kbd>"["</kbd>
                <span class="dtc-flow-tracebar__count">{move || format!("← {}", back.get())}</span>
                <span class="dtc-flow-tracebar__count">{move || format!("{} →", forward.get())}</span>
                <kbd>"]"</kbd>
            </span>
            <span class="dtc-flow-tracebar__spacer"></span>
            <span class="dtc-flow-tracebar__group">
                <span class="dtc-flow-tracebar__label">"reading head"</span>
                <button
                    type="button"
                    on:click=move |_| {
                        recalc_dispatch.dispatch(WorkspaceIntent::Recalculate);
                        sweep_reset.set(Some(0));
                    }
                >
                    "F9 sweep"
                </button>
                <button
                    type="button"
                    on:click=move |_| reading_head.update(|head| {
                        if let Some(value) = head {
                            *value = value.saturating_sub(1);
                        }
                    })
                >
                    "◀"
                </button>
                <span class="dtc-flow-tracebar__count">
                    {move || match reading_head.get() {
                        Some(value) => format!("{}/{}", value + 1, eval_len.get().max(1)),
                        None => "off".to_string(),
                    }}
                </span>
                <button
                    type="button"
                    on:click=move |_| {
                        let len = eval_len.get_untracked();
                        if len == 0 {
                            return; // no run yet — nothing to sweep
                        }
                        reading_head.update(|head| {
                            let next = head.map_or(0, |value| value + 1);
                            *head = Some(next.min(len.saturating_sub(1)));
                        });
                    }
                >
                    "▶"
                </button>
                <button type="button" on:click=move |_| reading_head.set(None)>"stop"</button>
            </span>
        </div>
    }
}

fn render_invocation(invocation: &DerivationInvocationProjection) -> AnyView {
    let label = format!(
        "{} → {}",
        invocation.function_name,
        invocation
            .kernel_returned_value
            .clone()
            .unwrap_or_else(|| "…".to_string())
    );
    let children = invocation.children.clone();
    view! {
        <li class="dtc-flow-explain__node">
            <span>{label}</span>
            {(!children.is_empty()).then(|| view! {
                <ul class="dtc-flow-explain__tree">
                    {children.iter().map(render_invocation).collect::<Vec<_>>()}
                </ul>
            }.into_any())}
        </li>
    }
    .into_any()
}

const FLOW_CSS: &str = r#"
.dtc-flow {
  display: flex; flex-direction: column; height: 100%;
  background: var(--dtc-surface); color: var(--dtc-text);
  font: 13px/1.4 var(--dtc-font, system-ui, sans-serif);
  outline: none;
}
.dtc-flow__body { display: flex; flex: 1; min-height: 0; }
.dtc-flow__stage-scroll { flex: 1; overflow: auto; padding: 8px 12px; min-width: 0; }
.dtc-flow-stage { position: relative; }
.dtc-flow-wires-layer { position: absolute; inset: 0; pointer-events: none; }
.dtc-flow-wires { position: absolute; inset: 0; overflow: visible; }
.dtc-flow-wire { fill: none; stroke: var(--dtc-border-strong); stroke-width: 1.25; opacity: 0.5; }
.dtc-flow-wire--trace { stroke: var(--dtc-accent); stroke-width: 2; opacity: 0.95; }

.dtc-flow-chip {
  position: absolute; box-sizing: border-box; display: flex; flex-direction: column;
  justify-content: center; gap: 2px; padding: 6px 10px; text-align: left;
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border);
  border-radius: 7px; cursor: pointer; box-shadow: 0 1px 2px var(--dtc-shadow-soft);
  transition: opacity 0.12s ease;
}
.dtc-flow-chip:hover { border-color: var(--dtc-border-strong); }
.dtc-flow-chip--trace { border-color: var(--dtc-accent-border); }
.dtc-flow-chip--dim { opacity: 0.32; }
.dtc-flow-chip--head { box-shadow: 0 0 0 2px var(--dtc-accent), 0 1px 3px var(--dtc-shadow-medium); }
.dtc-flow-chip__name { display: flex; align-items: center; gap: 6px; font-weight: 600; color: var(--dtc-text); }
.dtc-flow-chip__value { font-variant-numeric: tabular-nums; color: var(--dtc-text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

.dtc-flow-tracebar {
  display: flex; align-items: center; gap: 8px; padding: 6px 8px; margin-bottom: 8px;
  background: var(--dtc-surface-subtle); border: 1px solid var(--dtc-border-muted); border-radius: 6px;
}
.dtc-flow-tracebar__group { display: inline-flex; align-items: center; gap: 6px; }
.dtc-flow-tracebar__label { color: var(--dtc-text-subtle); text-transform: uppercase; font-size: 11px; letter-spacing: 0.04em; }
.dtc-flow-tracebar__count { font-variant-numeric: tabular-nums; color: var(--dtc-text-muted); }
.dtc-flow-tracebar__spacer { flex: 1; }
.dtc-flow-tracebar button {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 2px 8px; cursor: pointer; color: var(--dtc-text);
}

.dtc-flow-explain__result { font-weight: 600; margin-bottom: 4px; }
.dtc-flow-explain__tree { list-style: none; padding-left: 14px; margin: 2px 0; border-left: 1px dashed var(--dtc-border); }
.dtc-flow-explain__node { padding: 1px 0; }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spine_widgets::first_match;
    use dnatreecalc_skin_framework::{
        DependencyEdgeProjection, DependencyKindProjection, NodeContentKind, NodeValueProjection,
    };

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

    /// A → B → C: B depends on A, C depends on B.
    fn linear_workspace() -> WorkspaceState {
        let mut ws = WorkspaceState::default();
        for (key, path) in [("k:A", "A"), ("k:B", "B"), ("k:C", "C")] {
            let n = node(key, path);
            ws.node_order.push(n.id.clone());
            ws.key_order.push(n.key.clone());
            ws.paths_by_key.insert(n.key.clone(), n.id.clone());
            ws.nodes.insert(n.id.clone(), n.clone());
            ws.nodes_by_key.insert(n.key.clone(), n);
        }
        ws.dependencies
            .edges_by_owner_key
            .insert(NodeKey::new("k:B"), vec![edge("B", "A")]);
        ws.dependencies
            .edges_by_owner_key
            .insert(NodeKey::new("k:C"), vec![edge("C", "B")]);
        ws.dependencies
            .reverse_edges_by_key
            .insert(NodeKey::new("k:A"), vec![edge("B", "A")]);
        ws.dependencies
            .reverse_edges_by_key
            .insert(NodeKey::new("k:B"), vec![edge("C", "B")]);
        ws
    }

    #[test]
    fn layout_ranks_inputs_left_and_results_right() {
        let ws = linear_workspace();
        let layout = compute_layout(&ws);
        assert_eq!(layout.cells[&NodeKey::new("k:A")].col, 0);
        assert_eq!(layout.cells[&NodeKey::new("k:B")].col, 1);
        assert_eq!(layout.cells[&NodeKey::new("k:C")].col, 2);
    }

    #[test]
    fn layout_excludes_effective_meta_descendants() {
        let mut ws = linear_workspace();
        // Make A a meta node with a regular-flagged child: both must leave the
        // stage (effective-meta contagion), B/C stay.
        ws.nodes_by_key.get_mut(&NodeKey::new("k:A")).unwrap().is_meta = true;
        ws.nodes.get_mut(&NodeId::new("A")).unwrap().is_meta = true;
        let mut child = node("k:AC", "A.Child");
        child.parent = Some(NodeId::new("A"));
        ws.node_order.push(child.id.clone());
        ws.key_order.push(child.key.clone());
        ws.paths_by_key.insert(child.key.clone(), child.id.clone());
        ws.nodes.insert(child.id.clone(), child.clone());
        ws.nodes_by_key.insert(child.key.clone(), child);

        let layout = compute_layout(&ws);
        assert!(!layout.cells.contains_key(&NodeKey::new("k:A")));
        assert!(!layout.cells.contains_key(&NodeKey::new("k:AC")));
        assert!(layout.cells.contains_key(&NodeKey::new("k:B")));
        assert!(layout.cells.contains_key(&NodeKey::new("k:C")));
    }

    #[test]
    fn trace_includes_precedents_and_dependents_to_depth() {
        let ws = linear_workspace();
        let trace = trace_keys(&ws, &NodeKey::new("k:B"), 1, 1);
        assert!(trace.contains(&NodeKey::new("k:A")), "precedent A");
        assert!(trace.contains(&NodeKey::new("k:B")), "focus B");
        assert!(trace.contains(&NodeKey::new("k:C")), "dependent C");

        // Depth 0 in both directions is just the focus.
        let solo = trace_keys(&ws, &NodeKey::new("k:B"), 0, 0);
        assert_eq!(solo.len(), 1);
        assert!(solo.contains(&NodeKey::new("k:B")));
    }

    #[test]
    fn name_box_first_match_is_case_insensitive() {
        let ws = linear_workspace();
        assert_eq!(first_match(&ws, "c"), Some(NodeId::new("C")));
        assert_eq!(first_match(&ws, "ZZ"), None);
        assert_eq!(first_match(&ws, ""), None);
    }
}
