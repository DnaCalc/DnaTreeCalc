//! SEAM-ONECALC-FORMAT-PAYLOAD
//! SEAM-OXFML-FORMAT-PAYLOAD
//! SEAM-OXFUNC-FMT-GRAMMAR-VALIDATION
//! SEAM-OXFUNC-FMT-RED
//! SEAM-OXFUNC-FMT-CURRENCY
//! SEAM-OXFUNC-FMT-ACCOUNTING
//! SEAM-OXFUNC-FMT-ELAPSED
//! SEAM-OXFUNC-FMT-FRACTION
//! SEAM-OXFUNC-FMT-SCIENTIFIC
//! SEAM-OXFUNC-FMT-SPECIAL
//!
//! Target: a typed `CellFormatPayload` round-trips through `FormulaSpaceState`
//! and every format family renders through `FormatCodeEngine` to a cluster
//! field the Configure drawer Number tab reads.

use super::common::seam_pending;

/// Pending SEAM-ONECALC-FORMAT-PAYLOAD: formula space must carry a typed
/// `cell_format_payload` and the Configure drawer cluster must project it.
#[test]
#[ignore = "pending SEAM-ONECALC-FORMAT-PAYLOAD"]
fn format_payload_round_trips_through_formula_space_state() {
    seam_pending(
        "SEAM-ONECALC-FORMAT-PAYLOAD",
        "FormulaSpaceState must carry a typed cell_format_payload and project it to the cluster",
    );
}

/// Pending SEAM-OXFML-FORMAT-PAYLOAD: an `EditorDocument` carrying a
/// format payload on its provenance summary must surface the formatted
/// string on the cluster's `effective_display_summary`.
#[test]
#[ignore = "pending SEAM-OXFML-FORMAT-PAYLOAD"]
fn editor_document_format_payload_projects_display_format_code() {
    seam_pending(
        "SEAM-OXFML-FORMAT-PAYLOAD",
        "EditorDocument format payload must surface on the cluster's display_format_code field",
    );
}

/// Pending SEAM-OXFUNC-FMT-GRAMMAR-VALIDATION: an invalid format code must
/// produce a `FormatGrammarDiagnostic` that the Configure drawer format
/// input surfaces inline.
#[test]
#[ignore = "pending SEAM-OXFUNC-FMT-GRAMMAR-VALIDATION"]
fn invalid_format_code_emits_grammar_diagnostic_on_cluster() {
    seam_pending(
        "SEAM-OXFUNC-FMT-GRAMMAR-VALIDATION",
        "invalid format codes must emit FormatGrammarDiagnostic on the Configure drawer cluster",
    );
}

/// Pending SEAM-OXFUNC-FMT-RED: `[Red]` colour token must survive the
/// OneCalc projection so the cluster carries a typed colour annotation.
#[test]
#[ignore = "pending SEAM-OXFUNC-FMT-RED"]
fn red_colour_token_renders_with_typed_colour_annotation() {
    seam_pending(
        "SEAM-OXFUNC-FMT-RED",
        "[Red] format token must project as a typed colour annotation on the cluster",
    );
}

/// Pending SEAM-OXFUNC-FMT-CURRENCY: non-default currency symbols must
/// surface on the formatted string.
#[test]
#[ignore = "pending SEAM-OXFUNC-FMT-CURRENCY"]
fn non_default_currency_symbol_surfaces_on_effective_display() {
    seam_pending(
        "SEAM-OXFUNC-FMT-CURRENCY",
        "non-default currency symbols must render through the Configure drawer Number tab",
    );
}

/// Pending SEAM-OXFUNC-FMT-ACCOUNTING: accounting format family renders
/// with fixed accounting layout (no negative styles).
#[test]
#[ignore = "pending SEAM-OXFUNC-FMT-ACCOUNTING"]
fn accounting_format_family_renders_on_cluster() {
    seam_pending(
        "SEAM-OXFUNC-FMT-ACCOUNTING",
        "accounting format family must render on the Configure drawer Number tab cluster",
    );
}

/// Pending SEAM-OXFUNC-FMT-ELAPSED: `[h]`, `[m]`, `[s]` elapsed time
/// tokens must render.
#[test]
#[ignore = "pending SEAM-OXFUNC-FMT-ELAPSED"]
fn elapsed_time_tokens_render_on_cluster() {
    seam_pending(
        "SEAM-OXFUNC-FMT-ELAPSED",
        "[h]/[m]/[s] elapsed tokens must render on the Configure drawer Time tab",
    );
}

/// Pending SEAM-OXFUNC-FMT-FRACTION: fraction renderer for `# ?/?`,
/// `# ?/2`, `# ?/4`, `# ?/8`, `# ?/16`, `# ?/10`, `# ?/100`.
#[test]
#[ignore = "pending SEAM-OXFUNC-FMT-FRACTION"]
fn fraction_renderer_covers_every_supported_denominator() {
    seam_pending(
        "SEAM-OXFUNC-FMT-FRACTION",
        "fraction format family must render every denominator on the Configure drawer",
    );
}

/// Pending SEAM-OXFUNC-FMT-SCIENTIFIC: variable exponent width and the
/// `e+0` / `E-00` token variants must render.
#[test]
#[ignore = "pending SEAM-OXFUNC-FMT-SCIENTIFIC"]
fn scientific_format_renders_variable_exponent_width() {
    seam_pending(
        "SEAM-OXFUNC-FMT-SCIENTIFIC",
        "scientific format must render variable exponent width on the cluster",
    );
}

/// Pending SEAM-OXFUNC-FMT-SPECIAL: Special category renderer (Zip Code,
/// Zip+4, Phone Number, SSN, locale equivalents).
#[test]
#[ignore = "pending SEAM-OXFUNC-FMT-SPECIAL"]
fn special_category_renders_zip_phone_ssn_families() {
    seam_pending(
        "SEAM-OXFUNC-FMT-SPECIAL",
        "Special format category must render Zip/Phone/SSN families on the cluster",
    );
}
