//! The bridge's emission model — semantic events only (SHELL_SPEC §6).
//!
//! The bridge NEVER constructs `WorkspaceIntent` / `OneFormulaIntent` itself:
//! host adapters translate [`BridgeEvent`]s into whichever intent family
//! their document speaks (`OneFormulaIntent::EditText` in the Bench host,
//! `WorkspaceIntent::EnterGridCell` on the Calc degrade path). That
//! translation seam is exactly what lets one component serve both products
//! with no bridge API change at G1.

use leptos::prelude::Callback;

/// One semantic event out of the formula workbench.
///
/// Text and offsets are carried verbatim: `text` is the editor's content
/// byte-for-byte (the bridge never tokenizes, trims, or `=`-sniffs it), and
/// every offset is a UTF-8 byte offset into that text — the skin-IR span
/// convention — already converted from the DOM's UTF-16 selection units by
/// [`crate::vm::text_edited_from_dom`] / [`crate::vm::selection_from_dom`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BridgeEvent {
    /// The editor content changed. `caret` is the post-edit caret as a UTF-8
    /// byte offset into `text`.
    TextEdited { text: String, caret: usize },
    /// The selection changed without a text change. `anchor`/`focus` are
    /// UTF-8 byte offsets; `focus < anchor` encodes a backward selection.
    SelectionSet { anchor: usize, focus: usize },
    /// The user accepted a completion proposal. The id is the host's
    /// `CompletionItemProjection::proposal_id`, passed through untouched —
    /// the bridge never re-derives or edits text on the host's behalf.
    CompletionApplied { proposal_id: String },
    /// The user asked for the next window of an array the surfaces exposed.
    /// Offsets/counts honor the skin-IR window cap (≤ 64×64 ≤ 4096 cells).
    /// The host adapter resolves *which* array (result vs drill node) from
    /// the surfaces it supplied to this bridge instance.
    ArrayWindowRequested {
        row_offset: usize,
        col_offset: usize,
        row_count: usize,
        col_count: usize,
    },
    /// The X-Ray affordance was toggled.
    DrillToggled,
    /// Enter: commit through the context's single entry verb (the host maps
    /// this to `EnterGridCell` / `EditContent` / the name form).
    CommitRequested,
    /// Esc: exact-revert. The bridge dispatches nothing and reverts nothing
    /// itself — the host owns the committed text.
    RevertRequested,
}

/// The bridge's callback interface: every component takes one of these and
/// emits [`BridgeEvent`]s through it.
pub type BridgeEvents = Callback<BridgeEvent>;

/// The modeless 1-bit editing discipline (STRAND spine law: authoring is
/// modeless 1-bit). A bridge editor is either SELECTED (at rest) or EDITING
/// (buffer live); there is no third state. Mirrored onto the component root
/// as `data-edit-state` so chrome and tests can read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditDiscipline {
    #[default]
    Selected,
    Editing,
}

impl EditDiscipline {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::Editing => "editing",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_discipline_is_one_bit_and_stringly_stable() {
        assert_eq!(EditDiscipline::default(), EditDiscipline::Selected);
        assert_eq!(EditDiscipline::Selected.as_str(), "selected");
        assert_eq!(EditDiscipline::Editing.as_str(), "editing");
    }
}
