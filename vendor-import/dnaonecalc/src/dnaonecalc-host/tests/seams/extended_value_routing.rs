//! SEAM-ONECALC-EXTENDED-VALUE-ROUTING
//! SEAM-ONECALC-EXCEL-VALUE-INTO-EXTENDED
//!
//! Target: route upstream `CalcValue` / presentation hints through
//! `FormulaSpaceState` so the result cluster renders structurally. The
//! former OxFunc `ExtendedValue` carrier was removed by W099; OneCalc's
//! live typed path now pins the native `CalcValue` carrier directly.

use super::common::seam_pending;
use dnaonecalc_host::adapters::oxfml::{
    CalcValue, EditorAnalysisStage, NativeOxfmlHostSession, RecalcModeRequest,
    ScenarioPolicyRequest, TraceModeRequest,
};
use dnaonecalc_host::app::intents::ApplyFormulaEditIntent;
use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::services::editor_session::EditorSessionService;
use dnaonecalc_host::state::{
    FormulaSpaceCollectionState, FormulaSpaceState, ProjectionTruthSource,
};

#[test]
fn value_presentation_carries_typed_number_after_sum_round_trip() {
    let formula_space = edit_formula("typed-number", "=SUM(1,2,3)");
    let presentation = formula_space
        .editor_document
        .as_ref()
        .and_then(|document| document.value_presentation.as_ref())
        .expect("SUM should carry a live value presentation");

    assert_eq!(presentation.published_value, CalcValue::number(6.0));
    assert_eq!(
        formula_space.latest_evaluation_summary.as_deref(),
        Some("Number · 6")
    );
    assert_eq!(
        formula_space.effective_display_summary.as_deref(),
        Some("6")
    );
    assert_eq!(
        formula_space.context.truth_source,
        ProjectionTruthSource::LiveBacked
    );
}

#[test]
fn value_presentation_carries_typed_array_after_sequence_round_trip() {
    let formula_space = edit_formula("typed-array", "=SEQUENCE(2,2)");
    let presentation = formula_space
        .editor_document
        .as_ref()
        .and_then(|document| document.value_presentation.as_ref())
        .expect("SEQUENCE should carry a live value presentation");
    let array = presentation
        .published_value
        .as_array()
        .expect("SEQUENCE should publish a CalcValue array");

    assert_eq!(array.shape().rows, 2);
    assert_eq!(array.shape().cols, 2);
    assert_eq!(
        array.get(0, 0).and_then(|value| value.as_number()),
        Some(1.0)
    );
    assert_eq!(
        array.get(0, 1).and_then(|value| value.as_number()),
        Some(2.0)
    );
    assert_eq!(
        array.get(1, 0).and_then(|value| value.as_number()),
        Some(3.0)
    );
    assert_eq!(
        array.get(1, 1).and_then(|value| value.as_number()),
        Some(4.0)
    );
    assert_eq!(
        formula_space
            .array_preview
            .as_ref()
            .map(|preview| preview.rows.clone()),
        Some(vec![
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string(), "4".to_string()],
        ])
    );
}

/// Pending SEAM-ONECALC-EXCEL-VALUE-INTO-EXTENDED: `RetainedArtifactRecord`
/// carries Excel observations as `serde_json::Value`; an adapter should
/// lift them into the native typed value carrier so the Value Panel renders
/// structurally.
///
/// Passes when opening an Excel-sourced artifact surfaces a typed value on
/// the Workbench projection instead of raw JSON.
#[test]
#[ignore = "pending SEAM-ONECALC-EXCEL-VALUE-INTO-EXTENDED"]
fn excel_observation_summary_lifts_into_extended_value() {
    seam_pending(
        "SEAM-ONECALC-EXCEL-VALUE-INTO-EXTENDED",
        "excel_comparison_value must lift into a typed value for projection",
    );
}

fn edit_formula(formula_stable_id: &str, entered_text: &str) -> FormulaSpaceState {
    let formula_space_id = FormulaSpaceId::new(format!("space-{formula_stable_id}"));
    let mut formula_spaces = FormulaSpaceCollectionState::default();
    formula_spaces.insert(FormulaSpaceState::new(
        formula_space_id.clone(),
        entered_text,
    ));
    let bridge = NativeOxfmlHostSession::default();

    EditorSessionService::handle_formula_edit_intent(
        &bridge,
        &mut formula_spaces,
        ApplyFormulaEditIntent {
            formula_space_id: formula_space_id.clone(),
            formula_stable_id: formula_stable_id.to_string(),
            entered_text: entered_text.to_string(),
            cursor_offset: entered_text.len(),
            analysis_stage: EditorAnalysisStage::SyntaxAndBind,
            formatting_request: None,
            scenario_policy: ScenarioPolicyRequest::Deterministic,
            skip_runtime_evaluation: false,
            recalc_mode: RecalcModeRequest::Auto,
            language_tag: "en-US".to_string(),
            trace_mode: TraceModeRequest::PreparedCalls,
        },
    )
    .expect("live OxFml edit should update formula space");

    formula_spaces
        .get(&formula_space_id)
        .expect("formula space should remain present")
        .clone()
}
