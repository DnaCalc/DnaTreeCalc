//! BENCH — the ATLAS scenario bench / wind-tunnel lens.
//!
//! Bench is the consequence-free what-if surface over engine candidates: a
//! scenario rail (Base + host-managed scenarios), a typed override panel for
//! the selected node, a side-by-side comparison grid over the host's
//! comparative projection, and direct sensitivity sweeps with a series strip.
//!
//! Everything Bench shows is read from the published projection
//! (`scenarios`/`sweeps`/`comparison`/`series`); every change goes through the
//! closed `WorkspaceIntent` scenario surface. Bench never evaluates anything
//! itself — an unevaluated comparison cell renders a dash, never a fabricated
//! number — and override input classification is input handling only (the
//! host literalizes through OxFml).
//!
//! Shared-spine conformance (see [`crate::flow`], the reference lens):
//!
//! - **one grammar** — `/` Name-Box, Enter/Esc modeless edit, resolved through
//!   `KeybindingRegistry`; bare keys never fire while typing;
//! - **one styling** — `calc_state` is the only saturated channel, scenario
//!   and sweep values carry the structural `dtc-prov--scenario` tint;
//! - **one continuity** — selection rides the shared dispatcher, so jumping
//!   from an override row lands every other lens on the same node.

use std::collections::BTreeSet;
use std::sync::Arc;

use dnatreecalc_skin_framework::{
    ATLAS_SPINE_CSS, ComparativeSourceProjection, Dispatcher, IntentReceipt, KeyChord,
    KeybindingRegistry, NodeId, NodeKey, NodeValueProjection, NodeView, ScenarioProjection,
    SelectionState, SeriesProjection, SkinCapabilities, SkinCategory, SkinContext, SkinHandle,
    SkinId, SkinManifest, SkinState, SkinStateHandle, SkinVerb, SweepPointInput, SweepProjection,
    WorkspaceIntent, WorkspaceSkin, WorkspaceState, calc_state_class, selection_mode_class,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::spine_widgets::{
    ConsoleBar, NameBoxBar, NodeInspector, SPINE_WIDGETS_CSS, commit_content_edit,
    event_target_is_interactive, event_target_is_text_entry, focus_edit_buffer,
};
use crate::value_render::value_text;

pub const BENCH_ID: SkinId = SkinId::new("bench");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchState {
    /// Whether the sweep panel (sweep rail + series strip) is open.
    pub show_sweeps: bool,
}

impl Default for BenchState {
    fn default() -> Self {
        Self { show_sweeps: true }
    }
}

impl SkinState for BenchState {
    fn schema_version() -> u32 {
        1
    }
}

#[derive(Default)]
pub struct Bench;

impl Bench {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSkin for Bench {
    type State = BenchState;

    fn id(&self) -> SkinId {
        BENCH_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Bench",
            description: "Scenario bench: overrides, side-by-side comparison, and sweeps.",
            category: SkinCategory::Inspector,
            version: "0.1.0",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities {
            supports_multi_select: false,
            supports_inline_formula_edit: false,
            supports_meta_node_display: false,
            renders_arrays_inline: false,
            renders_table_values: false,
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        // Record the active lens for cross-lens chrome/continuity. Runs once.
        cx.shared.update(|state| {
            state.active_lens = Some(BENCH_ID.as_str().to_string());
        });
        SkinHandle::new(view! { <BenchView cx=cx /> }.into_any())
    }
}

// ---------------------------------------------------------------------------
// Pure helpers (unit-tested)
// ---------------------------------------------------------------------------

/// Lowercased-dashed slug of a display name; non-alphanumeric runs collapse to
/// a single dash. Falls back to `unnamed` so an id is never empty.
fn slugify(name: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = true; // suppress a leading dash
    for ch in name.trim().to_lowercase().chars() {
        if ch.is_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "unnamed".to_string()
    } else {
        slug
    }
}

/// Mint `prefix:slug`, suffixing `-2`, `-3`, … until it misses `existing`.
fn mint_prefixed_id(prefix: &str, name: &str, existing: &[String]) -> String {
    let base = format!("{prefix}:{}", slugify(name));
    if !existing.iter().any(|id| id == &base) {
        return base;
    }
    let mut suffix = 2usize;
    loop {
        let candidate = format!("{base}-{suffix}");
        if !existing.iter().any(|id| id == &candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

/// Stable host-side scenario id for a new scenario name.
fn mint_scenario_id(name: &str, existing: &[String]) -> String {
    mint_prefixed_id("scenario", name, existing)
}

/// Stable host-side sweep id for a new sweep name.
fn mint_sweep_id(name: &str, existing: &[String]) -> String {
    mint_prefixed_id("sweep", name, existing)
}

/// Classify override input text into a typed projection value. This is input
/// handling only — the host literalizes the typed value through OxFml; Bench
/// never parses formula text or computes anything.
fn build_override_value(text: &str) -> NodeValueProjection {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return NodeValueProjection::Empty;
    }
    if trimmed.parse::<f64>().is_ok() {
        return NodeValueProjection::Number {
            raw: trimmed.to_string(),
            display: trimmed.to_string(),
        };
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "true" => NodeValueProjection::Logical {
            value: true,
            display: "TRUE".to_string(),
        },
        "false" => NodeValueProjection::Logical {
            value: false,
            display: "FALSE".to_string(),
        },
        _ => NodeValueProjection::Text(trimmed.to_string()),
    }
}

/// Parse a comma-separated numeric point list ("8000, 10000, 12000") into
/// typed sweep points. Junk tokens reject by name; empty tokens (trailing
/// commas) are tolerated, but at least one point is required.
fn parse_sweep_points(text: &str) -> Result<Vec<SweepPointInput>, String> {
    let mut points = Vec::new();
    for token in text.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if token.parse::<f64>().is_err() {
            return Err(format!("sweep point '{token}' is not a number"));
        }
        points.push(SweepPointInput {
            point_id: format!("p{}", points.len()),
            label: token.to_string(),
            value: NodeValueProjection::Number {
                raw: token.to_string(),
                display: token.to_string(),
            },
        });
    }
    if points.is_empty() {
        return Err("enter at least one numeric sweep point".to_string());
    }
    Ok(points)
}

/// The interesting comparison population, in `key_order` order: keys valued in
/// at least one non-basis comparison column OR overridden in any scenario,
/// plus the selected key. Effectively-meta nodes stay off the bench.
fn comparison_rows(ws: &WorkspaceState, selected: Option<&NodeKey>) -> Vec<NodeKey> {
    let mut interesting: BTreeSet<NodeKey> = BTreeSet::new();
    for column in &ws.comparison.columns {
        interesting.extend(column.values.keys().cloned());
    }
    for scenario in &ws.scenarios.entries {
        interesting.extend(scenario.overridden_nodes.iter().cloned());
        interesting.extend(scenario.override_values.keys().cloned());
    }
    if let Some(key) = selected {
        interesting.insert(key.clone());
    }
    ws.key_order
        .iter()
        .filter(|key| interesting.contains(*key) && !ws.is_effective_meta(key))
        .cloned()
        .collect()
}

/// Build the `CreateScenario` intent for a (trimmed) name, branching from the
/// currently active scenario (`None` branches from published).
fn build_create_scenario_intent(
    name: &str,
    existing_ids: &[String],
    active_scenario_id: Option<String>,
) -> WorkspaceIntent {
    WorkspaceIntent::CreateScenario {
        scenario_id: mint_scenario_id(name, existing_ids),
        name: name.to_string(),
        base_scenario_id: active_scenario_id,
    }
}

/// Build the `SetScenarioOverride` intent from raw override input text.
fn build_set_override_intent(scenario_id: &str, node: &NodeKey, text: &str) -> WorkspaceIntent {
    WorkspaceIntent::SetScenarioOverride {
        scenario_id: scenario_id.to_string(),
        node: node.clone(),
        value: build_override_value(text),
    }
}

/// Build the `CreateScenarioSweep` intent over the selected input node, or a
/// human-readable error naming the offending point token.
fn build_create_sweep_intent(
    name: &str,
    points_text: &str,
    existing_ids: &[String],
    base_scenario_id: Option<String>,
    input_node: NodeKey,
) -> Result<WorkspaceIntent, String> {
    let points = parse_sweep_points(points_text)?;
    Ok(WorkspaceIntent::CreateScenarioSweep {
        sweep_id: mint_sweep_id(name, existing_ids),
        name: name.to_string(),
        base_scenario_id,
        input_node,
        points,
    })
}

/// Union of `overridden_nodes` and `override_values` keys, listed order first,
/// each with its typed override value when the manifest carries one.
fn scenario_override_rows(
    scenario: &ScenarioProjection,
) -> Vec<(NodeKey, Option<NodeValueProjection>)> {
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    for key in &scenario.overridden_nodes {
        if seen.insert(key.clone()) {
            rows.push((key.clone(), scenario.override_values.get(key).cloned()));
        }
    }
    for (key, value) in &scenario.override_values {
        if seen.insert(key.clone()) {
            rows.push((key.clone(), Some(value.clone())));
        }
    }
    rows
}

/// The human-readable rejection text for a refused receipt, `None` when the
/// intent was accepted.
fn receipt_rejection(receipt: &IntentReceipt) -> Option<String> {
    if receipt.accepted {
        return None;
    }
    Some(receipt.error.as_ref().map_or_else(
        || "the host rejected the intent".to_string(),
        |error| error.to_string(),
    ))
}

/// Structural provenance class for a comparative column/series source.
/// Scenario-backed values (scenarios and sweep points) carry the scenario
/// tint; candidates the speculative tint; published carries none.
fn column_provenance_class(source: &ComparativeSourceProjection) -> &'static str {
    match source {
        ComparativeSourceProjection::Published => "",
        ComparativeSourceProjection::Candidate { .. } => "dtc-prov--speculative",
        ComparativeSourceProjection::Scenario { .. }
        | ComparativeSourceProjection::SweepPoint { .. } => "dtc-prov--scenario",
    }
}

/// Numeric reading of a projected value, only when it is a parseable Number.
fn value_number(value: &NodeValueProjection) -> Option<f64> {
    match value {
        NodeValueProjection::Number { raw, .. } => raw.parse::<f64>().ok(),
        _ => None,
    }
}

/// Largest absolute parseable Number in a series; 0.0 when none parse.
fn series_max_abs(series: &SeriesProjection) -> f64 {
    series
        .points
        .iter()
        .filter_map(|point| value_number(&point.value))
        .map(f64::abs)
        .fold(0.0_f64, f64::max)
}

/// Proportional bar width percent for one series point — presentation only,
/// computed only over parseable Numbers; non-numeric points get no bar.
fn bar_percent(value: &NodeValueProjection, max_abs: f64) -> Option<f64> {
    let number = value_number(value)?;
    if max_abs <= 0.0 {
        return Some(0.0);
    }
    Some(number.abs() / max_abs * 100.0)
}

/// Row class for scenario-rail entries (shared by the Base row).
fn scenario_row_class(is_active: bool) -> &'static str {
    if is_active {
        "dtc-bench-scenario dtc-bench-scenario--active"
    } else {
        "dtc-bench-scenario"
    }
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

#[component]
fn BenchView(cx: SkinContext<BenchState>) -> impl IntoView {
    let workspace = cx.workspace;
    let selection = cx.selection;
    let shared = cx.shared;
    let dispatch = cx.dispatch.clone();
    let state = cx.state.clone();

    let editing = RwSignal::new(false);
    let editor_text = RwSignal::new(String::new());
    let name_box = RwSignal::new(Option::<String>::None);
    let rejection = RwSignal::new(Option::<String>::None);
    let edit_ref = NodeRef::<leptos::html::Textarea>::new();
    let root_ref = NodeRef::<leptos::html::Section>::new();

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
    // While NOT editing, keep the buffer synced to the published content;
    // while editing, never clobber.
    Effect::new(move |_| {
        if let Some(node) = selected.get()
            && !editing.get_untracked()
        {
            editor_text.set(node.content_text);
        }
    });

    // The one commit path, shared with every lens.
    let commit_dispatch = dispatch.clone();
    let commit = move || {
        let Some(node) = selected.get_untracked() else {
            return;
        };
        let receipt = commit_content_edit(
            &commit_dispatch,
            shared,
            node.id,
            editor_text.get_untracked(),
        );
        if receipt.accepted {
            editing.set(false);
        } else if let Some(message) = receipt_rejection(&receipt) {
            rejection.set(Some(message));
        }
    };

    // Lens-local grammar, resolved through the one registry. Global verbs
    // (F9, Ctrl+Z/Y, Ctrl+N, lens switch, arrows) bubble to the shell.
    let grammar = KeybindingRegistry::universal();
    let root_keydown = move |ev: leptos::ev::KeyboardEvent| {
        // Bare-key grammar never fires while typing — the contract.
        if event_target_is_text_entry(&ev) || editing.get_untracked() {
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
                if selected.get_untracked().is_some() {
                    ev.prevent_default();
                    editing.set(true);
                    focus_edit_buffer(edit_ref);
                }
            }
            SkinVerb::NameBox => {
                ev.prevent_default();
                name_box.set(Some(String::new()));
            }
            SkinVerb::Escape => {
                name_box.set(None);
                editing.set(false);
                rejection.set(None);
            }
            _ => {} // global verb: leave for the shell
        }
    };

    // Land keyboard focus on the bench when the lens mounts, so the grammar
    // works without a preparatory click.
    Effect::new(move |_| {
        if let Some(section) = root_ref.get() {
            let _ = section.focus();
        }
    });

    let css = format!("{ATLAS_SPINE_CSS}\n{SPINE_WIDGETS_CSS}\n{BENCH_CSS}");
    let rail_dispatch = dispatch.clone();
    let override_dispatch = dispatch.clone();
    let sweep_dispatch = dispatch.clone();
    let console_dispatch = dispatch.clone();

    view! {
        <style>{css}</style>
        <section class="dtc-bench" tabindex="0" node_ref=root_ref on:keydown=root_keydown>
            <div class="dtc-bench__body">
                <ScenarioRail workspace=workspace dispatch=rail_dispatch rejection=rejection />
                <div class="dtc-bench__center">
                    <NameBoxBar name_box=name_box workspace=workspace dispatch=dispatch.clone() />
                    {move || {
                        rejection
                            .get()
                            .map(|message| {
                                view! {
                                    <div class="dtc-bench-rejection" role="alert">
                                        <span class="dtc-bench-rejection__text">{message}</span>
                                        <button type="button" on:click=move |_| rejection.set(None)>
                                            "Dismiss"
                                        </button>
                                    </div>
                                }
                                .into_any()
                            })
                            .unwrap_or_else(|| ().into_any())
                    }}
                    <OverridePanel
                        workspace=workspace
                        selection=selection
                        dispatch=override_dispatch
                        rejection=rejection
                    />
                    <ComparisonGrid workspace=workspace selection=selection />
                    <SweepPanel
                        workspace=workspace
                        selection=selection
                        dispatch=sweep_dispatch
                        rejection=rejection
                        state=state.clone()
                    />
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

#[component]
fn ScenarioRail(
    workspace: ReadSignal<WorkspaceState>,
    dispatch: Arc<dyn Dispatcher>,
    rejection: RwSignal<Option<String>>,
) -> impl IntoView {
    let new_name = RwSignal::new(String::new());
    let base_active = Memo::new(move |_| workspace.with(|ws| ws.scenarios.active.is_none()));
    let base_dispatch = dispatch.clone();
    let rows_dispatch = dispatch.clone();
    let create_dispatch = dispatch;

    view! {
        <aside class="dtc-bench-rail" aria-label="Scenario rail">
            <div class="dtc-section-label">"Scenarios"</div>
            <button
                type="button"
                class=move || scenario_row_class(base_active.get())
                on:click=move |_| {
                    let receipt = base_dispatch
                        .dispatch(WorkspaceIntent::ActivateScenario { scenario_id: None });
                    if let Some(message) = receipt_rejection(&receipt) {
                        rejection.set(Some(message));
                    }
                }
            >
                <span class="dtc-bench-rail__radio">
                    {move || if base_active.get() { "●" } else { "○" }}
                </span>
                <span class="dtc-bench-scenario__name">"Base (published)"</span>
            </button>
            {move || {
                workspace.with(|ws| {
                    ws.scenarios
                        .entries
                        .iter()
                        .map(|scenario| scenario_row_view(scenario, rows_dispatch.clone(), rejection))
                        .collect::<Vec<_>>()
                })
            }}
            <div class="dtc-bench-rail__new">
                <input
                    class="dtc-bench-input"
                    placeholder="new scenario name…"
                    prop:value=move || new_name.get()
                    on:input=move |ev| new_name.set(event_target_value(&ev))
                    on:keydown=move |ev: leptos::ev::KeyboardEvent| ev.stop_propagation()
                />
                <button
                    type="button"
                    on:click=move |_| {
                        let name = new_name.get_untracked();
                        let trimmed = name.trim();
                        if trimmed.is_empty() {
                            return;
                        }
                        let ws = workspace.get_untracked();
                        let existing: Vec<String> = ws
                            .scenarios
                            .entries
                            .iter()
                            .map(|entry| entry.id.clone())
                            .collect();
                        let intent = build_create_scenario_intent(
                            trimmed,
                            &existing,
                            ws.scenarios.active.clone(),
                        );
                        let receipt = create_dispatch.dispatch(intent);
                        if let Some(message) = receipt_rejection(&receipt) {
                            rejection.set(Some(message));
                        } else {
                            new_name.set(String::new());
                        }
                    }
                >
                    "New scenario"
                </button>
            </div>
        </aside>
    }
}

fn scenario_row_view(
    scenario: &ScenarioProjection,
    dispatch: Arc<dyn Dispatcher>,
    rejection: RwSignal<Option<String>>,
) -> AnyView {
    let radio = if scenario.is_active { "●" } else { "○" };
    let row_class = scenario_row_class(scenario.is_active);
    let id_for_activate = scenario.id.clone();
    let id_for_delete = scenario.id.clone();
    let activate_dispatch = dispatch.clone();
    let delete_dispatch = dispatch;

    view! {
        <div class=row_class>
            <button
                type="button"
                class="dtc-bench-scenario__activate"
                on:click=move |_| {
                    let receipt = activate_dispatch.dispatch(WorkspaceIntent::ActivateScenario {
                        scenario_id: Some(id_for_activate.clone()),
                    });
                    if let Some(message) = receipt_rejection(&receipt) {
                        rejection.set(Some(message));
                    }
                }
            >
                <span class="dtc-bench-rail__radio">{radio}</span>
                <span class="dtc-bench-scenario__name">{scenario.name.clone()}</span>
                <span class="dtc-bench-scenario__badge" title="Override count">
                    {scenario.override_count.to_string()}
                </span>
            </button>
            <button
                type="button"
                title="Delete scenario"
                on:click=move |_| {
                    let receipt = delete_dispatch.dispatch(WorkspaceIntent::DeleteScenario {
                        scenario_id: id_for_delete.clone(),
                    });
                    if let Some(message) = receipt_rejection(&receipt) {
                        rejection.set(Some(message));
                    }
                }
            >
                "✕"
            </button>
        </div>
    }
    .into_any()
}

#[component]
fn OverridePanel(
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<SelectionState>,
    dispatch: Arc<dyn Dispatcher>,
    rejection: RwSignal<Option<String>>,
) -> impl IntoView {
    let override_text = RwSignal::new(String::new());
    let set_dispatch = dispatch.clone();
    let clear_dispatch = dispatch.clone();
    let rows_dispatch = dispatch;

    view! {
        <div class="dtc-bench-override">
            {move || {
                let ws = workspace.get();
                let Some(scenario) = ws.scenarios.entries.iter().find(|s| s.is_active).cloned()
                else {
                    return view! {
                        <p class="dtc-bench-empty">
                            "Activate a scenario to stage consequence-free overrides."
                        </p>
                    }
                    .into_any();
                };
                let selected = selection
                    .with(|s| s.primary.clone())
                    .and_then(|id| ws.node(&id).cloned());

                let selected_block = match selected {
                    None => view! {
                        <p class="dtc-bench-empty">"Select a node to stage an override."</p>
                    }
                    .into_any(),
                    Some(node) => {
                        let published = value_text(&node.computed_value);
                        let current = scenario
                            .override_values
                            .get(&node.key)
                            .cloned()
                            .or_else(|| node.scenario_override.clone());
                        let override_badge = match current.as_ref().map(value_text) {
                            Some(text) => view! {
                                <span class="dtc-prov--scenario">{format!("override: {text}")}</span>
                            }
                            .into_any(),
                            None => view! {
                                <span class="dtc-bench-override__none">"no override"</span>
                            }
                            .into_any(),
                        };
                        let sid_for_set = scenario.id.clone();
                        let sid_for_clear = scenario.id.clone();
                        let key_for_set = node.key.clone();
                        let key_for_clear = node.key.clone();
                        let set_dispatch = set_dispatch.clone();
                        let clear_dispatch = clear_dispatch.clone();
                        view! {
                            <div class="dtc-bench-override__selected">
                                <div class="dtc-bench-override__facts">
                                    <span class="dtc-bench-override__name">
                                        {node.display_name.clone()}
                                    </span>
                                    <span>{format!("published: {published}")}</span>
                                    {override_badge}
                                </div>
                                <div class="dtc-bench-override__controls">
                                    <input
                                        class="dtc-bench-input"
                                        placeholder="number, TRUE/FALSE, or text…"
                                        prop:value=move || override_text.get()
                                        on:input=move |ev| override_text.set(event_target_value(&ev))
                                        on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                            ev.stop_propagation()
                                        }
                                    />
                                    <button
                                        type="button"
                                        on:click=move |_| {
                                            let receipt = set_dispatch.dispatch(
                                                build_set_override_intent(
                                                    &sid_for_set,
                                                    &key_for_set,
                                                    &override_text.get_untracked(),
                                                ),
                                            );
                                            if let Some(message) = receipt_rejection(&receipt) {
                                                rejection.set(Some(message));
                                            } else {
                                                override_text.set(String::new());
                                            }
                                        }
                                    >
                                        "Override"
                                    </button>
                                    <button
                                        type="button"
                                        on:click=move |_| {
                                            let receipt = clear_dispatch.dispatch(
                                                WorkspaceIntent::ClearScenarioOverride {
                                                    scenario_id: sid_for_clear.clone(),
                                                    node: key_for_clear.clone(),
                                                },
                                            );
                                            if let Some(message) = receipt_rejection(&receipt) {
                                                rejection.set(Some(message));
                                            }
                                        }
                                    >
                                        "Clear"
                                    </button>
                                </div>
                            </div>
                        }
                        .into_any()
                    }
                };

                let rows = scenario_override_rows(&scenario)
                    .into_iter()
                    .map(|(key, value)| {
                        let value_label =
                            value.as_ref().map_or_else(|| "–".to_string(), value_text);
                        let jump_path = ws.paths_by_key.get(&key).cloned();
                        override_row_view(
                            key,
                            value_label,
                            jump_path,
                            scenario.id.clone(),
                            rows_dispatch.clone(),
                            rejection,
                        )
                    })
                    .collect::<Vec<_>>();

                view! {
                    <div class="dtc-bench-override__panel">
                        <div class="dtc-section-label">
                            {format!("Overrides — {}", scenario.name)}
                        </div>
                        {selected_block}
                        {(!rows.is_empty())
                            .then(|| {
                                view! { <div class="dtc-bench-override__rows">{rows}</div> }
                                    .into_any()
                            })}
                    </div>
                }
                .into_any()
            }}
        </div>
    }
}

fn override_row_view(
    key: NodeKey,
    value_label: String,
    jump_path: Option<NodeId>,
    scenario_id: String,
    dispatch: Arc<dyn Dispatcher>,
    rejection: RwSignal<Option<String>>,
) -> AnyView {
    let label = jump_path
        .as_ref()
        .map_or_else(|| key.to_string(), |path| path.as_str().to_string());
    let jump_disabled = jump_path.is_none();
    let jump_dispatch = dispatch.clone();
    let clear_dispatch = dispatch;
    let key_for_clear = key.clone();

    view! {
        <div class="dtc-bench-override-row">
            <span class="dtc-bench-override-row__path" title=key.to_string()>{label}</span>
            <span class="dtc-bench-override-row__value dtc-prov--scenario">{value_label}</span>
            <button
                type="button"
                disabled=jump_disabled
                title="Jump to node"
                on:click=move |_| {
                    if let Some(path) = jump_path.clone() {
                        jump_dispatch.dispatch(WorkspaceIntent::SelectNode(Some(path)));
                    }
                }
            >
                "Jump"
            </button>
            <button
                type="button"
                title="Clear this override"
                on:click=move |_| {
                    let receipt = clear_dispatch.dispatch(WorkspaceIntent::ClearScenarioOverride {
                        scenario_id: scenario_id.clone(),
                        node: key_for_clear.clone(),
                    });
                    if let Some(message) = receipt_rejection(&receipt) {
                        rejection.set(Some(message));
                    }
                }
            >
                "Clear"
            </button>
        </div>
    }
    .into_any()
}

#[component]
fn ComparisonGrid(
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<SelectionState>,
) -> impl IntoView {
    view! {
        <div class="dtc-bench-comparison">
            <div class="dtc-section-label">"Comparison"</div>
            {move || {
                let ws = workspace.get();
                let selected_key = selection.with(|sel| {
                    sel.primary
                        .as_ref()
                        .and_then(|id| ws.node(id))
                        .map(|node| node.key.clone())
                });
                let rows = comparison_rows(&ws, selected_key.as_ref());
                if rows.is_empty() {
                    return view! {
                        <p class="dtc-bench-empty">
                            "No scenario, candidate, or sweep values to compare yet."
                        </p>
                    }
                    .into_any();
                }
                let basis_label = if ws.comparison.basis.label.trim().is_empty() {
                    "Published".to_string()
                } else {
                    ws.comparison.basis.label.clone()
                };
                let mut headers: Vec<AnyView> = Vec::new();
                headers.push(
                    view! { <th class="dtc-bench-grid__col">{basis_label}</th> }.into_any(),
                );
                for column in &ws.comparison.columns {
                    let prov = column_provenance_class(&column.source);
                    let class = if prov.is_empty() {
                        "dtc-bench-grid__col".to_string()
                    } else {
                        format!("dtc-bench-grid__col {prov}")
                    };
                    headers.push(
                        view! { <th class=class>{column.label.clone()}</th> }.into_any(),
                    );
                }
                let body = rows
                    .iter()
                    .filter_map(|row_key| {
                        let node = ws.node_by_key(row_key)?;
                        Some(comparison_row_view(
                            node,
                            &ws,
                            selected_key.as_ref() == Some(row_key),
                        ))
                    })
                    .collect::<Vec<_>>();
                view! {
                    <table class="dtc-bench-grid">
                        <thead>
                            <tr>
                                <th class="dtc-bench-grid__node">"Node"</th>
                                {headers}
                            </tr>
                        </thead>
                        <tbody>{body}</tbody>
                    </table>
                }
                .into_any()
            }}
        </div>
    }
}

fn comparison_row_view(node: &NodeView, ws: &WorkspaceState, is_selected: bool) -> AnyView {
    let calc_class = calc_state_class(node.calc_state);
    let sel_class = selection_mode_class(is_selected, false);
    let mut row_class = String::from("dtc-bench-grid__row");
    if !sel_class.is_empty() {
        row_class.push(' ');
        row_class.push_str(sel_class);
    }
    let name_class = format!("dtc-bench-grid__name {calc_class}");

    let mut cells: Vec<AnyView> = Vec::new();
    cells.push(comparison_cell_view(
        ws.comparison.basis.values.get(&node.key),
        "",
    ));
    for column in &ws.comparison.columns {
        cells.push(comparison_cell_view(
            column.values.get(&node.key),
            column_provenance_class(&column.source),
        ));
    }

    view! {
        <tr class=row_class>
            <td class=name_class title=node.id.to_string()>
                <span class="dtc-calc-dot"></span>
                {node.display_name.clone()}
            </td>
            {cells}
        </tr>
    }
    .into_any()
}

/// One comparison cell: the projected value's text, or an honest dash when the
/// source has no value for this node (Bench never fabricates).
fn comparison_cell_view(value: Option<&NodeValueProjection>, prov_class: &'static str) -> AnyView {
    let text = value.map_or_else(|| "–".to_string(), value_text);
    let class = if prov_class.is_empty() {
        "dtc-bench-grid__cell".to_string()
    } else {
        format!("dtc-bench-grid__cell {prov_class}")
    };
    view! { <td class=class>{text}</td> }.into_any()
}

#[component]
fn SweepPanel(
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<SelectionState>,
    dispatch: Arc<dyn Dispatcher>,
    rejection: RwSignal<Option<String>>,
    state: SkinStateHandle<BenchState>,
) -> impl IntoView {
    let sweep_name = RwSignal::new(String::new());
    let sweep_points = RwSignal::new(String::new());
    let show = {
        let state = state.clone();
        Memo::new(move |_| state.with(|s| s.show_sweeps))
    };
    let toggle_state = state.clone();
    let rows_dispatch = dispatch.clone();
    let create_dispatch = dispatch;

    let selected_key = Memo::new(move |_| {
        selection
            .with(|s| s.primary.clone())
            .and_then(|id| workspace.with(|ws| ws.node(&id).map(|node| node.key.clone())))
    });

    view! {
        <div class="dtc-bench-sweeps">
            <div class="dtc-bench-sweeps__head">
                <span class="dtc-section-label">"Sweeps"</span>
                <button
                    type="button"
                    on:click=move |_| toggle_state.update(|s| s.show_sweeps = !s.show_sweeps)
                >
                    {move || if show.get() { "Hide" } else { "Show" }}
                </button>
            </div>
            {move || {
                if !show.get() {
                    return ().into_any();
                }
                let ws = workspace.get();
                let key = selected_key.get();
                let rows = ws
                    .sweeps
                    .entries
                    .iter()
                    .map(|sweep| sweep_row_view(sweep, &ws, rows_dispatch.clone(), rejection))
                    .collect::<Vec<_>>();
                let series = ws.series.entries.iter().map(series_view).collect::<Vec<_>>();
                let create_dispatch = create_dispatch.clone();
                let create_disabled = key.is_none();
                let active_scenario = ws.scenarios.active.clone();
                let existing: Vec<String> = ws
                    .sweeps
                    .entries
                    .iter()
                    .map(|entry| entry.id.clone())
                    .collect();
                view! {
                    <div class="dtc-bench-sweeps__body">
                        {(!rows.is_empty())
                            .then(|| {
                                view! { <div class="dtc-bench-sweeps__list">{rows}</div> }
                                    .into_any()
                            })}
                        <div class="dtc-bench-sweeps__new">
                            <input
                                class="dtc-bench-input"
                                placeholder="sweep name…"
                                prop:value=move || sweep_name.get()
                                on:input=move |ev| sweep_name.set(event_target_value(&ev))
                                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                    ev.stop_propagation()
                                }
                            />
                            <input
                                class="dtc-bench-input"
                                placeholder="points e.g. 8000, 10000, 12000"
                                prop:value=move || sweep_points.get()
                                on:input=move |ev| sweep_points.set(event_target_value(&ev))
                                on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                    ev.stop_propagation()
                                }
                            />
                            <button
                                type="button"
                                disabled=create_disabled
                                title="Sweep the selected input node"
                                on:click=move |_| {
                                    let Some(input_node) = key.clone() else {
                                        return;
                                    };
                                    let name = sweep_name.get_untracked();
                                    let trimmed = name.trim();
                                    if trimmed.is_empty() {
                                        rejection.set(Some("name the sweep first".to_string()));
                                        return;
                                    }
                                    match build_create_sweep_intent(
                                        trimmed,
                                        &sweep_points.get_untracked(),
                                        &existing,
                                        active_scenario.clone(),
                                        input_node,
                                    ) {
                                        Ok(intent) => {
                                            let receipt = create_dispatch.dispatch(intent);
                                            if let Some(message) = receipt_rejection(&receipt) {
                                                rejection.set(Some(message));
                                            } else {
                                                sweep_name.set(String::new());
                                                sweep_points.set(String::new());
                                            }
                                        }
                                        Err(message) => rejection.set(Some(message)),
                                    }
                                }
                            >
                                "New sweep"
                            </button>
                        </div>
                        {(!series.is_empty())
                            .then(|| {
                                view! { <div class="dtc-bench-series-strip">{series}</div> }
                                    .into_any()
                            })}
                    </div>
                }
                .into_any()
            }}
        </div>
    }
}

fn sweep_row_view(
    sweep: &SweepProjection,
    ws: &WorkspaceState,
    dispatch: Arc<dyn Dispatcher>,
    rejection: RwSignal<Option<String>>,
) -> AnyView {
    let radio = if sweep.is_active { "●" } else { "○" };
    let row_class = if sweep.is_active {
        "dtc-bench-sweep dtc-bench-sweep--active"
    } else {
        "dtc-bench-sweep"
    };
    let input_label = ws.paths_by_key.get(&sweep.input_node).map_or_else(
        || sweep.input_node.to_string(),
        |path| path.as_str().to_string(),
    );
    let id_for_activate = sweep.id.clone();
    let id_for_delete = sweep.id.clone();
    let activate_dispatch = dispatch.clone();
    let delete_dispatch = dispatch;

    view! {
        <div class=row_class>
            <button
                type="button"
                class="dtc-bench-sweep__activate"
                on:click=move |_| {
                    let receipt = activate_dispatch.dispatch(WorkspaceIntent::ActivateSweep {
                        sweep_id: Some(id_for_activate.clone()),
                    });
                    if let Some(message) = receipt_rejection(&receipt) {
                        rejection.set(Some(message));
                    }
                }
            >
                <span class="dtc-bench-rail__radio">{radio}</span>
                <span class="dtc-bench-sweep__name">{sweep.name.clone()}</span>
            </button>
            <span class="dtc-bench-sweep__input" title="Swept input node">{input_label}</span>
            <span class="dtc-bench-sweep__count">
                {format!("{} pts", sweep.points.len())}
            </span>
            <button
                type="button"
                title="Delete sweep"
                on:click=move |_| {
                    let receipt = delete_dispatch.dispatch(WorkspaceIntent::DeleteSweep {
                        sweep_id: id_for_delete.clone(),
                    });
                    if let Some(message) = receipt_rejection(&receipt) {
                        rejection.set(Some(message));
                    }
                }
            >
                "✕"
            </button>
        </div>
    }
    .into_any()
}

/// One series card: labelled point rows with value text and a proportional
/// horizontal bar (presentation only — widths come from parseable Numbers).
fn series_view(series: &SeriesProjection) -> AnyView {
    let max_abs = series_max_abs(series);
    let prov = column_provenance_class(&series.source);
    let label_class = if prov.is_empty() {
        "dtc-bench-series__title".to_string()
    } else {
        format!("dtc-bench-series__title {prov}")
    };
    let unit_badge = series
        .unit
        .clone()
        .map(|unit| view! { <span class="dtc-bench-series__unit">{unit}</span> }.into_any());
    let points = series
        .points
        .iter()
        .map(|point| {
            let text = value_text(&point.value);
            let bar_style = match bar_percent(&point.value, max_abs) {
                Some(percent) => format!("width:{percent:.1}%;"),
                None => "width:0;".to_string(),
            };
            view! {
                <div class="dtc-bench-series__row">
                    <span class="dtc-bench-series__point">{point.label.clone()}</span>
                    <span class="dtc-bench-series__value">{text}</span>
                    <div class="dtc-bench-series__track">
                        <div class="dtc-bench-series__bar" style=bar_style></div>
                    </div>
                </div>
            }
            .into_any()
        })
        .collect::<Vec<_>>();

    view! {
        <div class="dtc-bench-series">
            <div class="dtc-bench-series__head">
                <span class=label_class>{series.label.clone()}</span>
                {unit_badge}
            </div>
            {points}
        </div>
    }
    .into_any()
}

const BENCH_CSS: &str = r#"
.dtc-bench {
  display: flex; flex-direction: column; height: 100%;
  background: var(--dtc-surface); color: var(--dtc-text);
  font: 13px/1.4 var(--dtc-font, system-ui, sans-serif);
  outline: none;
}
.dtc-bench__body { display: flex; flex: 1; min-height: 0; }
.dtc-bench__center {
  flex: 1; min-width: 0; overflow: auto; padding: 8px 12px;
  display: flex; flex-direction: column; gap: 10px;
}

.dtc-bench-rail {
  width: 230px; flex-shrink: 0; overflow: auto; padding: 10px;
  border-right: 1px solid var(--dtc-border-muted); background: var(--dtc-surface-panel);
  display: flex; flex-direction: column; gap: 4px;
}
.dtc-bench-rail button, .dtc-bench__center button {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 2px 8px; cursor: pointer; color: var(--dtc-text); font: inherit;
}
.dtc-bench-rail button:disabled, .dtc-bench__center button:disabled {
  opacity: 0.45; cursor: default;
}
.dtc-bench-input {
  flex: 1; min-width: 0; padding: 3px 6px; border: 1px solid var(--dtc-border);
  border-radius: 5px; background: var(--dtc-surface-input); color: var(--dtc-text); font: inherit;
}

.dtc-bench-scenario {
  display: flex; align-items: center; gap: 6px; width: 100%; box-sizing: border-box;
  border: 1px solid transparent; border-radius: 6px; padding: 2px;
}
button.dtc-bench-scenario { border: 1px solid var(--dtc-border); padding: 4px 8px; text-align: left; }
.dtc-bench-scenario--active { border-color: var(--dtc-accent-border); background: var(--dtc-surface-subtle); }
.dtc-bench-scenario__activate {
  flex: 1; display: flex; align-items: center; gap: 6px; min-width: 0; text-align: left;
}
.dtc-bench-scenario__name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.dtc-bench-scenario__badge {
  padding: 0 6px; border: 1px solid var(--dtc-border); border-radius: 9px;
  font-size: 11px; color: var(--dtc-text-subtle); font-variant-numeric: tabular-nums;
}
.dtc-bench-rail__radio { width: 1em; text-align: center; color: var(--dtc-accent); }
.dtc-bench-rail__new { display: flex; gap: 6px; margin-top: 8px; }

.dtc-bench-rejection {
  display: flex; align-items: center; gap: 8px; padding: 6px 8px;
  border: 1px solid var(--dtc-danger); border-radius: 6px;
  background: var(--dtc-surface-subtle); color: var(--dtc-danger-text);
}
.dtc-bench-rejection__text { flex: 1; }

.dtc-bench-empty { color: var(--dtc-text-subtle); font-style: italic; margin: 4px 0; }

.dtc-bench-override__selected { display: flex; flex-direction: column; gap: 6px; }
.dtc-bench-override__facts { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; }
.dtc-bench-override__name { font-weight: 600; }
.dtc-bench-override__none { color: var(--dtc-text-subtle); font-style: italic; }
.dtc-bench-override__controls { display: flex; gap: 6px; }
.dtc-bench-override__rows { display: flex; flex-direction: column; gap: 2px; margin-top: 6px; }
.dtc-bench-override-row { display: flex; align-items: center; gap: 8px; padding: 2px 0; }
.dtc-bench-override-row__path {
  flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.dtc-bench-override-row__value { font-variant-numeric: tabular-nums; }

.dtc-bench-grid { border-collapse: collapse; width: 100%; }
.dtc-bench-grid th, .dtc-bench-grid td {
  border: 1px solid var(--dtc-border-muted); padding: 3px 8px; text-align: right;
  font-variant-numeric: tabular-nums; white-space: nowrap;
}
.dtc-bench-grid th { background: var(--dtc-surface-subtle); font-weight: 600; }
.dtc-bench-grid__node, .dtc-bench-grid__name { text-align: left; }
.dtc-bench-grid__name .dtc-calc-dot { margin-right: 6px; }

.dtc-bench-sweeps__head { display: flex; align-items: center; gap: 8px; }
.dtc-bench-sweeps__head .dtc-section-label { margin: 0; flex: 1; }
.dtc-bench-sweeps__body { display: flex; flex-direction: column; gap: 8px; margin-top: 6px; }
.dtc-bench-sweeps__list { display: flex; flex-direction: column; gap: 4px; }
.dtc-bench-sweep {
  display: flex; align-items: center; gap: 8px; border: 1px solid transparent;
  border-radius: 6px; padding: 2px;
}
.dtc-bench-sweep--active { border-color: var(--dtc-accent-border); background: var(--dtc-surface-subtle); }
.dtc-bench-sweep__activate { display: flex; align-items: center; gap: 6px; min-width: 0; }
.dtc-bench-sweep__name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.dtc-bench-sweep__input {
  flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  color: var(--dtc-text-muted);
}
.dtc-bench-sweep__count { color: var(--dtc-text-subtle); font-variant-numeric: tabular-nums; }
.dtc-bench-sweeps__new { display: flex; gap: 6px; }

.dtc-bench-series-strip { display: flex; gap: 10px; flex-wrap: wrap; }
.dtc-bench-series {
  border: 1px solid var(--dtc-border-muted); border-radius: 6px; padding: 6px 8px;
  min-width: 240px; flex: 1; max-width: 420px;
}
.dtc-bench-series__head { display: flex; align-items: center; gap: 6px; margin-bottom: 4px; }
.dtc-bench-series__title { font-weight: 600; }
.dtc-bench-series__unit {
  padding: 0 6px; border: 1px solid var(--dtc-border); border-radius: 9px;
  font-size: 11px; color: var(--dtc-text-subtle);
}
.dtc-bench-series__row {
  display: grid; grid-template-columns: minmax(70px, 1fr) auto minmax(80px, 2fr);
  gap: 8px; align-items: center; padding: 1px 0;
}
.dtc-bench-series__value { font-variant-numeric: tabular-nums; text-align: right; }
.dtc-bench-series__track {
  background: var(--dtc-surface-subtle); border-radius: 3px; height: 8px; overflow: hidden;
}
.dtc-bench-series__bar { height: 100%; background: var(--dtc-accent); opacity: 0.55; }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::{
        ComparativeColumnProjection, IntentError, NodeContentKind, ScenarioSourceProjection,
        SeriesPointProjection,
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

    fn workspace_with(nodes: Vec<NodeView>) -> WorkspaceState {
        let mut ws = WorkspaceState::default();
        for node in nodes {
            ws.node_order.push(node.id.clone());
            ws.key_order.push(node.key.clone());
            ws.paths_by_key.insert(node.key.clone(), node.id.clone());
            ws.nodes.insert(node.id.clone(), node.clone());
            ws.nodes_by_key.insert(node.key.clone(), node);
        }
        ws
    }

    fn number(raw: &str) -> NodeValueProjection {
        NodeValueProjection::Number {
            raw: raw.to_string(),
            display: raw.to_string(),
        }
    }

    fn scenario(
        id: &str,
        is_active: bool,
        overrides: &[(&str, NodeValueProjection)],
    ) -> ScenarioProjection {
        let override_values: BTreeMap<NodeKey, NodeValueProjection> = overrides
            .iter()
            .map(|(key, value)| (NodeKey::new(*key), value.clone()))
            .collect();
        ScenarioProjection {
            id: id.to_string(),
            name: id.to_string(),
            source: ScenarioSourceProjection::Candidate {
                handle: format!("cand:{id}"),
            },
            override_count: override_values.len(),
            overridden_nodes: override_values.keys().cloned().collect(),
            override_values,
            value_epoch: None,
            is_active,
        }
    }

    /// A, B, C in key_order; one scenario column valuing C; an active scenario
    /// overriding B.
    fn comparison_fixture() -> WorkspaceState {
        let mut ws = workspace_with(vec![node("k:A", "A"), node("k:B", "B"), node("k:C", "C")]);
        ws.comparison.columns.push(ComparativeColumnProjection {
            label: "What-if".to_string(),
            source: ComparativeSourceProjection::Scenario {
                id: "scenario:what-if".to_string(),
            },
            value_epoch: None,
            values: BTreeMap::from([(NodeKey::new("k:C"), number("9"))]),
        });
        ws.scenarios
            .entries
            .push(scenario("scenario:what-if", true, &[("k:B", number("5"))]));
        ws.scenarios.active = Some("scenario:what-if".to_string());
        ws
    }

    #[test]
    fn mint_scenario_id_slugs_the_name() {
        assert_eq!(
            mint_scenario_id("High Fuel Cost!", &[]),
            "scenario:high-fuel-cost"
        );
        assert_eq!(
            mint_scenario_id("  spaced   out  ", &[]),
            "scenario:spaced-out"
        );
        assert_eq!(mint_scenario_id("///", &[]), "scenario:unnamed");
    }

    #[test]
    fn mint_scenario_id_suffixes_on_collision() {
        let existing = vec!["scenario:base-case".to_string()];
        assert_eq!(
            mint_scenario_id("Base Case", &existing),
            "scenario:base-case-2"
        );
        let existing = vec![
            "scenario:base-case".to_string(),
            "scenario:base-case-2".to_string(),
        ];
        assert_eq!(
            mint_scenario_id("Base Case", &existing),
            "scenario:base-case-3"
        );
    }

    #[test]
    fn build_override_value_classifies_inputs() {
        assert_eq!(build_override_value(" 42.5 "), number("42.5"));
        assert_eq!(
            build_override_value("TRUE"),
            NodeValueProjection::Logical {
                value: true,
                display: "TRUE".to_string()
            }
        );
        assert_eq!(
            build_override_value("False"),
            NodeValueProjection::Logical {
                value: false,
                display: "FALSE".to_string()
            }
        );
        assert_eq!(build_override_value("   "), NodeValueProjection::Empty);
        assert_eq!(
            build_override_value("hello world"),
            NodeValueProjection::Text("hello world".to_string())
        );
    }

    #[test]
    fn parse_sweep_points_accepts_numeric_list() {
        let points = parse_sweep_points("8000, 10000, 12000").expect("numeric list parses");
        assert_eq!(points.len(), 3);
        assert_eq!(
            points[0],
            SweepPointInput {
                point_id: "p0".to_string(),
                label: "8000".to_string(),
                value: number("8000"),
            }
        );
        assert_eq!(points[2].point_id, "p2");
        assert_eq!(points[2].label, "12000");
        // Trailing commas are tolerated, not junk.
        assert_eq!(
            parse_sweep_points("1, 2,")
                .expect("trailing comma ok")
                .len(),
            2
        );
    }

    #[test]
    fn parse_sweep_points_names_the_junk_token() {
        let error = parse_sweep_points("8000, banana, 12000").expect_err("junk token rejects");
        assert!(
            error.contains("banana"),
            "error should name the token: {error}"
        );
        assert!(
            parse_sweep_points("  ").is_err(),
            "empty input has no points"
        );
    }

    #[test]
    fn comparison_rows_cover_columns_overrides_and_selection_in_key_order() {
        let ws = comparison_fixture();
        // Override-only key (B) and column-only key (C), in key_order order.
        assert_eq!(
            comparison_rows(&ws, None),
            vec![NodeKey::new("k:B"), NodeKey::new("k:C")]
        );
        // The selected key joins the population, still in key_order order.
        let selected = NodeKey::new("k:A");
        assert_eq!(
            comparison_rows(&ws, Some(&selected)),
            vec![
                NodeKey::new("k:A"),
                NodeKey::new("k:B"),
                NodeKey::new("k:C")
            ]
        );
    }

    #[test]
    fn comparison_rows_exclude_effective_meta_nodes() {
        let mut ws = comparison_fixture();
        ws.nodes_by_key
            .get_mut(&NodeKey::new("k:B"))
            .unwrap()
            .is_meta = true;
        ws.nodes.get_mut(&NodeId::new("B")).unwrap().is_meta = true;
        assert_eq!(comparison_rows(&ws, None), vec![NodeKey::new("k:C")]);
    }

    #[test]
    fn create_scenario_intent_carries_minted_id_and_active_base() {
        let intent = build_create_scenario_intent(
            "Fuel Spike",
            &["scenario:fuel-spike".to_string()],
            Some("scenario:base".to_string()),
        );
        assert_eq!(
            intent,
            WorkspaceIntent::CreateScenario {
                scenario_id: "scenario:fuel-spike-2".to_string(),
                name: "Fuel Spike".to_string(),
                base_scenario_id: Some("scenario:base".to_string()),
            }
        );
    }

    #[test]
    fn set_override_intent_carries_typed_value() {
        let intent = build_set_override_intent("scenario:s", &NodeKey::new("k:A"), "12");
        assert_eq!(
            intent,
            WorkspaceIntent::SetScenarioOverride {
                scenario_id: "scenario:s".to_string(),
                node: NodeKey::new("k:A"),
                value: number("12"),
            }
        );
    }

    #[test]
    fn create_sweep_intent_mints_id_and_points() {
        let intent = build_create_sweep_intent(
            "Fuel sweep",
            "8000, 10000",
            &[],
            Some("scenario:base".to_string()),
            NodeKey::new("k:A"),
        )
        .expect("numeric points build the intent");
        assert_eq!(
            intent,
            WorkspaceIntent::CreateScenarioSweep {
                sweep_id: "sweep:fuel-sweep".to_string(),
                name: "Fuel sweep".to_string(),
                base_scenario_id: Some("scenario:base".to_string()),
                input_node: NodeKey::new("k:A"),
                points: vec![
                    SweepPointInput {
                        point_id: "p0".to_string(),
                        label: "8000".to_string(),
                        value: number("8000"),
                    },
                    SweepPointInput {
                        point_id: "p1".to_string(),
                        label: "10000".to_string(),
                        value: number("10000"),
                    },
                ],
            }
        );
        assert!(
            build_create_sweep_intent("s", "junk", &[], None, NodeKey::new("k:A")).is_err(),
            "junk points never reach the dispatcher"
        );
    }

    #[test]
    fn scenario_override_rows_merge_listed_and_mapped_keys() {
        let mut sc = scenario("scenario:s", true, &[("k:A", number("1"))]);
        // A key listed without a typed value, and a mapped value not listed.
        sc.overridden_nodes.push(NodeKey::new("k:Listed"));
        sc.override_values
            .insert(NodeKey::new("k:Mapped"), number("2"));
        let rows = scenario_override_rows(&sc);
        assert_eq!(rows.len(), 3);
        let keys: Vec<String> = rows.iter().map(|(key, _)| key.to_string()).collect();
        assert!(keys.contains(&"k:A".to_string()));
        assert!(keys.contains(&"k:Listed".to_string()));
        assert!(keys.contains(&"k:Mapped".to_string()));
        let listed = rows
            .iter()
            .find(|(key, _)| key == &NodeKey::new("k:Listed"))
            .unwrap();
        assert!(
            listed.1.is_none(),
            "listed-but-untyped override has no value"
        );
    }

    #[test]
    fn receipt_rejection_reports_only_failures() {
        assert_eq!(receipt_rejection(&IntentReceipt::accepted()), None);
        let rejected = IntentReceipt::rejected(IntentError::UnknownScenario {
            scenario_id: "scenario:x".to_string(),
        });
        let message = receipt_rejection(&rejected).expect("rejected receipts report");
        assert!(message.contains("scenario:x"));
    }

    #[test]
    fn bar_percent_scales_only_parseable_numbers() {
        let series = SeriesProjection {
            id: "series:test".to_string(),
            label: "Test".to_string(),
            source: ComparativeSourceProjection::Published,
            unit: None,
            points: vec![
                SeriesPointProjection {
                    key: NodeKey::new("k:A"),
                    label: "A".to_string(),
                    value: number("50"),
                },
                SeriesPointProjection {
                    key: NodeKey::new("k:B"),
                    label: "B".to_string(),
                    value: number("-100"),
                },
                SeriesPointProjection {
                    key: NodeKey::new("k:C"),
                    label: "C".to_string(),
                    value: NodeValueProjection::Text("n/a".to_string()),
                },
            ],
        };
        let max_abs = series_max_abs(&series);
        assert_eq!(max_abs, 100.0);
        assert_eq!(bar_percent(&series.points[0].value, max_abs), Some(50.0));
        assert_eq!(bar_percent(&series.points[1].value, max_abs), Some(100.0));
        assert_eq!(bar_percent(&series.points[2].value, max_abs), None);
        assert_eq!(bar_percent(&number("0"), 0.0), Some(0.0));
    }

    #[test]
    fn column_provenance_marks_non_published_sources() {
        assert_eq!(
            column_provenance_class(&ComparativeSourceProjection::Published),
            ""
        );
        assert_eq!(
            column_provenance_class(&ComparativeSourceProjection::Candidate {
                handle: "c".to_string()
            }),
            "dtc-prov--speculative"
        );
        assert_eq!(
            column_provenance_class(&ComparativeSourceProjection::Scenario {
                id: "s".to_string()
            }),
            "dtc-prov--scenario"
        );
        assert_eq!(
            column_provenance_class(&ComparativeSourceProjection::SweepPoint {
                sweep_id: "sw".to_string(),
                point_id: "p0".to_string(),
                scenario_id: "s".to_string(),
            }),
            "dtc-prov--scenario"
        );
    }
}
