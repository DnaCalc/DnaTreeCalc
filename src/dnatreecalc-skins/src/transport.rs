//! TRANSPORT — the revision-DAG time scrubber (ATLAS lens).
//!
//! Transport renders the retained revision history as a vertical timeline rail:
//! undo as **navigation**, never inverse replay. Every card is an engine-owned
//! retained revision; clicking a non-current card dispatches
//! [`WorkspaceIntent::NavigateRevision`] and OxCalc owns the restoration
//! semantics. The lens never reconstructs, diffs, or replays model state — it
//! only reads the projected history facts and the typed invalidation
//! summaries the engine recorded for each transaction.
//!
//! Shared-spine conformance, exactly like `flow.rs`:
//!
//! - **one grammar** — `/` Name-Box, Enter/Esc modeless edit through
//!   `KeybindingRegistry`; Ctrl+Z/Y, F9, arrows, and lens-switch chords bubble
//!   to the shell;
//! - **one styling** — `calc_state` stays the only saturated channel (this
//!   lens renders no values of its own; the embedded inspector/console carry
//!   the calc dots), SELECTED/EDITING via the shared inspector;
//! - **one continuity** — selection flows through `SelectNode`, so jumping to
//!   an invalidated node lands every other lens on it too.
//!
//! The embedded **Lens** inspector and **Console** strip are the shared
//! [`crate::spine_widgets`] components.

use std::collections::BTreeMap;
use std::sync::Arc;

use dnatreecalc_skin_framework::{
    ATLAS_SPINE_CSS, CandidateProjection, CommandIntentKindProjection, Dispatcher, KeyChord,
    KeybindingRegistry, RevisionHistoryProjection, RevisionInvalidationSummaryEntryProjection,
    RevisionTransactionSummaryProjection, SelectionState, SkinCapabilities, SkinCategory,
    SkinContext, SkinHandle, SkinId, SkinManifest, SkinState, SkinStateHandle, SkinVerb,
    WorkspaceDelta, WorkspaceDeltaChange, WorkspaceIntent, WorkspaceSkin, WorkspaceState,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::spine_widgets::{
    ConsoleBar, NameBoxBar, NodeInspector, SPINE_WIDGETS_CSS, commit_content_edit,
    event_target_is_interactive, event_target_is_text_entry, focus_edit_buffer,
};

pub const TRANSPORT_ID: SkinId = SkinId::new("transport");

/// How many invalidated-node chips a timeline card shows before folding the
/// rest into a "+N more" tag.
const MAX_INVALIDATION_CHIPS: usize = 5;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportState {
    /// Whether each timeline card also shows its structural/input/namespace
    /// snapshot ids in faded monospace.
    pub show_snapshot_ids: bool,
}

impl SkinState for TransportState {
    fn schema_version() -> u32 {
        1
    }
}

#[derive(Default)]
pub struct TransportLens;

impl TransportLens {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSkin for TransportLens {
    type State = TransportState;

    fn id(&self) -> SkinId {
        TRANSPORT_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Transport",
            description: "Revision time-travel: navigate the retained history DAG.",
            category: SkinCategory::Inspector,
            version: "0.1.0",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities {
            supports_multi_select: false,
            supports_inline_formula_edit: false,
            // Transport shows history facts; invalidation summaries may name
            // meta structure and the timeline does not hide it.
            supports_meta_node_display: true,
            renders_arrays_inline: false,
            renders_table_values: false,
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        // Record the active lens for cross-lens chrome/continuity. Runs once.
        cx.shared.update(|state| {
            state.active_lens = Some(TRANSPORT_ID.as_str().to_string());
        });
        SkinHandle::new(view! { <TransportView cx=cx /> }.into_any())
    }
}

// ---------------------------------------------------------------------------
// Pure helpers — timeline building, intent builders, pulse summaries
// ---------------------------------------------------------------------------

/// One card on the timeline rail, derived purely from the projected history.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TimelineEntry {
    revision_id: String,
    parent_revision_id: Option<String>,
    transaction_id: Option<String>,
    is_current: bool,
    /// How many retained revisions name this one as their parent.
    /// `> 1` means this revision is a branch point in the retained DAG.
    branch_children: usize,
    summary: Option<RevisionTransactionSummaryProjection>,
}

/// Build the timeline, NEWEST-FIRST.
///
/// Ordering assumption: the host projects `revision_history.entries` in append
/// order (oldest first), so reversing it puts the current/new end of the DAG
/// at the top of the rail. Branch counting is order-independent: it scans the
/// whole projected history for entries naming each revision as their parent.
fn timeline_entries(history: &RevisionHistoryProjection) -> Vec<TimelineEntry> {
    let mut child_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for entry in &history.entries {
        if let Some(parent) = entry.parent_revision_id.as_deref() {
            *child_counts.entry(parent).or_insert(0) += 1;
        }
    }
    history
        .entries
        .iter()
        .rev()
        .map(|entry| TimelineEntry {
            revision_id: entry.revision_id.clone(),
            parent_revision_id: entry.parent_revision_id.clone(),
            transaction_id: entry.transaction_id.clone(),
            is_current: entry.is_current,
            branch_children: child_counts
                .get(entry.revision_id.as_str())
                .copied()
                .unwrap_or(0),
            summary: entry.transaction_summary.clone(),
        })
        .collect()
}

/// The one intent a timeline card may dispatch: navigate to a retained
/// revision. The current revision is not a navigation target (`None`), and
/// restoration semantics are engine-owned — this is never inverse replay.
fn navigate_intent(entry: &TimelineEntry) -> Option<WorkspaceIntent> {
    (!entry.is_current).then(|| WorkspaceIntent::NavigateRevision {
        revision_id: entry.revision_id.clone(),
    })
}

/// Summarize the latest [`WorkspaceDelta`] for the live pulse strip.
/// `None` when the delta carries no changes (nothing to pulse).
fn pulse_line(delta: &WorkspaceDelta) -> Option<String> {
    if delta.changes.is_empty() {
        return None;
    }
    let parts: Vec<String> = delta.changes.iter().map(pulse_change_summary).collect();
    Some(parts.join(" · "))
}

fn pulse_change_summary(change: &WorkspaceDeltaChange) -> String {
    match change {
        WorkspaceDeltaChange::FullReset => "full reset".to_string(),
        WorkspaceDeltaChange::Structural(delta) => {
            let mut line = format!(
                "structure: +{} -{}",
                delta.added.len(),
                delta.removed.len()
            );
            if !delta.changed.is_empty() {
                line.push_str(&format!(" ~{}", delta.changed.len()));
            }
            line
        }
        WorkspaceDeltaChange::NodesChanged(nodes) => {
            counted(nodes.len(), "node changed", "nodes changed")
        }
        WorkspaceDeltaChange::ValuesChanged(values) => {
            counted(values.len(), "value changed", "values changed")
        }
        WorkspaceDeltaChange::DepsChanged(deps) => counted(
            deps.len(),
            "dependency owner changed",
            "dependency owners changed",
        ),
        WorkspaceDeltaChange::CalcRun(_) => "calc run published".to_string(),
        WorkspaceDeltaChange::ClipboardChanged(Some(_)) => "clipboard updated".to_string(),
        WorkspaceDeltaChange::ClipboardChanged(None) => "clipboard cleared".to_string(),
        WorkspaceDeltaChange::FormulaReferenceInserted(_) => {
            "formula reference inserted".to_string()
        }
        WorkspaceDeltaChange::CandidateChanged(candidate) => {
            format!("candidate {} changed", candidate.handle)
        }
        WorkspaceDeltaChange::CandidateRemoved(handle) => format!("candidate {handle} removed"),
        WorkspaceDeltaChange::ScenarioChanged(scenario) => {
            format!("scenario {} changed", scenario.id)
        }
        WorkspaceDeltaChange::ScenarioRemoved(id) => format!("scenario {id} removed"),
        WorkspaceDeltaChange::SweepChanged(sweep) => format!("sweep {} changed", sweep.id),
        WorkspaceDeltaChange::SweepRemoved(id) => format!("sweep {id} removed"),
    }
}

fn counted(count: usize, singular: &str, plural: &str) -> String {
    if count == 1 {
        format!("1 {singular}")
    } else {
        format!("{count} {plural}")
    }
}

/// Count open candidates by the revision they are based on, so timeline cards
/// can badge "candidate based here".
fn candidate_basis_counts(candidates: &[CandidateProjection]) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for candidate in candidates {
        *counts
            .entry(candidate.basis_revision_id.clone())
            .or_insert(0) += 1;
    }
    counts
}

/// Compact display form of an opaque id (the full id always travels in the
/// `title` attribute). OxCalc revision/transaction ids are long *structured*
/// strings whose tails coincide across revisions, so a suffix slice collides;
/// an FNV-1a hash of the whole id gives a stable, distinguishable 8-hex handle.
/// Display-only — never used for addressing.
fn short_id(id: &str) -> String {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in id.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    format!("{hash:08x}")
}

/// Tooltip for an invalidated-node chip: the typed invalidation reasons,
/// plus the rebind marker when the engine flagged it.
fn invalidation_chip_title(invalidation: &RevisionInvalidationSummaryEntryProjection) -> String {
    let reasons: Vec<&str> = invalidation
        .reasons
        .iter()
        .map(|reason| reason.stable_id())
        .collect();
    let mut title = if reasons.is_empty() {
        "invalidated".to_string()
    } else {
        reasons.join(", ")
    };
    if invalidation.requires_rebind {
        title.push_str(" · requires rebind");
    }
    title
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

#[component]
fn TransportView(cx: SkinContext<TransportState>) -> impl IntoView {
    let workspace = cx.workspace;
    let latest_delta = cx.latest_delta;
    let selection = cx.selection;
    let shared = cx.shared;
    let dispatch = cx.dispatch.clone();
    let state = cx.state.clone();

    let editing = RwSignal::new(false);
    let editor_text = RwSignal::new(String::new());
    let name_box = RwSignal::new(Option::<String>::None);
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
    // revision navigation and external commits refresh it); while editing,
    // never clobber.
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
        }
    };

    // Lens-local grammar, resolved through the one registry. Global verbs
    // (F9, Ctrl+Z/Y, Ctrl+N, lens switch, arrows) bubble to the shell — in
    // particular Ctrl+Z/Y stay shell-owned even in the undo lens.
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
                // Enter on a focused button must activate the button —
                // timeline cards and transport buttons keep their native
                // Enter activation.
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
            }
            _ => {} // global verb: leave for the shell
        }
    };

    // Land keyboard focus on the rail when the lens mounts, so the grammar
    // works without a preparatory click.
    Effect::new(move |_| {
        if let Some(section) = stage_ref.get() {
            let _ = section.focus();
        }
    });

    // Live pulse over the latest published delta.
    let pulse = Memo::new(move |_| latest_delta.with(pulse_line));

    // The timeline rail: newest revision on top, candidate badges overlaid.
    let timeline_state = state.clone();
    let timeline_dispatch = dispatch.clone();
    let timeline = move || {
        let ws = workspace.get();
        let entries = timeline_entries(&ws.revision_history);
        if entries.is_empty() {
            return view! {
                <p class="dtc-transport-rail__empty">
                    "No revision history yet — edit the model to grow the timeline."
                </p>
            }
            .into_any();
        }
        let basis_counts = candidate_basis_counts(&ws.candidates);
        let show_ids = timeline_state.with(|s| s.show_snapshot_ids);
        let cards = entries
            .iter()
            .map(|entry| {
                let snapshot_ids = if show_ids {
                    ws.revision_history
                        .entries
                        .iter()
                        .find(|projected| projected.revision_id == entry.revision_id)
                        .map(|projected| {
                            (
                                projected.structural_snapshot_id.clone(),
                                projected.node_input_snapshot_id.clone(),
                                projected.namespace_snapshot_id.clone(),
                            )
                        })
                } else {
                    None
                };
                let candidate_count = basis_counts
                    .get(&entry.revision_id)
                    .copied()
                    .unwrap_or(0);
                timeline_card_view(
                    entry,
                    snapshot_ids,
                    candidate_count,
                    &ws,
                    timeline_dispatch.clone(),
                )
            })
            .collect::<Vec<_>>();
        view! {
            <div class="dtc-transport-rail" aria-label="Revision timeline">{cards}</div>
        }
        .into_any()
    };

    // Read-only side list of open candidates (overlay views on the DAG).
    let candidate_list = move || {
        let entries = workspace.with(|ws| {
            ws.candidates
                .iter()
                .map(|candidate| (candidate.handle.clone(), candidate.retention_pin_count))
                .collect::<Vec<_>>()
        });
        if entries.is_empty() {
            return ().into_any();
        }
        view! {
            <aside class="dtc-transport-candidates" aria-label="Open candidates">
                <div class="dtc-section-label">"Open candidates"</div>
                <ul class="dtc-transport-candidates__list">
                    {entries.into_iter().map(|(handle, pins)| {
                        let label = short_id(&handle).to_string();
                        let pins_label = counted(pins, "pin", "pins");
                        view! {
                            <li title=handle>
                                <code>{label}</code>
                                <span class="dtc-transport-candidates__pins">{pins_label}</span>
                            </li>
                        }
                    }).collect::<Vec<_>>()}
                </ul>
            </aside>
        }
        .into_any()
    };

    let css = format!("{ATLAS_SPINE_CSS}\n{SPINE_WIDGETS_CSS}\n{TRANSPORT_CSS}");
    let controls_state = state.clone();
    let console_dispatch = dispatch.clone();

    view! {
        <style>{css}</style>
        <section class="dtc-transport" tabindex="0" node_ref=stage_ref on:keydown=root_keydown>
            <div class="dtc-transport__body">
                <div class="dtc-transport__main">
                    <NameBoxBar name_box=name_box workspace=workspace dispatch=dispatch.clone() />
                    <TransportControls
                        workspace=workspace
                        selection=selection
                        state=controls_state
                        dispatch=dispatch.clone()
                    />
                    {move || pulse.get().map(|line| view! {
                        <div class="dtc-transport-pulse" aria-label="Latest change">{line}</div>
                    }.into_any())}
                    <div class="dtc-transport__columns">
                        <div class="dtc-transport__rail-scroll">{timeline}</div>
                        {candidate_list}
                    </div>
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

/// One timeline card. Non-current cards carry the navigation button; the
/// current card carries the "you are here" marker and is not clickable.
fn timeline_card_view(
    entry: &TimelineEntry,
    snapshot_ids: Option<(String, String, String)>,
    candidate_count: usize,
    workspace: &WorkspaceState,
    dispatch: Arc<dyn Dispatcher>,
) -> AnyView {
    let mut classes: Vec<&str> = vec!["dtc-transport-card"];
    if entry.is_current {
        classes.push("dtc-transport-card--current");
    }
    if entry.branch_children > 1 {
        classes.push("dtc-transport-card--branch");
    }
    let class_attr = classes.join(" ");

    let id_short = short_id(&entry.revision_id).to_string();
    let full_id = entry.revision_id.clone();

    let head = match navigate_intent(entry) {
        Some(intent) => {
            // Engine-owned restoration: dispatch the closed intent, never
            // replay edits ourselves.
            let nav_dispatch = dispatch.clone();
            let head_title = format!("Navigate to revision {full_id}");
            view! {
                <button
                    type="button"
                    class="dtc-transport-card__head"
                    title=head_title
                    on:click=move |_| {
                        nav_dispatch.dispatch(intent.clone());
                    }
                >
                    <span class="dtc-transport-card__id">{id_short}</span>
                </button>
            }
            .into_any()
        }
        None => view! {
            <div class="dtc-transport-card__head dtc-transport-card__head--current" title=full_id>
                <span class="dtc-transport-card__marker">"● you are here"</span>
                <span class="dtc-transport-card__id">{id_short}</span>
            </div>
        }
        .into_any(),
    };

    let tx_view = entry.transaction_id.as_ref().map(|tx| {
        let label = format!("tx {}", short_id(tx));
        let tx_full = tx.clone();
        view! { <span class="dtc-transport-card__tx" title=tx_full>{label}</span> }.into_any()
    });

    let branch_view = (entry.branch_children > 1).then(|| {
        view! {
            <span
                class="dtc-transport-card__badge"
                title="Branch point: multiple retained revisions share this parent"
            >
                {format!("branch ×{}", entry.branch_children)}
            </span>
        }
        .into_any()
    });

    let candidate_view = (candidate_count > 0).then(|| {
        let label = if candidate_count == 1 {
            "1 candidate based here".to_string()
        } else {
            format!("{candidate_count} candidates based here")
        };
        view! {
            <span class="dtc-transport-card__badge dtc-transport-card__badge--candidate">
                {label}
            </span>
        }
        .into_any()
    });

    let summary_view = entry.summary.as_ref().map(|summary| {
        let rebind_count = summary.requires_rebind.len();
        let chips = summary
            .invalidated_nodes
            .iter()
            .take(MAX_INVALIDATION_CHIPS)
            .map(|invalidation| invalidation_chip_view(invalidation, workspace, dispatch.clone()))
            .collect::<Vec<_>>();
        let overflow = summary
            .invalidated_nodes
            .len()
            .saturating_sub(MAX_INVALIDATION_CHIPS);
        view! {
            <div class="dtc-transport-card__summary">
                <span>{format!("{} nodes invalidated", summary.estimated_node_count)}</span>
                {(rebind_count > 0).then(|| view! {
                    <span class="dtc-transport-card__rebind">
                        {format!("{rebind_count} require rebind")}
                    </span>
                }.into_any())}
            </div>
            <div class="dtc-transport-card__chips">
                {chips}
                {(overflow > 0).then(|| view! {
                    <span class="dtc-transport-chip dtc-transport-chip--more">
                        {format!("+{overflow} more")}
                    </span>
                }.into_any())}
            </div>
        }
        .into_any()
    });

    let snapshots_view = snapshot_ids.map(|(structural, input, namespace)| {
        view! {
            <div class="dtc-transport-card__snapshots">
                <code>{format!("structural {structural}")}</code>
                <code>{format!("input {input}")}</code>
                <code>{format!("namespace {namespace}")}</code>
            </div>
        }
        .into_any()
    });

    view! {
        <article class=class_attr>
            {head}
            <div class="dtc-transport-card__meta">
                {tx_view}
                {branch_view}
                {candidate_view}
            </div>
            {summary_view}
            {snapshots_view}
        </article>
    }
    .into_any()
}

/// One invalidated-node chip. The chip shows the node's CURRENT display path
/// when the projection still resolves the stable key; a node that no longer
/// exists at this revision renders its raw key, faded and not clickable.
fn invalidation_chip_view(
    invalidation: &RevisionInvalidationSummaryEntryProjection,
    workspace: &WorkspaceState,
    dispatch: Arc<dyn Dispatcher>,
) -> AnyView {
    let title = invalidation_chip_title(invalidation);
    if let Some(id) = workspace.paths_by_key.get(&invalidation.node) {
        let id_for_click = id.clone();
        let label = id.as_str().to_string();
        view! {
            <button
                type="button"
                class="dtc-transport-chip"
                title=title
                on:click=move |_| {
                    dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id_for_click.clone())));
                }
            >
                {label}
            </button>
        }
        .into_any()
    } else {
        view! {
            <span class="dtc-transport-chip dtc-transport-chip--unresolved" title=title>
                {invalidation.node.to_string()}
            </span>
        }
        .into_any()
    }
}

/// The transport bar: Undo/Redo (enablement and disabled-reason read from the
/// projected command catalog), the "you are here" readout, and the
/// snapshot-id toggle.
#[component]
fn TransportControls(
    workspace: ReadSignal<WorkspaceState>,
    selection: ReadSignal<SelectionState>,
    state: SkinStateHandle<TransportState>,
    dispatch: Arc<dyn Dispatcher>,
) -> impl IntoView {
    let catalog = Memo::new(move |_| {
        workspace.with(|ws| selection.with(|sel| ws.command_catalog(sel)))
    });
    let undo_meta = Memo::new(move |_| {
        catalog.with(|c| c.get(CommandIntentKindProjection::Undo).cloned())
    });
    let redo_meta = Memo::new(move |_| {
        catalog.with(|c| c.get(CommandIntentKindProjection::Redo).cloned())
    });
    let here = Memo::new(move |_| {
        workspace.with(|ws| {
            (
                ws.revision_history.current_revision_id.clone(),
                ws.revision.value_epoch,
            )
        })
    });
    let show_ids = {
        let state = state.clone();
        Memo::new(move |_| state.with(|s| s.show_snapshot_ids))
    };
    let undo_dispatch = dispatch.clone();
    let redo_dispatch = dispatch.clone();
    let toggle_state = state.clone();

    view! {
        <div class="dtc-transport-controls" aria-label="Transport controls">
            <button
                type="button"
                disabled=move || !undo_meta.get().is_some_and(|meta| meta.enabled)
                title=move || undo_meta
                    .get()
                    .and_then(|meta| meta.disabled_reason)
                    .unwrap_or_else(|| "Navigate to the parent revision (Ctrl+Z)".to_string())
                on:click=move |_| {
                    undo_dispatch.dispatch(WorkspaceIntent::Undo);
                }
            >
                "◀ Undo"
            </button>
            <button
                type="button"
                disabled=move || !redo_meta.get().is_some_and(|meta| meta.enabled)
                title=move || redo_meta
                    .get()
                    .and_then(|meta| meta.disabled_reason)
                    .unwrap_or_else(|| "Navigate forward after undo (Ctrl+Y)".to_string())
                on:click=move |_| {
                    redo_dispatch.dispatch(WorkspaceIntent::Redo);
                }
            >
                "Redo ▶"
            </button>
            <span class="dtc-transport-controls__here" title="Current revision and value epoch">
                {move || {
                    let (revision, epoch) = here.get();
                    match revision {
                        Some(revision) => format!("@ {} · epoch {epoch}", short_id(&revision)),
                        None => format!("@ unrevisioned · epoch {epoch}"),
                    }
                }}
            </span>
            <span class="dtc-transport-controls__spacer"></span>
            <button
                type="button"
                class:dtc-transport-controls__toggle--on=move || show_ids.get()
                title="Show structural/input/namespace snapshot ids on each card"
                on:click=move |_| toggle_state.update(|s| s.show_snapshot_ids = !s.show_snapshot_ids)
            >
                "snapshot ids"
            </button>
        </div>
    }
}

const TRANSPORT_CSS: &str = r#"
.dtc-transport {
  display: flex; flex-direction: column; height: 100%;
  background: var(--dtc-surface); color: var(--dtc-text);
  font: 13px/1.4 var(--dtc-font, system-ui, sans-serif);
  outline: none;
}
.dtc-transport__body { display: flex; flex: 1; min-height: 0; }
.dtc-transport__main { flex: 1; display: flex; flex-direction: column; min-width: 0; padding: 8px 12px; overflow: hidden; }
.dtc-transport__columns { display: flex; flex: 1; gap: 12px; min-height: 0; }
.dtc-transport__rail-scroll { flex: 1; overflow: auto; min-width: 0; padding: 4px 4px 4px 0; }

.dtc-transport-controls {
  display: flex; align-items: center; gap: 8px; padding: 6px 8px; margin-bottom: 8px;
  background: var(--dtc-surface-subtle); border: 1px solid var(--dtc-border-muted); border-radius: 6px;
}
.dtc-transport-controls button {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 2px 8px; cursor: pointer; color: var(--dtc-text);
}
.dtc-transport-controls button:disabled { opacity: 0.45; cursor: default; }
.dtc-transport-controls__here { font-variant-numeric: tabular-nums; color: var(--dtc-text-muted); font-family: ui-monospace, monospace; font-size: 12px; }
.dtc-transport-controls__spacer { flex: 1; }
.dtc-transport-controls__toggle--on { border-color: var(--dtc-accent-border); box-shadow: inset 0 0 0 1px var(--dtc-accent-border); }

.dtc-transport-pulse {
  padding: 4px 8px; margin-bottom: 8px; font-size: 12px; color: var(--dtc-text-muted);
  border: 1px dashed var(--dtc-border-muted); border-radius: 6px;
}

.dtc-transport-rail { display: flex; flex-direction: column; padding-left: 16px; border-left: 2px solid var(--dtc-border); }
.dtc-transport-rail__empty { color: var(--dtc-text-subtle); font-style: italic; padding: 8px 0; }

.dtc-transport-card {
  position: relative; margin: 0 0 10px; padding: 8px 10px;
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 7px;
  box-shadow: 0 1px 2px var(--dtc-shadow-soft);
}
.dtc-transport-card::before {
  content: ""; position: absolute; left: -22px; top: 13px; width: 10px; height: 10px;
  border-radius: 50%; background: var(--dtc-border-strong);
}
.dtc-transport-card--current { border-color: var(--dtc-accent-border); }
.dtc-transport-card--current::before { background: var(--dtc-accent); }
.dtc-transport-card--branch { border-left: 3px solid var(--dtc-border-strong); }

.dtc-transport-card__head {
  display: flex; align-items: center; gap: 8px; width: 100%;
  background: none; border: none; padding: 0; margin: 0; text-align: left;
  color: var(--dtc-text); font: inherit; cursor: pointer;
}
.dtc-transport-card__head:hover .dtc-transport-card__id { text-decoration: underline; }
.dtc-transport-card__head--current { cursor: default; }
.dtc-transport-card__marker { color: var(--dtc-accent); font-weight: 600; }
.dtc-transport-card__id { font-family: ui-monospace, monospace; font-weight: 600; }

.dtc-transport-card__meta { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; margin-top: 4px; }
.dtc-transport-card__tx { color: var(--dtc-text-subtle); font-family: ui-monospace, monospace; font-size: 11px; }
.dtc-transport-card__badge {
  padding: 0 7px; border: 1px solid var(--dtc-border); border-radius: 9px;
  font-size: 11px; color: var(--dtc-text-muted);
}
.dtc-transport-card__badge--candidate { border-style: dashed; }

.dtc-transport-card__summary {
  display: flex; flex-wrap: wrap; align-items: center; gap: 8px; margin-top: 6px;
  color: var(--dtc-text-muted); font-size: 12px;
}
.dtc-transport-card__rebind { color: var(--dtc-warning); }
.dtc-transport-card__chips { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 4px; }
.dtc-transport-chip {
  padding: 1px 7px; border: 1px solid var(--dtc-border); border-radius: 9px;
  background: var(--dtc-surface-subtle); color: var(--dtc-text); font-size: 11px; cursor: pointer;
}
.dtc-transport-chip:hover { border-color: var(--dtc-border-strong); }
.dtc-transport-chip--unresolved { opacity: 0.5; cursor: default; }
.dtc-transport-chip--more { cursor: default; color: var(--dtc-text-subtle); }

.dtc-transport-card__snapshots { display: flex; flex-direction: column; gap: 1px; margin-top: 6px; }
.dtc-transport-card__snapshots code {
  font-family: ui-monospace, monospace; font-size: 10px;
  color: var(--dtc-text-subtle); opacity: 0.8;
}

.dtc-transport-candidates {
  width: 210px; flex-shrink: 0; overflow: auto; padding: 8px; align-self: flex-start;
  background: var(--dtc-surface-subtle); border: 1px solid var(--dtc-border-muted); border-radius: 6px;
}
.dtc-transport-candidates__list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
.dtc-transport-candidates__list li { display: flex; justify-content: space-between; gap: 8px; }
.dtc-transport-candidates__list code { font-family: ui-monospace, monospace; }
.dtc-transport-candidates__pins { color: var(--dtc-text-subtle); font-size: 11px; }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::{
        InvalidationReasonProjection, NodeKey, NodeValueDeltaProjection, NodeValueProjection,
        RevisionHistoryEntryProjection, StructuralDeltaProjection,
    };

    fn history_entry(
        revision_id: &str,
        parent: Option<&str>,
        is_current: bool,
    ) -> RevisionHistoryEntryProjection {
        RevisionHistoryEntryProjection {
            revision_id: revision_id.to_string(),
            parent_revision_id: parent.map(str::to_string),
            structural_snapshot_id: format!("structural:{revision_id}"),
            node_input_snapshot_id: format!("input:{revision_id}"),
            namespace_snapshot_id: format!("namespace:{revision_id}"),
            transaction_id: Some(format!("tx:{revision_id}")),
            transaction_summary: None,
            is_current,
        }
    }

    /// r1 → r2 and r1 → r3 (branch at r1); r3 is current. Projected order is
    /// oldest-first, mirroring the host's append order.
    fn branched_history() -> RevisionHistoryProjection {
        RevisionHistoryProjection {
            current_revision_id: Some("r3".to_string()),
            entries: vec![
                history_entry("r1", None, false),
                history_entry("r2", Some("r1"), false),
                history_entry("r3", Some("r1"), true),
            ],
        }
    }

    fn invalidation(
        key: &str,
        requires_rebind: bool,
        reasons: Vec<InvalidationReasonProjection>,
    ) -> RevisionInvalidationSummaryEntryProjection {
        RevisionInvalidationSummaryEntryProjection {
            node: NodeKey::new(key),
            requires_rebind,
            reasons,
        }
    }

    #[test]
    fn timeline_is_newest_first() {
        let entries = timeline_entries(&branched_history());
        let ids: Vec<&str> = entries.iter().map(|e| e.revision_id.as_str()).collect();
        assert_eq!(ids, vec!["r3", "r2", "r1"]);
        assert!(entries[0].is_current, "the current revision sits on top");
        assert_eq!(entries[0].transaction_id.as_deref(), Some("tx:r3"));
        assert_eq!(entries[0].parent_revision_id.as_deref(), Some("r1"));
    }

    #[test]
    fn branch_children_counts_entries_sharing_a_parent() {
        let entries = timeline_entries(&branched_history());
        let by_id = |id: &str| {
            entries
                .iter()
                .find(|entry| entry.revision_id == id)
                .expect("entry present")
        };
        assert_eq!(
            by_id("r1").branch_children,
            2,
            "r2 and r3 both name r1 as their parent — branch point"
        );
        assert_eq!(by_id("r2").branch_children, 0);
        assert_eq!(by_id("r3").branch_children, 0);
    }

    #[test]
    fn timeline_preserves_transaction_summaries() {
        let mut history = branched_history();
        history.entries[2].transaction_summary = Some(RevisionTransactionSummaryProjection {
            transaction_id: "tx:r3".to_string(),
            invalidated_nodes: vec![invalidation(
                "k:a",
                false,
                vec![InvalidationReasonProjection::UpstreamPublication],
            )],
            requires_rebind: Vec::new(),
            estimated_node_count: 1,
        });
        let entries = timeline_entries(&history);
        let summary = entries[0]
            .summary
            .as_ref()
            .expect("newest entry carries the summary");
        assert_eq!(summary.estimated_node_count, 1);
        assert_eq!(summary.invalidated_nodes.len(), 1);
    }

    #[test]
    fn navigate_intent_skips_the_current_revision() {
        let entries = timeline_entries(&branched_history());
        let current = entries.iter().find(|entry| entry.is_current).unwrap();
        let past = entries
            .iter()
            .find(|entry| entry.revision_id == "r2")
            .unwrap();
        assert_eq!(navigate_intent(current), None, "current is not clickable");
        assert_eq!(
            navigate_intent(past),
            Some(WorkspaceIntent::NavigateRevision {
                revision_id: "r2".to_string(),
            })
        );
    }

    fn delta_with(changes: Vec<WorkspaceDeltaChange>) -> WorkspaceDelta {
        WorkspaceDelta {
            from_seq: 1,
            to_seq: 2,
            changes,
        }
    }

    fn value_delta(key: &str) -> NodeValueDeltaProjection {
        NodeValueDeltaProjection {
            node: NodeKey::new(key),
            value: NodeValueProjection::Number {
                raw: "1".to_string(),
                display: "1".to_string(),
            },
        }
    }

    #[test]
    fn pulse_line_summarizes_value_changes() {
        let delta = delta_with(vec![WorkspaceDeltaChange::ValuesChanged(vec![
            value_delta("k:a"),
            value_delta("k:b"),
            value_delta("k:c"),
        ])]);
        assert_eq!(pulse_line(&delta), Some("3 values changed".to_string()));

        let single = delta_with(vec![WorkspaceDeltaChange::ValuesChanged(vec![value_delta(
            "k:a",
        )])]);
        assert_eq!(pulse_line(&single), Some("1 value changed".to_string()));
    }

    #[test]
    fn pulse_line_summarizes_structural_changes() {
        let delta = delta_with(vec![WorkspaceDeltaChange::Structural(
            StructuralDeltaProjection {
                added: vec![NodeKey::new("k:a"), NodeKey::new("k:b")],
                removed: vec![NodeKey::new("k:c")],
                changed: Vec::new(),
            },
        )]);
        assert_eq!(pulse_line(&delta), Some("structure: +2 -1".to_string()));
    }

    #[test]
    fn pulse_line_reports_full_reset_and_overlay_kinds() {
        assert_eq!(
            pulse_line(&delta_with(vec![WorkspaceDeltaChange::FullReset])),
            Some("full reset".to_string())
        );
        assert_eq!(
            pulse_line(&delta_with(vec![
                WorkspaceDeltaChange::CandidateRemoved("cand-7".to_string()),
                WorkspaceDeltaChange::ClipboardChanged(None),
            ])),
            Some("candidate cand-7 removed · clipboard cleared".to_string())
        );
    }

    #[test]
    fn pulse_line_is_none_when_nothing_changed() {
        assert_eq!(pulse_line(&WorkspaceDelta::unchanged(4)), None);
    }

    fn candidate(handle: &str, basis: &str) -> CandidateProjection {
        CandidateProjection {
            handle: handle.to_string(),
            basis_revision_id: basis.to_string(),
            parent_handle: None,
            retention_pin_count: 0,
            workspace_revision_id: format!("wr:{handle}"),
            revision_history: RevisionHistoryProjection::default(),
            nodes: Vec::new(),
            values_by_key: BTreeMap::new(),
            run: None,
        }
    }

    #[test]
    fn candidate_basis_counts_group_by_revision() {
        let candidates = vec![
            candidate("c1", "r1"),
            candidate("c2", "r1"),
            candidate("c3", "r2"),
        ];
        let counts = candidate_basis_counts(&candidates);
        assert_eq!(counts.get("r1"), Some(&2));
        assert_eq!(counts.get("r2"), Some(&1));
        assert_eq!(counts.get("r3"), None);
    }

    #[test]
    fn short_id_distinguishes_ids_that_share_a_tail() {
        // OxCalc revision ids are structured strings with coinciding tails —
        // the display hash must differ even when the suffix matches.
        let a = short_id("rev:structural=1;input=2;namespace:n=4:none");
        let b = short_id("rev:structural=9;input=7;namespace:n=4:none");
        assert_ne!(a, b);
        assert_eq!(a.len(), 8);
        assert_eq!(b.len(), 8);
        // Deterministic across calls.
        assert_eq!(a, short_id("rev:structural=1;input=2;namespace:n=4:none"));
    }

    #[test]
    fn invalidation_chip_title_lists_typed_reasons() {
        let entry = invalidation(
            "k:a",
            true,
            vec![
                InvalidationReasonProjection::UpstreamPublication,
                InvalidationReasonProjection::DependencyAdded,
            ],
        );
        assert_eq!(
            invalidation_chip_title(&entry),
            "upstream_publication, dependency_added · requires rebind"
        );

        let bare = invalidation("k:b", false, Vec::new());
        assert_eq!(invalidation_chip_title(&bare), "invalidated");
    }

    #[test]
    fn counted_handles_singular_and_plural() {
        assert_eq!(counted(1, "pin", "pins"), "1 pin");
        assert_eq!(counted(3, "pin", "pins"), "3 pins");
        assert_eq!(counted(0, "pin", "pins"), "0 pins");
    }
}
