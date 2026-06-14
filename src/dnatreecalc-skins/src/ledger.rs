//! LEDGER — the ATLAS population lens.
//!
//! Ledger thinks in populations, not single nodes: a wide audit table over the
//! whole projection, the shared **cleave** predicate (filter + sort) as its
//! signature control, multi-select over stable keys, and bulk authoring that
//! lands as ONE host transaction (`EditScopedContent` / `SetNumberFormat` over
//! `AuthoringScope::Nodes`).
//!
//! Ledger is the home/writer of the suite-wide cleave: the predicate lives in
//! [`SharedSkinState::cleave`] so a later lens lands on the same slice of the
//! model. The materialized row set is recomputed per lens from the projection
//! via [`WorkspaceState::cleave_filtered_keys`] — the predicate is continuity,
//! the result is not.
//!
//! Spine conformance, same as Flow:
//! - **one grammar** — verbs resolve through `KeybindingRegistry::universal()`;
//!   bare keys never fire while typing; Enter on a focused button activates the
//!   button.
//! - **one styling** — `calc_state` is the only saturated channel
//!   (`calc_state_class` + `dtc-calc-dot`), provenance is structural
//!   (`provenance_tint`), SELECTED/EDITING is the 1-bit border cue.
//! - **one continuity** — `selection_set`, `cleave`, and the primary selection
//!   are shared view-state.
//!
//! Everything Ledger shows is read from the published projection; every change
//! goes through the closed `WorkspaceIntent` surface. It never parses formula
//! text, computes a value, or uses grid coordinates.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use dnatreecalc_skin_framework::{
    ATLAS_SPINE_CSS, AuthoringScope, CleaveFilter, CleavePredicate, CleaveSort,
    ClipboardPayloadKind, Dispatcher, IntentReceipt, KeyChord, KeybindingRegistry,
    NodeCalcStateProjection, NodeKey, NodeValueProjection, NodeView, SharedSkinStateHandle,
    SharedStateChange, SharedStateOrigin, SkinCapabilities, SkinCategory, SkinContext, SkinHandle,
    SkinId, SkinManifest, SkinMountSlot, SkinState, SkinStateHandle, SkinVerb, WorkspaceIntent,
    WorkspaceSkin, WorkspaceState, calc_state_class, provenance_tint, selection_mode_class,
};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::spine_widgets::{
    ConsoleBar, NameBoxBar, NodeInspector, SPINE_WIDGETS_CSS, clear_content_edit,
    commit_content_edit, event_target_is_interactive, event_target_is_text_entry,
    focus_edit_buffer,
};
use crate::value_render::value_text;

pub const LEDGER_ID: SkinId = SkinId::new("ledger");

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerState {
    /// Group rows into sections by calc-state class (lens-side presentation
    /// only — the underlying cleave order is untouched).
    pub group_by_health: bool,
}

impl SkinState for LedgerState {
    fn schema_version() -> u32 {
        1
    }
}

#[derive(Default)]
pub struct LedgerLens;

impl LedgerLens {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl WorkspaceSkin for LedgerLens {
    type State = LedgerState;

    fn id(&self) -> SkinId {
        LEDGER_ID
    }

    fn manifest(&self) -> SkinManifest {
        SkinManifest {
            display_name: "Ledger",
            description: "Population lens: cleave, sort, and author whole classes of nodes at once.",
            category: SkinCategory::Overview,
            version: "0.1.0",
        }
    }

    fn capabilities(&self) -> SkinCapabilities {
        SkinCapabilities {
            supports_multi_select: true,
            supports_inline_formula_edit: true,
            supports_meta_node_display: false,
            renders_arrays_inline: false,
            renders_table_values: false,
            allowed_slots: None,
        }
    }

    fn mount(&self, cx: SkinContext<Self::State>) -> SkinHandle {
        crate::spine_widgets::stamp_active_lens(cx.shared, LEDGER_ID, cx.slot);
        SkinHandle::new(view! { <LedgerView cx=cx /> }.into_any())
    }
}

// ---------------------------------------------------------------------------
// Pure helpers (unit-tested)
// ---------------------------------------------------------------------------

/// The audited row population: the shared cleave applied to the projection
/// when present (else `key_order` order), with effectively-meta keys dropped.
/// Unknown keys never survive (`cleave_filtered_keys` drops them; the
/// `node_by_key` check covers the no-cleave path).
fn ledger_rows(workspace: &WorkspaceState, cleave: Option<&CleavePredicate>) -> Vec<NodeKey> {
    let keys = match cleave {
        Some(predicate) => workspace.cleave_filtered_keys(predicate),
        None => workspace.key_order.clone(),
    };
    keys.into_iter()
        .filter(|key| workspace.node_by_key(key).is_some() && !workspace.is_effective_meta(key))
        .collect()
}

/// Bulk content authoring: one intent, one host transaction, one undo.
fn bulk_content_intent(selection: &[NodeKey], content: String) -> WorkspaceIntent {
    WorkspaceIntent::EditScopedContent {
        scope: AuthoringScope::Nodes(selection.to_vec()),
        content,
    }
}

/// Bulk format authoring (or clearing, with `None`) over the selection set.
fn bulk_format_intent(
    selection: &[NodeKey],
    number_format_code: Option<String>,
) -> WorkspaceIntent {
    WorkspaceIntent::SetNumberFormat {
        scope: AuthoringScope::Nodes(selection.to_vec()),
        number_format_code,
    }
}

/// Copy the selection's values into the host-owned clipboard carrier.
fn bulk_copy_values_intent(selection: &[NodeKey]) -> WorkspaceIntent {
    WorkspaceIntent::CopyToClipboard {
        scope: AuthoringScope::Nodes(selection.to_vec()),
        payload: ClipboardPayloadKind::Values,
    }
}

/// Map the shared cleave filter to the cleave-bar select value. Filters this
/// bar cannot author (depends-on, dependency-kind, other calc-states) show as
/// `"other"` so the bar never misreports an active cleave as "All nodes".
fn cleave_filter_choice(filter: Option<&CleaveFilter>) -> &'static str {
    match filter {
        None => "",
        Some(CleaveFilter::HasError) => "errors",
        Some(CleaveFilter::CalcState(NodeCalcStateProjection::DirtyPending)) => "stale",
        Some(CleaveFilter::TextMatch(_)) => "text",
        Some(_) => "other",
    }
}

/// Map a cleave-bar select value back to a filter. Outer `None` means "leave
/// the shared predicate unchanged" (the `"other"` placeholder); inner `None`
/// means "no filter".
fn cleave_filter_for_choice(choice: &str, text: &str) -> Option<Option<CleaveFilter>> {
    match choice {
        "" => Some(None),
        "errors" => Some(Some(CleaveFilter::HasError)),
        "stale" => Some(Some(CleaveFilter::CalcState(
            NodeCalcStateProjection::DirtyPending,
        ))),
        "text" => Some(Some(CleaveFilter::TextMatch(text.to_string()))),
        _ => None,
    }
}

fn cleave_sort_choice(sort: Option<CleaveSort>) -> &'static str {
    match sort {
        None => "",
        Some(CleaveSort::NameAsc) => "name-asc",
        Some(CleaveSort::NameDesc) => "name-desc",
        Some(CleaveSort::ValueAsc) => "value-asc",
        Some(CleaveSort::ValueDesc) => "value-desc",
        Some(CleaveSort::DepthAsc) => "depth-asc",
        Some(CleaveSort::DepthDesc) => "depth-desc",
    }
}

fn cleave_sort_for_choice(choice: &str) -> Option<CleaveSort> {
    match choice {
        "name-asc" => Some(CleaveSort::NameAsc),
        "name-desc" => Some(CleaveSort::NameDesc),
        "value-asc" => Some(CleaveSort::ValueAsc),
        "value-desc" => Some(CleaveSort::ValueDesc),
        "depth-asc" => Some(CleaveSort::DepthAsc),
        "depth-desc" => Some(CleaveSort::DepthDesc),
        _ => None,
    }
}

/// Recompose the shared predicate, normalizing the empty predicate to `None`
/// so "no cleave" has exactly one representation in shared state.
fn normalized_predicate(
    filter: Option<CleaveFilter>,
    sort: Option<CleaveSort>,
) -> Option<CleavePredicate> {
    let predicate = CleavePredicate { filter, sort };
    if predicate.is_empty() {
        None
    } else {
        Some(predicate)
    }
}

/// Group rows into sections by calc-state class, preserving row order inside
/// each group and ordering groups by first appearance. Presentation only.
fn group_rows_by_calc_class(
    workspace: &WorkspaceState,
    rows: &[NodeKey],
) -> Vec<(&'static str, Vec<NodeKey>)> {
    let mut order: Vec<&'static str> = Vec::new();
    let mut groups: BTreeMap<&'static str, Vec<NodeKey>> = BTreeMap::new();
    for key in rows {
        let class = calc_state_class(workspace.node_by_key(key).and_then(|node| node.calc_state));
        if !groups.contains_key(class) {
            order.push(class);
        }
        groups.entry(class).or_default().push(key.clone());
    }
    order
        .into_iter()
        .map(|class| {
            let keys = groups.remove(class).unwrap_or_default();
            (class, keys)
        })
        .collect()
}

/// Human label for a calc-state class group header.
fn health_group_label(class: &str) -> &'static str {
    match class {
        "dtc-calc--clean" => "Clean",
        "dtc-calc--stale" => "Stale",
        "dtc-calc--evaluating" => "Evaluating",
        "dtc-calc--ready" => "Publish-ready",
        "dtc-calc--rejected" => "Rejected",
        "dtc-calc--cycle" => "Cycle-blocked",
        _ => "Unknown",
    }
}

/// Extract the message for the dismissible rejection strip; `None` when the
/// receipt was accepted (which also clears a stale strip).
fn rejection_message(receipt: &IntentReceipt) -> Option<String> {
    if receipt.accepted {
        None
    } else {
        Some(receipt.error.as_ref().map_or_else(
            || "the host rejected the intent".to_string(),
            ToString::to_string,
        ))
    }
}

// ---------------------------------------------------------------------------
// View
// ---------------------------------------------------------------------------

#[component]
fn LedgerView(cx: SkinContext<LedgerState>) -> impl IntoView {
    let workspace = cx.workspace;
    let selection = cx.selection;
    let shared = cx.shared;
    let dispatch = cx.dispatch.clone();
    let state = cx.state.clone();
    let shared_origin = SharedStateOrigin::lens(LEDGER_ID, cx.slot);

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
        } else {
            rejection.set(rejection_message(&receipt));
        }
    };

    // Lens-local grammar, resolved through the one registry. Global verbs
    // (F9, Ctrl+Z/Y, Ctrl+N, lens switch, arrows) bubble to the shell.
    let grammar = KeybindingRegistry::universal();
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

    // Land keyboard focus on the ledger when the lens mounts, so the grammar
    // works without a preparatory click.
    Effect::new(move |_| {
        if let Some(section) = root_ref.get() {
            let _ = section.focus();
        }
    });

    let shared_signal = shared.signal();
    let table_state = state.clone();
    let table_dispatch = dispatch.clone();
    let table_origin = shared_origin.clone();
    let table = move || {
        let ws = workspace.get();
        let shared_now = shared_signal.get();
        let group_by_health = table_state.with(|s| s.group_by_health);
        let sel_id = selection.with(|s| s.primary.clone());
        let rows = ledger_rows(&ws, shared_now.cleave.as_ref());
        let checked: BTreeSet<NodeKey> = shared_now.selection_set.iter().cloned().collect();

        let empty_notice = rows.is_empty().then(|| {
            view! {
                <tbody>
                    <tr class="dtc-ledger-empty">
                        <td colspan="9">"No nodes match the cleave."</td>
                    </tr>
                </tbody>
            }
            .into_any()
        });

        let sections: Vec<(Option<String>, Vec<NodeKey>)> = if group_by_health {
            group_rows_by_calc_class(&ws, &rows)
                .into_iter()
                .map(|(class, keys)| {
                    let label = format!("{} ({})", health_group_label(class), keys.len());
                    (Some(label), keys)
                })
                .collect()
        } else {
            vec![(None, rows)]
        };

        let bodies = sections
            .into_iter()
            .map(|(label, keys)| {
                let row_views = keys
                    .iter()
                    .filter_map(|key| {
                        let node = ws.node_by_key(key)?.clone();
                        let out_deps = ws
                            .dependencies
                            .edges_by_owner_key
                            .get(key)
                            .map_or(0, Vec::len);
                        let in_deps = ws
                            .dependencies
                            .reverse_edges_by_key
                            .get(key)
                            .map_or(0, Vec::len);
                        let prov_class = provenance_tint(&node, &ws).css_class();
                        let is_selected = sel_id.as_ref() == Some(&node.id);
                        Some(ledger_row_view(
                            node,
                            out_deps,
                            in_deps,
                            is_selected,
                            checked.contains(key),
                            prov_class,
                            shared,
                            &table_origin,
                            table_dispatch.clone(),
                            rejection,
                        ))
                    })
                    .collect::<Vec<_>>();
                let header = label.map(|text| {
                    view! {
                        <tr class="dtc-ledger-group-row">
                            <td colspan="9">{text}</td>
                        </tr>
                    }
                    .into_any()
                });
                view! {
                    <tbody>
                        {header}
                        {row_views}
                    </tbody>
                }
            })
            .collect::<Vec<_>>();

        view! {
            <table class="dtc-ledger-table" aria-label="Ledger population table">
                <thead>
                    <tr>
                        <th class="dtc-ledger-th--check"></th>
                        <th>"Path"</th>
                        <th>"Kind"</th>
                        <th>"Content"</th>
                        <th>"Value"</th>
                        <th class="dtc-ledger-th--dot"></th>
                        <th>"Format"</th>
                        <th>"Deps"</th>
                        <th>"Note"</th>
                    </tr>
                </thead>
                {empty_notice}
                {bodies}
            </table>
        }
    };

    let css = format!("{ATLAS_SPINE_CSS}\n{SPINE_WIDGETS_CSS}\n{LEDGER_CSS}");
    let console_dispatch = dispatch.clone();

    view! {
        <style>{css}</style>
        <section class="dtc-ledger" tabindex="0" node_ref=root_ref on:keydown=root_keydown>
            <div class="dtc-ledger__body">
                <div class="dtc-ledger__main">
                    <NameBoxBar name_box=name_box workspace=workspace dispatch=dispatch.clone() />
                    <CleaveBar shared=shared state=state.clone() origin=shared_origin.clone() />
                    <BulkBar
                        shared=shared
                        workspace=workspace
                        dispatch=dispatch.clone()
                        rejection=rejection
                        origin=shared_origin.clone()
                    />
                    {move || rejection.get().map(|message| view! {
                        <div class="dtc-ledger-reject" role="alert">
                            <span class="dtc-ledger-reject__message">{message}</span>
                            <button type="button" on:click=move |_| rejection.set(None)>
                                "Dismiss"
                            </button>
                        </div>
                    }.into_any())}
                    <div class="dtc-ledger__scroll">{table}</div>
                </div>
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
            <ConsoleBar workspace=workspace dispatch=console_dispatch shared=shared />
        </section>
    }
}

/// One audit row. Plain fn (not a component) so the table closure stays
/// shallow; all reactive inputs are resolved by the caller.
#[allow(clippy::too_many_arguments)]
fn ledger_row_view(
    node: NodeView,
    out_deps: usize,
    in_deps: usize,
    is_selected: bool,
    in_selection_set: bool,
    prov_class: &'static str,
    shared: SharedSkinStateHandle,
    origin: &SharedStateOrigin,
    dispatch: Arc<dyn Dispatcher>,
    rejection: RwSignal<Option<String>>,
) -> AnyView {
    let calc_class = calc_state_class(node.calc_state);
    let sel_class = selection_mode_class(is_selected, false);
    let row_class = if sel_class.is_empty() {
        "dtc-ledger-row".to_string()
    } else {
        format!("dtc-ledger-row {sel_class}")
    };
    let row_key = node.key.to_string();
    let dot_class = format!("dtc-calc-dot {calc_class}");
    let calc_title = node
        .calc_state
        .map_or_else(|| "unknown".to_string(), |calc| calc.to_string());
    let value = value_text(&node.computed_value);
    let is_error = matches!(node.computed_value, NodeValueProjection::Error(_));
    let value_class = if is_error {
        format!("dtc-ledger-cell dtc-ledger-cell--value dtc-value-display--error {prov_class}")
    } else {
        format!("dtc-ledger-cell dtc-ledger-cell--value {prov_class}")
    };
    let format_code = node
        .effective_format
        .as_ref()
        .and_then(|format| format.number_format_code.clone())
        .unwrap_or_else(|| "General".to_string());
    let deps = format!("{out_deps} out / {in_deps} in");
    let note_title = node
        .note
        .as_ref()
        .map(|note| note.text.clone())
        .unwrap_or_default();
    let note_marker = node.note.is_some().then_some("●");
    let kind = node.content_kind.to_string();
    let path = node.id.as_str().to_string();
    let checkbox_label = format!("Include {path} in the bulk selection");
    let content_label = format!("Content for {path}");
    let content = node.content_text.clone();

    let key_for_toggle = node.key.clone();
    let toggle_dispatch = dispatch.clone();
    let id_for_click = node.id.clone();
    let id_for_edit = node.id.clone();
    let edit_dispatch = dispatch.clone();
    let edit_origin = origin.clone();

    view! {
        <tr
            class=row_class
            data-node-key=row_key
            on:click=move |_| {
                // The audited primary selection: row click always routes
                // through the dispatcher, independent of the checkbox set.
                dispatch.dispatch(WorkspaceIntent::SelectNode(Some(id_for_click.clone())));
            }
        >
            <td class="dtc-ledger-cell dtc-ledger-cell--check">
                <input
                    type="checkbox"
                    aria-label=checkbox_label
                    prop:checked=in_selection_set
                    on:click=|ev| ev.stop_propagation()
                    on:keydown=|ev: leptos::ev::KeyboardEvent| ev.stop_propagation()
                    on:change=move |_| {
                        // Population selection is a dispatched, audited intent
                        // (B.3.4); the lens computes the resulting set and the
                        // host validates + mirrors it into shared state.
                        let mut keys = shared.get_untracked().selection_set;
                        if let Some(index) =
                            keys.iter().position(|key| key == &key_for_toggle)
                        {
                            keys.remove(index);
                        } else {
                            keys.push(key_for_toggle.clone());
                        }
                        toggle_dispatch.dispatch(WorkspaceIntent::SelectNodes {
                            keys,
                            anchor: Some(key_for_toggle.clone()),
                        });
                    }
                />
            </td>
            <td class="dtc-ledger-cell dtc-ledger-cell--path"><code>{path}</code></td>
            <td class="dtc-ledger-cell dtc-ledger-cell--kind">{kind}</td>
            <td class="dtc-ledger-cell dtc-ledger-cell--content">
                <input
                    class="dtc-ledger-content-input"
                    aria-label=content_label
                    prop:value=content
                    on:click=|ev| ev.stop_propagation()
                    on:keydown=|ev: leptos::ev::KeyboardEvent| ev.stop_propagation()
                    on:change=move |ev| {
                        // The one recalc-mode-aware commit path.
                        let receipt = commit_content_edit(
                            &edit_dispatch,
                            shared,
                            &edit_origin,
                            id_for_edit.clone(),
                            event_target_value(&ev),
                        );
                        rejection.set(rejection_message(&receipt));
                    }
                />
            </td>
            <td class=value_class>{value}</td>
            <td class="dtc-ledger-cell dtc-ledger-cell--dot">
                <span class=dot_class title=calc_title></span>
            </td>
            <td class="dtc-ledger-cell dtc-ledger-cell--format">{format_code}</td>
            <td class="dtc-ledger-cell dtc-ledger-cell--deps">{deps}</td>
            <td class="dtc-ledger-cell dtc-ledger-cell--note" title=note_title>{note_marker}</td>
        </tr>
    }
    .into_any()
}

/// The cleave bar — Ledger's signature control and the writer of the shared
/// predicate. Reads AND writes `SharedSkinState.cleave` (writes go through the
/// audited `apply` chokepoint), so the same slice follows the user into every
/// other lens.
#[component]
fn CleaveBar(
    shared: SharedSkinStateHandle,
    state: SkinStateHandle<LedgerState>,
    origin: SharedStateOrigin,
) -> impl IntoView {
    let shared_signal = shared.signal();
    let filter_choice = Memo::new(move |_| {
        shared_signal.with(|shared_state| {
            cleave_filter_choice(
                shared_state
                    .cleave
                    .as_ref()
                    .and_then(|cleave| cleave.filter.as_ref()),
            )
            .to_string()
        })
    });
    let sort_choice = Memo::new(move |_| {
        shared_signal.with(|shared_state| {
            cleave_sort_choice(
                shared_state
                    .cleave
                    .as_ref()
                    .and_then(|cleave| cleave.sort),
            )
            .to_string()
        })
    });
    let text_value = Memo::new(move |_| {
        shared_signal.with(|shared_state| {
            match shared_state
                .cleave
                .as_ref()
                .and_then(|cleave| cleave.filter.as_ref())
            {
                Some(CleaveFilter::TextMatch(text)) => text.clone(),
                _ => String::new(),
            }
        })
    });
    let group_on = {
        let state = state.clone();
        Memo::new(move |_| state.with(|s| s.group_by_health))
    };
    let toggle_state = state;
    // One owned origin per move closure — never share the original.
    let filter_origin = origin.clone();
    let text_origin = origin.clone();
    let sort_origin = origin.clone();
    let clear_origin = origin;

    view! {
        <div class="dtc-ledger-cleave" aria-label="Cleave (shared filter and sort)">
            <span class="dtc-ledger-cleave__label">"cleave"</span>
            <select
                aria-label="Cleave filter"
                prop:value=move || filter_choice.get()
                on:change=move |ev| {
                    let choice = event_target_value(&ev);
                    let current = shared.get_untracked().cleave.unwrap_or_default();
                    let text = match &current.filter {
                        Some(CleaveFilter::TextMatch(text)) => text.clone(),
                        _ => String::new(),
                    };
                    if let Some(filter) = cleave_filter_for_choice(&choice, &text) {
                        let cleave = normalized_predicate(filter, current.sort);
                        shared.apply(SharedStateChange::SetCleave(cleave), filter_origin.clone());
                    }
                }
            >
                <option value="">"All nodes"</option>
                <option value="errors">"Errors"</option>
                <option value="stale">"Stale"</option>
                <option value="text">"Text…"</option>
                <option value="other">"(custom)"</option>
            </select>
            {move || (filter_choice.get() == "text").then(|| {
                let text_origin = text_origin.clone();
                view! {
                    <input
                        class="dtc-ledger-cleave__text"
                        placeholder="match text…"
                        aria-label="Cleave text filter"
                        prop:value=move || text_value.get()
                        // Typed characters belong to this input alone.
                        on:keydown=|ev: leptos::ev::KeyboardEvent| ev.stop_propagation()
                        on:change=move |ev| {
                            let text = event_target_value(&ev);
                            let sort = shared
                                .get_untracked()
                                .cleave
                                .and_then(|cleave| cleave.sort);
                            let cleave =
                                normalized_predicate(Some(CleaveFilter::TextMatch(text)), sort);
                            shared.apply(
                                SharedStateChange::SetCleave(cleave),
                                text_origin.clone(),
                            );
                        }
                    />
                }
                .into_any()
            })}
            <select
                aria-label="Cleave sort"
                prop:value=move || sort_choice.get()
                on:change=move |ev| {
                    let choice = event_target_value(&ev);
                    let filter = shared
                        .get_untracked()
                        .cleave
                        .and_then(|cleave| cleave.filter);
                    let cleave = normalized_predicate(filter, cleave_sort_for_choice(&choice));
                    shared.apply(SharedStateChange::SetCleave(cleave), sort_origin.clone());
                }
            >
                <option value="">"Model order"</option>
                <option value="name-asc">"Name ↑"</option>
                <option value="name-desc">"Name ↓"</option>
                <option value="value-asc">"Value ↑"</option>
                <option value="value-desc">"Value ↓"</option>
                <option value="depth-asc">"Depth ↑"</option>
                <option value="depth-desc">"Depth ↓"</option>
            </select>
            <button
                type="button"
                on:click=move |_| {
                    shared.apply(SharedStateChange::SetCleave(None), clear_origin.clone());
                }
            >
                "Clear cleave"
            </button>
            <span class="dtc-ledger-cleave__spacer"></span>
            <button
                type="button"
                class=move || if group_on.get() {
                    "dtc-ledger-cleave__toggle dtc-ledger-cleave__toggle--on".to_string()
                } else {
                    "dtc-ledger-cleave__toggle".to_string()
                }
                on:click=move |_| toggle_state.update(|s| s.group_by_health = !s.group_by_health)
            >
                "Group by health"
            </button>
        </div>
    }
}

/// Selection controls plus the bulk authoring strip. The authoring controls
/// appear only while the shared selection set is non-empty; every Apply is
/// ONE dispatch over `AuthoringScope::Nodes` — one host transaction, one undo.
#[component]
fn BulkBar(
    shared: SharedSkinStateHandle,
    workspace: ReadSignal<WorkspaceState>,
    dispatch: Arc<dyn Dispatcher>,
    rejection: RwSignal<Option<String>>,
    origin: SharedStateOrigin,
) -> impl IntoView {
    let shared_signal = shared.signal();
    // One owned origin per move closure — never share the original.
    let select_all_dispatch = dispatch.clone();
    let clear_selection_dispatch = dispatch.clone();
    let _ = origin;
    let selected_count =
        Memo::new(move |_| shared_signal.with(|shared_state| shared_state.selection_set.len()));
    let bulk_content = RwSignal::new(String::new());
    let bulk_format = RwSignal::new(String::new());

    let content_dispatch = dispatch.clone();
    let format_dispatch = dispatch.clone();
    let clear_format_dispatch = dispatch.clone();
    let copy_dispatch = dispatch.clone();

    let authoring_class = move || {
        if selected_count.get() > 0 {
            "dtc-ledger-bulk__authoring".to_string()
        } else {
            "dtc-ledger-bulk__authoring dtc-ledger-bulk__authoring--hidden".to_string()
        }
    };

    view! {
        <div class="dtc-ledger-bulk" aria-label="Population selection and bulk authoring">
            <span class="dtc-ledger-bulk__count">
                {move || format!("{} selected", selected_count.get())}
            </span>
            <button
                type="button"
                on:click=move |_| {
                    let rows = ledger_rows(
                        &workspace.get_untracked(),
                        shared.get_untracked().cleave.as_ref(),
                    );
                    select_all_dispatch.dispatch(WorkspaceIntent::SelectNodes {
                        keys: rows,
                        anchor: None,
                    });
                }
            >
                "Select all visible"
            </button>
            <button
                type="button"
                on:click=move |_| {
                    clear_selection_dispatch.dispatch(WorkspaceIntent::SelectNodes {
                        keys: Vec::new(),
                        anchor: None,
                    });
                }
            >
                "Clear"
            </button>
            <span class=authoring_class>
                <input
                    class="dtc-ledger-bulk__input"
                    placeholder="content for selection…"
                    aria-label="Bulk content"
                    prop:value=move || bulk_content.get()
                    on:input=move |ev| bulk_content.set(event_target_value(&ev))
                    on:keydown=|ev: leptos::ev::KeyboardEvent| ev.stop_propagation()
                />
                <button
                    type="button"
                    title="Author this content onto every selected node as one transaction"
                    on:click=move |_| {
                        let keys = shared.get_untracked().selection_set;
                        if keys.is_empty() {
                            return;
                        }
                        let receipt = content_dispatch
                            .dispatch(bulk_content_intent(&keys, bulk_content.get_untracked()));
                        rejection.set(rejection_message(&receipt));
                    }
                >
                    "Set content"
                </button>
                <input
                    class="dtc-ledger-bulk__input dtc-ledger-bulk__input--format"
                    placeholder="format code…"
                    aria-label="Bulk number format code"
                    prop:value=move || bulk_format.get()
                    on:input=move |ev| bulk_format.set(event_target_value(&ev))
                    on:keydown=|ev: leptos::ev::KeyboardEvent| ev.stop_propagation()
                />
                <button
                    type="button"
                    title="Author this number format over the selection"
                    on:click=move |_| {
                        let keys = shared.get_untracked().selection_set;
                        if keys.is_empty() {
                            return;
                        }
                        let code = bulk_format.get_untracked().trim().to_string();
                        if code.is_empty() {
                            return;
                        }
                        let receipt = format_dispatch
                            .dispatch(bulk_format_intent(&keys, Some(code)));
                        rejection.set(rejection_message(&receipt));
                    }
                >
                    "Set format"
                </button>
                <button
                    type="button"
                    title="Clear the authored number format over the selection"
                    on:click=move |_| {
                        let keys = shared.get_untracked().selection_set;
                        if keys.is_empty() {
                            return;
                        }
                        let receipt = clear_format_dispatch
                            .dispatch(bulk_format_intent(&keys, None));
                        rejection.set(rejection_message(&receipt));
                    }
                >
                    "Clear format"
                </button>
                <button
                    type="button"
                    title="Copy the selection's values into the host clipboard"
                    on:click=move |_| {
                        let keys = shared.get_untracked().selection_set;
                        if keys.is_empty() {
                            return;
                        }
                        let receipt = copy_dispatch.dispatch(bulk_copy_values_intent(&keys));
                        rejection.set(rejection_message(&receipt));
                    }
                >
                    "Copy values"
                </button>
            </span>
        </div>
    }
}

const LEDGER_CSS: &str = r#"
.dtc-ledger {
  display: flex; flex-direction: column; height: 100%;
  background: var(--dtc-surface); color: var(--dtc-text);
  font: 13px/1.4 var(--dtc-font, system-ui, sans-serif);
  outline: none;
}
.dtc-ledger__body { display: flex; flex: 1; min-height: 0; }
.dtc-ledger__main { display: flex; flex-direction: column; flex: 1; min-width: 0; padding: 8px 12px; }
.dtc-ledger__scroll { flex: 1; overflow: auto; }

.dtc-ledger-cleave {
  display: flex; align-items: center; flex-wrap: wrap; gap: 8px; padding: 6px 8px; margin-bottom: 8px;
  background: var(--dtc-surface-subtle); border: 1px solid var(--dtc-border-muted); border-radius: 6px;
}
.dtc-ledger-cleave__label { color: var(--dtc-text-subtle); text-transform: uppercase; font-size: 11px; letter-spacing: 0.04em; }
.dtc-ledger-cleave select, .dtc-ledger-cleave input {
  padding: 3px 6px; border: 1px solid var(--dtc-border); border-radius: 5px;
  background: var(--dtc-surface-input); color: var(--dtc-text);
}
.dtc-ledger-cleave button {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 2px 8px; cursor: pointer; color: var(--dtc-text);
}
.dtc-ledger-cleave__spacer { flex: 1; }
.dtc-ledger-cleave__toggle--on { border-color: var(--dtc-accent-border); box-shadow: inset 0 0 0 1px var(--dtc-accent-border); }

.dtc-ledger-bulk {
  display: flex; align-items: center; flex-wrap: wrap; gap: 8px; padding: 6px 8px; margin-bottom: 8px;
  background: var(--dtc-surface-subtle); border: 1px solid var(--dtc-border-muted); border-radius: 6px;
}
.dtc-ledger-bulk__count { font-variant-numeric: tabular-nums; color: var(--dtc-text-muted); }
.dtc-ledger-bulk button {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 2px 8px; cursor: pointer; color: var(--dtc-text);
}
.dtc-ledger-bulk__authoring { display: inline-flex; align-items: center; flex-wrap: wrap; gap: 8px; }
.dtc-ledger-bulk__authoring--hidden { display: none; }
.dtc-ledger-bulk__input {
  padding: 3px 6px; border: 1px solid var(--dtc-border); border-radius: 5px;
  background: var(--dtc-surface-input); color: var(--dtc-text);
  font-family: ui-monospace, monospace;
}
.dtc-ledger-bulk__input--format { width: 110px; }

.dtc-ledger-reject {
  display: flex; align-items: center; gap: 8px; padding: 6px 8px; margin-bottom: 8px;
  border: 1px solid var(--dtc-danger); border-radius: 6px;
  background: var(--dtc-surface-subtle); color: var(--dtc-danger-text);
}
.dtc-ledger-reject__message { flex: 1; }
.dtc-ledger-reject button {
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border); border-radius: 5px;
  padding: 2px 8px; cursor: pointer; color: var(--dtc-text);
}

.dtc-ledger-table { width: 100%; border-collapse: collapse; }
.dtc-ledger-table th {
  position: sticky; top: 0; background: var(--dtc-surface-subtle); text-align: left;
  padding: 4px 8px; border-bottom: 1px solid var(--dtc-border);
  font-size: 11px; text-transform: uppercase; letter-spacing: 0.04em; color: var(--dtc-text-subtle);
}
.dtc-ledger-cell { padding: 3px 8px; border-bottom: 1px solid var(--dtc-border-muted); }
.dtc-ledger-row { cursor: pointer; }
.dtc-ledger-row:hover { background: var(--dtc-surface-panel); }
.dtc-ledger-row.dtc-sel--selected { background: var(--dtc-surface-panel); }
.dtc-ledger-group-row td {
  padding: 6px 8px; font-weight: 600; color: var(--dtc-text-muted);
  background: var(--dtc-surface-subtle); border-bottom: 1px solid var(--dtc-border);
}
.dtc-ledger-empty td { padding: 12px 8px; color: var(--dtc-text-subtle); font-style: italic; }
.dtc-ledger-cell--value { font-variant-numeric: tabular-nums; white-space: nowrap; }
.dtc-ledger-cell--path code { color: var(--dtc-text-muted); font-size: 12px; }
.dtc-ledger-cell--kind, .dtc-ledger-cell--format, .dtc-ledger-cell--deps { color: var(--dtc-text-muted); white-space: nowrap; }
.dtc-ledger-cell--note { text-align: center; color: var(--dtc-accent); }
.dtc-ledger-content-input {
  width: 100%; box-sizing: border-box; padding: 2px 6px;
  border: 1px solid var(--dtc-border); border-radius: 4px;
  background: var(--dtc-surface-input); color: var(--dtc-text);
  font-family: ui-monospace, monospace;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnatreecalc_skin_framework::{IntentError, NodeContentKind, NodeId, SharedSkinState};

    fn node(key: &str, path: &str) -> NodeView {
        NodeView {
            key: NodeKey::new(key),
            id: NodeId::new(path),
            display_name: path.rsplit('.').next().unwrap_or(path).to_string(),
            parent: None,
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

    fn number_node(key: &str, path: &str, raw: &str) -> NodeView {
        let mut n = node(key, path);
        n.computed_value = NodeValueProjection::Number {
            raw: raw.to_string(),
            display: raw.to_string(),
        };
        n
    }

    fn error_node(key: &str, path: &str) -> NodeView {
        let mut n = node(key, path);
        n.computed_value = NodeValueProjection::Error("#REF".to_string());
        n
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

    #[test]
    fn ledger_rows_without_cleave_keeps_key_order_and_drops_effective_meta() {
        let mut meta = node("k:m", "Meta");
        meta.is_meta = true;
        // Regular-flagged child of a meta parent: effectively meta, must drop.
        let mut meta_child = node("k:mc", "Meta.Child");
        meta_child.parent = Some(NodeId::new("Meta"));
        let ws = workspace_with(vec![
            node("k:a", "A"),
            meta,
            meta_child,
            node("k:b", "B"),
        ]);

        assert_eq!(
            ledger_rows(&ws, None),
            vec![NodeKey::new("k:a"), NodeKey::new("k:b")]
        );
    }

    #[test]
    fn ledger_rows_applies_cleave_filter_and_still_drops_meta() {
        let mut meta_error = error_node("k:m", "Meta");
        meta_error.is_meta = true;
        let ws = workspace_with(vec![
            number_node("k:a", "A", "5"),
            error_node("k:b", "B"),
            meta_error,
        ]);
        let predicate = CleavePredicate {
            filter: Some(CleaveFilter::HasError),
            sort: None,
        };

        // The meta node also has an error, but effective-meta wins.
        assert_eq!(
            ledger_rows(&ws, Some(&predicate)),
            vec![NodeKey::new("k:b")]
        );
    }

    #[test]
    fn ledger_rows_applies_cleave_sort() {
        let ws = workspace_with(vec![
            number_node("k:a", "A", "5"),
            error_node("k:b", "B"),
            number_node("k:c", "C", "2"),
        ]);
        let predicate = CleavePredicate {
            filter: None,
            sort: Some(CleaveSort::ValueDesc),
        };

        // Numbers descending first, non-numbers after.
        assert_eq!(
            ledger_rows(&ws, Some(&predicate)),
            vec![
                NodeKey::new("k:a"),
                NodeKey::new("k:c"),
                NodeKey::new("k:b")
            ]
        );
    }

    #[test]
    fn bulk_content_intent_builds_one_scoped_edit() {
        let keys = vec![NodeKey::new("k:a"), NodeKey::new("k:b")];
        assert_eq!(
            bulk_content_intent(&keys, "=Base*1.1".to_string()),
            WorkspaceIntent::EditScopedContent {
                scope: AuthoringScope::Nodes(vec![NodeKey::new("k:a"), NodeKey::new("k:b")]),
                content: "=Base*1.1".to_string(),
            }
        );
    }

    #[test]
    fn bulk_format_intent_sets_and_clears_codes() {
        let keys = vec![NodeKey::new("k:a")];
        assert_eq!(
            bulk_format_intent(&keys, Some("0.00".to_string())),
            WorkspaceIntent::SetNumberFormat {
                scope: AuthoringScope::Nodes(vec![NodeKey::new("k:a")]),
                number_format_code: Some("0.00".to_string()),
            }
        );
        assert_eq!(
            bulk_format_intent(&keys, None),
            WorkspaceIntent::SetNumberFormat {
                scope: AuthoringScope::Nodes(vec![NodeKey::new("k:a")]),
                number_format_code: None,
            }
        );
    }

    #[test]
    fn bulk_copy_values_intent_targets_the_selection() {
        let keys = vec![NodeKey::new("k:a"), NodeKey::new("k:b")];
        assert_eq!(
            bulk_copy_values_intent(&keys),
            WorkspaceIntent::CopyToClipboard {
                scope: AuthoringScope::Nodes(vec![NodeKey::new("k:a"), NodeKey::new("k:b")]),
                payload: ClipboardPayloadKind::Values,
            }
        );
    }

    #[test]
    fn row_checkbox_toggle_is_typed_and_audited() {
        // The row checkbox now routes through the audited chokepoint as
        // `ToggleSelection`: add on first toggle, remove on the second,
        // preserving insertion order for the keys that stay.
        let shared = SharedSkinStateHandle::new(SharedSkinState::default());
        let origin = SharedStateOrigin::lens(SkinId::new("test"), SkinMountSlot::Main);

        shared.apply(
            SharedStateChange::ToggleSelection(NodeKey::new("k:a")),
            origin.clone(),
        );
        shared.apply(
            SharedStateChange::ToggleSelection(NodeKey::new("k:b")),
            origin.clone(),
        );
        assert_eq!(
            shared.get_untracked().selection_set,
            vec![NodeKey::new("k:a"), NodeKey::new("k:b")]
        );

        shared.apply(
            SharedStateChange::ToggleSelection(NodeKey::new("k:a")),
            origin.clone(),
        );
        assert_eq!(
            shared.get_untracked().selection_set,
            vec![NodeKey::new("k:b")]
        );

        // Every toggle landed in the audit ring, attributed to the lens.
        let audit = shared.audit_log();
        assert_eq!(audit.len(), 3);
        assert!(audit.iter().all(|record| record.origin == origin));
    }

    #[test]
    fn cleave_filter_choice_round_trips_authorable_filters() {
        assert_eq!(cleave_filter_choice(None), "");
        assert_eq!(cleave_filter_choice(Some(&CleaveFilter::HasError)), "errors");
        assert_eq!(
            cleave_filter_choice(Some(&CleaveFilter::CalcState(
                NodeCalcStateProjection::DirtyPending
            ))),
            "stale"
        );
        assert_eq!(
            cleave_filter_choice(Some(&CleaveFilter::TextMatch("rev".to_string()))),
            "text"
        );
        // Filters the bar cannot author display as the inert custom slot.
        assert_eq!(
            cleave_filter_choice(Some(&CleaveFilter::DependsOn(NodeKey::new("k:a")))),
            "other"
        );

        assert_eq!(cleave_filter_for_choice("", "ignored"), Some(None));
        assert_eq!(
            cleave_filter_for_choice("errors", ""),
            Some(Some(CleaveFilter::HasError))
        );
        assert_eq!(
            cleave_filter_for_choice("stale", ""),
            Some(Some(CleaveFilter::CalcState(
                NodeCalcStateProjection::DirtyPending
            )))
        );
        assert_eq!(
            cleave_filter_for_choice("text", "rev"),
            Some(Some(CleaveFilter::TextMatch("rev".to_string())))
        );
        // Custom/unknown choices leave the shared predicate untouched.
        assert_eq!(cleave_filter_for_choice("other", ""), None);
    }

    #[test]
    fn cleave_sort_choice_round_trips_all_variants() {
        let variants = [
            CleaveSort::NameAsc,
            CleaveSort::NameDesc,
            CleaveSort::ValueAsc,
            CleaveSort::ValueDesc,
            CleaveSort::DepthAsc,
            CleaveSort::DepthDesc,
        ];
        for sort in variants {
            let choice = cleave_sort_choice(Some(sort));
            assert_eq!(cleave_sort_for_choice(choice), Some(sort));
        }
        assert_eq!(cleave_sort_choice(None), "");
        assert_eq!(cleave_sort_for_choice(""), None);
        assert_eq!(cleave_sort_for_choice("bogus"), None);
    }

    #[test]
    fn normalized_predicate_collapses_empty_to_none() {
        assert_eq!(normalized_predicate(None, None), None);
        assert_eq!(
            normalized_predicate(Some(CleaveFilter::HasError), None),
            Some(CleavePredicate {
                filter: Some(CleaveFilter::HasError),
                sort: None,
            })
        );
        assert_eq!(
            normalized_predicate(None, Some(CleaveSort::NameAsc)),
            Some(CleavePredicate {
                filter: None,
                sort: Some(CleaveSort::NameAsc),
            })
        );
    }

    #[test]
    fn group_rows_by_calc_class_orders_groups_by_first_appearance() {
        let mut a = node("k:a", "A");
        a.calc_state = Some(NodeCalcStateProjection::Clean);
        let mut b = node("k:b", "B");
        b.calc_state = Some(NodeCalcStateProjection::DirtyPending);
        let mut c = node("k:c", "C");
        c.calc_state = Some(NodeCalcStateProjection::Clean);
        let ws = workspace_with(vec![a, b, c]);
        let rows = ledger_rows(&ws, None);

        let groups = group_rows_by_calc_class(&ws, &rows);
        assert_eq!(
            groups,
            vec![
                (
                    "dtc-calc--clean",
                    vec![NodeKey::new("k:a"), NodeKey::new("k:c")]
                ),
                ("dtc-calc--stale", vec![NodeKey::new("k:b")]),
            ]
        );
        assert_eq!(health_group_label("dtc-calc--clean"), "Clean");
        assert_eq!(health_group_label("dtc-calc--stale"), "Stale");
        assert_eq!(health_group_label("dtc-calc--whatever"), "Unknown");
    }

    #[test]
    fn rejection_message_reads_the_receipt() {
        assert_eq!(rejection_message(&IntentReceipt::accepted()), None);
        assert_eq!(
            rejection_message(&IntentReceipt::rejected(IntentError::Unsupported)),
            Some("intent variant not yet supported by this dispatcher".to_string())
        );
    }
}
