use crate::domain::ids::FormulaSpaceId;
use crate::services::ambient_app_context::detect_ambient_app_context_for_platform;
use crate::state::{FormulaSpaceState, OneCalcHostState};

/// Seed for the WS-14 home shell — a single empty `untitled-1` formula
/// space, no retained-artifact catalog, no demo-mode hints. Used by
/// `mount_onecalc_preview`.
///
/// The workspace's `AmbientAppContext` is initialised from the
/// platform (browser `navigator.language` on wasm, ISO defaults on
/// SSR builds) so `=NOW()` and `=TODAY()` show up in the user's
/// regional shape on first render. Users can override later through
/// the workspace preferences (UI knob is a SEAM-pending follow-up;
/// the state slot is in place).
pub fn preview_minimal_host_state() -> OneCalcHostState {
    let mut state = OneCalcHostState::default();
    state.ambient_app_context = detect_ambient_app_context_for_platform();
    let space_id = FormulaSpaceId::new("untitled-1");
    state.workspace_shell.active_formula_space_id = Some(space_id.clone());
    state
        .workspace_shell
        .open_formula_space_order
        .push(space_id.clone());
    state
        .formula_spaces
        .insert(FormulaSpaceState::new(space_id, ""));
    state
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the fixture shape of `preview_minimal_host_state` — single
    /// empty formula space, no retained-artifact catalog.
    #[test]
    fn preview_minimal_host_state_seeds_single_untitled_formula_space() {
        let state = preview_minimal_host_state();

        // Exactly one formula space, keyed `untitled-1`, active.
        assert_eq!(state.formula_spaces.spaces.len(), 1);
        let active_id = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .expect("active formula space id seeded");
        assert_eq!(active_id.as_str(), "untitled-1");

        let formula_space = state
            .formula_spaces
            .get(active_id)
            .expect("active formula space present");
        assert_eq!(formula_space.raw_entered_cell_text, "");
        assert!(formula_space.editor_document.is_none());
        assert!(formula_space.committed_cell_text.is_none());

        // Open order contains exactly one entry; recents and pinned empty.
        assert_eq!(state.workspace_shell.open_formula_space_order.len(), 1);
        assert!(state.workspace_shell.recent_formula_space_order.is_empty());
        assert!(state.workspace_shell.recent_formula_spaces.is_empty());
        assert!(state.workspace_shell.pinned_formula_space_ids.is_empty());

        // No retained-artifact catalog entries.
        assert!(state.retained_artifacts.catalog.is_empty());
        assert!(state.retained_artifacts.open_artifact_id.is_none());
    }
}
