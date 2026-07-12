//! Case / formula-space lifecycle reducer functions.
//!
//! Covers the shell-level actions the rail surfaces: create a fresh formula
//! space, rename it, duplicate it, close it, toggle pinned status. Every
//! function is a pure mutation on `OneCalcHostState` so `OneCalcShellApp` can
//! wire them to callbacks without threading additional services.

use crate::domain::ids::FormulaSpaceId;
use crate::state::{AppMode, ClosedFormulaSpaceRecord, FormulaSpaceState, OneCalcHostState};

const MAX_RECENT_FORMULA_SPACES: usize = 8;

/// Create a fresh empty formula space, insert it into the workspace, and
/// activate it. Returns the generated id so the caller can show toast or
/// focus-related UI.
pub fn new_formula_space(state: &mut OneCalcHostState) -> FormulaSpaceId {
    let next_index = next_untitled_index(state);
    let id_string = format!("untitled-{next_index}");
    let formula_space_id = FormulaSpaceId::new(id_string);
    let label = format!("Untitled {next_index}");

    let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "");
    formula_space.context.scenario_label = label;
    state.formula_spaces.insert(formula_space);
    state
        .workspace_shell
        .open_formula_space_order
        .push(formula_space_id.clone());
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state
        .workspace_shell
        .formula_space_modes
        .insert(formula_space_id.clone(), AppMode::Explore);
    state.workspace_shell.navigation_selection =
        crate::state::WorkspaceNavigationSelection::FormulaSpace(formula_space_id.clone());
    formula_space_id
}

fn next_untitled_index(state: &OneCalcHostState) -> usize {
    let mut max_index = 0usize;
    for id in &state.workspace_shell.open_formula_space_order {
        if let Some(rest) = id.as_str().strip_prefix("untitled-") {
            if let Ok(index) = rest.parse::<usize>() {
                if index > max_index {
                    max_index = index;
                }
            }
        }
    }
    max_index + 1
}

pub fn rename_formula_space(
    state: &mut OneCalcHostState,
    formula_space_id: &str,
    next_label: impl Into<String>,
) -> bool {
    let next_label = next_label.into();
    if next_label.trim().is_empty() {
        return false;
    }
    let id = FormulaSpaceId::new(formula_space_id.to_string());
    match state.formula_spaces.get_mut(&id) {
        Some(formula_space) => {
            formula_space.context.scenario_label = next_label;
            true
        }
        None => false,
    }
}

/// Open the inline-rename input on the tab strip for the given
/// formula. Seeds `pending_rename_text` with the current label so
/// the input shows the existing name on focus. No-op (returns
/// `false`) when the formula doesn't exist or when a rename is
/// already in progress for a different formula — caller is expected
/// to commit / cancel any in-flight rename first.
pub fn begin_formula_rename(state: &mut OneCalcHostState, formula_space_id: &str) -> bool {
    let id = FormulaSpaceId::new(formula_space_id.to_string());
    let Some(formula_space) = state.formula_spaces.get(&id) else {
        return false;
    };
    let current_label = formula_space.context.scenario_label.clone();
    state.workspace_shell.renaming_formula_space_id = Some(id);
    state.workspace_shell.pending_rename_text = current_label;
    true
}

/// Update the buffered rename text as the user types into the
/// inline rename input. The input event uses this; commit only
/// happens on Enter / blur via `commit_formula_rename`.
pub fn update_pending_rename_text(state: &mut OneCalcHostState, next_text: impl Into<String>) {
    state.workspace_shell.pending_rename_text = next_text.into();
}

/// Commit the in-flight rename. Trims whitespace; refuses to commit
/// an empty label (the rename UI keeps the input open and the
/// formula keeps its previous label). Returns `true` when the
/// formula's `scenario_label` actually changed.
pub fn commit_formula_rename(state: &mut OneCalcHostState) -> bool {
    let Some(id) = state.workspace_shell.renaming_formula_space_id.clone() else {
        return false;
    };
    let next_label = state.workspace_shell.pending_rename_text.trim().to_string();
    if next_label.is_empty() {
        // Don't commit a blank rename — leave the input open so
        // the user can either type a real label or hit Esc to
        // cancel back to the previous one.
        return false;
    }
    let changed = match state.formula_spaces.get_mut(&id) {
        Some(formula_space) => {
            let prior = formula_space.context.scenario_label.clone();
            formula_space.context.scenario_label = next_label;
            formula_space.context.scenario_label != prior
        }
        None => false,
    };
    state.workspace_shell.renaming_formula_space_id = None;
    state.workspace_shell.pending_rename_text.clear();
    changed
}

/// Cancel an in-flight rename, discarding the buffered text. The
/// formula's `scenario_label` is left unchanged.
pub fn cancel_formula_rename(state: &mut OneCalcHostState) {
    state.workspace_shell.renaming_formula_space_id = None;
    state.workspace_shell.pending_rename_text.clear();
}

pub fn duplicate_formula_space(
    state: &mut OneCalcHostState,
    formula_space_id: &str,
) -> Option<FormulaSpaceId> {
    let source_id = FormulaSpaceId::new(formula_space_id.to_string());
    let source = state.formula_spaces.get(&source_id)?.clone();
    let next_index = next_untitled_index(state);
    let new_id = FormulaSpaceId::new(format!("copy-{next_index}-of-{formula_space_id}"));

    let mut duplicate =
        FormulaSpaceState::new(new_id.clone(), source.raw_entered_cell_text.clone());
    duplicate.context = source.context.clone();
    duplicate.context.scenario_label = format!("{} (copy)", source.context.scenario_label);
    duplicate.committed_cell_text = source.committed_cell_text.clone();
    duplicate.proofed_cell_text = source.proofed_cell_text.clone();
    duplicate.expanded_editor = source.expanded_editor;
    // Per `WS14_DESIGN_BACKLOG_2026-05-04.md` §1, the clone copies
    // formatting (number-format code, font / fill colours, locale,
    // CF rules, scenario policy) and the drill-down expansion
    // state so the new formula opens with the same authoring
    // surface the user was looking at. Bridge-derived fields
    // (`editor_document`, `editor_box_metrics`, completion popup,
    // etc.) stay at their `FormulaSpaceState::new` defaults — the
    // first bridge round-trip on the new id refreshes them.
    duplicate.formatting = source.formatting.clone();
    duplicate.formula_drill_open = source.formula_drill_open;
    duplicate.formatting_panel_open = source.formatting_panel_open;

    state.formula_spaces.insert(duplicate);
    state
        .workspace_shell
        .open_formula_space_order
        .push(new_id.clone());
    state.workspace_shell.active_formula_space_id = Some(new_id.clone());
    let duplicated_mode = state
        .workspace_shell
        .formula_space_modes
        .get(&source_id)
        .copied()
        .unwrap_or(AppMode::Explore);
    state
        .workspace_shell
        .formula_space_modes
        .insert(new_id.clone(), duplicated_mode);
    state.workspace_shell.navigation_selection =
        crate::state::WorkspaceNavigationSelection::FormulaSpace(new_id.clone());
    Some(new_id)
}

/// Convenience wrapper around [`duplicate_formula_space`] that
/// targets the workspace's *active* formula. Returns the new id
/// or `None` when there is no active formula. Per WS14 §1
/// "Clone vs. duplicate vs. save-as": the term Clone is what the
/// user surfaces (breadcrumb dropdown action label); internally
/// this re-uses the existing duplicate machinery.
pub fn clone_active_formula_space(state: &mut OneCalcHostState) -> Option<FormulaSpaceId> {
    let active_id = state.workspace_shell.active_formula_space_id.clone()?;
    duplicate_formula_space(state, active_id.as_str())
}

/// Pin the workspace's *active* formula. Idempotent — returns
/// `true` on the first pin and `false` when the formula was
/// already pinned (or there is no active formula). Per
/// `workspace_shell.pinned_formula_space_ids` semantics, pinned
/// ids survive workspace-level cleanup and surface in the
/// scenario-breadcrumb dropdown's "Pinned" section.
pub fn pin_active_formula_space(state: &mut OneCalcHostState) -> bool {
    let Some(active_id) = state.workspace_shell.active_formula_space_id.clone() else {
        return false;
    };
    if state
        .workspace_shell
        .pinned_formula_space_ids
        .contains(&active_id)
    {
        return false;
    }
    state
        .workspace_shell
        .pinned_formula_space_ids
        .insert(active_id);
    true
}

/// Remove the formula with the given id from the workspace's
/// pinned set. Returns `true` when the pin was present, `false`
/// when the formula wasn't pinned (no-op).
pub fn unpin_formula_space(state: &mut OneCalcHostState, formula_space_id: &str) -> bool {
    let id = FormulaSpaceId::new(formula_space_id.to_string());
    state.workspace_shell.pinned_formula_space_ids.remove(&id)
}

/// Pin the formula with the given id. Used by the
/// manage-formulas overlay's per-row pin button — that surface
/// can target a non-active formula, so the active-only helper
/// `pin_active_formula_space` doesn't fit. Returns `true` when
/// the pin was newly added; `false` when already pinned (no-op).
pub fn pin_formula_space(state: &mut OneCalcHostState, formula_space_id: &str) -> bool {
    let id = FormulaSpaceId::new(formula_space_id.to_string());
    state.workspace_shell.pinned_formula_space_ids.insert(id)
}

pub fn close_formula_space(state: &mut OneCalcHostState, formula_space_id: &str) -> bool {
    let id = FormulaSpaceId::new(formula_space_id.to_string());
    let Some(closed_formula_space) = state.formula_spaces.spaces.remove(&id) else {
        return false;
    };
    state
        .workspace_shell
        .open_formula_space_order
        .retain(|candidate| candidate != &id);
    state.workspace_shell.pinned_formula_space_ids.remove(&id);
    let last_active_mode = state
        .workspace_shell
        .formula_space_modes
        .remove(&id)
        .unwrap_or(AppMode::Explore);
    remember_recent_formula_space(state, closed_formula_space, last_active_mode);

    let was_active = state
        .workspace_shell
        .active_formula_space_id
        .as_ref()
        .map(|active| active == &id)
        .unwrap_or(false);
    if was_active {
        state.workspace_shell.active_formula_space_id = state
            .workspace_shell
            .open_formula_space_order
            .first()
            .cloned();
        if let Some(next_active_formula_space_id) =
            state.workspace_shell.active_formula_space_id.as_ref()
        {
            state.active_formula_space_view.active_mode = state
                .workspace_shell
                .formula_space_modes
                .get(next_active_formula_space_id)
                .copied()
                .unwrap_or(AppMode::Explore);
            state.active_formula_space_view.selected_formula_space_id =
                Some(next_active_formula_space_id.clone());
        }
    }

    // Keep the workspace from ever being empty: if closing the last space
    // leaves nothing open, spin a fresh Untitled so the editor still has a
    // surface to mount against.
    if state.workspace_shell.open_formula_space_order.is_empty() {
        let _ = new_formula_space(state);
    }
    true
}

pub fn reopen_formula_space(state: &mut OneCalcHostState, formula_space_id: &str) -> bool {
    let id = FormulaSpaceId::new(formula_space_id.to_string());
    if state
        .workspace_shell
        .open_formula_space_order
        .iter()
        .any(|candidate| candidate == &id)
    {
        state.workspace_shell.active_formula_space_id = Some(id.clone());
        state.active_formula_space_view.selected_formula_space_id = Some(id.clone());
        state.active_formula_space_view.active_mode = state
            .workspace_shell
            .formula_space_modes
            .get(&id)
            .copied()
            .unwrap_or(AppMode::Explore);
        state.workspace_shell.navigation_selection =
            crate::state::WorkspaceNavigationSelection::FormulaSpace(id);
        return true;
    }

    let Some(record) = state.workspace_shell.recent_formula_spaces.remove(&id) else {
        return false;
    };
    state
        .workspace_shell
        .recent_formula_space_order
        .retain(|candidate| candidate != &id);
    state.formula_spaces.insert(record.formula_space);
    state
        .workspace_shell
        .open_formula_space_order
        .push(id.clone());
    state.workspace_shell.active_formula_space_id = Some(id.clone());
    state.active_formula_space_view.selected_formula_space_id = Some(id.clone());
    state.active_formula_space_view.active_mode = record.last_active_mode;
    state
        .workspace_shell
        .formula_space_modes
        .insert(id.clone(), record.last_active_mode);
    state.workspace_shell.navigation_selection =
        crate::state::WorkspaceNavigationSelection::FormulaSpace(id);
    true
}

/// Forget a closed formula entirely — drops it from
/// `recent_formula_spaces` and `recent_formula_space_order`. Used
/// by the manage-formulas overlay's per-row "Forget" action so the
/// user can prune recents they don't want surfaced anymore. No-op
/// (returns `false`) when the id isn't in the recents list, since
/// open formulas use `close_formula_space` instead.
pub fn forget_recent_formula_space(state: &mut OneCalcHostState, formula_space_id: &str) -> bool {
    let id = FormulaSpaceId::new(formula_space_id.to_string());
    if state
        .workspace_shell
        .recent_formula_spaces
        .remove(&id)
        .is_none()
    {
        return false;
    }
    state
        .workspace_shell
        .recent_formula_space_order
        .retain(|candidate| candidate != &id);
    // A pinned-then-forgotten formula has nothing to pin to anymore.
    state.workspace_shell.pinned_formula_space_ids.remove(&id);
    true
}

/// Insert a loaded `Scenario` into the workspace as a new formula
/// space and switch to it. Used by the breadcrumb's `Open…` action
/// in slice 1b. Reuses the loaded scenario's `identity.id` when
/// possible; appends a numeric suffix when the id is already taken
/// in this workspace, so opening the same file twice does not
/// silently overwrite the first instance.
///
/// The previously-active formula space is preserved in
/// `workspace_shell.open_formula_space_order` and remains in the
/// `formula_spaces` map — the user can switch back via the
/// breadcrumb dropdown / command palette.
pub fn open_loaded_scenario_into_workspace(
    state: &mut OneCalcHostState,
    loaded: crate::persistence::LoadedFormula,
) -> FormulaSpaceId {
    let crate::persistence::LoadedFormula {
        scenario,
        diagnostics,
    } = loaded;
    let id = derive_unique_formula_space_id(state, &scenario.identity.id);
    let mut formula_space = FormulaSpaceState::new(id.clone(), &scenario.entry.text);
    crate::persistence::apply_loaded_scenario_with_diagnostics(
        &mut formula_space,
        scenario,
        diagnostics,
    );

    state.formula_spaces.insert(formula_space);
    if !state
        .workspace_shell
        .open_formula_space_order
        .iter()
        .any(|existing| existing == &id)
    {
        state
            .workspace_shell
            .open_formula_space_order
            .push(id.clone());
    }
    state.workspace_shell.active_formula_space_id = Some(id.clone());
    state
        .workspace_shell
        .formula_space_modes
        .insert(id.clone(), AppMode::Explore);
    state.workspace_shell.navigation_selection =
        crate::state::WorkspaceNavigationSelection::FormulaSpace(id.clone());
    id
}

fn derive_unique_formula_space_id(state: &OneCalcHostState, candidate: &str) -> FormulaSpaceId {
    let trimmed = candidate.trim();
    let base = if trimmed.is_empty() {
        "imported".to_string()
    } else {
        trimmed.to_string()
    };

    let candidate_id = FormulaSpaceId::new(base.clone());
    if !state.formula_spaces.spaces.contains_key(&candidate_id) {
        return candidate_id;
    }

    for counter in 1usize.. {
        let try_id = FormulaSpaceId::new(format!("{base}-{counter}"));
        if !state.formula_spaces.spaces.contains_key(&try_id) {
            return try_id;
        }
    }
    unreachable!("usize range exhausted")
}

pub fn toggle_pin_formula_space(state: &mut OneCalcHostState, formula_space_id: &str) -> bool {
    let id = FormulaSpaceId::new(formula_space_id.to_string());
    if state.formula_spaces.get(&id).is_none() {
        return false;
    }
    if state.workspace_shell.pinned_formula_space_ids.contains(&id) {
        state.workspace_shell.pinned_formula_space_ids.remove(&id);
    } else {
        state.workspace_shell.pinned_formula_space_ids.insert(id);
    }
    true
}

fn remember_recent_formula_space(
    state: &mut OneCalcHostState,
    formula_space: FormulaSpaceState,
    last_active_mode: AppMode,
) {
    let id = formula_space.formula_space_id.clone();
    state
        .workspace_shell
        .recent_formula_space_order
        .retain(|candidate| candidate != &id);
    state.workspace_shell.recent_formula_spaces.insert(
        id.clone(),
        ClosedFormulaSpaceRecord {
            formula_space,
            last_active_mode,
        },
    );
    state
        .workspace_shell
        .recent_formula_space_order
        .insert(0, id);
    while state.workspace_shell.recent_formula_space_order.len() > MAX_RECENT_FORMULA_SPACES {
        if let Some(removed_id) = state.workspace_shell.recent_formula_space_order.pop() {
            state
                .workspace_shell
                .recent_formula_spaces
                .remove(&removed_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_state_with_space(id: &str) -> OneCalcHostState {
        let mut state = OneCalcHostState::default();
        let formula_space_id = FormulaSpaceId::new(id.to_string());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space_id.clone());
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=1"));
        state
    }

    #[test]
    fn new_formula_space_inserts_untitled_and_activates_it() {
        let mut state = OneCalcHostState::default();
        let id = new_formula_space(&mut state);
        assert_eq!(id.as_str(), "untitled-1");
        assert_eq!(state.formula_spaces.spaces.len(), 1);
        assert_eq!(
            state.workspace_shell.active_formula_space_id,
            Some(id.clone())
        );
        assert_eq!(state.workspace_shell.open_formula_space_order, vec![id]);
        assert_eq!(
            state
                .workspace_shell
                .formula_space_modes
                .get(&FormulaSpaceId::new("untitled-1")),
            Some(&AppMode::Explore)
        );
    }

    #[test]
    fn new_formula_space_uses_incrementing_index() {
        let mut state = OneCalcHostState::default();
        let first = new_formula_space(&mut state);
        let second = new_formula_space(&mut state);
        assert_eq!(first.as_str(), "untitled-1");
        assert_eq!(second.as_str(), "untitled-2");
    }

    #[test]
    fn rename_updates_scenario_label_when_id_matches() {
        let mut state = fresh_state_with_space("space-1");
        assert!(rename_formula_space(&mut state, "space-1", "Renamed"));
        assert_eq!(
            state
                .formula_spaces
                .get(&FormulaSpaceId::new("space-1".to_string()))
                .unwrap()
                .context
                .scenario_label,
            "Renamed"
        );
    }

    #[test]
    fn rename_rejects_empty_label() {
        let mut state = fresh_state_with_space("space-1");
        assert!(!rename_formula_space(&mut state, "space-1", "   "));
    }

    #[test]
    fn begin_rename_seeds_buffer_with_existing_label_and_marks_target() {
        let mut state = fresh_state_with_space("space-1");
        let id = FormulaSpaceId::new("space-1".to_string());
        state
            .formula_spaces
            .get_mut(&id)
            .unwrap()
            .context
            .scenario_label = "My formula".to_string();

        assert!(begin_formula_rename(&mut state, "space-1"));
        assert_eq!(
            state.workspace_shell.renaming_formula_space_id.as_ref(),
            Some(&id),
        );
        assert_eq!(state.workspace_shell.pending_rename_text, "My formula");
    }

    #[test]
    fn begin_rename_returns_false_for_unknown_id() {
        let mut state = fresh_state_with_space("space-1");
        assert!(!begin_formula_rename(&mut state, "does-not-exist"));
        assert!(state.workspace_shell.renaming_formula_space_id.is_none());
    }

    #[test]
    fn commit_rename_writes_buffer_and_clears_state() {
        let mut state = fresh_state_with_space("space-1");
        assert!(begin_formula_rename(&mut state, "space-1"));
        update_pending_rename_text(&mut state, "Renamed live");
        assert!(commit_formula_rename(&mut state));
        let id = FormulaSpaceId::new("space-1".to_string());
        assert_eq!(
            state
                .formula_spaces
                .get(&id)
                .unwrap()
                .context
                .scenario_label,
            "Renamed live",
        );
        assert!(state.workspace_shell.renaming_formula_space_id.is_none());
        assert!(state.workspace_shell.pending_rename_text.is_empty());
    }

    #[test]
    fn commit_rename_with_empty_buffer_keeps_input_open() {
        let mut state = fresh_state_with_space("space-1");
        assert!(begin_formula_rename(&mut state, "space-1"));
        update_pending_rename_text(&mut state, "   ");
        assert!(!commit_formula_rename(&mut state));
        // The rename remains in progress so the user can type a real
        // label or hit Esc to cancel.
        assert!(state.workspace_shell.renaming_formula_space_id.is_some());
    }

    #[test]
    fn cancel_rename_discards_buffer_and_leaves_label() {
        let mut state = fresh_state_with_space("space-1");
        let id = FormulaSpaceId::new("space-1".to_string());
        state
            .formula_spaces
            .get_mut(&id)
            .unwrap()
            .context
            .scenario_label = "Original".to_string();

        assert!(begin_formula_rename(&mut state, "space-1"));
        update_pending_rename_text(&mut state, "Discarded");
        cancel_formula_rename(&mut state);
        assert_eq!(
            state
                .formula_spaces
                .get(&id)
                .unwrap()
                .context
                .scenario_label,
            "Original"
        );
        assert!(state.workspace_shell.renaming_formula_space_id.is_none());
        assert!(state.workspace_shell.pending_rename_text.is_empty());
    }

    #[test]
    fn duplicate_creates_new_space_with_suffix_label() {
        let mut state = fresh_state_with_space("space-1");
        let new_id = duplicate_formula_space(&mut state, "space-1").expect("duplicated");
        assert!(new_id.as_str().contains("copy-"));
        let duplicate = state.formula_spaces.get(&new_id).unwrap();
        assert!(duplicate.context.scenario_label.ends_with("(copy)"));
        assert_eq!(duplicate.raw_entered_cell_text, "=1");
        assert_eq!(
            state.workspace_shell.active_formula_space_id.as_ref(),
            Some(&new_id)
        );
    }

    #[test]
    fn close_removes_space_and_activates_another_when_present() {
        let mut state = fresh_state_with_space("space-1");
        state
            .workspace_shell
            .open_formula_space_order
            .push(FormulaSpaceId::new("space-2".to_string()));
        state.formula_spaces.insert(FormulaSpaceState::new(
            FormulaSpaceId::new("space-2".to_string()),
            "=2",
        ));

        assert!(close_formula_space(&mut state, "space-1"));
        assert!(state
            .formula_spaces
            .get(&FormulaSpaceId::new("space-1".to_string()))
            .is_none());
        assert_eq!(
            state.workspace_shell.active_formula_space_id,
            Some(FormulaSpaceId::new("space-2".to_string()))
        );
    }

    #[test]
    fn close_last_space_creates_a_fresh_untitled() {
        let mut state = fresh_state_with_space("space-1");
        close_formula_space(&mut state, "space-1");
        assert_eq!(state.formula_spaces.spaces.len(), 1);
        assert_eq!(state.workspace_shell.recent_formula_space_order.len(), 1);
        let active_id = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .unwrap();
        assert!(active_id.as_str().starts_with("untitled-"));
    }

    #[test]
    fn forget_recent_drops_the_record_from_recents() {
        let mut state = fresh_state_with_space("space-1");
        assert!(close_formula_space(&mut state, "space-1"));
        assert!(state
            .workspace_shell
            .recent_formula_spaces
            .contains_key(&FormulaSpaceId::new("space-1".to_string())));

        assert!(forget_recent_formula_space(&mut state, "space-1"));
        assert!(!state
            .workspace_shell
            .recent_formula_spaces
            .contains_key(&FormulaSpaceId::new("space-1".to_string())));
        assert!(!state
            .workspace_shell
            .recent_formula_space_order
            .iter()
            .any(|id| id.as_str() == "space-1"));
    }

    /// Defensive: if a formula is somehow pinned AND in recents
    /// (a future workflow could let that happen), forgetting the
    /// recent should also clear the pin so the user doesn't see a
    /// pinned ghost in the breadcrumb after they "forget" it.
    #[test]
    fn forget_recent_clears_a_pinned_recent_too() {
        let mut state = fresh_state_with_space("space-1");
        assert!(close_formula_space(&mut state, "space-1"));
        // Inject a manual pin on the closed id to model the
        // hypothetical workflow.
        state
            .workspace_shell
            .pinned_formula_space_ids
            .insert(FormulaSpaceId::new("space-1".to_string()));

        assert!(forget_recent_formula_space(&mut state, "space-1"));
        assert!(!state
            .workspace_shell
            .pinned_formula_space_ids
            .contains(&FormulaSpaceId::new("space-1".to_string())));
    }

    #[test]
    fn forget_recent_returns_false_for_unknown_or_open_id() {
        let mut state = fresh_state_with_space("space-1");
        // Open formulas aren't recents — the caller should use
        // close_formula_space instead.
        assert!(!forget_recent_formula_space(&mut state, "space-1"));
        // Unknown id is also a no-op.
        assert!(!forget_recent_formula_space(&mut state, "does-not-exist"));
    }

    #[test]
    fn pin_formula_space_inserts_idempotently() {
        let mut state = fresh_state_with_space("space-1");
        assert!(pin_formula_space(&mut state, "space-1"));
        assert!(!pin_formula_space(&mut state, "space-1"));
        assert!(state
            .workspace_shell
            .pinned_formula_space_ids
            .contains(&FormulaSpaceId::new("space-1".to_string())));
    }

    #[test]
    fn close_archives_formula_space_for_reopen() {
        let mut state = fresh_state_with_space("space-1");
        state.active_formula_space_view.active_mode = AppMode::Inspect;
        state
            .workspace_shell
            .formula_space_modes
            .insert(FormulaSpaceId::new("space-1".to_string()), AppMode::Inspect);

        assert!(close_formula_space(&mut state, "space-1"));

        let archived = state
            .workspace_shell
            .recent_formula_spaces
            .get(&FormulaSpaceId::new("space-1".to_string()))
            .expect("archived formula space");
        assert_eq!(archived.formula_space.raw_entered_cell_text, "=1");
        assert_eq!(archived.last_active_mode, AppMode::Inspect);
    }

    #[test]
    fn reopen_restores_formula_space_and_mode() {
        let mut state = fresh_state_with_space("space-1");
        state.active_formula_space_view.active_mode = AppMode::Workbench;
        state.workspace_shell.formula_space_modes.insert(
            FormulaSpaceId::new("space-1".to_string()),
            AppMode::Workbench,
        );
        assert!(close_formula_space(&mut state, "space-1"));

        assert!(reopen_formula_space(&mut state, "space-1"));
        assert!(state
            .workspace_shell
            .recent_formula_spaces
            .get(&FormulaSpaceId::new("space-1".to_string()))
            .is_none());
        assert!(state
            .workspace_shell
            .open_formula_space_order
            .contains(&FormulaSpaceId::new("space-1".to_string())));
        assert_eq!(
            state.active_formula_space_view.active_mode,
            AppMode::Workbench
        );
    }

    #[test]
    fn toggle_pin_flips_membership() {
        let mut state = fresh_state_with_space("space-1");
        toggle_pin_formula_space(&mut state, "space-1");
        assert!(state
            .workspace_shell
            .pinned_formula_space_ids
            .contains(&FormulaSpaceId::new("space-1".to_string())));
        toggle_pin_formula_space(&mut state, "space-1");
        assert!(!state
            .workspace_shell
            .pinned_formula_space_ids
            .contains(&FormulaSpaceId::new("space-1".to_string())));
    }
}
