use crate::ui::editor::state::{EditorCaret, EditorSelection, EditorSurfaceState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorCommand {
    InsertText(String),
    InsertNewline,
    CutSelection,
    MoveCaretLeft,
    MoveCaretRight,
    ExtendSelectionLeft,
    ExtendSelectionRight,
    SelectPreviousCompletion,
    SelectNextCompletion,
    SelectCompletionByIndex(usize),
    AcceptSelectedCompletion,
    AcceptCompletionByIndex(usize),
    Backspace,
    Delete,
    IndentWithSpaces,
    OutdentWithSpaces,
    CommitEntry,
    CancelEntry,
    RequestProof,
    ForceShowCompletion,
    DismissCompletion,
    CycleReferenceForm,
    ToggleExpandedHeight,
    SendSelectionToInspect,
    ToggleEditorSettingsPopover,
    UpdateEditorSetting(crate::ui::editor::state::EditorSettingUpdate),
    ToggleConfigureDrawer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EditorKeyContext {
    pub completion_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorInputKind {
    InsertText,
    DeleteBackward,
    DeleteForward,
    InsertFromPaste,
    /// Synthetic event the host emits when the caret moves without
    /// the text changing (mouse click, arrow key, Home / End,
    /// PageUp / PageDown). The bridge skips runtime evaluation for
    /// these — the formula's value cannot have changed, so popups
    /// + signature help refresh against the new caret position
    /// without paying the runtime cost.
    CaretSync,
    Other,
}

impl EditorInputKind {
    /// Whether this input kind changes the formula text. Caret-only
    /// events return `false`; all other variants return `true`.
    pub fn mutates_text(self) -> bool {
        !matches!(self, Self::CaretSync)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorInputEvent {
    pub text: String,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
    pub input_kind: EditorInputKind,
    pub inserted_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorCommandResult {
    pub text: String,
    pub state: EditorSurfaceState,
}

pub fn apply_editor_command(
    text: &str,
    state: &EditorSurfaceState,
    command: EditorCommand,
) -> EditorCommandResult {
    match command {
        EditorCommand::InsertText(inserted_text) => insert_text(text, state, &inserted_text),
        EditorCommand::InsertNewline => insert_text(text, state, "\n"),
        EditorCommand::CutSelection => cut_selection(text, state),
        EditorCommand::MoveCaretLeft => {
            let next = state.caret.offset.saturating_sub(1);
            EditorCommandResult {
                text: text.to_string(),
                state: state_with_selection(state, next, next),
            }
        }
        EditorCommand::MoveCaretRight => {
            let next = (state.caret.offset + 1).min(text.chars().count());
            EditorCommandResult {
                text: text.to_string(),
                state: state_with_selection(state, next, next),
            }
        }
        EditorCommand::ExtendSelectionLeft => {
            let next = state.caret.offset.saturating_sub(1);
            EditorCommandResult {
                text: text.to_string(),
                state: EditorSurfaceState {
                    caret: EditorCaret { offset: next },
                    selection: EditorSelection {
                        anchor: state.selection.anchor,
                        focus: next,
                    },
                    scroll_window: state.scroll_window.clone(),
                    completion_anchor_offset: None,
                    completion_selected_index: None,
                    signature_help_anchor_offset: None,
                },
            }
        }
        EditorCommand::ExtendSelectionRight => {
            let next = (state.caret.offset + 1).min(text.chars().count());
            EditorCommandResult {
                text: text.to_string(),
                state: EditorSurfaceState {
                    caret: EditorCaret { offset: next },
                    selection: EditorSelection {
                        anchor: state.selection.anchor,
                        focus: next,
                    },
                    scroll_window: state.scroll_window.clone(),
                    completion_anchor_offset: None,
                    completion_selected_index: None,
                    signature_help_anchor_offset: None,
                },
            }
        }
        EditorCommand::SelectPreviousCompletion
        | EditorCommand::SelectNextCompletion
        | EditorCommand::SelectCompletionByIndex(_)
        | EditorCommand::AcceptSelectedCompletion
        | EditorCommand::AcceptCompletionByIndex(_)
        | EditorCommand::CommitEntry
        | EditorCommand::CancelEntry
        | EditorCommand::RequestProof
        | EditorCommand::ForceShowCompletion
        | EditorCommand::DismissCompletion
        | EditorCommand::CycleReferenceForm
        | EditorCommand::ToggleExpandedHeight
        | EditorCommand::SendSelectionToInspect
        | EditorCommand::ToggleEditorSettingsPopover
        | EditorCommand::UpdateEditorSetting(_)
        | EditorCommand::ToggleConfigureDrawer => EditorCommandResult {
            text: text.to_string(),
            state: state.clone(),
        },
        EditorCommand::Backspace => backspace(text, state),
        EditorCommand::Delete => delete(text, state),
        EditorCommand::IndentWithSpaces => indent_with_spaces(text, state),
        EditorCommand::OutdentWithSpaces => outdent_with_spaces(text, state),
    }
}

/// Map a keydown event to an editor command if — and only if — the editor
/// needs to re-own that key. Native textarea behaviour handles everything
/// else: ArrowLeft/Right/Up/Down, Home, End, PageUp/Down, Backspace, Delete,
/// plain Enter (inserts newline), plain character entry, IME, clipboard
/// shortcuts, native undo/redo. The editor only intercepts keys where native
/// behaviour is wrong or absent: Tab (indent / completion accept), Shift+Tab
/// (outdent), Ctrl+Space (force completion), Escape (cancel or dismiss
/// popup), F4 (cycle reference), Ctrl+Enter (request proof), Ctrl+Shift+U
/// (toggle expanded height), Ctrl+Alt+I (send selection to Inspect), and —
/// only while the completion popup is visible — ArrowUp/Down and Enter to
/// navigate and accept proposals.
pub fn keydown_to_command(
    key: &str,
    shift_key: bool,
    shortcut_key: bool,
    alt_key: bool,
    context: EditorKeyContext,
) -> Option<EditorCommand> {
    match (key, shift_key, shortcut_key, alt_key) {
        // Completion popup navigation and accept — only when popup visible.
        ("ArrowUp", false, false, false) if context.completion_active => {
            Some(EditorCommand::SelectPreviousCompletion)
        }
        ("ArrowDown", false, false, false) if context.completion_active => {
            Some(EditorCommand::SelectNextCompletion)
        }
        ("Enter", false, false, false) if context.completion_active => {
            Some(EditorCommand::AcceptSelectedCompletion)
        }
        ("Tab", false, false, false) if context.completion_active => {
            Some(EditorCommand::AcceptSelectedCompletion)
        }
        ("Escape", false, false, false) if context.completion_active => {
            Some(EditorCommand::DismissCompletion)
        }
        // Editor-owned chords (regardless of completion state).
        (" ", false, true, false) => Some(EditorCommand::ForceShowCompletion),
        ("Enter", false, true, false) => Some(EditorCommand::RequestProof),
        ("Escape", false, false, false) => Some(EditorCommand::CancelEntry),
        ("Tab", false, false, false) => Some(EditorCommand::IndentWithSpaces),
        ("Tab", true, false, false) => Some(EditorCommand::OutdentWithSpaces),
        ("F4", false, false, false) => Some(EditorCommand::CycleReferenceForm),
        ("U" | "u", true, true, false) => Some(EditorCommand::ToggleExpandedHeight),
        ("I" | "i", false, true, true) => Some(EditorCommand::SendSelectionToInspect),
        // Everything else — ArrowKeys, Backspace, Delete, plain Enter (newline),
        // character entry, clipboard, undo/redo — falls through to the native
        // textarea. The editor observes the resulting state via the input
        // event, not via prevent_default.
        _ => None,
    }
}

pub fn classify_dom_input(input_type: &str) -> EditorInputKind {
    match input_type {
        "insertText" => EditorInputKind::InsertText,
        "deleteContentBackward" => EditorInputKind::DeleteBackward,
        "deleteContentForward" => EditorInputKind::DeleteForward,
        "insertFromPaste" => EditorInputKind::InsertFromPaste,
        _ => EditorInputKind::Other,
    }
}

pub fn cycle_completion_selection(
    current: Option<usize>,
    proposal_count: usize,
    delta: isize,
) -> Option<usize> {
    if proposal_count == 0 {
        return None;
    }

    let current = current.unwrap_or(0) as isize;
    let proposal_count = proposal_count as isize;
    Some((current + delta).rem_euclid(proposal_count) as usize)
}

fn insert_text(text: &str, state: &EditorSurfaceState, inserted_text: &str) -> EditorCommandResult {
    let selection_start = state.selection.start();
    let selection_end = state.selection.end();
    let start_idx = char_to_byte_idx(text, selection_start);
    let end_idx = char_to_byte_idx(text, selection_end);

    let mut result = String::with_capacity(text.len() + inserted_text.len());
    result.push_str(&text[..start_idx]);
    result.push_str(inserted_text);
    result.push_str(&text[end_idx..]);

    let next_offset = selection_start + inserted_text.chars().count();
    EditorCommandResult {
        text: result,
        state: state_with_selection(state, next_offset, next_offset),
    }
}

fn cut_selection(text: &str, state: &EditorSurfaceState) -> EditorCommandResult {
    if state.selection.is_collapsed() {
        return EditorCommandResult {
            text: text.to_string(),
            state: state.clone(),
        };
    }

    insert_text(text, state, "")
}

fn backspace(text: &str, state: &EditorSurfaceState) -> EditorCommandResult {
    if !state.selection.is_collapsed() {
        return insert_text(text, state, "");
    }

    if state.caret.offset == 0 {
        return EditorCommandResult {
            text: text.to_string(),
            state: state.clone(),
        };
    }

    let start = state.caret.offset.saturating_sub(1);
    let start_idx = char_to_byte_idx(text, start);
    let end_idx = char_to_byte_idx(text, state.caret.offset);
    let mut result = text.to_string();
    result.replace_range(start_idx..end_idx, "");
    EditorCommandResult {
        text: result,
        state: state_with_selection(state, start, start),
    }
}

fn delete(text: &str, state: &EditorSurfaceState) -> EditorCommandResult {
    if !state.selection.is_collapsed() {
        return insert_text(text, state, "");
    }

    if state.caret.offset >= text.chars().count() {
        return EditorCommandResult {
            text: text.to_string(),
            state: state.clone(),
        };
    }

    let start_idx = char_to_byte_idx(text, state.caret.offset);
    let end_idx = char_to_byte_idx(text, state.caret.offset + 1);
    let mut result = text.to_string();
    result.replace_range(start_idx..end_idx, "");
    EditorCommandResult {
        text: result,
        state: state_with_selection(state, state.caret.offset, state.caret.offset),
    }
}

fn indent_with_spaces(text: &str, state: &EditorSurfaceState) -> EditorCommandResult {
    let indent = "    ";
    let line_starts = selected_line_starts(text, &state.selection);
    let mut result = text.to_string();
    let mut inserted_before_caret = 0usize;
    let mut inserted_before_anchor = 0usize;
    let mut inserted_before_focus = 0usize;

    for line_start in line_starts.into_iter().rev() {
        let byte_idx = char_to_byte_idx(&result, line_start);
        result.insert_str(byte_idx, indent);
        if line_start <= state.caret.offset {
            inserted_before_caret += indent.chars().count();
        }
        if line_start <= state.selection.anchor {
            inserted_before_anchor += indent.chars().count();
        }
        if line_start <= state.selection.focus {
            inserted_before_focus += indent.chars().count();
        }
    }

    let new_caret = state.caret.offset + inserted_before_caret;
    EditorCommandResult {
        text: result,
        state: EditorSurfaceState {
            caret: EditorCaret { offset: new_caret },
            selection: EditorSelection {
                anchor: state.selection.anchor + inserted_before_anchor,
                focus: state.selection.focus + inserted_before_focus,
            },
            scroll_window: state.scroll_window.clone(),
            completion_anchor_offset: None,
            completion_selected_index: None,
            signature_help_anchor_offset: None,
        },
    }
}

fn outdent_with_spaces(text: &str, state: &EditorSurfaceState) -> EditorCommandResult {
    let line_starts = selected_line_starts(text, &state.selection);
    let mut result = text.to_string();
    let mut removed_before_caret = 0usize;
    let mut removed_before_anchor = 0usize;
    let mut removed_before_focus = 0usize;

    for line_start in line_starts.into_iter().rev() {
        let removal = leading_spaces_at(&result, line_start).min(4);
        if removal == 0 {
            continue;
        }

        let byte_idx = char_to_byte_idx(&result, line_start);
        let end_idx = char_to_byte_idx(&result, line_start + removal);
        result.replace_range(byte_idx..end_idx, "");

        if line_start < state.caret.offset {
            removed_before_caret += removal.min(state.caret.offset - line_start);
        }
        if line_start < state.selection.anchor {
            removed_before_anchor += removal.min(state.selection.anchor - line_start);
        }
        if line_start < state.selection.focus {
            removed_before_focus += removal.min(state.selection.focus - line_start);
        }
    }

    let new_caret = state.caret.offset.saturating_sub(removed_before_caret);
    EditorCommandResult {
        text: result,
        state: EditorSurfaceState {
            caret: EditorCaret { offset: new_caret },
            selection: EditorSelection {
                anchor: state.selection.anchor.saturating_sub(removed_before_anchor),
                focus: state.selection.focus.saturating_sub(removed_before_focus),
            },
            scroll_window: state.scroll_window.clone(),
            completion_anchor_offset: None,
            completion_selected_index: None,
            signature_help_anchor_offset: None,
        },
    }
}

fn state_with_selection(
    state: &EditorSurfaceState,
    anchor: usize,
    focus: usize,
) -> EditorSurfaceState {
    EditorSurfaceState {
        caret: EditorCaret { offset: focus },
        selection: EditorSelection { anchor, focus },
        scroll_window: state.scroll_window.clone(),
        completion_anchor_offset: None,
        completion_selected_index: None,
        signature_help_anchor_offset: None,
    }
}

fn selected_line_starts(text: &str, selection: &EditorSelection) -> Vec<usize> {
    let mut starts = line_start_offsets(text);
    let start = line_start_for_offset(text, selection.start());
    let end = line_start_for_offset(text, selection.end());
    starts.retain(|offset| *offset >= start && *offset <= end);
    if starts.is_empty() {
        vec![0]
    } else {
        starts
    }
}

fn line_start_offsets(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, ch) in text.chars().enumerate() {
        if ch == '\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

fn line_start_for_offset(text: &str, offset: usize) -> usize {
    let mut current = 0usize;
    for (idx, ch) in text.chars().enumerate() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            current = idx + 1;
        }
    }
    current
}

fn leading_spaces_at(text: &str, line_start: usize) -> usize {
    text.chars()
        .skip(line_start)
        .take_while(|ch| *ch == ' ')
        .count()
}

fn char_to_byte_idx(text: &str, char_idx: usize) -> usize {
    text.char_indices()
        .map(|(idx, _)| idx)
        .nth(char_idx)
        .unwrap_or(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::editor::state::{EditorScrollWindow, EditorSelection, EditorSurfaceState};

    #[test]
    fn move_caret_commands_stay_in_bounds() {
        let text = "=SUM(1,2)";
        let state = EditorSurfaceState {
            caret: EditorCaret { offset: 1 },
            selection: EditorSelection::collapsed(1),
            scroll_window: EditorScrollWindow {
                first_visible_line: 0,
                visible_line_count: 4,
            },
            completion_anchor_offset: None,
            completion_selected_index: None,
            signature_help_anchor_offset: None,
        };

        let moved_left = apply_editor_command(text, &state, EditorCommand::MoveCaretLeft);
        assert_eq!(moved_left.state.caret.offset, 0);

        let moved_right =
            apply_editor_command(text, &moved_left.state, EditorCommand::MoveCaretRight);
        assert_eq!(moved_right.state.caret.offset, 1);
    }

    #[test]
    fn extend_selection_commands_grow_selection_from_current_anchor() {
        let text = "=SUM(1,2)";
        let state = EditorSurfaceState {
            caret: EditorCaret { offset: 4 },
            selection: EditorSelection::collapsed(4),
            scroll_window: EditorScrollWindow {
                first_visible_line: 0,
                visible_line_count: 4,
            },
            completion_anchor_offset: None,
            completion_selected_index: None,
            signature_help_anchor_offset: None,
        };

        let extended_left = apply_editor_command(text, &state, EditorCommand::ExtendSelectionLeft);
        assert_eq!(extended_left.state.caret.offset, 3);
        assert_eq!(extended_left.state.selection.anchor, 4);
        assert_eq!(extended_left.state.selection.focus, 3);
        assert!(!extended_left.state.selection.is_collapsed());

        let extended_right = apply_editor_command(
            text,
            &extended_left.state,
            EditorCommand::ExtendSelectionRight,
        );
        assert_eq!(extended_right.state.caret.offset, 4);
        assert_eq!(extended_right.state.selection.anchor, 4);
        assert_eq!(extended_right.state.selection.focus, 4);
        assert!(extended_right.state.selection.is_collapsed());
    }

    #[test]
    fn indent_with_spaces_inserts_four_spaces_on_selected_lines() {
        let text = "SUM(\n1,\n2)";
        let state = EditorSurfaceState {
            caret: EditorCaret { offset: 3 },
            selection: EditorSelection {
                anchor: 0,
                focus: text.chars().count(),
            },
            scroll_window: EditorScrollWindow {
                first_visible_line: 0,
                visible_line_count: 4,
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
    fn outdent_with_spaces_removes_up_to_four_spaces_per_line() {
        let text = "    SUM(\n    1,\n    2)";
        let state = EditorSurfaceState {
            caret: EditorCaret { offset: 4 },
            selection: EditorSelection {
                anchor: 0,
                focus: text.chars().count(),
            },
            scroll_window: EditorScrollWindow {
                first_visible_line: 0,
                visible_line_count: 4,
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
    fn keydown_mapping_only_intercepts_editor_owned_keys() {
        let with_completion = EditorKeyContext {
            completion_active: true,
        };
        let no_completion = EditorKeyContext::default();

        // Completion popup navigation intercepts only while the popup is visible.
        assert_eq!(
            keydown_to_command("ArrowUp", false, false, false, with_completion),
            Some(EditorCommand::SelectPreviousCompletion)
        );
        assert_eq!(
            keydown_to_command("ArrowDown", false, false, false, with_completion),
            Some(EditorCommand::SelectNextCompletion)
        );
        assert_eq!(
            keydown_to_command("Enter", false, false, false, with_completion),
            Some(EditorCommand::AcceptSelectedCompletion)
        );
        assert_eq!(
            keydown_to_command("Tab", false, false, false, with_completion),
            Some(EditorCommand::AcceptSelectedCompletion)
        );
        assert_eq!(
            keydown_to_command("Escape", false, false, false, with_completion),
            Some(EditorCommand::DismissCompletion)
        );

        // Editor-owned chords (independent of completion state).
        assert_eq!(
            keydown_to_command("Enter", false, true, false, no_completion),
            Some(EditorCommand::RequestProof)
        );
        assert_eq!(
            keydown_to_command("Escape", false, false, false, no_completion),
            Some(EditorCommand::CancelEntry)
        );
        assert_eq!(
            keydown_to_command("Tab", false, false, false, no_completion),
            Some(EditorCommand::IndentWithSpaces)
        );
        assert_eq!(
            keydown_to_command("Tab", true, false, false, no_completion),
            Some(EditorCommand::OutdentWithSpaces)
        );
        assert_eq!(
            keydown_to_command(" ", false, true, false, no_completion),
            Some(EditorCommand::ForceShowCompletion)
        );
        assert_eq!(
            keydown_to_command("F4", false, false, false, no_completion),
            Some(EditorCommand::CycleReferenceForm)
        );
        assert_eq!(
            keydown_to_command("U", true, true, false, no_completion),
            Some(EditorCommand::ToggleExpandedHeight)
        );
        assert_eq!(
            keydown_to_command("i", false, true, true, no_completion),
            Some(EditorCommand::SendSelectionToInspect)
        );

        // Native-handled keys must fall through (return None) so the textarea
        // keeps ownership of caret movement, deletion, and newline insertion.
        assert_eq!(
            keydown_to_command("ArrowLeft", false, false, false, no_completion),
            None
        );
        assert_eq!(
            keydown_to_command("ArrowRight", false, false, false, no_completion),
            None
        );
        assert_eq!(
            keydown_to_command("ArrowLeft", true, false, false, no_completion),
            None
        );
        assert_eq!(
            keydown_to_command("ArrowRight", true, false, false, no_completion),
            None
        );
        assert_eq!(
            keydown_to_command("ArrowUp", false, false, false, no_completion),
            None
        );
        assert_eq!(
            keydown_to_command("ArrowDown", false, false, false, no_completion),
            None
        );
        assert_eq!(
            keydown_to_command("Backspace", false, false, false, no_completion),
            None
        );
        assert_eq!(
            keydown_to_command("Delete", false, false, false, no_completion),
            None
        );
        // Plain Enter without an open completion popup inserts a newline natively
        // (textarea default), so it must not map to any command.
        assert_eq!(
            keydown_to_command("Enter", false, false, false, no_completion),
            None
        );
        // Ctrl+X is handled natively by the textarea and its on:cut handler.
        assert_eq!(
            keydown_to_command("x", false, true, false, no_completion),
            None
        );
    }

    #[test]
    fn dom_input_types_map_to_editor_input_kinds() {
        assert_eq!(
            classify_dom_input("insertText"),
            EditorInputKind::InsertText
        );
        assert_eq!(
            classify_dom_input("deleteContentBackward"),
            EditorInputKind::DeleteBackward
        );
        assert_eq!(
            classify_dom_input("deleteContentForward"),
            EditorInputKind::DeleteForward
        );
        assert_eq!(
            classify_dom_input("insertFromPaste"),
            EditorInputKind::InsertFromPaste
        );
        assert_eq!(classify_dom_input("historyUndo"), EditorInputKind::Other);
    }

    #[test]
    fn insert_text_replaces_non_collapsed_selection() {
        let text = "=SUM(1,2)";
        let state = EditorSurfaceState {
            caret: EditorCaret { offset: 5 },
            selection: EditorSelection {
                anchor: 1,
                focus: 4,
            },
            scroll_window: EditorScrollWindow {
                first_visible_line: 0,
                visible_line_count: 4,
            },
            completion_anchor_offset: Some(4),
            completion_selected_index: Some(0),
            signature_help_anchor_offset: Some(4),
        };

        let result =
            apply_editor_command(text, &state, EditorCommand::InsertText("AVG".to_string()));
        assert_eq!(result.text, "=AVG(1,2)");
        assert_eq!(result.state.caret.offset, 4);
        assert!(result.state.selection.is_collapsed());
        assert!(result.state.completion_anchor_offset.is_none());
        assert!(result.state.completion_selected_index.is_none());
        assert!(result.state.signature_help_anchor_offset.is_none());
    }

    #[test]
    fn backspace_and_delete_are_selection_aware() {
        let text = "=SUM(1,2)";
        let selected = EditorSurfaceState {
            caret: EditorCaret { offset: 5 },
            selection: EditorSelection {
                anchor: 1,
                focus: 4,
            },
            scroll_window: EditorScrollWindow {
                first_visible_line: 0,
                visible_line_count: 4,
            },
            completion_anchor_offset: None,
            completion_selected_index: None,
            signature_help_anchor_offset: None,
        };

        let backspace = apply_editor_command(text, &selected, EditorCommand::Backspace);
        assert_eq!(backspace.text, "=(1,2)");
        assert_eq!(backspace.state.caret.offset, 1);
        assert!(backspace.state.selection.is_collapsed());

        let collapsed = EditorSurfaceState {
            caret: EditorCaret { offset: 1 },
            selection: EditorSelection::collapsed(1),
            scroll_window: EditorScrollWindow {
                first_visible_line: 0,
                visible_line_count: 4,
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
    fn insert_newline_inserts_line_feed_at_caret() {
        let text = "=SUM(1,2)";
        let state = EditorSurfaceState {
            caret: EditorCaret { offset: 5 },
            selection: EditorSelection::collapsed(5),
            scroll_window: EditorScrollWindow {
                first_visible_line: 0,
                visible_line_count: 4,
            },
            completion_anchor_offset: None,
            completion_selected_index: None,
            signature_help_anchor_offset: None,
        };
        let result = apply_editor_command(text, &state, EditorCommand::InsertNewline);
        assert_eq!(result.text, "=SUM(\n1,2)");
        assert_eq!(result.state.caret.offset, 6);
    }

    #[test]
    fn completion_selection_cycles_through_visible_proposals() {
        assert_eq!(cycle_completion_selection(None, 3, 1), Some(1));
        assert_eq!(cycle_completion_selection(Some(0), 3, 1), Some(1));
        assert_eq!(cycle_completion_selection(Some(0), 3, -1), Some(2));
        assert_eq!(cycle_completion_selection(Some(2), 3, 1), Some(0));
        assert_eq!(cycle_completion_selection(Some(0), 0, 1), None);
    }
}
