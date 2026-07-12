//! SEAM-OXFML-PRESENTATION-PROPAGATION
//!
//! Target: every function whose output has a natural presentation
//! classification (TEXT, DOLLAR, FIXED, NOW, TODAY, DATE, TIME, PERCENT
//! operators, HYPERLINK, etc.) emits a `CalcValue` presentation hint. This
//! unlocks pre-fill behaviour in the Configure drawer Number tab and the
//! Value Panel's Presentation section.

use super::common::seam_pending;

#[test]
#[ignore = "pending SEAM-OXFML-PRESENTATION-PROPAGATION"]
fn child_walk_nodes_inherit_presentation_from_parent() {
    seam_pending(
        "SEAM-OXFML-PRESENTATION-PROPAGATION",
        "Inspect walk nodes must inherit presentation from their parent function output",
    );
}
