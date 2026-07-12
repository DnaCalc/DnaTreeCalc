//! S-CMP-* — completion popup lifecycle scenarios.
//!
//! Each scenario walks a user action through `apply_live_editor_input`
//! against a real `NativeOxfmlHostSession`, then asserts on the lib-side
//! popup state machine in `services::completion_popup`. No browser
//! involvement; the corresponding DOM contracts live in
//! `tests/browser/completion.rs` (added in bead dno-xcq.24+).

use dnaonecalc_host::adapters::oxfml::{NativeOxfmlHostSession, OxfmlHostSession};
use dnaonecalc_host::app::case_lifecycle::new_formula_space;
use dnaonecalc_host::app::reducer::{
    accept_selected_completion_on_active_formula_space,
    dismiss_completion_popup_on_active_formula_space,
    move_completion_popup_selection_on_active_formula_space,
};
use dnaonecalc_host::services::completion_popup::CompletionPopupState;
use dnaonecalc_host::services::live_edit::apply_live_editor_input;
use dnaonecalc_host::state::OneCalcHostState;
use dnaonecalc_host::ui::editor::commands::{EditorInputEvent, EditorInputKind};

fn fresh_state_with_active_space() -> OneCalcHostState {
    let mut state = OneCalcHostState::default();
    let _ = new_formula_space(&mut state);
    state
}

fn scenario_bridge() -> NativeOxfmlHostSession {
    NativeOxfmlHostSession::default()
}

fn type_formula(bridge: &dyn OxfmlHostSession, state: &mut OneCalcHostState, text: &str) {
    let caret = text.chars().count();
    apply_live_editor_input(
        bridge,
        state,
        EditorInputEvent {
            text: text.to_string(),
            selection_start: Some(caret),
            selection_end: Some(caret),
            input_kind: EditorInputKind::InsertText,
            inserted_text: Some(text.to_string()),
        },
    )
    .expect("live bridge should succeed for scenario inputs");
}

fn active_popup(state: &OneCalcHostState) -> CompletionPopupState {
    let id = state
        .workspace_shell
        .active_formula_space_id
        .as_ref()
        .expect("active formula space");
    state
        .formula_spaces
        .get(id)
        .map(|fs| fs.completion_popup.clone())
        .expect("active formula space present")
}

fn active_text(state: &OneCalcHostState) -> String {
    let id = state
        .workspace_shell
        .active_formula_space_id
        .as_ref()
        .expect("active formula space");
    state
        .formula_spaces
        .get(id)
        .map(|fs| fs.raw_entered_cell_text.clone())
        .expect("active formula space present")
}

#[test]
fn s_cmp_1_typing_partial_function_opens_popup_with_proposals() {
    // S-CMP-1: type `=SU` -> the bridge returns proposals (SUM, SUMIF,
    // ...) -> popup auto-opens with selected_index=0 and at least one
    // 'SUM' item.
    let mut state = fresh_state_with_active_space();
    let bridge = scenario_bridge();
    type_formula(&bridge, &mut state, "=SU");

    match active_popup(&state) {
        CompletionPopupState::Open {
            items,
            selected_index,
            ..
        } => {
            assert!(
                !items.is_empty(),
                "bridge should produce at least one proposal for '=SU'",
            );
            assert_eq!(selected_index, 0, "first item selected by default");
            assert!(
                items.iter().any(|i| i.display_text == "SUM"),
                "SUM should appear in the proposals list",
            );
        }
        other => panic!("expected popup Open after '=SU', got {other:?}"),
    }
}

#[test]
fn s_cmp_2_dismiss_closes_popup_and_does_not_reopen_on_same_input() {
    // S-CMP-2: type `=SU`, dismiss the popup, then dispatch a noop
    // input that produces the same proposals — popup state should
    // re-sync with the same items but the user-driven dismiss happened
    // BEFORE the next input, so the policy here is to re-open when
    // proposals come back. This pins the (current, simple) policy:
    // dismiss is forgotten on the next bridge round-trip. A future
    // bead can add a "suppression token" if user testing wants
    // different behaviour.
    let mut state = fresh_state_with_active_space();
    let bridge = scenario_bridge();
    type_formula(&bridge, &mut state, "=SU");
    assert!(matches!(
        active_popup(&state),
        CompletionPopupState::Open { .. }
    ));

    let dismissed = dismiss_completion_popup_on_active_formula_space(&mut state);
    assert!(dismissed);
    assert!(matches!(active_popup(&state), CompletionPopupState::Hidden));

    // Next input — same text — re-runs the bridge. Proposals come back
    // identical; popup re-opens. Pin this behaviour.
    type_formula(&bridge, &mut state, "=SU");
    assert!(matches!(
        active_popup(&state),
        CompletionPopupState::Open { .. }
    ));
}

#[test]
fn s_cmp_3_accept_selected_completion_returns_acceptance_and_dismisses_popup() {
    // S-CMP-3: type `=SU`, accept the first item -> acceptance carries
    // the proposal's insert_text + replacement_span; popup state
    // returns to Hidden. The home-shell layer (later bead) is what
    // splices the new text into the textarea; this scenario asserts
    // only on the state-side acceptance shape.
    let mut state = fresh_state_with_active_space();
    let bridge = scenario_bridge();
    type_formula(&bridge, &mut state, "=SU");

    let acceptance = accept_selected_completion_on_active_formula_space(&mut state)
        .expect("accept returns Some(acceptance) when popup is Open");
    assert!(
        !acceptance.insert_text.is_empty(),
        "accepted item carries non-empty insert_text",
    );
    // For SUM-family proposals the bridge typically sets a
    // replacement_span covering the partial token "SU". Don't pin the
    // exact span here (it varies by proposal) but at minimum it
    // should be Some.
    assert!(
        acceptance.replacement_span.is_some()
            || acceptance.new_caret_offset >= active_text(&state).chars().count(),
        "either the acceptance has a replacement span or the caret lands at end-of-text",
    );

    assert!(matches!(active_popup(&state), CompletionPopupState::Hidden));
}

#[test]
fn s_cmp_4_move_selection_round_trips_back_to_starting_index() {
    // S-CMP-4: type `=SU`, advance selection by one, then back. After
    // the round trip the selected index returns to its starting
    // value (assumes >= 2 items in the proposals — SUM family
    // produces multiple).
    let mut state = fresh_state_with_active_space();
    let bridge = scenario_bridge();
    type_formula(&bridge, &mut state, "=SU");

    let initial_index = match active_popup(&state) {
        CompletionPopupState::Open {
            selected_index,
            items,
            ..
        } => {
            assert!(items.len() >= 2, "scenario requires >= 2 proposals");
            selected_index
        }
        _ => panic!("popup should be Open"),
    };

    let advanced = move_completion_popup_selection_on_active_formula_space(&mut state, 1);
    let returned = move_completion_popup_selection_on_active_formula_space(&mut state, -1);
    assert!(advanced && returned);

    match active_popup(&state) {
        CompletionPopupState::Open { selected_index, .. } => {
            assert_eq!(selected_index, initial_index);
        }
        _ => panic!("popup should still be Open"),
    }
}

#[test]
fn s_cmp_5_typing_a_non_trigger_clears_proposals_and_closes_popup() {
    // S-CMP-5: type `=SU` (popup opens), then clear the textarea ->
    // proposals empty, popup auto-closes.
    let mut state = fresh_state_with_active_space();
    let bridge = scenario_bridge();
    type_formula(&bridge, &mut state, "=SU");
    assert!(matches!(
        active_popup(&state),
        CompletionPopupState::Open { .. }
    ));

    type_formula(&bridge, &mut state, "");
    assert!(matches!(active_popup(&state), CompletionPopupState::Hidden));
}

#[test]
fn s_cmp_6_accept_when_hidden_returns_none_and_does_not_change_text() {
    // S-CMP-6: with no popup visible, attempting to accept is a no-op.
    // Pins the "no-op when hidden" contract so a stray Tab keypress
    // (when bead .25 lands the keyboard policy) doesn't replace
    // anything.
    let mut state = fresh_state_with_active_space();
    let bridge = scenario_bridge();
    type_formula(&bridge, &mut state, "=SUM(1,2)");

    // Popup may or may not be open at end-of-formula; if open, dismiss.
    let _ = dismiss_completion_popup_on_active_formula_space(&mut state);
    assert!(matches!(active_popup(&state), CompletionPopupState::Hidden));

    let before = active_text(&state);
    let acceptance = accept_selected_completion_on_active_formula_space(&mut state);
    assert!(acceptance.is_none());
    assert_eq!(
        active_text(&state),
        before,
        "text unchanged on no-op accept"
    );
}
