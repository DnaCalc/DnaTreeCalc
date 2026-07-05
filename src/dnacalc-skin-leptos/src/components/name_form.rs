//! `+ name` creation form + rename inline + delete-with-name confirm (N3,
//! route-map §B.3's Name loop, §B.7's `_names` backing-cell convention).
//!
//! Three pieces, imitating N2's `cell_entry.rs` shape (pure classification fn
//! + shared component + shared CSS):
//!
//! - [`NameForm`]: the `+ name` two-field inline form (name, body). On
//!   commit it dispatches **two** intents in order — `EnterGridCell` at the
//!   allocated `_names` backing cell, then `SetDefinedName{Static}` over it
//!   (§B.3's Decision paragraph: two dispatched intents, two undo steps,
//!   accepted v1) — or `SetDefinedName{Dynamic}` alone when the "advanced:
//!   dynamic" flag is set (no backing cell for a dynamic name).
//! - [`classify_name_commit`] / [`NameCommitResult`]: the duplicate-name
//!   pre-check (see below) plus the receipt classification for the two-step
//!   creation flow.
//! - [`RenameNameForm`]: rename-inline, dispatching **only**
//!   `RenameDefinedName` (acceptance 3 — no re-allocation, no `SetDefinedName`
//!   involved in a rename).
//!
//! **Duplicate-name rejection is a host-core-honest CLIENT-SIDE pre-check**
//! (ENGINE-HONESTY, this bead's process note): ground-truthed against
//! `dnacalc-host-core/src/defined_names.rs`'s own doc comment and its `present_defined_name_rejection`
//! map — the engine's *only* typed duplicate-name rejection is
//! `DefinedNameCollidesWithTreeNode` (a workbook-scope name colliding with a
//! root **tree** node's symbol, D2 §4.3 rule 4 / V8). Plain redefinition of an
//! existing name at the same scope (the ordinary "I typed a name that's
//! already taken" case a `+ name` form must catch) is the engine's own
//! **last-write-wins** semantics — NOT a rejection engine-side. So catching it
//! *before* dispatch, by checking the name against the workspace's own
//! [`DefinedNamesProjection`] (already mirrored client-side, no extra round
//! trip), is the only way to give the honest "duplicate-name rejection renders
//! inline, form stays open" behavior the bead's acceptance (2) asks for — a
//! real, typed rejection (reusing [`IntentError::DefinedNameCollision`], the
//! same shape the engine's own tree-node-collision path produces) rather than
//! a fabricated engine behavior. §A.4's table lists `DefinedNameCollidesWithTreeNode`
//! as "names-manager inline validation" — this pre-check produces the
//! identical inline-validation *experience* for the sibling duplicate case
//! §A.4 does not itself cover, without pretending the engine rejects it.
//!
//! **Scope (N3).** `+ name` form, rename inline, delete-with-name confirm
//! (the confirm dialog only — the actual delete dispatches through
//! `notebook.rs`'s existing entry-delete path, N3's own wiring). No
//! dynamic-name authoring UI beyond the single "advanced: dynamic" flag
//! (static is the default); no names-manager modal (K5).

use std::sync::Arc;

use leptos::prelude::*;

use dnacalc_skin_ir::identity::NodeId;
use dnacalc_skin_ir::intent::{
    DefinedNameTargetIntent, Dispatcher, IntentError, IntentReceipt, WorkspaceIntent,
};
use dnacalc_skin_ir::workspace::{
    DefinedNameScopeProjection, DefinedNamesProjection, GridRectProjection,
};

/// The `_names` backing sheet's well-known grid id, as the notebook skin
/// addresses it (§B.7/F.4: a hidden-in-notebook, ordinary, append-only
/// sheet). Mirrors `dnacalc-host-core::NAMES_BACKING_SHEET`'s display-name
/// convention on the skin's own stable [`NodeId`] namespace — the skin never
/// sees the engine's opaque `sheet:{u64}` address (§A.2), so this is the
/// client-side name for the same sheet host-core resolves by display name.
pub const NAMES_BACKING_GRID: &str = "_names";

/// Compute the next free `_names` backing cell **client-side**, from the
/// workspace's own [`DefinedNamesProjection`] mirror — the read-side twin of
/// `dnacalc-host-core::WorkbookSession::allocate_next_names_backing_cell`
/// (same policy: highest existing static target row on the `_names` grid,
/// plus one; row 1 if none exist yet). Column A always (§B.7).
///
/// This is a **preview**, not a reservation: the skin has no session handle,
/// only the [`Dispatcher`] seam (§A.5), so it cannot itself create the
/// `_names` sheet or hold a lock on the next row. The host-core function of
/// the same name is authoritative once dispatch resolves `EnterGridCell`
/// against a lazily-created `_names` sheet (an H-track wiring note, not N3's
/// scope — N3 owns the notebook-side call site and the host-core function,
/// not the live dispatch plumbing connecting them).
#[must_use]
pub fn next_names_backing_cell(defined_names: &DefinedNamesProjection) -> (NodeId, u32, u32) {
    let grid = NodeId::new(NAMES_BACKING_GRID);
    let next_row = defined_names
        .entries
        .iter()
        .filter_map(|entry| {
            let DefinedNameScopeProjection::Sheet(scope_grid) = &entry.scope else {
                return None;
            };
            if scope_grid.as_str() != NAMES_BACKING_GRID {
                return None;
            }
            match &entry.target {
                dnacalc_skin_ir::workspace::DefinedNameTargetProjection::Static(rect) => {
                    Some(rect.bottom_row)
                }
                dnacalc_skin_ir::workspace::DefinedNameTargetProjection::Dynamic { .. } => None,
            }
        })
        .max()
        .map_or(1, |highest_row| highest_row + 1);
    (grid, next_row, 1)
}

/// A duplicate-name check against the workspace's own name catalog (see the
/// module doc's ENGINE-HONESTY note): `true` when `name` is already defined
/// at any scope. The form pre-empts dispatch entirely on a hit — no intent
/// goes out at all, matching "no mutation on a rejected write" for a case the
/// engine itself would silently last-write-win.
#[must_use]
pub fn name_already_defined(defined_names: &DefinedNamesProjection, name: &str) -> bool {
    defined_names.entries.iter().any(|entry| entry.name == name)
}

/// The result of attempting to commit a `+ name` creation (§B.3's Name loop):
/// the pre-check outcome, folded with the two-intent dispatch's own outcome
/// when the pre-check passes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameCommitResult {
    /// Both intents were dispatched and accepted (or the single `SetDefinedName`
    /// for a dynamic name).
    Committed,
    /// The client-side duplicate-name pre-check rejected the commit before any
    /// intent was dispatched — the honest non-engine rejection path (module
    /// doc). The form stays open with the entered text intact.
    DuplicateName { name: String },
    /// The engine rejected the write once dispatched (e.g. a workbook-scope
    /// name colliding with a root tree node's symbol,
    /// `IntentError::DefinedNameCollision`, or any other rejection) — surfaced
    /// verbatim, never dropped.
    EngineRejected { message: String },
}

/// Commit a **static** name creation (§B.3's Name loop, the Decision
/// paragraph): pre-check the name for a client-side duplicate, then dispatch
/// `EnterGridCell` at the allocated `_names` cell followed by
/// `SetDefinedName{Static}` over it — two intents, in that order, whenever the
/// pre-check passes. Returns the classified result; the caller (the form
/// component) reacts to it (stay open + show message on any rejection, close
/// on `Committed`).
///
/// Acceptance (1): the two intents are dispatched **in this order** — callers
/// asserting against a `RecordingDispatcher`/`RecordingPublisher`-style log
/// see `EnterGridCell` immediately followed by `SetDefinedName` for one
/// commit.
pub fn commit_static_name(
    dispatch: &dyn Dispatcher,
    defined_names: &DefinedNamesProjection,
    name: &str,
    body_text: &str,
) -> NameCommitResult {
    if name_already_defined(defined_names, name) {
        return NameCommitResult::DuplicateName {
            name: name.to_string(),
        };
    }

    let (grid, row, col) = next_names_backing_cell(defined_names);

    let enter_receipt = dispatch.dispatch(WorkspaceIntent::EnterGridCell {
        grid: grid.clone(),
        row,
        col,
        text: body_text.to_string(),
    });
    if !enter_receipt.accepted {
        return engine_rejection_result(&enter_receipt);
    }

    let define_receipt = dispatch.dispatch(WorkspaceIntent::SetDefinedName {
        scope: DefinedNameScopeProjection::Sheet(grid),
        name: name.to_string(),
        target: DefinedNameTargetIntent::Static(GridRectProjection {
            top_row: row,
            left_col: col,
            bottom_row: row,
            right_col: col,
        }),
    });
    if !define_receipt.accepted {
        return engine_rejection_result(&define_receipt);
    }

    NameCommitResult::Committed
}

/// Commit a **dynamic** name creation (§B.3's Name loop: "or `SetDefinedName{Dynamic}`
/// directly when the user marks it dynamic") — a single intent, no backing
/// cell, no `EnterGridCell`. Still pre-checked for a client-side duplicate
/// first, same as the static path.
pub fn commit_dynamic_name(
    dispatch: &dyn Dispatcher,
    defined_names: &DefinedNamesProjection,
    name: &str,
    source_text: &str,
) -> NameCommitResult {
    if name_already_defined(defined_names, name) {
        return NameCommitResult::DuplicateName {
            name: name.to_string(),
        };
    }
    let receipt = dispatch.dispatch(WorkspaceIntent::SetDefinedName {
        scope: DefinedNameScopeProjection::Sheet(NodeId::new(NAMES_BACKING_GRID)),
        name: name.to_string(),
        target: DefinedNameTargetIntent::Dynamic {
            source_text: source_text.to_string(),
        },
    });
    if !receipt.accepted {
        return engine_rejection_result(&receipt);
    }
    NameCommitResult::Committed
}

fn engine_rejection_result(receipt: &IntentReceipt) -> NameCommitResult {
    let message = match &receipt.error {
        Some(IntentError::DefinedNameCollision { name }) => {
            format!("'{name}' collides with an existing name or node")
        }
        Some(other) => other.to_string(),
        None => "the engine rejected this change".to_string(),
    };
    NameCommitResult::EngineRejected { message }
}

/// Rename an existing name in place (§B.3's Name loop: "Rename inline in the
/// entry header -> `RenameDefinedName` (engine heals references)").
///
/// Acceptance (3): dispatches **only** `RenameDefinedName` — no
/// `SetDefinedName`, no `EnterGridCell`, no allocation of any kind (a rename
/// never touches the backing cell).
pub fn commit_rename(
    dispatch: &dyn Dispatcher,
    scope: DefinedNameScopeProjection,
    old_name: &str,
    new_name: &str,
) -> IntentReceipt {
    dispatch.dispatch(WorkspaceIntent::RenameDefinedName {
        scope,
        old_name: old_name.to_string(),
        new_name: new_name.to_string(),
    })
}

/// Feedback the `+ name` form and the rename form show after a commit
/// attempt — `RwSignal`-friendly, matching `EntryFeedback`'s shape
/// (`cell_entry.rs`) so the notebook renders both through familiar patterns.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NameFormFeedback {
    #[default]
    None,
    DuplicateName {
        name: String,
    },
    EngineRejected {
        message: String,
    },
}

impl NameFormFeedback {
    #[must_use]
    pub fn from_result(result: &NameCommitResult) -> Self {
        match result {
            NameCommitResult::Committed => NameFormFeedback::None,
            NameCommitResult::DuplicateName { name } => {
                NameFormFeedback::DuplicateName { name: name.clone() }
            }
            NameCommitResult::EngineRejected { message } => NameFormFeedback::EngineRejected {
                message: message.clone(),
            },
        }
    }

    #[must_use]
    pub fn is_rejection(&self) -> bool {
        !matches!(self, NameFormFeedback::None)
    }
}

/// The `+ name` inline creation form (§B.3's Name loop): two fields (name,
/// body), an "advanced: dynamic" checkbox (the sole dynamic-name authoring
/// affordance this bead ships, per the NON-goals), and a Create button.
///
/// On commit: [`commit_static_name`] or [`commit_dynamic_name`] per the
/// dynamic flag. A rejection (duplicate or engine) keeps the form open with
/// both fields intact and renders the message inline; a clean commit clears
/// the fields and closes the form (`open` flips to `false`) so the caller's
/// entry list re-renders the new name from the next tick.
#[component]
pub fn NameForm(
    /// The workspace's current defined-names catalog — read fresh on every
    /// commit attempt (no stale snapshot risk: the caller re-renders this
    /// prop from its own reactive `WorkspaceState` read).
    defined_names: DefinedNamesProjection,
    /// The dispatcher the commit routes through.
    dispatch: Arc<dyn Dispatcher>,
    /// Whether the form is open; the caller owns this so the `+ name`
    /// affordance can toggle it.
    open: RwSignal<bool>,
) -> impl IntoView {
    let name_buffer = RwSignal::new(String::new());
    let body_buffer = RwSignal::new(String::new());
    let dynamic_flag = RwSignal::new(false);
    let feedback = RwSignal::new(NameFormFeedback::None);

    let commit = move |_| {
        let name = name_buffer.get_untracked();
        let body = body_buffer.get_untracked();
        if name.trim().is_empty() {
            return;
        }
        let result = if dynamic_flag.get_untracked() {
            commit_dynamic_name(dispatch.as_ref(), &defined_names, &name, &body)
        } else {
            commit_static_name(dispatch.as_ref(), &defined_names, &name, &body)
        };
        let next_feedback = NameFormFeedback::from_result(&result);
        if next_feedback.is_rejection() {
            // Stay open, keep both fields intact (module doc: "form stays
            // open" per acceptance (2)).
            feedback.set(next_feedback);
        } else {
            name_buffer.set(String::new());
            body_buffer.set(String::new());
            feedback.set(NameFormFeedback::None);
            open.set(false);
        }
    };

    let cancel = move |_| {
        name_buffer.set(String::new());
        body_buffer.set(String::new());
        feedback.set(NameFormFeedback::None);
        open.set(false);
    };

    let feedback_view = move || match feedback.get() {
        NameFormFeedback::None => ().into_any(),
        NameFormFeedback::DuplicateName { name } => view! {
            <div class="dtc-name-form__error" role="alert">
                {format!("'{name}' is already defined — choose a different name")}
            </div>
        }
        .into_any(),
        NameFormFeedback::EngineRejected { message } => view! {
            <div class="dtc-name-form__error" role="alert">{message}</div>
        }
        .into_any(),
    };

    view! {
        <div class="dtc-name-form" role="form" aria-label="New name">
            <input
                class="dtc-name-form__name"
                type="text"
                placeholder="name"
                prop:value=move || name_buffer.get()
                on:input=move |ev| name_buffer.set(event_target_value(&ev))
                aria-label="Name"
            />
            <span class="dtc-name-form__eq">"="</span>
            <input
                class="dtc-name-form__body"
                type="text"
                placeholder="0.065"
                prop:value=move || body_buffer.get()
                on:input=move |ev| body_buffer.set(event_target_value(&ev))
                aria-label="Value or formula"
            />
            <label class="dtc-name-form__advanced">
                <input
                    type="checkbox"
                    prop:checked=move || dynamic_flag.get()
                    on:change=move |ev| dynamic_flag.set(event_target_checked(&ev))
                />
                "advanced: dynamic"
            </label>
            <button type="button" class="dtc-name-form__create" on:click=commit>"Create"</button>
            <button type="button" class="dtc-name-form__cancel" on:click=cancel>"Cancel"</button>
            {feedback_view}
        </div>
    }
}

/// Rename-inline form (§B.3's Name loop: "Rename inline in the entry header").
///
/// Commits **only** `RenameDefinedName` (acceptance 3) — a text field seeded
/// from the current name, `Enter`/blur commits, `Esc` reverts without
/// dispatch (the same modeless-1-bit rule `CellEntryEditor` follows).
#[component]
pub fn RenameNameForm(
    scope: DefinedNameScopeProjection,
    current_name: String,
    dispatch: Arc<dyn Dispatcher>,
    /// Whether the rename field is open; the caller owns this the same way
    /// `NameForm`'s `open` works.
    editing: RwSignal<bool>,
) -> impl IntoView {
    let buffer = RwSignal::new(current_name.clone());
    let feedback = RwSignal::new(NameFormFeedback::None);

    let commit = {
        let scope = scope.clone();
        let current_name = current_name.clone();
        let dispatch = dispatch.clone();
        move || {
            let new_name = buffer.get_untracked();
            if new_name == current_name {
                editing.set(false);
                return;
            }
            let receipt = commit_rename(dispatch.as_ref(), scope.clone(), &current_name, &new_name);
            if receipt.accepted {
                feedback.set(NameFormFeedback::None);
                editing.set(false);
            } else {
                let message = match &receipt.error {
                    Some(IntentError::DefinedNameCollision { name }) => {
                        format!("'{name}' collides with an existing name or node")
                    }
                    Some(other) => other.to_string(),
                    None => "the engine rejected this change".to_string(),
                };
                feedback.set(NameFormFeedback::EngineRejected { message });
            }
        }
    };

    let commit_for_keys = commit.clone();
    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        ev.stop_propagation();
        match ev.key().as_str() {
            "Enter" => {
                ev.prevent_default();
                commit_for_keys();
            }
            "Escape" => {
                ev.prevent_default();
                buffer.set(current_name.clone());
                feedback.set(NameFormFeedback::None);
                editing.set(false);
            }
            _ => {}
        }
    };
    let on_blur = move |_| commit();
    let feedback_view = move || match feedback.get() {
        NameFormFeedback::None => ().into_any(),
        NameFormFeedback::DuplicateName { name } => view! {
            <div class="dtc-name-form__error" role="alert">
                {format!("'{name}' is already defined")}
            </div>
        }
        .into_any(),
        NameFormFeedback::EngineRejected { message } => view! {
            <div class="dtc-name-form__error" role="alert">{message}</div>
        }
        .into_any(),
    };

    view! {
        <div class="dtc-name-form dtc-name-form--rename">
            <input
                class="dtc-name-form__name"
                type="text"
                prop:value=move || buffer.get()
                on:input=move |ev| buffer.set(event_target_value(&ev))
                on:keydown=on_keydown
                on:blur=on_blur
                aria-label="Rename this name"
            />
            {feedback_view}
        </div>
    }
}

/// Shared CSS for the name-creation/rename forms (the `CELL_ENTRY_CSS`
/// pattern).
pub const NAME_FORM_CSS: &str = r#"
.dtc-name-form {
  display: flex; align-items: center; gap: 6px; flex-wrap: wrap;
  padding: 6px 8px; margin-top: 6px;
  background: var(--dtc-surface-panel); border: 1px solid var(--dtc-border);
  border-radius: 7px;
}
.dtc-name-form__name, .dtc-name-form__body {
  padding: 3px 6px; font: inherit; font-family: ui-monospace, monospace;
  color: var(--dtc-text); background: var(--dtc-surface);
  border: 1px solid var(--dtc-border); border-radius: 5px;
}
.dtc-name-form__eq { color: var(--dtc-text-subtle); }
.dtc-name-form__advanced { font-size: 11px; color: var(--dtc-text-subtle); display: flex; align-items: center; gap: 4px; }
.dtc-name-form__create, .dtc-name-form__cancel {
  padding: 3px 10px; border-radius: 5px; border: 1px solid var(--dtc-border);
  background: var(--dtc-surface); color: var(--dtc-text); cursor: pointer;
}
.dtc-name-form__error {
  flex-basis: 100%; padding: 3px 8px; border-radius: 5px; font-size: 12px;
  background: var(--dtc-error-surface, #fdecec); color: var(--dtc-error-text, #a11a1a);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnacalc_skin_ir::dispatcher::RecordingDispatcher;
    use dnacalc_skin_ir::workspace::DefinedNameProjection;

    fn rect(row: u32, col: u32) -> GridRectProjection {
        GridRectProjection {
            top_row: row,
            left_col: col,
            bottom_row: row,
            right_col: col,
        }
    }

    fn static_name(name: &str, row: u32) -> DefinedNameProjection {
        DefinedNameProjection {
            scope: DefinedNameScopeProjection::Sheet(NodeId::new(NAMES_BACKING_GRID)),
            name: name.to_string(),
            target: dnacalc_skin_ir::workspace::DefinedNameTargetProjection::Static(rect(row, 1)),
            is_dynamic: false,
        }
    }

    /// N3 acceptance (1): creating `rate = 0.065` dispatches `EnterGridCell`
    /// then `SetDefinedName` with the allocated `_names` cell — recorded
    /// order asserted.
    #[test]
    fn commit_static_name_dispatches_enter_then_set_defined_name_in_order() {
        let dispatcher = RecordingDispatcher::new();
        let catalog = DefinedNamesProjection::default();

        let result = commit_static_name(&dispatcher, &catalog, "rate", "0.065");
        assert_eq!(result, NameCommitResult::Committed);

        let intents = dispatcher.intents();
        assert_eq!(intents.len(), 2, "exactly two intents dispatched");
        match &intents[0] {
            WorkspaceIntent::EnterGridCell {
                grid,
                row,
                col,
                text,
            } => {
                assert_eq!(grid, &NodeId::new(NAMES_BACKING_GRID));
                assert_eq!((*row, *col), (1, 1), "first allocation lands on row 1");
                assert_eq!(text, "0.065");
            }
            other => panic!("expected EnterGridCell first, got {other:?}"),
        }
        match &intents[1] {
            WorkspaceIntent::SetDefinedName {
                scope,
                name,
                target,
            } => {
                assert_eq!(
                    scope,
                    &DefinedNameScopeProjection::Sheet(NodeId::new(NAMES_BACKING_GRID))
                );
                assert_eq!(name, "rate");
                assert_eq!(
                    target,
                    &DefinedNameTargetIntent::Static(rect(1, 1)),
                    "SetDefinedName targets the exact cell EnterGridCell just wrote"
                );
            }
            other => panic!("expected SetDefinedName second, got {other:?}"),
        }
    }

    /// N3: the allocation appends past an existing name's backing row rather
    /// than colliding with it.
    #[test]
    fn commit_static_name_appends_past_existing_names() {
        let dispatcher = RecordingDispatcher::new();
        let mut catalog = DefinedNamesProjection::default();
        catalog.entries.push(static_name("existing", 1));

        commit_static_name(&dispatcher, &catalog, "second", "1");

        let intents = dispatcher.intents();
        match &intents[0] {
            WorkspaceIntent::EnterGridCell { row, col, .. } => {
                assert_eq!(
                    (*row, *col),
                    (2, 1),
                    "appends at row 2, not colliding with row 1"
                );
            }
            other => panic!("expected EnterGridCell, got {other:?}"),
        }
    }

    /// N3 acceptance (2): a duplicate name is rejected inline, BEFORE any
    /// intent is dispatched (the honest client-side pre-check, module doc) —
    /// zero dispatches, not a partial write.
    #[test]
    fn commit_static_name_rejects_duplicate_before_any_dispatch() {
        let dispatcher = RecordingDispatcher::new();
        let mut catalog = DefinedNamesProjection::default();
        catalog.entries.push(static_name("rate", 1));

        let result = commit_static_name(&dispatcher, &catalog, "rate", "0.07");
        assert_eq!(
            result,
            NameCommitResult::DuplicateName {
                name: "rate".to_string()
            }
        );
        assert!(
            dispatcher.intents().is_empty(),
            "a duplicate name dispatches nothing at all"
        );
    }

    /// N3 acceptance (3): rename dispatches ONLY `RenameDefinedName` — no
    /// `SetDefinedName`, no `EnterGridCell`.
    #[test]
    fn commit_rename_dispatches_only_rename_defined_name() {
        let dispatcher = RecordingDispatcher::new();
        let receipt = commit_rename(
            &dispatcher,
            DefinedNameScopeProjection::Sheet(NodeId::new(NAMES_BACKING_GRID)),
            "rate",
            "taxRate",
        );
        assert!(receipt.accepted);

        let intents = dispatcher.intents();
        assert_eq!(intents.len(), 1, "exactly one intent dispatched");
        match &intents[0] {
            WorkspaceIntent::RenameDefinedName {
                old_name, new_name, ..
            } => {
                assert_eq!(old_name, "rate");
                assert_eq!(new_name, "taxRate");
            }
            other => panic!("expected RenameDefinedName, got {other:?}"),
        }
    }

    /// A dynamic name creation dispatches only `SetDefinedName{Dynamic}` — no
    /// backing-cell allocation at all (§B.3's Name loop: "or
    /// `SetDefinedName{Dynamic}` directly").
    #[test]
    fn commit_dynamic_name_dispatches_only_set_defined_name() {
        let dispatcher = RecordingDispatcher::new();
        let catalog = DefinedNamesProjection::default();

        let result = commit_dynamic_name(&dispatcher, &catalog, "monthly", "=PMT(rate/12,360,-1)");
        assert_eq!(result, NameCommitResult::Committed);

        let intents = dispatcher.intents();
        assert_eq!(intents.len(), 1);
        match &intents[0] {
            WorkspaceIntent::SetDefinedName { name, target, .. } => {
                assert_eq!(name, "monthly");
                assert_eq!(
                    target,
                    &DefinedNameTargetIntent::Dynamic {
                        source_text: "=PMT(rate/12,360,-1)".to_string()
                    }
                );
            }
            other => panic!("expected SetDefinedName, got {other:?}"),
        }
    }

    /// `next_names_backing_cell` on an empty catalog allocates row 1.
    #[test]
    fn next_names_backing_cell_starts_at_row_one() {
        let catalog = DefinedNamesProjection::default();
        let (grid, row, col) = next_names_backing_cell(&catalog);
        assert_eq!(grid, NodeId::new(NAMES_BACKING_GRID));
        assert_eq!((row, col), (1, 1));
    }

    /// `name_already_defined` checks across every scope, not just `_names`.
    #[test]
    fn name_already_defined_checks_any_scope() {
        let mut catalog = DefinedNamesProjection::default();
        catalog.entries.push(DefinedNameProjection {
            scope: DefinedNameScopeProjection::Workbook,
            name: "Total".to_string(),
            target: dnacalc_skin_ir::workspace::DefinedNameTargetProjection::Static(rect(5, 5)),
            is_dynamic: false,
        });
        assert!(name_already_defined(&catalog, "Total"));
        assert!(!name_already_defined(&catalog, "Other"));
    }
}
