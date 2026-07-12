//! SEAM-ONECALC-PARITY-MATRIX-VIEW
//!
//! Target: the home shell's compare drill-down exposes a structured parity
//! matrix (value / display / replay rows × scenario columns) rather than
//! flat outcome / evidence / lineage strings.

use super::common::seam_pending;

/// Pending SEAM-ONECALC-PARITY-MATRIX-VIEW: the compare drill-down view
/// model must build a typed parity matrix from a `RetainedArtifactRecord`,
/// not a flat string list.
///
/// Passes when the compare-drill view-model is populated with rows keyed by
/// parity axis (value / display / replay) and columns keyed by scenario
/// label, each cell carrying an explicit verdict enum.
///
/// Ownership: WS-14 Compare-with-Excel epic.
#[test]
#[ignore = "pending SEAM-ONECALC-PARITY-MATRIX-VIEW"]
fn compare_drill_view_model_exposes_structured_parity_matrix() {
    seam_pending(
        "SEAM-ONECALC-PARITY-MATRIX-VIEW",
        "Compare drill-down view-model must carry a structured ParityMatrix, not flat outcome strings",
    );
}

/// Pending SEAM-ONECALC-PARITY-MATRIX-VIEW: mismatched retained artifacts
/// must surface per-axis verdict cells that the drill-down can render with
/// colour and link.
#[test]
#[ignore = "pending SEAM-ONECALC-PARITY-MATRIX-VIEW"]
fn parity_matrix_cells_carry_verdict_enum_not_strings() {
    seam_pending(
        "SEAM-ONECALC-PARITY-MATRIX-VIEW",
        "ParityMatrix cells must carry a typed verdict enum (Matched / Mismatched / Blocked)",
    );
}
