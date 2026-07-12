//! SEAM-OXFML-R1C1-PUBLIC
//!
//! Target: reference style (A1 vs R1C1) becomes a public
//! `EditorPlanOptions.reference_style` enum and round-trips through
//! `EditorSettings` so the editor cluster renders the corresponding
//! formula text.

use super::common::seam_pending;

/// Pending SEAM-OXFML-R1C1-PUBLIC: flipping the reference style setting
/// should re-render the same formula text in R1C1 form on the editor
/// cluster.
#[test]
#[ignore = "pending SEAM-OXFML-R1C1-PUBLIC"]
fn reference_style_round_trips_a1_and_r1c1_through_editor_settings() {
    seam_pending(
        "SEAM-OXFML-R1C1-PUBLIC",
        "EditorSettings.reference_style must round-trip A1 ↔ R1C1 through the cluster",
    );
}
