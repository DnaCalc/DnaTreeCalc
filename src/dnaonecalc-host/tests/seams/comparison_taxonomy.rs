//! SEAM-OXFML-COMPARISON-VIEW-TAXONOMY
//!
//! Target: the `view_family` strings produced by `ReplayComparisonView`
//! carry a stable taxonomy so the Parity Matrix can render family-specific
//! UI. Today the set is implicit.

use super::common::seam_pending;

#[test]
#[ignore = "pending SEAM-OXFML-COMPARISON-VIEW-TAXONOMY"]
fn workbench_groups_mismatches_by_taxonomy_tag() {
    seam_pending(
        "SEAM-OXFML-COMPARISON-VIEW-TAXONOMY",
        "Workbench projection must group OxReplayMismatchRecord entries by a typed taxonomy tag",
    );
}
