//! S2 / S3 / S4 / S5 / S12 — formula entry scenarios against the
//! WS-14 home shell.
//!
//! Each test dispatches `apply_live_editor_input` against a real
//! `NativeOxfmlHostSession` (or a `FakeBridge` where the live bridge can't yet
//! return the richer document a scenario needs), then projects the host
//! state through `build_home_shell_view_model` and asserts on the
//! `ResultView` that the home shell renders.

use dnaonecalc_host::adapters::oxfml::{
    EditorDocument, FormulaEditRequest, FormulaEditResult, NativeOxfmlHostSession,
    OxfmlHostSession, OxfmlHostSessionError,
};
use dnaonecalc_host::app::case_lifecycle::new_formula_space;
use dnaonecalc_host::services::home_shell_view_model::{
    build_home_shell_view_model, HomeShellViewModel, ResultKind, ResultView,
};
use dnaonecalc_host::services::live_edit::apply_live_editor_input;
use dnaonecalc_host::state::OneCalcHostState;
use dnaonecalc_host::test_support::sample_editor_document;
use dnaonecalc_host::ui::editor::commands::{EditorInputEvent, EditorInputKind};
use dnaonecalc_host::ui::editor::state::EditorLiveState;

/// Create a fresh workspace with one active untitled formula space.
fn fresh_state_with_active_space() -> OneCalcHostState {
    let mut state = OneCalcHostState::default();
    let _ = new_formula_space(&mut state);
    state
}

/// Build the real live OxFml editor bridge used by the running app.
fn scenario_bridge() -> NativeOxfmlHostSession {
    NativeOxfmlHostSession::default()
}

/// Dispatch a user-level "type the whole string" input event through the
/// real reducer + the provided bridge. Matches how the editor surface
/// forwards a textarea input change.
fn type_formula(bridge: &dyn OxfmlHostSession, state: &mut OneCalcHostState, text: &str) {
    let caret_offset = text.chars().count();
    apply_live_editor_input(
        bridge,
        state,
        EditorInputEvent {
            text: text.to_string(),
            selection_start: Some(caret_offset),
            selection_end: Some(caret_offset),
            input_kind: EditorInputKind::InsertText,
            inserted_text: Some(text.to_string()),
        },
    )
    .expect("live bridge should succeed for scenario inputs");
}

fn home_view(state: &OneCalcHostState) -> HomeShellViewModel {
    build_home_shell_view_model(state).expect("home-shell view-model should be available")
}

#[test]
fn typing_a_sum_formula_shows_the_numeric_result() {
    // S2 / S3: type `=SUM(1,2,3)` and see `6` in the home-shell result.
    // Drives the full OxFml + OxFunc runtime through `NativeOxfmlHostSession` so
    // the assertion is on a real evaluation, not a fallback hand-eval.
    let mut state = fresh_state_with_active_space();
    let bridge = scenario_bridge();

    type_formula(&bridge, &mut state, "=SUM(1,2,3)");

    let home = home_view(&state);
    assert_eq!(home.raw_entered_cell_text, "=SUM(1,2,3)");
    match home.result_view {
        ResultView::Display { text, kind, .. } => {
            assert_eq!(text, "6");
            assert_eq!(kind, ResultKind::Number);
        }
        other => panic!("expected Display(Number, '6'), got {other:?}"),
    }
}

#[test]
fn typing_a_two_arg_sum_shows_the_addition_result() {
    // S2: type `=SUM(1,1)` and see `2` through the real engine.
    let mut state = fresh_state_with_active_space();
    let bridge = scenario_bridge();

    type_formula(&bridge, &mut state, "=SUM(1,1)");

    let home = home_view(&state);
    match home.result_view {
        ResultView::Display { text, kind, .. } => {
            assert_eq!(text, "2");
            assert_eq!(kind, ResultKind::Number);
        }
        other => panic!("expected Display(Number, '2'), got {other:?}"),
    }
}

#[test]
fn typing_a_sequence_formula_shows_the_two_by_two_array_preview() {
    // S4: type `=SEQUENCE(2,2)` and see a 2×2 array shape in the home
    // shell's `Array` result view; the underlying array preview lives on
    // the formula space and carries the full grid.
    let mut state = fresh_state_with_active_space();
    let bridge = scenario_bridge();

    type_formula(&bridge, &mut state, "=SEQUENCE(2,2)");

    let home = home_view(&state);
    match home.result_view {
        ResultView::Array {
            total_rows,
            total_cols,
            ..
        } => {
            assert_eq!(total_rows, 2);
            assert_eq!(total_cols, 2);
        }
        other => panic!("expected Array{{2x2}}, got {other:?}"),
    }

    // The state-level array preview retains the full 2×2 grid for downstream
    // consumers (drill-down / array-view component).
    let active_id = state
        .workspace_shell
        .active_formula_space_id
        .as_ref()
        .expect("active formula space");
    let preview = state
        .formula_spaces
        .get(active_id)
        .and_then(|space| space.array_preview.as_ref())
        .expect("sequence scenario should populate array preview");
    assert_eq!(preview.rows.len(), 2);
    assert!(preview.rows.iter().all(|row| row.len() == 2));
    assert_eq!(preview.rows[0], vec!["1".to_string(), "2".to_string()]);
    assert_eq!(preview.rows[1], vec!["3".to_string(), "4".to_string()]);
}

struct DiagnosticFakeBridge {
    document: EditorDocument,
}

impl OxfmlHostSession for DiagnosticFakeBridge {
    fn apply_formula_edit(
        &self,
        _request: FormulaEditRequest,
    ) -> Result<FormulaEditResult, OxfmlHostSessionError> {
        Ok(FormulaEditResult {
            document: self.document.clone(),
        })
    }
}

#[test]
fn typing_an_invalid_formula_surfaces_a_diagnostic_in_the_result_view() {
    // S5: type `=SUM(` with an unclosed paren. The live bridge does not
    // currently emit diagnostics, so this scenario uses a fake bridge that
    // returns a document with a populated `live_diagnostics` snapshot. The
    // assertion is that the diagnostic reaches the home shell as an
    // `Error{code:"DIAGNOSTIC"}` ResultView and the editor document carries
    // the diagnostic so a future drill-down can render the squiggle.
    let mut state = fresh_state_with_active_space();
    let mut document = sample_editor_document("=SUM(");
    document.live_diagnostics = dnaonecalc_host::test_support::live_diagnostic_snapshot_with(vec![
        dnaonecalc_host::test_support::make_live_diagnostic("diag-1", "unmatched '('", 4, 1),
    ]);
    let bridge = DiagnosticFakeBridge { document };

    apply_live_editor_input(
        &bridge,
        &mut state,
        EditorInputEvent {
            text: "=SUM(".to_string(),
            selection_start: Some(5),
            selection_end: Some(5),
            input_kind: EditorInputKind::InsertText,
            inserted_text: Some("=SUM(".to_string()),
        },
    )
    .expect("fake bridge always succeeds");

    let home = home_view(&state);
    match &home.result_view {
        ResultView::Error { code, surface_repr } => {
            assert_eq!(code, "DIAGNOSTIC");
            assert_eq!(surface_repr.as_deref(), Some("unmatched '('"));
        }
        other => panic!("expected Error(DIAGNOSTIC), got {other:?}"),
    }

    // The diagnostic itself is reachable via the editor document on the
    // active formula space, ready for a drill-down render.
    let active_id = state
        .workspace_shell
        .active_formula_space_id
        .as_ref()
        .expect("active formula space");
    let diagnostics = state
        .formula_spaces
        .get(active_id)
        .and_then(|space| space.editor_document.as_ref())
        .map(|doc| &doc.live_diagnostics.diagnostics)
        .expect("editor document carries diagnostics");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].message, "unmatched '('");
}

/// User types `=SUM(1,2,3)` and then presses ArrowRight to move
/// the caret one position past the close paren. The signature-help
/// popup must be suppressed at every cursor position outside the
/// call's argument list, end-to-end through the real bridge.
///
/// Originally pinned-failing against an OxFml regression where the
/// past-`)` guard looked for `RParen` as a direct child of the
/// CallExpr node, but the parser placed `)` inside the ArgumentList
/// child. OxFml's `closed_call_close_paren_end` now walks into the
/// ArgumentList, the regression is fixed, and the test is live.
/// See `docs/HANDOFF_OXFML_SIGNATURE_HELP_PAST_PAREN_REGRESSION.md`.
#[test]
fn signature_help_disappears_when_caret_is_past_close_paren() {
    let mut state = fresh_state_with_active_space();
    let bridge = scenario_bridge();

    // First: type the open paren and confirm signature help is on.
    type_formula(&bridge, &mut state, "=SUM(");
    let active_id = state
        .workspace_shell
        .active_formula_space_id
        .clone()
        .expect("active formula space");
    {
        let space = state.formula_spaces.get(&active_id).expect("space present");
        let doc = space
            .editor_document
            .as_ref()
            .expect("document built by bridge");
        assert!(
            doc.signature_help.is_some(),
            "expected signature help inside `=SUM(`",
        );
    }

    // Type the rest, leaving caret immediately after the `)` —
    // exactly the position the user reported the popup persisting at.
    type_formula(&bridge, &mut state, "=SUM(1,2,3)");
    let space = state.formula_spaces.get(&active_id).expect("space present");
    let doc = space
        .editor_document
        .as_ref()
        .expect("document built by bridge");
    assert!(
        doc.signature_help.is_none(),
        "expected signature help suppressed once caret is past `)`; document still carries: {:?}",
        doc.signature_help,
    );

    let home = home_view(&state);
    assert!(
        home.signature_help.is_none(),
        "view-model must not project a signature help popup past `)`",
    );
}

/// Caret-only navigation (mouse click, arrow keys) past a closed
/// call must suppress signature help — the user expects the popup
/// to disappear without having to type more text. Companion to
/// `signature_help_disappears_when_caret_is_past_close_paren`; the
/// OxFml fix that makes the typed-past-`)` guard work covers this
/// caret-move shape too.
#[test]
fn signature_help_disappears_after_caret_moves_past_close_paren() {
    let mut state = fresh_state_with_active_space();
    let bridge = scenario_bridge();

    // Type the formula in two halves so the caret ends up inside the
    // open parens first (signature help on), then advance past `)` by
    // synthesising a caret-only move (the same shape the on:keyup /
    // on:click handlers in the home shell emit).
    type_formula(&bridge, &mut state, "=SUM(1,2,3)");

    // First confirm the popup is gone post-typing-)`.
    let active_id = state
        .workspace_shell
        .active_formula_space_id
        .clone()
        .expect("active formula space");
    {
        let doc = state
            .formula_spaces
            .get(&active_id)
            .and_then(|space| space.editor_document.as_ref())
            .expect("document built by bridge");
        assert!(doc.signature_help.is_none());
    }

    // Now simulate the user moving the caret BACK INTO the call
    // (cursor=7 is between `1,` and `2`) — popup must come back.
    apply_live_editor_input(
        &bridge,
        &mut state,
        EditorInputEvent {
            text: "=SUM(1,2,3)".to_string(),
            selection_start: Some(7),
            selection_end: Some(7),
            input_kind: EditorInputKind::Other,
            inserted_text: None,
        },
    )
    .expect("live bridge");
    {
        let doc = state
            .formula_spaces
            .get(&active_id)
            .and_then(|space| space.editor_document.as_ref())
            .expect("document built by bridge");
        assert!(
            doc.signature_help.is_some(),
            "expected signature help to return when caret moves back inside `(...)`",
        );
    }

    // And out again past `)` — caret_offset 11.
    apply_live_editor_input(
        &bridge,
        &mut state,
        EditorInputEvent {
            text: "=SUM(1,2,3)".to_string(),
            selection_start: Some(11),
            selection_end: Some(11),
            input_kind: EditorInputKind::Other,
            inserted_text: None,
        },
    )
    .expect("live bridge");
    let doc = state
        .formula_spaces
        .get(&active_id)
        .and_then(|space| space.editor_document.as_ref())
        .expect("document built by bridge");
    assert!(
        doc.signature_help.is_none(),
        "expected signature help suppressed after caret-only move past `)`; got: {:?}",
        doc.signature_help,
    );
}

#[test]
fn rapid_typing_preserves_the_latest_input_without_stale_state() {
    // S12: dispatch three sequential input events through the live
    // bridge. After the third, the home view-model must reflect the third
    // input exactly, the final evaluation must be the real OxFml+OxFunc
    // result, and no stale diagnostics may remain on the editor document.
    let mut state = fresh_state_with_active_space();
    let bridge = scenario_bridge();

    type_formula(&bridge, &mut state, "=");
    type_formula(&bridge, &mut state, "=SU");
    type_formula(&bridge, &mut state, "=SUM(1,2,3)");

    let home = home_view(&state);
    assert_eq!(home.raw_entered_cell_text, "=SUM(1,2,3)");
    match home.result_view {
        ResultView::Display { text, kind, .. } => {
            assert_eq!(text, "6");
            assert_eq!(kind, ResultKind::Number);
        }
        other => panic!("expected Display(Number, '6'), got {other:?}"),
    }

    // Live state: user has typed but not committed; the formula space
    // should report `EditingLive`. No stale diagnostics on the document.
    let active_id = state
        .workspace_shell
        .active_formula_space_id
        .as_ref()
        .expect("active formula space");
    let formula_space = state
        .formula_spaces
        .get(active_id)
        .expect("active formula space present");
    assert_eq!(formula_space.live_state(), EditorLiveState::EditingLive);
    let diagnostics = formula_space
        .editor_document
        .as_ref()
        .map(|doc| doc.live_diagnostics.diagnostics.len())
        .unwrap_or(0);
    assert_eq!(diagnostics, 0);
}
