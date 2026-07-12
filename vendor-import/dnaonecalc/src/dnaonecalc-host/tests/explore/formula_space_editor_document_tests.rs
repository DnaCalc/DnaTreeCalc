use dnaonecalc_host::adapters::oxfml::{
    EditorAnalysisStage, FormulaEditRequest, FormulaEditResult, OxfmlHostSession,
    OxfmlHostSessionError,
};
use dnaonecalc_host::app::intents::ApplyFormulaEditIntent;
use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::services::editor_session::EditorSessionService;
use dnaonecalc_host::state::{FormulaSpaceCollectionState, FormulaSpaceState};
use dnaonecalc_host::test_support::sample_editor_document;

#[test]
fn ex_02_editor_session_keeps_raw_cell_entry_text_and_oxfml_document() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut formula_spaces = FormulaSpaceCollectionState::default();
    formula_spaces.insert(FormulaSpaceState::new(
        formula_space_id.clone(),
        "=SUM(1,2,3)",
    ));

    EditorSessionService::apply_editor_document(
        &mut formula_spaces,
        &formula_space_id,
        sample_editor_document("'123.4"),
    )
    .expect("known formula space should update");

    let formula_space = formula_spaces.get(&formula_space_id).expect("space exists");
    assert_eq!(formula_space.raw_entered_cell_text, "'123.4");
    assert_eq!(
        formula_space
            .editor_document
            .as_ref()
            .expect("editor document retained")
            .editor_syntax_snapshot
            .green_tree_key,
        "green-1"
    );
    assert_eq!(formula_space.completion_help.completion_count, 1);
}

struct ExploreBridge {
    document: dnaonecalc_host::adapters::oxfml::EditorDocument,
}

impl OxfmlHostSession for ExploreBridge {
    fn apply_formula_edit(
        &self,
        request: FormulaEditRequest,
    ) -> Result<FormulaEditResult, OxfmlHostSessionError> {
        assert_eq!(request.formula_stable_id, "formula-2");
        assert_eq!(request.entered_text, "123.4");
        assert_eq!(request.cursor_offset, 5);
        assert_eq!(request.analysis_stage, EditorAnalysisStage::SyntaxOnly);
        Ok(FormulaEditResult {
            document: self.document.clone(),
        })
    }
}

#[test]
fn ex_03_formula_edit_intent_preserves_raw_cell_entry_path_for_direct_values() {
    let formula_space_id = FormulaSpaceId::new("space-2");
    let mut formula_spaces = FormulaSpaceCollectionState::default();
    formula_spaces.insert(FormulaSpaceState::new(formula_space_id.clone(), "1"));
    let bridge = ExploreBridge {
        document: sample_editor_document("123.4"),
    };

    EditorSessionService::handle_formula_edit_intent(
        &bridge,
        &mut formula_spaces,
        ApplyFormulaEditIntent {
            formula_space_id: formula_space_id.clone(),
            formula_stable_id: "formula-2".to_string(),
            entered_text: "123.4".to_string(),
            cursor_offset: 5,
            analysis_stage: EditorAnalysisStage::SyntaxOnly,
            formatting_request: None,
            scenario_policy: dnaonecalc_host::adapters::oxfml::ScenarioPolicyRequest::Deterministic,
            skip_runtime_evaluation: false,
            recalc_mode: dnaonecalc_host::adapters::oxfml::RecalcModeRequest::Auto,
            language_tag: "en-US".to_string(),
            trace_mode: dnaonecalc_host::adapters::oxfml::TraceModeRequest::ValueOnly,
        },
    )
    .expect("direct value edit should round-trip through bridge");

    let updated = formula_spaces.get(&formula_space_id).expect("space exists");
    assert_eq!(updated.raw_entered_cell_text, "123.4");
    assert_eq!(
        updated
            .editor_document
            .as_ref()
            .expect("editor document retained")
            .green_tree_key(),
        "green-1"
    );
}
