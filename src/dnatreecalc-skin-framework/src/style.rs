//! The one styling language — shared visual invariants every lens obeys so
//! the suite reads as one tool, not seven.
//!
//! Three rules, codified once:
//! 1. `calc_state` is the **only saturated channel** on a node. These helpers
//!    map the typed [`NodeCalcStateProjection`] to a single class set.
//! 2. Provenance is **structural**, never ambiguous: published / pending /
//!    speculative / scenario / external get distinct *structural* tints.
//! 3. Authoring is **modeless 1-bit**: SELECTED vs EDITING is one border cue.
//!
//! The classes resolve through the existing `var(--dtc-...)` design tokens
//! (see [`crate::ThemeTokens`]); [`ATLAS_SPINE_CSS`] defines them. A lens
//! includes that CSS once and applies the class names these helpers return.

use crate::workspace::{NodeCalcStateProjection, NodeValueProjection, NodeView, WorkspaceState};

/// Map a node's calc-state to its (single, saturated) status class.
///
/// `calc_state` is the only channel allowed to carry saturated color in any
/// lens; everything else is structural.
#[must_use]
pub fn calc_state_class(state: Option<NodeCalcStateProjection>) -> &'static str {
    match state {
        Some(NodeCalcStateProjection::Clean | NodeCalcStateProjection::VerifiedClean) => {
            "dtc-calc--clean"
        }
        Some(NodeCalcStateProjection::DirtyPending | NodeCalcStateProjection::Needed) => {
            "dtc-calc--stale"
        }
        Some(NodeCalcStateProjection::Evaluating) => "dtc-calc--evaluating",
        Some(NodeCalcStateProjection::PublishReady) => "dtc-calc--ready",
        Some(NodeCalcStateProjection::RejectedPendingRepair) => "dtc-calc--rejected",
        Some(NodeCalcStateProjection::CycleBlocked) => "dtc-calc--cycle",
        None => "dtc-calc--unknown",
    }
}

/// Where a projected value came from — never decorative, always structural.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceTint {
    Published,
    Pending,
    Speculative,
    Scenario,
    External,
}

impl ProvenanceTint {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Published => "published",
            Self::Pending => "pending",
            Self::Speculative => "speculative",
            Self::Scenario => "scenario",
            Self::External => "external",
        }
    }

    #[must_use]
    pub const fn css_class(self) -> &'static str {
        match self {
            Self::Published => "dtc-prov--published",
            Self::Pending => "dtc-prov--pending",
            Self::Speculative => "dtc-prov--speculative",
            Self::Scenario => "dtc-prov--scenario",
            Self::External => "dtc-prov--external",
        }
    }
}

/// Classify a published node's provenance for tinting.
///
/// A node read from `WorkspaceState.nodes` is `Published` unless an active
/// scenario overrides it (`Scenario`) or its value is still in flight
/// (`Pending`). `Speculative`/`External` apply when rendering candidate or
/// external values respectively and are exposed for those call sites.
#[must_use]
pub fn provenance_tint(node: &NodeView, workspace: &WorkspaceState) -> ProvenanceTint {
    if node.scenario_override.is_some() && workspace.scenarios.active.is_some() {
        return ProvenanceTint::Scenario;
    }
    match node.computed_value {
        NodeValueProjection::Pending | NodeValueProjection::Unevaluated => ProvenanceTint::Pending,
        _ => ProvenanceTint::Published,
    }
}

/// The modeless 1-bit border cue. `editing` wins over `selected`.
#[must_use]
pub fn selection_mode_class(is_selected: bool, is_editing: bool) -> &'static str {
    if is_editing {
        "dtc-sel--editing"
    } else if is_selected {
        "dtc-sel--selected"
    } else {
        ""
    }
}

/// Shared CSS for the spine classes. A lens includes this once. Every rule
/// resolves through the existing design tokens so it themes automatically.
pub const ATLAS_SPINE_CSS: &str = r#"
.dtc-calc--clean { --dtc-calc-color: var(--dtc-success); }
.dtc-calc--stale { --dtc-calc-color: var(--dtc-warning); }
.dtc-calc--evaluating { --dtc-calc-color: var(--dtc-accent); }
.dtc-calc--ready { --dtc-calc-color: var(--dtc-accent); }
.dtc-calc--rejected { --dtc-calc-color: var(--dtc-danger); }
.dtc-calc--cycle { --dtc-calc-color: var(--dtc-danger); }
.dtc-calc--unknown { --dtc-calc-color: var(--dtc-text-subtle); }
.dtc-calc-dot {
  display: inline-block; width: 0.6em; height: 0.6em; border-radius: 50%;
  background: var(--dtc-calc-color, var(--dtc-text-subtle));
}
.dtc-calc-bar { box-shadow: inset 3px 0 0 0 var(--dtc-calc-color, transparent); }

.dtc-prov--published { opacity: 1; }
.dtc-prov--pending { opacity: 0.55; font-style: italic; }
.dtc-prov--speculative { opacity: 0.8; border-style: dashed; }
.dtc-prov--scenario { text-decoration: underline dotted var(--dtc-accent); }
.dtc-prov--external { opacity: 0.85; font-style: italic; }

.dtc-sel--selected { box-shadow: inset 0 0 0 2px var(--dtc-accent); }
.dtc-sel--editing { box-shadow: inset 0 0 0 2px var(--dtc-warning); }
"#;
