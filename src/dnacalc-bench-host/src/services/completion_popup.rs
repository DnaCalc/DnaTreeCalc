//! Completion popup state model.
//!
//! Pure state machine. No UI rendering, no DOM coupling, no keyboard
//! event plumbing — those live in later beads. This module owns:
//!
//!   * the state shape (`CompletionPopupState` + the items it carries),
//!   * the transition functions exposed to the reducer,
//!   * the auto-open / auto-close policy applied after every bridge
//!     round-trip.
//!
//! The discipline that broke the previous WS-13 attempt at this work
//! was that UI listeners leaked onto the textarea even when no popup
//! was visible — intercepting native arrow-key behaviour. This module
//! ensures the *state* knows whether the popup is open; later beads
//! gate keyboard listeners on that state, with explicit tests that no
//! handler is installed when `Hidden`.
//!
//! ## Lifecycle
//!
//! ```text
//!          sync_with_proposals(non-empty)
//!   Hidden ──────────────────────────────▶ Open
//!     ▲                                     │
//!     │ sync_with_proposals(empty)          │ move(±)
//!     │ dismiss                             │ — stays Open with new
//!     │ accept                              │   selected_index
//!     └─────────────────────────────────────┘
//! ```

use crate::adapters::oxfml::{CompletionProposal, CompletionProposalKind, FormulaTextSpan};

/// The popup's lifecycle state. Sits on `FormulaSpaceState`, defaults
/// to [`CompletionPopupState::Hidden`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CompletionPopupState {
    #[default]
    Hidden,
    Open {
        /// Caret offset where the popup anchors. The popup positions
        /// itself one character cell below this offset.
        anchor_offset: usize,
        items: Vec<CompletionPopupItem>,
        /// Index into `items`. Always in `0..items.len()` when state
        /// is `Open` (verified at every transition).
        selected_index: usize,
    },
}

/// One row of the popup. Carries the upstream proposal payload plus a
/// kind tag, decoupled from `CompletionProposalKind` so the popup
/// view-model never has to know about the upstream re-export shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionPopupItem {
    pub proposal_id: String,
    pub display_text: String,
    pub insert_text: String,
    pub kind: CompletionPopupKind,
    pub replacement_span: Option<FormulaTextSpan>,
    pub documentation_ref: Option<String>,
}

/// Mirror of `CompletionProposalKind` lifted to the popup-state layer.
/// Keeping it local lets the UI evolve glyph / label vocabulary without
/// touching the bridge re-exports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionPopupKind {
    Function,
    DefinedName,
    TableName,
    TableColumn,
    StructuredSelector,
    SyntaxAssist,
    ProfileReference,
}

impl CompletionPopupKind {
    pub fn from_proposal_kind(kind: CompletionProposalKind) -> Self {
        match kind {
            CompletionProposalKind::Function => Self::Function,
            CompletionProposalKind::DefinedName => Self::DefinedName,
            CompletionProposalKind::TableName => Self::TableName,
            CompletionProposalKind::TableColumn => Self::TableColumn,
            CompletionProposalKind::StructuredSelector => Self::StructuredSelector,
            CompletionProposalKind::SyntaxAssist => Self::SyntaxAssist,
            CompletionProposalKind::ProfileReference => Self::ProfileReference,
        }
    }
}

/// Result of accepting the selected completion. The reducer hands this
/// to the editor-input layer so the textarea text + caret update, and
/// the bridge re-runs to refresh proposals / diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionAcceptance {
    /// The text to splice into the textarea.
    pub insert_text: String,
    /// The span of source text the insertion replaces.
    pub replacement_span: Option<FormulaTextSpan>,
    /// The caret offset to set after the insertion (always positioned
    /// at the end of the inserted text in the post-edit string).
    pub new_caret_offset: usize,
}

impl CompletionPopupItem {
    /// Build a popup item from an upstream proposal.
    pub fn from_proposal(proposal: CompletionProposal) -> Self {
        Self {
            proposal_id: proposal.proposal_id,
            display_text: proposal.display_text,
            insert_text: proposal.insert_text,
            kind: CompletionPopupKind::from_proposal_kind(proposal.proposal_kind),
            replacement_span: proposal.replacement_span,
            documentation_ref: proposal.documentation_ref,
        }
    }
}

/// Reconcile the popup state with a fresh list of proposals from the
/// bridge. Called after every `apply_live_editor_input`.
///
/// Transitions:
///   * `Hidden + non-empty` -> `Open { anchor_offset, items, selected_index = 0 }`
///   * `Hidden + empty` -> stays `Hidden`
///   * `Open + non-empty (same length)` -> `Open` with `selected_index`
///     preserved (clamped to new bounds defensively)
///   * `Open + non-empty (different length)` -> `Open` with
///     `selected_index = 0` (the proposal list shape changed; the
///     prior selection is no longer meaningful)
///   * `Open + empty` -> `Hidden`
///
/// Returns `true` when state actually changed so the reducer can
/// short-circuit re-renders.
pub fn sync_completion_popup_with_proposals(
    state: &mut CompletionPopupState,
    anchor_offset: usize,
    items: Vec<CompletionPopupItem>,
) -> bool {
    if items.is_empty() {
        if matches!(state, CompletionPopupState::Hidden) {
            return false;
        }
        *state = CompletionPopupState::Hidden;
        return true;
    }

    let len = items.len();
    let next = match state {
        CompletionPopupState::Hidden => CompletionPopupState::Open {
            anchor_offset,
            items,
            selected_index: 0,
        },
        CompletionPopupState::Open {
            items: prior_items,
            selected_index: prior_index,
            ..
        } => {
            let preserved_index = if prior_items.len() == len {
                (*prior_index).min(len.saturating_sub(1))
            } else {
                0
            };
            CompletionPopupState::Open {
                anchor_offset,
                items,
                selected_index: preserved_index,
            }
        }
    };

    if state == &next {
        return false;
    }
    *state = next;
    true
}

/// Move the selected index forward (`+1`) or backward (`-1`). Wraps
/// at both ends. No-op when `Hidden`. Returns `true` when state
/// changed.
pub fn move_completion_selection(state: &mut CompletionPopupState, delta: i32) -> bool {
    let CompletionPopupState::Open {
        items,
        selected_index,
        ..
    } = state
    else {
        return false;
    };
    if items.is_empty() {
        return false;
    }
    let len = items.len() as i32;
    let next_index = ((*selected_index as i32) + delta).rem_euclid(len) as usize;
    if next_index == *selected_index {
        return false;
    }
    *selected_index = next_index;
    true
}

/// Force the popup to `Hidden`. No-op when already `Hidden`. Returns
/// `true` when state changed.
pub fn dismiss_completion_popup(state: &mut CompletionPopupState) -> bool {
    if matches!(state, CompletionPopupState::Hidden) {
        return false;
    }
    *state = CompletionPopupState::Hidden;
    true
}

/// Accept the selected completion. Returns `Some(CompletionAcceptance)`
/// describing the splice the editor layer should apply, plus
/// transitions to `Hidden`. Returns `None` (and leaves state alone)
/// when the popup is `Hidden`.
pub fn accept_selected_completion(
    state: &mut CompletionPopupState,
    raw_text: &str,
) -> Option<CompletionAcceptance> {
    let CompletionPopupState::Open {
        items,
        selected_index,
        ..
    } = state
    else {
        return None;
    };
    let item = items.get(*selected_index).cloned()?;

    let new_caret_offset = compute_caret_after_insert(raw_text, &item);
    *state = CompletionPopupState::Hidden;
    Some(CompletionAcceptance {
        insert_text: item.insert_text,
        replacement_span: item.replacement_span,
        new_caret_offset,
    })
}

/// Compute where the caret lands after the proposal's `insert_text`
/// replaces its `replacement_span` in `raw_text`. If `replacement_span`
/// is `None`, the insertion is treated as appending at the end of the
/// text.
fn compute_caret_after_insert(raw_text: &str, item: &CompletionPopupItem) -> usize {
    let insert_chars = item.insert_text.chars().count();
    match item.replacement_span {
        Some(span) => span.start.saturating_add(insert_chars),
        None => raw_text.chars().count().saturating_add(insert_chars),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, display: &str, insert: &str, span: FormulaTextSpan) -> CompletionPopupItem {
        CompletionPopupItem {
            proposal_id: id.to_string(),
            display_text: display.to_string(),
            insert_text: insert.to_string(),
            kind: CompletionPopupKind::Function,
            replacement_span: Some(span),
            documentation_ref: None,
        }
    }

    fn span(start: usize, len: usize) -> FormulaTextSpan {
        FormulaTextSpan { start, len }
    }

    fn three_items() -> Vec<CompletionPopupItem> {
        vec![
            item("p-1", "SUM", "SUM(", span(1, 2)),
            item("p-2", "SUMIF", "SUMIF(", span(1, 2)),
            item("p-3", "SUMIFS", "SUMIFS(", span(1, 2)),
        ]
    }

    // ----- sync transitions -----

    #[test]
    fn sync_hidden_with_non_empty_proposals_opens_with_first_item_selected() {
        let mut state = CompletionPopupState::Hidden;
        let changed = sync_completion_popup_with_proposals(&mut state, 3, three_items());
        assert!(changed);
        match &state {
            CompletionPopupState::Open {
                anchor_offset,
                items,
                selected_index,
            } => {
                assert_eq!(*anchor_offset, 3);
                assert_eq!(items.len(), 3);
                assert_eq!(*selected_index, 0);
            }
            _ => panic!("expected Open"),
        }
    }

    #[test]
    fn sync_hidden_with_empty_proposals_is_a_noop() {
        let mut state = CompletionPopupState::Hidden;
        let changed = sync_completion_popup_with_proposals(&mut state, 0, vec![]);
        assert!(!changed);
        assert!(matches!(state, CompletionPopupState::Hidden));
    }

    #[test]
    fn sync_open_with_same_length_proposals_preserves_selected_index() {
        let mut state = CompletionPopupState::Open {
            anchor_offset: 3,
            items: three_items(),
            selected_index: 2,
        };
        // Same length; expect selected_index preserved.
        let _ = sync_completion_popup_with_proposals(&mut state, 4, three_items());
        match state {
            CompletionPopupState::Open {
                selected_index,
                anchor_offset,
                ..
            } => {
                assert_eq!(selected_index, 2);
                assert_eq!(anchor_offset, 4, "anchor still updates");
            }
            _ => panic!("expected Open"),
        }
    }

    #[test]
    fn sync_open_with_different_length_proposals_resets_selected_index() {
        let mut state = CompletionPopupState::Open {
            anchor_offset: 3,
            items: three_items(),
            selected_index: 2,
        };
        let single = vec![item("p-x", "SUMSQ", "SUMSQ(", span(1, 2))];
        let _ = sync_completion_popup_with_proposals(&mut state, 3, single);
        match state {
            CompletionPopupState::Open {
                selected_index,
                items,
                ..
            } => {
                assert_eq!(items.len(), 1);
                assert_eq!(selected_index, 0);
            }
            _ => panic!("expected Open"),
        }
    }

    #[test]
    fn sync_open_with_empty_proposals_dismisses_to_hidden() {
        let mut state = CompletionPopupState::Open {
            anchor_offset: 3,
            items: three_items(),
            selected_index: 1,
        };
        let changed = sync_completion_popup_with_proposals(&mut state, 0, vec![]);
        assert!(changed);
        assert!(matches!(state, CompletionPopupState::Hidden));
    }

    // ----- selection movement -----

    #[test]
    fn move_selection_forward_advances_index() {
        let mut state = CompletionPopupState::Open {
            anchor_offset: 3,
            items: three_items(),
            selected_index: 0,
        };
        assert!(move_completion_selection(&mut state, 1));
        match state {
            CompletionPopupState::Open { selected_index, .. } => assert_eq!(selected_index, 1),
            _ => panic!("expected Open"),
        }
    }

    #[test]
    fn move_selection_forward_at_last_wraps_to_first() {
        let mut state = CompletionPopupState::Open {
            anchor_offset: 3,
            items: three_items(),
            selected_index: 2,
        };
        assert!(move_completion_selection(&mut state, 1));
        match state {
            CompletionPopupState::Open { selected_index, .. } => assert_eq!(selected_index, 0),
            _ => panic!("expected Open"),
        }
    }

    #[test]
    fn move_selection_backward_at_first_wraps_to_last() {
        let mut state = CompletionPopupState::Open {
            anchor_offset: 3,
            items: three_items(),
            selected_index: 0,
        };
        assert!(move_completion_selection(&mut state, -1));
        match state {
            CompletionPopupState::Open { selected_index, .. } => assert_eq!(selected_index, 2),
            _ => panic!("expected Open"),
        }
    }

    #[test]
    fn move_selection_when_hidden_is_a_noop() {
        let mut state = CompletionPopupState::Hidden;
        assert!(!move_completion_selection(&mut state, 1));
        assert!(matches!(state, CompletionPopupState::Hidden));
    }

    #[test]
    fn move_selection_with_one_item_does_not_change_index() {
        let mut state = CompletionPopupState::Open {
            anchor_offset: 3,
            items: vec![item("p", "SUM", "SUM(", span(1, 2))],
            selected_index: 0,
        };
        assert!(!move_completion_selection(&mut state, 1));
        assert!(!move_completion_selection(&mut state, -1));
    }

    // ----- dismiss & accept -----

    #[test]
    fn dismiss_open_transitions_to_hidden() {
        let mut state = CompletionPopupState::Open {
            anchor_offset: 3,
            items: three_items(),
            selected_index: 1,
        };
        assert!(dismiss_completion_popup(&mut state));
        assert!(matches!(state, CompletionPopupState::Hidden));
    }

    #[test]
    fn dismiss_when_already_hidden_is_a_noop() {
        let mut state = CompletionPopupState::Hidden;
        assert!(!dismiss_completion_popup(&mut state));
    }

    #[test]
    fn accept_when_open_returns_acceptance_and_transitions_to_hidden() {
        let mut state = CompletionPopupState::Open {
            anchor_offset: 3,
            items: three_items(),
            selected_index: 1,
        };
        let acceptance = accept_selected_completion(&mut state, "=SU");
        assert!(matches!(state, CompletionPopupState::Hidden));
        let acceptance = acceptance.expect("acceptance returned");
        assert_eq!(acceptance.insert_text, "SUMIF(");
        assert_eq!(acceptance.replacement_span, Some(span(1, 2)));
        // span replaces chars [1..3); inserting "SUMIF(" (6 chars) yields
        // caret at 1 + 6 = 7.
        assert_eq!(acceptance.new_caret_offset, 7);
    }

    #[test]
    fn accept_when_hidden_returns_none_and_does_not_change_state() {
        let mut state = CompletionPopupState::Hidden;
        let acceptance = accept_selected_completion(&mut state, "=SU");
        assert!(acceptance.is_none());
        assert!(matches!(state, CompletionPopupState::Hidden));
    }

    #[test]
    fn accept_with_no_replacement_span_appends_at_end() {
        let mut state = CompletionPopupState::Open {
            anchor_offset: 3,
            items: vec![CompletionPopupItem {
                proposal_id: "p".to_string(),
                display_text: "SUM".to_string(),
                insert_text: "SUM(".to_string(),
                kind: CompletionPopupKind::Function,
                replacement_span: None,
                documentation_ref: None,
            }],
            selected_index: 0,
        };
        let acceptance = accept_selected_completion(&mut state, "=SU").expect("acceptance");
        // append at end: text length 3 chars + 4 inserted = caret at 7
        assert_eq!(acceptance.new_caret_offset, 7);
    }

    // ----- defensive bounds -----

    #[test]
    fn sync_open_with_shorter_list_clamps_preserved_index_to_new_bounds() {
        let mut state = CompletionPopupState::Open {
            anchor_offset: 3,
            items: three_items(),
            selected_index: 2,
        };
        let two = vec![
            item("p-1", "SUM", "SUM(", span(1, 2)),
            item("p-2", "SUMIF", "SUMIF(", span(1, 2)),
        ];
        let _ = sync_completion_popup_with_proposals(&mut state, 3, two);
        match state {
            CompletionPopupState::Open {
                items,
                selected_index,
                ..
            } => {
                // length changed (3 -> 2), so selected_index resets to 0
                assert_eq!(items.len(), 2);
                assert_eq!(selected_index, 0);
            }
            _ => panic!("expected Open"),
        }
    }

    #[test]
    fn from_proposal_kind_covers_all_variants() {
        use crate::adapters::oxfml::CompletionProposalKind as Up;
        assert_eq!(
            CompletionPopupKind::from_proposal_kind(Up::Function),
            CompletionPopupKind::Function
        );
        assert_eq!(
            CompletionPopupKind::from_proposal_kind(Up::DefinedName),
            CompletionPopupKind::DefinedName
        );
        assert_eq!(
            CompletionPopupKind::from_proposal_kind(Up::TableName),
            CompletionPopupKind::TableName
        );
        assert_eq!(
            CompletionPopupKind::from_proposal_kind(Up::TableColumn),
            CompletionPopupKind::TableColumn
        );
        assert_eq!(
            CompletionPopupKind::from_proposal_kind(Up::StructuredSelector),
            CompletionPopupKind::StructuredSelector
        );
        assert_eq!(
            CompletionPopupKind::from_proposal_kind(Up::SyntaxAssist),
            CompletionPopupKind::SyntaxAssist
        );
        assert_eq!(
            CompletionPopupKind::from_proposal_kind(Up::ProfileReference),
            CompletionPopupKind::ProfileReference
        );
    }
}
