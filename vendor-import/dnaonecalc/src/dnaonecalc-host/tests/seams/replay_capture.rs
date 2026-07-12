//! SEAM-OXXLPLAY-CAPTURE-FONT
//! SEAM-OXXLPLAY-CAPTURE-BORDER
//! SEAM-OXXLPLAY-CAPTURE-ALIGNMENT
//! SEAM-OXXLPLAY-CAPTURE-PROTECTION
//! SEAM-OXXLPLAY-CAPTURE-CF-VISUAL
//! SEAM-OXXLPLAY-INPUT-CONTEXT
//!
//! Target: OxXlPlay capture surfaces (font / border / alignment /
//! protection / CF visual) and the `input_context` payload round-trip
//! through verification bundle records so Workbench's parity matrix can
//! project them.

use super::common::seam_pending;

#[test]
#[ignore = "pending SEAM-OXXLPLAY-CAPTURE-FONT"]
fn replay_record_with_captured_font_projects_to_parity_matrix() {
    seam_pending(
        "SEAM-OXXLPLAY-CAPTURE-FONT",
        "Replay records with captured font must project to the Workbench parity matrix",
    );
}

#[test]
#[ignore = "pending SEAM-OXXLPLAY-CAPTURE-BORDER"]
fn replay_record_with_captured_border_projects_to_parity_matrix() {
    seam_pending(
        "SEAM-OXXLPLAY-CAPTURE-BORDER",
        "Replay records with captured border must project to the Workbench parity matrix",
    );
}

#[test]
#[ignore = "pending SEAM-OXXLPLAY-CAPTURE-ALIGNMENT"]
fn replay_record_with_captured_alignment_projects_to_parity_matrix() {
    seam_pending(
        "SEAM-OXXLPLAY-CAPTURE-ALIGNMENT",
        "Replay records with captured alignment must project to the Workbench parity matrix",
    );
}

#[test]
#[ignore = "pending SEAM-OXXLPLAY-CAPTURE-PROTECTION"]
fn replay_record_with_captured_protection_projects_to_parity_matrix() {
    seam_pending(
        "SEAM-OXXLPLAY-CAPTURE-PROTECTION",
        "Replay records with captured protection must project to the Workbench parity matrix",
    );
}

#[test]
#[ignore = "pending SEAM-OXXLPLAY-CAPTURE-CF-VISUAL"]
fn replay_record_with_captured_cf_visual_projects_to_parity_matrix() {
    seam_pending(
        "SEAM-OXXLPLAY-CAPTURE-CF-VISUAL",
        "Replay records with captured CF visuals must project to the Workbench parity matrix",
    );
}

#[test]
#[ignore = "pending SEAM-OXXLPLAY-INPUT-CONTEXT"]
fn verification_bundle_round_trips_formula_space_context_payload() {
    seam_pending(
        "SEAM-OXXLPLAY-INPUT-CONTEXT",
        "Verification bundle must round-trip a FormulaSpaceContext-shaped input_context payload",
    );
}
