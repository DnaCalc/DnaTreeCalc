//! SEAM-ONECALC-PERSISTENCE-V1 — LANDED
//!
//! `persistence::serialize_workspace` + `deserialize_workspace` +
//! `WorkspaceJson::apply_to_state` round-trip a workspace across
//! sessions. The localStorage / disk adapter sits on top
//! (`save_workspace_to_local_storage`, `hydrate_state_from_local_storage`).
//!
//! The pins below are now positive assertions against the
//! reducer + persistence flow rather than `seam_pending` markers.

use dnaonecalc_host::app::case_lifecycle::{
    close_formula_space, new_formula_space, pin_active_formula_space,
};
use dnaonecalc_host::persistence::{deserialize_workspace, serialize_workspace};
use dnaonecalc_host::state::OneCalcHostState;

#[test]
fn workspace_json_v1_round_trips_two_spaces() {
    // Build a workspace with two formula spaces; one is the
    // active, the other was the first opened.
    let mut state = OneCalcHostState::default();
    let first_id = new_formula_space(&mut state);
    let second_id = new_formula_space(&mut state);

    // Author some text on the active formula so we can assert it
    // round-trips.
    let active = state
        .formula_spaces
        .get_mut(&second_id)
        .expect("active space");
    active.raw_entered_cell_text = "=SUM(1,2,3)".to_string();
    active.formatting.number_format_code = "0.00".to_string();

    // Pin the active formula so the pinned-set round-trip is
    // exercised too.
    assert!(pin_active_formula_space(&mut state));

    // Serialise.
    let json = serialize_workspace(&state).expect("serialise");
    let envelope = deserialize_workspace(&json).expect("parse");

    // Restore into a fresh host.
    let mut restored = OneCalcHostState::default();
    let _ = new_formula_space(&mut restored);
    envelope
        .apply_to_state(&mut restored)
        .expect("apply round-trip");

    // Both spaces survive.
    let restored_ids: Vec<_> = restored
        .workspace_shell
        .open_formula_space_order
        .iter()
        .map(|id| id.as_str().to_string())
        .collect();
    assert!(restored_ids.contains(&first_id.as_str().to_string()));
    assert!(restored_ids.contains(&second_id.as_str().to_string()));

    // Active text + formatting round-trip.
    let restored_active = restored
        .workspace_shell
        .active_formula_space_id
        .as_ref()
        .and_then(|id| restored.formula_spaces.get(id))
        .expect("active restored");
    assert_eq!(restored_active.raw_entered_cell_text, "=SUM(1,2,3)");
    assert_eq!(restored_active.formatting.number_format_code, "0.00");

    // Pin survives.
    let pinned: Vec<_> = restored
        .workspace_shell
        .pinned_formula_space_ids
        .iter()
        .map(|id| id.as_str().to_string())
        .collect();
    assert_eq!(pinned, vec![second_id.as_str().to_string()]);
}

#[test]
fn recent_formula_space_ids_carry_across_a_save_load_cycle() {
    let mut state = OneCalcHostState::default();
    let id = new_formula_space(&mut state);
    // Author a non-empty formula so the recent record is non-trivial.
    state
        .formula_spaces
        .get_mut(&id)
        .expect("space")
        .raw_entered_cell_text = "=NOW()".to_string();
    // Open a second formula then close the first → it's now in recents.
    let _ = new_formula_space(&mut state);
    assert!(close_formula_space(&mut state, id.as_str()));
    assert!(!state.workspace_shell.recent_formula_space_order.is_empty());

    // Round-trip.
    let json = serialize_workspace(&state).expect("serialise");
    let envelope = deserialize_workspace(&json).expect("parse");

    let mut restored = OneCalcHostState::default();
    let _ = new_formula_space(&mut restored);
    envelope
        .apply_to_state(&mut restored)
        .expect("apply round-trip");

    // Recents carry the closed id verbatim.
    let recent_ids: Vec<_> = restored
        .workspace_shell
        .recent_formula_space_order
        .iter()
        .map(|id| id.as_str().to_string())
        .collect();
    assert!(
        recent_ids.contains(&id.as_str().to_string()),
        "recents must carry {id:?}; got {recent_ids:?}",
    );
}
