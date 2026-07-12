//! SEAM-ONECALC-VERIFICATION-BUNDLE-CONTEXT
//!
//! Target: `services/verification_bundle.rs` round-trips `CellFormatPayload`
//! and `CalcOptionsPayload` on imported records even though no current
//! consumer reads them.

use super::common::seam_pending;

/// Pending SEAM-ONECALC-VERIFICATION-BUNDLE-CONTEXT: a verification bundle
/// report that carries a `cell_format_payload` must round-trip that field
/// onto the `RetainedArtifactRecord`.
///
/// Passes when importing a report with `cell_format_payload: { number_format_code: "$#,##0.00" }`
/// surfaces that payload on the retained artifact record.
#[test]
#[ignore = "pending SEAM-ONECALC-VERIFICATION-BUNDLE-CONTEXT"]
fn bundle_import_round_trips_cell_format_payload() {
    seam_pending(
        "SEAM-ONECALC-VERIFICATION-BUNDLE-CONTEXT",
        "import_verification_bundle_report_json must round-trip cell_format_payload",
    );
}

/// Pending SEAM-ONECALC-VERIFICATION-BUNDLE-CONTEXT: the bundle import
/// must also round-trip `CalcOptionsPayload` so the Configure drawer
/// Scenario Settings tab has input to work against.
#[test]
#[ignore = "pending SEAM-ONECALC-VERIFICATION-BUNDLE-CONTEXT"]
fn bundle_import_round_trips_calc_options_payload() {
    seam_pending(
        "SEAM-ONECALC-VERIFICATION-BUNDLE-CONTEXT",
        "import_verification_bundle_report_json must round-trip calc_options_payload",
    );
}
