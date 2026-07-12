use dnaonecalc_host::ui::editor::commands::{
    apply_editor_command, keydown_to_command, EditorCommand, EditorKeyContext,
};
use dnaonecalc_host::ui::editor::state::{
    EditorCaret, EditorScrollWindow, EditorSelection, EditorSurfaceState,
};

#[test]
fn ex_12_indent_command_inserts_spaces_for_multiline_selection() {
    let text = "SUM(\n1,\n2)";
    let state = EditorSurfaceState {
        caret: EditorCaret { offset: 0 },
        selection: EditorSelection {
            anchor: 0,
            focus: text.chars().count(),
        },
        scroll_window: EditorScrollWindow {
            first_visible_line: 0,
            visible_line_count: 6,
        },
        completion_anchor_offset: None,
        completion_selected_index: None,
        signature_help_anchor_offset: None,
    };

    let result = apply_editor_command(text, &state, EditorCommand::IndentWithSpaces);
    assert!(result.text.starts_with("    SUM("));
    assert!(result.text.contains("\n    1,"));
    assert!(result.text.contains("\n    2)"));
}

#[test]
fn ex_13_outdent_command_removes_spaces_for_multiline_selection() {
    let text = "    SUM(\n    1,\n    2)";
    let state = EditorSurfaceState {
        caret: EditorCaret { offset: 4 },
        selection: EditorSelection {
            anchor: 0,
            focus: text.chars().count(),
        },
        scroll_window: EditorScrollWindow {
            first_visible_line: 0,
            visible_line_count: 6,
        },
        completion_anchor_offset: None,
        completion_selected_index: None,
        signature_help_anchor_offset: None,
    };

    let result = apply_editor_command(text, &state, EditorCommand::OutdentWithSpaces);
    assert!(result.text.starts_with("SUM("));
    assert!(result.text.contains("\n1,"));
    assert!(result.text.contains("\n2)"));
}

#[test]
fn ex_14_keydown_mapping_intercepts_only_editor_owned_keys() {
    let ctx = EditorKeyContext::default();
    // Caret navigation, deletion, and plain Enter fall through so the native
    // textarea owns cursor tracking. See ui::editor::commands::keydown_to_command.
    assert_eq!(
        keydown_to_command("ArrowLeft", false, false, false, ctx),
        None
    );
    assert_eq!(
        keydown_to_command("ArrowRight", false, false, false, ctx),
        None
    );
    assert_eq!(
        keydown_to_command("ArrowLeft", true, false, false, ctx),
        None
    );
    assert_eq!(
        keydown_to_command("ArrowRight", true, false, false, ctx),
        None
    );
    assert_eq!(
        keydown_to_command("Backspace", false, false, false, ctx),
        None
    );
    assert_eq!(keydown_to_command("Delete", false, false, false, ctx), None);
    assert_eq!(keydown_to_command("Enter", false, false, false, ctx), None);
    assert_eq!(keydown_to_command("x", false, true, false, ctx), None);

    // Editor-owned chords.
    assert_eq!(
        keydown_to_command("Tab", false, false, false, ctx),
        Some(EditorCommand::IndentWithSpaces)
    );
    assert_eq!(
        keydown_to_command("Tab", true, false, false, ctx),
        Some(EditorCommand::OutdentWithSpaces)
    );
    assert_eq!(
        keydown_to_command("Escape", false, false, false, ctx),
        Some(EditorCommand::CancelEntry)
    );
    assert_eq!(
        keydown_to_command("Enter", false, true, false, ctx),
        Some(EditorCommand::RequestProof)
    );
}

#[test]
fn ex_17_shift_arrow_commands_expand_and_contract_selection() {
    let text = "=SUM(1,2)";
    let state = EditorSurfaceState {
        caret: EditorCaret { offset: 4 },
        selection: EditorSelection::collapsed(4),
        scroll_window: EditorScrollWindow {
            first_visible_line: 0,
            visible_line_count: 6,
        },
        completion_anchor_offset: None,
        completion_selected_index: None,
        signature_help_anchor_offset: None,
    };

    let left = apply_editor_command(text, &state, EditorCommand::ExtendSelectionLeft);
    assert_eq!(left.state.selection.anchor, 4);
    assert_eq!(left.state.selection.focus, 3);
    assert!(!left.state.selection.is_collapsed());

    let right = apply_editor_command(text, &left.state, EditorCommand::ExtendSelectionRight);
    assert_eq!(right.state.selection.anchor, 4);
    assert_eq!(right.state.selection.focus, 4);
    assert!(right.state.selection.is_collapsed());
}

#[test]
fn ex_21_insert_text_replaces_non_collapsed_selection() {
    let text = "=SUM(1,2)";
    let state = EditorSurfaceState {
        caret: EditorCaret { offset: 5 },
        selection: EditorSelection {
            anchor: 1,
            focus: 4,
        },
        scroll_window: EditorScrollWindow {
            first_visible_line: 0,
            visible_line_count: 6,
        },
        completion_anchor_offset: Some(4),
        completion_selected_index: Some(0),
        signature_help_anchor_offset: Some(4),
    };

    let result = apply_editor_command(text, &state, EditorCommand::InsertText("AVG".to_string()));
    assert_eq!(result.text, "=AVG(1,2)");
    assert_eq!(result.state.caret.offset, 4);
    assert!(result.state.selection.is_collapsed());
}

#[test]
fn ex_22_backspace_and_delete_respect_selection_and_caret() {
    let text = "=SUM(1,2)";
    let selected = EditorSurfaceState {
        caret: EditorCaret { offset: 5 },
        selection: EditorSelection {
            anchor: 1,
            focus: 4,
        },
        scroll_window: EditorScrollWindow {
            first_visible_line: 0,
            visible_line_count: 6,
        },
        completion_anchor_offset: None,
        completion_selected_index: None,
        signature_help_anchor_offset: None,
    };

    let backspace = apply_editor_command(text, &selected, EditorCommand::Backspace);
    assert_eq!(backspace.text, "=(1,2)");
    assert_eq!(backspace.state.caret.offset, 1);

    let collapsed = EditorSurfaceState {
        caret: EditorCaret { offset: 1 },
        selection: EditorSelection::collapsed(1),
        scroll_window: EditorScrollWindow {
            first_visible_line: 0,
            visible_line_count: 6,
        },
        completion_anchor_offset: None,
        completion_selected_index: None,
        signature_help_anchor_offset: None,
    };

    let delete = apply_editor_command(text, &collapsed, EditorCommand::Delete);
    assert_eq!(delete.text, "=UM(1,2)");
    assert_eq!(delete.state.caret.offset, 1);
}

#[test]
fn ex_23_cut_selection_removes_selected_text_without_moving_focus_state() {
    let text = "=SUM(1,2)";
    let selected = EditorSurfaceState {
        caret: EditorCaret { offset: 4 },
        selection: EditorSelection {
            anchor: 1,
            focus: 4,
        },
        scroll_window: EditorScrollWindow {
            first_visible_line: 0,
            visible_line_count: 6,
        },
        completion_anchor_offset: None,
        completion_selected_index: None,
        signature_help_anchor_offset: None,
    };

    let cut = apply_editor_command(text, &selected, EditorCommand::CutSelection);
    assert_eq!(cut.text, "=(1,2)");
    assert_eq!(cut.state.caret.offset, 1);
    assert!(cut.state.selection.is_collapsed());
}
