//! Personas — who may ask the host for what (tenet 9: every interaction is
//! governed at the dispatcher chokepoint).
//!
//! The policy is intentionally a closed, reviewable function over the closed
//! intent enum: adding an intent variant forces a decision here. First slice:
//!
//! - **Author** — everything.
//! - **Reviewer** — read/navigate, recalculate, annotate (`SetNote`), copy,
//!   and the *non-publishing* candidate verbs (speculation is
//!   consequence-free by construction, so reviewers may explore what-ifs —
//!   but never `CommitCandidate`, never structural/content edits, never
//!   scenario/sweep bookkeeping).
//! - **ReadOnly** — selection and workspace switching only.
//!
//! Persona switching itself travels as an intent (`SetPersona`) so it is
//! audited like everything else; in this first slice any persona may switch
//! (a real deployment binds personas to an auth layer — ROADMAP open
//! question 12).

use serde::{Deserialize, Serialize};

use crate::command::CommandIntentKindProjection;
use crate::intent::WorkspaceIntent;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Persona {
    #[default]
    Author,
    Reviewer,
    ReadOnly,
}

impl Persona {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Author => "author",
            Self::Reviewer => "reviewer",
            Self::ReadOnly => "read_only",
        }
    }

    #[must_use]
    pub const fn can_mutate(self) -> bool {
        matches!(self, Self::Author)
    }

    /// The governance policy: may this persona dispatch this intent?
    #[must_use]
    pub fn allows(self, intent: &WorkspaceIntent) -> bool {
        use WorkspaceIntent as I;
        match self {
            Self::Author => true,
            Self::Reviewer => matches!(
                intent,
                // Read/navigate.
                I::SelectNode(_)
                    | I::SelectNodes { .. }
                    | I::SelectTableCell { .. }
                    | I::SwitchWorkspace { .. }
                    | I::Recalculate
                    // Annotate + copy.
                    | I::SetNote { .. }
                    | I::CopyToClipboard { .. }
                    // Non-publishing speculation (never CommitCandidate).
                    | I::OpenCandidate { .. }
                    | I::EditCandidateContent { .. }
                    | I::RenameCandidateNode { .. }
                    | I::MoveCandidateNode { .. }
                    | I::ReorderCandidateNode { .. }
                    | I::DeleteCandidateNode { .. }
                    | I::AddCandidateNode { .. }
                    | I::EvaluateCandidate { .. }
                    | I::RebaseCandidate { .. }
                    | I::DiscardCandidate { .. }
                    | I::PinCandidateRetention { .. }
                    | I::UnpinCandidateRetention { .. }
            ),
            Self::ReadOnly => matches!(
                intent,
                I::SelectNode(_)
                    | I::SelectNodes { .. }
                    | I::SelectTableCell { .. }
                    | I::SwitchWorkspace { .. }
            ),
        }
    }

    /// The same governance policy, projected onto the read-side command
    /// catalog's intent *kinds*. This lets a lens pre-disable an affordance
    /// the active persona may not dispatch — instead of letting the user
    /// discover the typed `Forbidden` rejection only after pressing the
    /// button. It must agree with [`Self::allows`] for every kind: a catalog
    /// command maps to a dispatched intent of the same kind, and
    /// `permission_policy_is_consistent` pins the two together.
    ///
    /// Note the catalog carries no pure-navigation kinds (selection and
    /// workspace switching are not commands), so a `ReadOnly` persona — which
    /// may *only* navigate — forbids every catalog command.
    #[must_use]
    pub fn allows_intent_kind(self, kind: CommandIntentKindProjection) -> bool {
        use CommandIntentKindProjection as K;
        match self {
            Self::Author => true,
            Self::Reviewer => matches!(
                kind,
                K::Recalculate
                    | K::SetNote
                    | K::CopyValues
                    | K::CopyFormula
                    | K::CopyFormat
                    | K::CopySubtree
                    // Non-publishing speculation only — never CommitCandidate
                    // or candidate reclamation.
                    | K::OpenCandidate
                    | K::EvaluateCandidate
                    | K::RebaseCandidate
                    | K::DiscardCandidate
            ),
            Self::ReadOnly => false,
        }
    }
}

impl std::fmt::Display for Persona {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.stable_id())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{NodeId, NodeKey};
    use crate::intent::{AuthoringScope, ClipboardPayloadKind};
    use crate::workspace::InitialNodeContentProjection;

    /// A representative dispatched intent for each catalog kind, so the
    /// read-side `allows_intent_kind` can be checked against the authoritative
    /// `allows` over real intents. `allows` only matches on the variant, so
    /// the payloads here are minimal stand-ins.
    fn representative_intent(kind: CommandIntentKindProjection) -> WorkspaceIntent {
        use CommandIntentKindProjection as K;
        use WorkspaceIntent as I;
        let node = || NodeId::new("tree-node:1");
        let key = || NodeKey::new("k1");
        let scope = || AuthoringScope::Node(key());
        match kind {
            K::NewWorkspace => I::NewWorkspace,
            K::Recalculate => I::Recalculate,
            K::AddNode => I::AddNode {
                parent: None,
                symbol: "N".to_string(),
                initial: InitialNodeContentProjection::Empty,
                is_meta: false,
            },
            K::RenameNode => I::RenameNode {
                node: node(),
                new_symbol: "N".to_string(),
            },
            K::MoveNode => I::MoveNode {
                node: node(),
                new_parent: None,
                new_index: None,
            },
            K::ReorderNode => I::ReorderNode {
                node: node(),
                new_index: 0,
            },
            K::DeleteNode => I::DeleteNode { node: node() },
            K::EditContent => I::EditContent {
                node: node(),
                content: String::new(),
            },
            K::SetNumberFormat => I::SetNumberFormat {
                scope: scope(),
                number_format_code: None,
            },
            K::SetNote => I::SetNote {
                node: key(),
                note: None,
            },
            K::CopyValues => I::CopyToClipboard {
                scope: scope(),
                payload: ClipboardPayloadKind::Values,
            },
            K::CopyFormula => I::CopyToClipboard {
                scope: scope(),
                payload: ClipboardPayloadKind::Formula,
            },
            K::CopyFormat => I::CopyToClipboard {
                scope: scope(),
                payload: ClipboardPayloadKind::Format,
            },
            K::CopySubtree => I::CopyToClipboard {
                scope: scope(),
                payload: ClipboardPayloadKind::Subtree,
            },
            K::PasteClipboardValues => I::PasteClipboardValues { target: scope() },
            K::PasteClipboardFormat => I::PasteClipboardFormat { target: scope() },
            K::OpenCandidate => I::OpenCandidate { parent: None },
            K::EvaluateCandidate => I::EvaluateCandidate {
                handle: "h".to_string(),
            },
            K::RebaseCandidate => I::RebaseCandidate {
                handle: "h".to_string(),
            },
            K::CommitCandidate => I::CommitCandidate {
                handle: "h".to_string(),
            },
            K::DiscardCandidate => I::DiscardCandidate {
                handle: "h".to_string(),
            },
            K::ReapCandidates => I::ReapCandidates { max_retained: 0 },
            K::CreateScenario => I::CreateScenario {
                scenario_id: "s".to_string(),
                name: "S".to_string(),
                base_scenario_id: None,
            },
            K::ActivateScenario => I::ActivateScenario { scenario_id: None },
            K::DeleteScenario => I::DeleteScenario {
                scenario_id: "s".to_string(),
            },
            K::CreateScenarioSweep => I::CreateScenarioSweep {
                sweep_id: "w".to_string(),
                name: "W".to_string(),
                base_scenario_id: None,
                input_node: key(),
                points: Vec::new(),
            },
            K::ActivateSweep => I::ActivateSweep { sweep_id: None },
            K::DeleteSweep => I::DeleteSweep {
                sweep_id: "w".to_string(),
            },
            K::Undo => I::Undo,
            K::Redo => I::Redo,
            K::EditTableCell => I::EditTableCell {
                table: node(),
                row_id: "r".to_string(),
                column_id: "c".to_string(),
                content: String::new(),
            },
            K::AddTableRow => I::AddTableRow {
                table: node(),
                row_id: "r".to_string(),
                values: Vec::new(),
            },
            K::AddTableColumn => I::AddTableColumn {
                table: node(),
                column_id: "c".to_string(),
                name: "C".to_string(),
                values: Vec::new(),
            },
        }
    }

    const ALL_KINDS: [CommandIntentKindProjection; 33] = {
        use CommandIntentKindProjection as K;
        [
            K::NewWorkspace,
            K::Recalculate,
            K::AddNode,
            K::RenameNode,
            K::MoveNode,
            K::ReorderNode,
            K::DeleteNode,
            K::EditContent,
            K::SetNumberFormat,
            K::SetNote,
            K::CopyValues,
            K::CopyFormula,
            K::CopyFormat,
            K::CopySubtree,
            K::PasteClipboardValues,
            K::PasteClipboardFormat,
            K::OpenCandidate,
            K::EvaluateCandidate,
            K::RebaseCandidate,
            K::CommitCandidate,
            K::DiscardCandidate,
            K::ReapCandidates,
            K::CreateScenario,
            K::ActivateScenario,
            K::DeleteScenario,
            K::CreateScenarioSweep,
            K::ActivateSweep,
            K::DeleteSweep,
            K::Undo,
            K::Redo,
            K::EditTableCell,
            K::AddTableRow,
            K::AddTableColumn,
        ]
    };

    #[test]
    fn permission_policy_is_consistent() {
        for persona in [Persona::Author, Persona::Reviewer, Persona::ReadOnly] {
            for kind in ALL_KINDS {
                let intent = representative_intent(kind);
                assert_eq!(
                    persona.allows_intent_kind(kind),
                    persona.allows(&intent),
                    "catalog gate diverges from dispatch gate for {persona} / {}",
                    kind.stable_id()
                );
            }
        }
    }

    #[test]
    fn reviewer_governs_the_load_bearing_cases() {
        let reviewer = Persona::Reviewer;
        // May review: recalc, annotate, copy, non-publishing speculation.
        assert!(reviewer.allows_intent_kind(CommandIntentKindProjection::Recalculate));
        assert!(reviewer.allows_intent_kind(CommandIntentKindProjection::SetNote));
        assert!(reviewer.allows_intent_kind(CommandIntentKindProjection::CopyValues));
        assert!(reviewer.allows_intent_kind(CommandIntentKindProjection::OpenCandidate));
        // Never publishes or mutates the model.
        assert!(!reviewer.allows_intent_kind(CommandIntentKindProjection::CommitCandidate));
        assert!(!reviewer.allows_intent_kind(CommandIntentKindProjection::EditContent));
        assert!(!reviewer.allows_intent_kind(CommandIntentKindProjection::DeleteNode));
        assert!(!reviewer.allows_intent_kind(CommandIntentKindProjection::CreateScenario));
    }

    #[test]
    fn read_only_forbids_every_command() {
        // The catalog has no pure-navigation kinds, so a read-only persona —
        // which may only select and switch workspaces — runs nothing.
        for kind in ALL_KINDS {
            assert!(!Persona::ReadOnly.allows_intent_kind(kind));
        }
    }
}
