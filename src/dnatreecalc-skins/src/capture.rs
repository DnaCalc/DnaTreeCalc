//! CAPTURE — structure-by-typing (ATLAS Phase A).
//!
//! Capture's signature is one large input line: type a dotted path and the
//! tree scaffolds itself. `Accounts.Q1.Sales = 5` creates every missing
//! segment on the way down and hands the content after the first `=` to the
//! host **verbatim** — `Margin = =Net/Sales` authors a formula, `Margin = 5`
//! a constant; classification of content is host truth, never lens truth.
//!
//! Doctrine, identical to the Flow reference lens:
//! - **one grammar** — `/` Name-Box, Enter/Esc modeless edit, resolved through
//!   `KeybindingRegistry::universal()`; bare keys never fire while typing;
//! - **one styling** — `calc_state` is the only saturated channel
//!   (`calc_state_class`), provenance is structural (`provenance_tint`), and a
//!   1-bit SELECTED/EDITING border (`selection_mode_class`);
//! - **one continuity** — selection is shared view-state, so a later lens
//!   lands on the same node.
//!
//! Multi-segment scaffolds run through the **candidate lane**
//! (`OpenCandidate` → `AddCandidateNode`* → `EvaluateCandidate` →
//! `CommitCandidate`) so the whole scaffold publishes as ONE revision — one
//! undo. Any rejection discards the candidate: nothing publishes, true
//! atomicity. Capture reads only the published projection and writes only
//! closed `WorkspaceIntent`s; it never parses formula text (its dotted-path
//! line syntax is lens-local, the `=` split hands content through untouched),
//! never computes a value, and never uses grid coordinates.

use std::collections::BTreeSet;
use std::sync::Arc;

use dnatreecalc_skin_framework::{
    ATLAS_SPINE_CSS, Dispatcher, InitialNodeContentProjection, IntentReceipt, KeyChord,
    KeybindingRegistry, NodeId, NodeKey, NodeView, SharedSkinStateHandle, SkinCapabilities,
    SkinCategory, SkinContext, SkinHandle, SkinId, SkinManifest, SkinState, SkinVerb,
    TemplateProjection, WorkspaceIntent, WorkspaceSkin, WorkspaceState, calc_state_class,
    provenance_tint, selection_mode_class,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::spine_widgets::{
    ConsoleBar, NameBoxBar, NodeInspector, SPINE_WIDGETS_CSS, commit_content_edit,
    event_target_is_interactive, event_target_is_text_entry,
};
use crate::value_render::value_text;

pub const CAPTURE_ID: SkinId = SkinId::new("capture");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureState {
    /// How many capture-history entries to retain (newest first).
    pub history_limit: u32,
}

impl Default for CaptureState {
    fn default() -> Self {
        Self { history_limit: 20 }
    }
}

impl SkinState for CaptureState {
    fn schema_version() -> u32 {
        1
    }
}

#[derive(Default)]
pub struct CaptureLens;

impl CaptureLens {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSkin for CaptureLens {
    type State = CaptureState;

    fn id(&self) -> SkinId {
        CAPTURE_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Capture",
            description: "Structure-by-typing: dotted paths scaffold the tree as you enter them.",
            category: SkinCategory::Editor,
            version: "0.1.0",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities {
            supports_multi_select: false,
            supports_inline_formula_edit: true,
            supports_meta_node_display: false,
            renders_arrays_inline: false,
            renders_table_values: false,
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        // Record the active lens for cross-lens chrome/continuity. Runs once.
        cx.shared.update(|state| {
            state.active_lens = Some(CAPTURE_ID.as_str().to_string());
        });
        SkinHandle::new(view! { <CaptureView cx=cx /> }.into_any())
    }
}

// ---------------------------------------------------------------------------
// Pure helpers (lens-local line syntax + scaffold planning — unit tested)
// ---------------------------------------------------------------------------

/// One parsed capture line. The split at the FIRST `=` is lens-local *line*
/// syntax, not formula parsing: everything after it is handed to the host
/// verbatim, so a leading `=` in the content authors a formula there.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CaptureLine {
    path_segments: Vec<String>,
    content: Option<String>,
}

/// Parse `Dotted.Path.Leaf` or `Dotted.Path.Leaf = content`. Rejects an empty
/// path and empty segment names; segment names are trimmed.
fn parse_capture_line(line: &str) -> Option<CaptureLine> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (path_part, content) = match trimmed.split_once('=') {
        Some((path, rest)) => (path, Some(rest.trim().to_string())),
        None => (trimmed, None),
    };
    let path_part = path_part.trim();
    if path_part.is_empty() {
        return None;
    }
    let mut path_segments = Vec::new();
    for raw_segment in path_part.split('.') {
        let segment = raw_segment.trim();
        if segment.is_empty() {
            return None;
        }
        path_segments.push(segment.to_string());
    }
    Some(CaptureLine {
        path_segments,
        content,
    })
}

/// Where an entered path lands against the published projection.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ScaffoldPlan {
    /// Deepest existing prefix node; `None` means scaffold from the root.
    existing_parent: Option<(NodeId, NodeKey)>,
    /// Remaining segment names to create, in order. Empty means the line
    /// targets an existing node.
    missing: Vec<String>,
}

/// Find the longest existing prefix by joining segments progressively and
/// looking them up in `ws.nodes` (`NodeId` IS the dotted path — projection
/// truth, never re-derived).
fn plan_scaffold(ws: &WorkspaceState, segments: &[String]) -> ScaffoldPlan {
    let mut existing_parent = None;
    let mut matched = 0usize;
    let mut path = String::new();
    for (index, segment) in segments.iter().enumerate() {
        if index > 0 {
            path.push('.');
        }
        path.push_str(segment);
        match ws.node(&NodeId::new(path.clone())) {
            Some(node) => {
                existing_parent = Some((node.id.clone(), node.key.clone()));
                matched = index + 1;
            }
            None => break,
        }
    }
    ScaffoldPlan {
        existing_parent,
        missing: segments[matched..].to_vec(),
    }
}

/// After a successful line, the input resets to the parent path plus a
/// trailing dot so Enter drops the next sibling:
/// `Accounts.Q1.Sales = 5` leaves `Accounts.Q1.` ready.
fn sibling_prefill(segments: &[String]) -> String {
    if segments.len() <= 1 {
        return String::new();
    }
    let mut prefill = segments[..segments.len() - 1].join(".");
    prefill.push('.');
    prefill
}

/// Resolve the LEAF initial: explicit content wins, then an armed template
/// (cloned verbatim from the projection), then `Empty`.
fn leaf_initial(
    content: Option<String>,
    armed: Option<InitialNodeContentProjection>,
) -> InitialNodeContentProjection {
    match (content, armed) {
        (Some(content), _) => InitialNodeContentProjection::Literal { content },
        (None, Some(initial)) => initial,
        (None, None) => InitialNodeContentProjection::Empty,
    }
}

/// The single-missing-segment `AddNode` payload. Parent is the existing
/// prefix's `NodeId` (`None` scaffolds at the root).
fn build_single_add_node(
    parent: Option<NodeId>,
    symbol: &str,
    initial: InitialNodeContentProjection,
) -> WorkspaceIntent {
    WorkspaceIntent::AddNode {
        parent,
        symbol: symbol.to_string(),
        initial,
        is_meta: false,
    }
}

/// One entry on the lens-local history strip.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CaptureEntry {
    line: String,
    accepted: bool,
    message: Option<String>,
}

/// Newest first, capped at the configured limit.
fn push_capture_history(entries: &mut Vec<CaptureEntry>, entry: CaptureEntry, limit: u32) {
    entries.insert(0, entry);
    entries.truncate(limit as usize);
}

/// The typed error Display text from a rejected receipt — never invented.
fn receipt_error_text(receipt: &IntentReceipt) -> String {
    receipt
        .error
        .as_ref()
        .map_or_else(|| "the host rejected the intent".to_string(), ToString::to_string)
}

/// Depth-first visible-rows walk for the echo pane. Effectively-meta subtrees
/// are pruned via [`WorkspaceState::is_effective_meta`] (never the raw flag).
fn echo_nodes(workspace: &WorkspaceState) -> Vec<NodeView> {
    let mut rows = Vec::new();
    let mut stack: Vec<NodeId> = workspace.root_paths.iter().rev().cloned().collect();
    while let Some(id) = stack.pop() {
        let Some(node) = workspace.node(&id) else {
            continue;
        };
        if workspace.is_effective_meta(&node.key) {
            continue;
        }
        rows.push(node.clone());
        for child in node.children.iter().rev() {
            stack.push(child.clone());
        }
    }
    rows
}

// ---------------------------------------------------------------------------
// Execution (frame only: closed intents in, typed receipts out)
// ---------------------------------------------------------------------------

/// An armed template starter: the projected initial cloned verbatim, plus the
/// name so the chip row can show what is armed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ArmedTemplate {
    name: String,
    initial: InitialNodeContentProjection,
}

/// Discard a failed candidate and pass the typed error through — nothing
/// publishes on any rejection.
fn discard_and_report(dispatch: &Arc<dyn Dispatcher>, handle: &str, message: String) -> String {
    dispatch.dispatch(WorkspaceIntent::DiscardCandidate {
        handle: handle.to_string(),
    });
    message
}

/// Execute one parsed capture line against the host.
///
/// - existing node: commit content (recalc-mode aware) or just select;
/// - one missing segment: a single `AddNode` (armed template may supply the
///   initial when the line has no explicit content);
/// - two or more: the candidate lane, so the whole scaffold publishes as one
///   revision (one undo).
fn execute_capture_line(
    line: &CaptureLine,
    workspace: ReadSignal<WorkspaceState>,
    dispatch: &Arc<dyn Dispatcher>,
    shared: SharedSkinStateHandle,
    pending_template: RwSignal<Option<ArmedTemplate>>,
) -> Result<(), String> {
    let ws = workspace.get_untracked();
    let plan = plan_scaffold(&ws, &line.path_segments);
    let full_path = line.path_segments.join(".");

    if plan.missing.is_empty() {
        // The line targets an existing node.
        let Some((id, _key)) = plan.existing_parent else {
            return Err("capture line resolved to no node".to_string());
        };
        if let Some(content) = &line.content {
            let receipt = commit_content_edit(dispatch, shared, id.clone(), content.clone());
            if !receipt.accepted {
                return Err(receipt_error_text(&receipt));
            }
        }
        dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id)));
        return Ok(());
    }

    if plan.missing.len() == 1 {
        let symbol = plan.missing[0].clone();
        let armed = if line.content.is_none() {
            pending_template.get_untracked()
        } else {
            None
        };
        let used_template = armed.is_some();
        let initial = leaf_initial(line.content.clone(), armed.map(|template| template.initial));
        let parent = plan.existing_parent.map(|(id, _)| id);
        let receipt = dispatch.dispatch(build_single_add_node(parent, &symbol, initial));
        if !receipt.accepted {
            return Err(receipt_error_text(&receipt));
        }
        if used_template {
            pending_template.set(None);
        }
        dispatch.dispatch(WorkspaceIntent::SelectNode(Some(NodeId::new(full_path))));
        return Ok(());
    }

    // Candidate lane: the whole scaffold publishes as ONE revision.
    let before: BTreeSet<String> = ws
        .candidates
        .iter()
        .map(|candidate| candidate.handle.clone())
        .collect();
    let receipt = dispatch.dispatch(WorkspaceIntent::OpenCandidate { parent: None });
    if !receipt.accepted {
        return Err(receipt_error_text(&receipt));
    }
    let handle = workspace
        .get_untracked()
        .candidates
        .iter()
        .map(|candidate| candidate.handle.clone())
        .find(|handle| !before.contains(handle))
        .ok_or_else(|| "the host did not surface a new candidate handle".to_string())?;

    let mut parent_key = plan
        .existing_parent
        .as_ref()
        .map(|(_, key)| key.clone());
    let mut path_so_far = plan
        .existing_parent
        .as_ref()
        .map(|(id, _)| id.as_str().to_string())
        .unwrap_or_default();
    let last = plan.missing.len() - 1;

    for (index, symbol) in plan.missing.iter().enumerate() {
        let initial = if index == last {
            match &line.content {
                Some(content) => InitialNodeContentProjection::Literal {
                    content: content.clone(),
                },
                None => InitialNodeContentProjection::Empty,
            }
        } else {
            InitialNodeContentProjection::Empty
        };
        let receipt = dispatch.dispatch(WorkspaceIntent::AddCandidateNode {
            handle: handle.clone(),
            parent: parent_key.clone(),
            symbol: symbol.clone(),
            initial,
            is_meta: false,
        });
        if !receipt.accepted {
            return Err(discard_and_report(
                dispatch,
                &handle,
                receipt_error_text(&receipt),
            ));
        }
        // Locate the created node in the candidate projection by its dotted
        // path so the next segment can parent under it.
        path_so_far = if path_so_far.is_empty() {
            symbol.clone()
        } else {
            format!("{path_so_far}.{symbol}")
        };
        let created_key = workspace
            .get_untracked()
            .candidates
            .iter()
            .find(|candidate| candidate.handle == handle)
            .and_then(|candidate| {
                candidate
                    .nodes
                    .iter()
                    .find(|node| node.id.as_str() == path_so_far)
                    .map(|node| node.key.clone())
            });
        let Some(created_key) = created_key else {
            return Err(discard_and_report(
                dispatch,
                &handle,
                format!("candidate projection did not surface {path_so_far}"),
            ));
        };
        parent_key = Some(created_key);
    }

    let receipt = dispatch.dispatch(WorkspaceIntent::EvaluateCandidate {
        handle: handle.clone(),
    });
    if !receipt.accepted {
        return Err(discard_and_report(
            dispatch,
            &handle,
            receipt_error_text(&receipt),
        ));
    }
    let receipt = dispatch.dispatch(WorkspaceIntent::CommitCandidate {
        handle: handle.clone(),
    });
    if !receipt.accepted {
        return Err(discard_and_report(
            dispatch,
            &handle,
            receipt_error_text(&receipt),
        ));
    }
    dispatch.dispatch(WorkspaceIntent::SelectNode(Some(NodeId::new(full_path))));
    Ok(())
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

#[component]
fn CaptureView(cx: SkinContext<CaptureState>) -> impl IntoView {
    let workspace = cx.workspace;
    let selection = cx.selection;
    let shared = cx.shared;
    let dispatch = cx.dispatch.clone();
    let state = cx.state.clone();

    let editing = RwSignal::new(false);
    let editor_text = RwSignal::new(String::new());
    let name_box = RwSignal::new(Option::<String>::None);
    let capture_text = RwSignal::new(String::new());
    let pending_template = RwSignal::new(Option::<ArmedTemplate>::None);
    let history = RwSignal::new(Vec::<CaptureEntry>::new());
    let edit_ref = NodeRef::<leptos::html::Textarea>::new();
    let root_ref = NodeRef::<leptos::html::Section>::new();
    let line_ref = NodeRef::<leptos::html::Input>::new();

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
                // The capture line IS this lens's edit surface — Commit lands
                // the caret there, ready to type the next path.
                ev.prevent_default();
                if let Some(input) = line_ref.get_untracked() {
                    let _ = input.focus();
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

    // Land keyboard focus on the lens when it mounts, so the grammar works
    // without a preparatory click.
    Effect::new(move |_| {
        if let Some(section) = root_ref.get() {
            let _ = section.focus();
        }
    });

    // Enter in the capture line: parse, plan, execute, record, prefill.
    let submit_dispatch = dispatch.clone();
    let submit = move || {
        let line = capture_text.get_untracked().trim().to_string();
        if line.is_empty() {
            return;
        }
        let limit = state.get_untracked().history_limit;
        let outcome = match parse_capture_line(&line) {
            None => Err(
                "unrecognized capture line — type Dotted.Path or Dotted.Path = content"
                    .to_string(),
            ),
            Some(parsed) => execute_capture_line(
                &parsed,
                workspace,
                &submit_dispatch,
                shared,
                pending_template,
            )
            .map(|()| parsed),
        };
        match outcome {
            Ok(parsed) => {
                history.update(|entries| {
                    push_capture_history(
                        entries,
                        CaptureEntry {
                            line,
                            accepted: true,
                            message: None,
                        },
                        limit,
                    );
                });
                capture_text.set(sibling_prefill(&parsed.path_segments));
            }
            Err(message) => {
                history.update(|entries| {
                    push_capture_history(
                        entries,
                        CaptureEntry {
                            line,
                            accepted: false,
                            message: Some(message),
                        },
                        limit,
                    );
                });
            }
        }
    };

    // Template starter chips: projected initials, cloned verbatim when armed.
    let template_chips = move || {
        let templates = workspace.with(|ws| ws.templates.entries.clone());
        let armed = pending_template.get();
        if templates.is_empty() && armed.is_none() {
            return ().into_any();
        }
        let chips = templates
            .into_iter()
            .map(|template| template_chip_view(template, pending_template))
            .collect::<Vec<_>>();
        view! {
            <div class="dtc-capture-templates" aria-label="Template starters">
                <span class="dtc-section-label">"Templates"</span>
                <div class="dtc-capture-templates__row">
                    {chips}
                    {armed.map(|armed| view! {
                        <span class="dtc-capture-templates__armed">
                            <span>{format!("armed: {}", armed.name)}</span>
                            <button
                                type="button"
                                title="Disarm template"
                                on:click=move |_| pending_template.set(None)
                            >
                                "clear"
                            </button>
                        </span>
                    }.into_any())}
                </div>
            </div>
        }
        .into_any()
    };

    // History strip: newest first; rejected entries carry the typed error.
    let history_view = move || {
        let entries = history.get();
        if entries.is_empty() {
            return ().into_any();
        }
        view! {
            <div class="dtc-capture-history" aria-label="Capture history">
                {entries.into_iter().map(history_entry_view).collect::<Vec<_>>()}
            </div>
        }
        .into_any()
    };

    // Echo pane: a read-only indented outline so structure visibly grows as
    // lines land.
    let echo_dispatch = dispatch.clone();
    let echo = move || {
        let ws = workspace.get();
        let sel = selection.with(|s| s.primary.clone());
        let rows = echo_nodes(&ws)
            .into_iter()
            .map(|node| {
                let is_selected = sel.as_ref() == Some(&node.id);
                echo_row_view(node, is_selected, &ws, echo_dispatch.clone(), root_ref)
            })
            .collect::<Vec<_>>();
        if rows.is_empty() {
            view! {
                <p class="dtc-capture-echo__empty">
                    "No nodes yet — type a dotted path above to scaffold the tree."
                </p>
            }
            .into_any()
        } else {
            view! { <div class="dtc-capture-echo__rows">{rows}</div> }.into_any()
        }
    };

    let css = format!("{ATLAS_SPINE_CSS}\n{SPINE_WIDGETS_CSS}\n{CAPTURE_CSS}");
    let console_dispatch = dispatch.clone();

    view! {
        <style>{css}</style>
        <section class="dtc-capture" tabindex="0" node_ref=root_ref on:keydown=root_keydown>
            <div class="dtc-capture__body">
                <div class="dtc-capture__main">
                    <NameBoxBar name_box=name_box workspace=workspace dispatch=dispatch.clone() />
                    <div class="dtc-capture-line">
                        <span class="dtc-capture-line__sigil">"»"</span>
                        <input
                            class="dtc-capture-line__input"
                            node_ref=line_ref
                            aria-label="Capture line"
                            placeholder="Dotted.Path.Leaf = content"
                            prop:value=move || capture_text.get()
                            on:input=move |ev| capture_text.set(event_target_value(&ev))
                            on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                                // Typed characters belong to the capture line alone.
                                ev.stop_propagation();
                                match ev.key().as_str() {
                                    "Enter" => {
                                        ev.prevent_default();
                                        submit();
                                    }
                                    "Escape" => {
                                        ev.prevent_default();
                                        capture_text.set(String::new());
                                    }
                                    _ => {}
                                }
                            }
                        />
                    </div>
                    {template_chips}
                    {history_view}
                    <div class="dtc-capture-echo">
                        <div class="dtc-section-label">"Model echo"</div>
                        {echo}
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

fn template_chip_view(
    template: TemplateProjection,
    pending_template: RwSignal<Option<ArmedTemplate>>,
) -> AnyView {
    let name = template.name.clone();
    let title = template
        .preview_content
        .clone()
        .or_else(|| template.description.clone())
        .unwrap_or_default();
    let initial = template.initial.clone();
    let name_for_click = name.clone();
    view! {
        <button
            type="button"
            class="dtc-capture-template-chip"
            title=title
            on:click=move |_| {
                pending_template.set(Some(ArmedTemplate {
                    name: name_for_click.clone(),
                    initial: initial.clone(),
                }));
            }
        >
            {name}
        </button>
    }
    .into_any()
}

fn history_entry_view(entry: CaptureEntry) -> AnyView {
    let class = if entry.accepted {
        "dtc-capture-history__entry dtc-capture-history__entry--ok"
    } else {
        "dtc-capture-history__entry dtc-capture-history__entry--err"
    };
    let marker = if entry.accepted { "✓" } else { "✗" };
    view! {
        <div class=class>
            <span class="dtc-capture-history__marker">{marker}</span>
            <code class="dtc-capture-history__line">{entry.line}</code>
            {entry.message.map(|message| view! {
                <span class="dtc-capture-history__message">{message}</span>
            }.into_any())}
        </div>
    }
    .into_any()
}

fn echo_row_view(
    node: NodeView,
    is_selected: bool,
    workspace: &WorkspaceState,
    dispatch: Arc<dyn Dispatcher>,
    root_ref: NodeRef<leptos::html::Section>,
) -> AnyView {
    let calc_class = calc_state_class(node.calc_state);
    let prov = provenance_tint(&node, workspace);
    let sel_class = selection_mode_class(is_selected, false);
    let mut classes: Vec<&str> = vec!["dtc-capture-echo-row", calc_class, prov.css_class()];
    if !sel_class.is_empty() {
        classes.push(sel_class);
    }
    let class_attr = classes.join(" ");
    let indent = format!("padding-left:{}px;", 8 + node.depth as usize * 16);
    let name = node.display_name.clone();
    let value = value_text(&node.computed_value);
    let title = node.id.to_string();
    let key_attr = node.key.to_string();
    let id_for_click = node.id.clone();

    view! {
        <button
            type="button"
            class=class_attr
            style=indent
            tabindex="-1"
            // Rows never take keyboard focus: focus stays on the lens so the
            // grammar keeps working after a click.
            on:mousedown=move |ev| ev.prevent_default()
            on:click=move |_| {
                dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id_for_click.clone())));
                if let Some(section) = root_ref.get_untracked() {
                    let _ = section.focus();
                }
            }
            title=title
            data-node-key=key_attr
        >
            <span class="dtc-calc-dot"></span>
            <span class="dtc-capture-echo-row__name">{name}</span>
            <span class="dtc-capture-echo-row__value">{value}</span>
        </button>
    }
    .into_any()
}

const CAPTURE_CSS: &str = r#"
.dtc-capture {
  display: flex; flex-direction: column; height: 100%;
  background: var(--dtc-surface); color: var(--dtc-text);
  font: 13px/1.4 var(--dtc-font, system-ui, sans-serif);
  outline: none;
}
.dtc-capture__body { display: flex; flex: 1; min-height: 0; }
.dtc-capture__main {
  flex: 1; min-width: 0; overflow: auto; padding: 8px 12px;
  display: flex; flex-direction: column; gap: 8px;
}

.dtc-capture-line {
  display: flex; align-items: center; gap: 8px; padding: 8px 10px;
  background: var(--dtc-surface-subtle); border: 1px solid var(--dtc-border-muted); border-radius: 8px;
}
.dtc-capture-line__sigil { font-weight: 700; font-size: 16px; color: var(--dtc-accent); }
.dtc-capture-line__input {
  flex: 1; padding: 6px 10px; font: 16px/1.4 ui-monospace, monospace;
  border: 1px solid var(--dtc-border); border-radius: 6px;
  background: var(--dtc-surface-input); color: var(--dtc-text);
}

.dtc-capture-templates__row { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
.dtc-capture-template-chip {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 12px;
  padding: 2px 10px; cursor: pointer; color: var(--dtc-text);
}
.dtc-capture-template-chip:hover { border-color: var(--dtc-border-strong); }
.dtc-capture-templates__armed {
  display: inline-flex; align-items: center; gap: 6px; padding: 2px 8px;
  border: 1px dashed var(--dtc-accent-border, var(--dtc-accent)); border-radius: 12px;
  color: var(--dtc-text-muted);
}
.dtc-capture-templates__armed button {
  background: none; border: none; padding: 0 2px; cursor: pointer; color: var(--dtc-text-subtle);
}

.dtc-capture-history {
  display: flex; flex-direction: column; gap: 2px; max-height: 132px; overflow: auto;
}
.dtc-capture-history__entry {
  display: flex; align-items: baseline; gap: 8px; padding: 2px 6px; border-radius: 4px;
}
.dtc-capture-history__entry--ok .dtc-capture-history__marker { color: var(--dtc-success); }
.dtc-capture-history__entry--err { background: var(--dtc-surface-subtle); }
.dtc-capture-history__entry--err .dtc-capture-history__marker { color: var(--dtc-danger); }
.dtc-capture-history__line { color: var(--dtc-text-muted); }
.dtc-capture-history__message { color: var(--dtc-danger-text); font-size: 12px; }

.dtc-capture-echo { flex: 1; min-height: 0; overflow: auto; }
.dtc-capture-echo__rows { display: flex; flex-direction: column; gap: 1px; }
.dtc-capture-echo-row {
  display: flex; align-items: center; gap: 8px; width: 100%; box-sizing: border-box;
  padding: 3px 8px; text-align: left; background: none; border: none; border-radius: 5px;
  cursor: pointer; color: var(--dtc-text);
}
.dtc-capture-echo-row:hover { background: var(--dtc-surface-subtle); }
.dtc-capture-echo-row__name { font-weight: 600; }
.dtc-capture-echo-row__value {
  margin-left: auto; font-variant-numeric: tabular-nums; color: var(--dtc-text-muted);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.dtc-capture-echo__empty { color: var(--dtc-text-subtle); font-style: italic; }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::{
        IntentError, NodeContentKind, NodeValueProjection,
    };
    use std::collections::BTreeMap;

    fn node(path: &str, is_meta: bool) -> NodeView {
        NodeView {
            key: NodeKey::new(format!("k:{path}")),
            id: NodeId::new(path),
            display_name: path.rsplit('.').next().unwrap_or(path).to_string(),
            parent: path
                .rsplit_once('.')
                .map(|(parent, _)| NodeId::new(parent)),
            children: Vec::new(),
            depth: path.matches('.').count() as u32,
            content_kind: NodeContentKind::Constant,
            content_text: "1".to_string(),
            computed_value: NodeValueProjection::Number {
                raw: "1".to_string(),
                display: "1".to_string(),
            },
            scenario_override: None,
            literalized_value_input: None,
            value_epoch: None,
            calc_state: None,
            effective_format: None,
            note: None,
            attributes: BTreeMap::new(),
            binding_diagnostics: Vec::new(),
            is_meta,
            table: None,
        }
    }

    fn workspace_with(paths: &[(&str, bool)]) -> WorkspaceState {
        let mut ws = WorkspaceState::default();
        for (path, is_meta) in paths {
            let n = node(path, *is_meta);
            ws.node_order.push(n.id.clone());
            ws.key_order.push(n.key.clone());
            ws.paths_by_key.insert(n.key.clone(), n.id.clone());
            ws.nodes.insert(n.id.clone(), n.clone());
            ws.nodes_by_key.insert(n.key.clone(), n);
        }
        // Wire children + roots from parent pointers, preserving entry order.
        let ids = ws.node_order.clone();
        for id in &ids {
            let parent = ws.nodes[id].parent.clone();
            match parent {
                Some(parent_id) => {
                    if let Some(parent_node) = ws.nodes.get_mut(&parent_id) {
                        parent_node.children.push(id.clone());
                    }
                }
                None => ws.root_paths.push(id.clone()),
            }
        }
        ws
    }

    fn segs(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| (*part).to_string()).collect()
    }

    // -- parse_capture_line --------------------------------------------------

    #[test]
    fn parse_capture_line_accepts_plain_path() {
        let line = parse_capture_line("  Accounts.Q1.Sales  ").unwrap();
        assert_eq!(line.path_segments, vec!["Accounts", "Q1", "Sales"]);
        assert_eq!(line.content, None);
    }

    #[test]
    fn parse_capture_line_splits_constant_content_at_first_equals() {
        let line = parse_capture_line("Accounts.Q1.Sales = 5").unwrap();
        assert_eq!(line.path_segments, vec!["Accounts", "Q1", "Sales"]);
        assert_eq!(line.content, Some("5".to_string()));
    }

    #[test]
    fn parse_capture_line_hands_formula_content_through_verbatim() {
        // The split happens at the FIRST `=`; the leading `=` of the content
        // survives untouched so the HOST classifies it as a formula.
        let line = parse_capture_line("Margin = =Net/Sales").unwrap();
        assert_eq!(line.path_segments, vec!["Margin"]);
        assert_eq!(line.content, Some("=Net/Sales".to_string()));
    }

    #[test]
    fn parse_capture_line_rejects_junk() {
        assert_eq!(parse_capture_line(""), None);
        assert_eq!(parse_capture_line("   "), None);
        assert_eq!(parse_capture_line("= 5"), None);
        assert_eq!(parse_capture_line("A..B"), None);
        assert_eq!(parse_capture_line(".Lead"), None);
        assert_eq!(parse_capture_line("Trail. = 1"), None);
    }

    // -- plan_scaffold -------------------------------------------------------

    #[test]
    fn plan_scaffold_finds_full_existing_path() {
        let ws = workspace_with(&[("A", false), ("A.B", false)]);
        let plan = plan_scaffold(&ws, &segs(&["A", "B"]));
        assert_eq!(
            plan.existing_parent,
            Some((NodeId::new("A.B"), NodeKey::new("k:A.B")))
        );
        assert!(plan.missing.is_empty());
    }

    #[test]
    fn plan_scaffold_finds_partial_prefix() {
        let ws = workspace_with(&[("A", false), ("A.B", false)]);
        let plan = plan_scaffold(&ws, &segs(&["A", "Q", "R"]));
        assert_eq!(
            plan.existing_parent,
            Some((NodeId::new("A"), NodeKey::new("k:A")))
        );
        assert_eq!(plan.missing, vec!["Q".to_string(), "R".to_string()]);
    }

    #[test]
    fn plan_scaffold_scaffolds_from_root_when_nothing_exists() {
        let ws = workspace_with(&[("A", false)]);
        let plan = plan_scaffold(&ws, &segs(&["X", "Y"]));
        assert_eq!(plan.existing_parent, None);
        assert_eq!(plan.missing, vec!["X".to_string(), "Y".to_string()]);
    }

    // -- single AddNode payload builder --------------------------------------

    #[test]
    fn single_add_node_payload_is_exact() {
        let intent = build_single_add_node(
            Some(NodeId::new("Accounts.Q1")),
            "Sales",
            InitialNodeContentProjection::Literal {
                content: "5".to_string(),
            },
        );
        assert_eq!(
            intent,
            WorkspaceIntent::AddNode {
                parent: Some(NodeId::new("Accounts.Q1")),
                symbol: "Sales".to_string(),
                initial: InitialNodeContentProjection::Literal {
                    content: "5".to_string()
                },
                is_meta: false,
            }
        );
    }

    #[test]
    fn single_add_node_at_root_has_no_parent() {
        let intent =
            build_single_add_node(None, "Margin", InitialNodeContentProjection::Empty);
        assert_eq!(
            intent,
            WorkspaceIntent::AddNode {
                parent: None,
                symbol: "Margin".to_string(),
                initial: InitialNodeContentProjection::Empty,
                is_meta: false,
            }
        );
    }

    // -- sibling_prefill -----------------------------------------------------

    #[test]
    fn sibling_prefill_keeps_the_parent_path_open() {
        assert_eq!(
            sibling_prefill(&segs(&["Accounts", "Q1", "Sales"])),
            "Accounts.Q1."
        );
        assert_eq!(sibling_prefill(&segs(&["Sales"])), "");
        assert_eq!(sibling_prefill(&[]), "");
    }

    // -- leaf_initial --------------------------------------------------------

    #[test]
    fn leaf_initial_explicit_content_beats_armed_template() {
        assert_eq!(
            leaf_initial(
                Some("=A+B".to_string()),
                Some(InitialNodeContentProjection::TemplateBound {
                    template_id: "t".to_string()
                })
            ),
            InitialNodeContentProjection::Literal {
                content: "=A+B".to_string()
            }
        );
        assert_eq!(
            leaf_initial(
                None,
                Some(InitialNodeContentProjection::TemplateBound {
                    template_id: "t".to_string()
                })
            ),
            InitialNodeContentProjection::TemplateBound {
                template_id: "t".to_string()
            }
        );
        assert_eq!(
            leaf_initial(None, None),
            InitialNodeContentProjection::Empty
        );
    }

    // -- push_capture_history ------------------------------------------------

    #[test]
    fn push_capture_history_caps_newest_first() {
        let mut entries = Vec::new();
        for index in 0..5 {
            push_capture_history(
                &mut entries,
                CaptureEntry {
                    line: format!("L{index}"),
                    accepted: true,
                    message: None,
                },
                3,
            );
        }
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].line, "L4");
        assert_eq!(entries[2].line, "L2");
    }

    // -- echo_nodes ----------------------------------------------------------

    #[test]
    fn echo_nodes_walks_depth_first_and_prunes_effective_meta() {
        let ws = workspace_with(&[
            ("A", false),
            ("A.B", false),
            ("M", true),
            ("M.X", false), // regular flag, but effectively meta via its parent
            ("Z", false),
        ]);
        let ids: Vec<String> = echo_nodes(&ws)
            .iter()
            .map(|node| node.id.to_string())
            .collect();
        assert_eq!(ids, vec!["A", "A.B", "Z"]);
    }

    // -- receipt_error_text --------------------------------------------------

    #[test]
    fn receipt_error_text_surfaces_the_typed_display() {
        let rejected = IntentReceipt::rejected(IntentError::Unsupported);
        assert_eq!(
            receipt_error_text(&rejected),
            "intent variant not yet supported by this dispatcher"
        );
        let mut bare = IntentReceipt::accepted();
        bare.accepted = false;
        assert_eq!(receipt_error_text(&bare), "the host rejected the intent");
    }
}
