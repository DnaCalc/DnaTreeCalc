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
//! Embedded for the Phase-A mono-lens core: the **Lens** inspector (selection
//! detail) and the **Console** health/Name-Box strip live inside Flow; they
//! become real companion slots in the Phase-B cockpit.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use dnatreecalc_skin_framework::{
    ATLAS_SPINE_CSS, DerivationInvocationProjection, Dispatcher, KeyChord, KeybindingRegistry,
    NodeId, NodeKey, NodeValueProjection, NodeView, SkinCapabilities, SkinCategory, SkinContext,
    SkinHandle, SkinId, SkinManifest, SkinState, SkinVerb, WorkspaceIntent, WorkspaceRecalcMode,
    WorkspaceSkin, WorkspaceState, calc_state_class, provenance_tint, selection_mode_class,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::value_render::render_value;

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
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        // Record the active lens for cross-lens chrome/continuity. Runs once.
        cx.shared.update(|state| {
            state.active_lens = Some(FLOW_ID.as_str().to_string());
        });
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
fn compute_layout(workspace: &WorkspaceState) -> FlowLayout {
    let layout_keys: Vec<NodeKey> = workspace
        .key_order
        .iter()
        .filter(|key| workspace.node_by_key(key).is_some_and(|node| !node.is_meta))
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
/// the overlay maps 1:1 onto the chip pixels.
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

    let editing = RwSignal::new(false);
    let editor_text = RwSignal::new(String::new());
    let name_box = RwSignal::new(Option::<String>::None);
    let reading_head = RwSignal::new(Option::<usize>::None);

    let selected = Memo::new(move |_| {
        let primary = selection.with(|s| s.primary.clone());
        workspace.with(|ws| primary.as_ref().and_then(|id| ws.node(id).cloned()))
    });

    // Sync the edit buffer to the selection and leave edit mode on reselect.
    Effect::new(move |_| {
        if let Some(node) = selected.get() {
            editor_text.set(node.content_text);
            editing.set(false);
        }
    });

    // The commit path: respects the shared recalc mode, like the other skins.
    let commit_dispatch = dispatch.clone();
    let commit = move || {
        let Some(node) = selected.get_untracked() else {
            return;
        };
        let content = editor_text.get_untracked();
        match shared.get_untracked().recalc_mode {
            WorkspaceRecalcMode::Auto => {
                commit_dispatch.dispatch(WorkspaceIntent::EditContent {
                    node: node.id,
                    content,
                });
                shared.update(|state| state.manual_recalc_pending = false);
            }
            WorkspaceRecalcMode::Manual => {
                commit_dispatch.dispatch(WorkspaceIntent::EditContentDeferred {
                    node: node.id,
                    content,
                });
                shared.update(|state| state.manual_recalc_pending = true);
            }
        }
        editing.set(false);
    };

    // Lens-local grammar, resolved through the one registry. Global verbs
    // (F9, Ctrl+Z/Y, Ctrl+N, lens switch, arrows) bubble to the shell.
    let grammar = KeybindingRegistry::universal();
    let grammar_state = state.clone();
    let root_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if editing.get_untracked() {
            return; // the edit buffer owns keys while editing
        }
        let command = ev.ctrl_key() || ev.meta_key();
        let chord = KeyChord::from_parts(ev.key(), command, ev.alt_key(), ev.shift_key());
        let Some(verb) = grammar.resolve(&chord) else {
            return;
        };
        match verb {
            SkinVerb::Commit => {
                if selected.get_untracked().is_some() {
                    ev.prevent_default();
                    editing.set(true);
                }
            }
            SkinVerb::Explain => {
                ev.prevent_default();
                grammar_state.update(|s| s.explain_open = !s.explain_open);
            }
            SkinVerb::TraceForward => {
                ev.prevent_default();
                grammar_state.update(|s| s.forward_depth = (s.forward_depth + 1) % (MAX_TRACE_DEPTH + 1));
            }
            SkinVerb::TraceBack => {
                ev.prevent_default();
                grammar_state.update(|s| s.back_depth = (s.back_depth + 1) % (MAX_TRACE_DEPTH + 1));
            }
            SkinVerb::NameBox => {
                ev.prevent_default();
                name_box.set(Some(String::new()));
            }
            SkinVerb::Escape => {
                name_box.set(None);
            }
            _ => {} // global verb: leave for the shell
        }
    };

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
        let (back_depth, forward_depth) =
            stage_state.with(|s| (s.back_depth, s.forward_depth));
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

    let css = format!("{ATLAS_SPINE_CSS}\n{FLOW_CSS}");
    let inspector_state = state.clone();
    let console_dispatch = dispatch.clone();

    view! {
        <style>{css}</style>
        <section class="dtc-flow" on:keydown=root_keydown>
            <div class="dtc-flow__body">
                <div class="dtc-flow__stage-scroll">
                    <NameBoxBar name_box=name_box workspace=workspace selection=selection dispatch=dispatch.clone() />
                    <TraceBar state=state.clone() reading_head=reading_head workspace=workspace dispatch=dispatch.clone() />
                    {stage}
                </div>
                <InspectorPanel
                    cx_state=inspector_state
                    workspace=workspace
                    selection=selection
                    editing=editing
                    editor_text=editor_text
                    commit=Arc::new(commit)
                />
            </div>
            <ConsoleBar workspace=workspace selection=selection dispatch=console_dispatch />
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
            on:click=move |_| {
                dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id_for_click.clone())));
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
fn NameBoxBar(
    name_box: RwSignal<Option<String>>,
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<dnatreecalc_skin_framework::SelectionState>,
    dispatch: Arc<dyn Dispatcher>,
) -> impl IntoView {
    let _ = selection;
    let jump_dispatch = dispatch.clone();
    view! {
        {move || {
            name_box.get().map(|query| {
                let jump_dispatch = jump_dispatch.clone();
                let query_for_jump = query.clone();
                view! {
                    <div class="dtc-flow-namebox">
                        <span class="dtc-flow-namebox__sigil">"/"</span>
                        <input
                            class="dtc-flow-namebox__input"
                            autofocus=true
                            placeholder="jump to node…"
                            prop:value=query.clone()
                            on:input=move |ev| name_box.set(Some(event_target_value(&ev)))
                            on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                match ev.key().as_str() {
                                    "Enter" => {
                                        ev.prevent_default();
                                        if let Some(id) = first_match(&workspace.get_untracked(), &query_for_jump) {
                                            jump_dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id)));
                                        }
                                        name_box.set(None);
                                    }
                                    "Escape" => {
                                        ev.prevent_default();
                                        name_box.set(None);
                                    }
                                    _ => {}
                                }
                            }
                        />
                    </div>
                }
                .into_any()
            })
            .unwrap_or_else(|| ().into_any())
        }}
    }
}

fn first_match(workspace: &WorkspaceState, query: &str) -> Option<NodeId> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return None;
    }
    workspace.node_order.iter().find_map(|id| {
        let node = workspace.node(id)?;
        if node.is_meta {
            return None;
        }
        let matches = id.as_str().to_lowercase().contains(&needle)
            || node.display_name.to_lowercase().contains(&needle);
        matches.then(|| id.clone())
    })
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

#[component]
fn InspectorPanel(
    cx_state: dnatreecalc_skin_framework::SkinStateHandle<FlowState>,
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<dnatreecalc_skin_framework::SelectionState>,
    editing: RwSignal<bool>,
    editor_text: RwSignal<String>,
    commit: Arc<dyn Fn() + Send + Sync>,
) -> impl IntoView {
    let detail = Memo::new(move |_| {
        selection.with(|sel| workspace.with(|ws| ws.active_node_detail(sel)))
    });
    let explain_open = {
        let cx_state = cx_state.clone();
        Memo::new(move |_| cx_state.with(|s| s.explain_open))
    };
    let traces = Memo::new(move |_| {
        let owner = detail.with(|d| d.as_ref().map(|d| d.node_key.clone()));
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
    let commit_for_button = commit.clone();
    let commit_for_keys = commit;

    view! {
        <aside class="dtc-flow-inspector" aria-label="Selection inspector">
            {move || match detail.get() {
                None => view! { <p class="dtc-flow-inspector__empty">"Select a node to inspect it."</p> }.into_any(),
                Some(detail) => {
                    let calc = detail.calc_state.map_or("—".to_string(), |state| state.to_string());
                    let format = detail
                        .effective_format
                        .as_ref()
                        .and_then(|fmt| fmt.number_format_code.clone())
                        .unwrap_or_else(|| "General".to_string());
                    let out_refs = detail.outgoing_references.len();
                    let in_refs = detail.incoming_reference_handles.len();
                    let diagnostics = detail.binding_diagnostics.clone();
                    view! {
                        <header class="dtc-flow-inspector__header">
                            <span class="dtc-flow-inspector__name">{detail.display_name.clone()}</span>
                            <code class="dtc-flow-inspector__path">{detail.node.to_string()}</code>
                        </header>
                        <dl class="dtc-flow-inspector__facts">
                            <div><dt>"calc-state"</dt><dd>{calc}</dd></div>
                            <div><dt>"format"</dt><dd>{format}</dd></div>
                            <div><dt>"precedents"</dt><dd>{out_refs}</dd></div>
                            <div><dt>"dependents"</dt><dd>{in_refs}</dd></div>
                        </dl>
                        <div class="dtc-section-label">"Value"</div>
                        {render_value(&detail.value)}
                        <div class="dtc-section-label">"Content"</div>
                        <textarea
                            class="dtc-flow-inspector__edit"
                            class:dtc-flow-inspector__edit--active=move || editing.get()
                            prop:value=move || editor_text.get()
                            on:focus=move |_| editing.set(true)
                            on:input=move |ev| editor_text.set(event_target_value(&ev))
                            on:keydown={
                                let commit_for_keys = commit_for_keys.clone();
                                move |ev: leptos::ev::KeyboardEvent| {
                                    match ev.key().as_str() {
                                        "Enter" if !ev.shift_key() => {
                                            ev.prevent_default();
                                            ev.stop_propagation();
                                            (commit_for_keys)();
                                        }
                                        "Escape" => {
                                            ev.prevent_default();
                                            ev.stop_propagation();
                                            editing.set(false);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        />
                        <button
                            type="button"
                            class="dtc-flow-inspector__commit"
                            on:click={
                                let commit_for_button = commit_for_button.clone();
                                move |_| (commit_for_button)()
                            }
                        >
                            "Commit (Enter)"
                        </button>
                        {(!diagnostics.is_empty()).then(|| view! {
                            <div class="dtc-section-label">"Binding diagnostics"</div>
                            <ul class="dtc-flow-inspector__diagnostics">
                                {diagnostics.into_iter().map(|diag| view! {
                                    <li>{diag.message}</li>
                                }).collect::<Vec<_>>()}
                            </ul>
                        }.into_any())}
                        <div class="dtc-flow-inspector__explain-head">
                            <div class="dtc-section-label">"Explain (E)"</div>
                        </div>
                        {move || explain_open.get().then(|| {
                            let traces = traces.get();
                            if traces.is_empty() {
                                view! { <p class="dtc-flow-inspector__empty">"No derivation trace for this node yet — recalculate."</p> }.into_any()
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
                    }
                    .into_any()
                }
            }}
        </aside>
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

#[component]
fn ConsoleBar(
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<dnatreecalc_skin_framework::SelectionState>,
    dispatch: Arc<dyn Dispatcher>,
) -> impl IntoView {
    let _ = selection;
    let health = Memo::new(move |_| {
        workspace.with(|ws| {
            let mut clean = 0usize;
            let mut stale = 0usize;
            let mut error = 0usize;
            for node in ws.nodes_by_key.values() {
                if node.is_meta {
                    continue;
                }
                if matches!(node.computed_value, NodeValueProjection::Error(_)) {
                    error += 1;
                }
                let class = calc_state_class(node.calc_state);
                if class == "dtc-calc--clean" {
                    clean += 1;
                } else if class == "dtc-calc--stale" || class == "dtc-calc--unknown" {
                    stale += 1;
                }
            }
            (ws.nodes_by_key.values().filter(|n| !n.is_meta).count(), clean, stale, error)
        })
    });
    let recalc_dispatch = dispatch.clone();

    view! {
        <footer class="dtc-flow-console" aria-label="Model health">
            <span class="dtc-flow-console__group">
                <span class="dtc-calc-dot dtc-calc--clean"></span>
                <span>{move || format!("{} clean", health.get().1)}</span>
            </span>
            <span class="dtc-flow-console__group">
                <span class="dtc-calc-dot dtc-calc--stale"></span>
                <span>{move || format!("{} stale", health.get().2)}</span>
            </span>
            <span class="dtc-flow-console__group">
                <span class="dtc-calc-dot dtc-calc--rejected"></span>
                <span>{move || format!("{} error", health.get().3)}</span>
            </span>
            <span class="dtc-flow-tracebar__spacer"></span>
            <span class="dtc-flow-console__group">{move || format!("{} nodes", health.get().0)}</span>
            <button type="button" on:click=move |_| { recalc_dispatch.dispatch(WorkspaceIntent::Recalculate); }>
                "Recalculate (F9)"
            </button>
        </footer>
    }
}

const FLOW_CSS: &str = r#"
.dtc-flow {
  display: flex; flex-direction: column; height: 100%;
  background: var(--dtc-surface); color: var(--dtc-text);
  font: 13px/1.4 var(--dtc-font, system-ui, sans-serif);
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

.dtc-flow-tracebar, .dtc-flow-namebox {
  display: flex; align-items: center; gap: 8px; padding: 6px 8px; margin-bottom: 8px;
  background: var(--dtc-surface-subtle); border: 1px solid var(--dtc-border-muted); border-radius: 6px;
}
.dtc-flow-tracebar__group { display: inline-flex; align-items: center; gap: 6px; }
.dtc-flow-tracebar__label { color: var(--dtc-text-subtle); text-transform: uppercase; font-size: 11px; letter-spacing: 0.04em; }
.dtc-flow-tracebar__count { font-variant-numeric: tabular-nums; color: var(--dtc-text-muted); }
.dtc-flow-tracebar__spacer { flex: 1; }
.dtc-flow-tracebar button, .dtc-flow-console button, .dtc-flow-inspector__commit {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 2px 8px; cursor: pointer; color: var(--dtc-text);
}
.dtc-flow-namebox__sigil { font-weight: 700; color: var(--dtc-accent); }
.dtc-flow-namebox__input { flex: 1; padding: 3px 6px; border: 1px solid var(--dtc-border); border-radius: 5px; background: var(--dtc-surface-input); color: var(--dtc-text); }

.dtc-flow-inspector {
  width: 320px; flex-shrink: 0; overflow: auto; padding: 12px;
  border-left: 1px solid var(--dtc-border-muted); background: var(--dtc-surface-panel);
}
.dtc-flow-inspector__empty { color: var(--dtc-text-subtle); font-style: italic; }
.dtc-flow-inspector__header { display: flex; flex-direction: column; gap: 2px; margin-bottom: 8px; }
.dtc-flow-inspector__name { font-weight: 700; font-size: 15px; }
.dtc-flow-inspector__path { color: var(--dtc-text-subtle); font-size: 12px; }
.dtc-flow-inspector__facts { display: grid; grid-template-columns: 1fr 1fr; gap: 4px 12px; margin: 0 0 8px; }
.dtc-flow-inspector__facts div { display: flex; justify-content: space-between; }
.dtc-flow-inspector__facts dt { color: var(--dtc-text-subtle); }
.dtc-flow-inspector__facts dd { margin: 0; font-variant-numeric: tabular-nums; }
.dtc-flow-inspector__edit { width: 100%; min-height: 60px; box-sizing: border-box; padding: 6px; border: 1px solid var(--dtc-border); border-radius: 5px; background: var(--dtc-surface-input); color: var(--dtc-text); font-family: ui-monospace, monospace; }
.dtc-flow-inspector__edit--active { border-color: var(--dtc-warning); box-shadow: inset 0 0 0 1px var(--dtc-warning); }
.dtc-flow-inspector__commit { margin-top: 6px; }
.dtc-flow-inspector__diagnostics { margin: 4px 0; padding-left: 16px; color: var(--dtc-danger-text); }
.dtc-flow-explain__result { font-weight: 600; margin-bottom: 4px; }
.dtc-flow-explain__tree { list-style: none; padding-left: 14px; margin: 2px 0; border-left: 1px dashed var(--dtc-border); }
.dtc-flow-explain__node { padding: 1px 0; }

.dtc-section-label { margin: 10px 0 4px; font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--dtc-text-subtle); }

.dtc-flow-console {
  display: flex; align-items: center; gap: 14px; padding: 6px 12px;
  border-top: 1px solid var(--dtc-border-muted); background: var(--dtc-surface-subtle);
}
.dtc-flow-console__group { display: inline-flex; align-items: center; gap: 6px; color: var(--dtc-text-muted); }
.dtc-value-display { padding: 4px 0; font-variant-numeric: tabular-nums; }
.dtc-value-display--error { color: var(--dtc-danger-text); }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::{
        DependencyEdgeProjection, DependencyKindProjection, NodeContentKind,
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
