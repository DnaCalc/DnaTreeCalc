//! The read-side preview seam — *predict before you pay* (tenet 7).
//!
//! A skin may ask, in the interrogative mood, what an intent **would** do —
//! dry-bind verdicts and mutation impact — without mutating anything. The
//! answers are computed by the layers that own the truth (OxFml dry-bind,
//! OxCalc invalidation planning, host collision/orphan joins) and surfaced
//! verbatim; the skin renders them and never re-derives legality itself.
//!
//! The service is optional on [`SkinContext`](crate::SkinContext): test
//! dispatchers and minimal hosts may omit it, and lenses degrade to the typed
//! post-attempt rejection channel (`IntentReceipt::error`).

use crate::identity::NodeId;
use crate::workspace::{
    FormulaBindPreviewProjection, MutationImpactIntentProjection, MutationImpactProjection,
};

/// Why a preview could not be computed. Previews failing is never an error a
/// lens escalates — it falls back to post-attempt feedback.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PreviewError {
    #[error("preview is not available in this host")]
    Unavailable,
    #[error("preview target {node} is unknown")]
    UnknownNode { node: String },
    #[error("host failed to compute the preview: {0}")]
    Host(String),
}

/// Non-mutating foresight over the closed intent surface.
///
/// Implementations must be pure observers: no engine mutation, no publication,
/// no projection republish may result from any call.
pub trait PreviewService: Send + Sync {
    /// Dry-bind prospective content in a node's context: syntax + bind
    /// diagnostics and capability-profile violations, typed.
    fn preview_formula_bind(
        &self,
        node: &NodeId,
        content: &str,
    ) -> Result<FormulaBindPreviewProjection, PreviewError>;

    /// Full legality + invalidation impact for a prospective mutation:
    /// what it would invalidate, collide with, rebind, or orphan, and the
    /// typed reason it would be blocked.
    fn preview_mutation_impact(
        &self,
        intent: &MutationImpactIntentProjection,
    ) -> Result<MutationImpactProjection, PreviewError>;
}
