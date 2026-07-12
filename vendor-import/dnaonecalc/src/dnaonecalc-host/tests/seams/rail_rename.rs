//! SEAM-ONECALC-RAIL-INLINE-RENAME — LANDED 2026-05-07
//!
//! The original phrasing referenced the retired shell-frame rail's
//! `ShellFormulaSpaceListItemViewModel`. WS-14's progressive home
//! shell replaces the rail with a tab strip + breadcrumb + manage-
//! formulas overlay; the inline-rename surface now lives on the
//! tab strip (double-click a tab name) and the breadcrumb's
//! "Rename…" action.
//!
//! Reducer flow:
//! - `case_lifecycle::begin_formula_rename(state, id)` flags the
//!   target formula and seeds `pending_rename_text` with its
//!   current `scenario_label`.
//! - `case_lifecycle::update_pending_rename_text` updates the
//!   buffer as the user types.
//! - `case_lifecycle::commit_formula_rename` writes the buffer to
//!   `formula_space.context.scenario_label` and clears the
//!   in-flight rename state. Empty / whitespace-only buffers leave
//!   the rename open so the user can either type a real label or
//!   hit Esc.
//! - `case_lifecycle::cancel_formula_rename` discards the buffer.
//!
//! View-model: `FormulaTabChip.is_renaming` + `rename_buffer`
//! (`services::home_shell_view_model::project_formula_tab_strip`).
//!
//! The pin below is now a positive assertion against the reducer
//! flow rather than a `seam_pending` marker.

use dnaonecalc_host::app::case_lifecycle::{
    begin_formula_rename, cancel_formula_rename, commit_formula_rename, new_formula_space,
    update_pending_rename_text,
};
use dnaonecalc_host::domain::ids::FormulaSpaceId;
use dnaonecalc_host::services::home_shell_view_model::build_home_shell_view_model;
use dnaonecalc_host::state::OneCalcHostState;

#[test]
fn begin_and_commit_inline_rename_updates_active_formula_label() {
    let mut state = OneCalcHostState::default();
    let id = new_formula_space(&mut state);

    // Begin rename: the workspace shell flags this id as the rename
    // target and seeds the buffer with the current label.
    assert!(begin_formula_rename(&mut state, id.as_str()));
    assert_eq!(
        state.workspace_shell.renaming_formula_space_id.as_ref(),
        Some(&id),
    );
    assert_eq!(state.workspace_shell.pending_rename_text, "Untitled 1");

    // The view-model surfaces the editing state on the tab chip.
    let vm = build_home_shell_view_model(&state).expect("active formula space");
    let chip = vm
        .formula_tab_strip
        .chips
        .iter()
        .find(|chip| chip.formula_space_id == id.as_str())
        .expect("matching chip");
    assert!(chip.is_renaming);
    assert_eq!(chip.rename_buffer, "Untitled 1");

    // Update the buffer + commit.
    update_pending_rename_text(&mut state, "Renamed live");
    assert!(commit_formula_rename(&mut state));
    assert_eq!(
        state
            .formula_spaces
            .get(&FormulaSpaceId::new(id.as_str().to_string()))
            .unwrap()
            .context
            .scenario_label,
        "Renamed live",
    );
    // Rename state cleared after commit.
    assert!(state.workspace_shell.renaming_formula_space_id.is_none());
    assert!(state.workspace_shell.pending_rename_text.is_empty());
}

#[test]
fn cancel_inline_rename_discards_buffer_without_writing_label() {
    let mut state = OneCalcHostState::default();
    let id = new_formula_space(&mut state);
    assert!(begin_formula_rename(&mut state, id.as_str()));
    update_pending_rename_text(&mut state, "Discarded");
    cancel_formula_rename(&mut state);
    assert_eq!(
        state
            .formula_spaces
            .get(&FormulaSpaceId::new(id.as_str().to_string()))
            .unwrap()
            .context
            .scenario_label,
        "Untitled 1",
    );
    assert!(state.workspace_shell.renaming_formula_space_id.is_none());
}
