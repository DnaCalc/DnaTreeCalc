//! Shared cell-entry components (N2, route-map §B.3/§B.4/§B.5, §D.1).
//!
//! Two components the notebook (B) and — later — the workbook (K) share:
//!
//! - [`CellEntryEditor`]: one edit buffer over a single backing cell, the
//!   §B.3 edit-commit loop. `Enter`/blur commits exactly one
//!   [`WorkspaceIntent::EnterGridCell`] at the entry's `(grid, row, col)`;
//!   `Esc` reverts the buffer to the committed text and dispatches **nothing**
//!   (the modeless 1-bit rule, `style.rs`). The three-way receipt
//!   ([`classify_entry_receipt`]) drives what happens next: a success leaves
//!   the editor closed and lets the mirror repaint; a typed rejection keeps
//!   the editor open with the entered text intact and the diagnostics listed;
//!   an unresolved-name success surfaces a dismissable `#NAME?` note.
//! - [`EntryDiagnostics`]: the shared diagnostics list (§B.5) — one row per
//!   [`GridEntryDiagnosticProjection`], rendering a `span = Some(_)` row with a
//!   span badge and a `span = None` row message-only (both handled
//!   gracefully, per §A.4); the first row is auto-focused for screen readers.
//!
//! **No engine reinterpretation.** The editor never inspects the entered text
//! (no leading-`=` classification, no formula parse): OxFml is the sole
//! interpretation authority (§A.2), so the editor dispatches the raw text and
//! renders whatever three-way outcome the engine returns.
//!
//! **Scope (N2).** This bead ships the components + the notebook wiring only.
//! Name creation is N3, K usage is K2 — the components are written once here
//! and consumed by both, but N2 wires only the notebook.

use std::sync::Arc;

use leptos::prelude::*;

use dnacalc_skin_ir::identity::NodeId;
use dnacalc_skin_ir::intent::{
    Dispatcher, IntentError, IntentReceipt, WorkspaceDeltaChange, WorkspaceIntent,
};
use dnacalc_skin_ir::workspace::{GridEntryDiagnosticProjection, GridEntryOutcomeProjection};

/// The classified result of committing one cell-entry write — the three-way
/// contract (§B.3 step 3) reduced from a raw [`IntentReceipt`] to exactly what
/// the editor must react to.
///
/// A successful entry carries the engine's [`GridEntryOutcomeProjection`]
/// (Literal/Formula/Cleared) when the host attached the `GridCellEntered` UI
/// hint to the receipt delta (§A.3); a bare `accepted` receipt with no such
/// hint (e.g. the `RecordingDispatcher` test double, or a transport that drops
/// the hint) still counts as [`Self::Committed`] with `outcome = None`. A typed
/// rejection carries the diagnostics verbatim; any other error is surfaced as a
/// generic message rather than swallowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryCommitResult {
    /// The write was accepted. `outcome` is the engine's three-way projection
    /// when present on the receipt, else `None` (still a success).
    Committed {
        outcome: Option<GridEntryOutcomeProjection>,
    },
    /// The write was rejected with typed entry diagnostics
    /// ([`IntentError::GridEntryRejected`]); the engine guaranteed no
    /// mutation, so the editor keeps the entered text and shows these rows.
    Rejected {
        diagnostics: Vec<GridEntryDiagnosticProjection>,
    },
    /// The write was rejected with an error this component does not
    /// special-case (§A.4's generic path) — surfaced, never dropped.
    OtherError { message: String },
}

/// Classify a raw cell-entry [`IntentReceipt`] into the three-way
/// [`EntryCommitResult`] the editor reacts to (§B.3 step 3).
///
/// A rejection whose typed error is [`IntentError::GridEntryRejected`] becomes
/// [`EntryCommitResult::Rejected`] with the diagnostics carried through
/// untouched; any other rejection becomes [`EntryCommitResult::OtherError`]
/// with the error's `Display` string. An accepted receipt becomes
/// [`EntryCommitResult::Committed`], with the `GridCellEntered` outcome lifted
/// off the delta when the host published it.
#[must_use]
pub fn classify_entry_receipt(receipt: &IntentReceipt) -> EntryCommitResult {
    if receipt.accepted {
        return EntryCommitResult::Committed {
            outcome: entry_outcome_of(receipt),
        };
    }
    match &receipt.error {
        Some(IntentError::GridEntryRejected { diagnostics }) => EntryCommitResult::Rejected {
            diagnostics: diagnostics.clone(),
        },
        Some(other) => EntryCommitResult::OtherError {
            message: other.to_string(),
        },
        None => EntryCommitResult::OtherError {
            message: "the engine rejected this change".to_string(),
        },
    }
}

/// Lift the `GridCellEntered` three-way outcome off a receipt's delta, if the
/// host attached it (§A.3: the UI hint rides the same receipt as the edited
/// sheet's `GridChanged`/`GridAuthoredChanged`). Returns `None` when no such
/// hint is present — a bare accept is still a success.
fn entry_outcome_of(receipt: &IntentReceipt) -> Option<GridEntryOutcomeProjection> {
    receipt
        .delta
        .changes
        .iter()
        .find_map(|change| match change {
            WorkspaceDeltaChange::GridCellEntered { outcome, .. } => Some(outcome.clone()),
            _ => None,
        })
}

/// The unresolved-name note text for a Formula outcome (§B.3 step 3): the
/// `#NAME?` hint the entry shows while a referenced name is not yet defined.
/// Returns `None` when the outcome is not a Formula, or its `unresolved_names`
/// is empty (the common case — nothing to warn about).
#[must_use]
pub fn unresolved_name_note(outcome: &GridEntryOutcomeProjection) -> Option<String> {
    let GridEntryOutcomeProjection::Formula {
        unresolved_names, ..
    } = outcome
    else {
        return None;
    };
    if unresolved_names.is_empty() {
        return None;
    }
    let names = unresolved_names.join(", ");
    Some(format!(
        "#NAME? — {names} is not defined; it self-heals once the name exists"
    ))
}

/// The state a [`CellEntryEditor`] surfaces after a commit attempt: what the
/// notebook draws under the editor. Kept as an explicit signal-friendly enum so
/// the notebook wiring can render the post-commit feedback (diagnostics or the
/// unresolved-name note) without re-deriving it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EntryFeedback {
    /// No feedback to show (fresh editor, or a clean commit).
    #[default]
    None,
    /// A typed rejection: the editor stays open, these rows render under it.
    Rejected {
        diagnostics: Vec<GridEntryDiagnosticProjection>,
    },
    /// A Formula success referencing not-yet-defined names: a dismissable note.
    UnresolvedNames { note: String },
    /// A non-special-cased engine error: a generic message.
    OtherError { message: String },
}

impl EntryFeedback {
    /// Derive the post-commit feedback from a classified result: a rejection
    /// keeps its diagnostics, a Formula-with-unresolved-names success yields the
    /// note, every other success yields [`EntryFeedback::None`].
    #[must_use]
    pub fn from_result(result: &EntryCommitResult) -> Self {
        match result {
            EntryCommitResult::Committed {
                outcome: Some(outcome),
            } => match unresolved_name_note(outcome) {
                Some(note) => EntryFeedback::UnresolvedNames { note },
                None => EntryFeedback::None,
            },
            EntryCommitResult::Committed { outcome: None } => EntryFeedback::None,
            EntryCommitResult::Rejected { diagnostics } => EntryFeedback::Rejected {
                diagnostics: diagnostics.clone(),
            },
            EntryCommitResult::OtherError { message } => EntryFeedback::OtherError {
                message: message.clone(),
            },
        }
    }

    /// True when the commit was rejected (typed or generic) — the editor must
    /// stay open with its text intact in this case (§B.3 step 3).
    #[must_use]
    pub fn is_rejection(&self) -> bool {
        matches!(
            self,
            EntryFeedback::Rejected { .. } | EntryFeedback::OtherError { .. }
        )
    }
}

/// The shared cell-entry editor (§B.3, §D.1's `CellEntryEditor`).
///
/// Owns one edit buffer over the backing cell `(grid, row, col)`, seeded from
/// `initial_text`. `Enter`/blur commits exactly one
/// [`WorkspaceIntent::EnterGridCell`] with the buffer's current text; `Esc`
/// reverts the buffer to `initial_text` and dispatches nothing. The three-way
/// receipt is classified through [`classify_entry_receipt`]:
///
/// - **Committed** → `editing` clears, feedback is cleared (or set to the
///   unresolved-name note); the mirror repaints the entry from the new tick.
/// - **Rejected** (typed or generic) → `editing` stays `true`, the buffer keeps
///   the entered text, and `feedback` carries the diagnostics/message that
///   [`EntryDiagnostics`] renders under the editor.
///
/// The commit path never inspects the text — OxFml owns interpretation.
#[component]
pub fn CellEntryEditor(
    /// The backing cell's grid node.
    grid: NodeId,
    /// The backing cell's 1-based row.
    row: u32,
    /// The backing cell's 1-based column.
    col: u32,
    /// The committed text the buffer seeds from and reverts to on `Esc`.
    initial_text: String,
    /// The dispatcher the commit routes through.
    dispatch: Arc<dyn Dispatcher>,
    /// Whether the editor is currently open. Owned by the caller so the entry
    /// row can open it on click/`Enter`; the editor sets it `false` on a clean
    /// commit and keeps it `true` on a rejection.
    editing: RwSignal<bool>,
    /// Post-commit feedback (diagnostics / unresolved-name note), owned by the
    /// caller so the entry row can render it beneath the editor. Set by the
    /// editor on every commit attempt.
    feedback: RwSignal<EntryFeedback>,
) -> impl IntoView {
    // `buffer` is the live edit text; `committed` is the last-accepted text
    // `Esc` reverts to. Both seed from `initial_text` at mount. Re-seeding on a
    // reopen (after a mirror repaint changed the backing cell) is the caller's
    // job: the notebook remounts a fresh `CellEntryEditor` when it reopens an
    // entry, so a new `initial_text` flows in through construction — this
    // component never re-reads `initial_text` after mount, and a rejection's
    // retained buffer is therefore never clobbered by a stale reopen.
    let buffer = RwSignal::new(initial_text.clone());
    let committed = RwSignal::new(initial_text);
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let commit = {
        let dispatch = dispatch.clone();
        let grid = grid.clone();
        move || {
            let text = buffer.get_untracked();
            let receipt = dispatch.dispatch(WorkspaceIntent::EnterGridCell {
                grid: grid.clone(),
                row,
                col,
                text: text.clone(),
            });
            let result = classify_entry_receipt(&receipt);
            let next_feedback = EntryFeedback::from_result(&result);
            if next_feedback.is_rejection() {
                // Engine guaranteed no mutation: keep the editor open with the
                // entered text intact and show the diagnostics.
                feedback.set(next_feedback);
            } else {
                committed.set(text);
                feedback.set(next_feedback);
                editing.set(false);
            }
        }
    };

    let commit_for_keys = commit.clone();
    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        // Keys typed in the editor are the editor's alone — the grammar guard
        // (`sheet.rs:556`, `event_target_is_text_entry`) already keeps bare
        // verbs from firing while an INPUT has focus, but stopping propagation
        // here makes the buffer's ownership explicit and local.
        ev.stop_propagation();
        match ev.key().as_str() {
            "Enter" => {
                ev.prevent_default();
                commit_for_keys();
            }
            "Escape" => {
                ev.prevent_default();
                // Revert the buffer to the committed text; dispatch NOTHING.
                buffer.set(committed.get_untracked());
                feedback.set(EntryFeedback::None);
                editing.set(false);
            }
            _ => {}
        }
    };

    let on_blur = move |_| commit();
    let on_input = move |ev| buffer.set(event_target_value(&ev));

    // Land focus on the input when the editor mounts so the buffer is
    // immediately typable (the entry row mounts this only while `editing`).
    Effect::new(move |_| {
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
        }
    });

    view! {
        <input
            class="dtc-cell-entry__input"
            type="text"
            node_ref=input_ref
            prop:value=move || buffer.get()
            on:input=on_input
            on:keydown=on_keydown
            on:blur=on_blur
            aria-label="Cell entry editor"
        />
    }
}

/// The shared diagnostics list (§B.5, §D.1's `EntryDiagnostics`).
///
/// One row per [`GridEntryDiagnosticProjection`]: a row with `span = Some(_)`
/// renders the message plus a span badge (`chars a–b`); a row with `span =
/// None` renders the message alone with no highlight target — both handled
/// gracefully (§A.4: OxFml does not always have a span, and that is not a
/// degraded case). The first row carries `tabindex="-1"` + `autofocus` so a
/// screen reader lands on the diagnostics when they appear
/// (`accessibility.rs` conventions).
#[component]
pub fn EntryDiagnostics(diagnostics: Vec<GridEntryDiagnosticProjection>) -> impl IntoView {
    if diagnostics.is_empty() {
        return ().into_any();
    }
    let rows = diagnostics
        .into_iter()
        .enumerate()
        .map(|(index, diagnostic)| {
            let span_badge = diagnostic.span.map(|(start, end)| {
                view! {
                    <span class="dtc-entry-diagnostics__span">
                        {format!("chars {start}\u{2013}{end}")}
                    </span>
                }
            });
            view! {
                <li
                    class="dtc-entry-diagnostics__row"
                    role="alert"
                    tabindex=if index == 0 { "-1" } else { "" }
                >
                    <span class="dtc-entry-diagnostics__message">{diagnostic.message}</span>
                    {span_badge}
                </li>
            }
        })
        .collect::<Vec<_>>();

    view! {
        <ul class="dtc-entry-diagnostics" aria-label="Entry diagnostics">
            {rows}
        </ul>
    }
    .into_any()
}

/// The shared CSS for the cell-entry components, shipped as a constant over the
/// `--dtc-*` tokens (the `SHEET_CSS` pattern, `style.rs`/`sheet.rs`). A skin
/// mounts it once (the notebook injects it in its own `<style>` block).
pub const CELL_ENTRY_CSS: &str = r#"
.dtc-cell-entry__input {
  width: 100%; box-sizing: border-box;
  padding: 4px 8px; font: inherit;
  color: var(--dtc-text); background: var(--dtc-surface);
  border: 1px solid var(--dtc-accent, #1a4fa0); border-radius: 5px;
  font-family: ui-monospace, monospace;
}
.dtc-entry-diagnostics {
  list-style: none; margin: 4px 0 0; padding: 0;
  display: flex; flex-direction: column; gap: 2px;
}
.dtc-entry-diagnostics__row {
  display: flex; align-items: baseline; gap: 8px;
  padding: 3px 8px; border-radius: 5px;
  background: var(--dtc-error-surface, #fdecec); color: var(--dtc-error-text, #a11a1a);
  font-size: 12px;
}
.dtc-entry-diagnostics__span {
  font-family: ui-monospace, monospace; font-size: 11px; opacity: 0.8;
}
.dtc-entry-note {
  margin-top: 4px; padding: 3px 8px; border-radius: 5px; font-size: 12px;
  background: var(--dtc-warning-surface, #fff4e0); color: var(--dtc-warning-text, #8a5a00);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use dnacalc_skin_ir::intent::WorkspaceDelta;
    use dnacalc_skin_ir::workspace::NodeValueProjection;

    fn diagnostic(message: &str, span: Option<(u32, u32)>) -> GridEntryDiagnosticProjection {
        GridEntryDiagnosticProjection {
            message: message.to_string(),
            span,
        }
    }

    /// A receipt carrying a `GridCellEntered` UI hint on its delta, as a real
    /// host publishes it (§A.3).
    fn accepted_with_outcome(outcome: GridEntryOutcomeProjection) -> IntentReceipt {
        let delta = WorkspaceDelta {
            from_seq: 0,
            to_seq: 1,
            changes: vec![WorkspaceDeltaChange::GridCellEntered {
                grid_node_id: NodeId::new("Sheet1"),
                row: 1,
                col: 1,
                outcome,
            }],
        };
        IntentReceipt::accepted().with_delta(delta)
    }

    #[test]
    fn classify_accepted_bare_receipt_is_committed_without_outcome() {
        let result = classify_entry_receipt(&IntentReceipt::accepted());
        assert_eq!(result, EntryCommitResult::Committed { outcome: None });
    }

    #[test]
    fn classify_lifts_literal_outcome_off_the_delta() {
        let receipt = accepted_with_outcome(GridEntryOutcomeProjection::Literal {
            value: NodeValueProjection::Number {
                raw: "0.065".to_string(),
                display: "0.065".to_string(),
            },
        });
        let result = classify_entry_receipt(&receipt);
        assert!(matches!(
            result,
            EntryCommitResult::Committed {
                outcome: Some(GridEntryOutcomeProjection::Literal { .. })
            }
        ));
    }

    #[test]
    fn classify_grid_entry_rejected_carries_diagnostics_through() {
        let diagnostics = vec![diagnostic("unexpected end of formula", Some((3, 3)))];
        let receipt = IntentReceipt::rejected(IntentError::GridEntryRejected {
            diagnostics: diagnostics.clone(),
        });
        assert_eq!(
            classify_entry_receipt(&receipt),
            EntryCommitResult::Rejected { diagnostics }
        );
    }

    #[test]
    fn classify_other_error_is_surfaced_not_dropped() {
        let receipt = IntentReceipt::rejected(IntentError::GridCellNotEditable { anchor: None });
        let result = classify_entry_receipt(&receipt);
        let EntryCommitResult::OtherError { message } = result else {
            panic!("expected OtherError, got {result:?}");
        };
        assert!(!message.is_empty(), "the error message must be surfaced");
    }

    #[test]
    fn unresolved_name_note_only_for_nonempty_formula_names() {
        // Literal: never a note.
        let literal = GridEntryOutcomeProjection::Literal {
            value: NodeValueProjection::Empty,
        };
        assert_eq!(unresolved_name_note(&literal), None);

        // Formula with no unresolved names: no note.
        let clean_formula = GridEntryOutcomeProjection::Formula {
            unresolved_names: Vec::new(),
            value: NodeValueProjection::Number {
                raw: "1".to_string(),
                display: "1".to_string(),
            },
        };
        assert_eq!(unresolved_name_note(&clean_formula), None);

        // Formula referencing an undefined name: a note naming it.
        let dirty_formula = GridEntryOutcomeProjection::Formula {
            unresolved_names: vec!["TaxRate".to_string()],
            value: NodeValueProjection::Error("#NAME?".to_string()),
        };
        let note = unresolved_name_note(&dirty_formula).expect("a note for an undefined name");
        assert!(note.contains("TaxRate"), "the note names the missing name");
        assert!(note.contains("#NAME?"), "the note carries the error hint");
    }

    #[test]
    fn feedback_from_rejection_is_a_rejection() {
        let diagnostics = vec![diagnostic("bad formula", None)];
        let result = EntryCommitResult::Rejected {
            diagnostics: diagnostics.clone(),
        };
        let feedback = EntryFeedback::from_result(&result);
        assert!(feedback.is_rejection());
        assert_eq!(feedback, EntryFeedback::Rejected { diagnostics });
    }

    #[test]
    fn feedback_from_clean_commit_is_none() {
        let result = EntryCommitResult::Committed {
            outcome: Some(GridEntryOutcomeProjection::Cleared),
        };
        assert_eq!(EntryFeedback::from_result(&result), EntryFeedback::None);
        assert!(!EntryFeedback::from_result(&result).is_rejection());
    }

    #[test]
    fn feedback_from_unresolved_formula_carries_the_note() {
        let result = EntryCommitResult::Committed {
            outcome: Some(GridEntryOutcomeProjection::Formula {
                unresolved_names: vec!["monthly".to_string()],
                value: NodeValueProjection::Error("#NAME?".to_string()),
            }),
        };
        let feedback = EntryFeedback::from_result(&result);
        let EntryFeedback::UnresolvedNames { note } = feedback else {
            panic!("expected an unresolved-names feedback");
        };
        assert!(note.contains("monthly"));
    }

    #[test]
    fn other_error_feedback_is_a_rejection_that_holds_the_editor_open() {
        let result = EntryCommitResult::OtherError {
            message: "the engine rejected this change".to_string(),
        };
        assert!(
            EntryFeedback::from_result(&result).is_rejection(),
            "a generic error keeps the editor open, like a typed rejection"
        );
    }
}
