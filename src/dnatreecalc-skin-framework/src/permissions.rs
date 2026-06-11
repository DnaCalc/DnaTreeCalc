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
                I::SelectNode(_) | I::SelectTableCell { .. } | I::SwitchWorkspace { .. }
            ),
        }
    }
}

impl std::fmt::Display for Persona {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.stable_id())
    }
}
