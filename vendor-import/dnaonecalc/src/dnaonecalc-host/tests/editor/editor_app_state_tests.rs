use dnaonecalc_host::app::reducer::{
    apply_editor_command_to_active_formula_space, apply_editor_input_to_active_formula_space,
};
use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::state::{FormulaSpaceState, OneCalcHostState};
use dnaonecalc_host::test_support::sample_editor_document;
use dnaonecalc_host::ui::editor::commands::{EditorCommand, EditorInputEvent, EditorInputKind};
use dnaonecalc_host::ui::editor::state::EditorSelection;

#[test]
fn ex_15_editor_input_event_updates_active_formula_space_in_host_state() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state.formula_spaces.insert(FormulaSpaceState::new(
        formula_space_id.clone(),
        "=SUM(1,2)",
    ));

    let changed = apply_editor_input_to_active_formula_space(
        &mut state,
        EditorInputEvent {
            text: "=SUM(1,2,3)".to_string(),
            selection_start: None,
            selection_end: None,
            input_kind: EditorInputKind::Other,
            inserted_text: None,
        },
    );

    assert!(changed);
    let active = state
        .formula_spaces
        .get(&formula_space_id)
        .expect("space exists");
    assert_eq!(active.raw_entered_cell_text, "=SUM(1,2,3)");
    assert_eq!(active.editor_surface_state.caret.offset, 11);
    assert!(active.editor_document.is_none());
}

#[test]
fn ex_20_editor_input_event_preserves_selection_offsets_in_host_state() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state.formula_spaces.insert(FormulaSpaceState::new(
        formula_space_id.clone(),
        "=SUM(1,2)",
    ));

    let changed = apply_editor_input_to_active_formula_space(
        &mut state,
        EditorInputEvent {
            text: "=SUM(1,2)".to_string(),
            selection_start: Some(2),
            selection_end: Some(5),
            input_kind: EditorInputKind::InsertText,
            inserted_text: Some("M".to_string()),
        },
    );

    assert!(changed);
    let active = state
        .formula_spaces
        .get(&formula_space_id)
        .expect("space exists");
    assert_eq!(active.editor_surface_state.selection.anchor, 2);
    assert_eq!(active.editor_surface_state.selection.focus, 5);
    assert_eq!(active.editor_surface_state.caret.offset, 5);
}

#[test]
fn ex_16_editor_command_updates_caret_and_selection_in_host_state() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state.formula_spaces.insert(FormulaSpaceState::new(
        formula_space_id.clone(),
        "=SUM(1,2)",
    ));

    let changed =
        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::MoveCaretLeft);

    assert!(changed);
    let active = state
        .formula_spaces
        .get(&formula_space_id)
        .expect("space exists");
    assert_eq!(active.editor_surface_state.caret.offset, 8);
    assert!(active.editor_surface_state.selection.is_collapsed());
}

#[test]
fn ex_18_shift_arrow_command_expands_selection_in_host_state() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state.formula_spaces.insert(FormulaSpaceState::new(
        formula_space_id.clone(),
        "=SUM(1,2)",
    ));

    let changed = apply_editor_command_to_active_formula_space(
        &mut state,
        EditorCommand::ExtendSelectionLeft,
    );

    assert!(changed);
    let active = state
        .formula_spaces
        .get(&formula_space_id)
        .expect("space exists");
    assert_eq!(active.editor_surface_state.caret.offset, 8);
    assert_eq!(active.editor_surface_state.selection.anchor, 9);
    assert_eq!(active.editor_surface_state.selection.focus, 8);
    assert!(!active.editor_surface_state.selection.is_collapsed());
}

#[test]
fn ex_23_insert_text_command_replaces_selected_range_in_host_state() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
    formula_space.editor_surface_state.selection = EditorSelection {
        anchor: 1,
        focus: 4,
    };
    formula_space.editor_surface_state.caret.offset = 4;
    state.formula_spaces.insert(formula_space);

    let changed = apply_editor_command_to_active_formula_space(
        &mut state,
        EditorCommand::InsertText("AVG".to_string()),
    );

    assert!(changed);
    let active = state
        .formula_spaces
        .get(&formula_space_id)
        .expect("space exists");
    assert_eq!(active.raw_entered_cell_text, "=AVG(1,2)");
    assert_eq!(active.editor_surface_state.caret.offset, 4);
    assert!(active.editor_surface_state.selection.is_collapsed());
}

#[test]
fn ex_24_delete_command_removes_current_selection_in_host_state() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
    formula_space.editor_surface_state.selection = EditorSelection {
        anchor: 1,
        focus: 4,
    };
    formula_space.editor_surface_state.caret.offset = 4;
    state.formula_spaces.insert(formula_space);

    let changed = apply_editor_command_to_active_formula_space(&mut state, EditorCommand::Delete);

    assert!(changed);
    let active = state
        .formula_spaces
        .get(&formula_space_id)
        .expect("space exists");
    assert_eq!(active.raw_entered_cell_text, "=(1,2)");
    assert_eq!(active.editor_surface_state.caret.offset, 1);
    assert!(active.editor_surface_state.selection.is_collapsed());
}

#[test]
fn ex_25_completion_navigation_updates_selected_proposal_index() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SUM(1,2)");
    let mut editor_document = sample_editor_document("=SUM(1,2)");
    editor_document.completion_proposals.push(
        dnaonecalc_host::adapters::oxfml::CompletionProposal {
            proposal_id: "proposal-2".to_string(),
            proposal_kind: dnaonecalc_host::adapters::oxfml::CompletionProposalKind::Function,
            display_text: "SUBTOTAL".to_string(),
            insert_text: "SUBTOTAL(".to_string(),
            replacement_span: Some(dnaonecalc_host::adapters::oxfml::FormulaTextSpan {
                start: 1,
                len: 3,
            }),
            documentation_ref: Some("preview:function:SUBTOTAL".to_string()),
            requires_revalidation: true,
        },
    );
    formula_space.editor_document = Some(editor_document);
    formula_space.editor_surface_state.completion_selected_index = Some(0);
    state.formula_spaces.insert(formula_space);

    let changed = apply_editor_command_to_active_formula_space(
        &mut state,
        EditorCommand::SelectNextCompletion,
    );

    assert!(changed);
    let active = state
        .formula_spaces
        .get(&formula_space_id)
        .expect("space exists");
    assert_eq!(
        active.editor_surface_state.completion_selected_index,
        Some(1)
    );
}

#[test]
fn ex_26_accept_completion_replaces_current_prefix_range() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SU");
    let mut editor_document = sample_editor_document("=SU");
    editor_document.completion_proposals[0].display_text = "SUM".to_string();
    editor_document.completion_proposals[0].insert_text = "SUM(".to_string();
    editor_document.completion_proposals[0].replacement_span =
        Some(dnaonecalc_host::adapters::oxfml::FormulaTextSpan { start: 1, len: 2 });
    formula_space.editor_document = Some(editor_document);
    formula_space.editor_surface_state.completion_selected_index = Some(0);
    state.formula_spaces.insert(formula_space);

    let changed = apply_editor_command_to_active_formula_space(
        &mut state,
        EditorCommand::AcceptSelectedCompletion,
    );

    assert!(changed);
    let active = state
        .formula_spaces
        .get(&formula_space_id)
        .expect("space exists");
    assert_eq!(active.raw_entered_cell_text, "=SUM(");
    assert_eq!(active.editor_surface_state.caret.offset, 5);
    assert!(active.editor_surface_state.selection.is_collapsed());
    assert!(active.editor_document.is_none());
}

#[test]
fn ex_27_accept_completion_by_index_updates_selection_and_replaces_text() {
    let formula_space_id = FormulaSpaceId::new("space-1");
    let mut state = OneCalcHostState::default();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SU");
    let mut editor_document = sample_editor_document("=SU");
    editor_document.completion_proposals = vec![
        dnaonecalc_host::adapters::oxfml::CompletionProposal {
            proposal_id: "proposal-1".to_string(),
            proposal_kind: dnaonecalc_host::adapters::oxfml::CompletionProposalKind::Function,
            display_text: "SUM".to_string(),
            insert_text: "SUM(".to_string(),
            replacement_span: Some(dnaonecalc_host::adapters::oxfml::FormulaTextSpan {
                start: 1,
                len: 2,
            }),
            documentation_ref: Some("preview:function:SUM".to_string()),
            requires_revalidation: true,
        },
        dnaonecalc_host::adapters::oxfml::CompletionProposal {
            proposal_id: "proposal-2".to_string(),
            proposal_kind: dnaonecalc_host::adapters::oxfml::CompletionProposalKind::Function,
            display_text: "SUBTOTAL".to_string(),
            insert_text: "SUBTOTAL(".to_string(),
            replacement_span: Some(dnaonecalc_host::adapters::oxfml::FormulaTextSpan {
                start: 1,
                len: 2,
            }),
            documentation_ref: Some("preview:function:SUBTOTAL".to_string()),
            requires_revalidation: true,
        },
    ];
    formula_space.editor_document = Some(editor_document);
    state.formula_spaces.insert(formula_space);

    let changed = apply_editor_command_to_active_formula_space(
        &mut state,
        EditorCommand::AcceptCompletionByIndex(1),
    );

    assert!(changed);
    let active = state
        .formula_spaces
        .get(&formula_space_id)
        .expect("space exists");
    assert_eq!(active.raw_entered_cell_text, "=SUBTOTAL(");
    assert_eq!(active.editor_surface_state.caret.offset, 10);
}
