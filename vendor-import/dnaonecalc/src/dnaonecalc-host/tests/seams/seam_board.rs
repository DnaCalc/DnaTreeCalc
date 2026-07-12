//! SEAM-ONECALC-SEAM-BOARD
//!
//! Target: the workspace settings page renders a seam status board that
//! lists every seam id referenced by an active formula space.

use super::common::seam_pending;

/// Pending SEAM-ONECALC-SEAM-BOARD: workspace-level state must carry a
/// typed registry of active seams referenced by formula spaces, and the
/// workspace settings cluster must project it.
///
/// Passes when a workspace with a formula space that carries a seam
/// reference (e.g. a format payload whose seam is pending) surfaces that
/// seam id on the seam board cluster with its status.
///
/// Ownership: Phase B step 10 (workspace settings page).
#[test]
#[ignore = "pending SEAM-ONECALC-SEAM-BOARD"]
fn workspace_seam_registry_lists_active_seams_referenced_by_spaces() {
    seam_pending(
        "SEAM-ONECALC-SEAM-BOARD",
        "workspace settings cluster must project a seam board built from the active seam registry",
    );
}
