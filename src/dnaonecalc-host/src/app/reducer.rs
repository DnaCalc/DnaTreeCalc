use crate::services::completion_popup::{
    accept_selected_completion as popup_accept_selected, dismiss_completion_popup as popup_dismiss,
    move_completion_selection as popup_move_selection, CompletionAcceptance,
};
use crate::services::retained_artifacts::{
    import_manual_artifact_for_active_formula_space, import_verification_bundle_report_json,
    open_retained_artifact_by_id, open_retained_artifact_in_inspect_by_id,
    ManualRetainedArtifactImportRequest, VerificationBundleImportRequest,
};
use crate::state::{CompletionHelpState, FormulaSpaceState, OneCalcHostState};
use crate::state::{
    VbaHostAssociationLoadStatus, VbaHostAssociationSourceKind, VbaHostAssociationState,
};
use crate::ui::editor::commands::{
    apply_editor_command, cycle_completion_selection, EditorCommand, EditorInputEvent,
};
use crate::ui::editor::geometry::{EditorOverlayMeasurementEvent, TextareaMeasurementMetrics};
use crate::ui::editor::state::EditorSurfaceState;

pub fn apply_editor_input_to_active_formula_space(
    state: &mut OneCalcHostState,
    input_event: EditorInputEvent,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };

    let next_editor_state = if let (Some(selection_start), Some(selection_end)) =
        (input_event.selection_start, input_event.selection_end)
    {
        EditorSurfaceState::for_text_with_selection(
            &input_event.text,
            selection_start,
            selection_end,
        )
    } else {
        EditorSurfaceState::for_text(&input_event.text)
    };
    apply_local_editor_text_change(formula_space, input_event.text, next_editor_state);
    true
}

pub fn apply_skin_intent_to_host_state(
    state: &mut OneCalcHostState,
    intent: dnacalc_skin_ir::SkinIntent,
) -> bool {
    match intent {
        dnacalc_skin_ir::SkinIntent::OneFormula(intent) => {
            apply_one_formula_skin_intent(state, intent)
        }
        dnacalc_skin_ir::SkinIntent::Shell(intent) => apply_shell_skin_intent(state, intent),
        dnacalc_skin_ir::SkinIntent::TreeWorkspace(_) => false,
    }
}

fn apply_shell_skin_intent(
    state: &mut OneCalcHostState,
    intent: dnacalc_skin_ir::SkinShellIntent,
) -> bool {
    match intent {
        dnacalc_skin_ir::SkinShellIntent::OpenCommandPalette => {
            if state.global_ui_chrome.command_palette_open {
                return false;
            }
            state.global_ui_chrome.command_palette_open = true;
            true
        }
        dnacalc_skin_ir::SkinShellIntent::CloseCommandPalette => {
            if !state.global_ui_chrome.command_palette_open {
                return false;
            }
            state.global_ui_chrome.command_palette_open = false;
            true
        }
        dnacalc_skin_ir::SkinShellIntent::SetActiveDocument { document_id } => {
            let formula_space_id = crate::domain::ids::FormulaSpaceId::new(document_id);
            if state.workspace_shell.active_formula_space_id.as_ref() == Some(&formula_space_id) {
                return false;
            }
            if !state.formula_spaces.spaces.contains_key(&formula_space_id) {
                return false;
            }
            state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
            state.active_formula_space_view.selected_formula_space_id = Some(formula_space_id);
            true
        }
        dnacalc_skin_ir::SkinShellIntent::Save => {
            state.workspace_shell.pending_persistence_intent =
                Some(crate::state::WorkspacePersistenceIntent::Save);
            true
        }
        dnacalc_skin_ir::SkinShellIntent::SaveAs { suggested_path } => {
            state.workspace_shell.pending_persistence_intent =
                Some(crate::state::WorkspacePersistenceIntent::SaveAs { suggested_path });
            true
        }
        dnacalc_skin_ir::SkinShellIntent::Open { requested_path } => {
            state.workspace_shell.pending_persistence_intent =
                Some(crate::state::WorkspacePersistenceIntent::Open { requested_path });
            true
        }
        dnacalc_skin_ir::SkinShellIntent::OpenRecent { document_id } => {
            crate::app::case_lifecycle::reopen_formula_space(state, &document_id)
        }
    }
}

fn apply_one_formula_skin_intent(
    state: &mut OneCalcHostState,
    intent: dnacalc_skin_ir::OneFormulaIntent,
) -> bool {
    match intent {
        dnacalc_skin_ir::OneFormulaIntent::EditText {
            formula_space_id,
            text,
            caret_offset,
        } => {
            let formula_space_id = crate::domain::ids::FormulaSpaceId::new(formula_space_id);
            let Some(formula_space) = state.formula_spaces.get_mut(&formula_space_id) else {
                return false;
            };
            let caret_offset = caret_offset.min(text.len());
            let next_editor_state =
                EditorSurfaceState::for_text_with_selection(&text, caret_offset, caret_offset);
            apply_local_editor_text_change(formula_space, text, next_editor_state);
            true
        }
        dnacalc_skin_ir::OneFormulaIntent::SetSelection {
            formula_space_id,
            anchor,
            focus,
        } => {
            let formula_space_id = crate::domain::ids::FormulaSpaceId::new(formula_space_id);
            let Some(formula_space) = state.formula_spaces.get_mut(&formula_space_id) else {
                return false;
            };
            let len = formula_space.raw_entered_cell_text.len();
            let next_editor_state = EditorSurfaceState::for_text_with_selection(
                &formula_space.raw_entered_cell_text,
                anchor.min(len),
                focus.min(len),
            );
            if formula_space.editor_surface_state == next_editor_state {
                return false;
            }
            formula_space.editor_surface_state = next_editor_state;
            true
        }
        dnacalc_skin_ir::OneFormulaIntent::ApplyCompletion {
            formula_space_id,
            proposal_id,
        } => with_active_formula_space(state, formula_space_id, |state| {
            accept_completion_by_proposal_id_on_active_formula_space(state, &proposal_id).is_some()
        }),
        dnacalc_skin_ir::OneFormulaIntent::SetNumberFormat {
            formula_space_id,
            number_format_code,
        } => with_active_formula_space(state, formula_space_id, |state| {
            set_active_number_format_code(state, number_format_code.unwrap_or_default())
        }),
        dnacalc_skin_ir::OneFormulaIntent::SetScenarioPolicy {
            formula_space_id,
            policy,
        } => with_active_formula_space(state, formula_space_id, |state| {
            set_active_scenario_policy(state, skin_scenario_policy_to_host(policy))
        }),
        dnacalc_skin_ir::OneFormulaIntent::ToggleFormulaDrill { formula_space_id } => {
            with_active_formula_space(state, formula_space_id, |state| {
                let before = active_formula_space_mut(state)
                    .map(|formula_space| formula_space.formula_drill_open)
                    .unwrap_or(false);
                let after = toggle_formula_drill_on_active_formula_space(state);
                before != after
            })
        }
        dnacalc_skin_ir::OneFormulaIntent::Recalculate { formula_space_id } => {
            let formula_space_id = crate::domain::ids::FormulaSpaceId::new(formula_space_id);
            let Some(formula_space) = state.formula_spaces.get_mut(&formula_space_id) else {
                return false;
            };
            formula_space.editor_document = None;
            formula_space.latest_evaluation_summary = None;
            formula_space.effective_display_summary = None;
            true
        }
        dnacalc_skin_ir::OneFormulaIntent::RequestResultArrayWindow { .. }
        | dnacalc_skin_ir::OneFormulaIntent::RequestDrillArrayWindow { .. } => false,
    }
}

fn with_active_formula_space(
    state: &mut OneCalcHostState,
    formula_space_id: String,
    f: impl FnOnce(&mut OneCalcHostState) -> bool,
) -> bool {
    let formula_space_id = crate::domain::ids::FormulaSpaceId::new(formula_space_id);
    if !state.formula_spaces.spaces.contains_key(&formula_space_id) {
        return false;
    }
    let previous_workspace_active = state.workspace_shell.active_formula_space_id.clone();
    let previous_selected = state
        .active_formula_space_view
        .selected_formula_space_id
        .clone();
    state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
    state.active_formula_space_view.selected_formula_space_id = Some(formula_space_id);
    let changed = f(state);
    state.workspace_shell.active_formula_space_id = previous_workspace_active;
    state.active_formula_space_view.selected_formula_space_id = previous_selected;
    changed
}

fn skin_scenario_policy_to_host(
    policy: dnacalc_skin_ir::ScenarioPolicyProjection,
) -> crate::persistence::ScenarioPolicy {
    match policy {
        dnacalc_skin_ir::ScenarioPolicyProjection::Deterministic => {
            crate::persistence::ScenarioPolicy::Deterministic
        }
        dnacalc_skin_ir::ScenarioPolicyProjection::LiveRecalc => {
            crate::persistence::ScenarioPolicy::LiveRecalc
        }
        dnacalc_skin_ir::ScenarioPolicyProjection::ManualRecalc => {
            crate::persistence::ScenarioPolicy::ManualRecalc
        }
    }
}

pub fn apply_editor_command_to_active_formula_space(
    state: &mut OneCalcHostState,
    command: EditorCommand,
) -> bool {
    match &command {
        EditorCommand::ToggleEditorSettingsPopover => {
            return toggle_editor_settings_popover(state);
        }
        EditorCommand::UpdateEditorSetting(update) => {
            return update_editor_setting(state, *update);
        }
        EditorCommand::ToggleConfigureDrawer => {
            return toggle_configure_drawer(state);
        }
        _ => {}
    }

    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };

    if apply_completion_command(formula_space, &command) {
        return true;
    }

    if apply_live_state_command(formula_space, &command) {
        return true;
    }

    let result = apply_editor_command(
        &formula_space.raw_entered_cell_text,
        &formula_space.editor_surface_state,
        command,
    );
    apply_local_editor_text_change(formula_space, result.text, result.state);
    true
}

fn apply_live_state_command(
    formula_space: &mut FormulaSpaceState,
    command: &EditorCommand,
) -> bool {
    // UI chrome commands are handled at the host level; delegate them back up.
    if matches!(
        command,
        EditorCommand::ToggleEditorSettingsPopover
            | EditorCommand::UpdateEditorSetting(_)
            | EditorCommand::ToggleConfigureDrawer
    ) {
        return false;
    }
    match command {
        EditorCommand::CommitEntry => {
            formula_space.committed_cell_text = Some(formula_space.raw_entered_cell_text.clone());
            formula_space.proofed_cell_text = Some(formula_space.raw_entered_cell_text.clone());
            true
        }
        EditorCommand::RequestProof => {
            formula_space.proofed_cell_text = Some(formula_space.raw_entered_cell_text.clone());
            true
        }
        EditorCommand::CancelEntry => {
            if let Some(committed) = formula_space.committed_cell_text.clone() {
                let restored_state = EditorSurfaceState::for_text(&committed);
                apply_local_editor_text_change(formula_space, committed.clone(), restored_state);
                formula_space.committed_cell_text = Some(committed.clone());
                formula_space.proofed_cell_text = Some(committed);
            }
            true
        }
        EditorCommand::ToggleExpandedHeight => {
            formula_space.expanded_editor = !formula_space.expanded_editor;
            true
        }
        EditorCommand::DismissCompletion => {
            formula_space.editor_surface_state.completion_anchor_offset = None;
            formula_space.editor_surface_state.completion_selected_index = None;
            true
        }
        EditorCommand::CycleReferenceForm => {
            let selection = &formula_space.editor_surface_state.selection;
            if let Some(result) = crate::ui::editor::reference_cycle::cycle_reference_form(
                &formula_space.raw_entered_cell_text,
                selection.start(),
                selection.end(),
            ) {
                let next_state = crate::ui::editor::state::EditorSurfaceState {
                    caret: crate::ui::editor::state::EditorCaret {
                        offset: result.reference_end,
                    },
                    selection: crate::ui::editor::state::EditorSelection {
                        anchor: result.reference_start,
                        focus: result.reference_end,
                    },
                    scroll_window: formula_space.editor_surface_state.scroll_window.clone(),
                    completion_anchor_offset: None,
                    completion_selected_index: None,
                    signature_help_anchor_offset: None,
                };
                apply_local_editor_text_change(formula_space, result.text, next_state);
            }
            true
        }
        EditorCommand::ForceShowCompletion | EditorCommand::SendSelectionToInspect => {
            // Model-level no-ops that the view layer or downstream services consume.
            true
        }
        _ => false,
    }
}

pub fn update_editor_setting(
    state: &mut OneCalcHostState,
    update: crate::ui::editor::state::EditorSettingUpdate,
) -> bool {
    state.global_ui_chrome.editor_settings.apply(update);
    true
}

pub fn toggle_editor_settings_popover(state: &mut OneCalcHostState) -> bool {
    state.global_ui_chrome.editor_settings_popover_open =
        !state.global_ui_chrome.editor_settings_popover_open;
    true
}

pub fn toggle_configure_drawer(state: &mut OneCalcHostState) -> bool {
    state.global_ui_chrome.configure_drawer_open = !state.global_ui_chrome.configure_drawer_open;
    true
}

pub fn set_pending_vba_project_path(state: &mut OneCalcHostState, path: impl Into<String>) -> bool {
    let path = path.into();
    if state.vba_host_context.pending_project_path == path {
        return false;
    }
    state.vba_host_context.pending_project_path = path;
    true
}

pub fn add_pending_vba_project_path(state: &mut OneCalcHostState) -> bool {
    let source_ref = state
        .vba_host_context
        .pending_project_path
        .trim()
        .to_string();
    if source_ref.is_empty() {
        return false;
    }
    add_vba_host_association(
        state,
        source_ref.clone(),
        source_ref,
        VbaHostAssociationSourceKind::ProjectPath,
    );
    state.vba_host_context.pending_project_path.clear();
    true
}

pub fn add_vba_module_source_for_host_context(
    state: &mut OneCalcHostState,
    display_name: impl Into<String>,
    source_label: impl Into<String>,
    admitted_udfs: Vec<String>,
    rejected_candidates: Vec<String>,
) -> bool {
    let display_name = display_name.into();
    let source_label = source_label.into();
    if display_name.trim().is_empty() || source_label.trim().is_empty() {
        return false;
    }
    let association = next_vba_association(
        state,
        display_name,
        source_label,
        VbaHostAssociationSourceKind::ModuleSource,
        VbaHostAssociationLoadStatus::PendingLoad,
        admitted_udfs,
        rejected_candidates,
    );
    state.vba_host_context.associations.push(association);
    true
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VbaModuleSourceLoadRequest {
    pub display_name: String,
    pub source_ref: String,
    pub source_text: String,
    pub diagnostics: Vec<String>,
}

impl VbaModuleSourceLoadRequest {
    pub fn browser_file(file_name: String, source_text: String) -> Self {
        Self {
            source_ref: format!("browser-file:{file_name}"),
            display_name: file_name,
            source_text,
            diagnostics: Vec::new(),
        }
    }

    pub fn native_file(
        display_name: String,
        source_ref: String,
        source_text: String,
        diagnostics: Vec<String>,
    ) -> Self {
        Self {
            display_name,
            source_ref,
            source_text,
            diagnostics,
        }
    }
}

/// Record a selected `.bas` file while OxVba hosting is deferred for the
/// native OxFml/OxFunc resync. Formal OxVba loading will return through the
/// current OxVba APIs in a later integration.
pub fn load_vba_module_source_for_host_context(
    state: &mut OneCalcHostState,
    request: VbaModuleSourceLoadRequest,
) -> Option<crate::services::vba_host::VbaHostRuntime> {
    use crate::services::vba_host::{
        VbaHostRuntime, VbaProjectAssociation, VbaSourceModule, VbaSourceProject,
    };

    if request.display_name.trim().is_empty() || request.source_ref.trim().is_empty() {
        return None;
    }

    if request.source_text.trim().is_empty() {
        let diagnostic = if request.diagnostics.is_empty() {
            "selected VBA module source was empty".to_string()
        } else {
            request.diagnostics.join("; ")
        };
        let assoc = next_vba_association(
            state,
            request.display_name,
            request.source_ref,
            VbaHostAssociationSourceKind::ModuleSource,
            VbaHostAssociationLoadStatus::Failed(diagnostic),
            Vec::new(),
            Vec::new(),
        );
        state.vba_host_context.associations.push(assoc);
        return None;
    }

    let module_name = request
        .display_name
        .strip_suffix(".bas")
        .or_else(|| request.display_name.strip_suffix(".BAS"))
        .unwrap_or(&request.display_name)
        .to_string();

    let association =
        VbaProjectAssociation::workspace_source("selected-vba-module", &request.source_ref);

    let project = VbaSourceProject {
        project_name: module_name.clone(),
        modules: vec![VbaSourceModule {
            module_name: module_name.clone(),
            source: request.source_text,
        }],
    };

    match VbaHostRuntime::load_source_project_for_evaluation(
        association,
        project,
        env!("CARGO_PKG_VERSION"),
    ) {
        Ok(runtime) => {
            let admitted_udfs: Vec<String> = runtime
                .registrations()
                .iter()
                .map(|r| r.formula_name.clone())
                .collect();
            let rejected_candidates: Vec<String> = runtime
                .candidates()
                .iter()
                .filter(|c| {
                    c.admission_status != crate::services::vba_host::VbaUdfAdmissionStatus::Admitted
                })
                .map(|c| c.procedure_name.clone())
                .collect();
            let admitted_count = admitted_udfs.len();
            let rejected_count = rejected_candidates.len();
            let assoc = next_vba_association(
                state,
                request.display_name,
                request.source_ref,
                VbaHostAssociationSourceKind::ModuleSource,
                VbaHostAssociationLoadStatus::Loaded,
                admitted_udfs,
                rejected_candidates,
            );
            state
                .vba_host_context
                .associations
                .push(VbaHostAssociationState {
                    admitted_udf_count: admitted_count,
                    rejected_candidate_count: rejected_count,
                    ..assoc
                });
            Some(runtime)
        }
        Err(diagnostic) => {
            let assoc = next_vba_association(
                state,
                request.display_name,
                request.source_ref,
                VbaHostAssociationSourceKind::ModuleSource,
                VbaHostAssociationLoadStatus::Failed(diagnostic),
                Vec::new(),
                Vec::new(),
            );
            state.vba_host_context.associations.push(assoc);
            None
        }
    }
}

pub fn remove_vba_host_association(state: &mut OneCalcHostState, association_id: &str) -> bool {
    let before = state.vba_host_context.associations.len();
    state
        .vba_host_context
        .associations
        .retain(|association| association.association_id != association_id);
    before != state.vba_host_context.associations.len()
}

fn add_vba_host_association(
    state: &mut OneCalcHostState,
    display_name: String,
    source_ref: String,
    source_kind: VbaHostAssociationSourceKind,
) {
    let association = next_vba_association(
        state,
        display_name,
        source_ref,
        source_kind,
        VbaHostAssociationLoadStatus::PendingLoad,
        Vec::new(),
        Vec::new(),
    );
    state.vba_host_context.associations.push(association);
}

fn next_vba_association(
    state: &mut OneCalcHostState,
    display_name: String,
    source_ref: String,
    source_kind: VbaHostAssociationSourceKind,
    load_status: VbaHostAssociationLoadStatus,
    admitted_udfs: Vec<String>,
    rejected_candidates: Vec<String>,
) -> VbaHostAssociationState {
    state.vba_host_context.next_association_ordinal += 1;
    VbaHostAssociationState {
        association_id: format!(
            "vba-assoc-{}",
            state.vba_host_context.next_association_ordinal
        ),
        display_name,
        source_ref,
        source_kind,
        enabled: true,
        load_status,
        admitted_udf_count: admitted_udfs.len(),
        rejected_candidate_count: rejected_candidates.len(),
        admitted_udfs,
        rejected_candidates,
    }
}

/// Toggle the titlebar scenario-breadcrumb dropdown. Returns `true`
/// to signal the reducer caller that state changed.
pub fn toggle_scenario_breadcrumb(state: &mut OneCalcHostState) -> bool {
    state.global_ui_chrome.scenario_breadcrumb_open =
        !state.global_ui_chrome.scenario_breadcrumb_open;
    true
}

/// Force the scenario-breadcrumb dropdown closed (outside-click,
/// Esc). No-op when already closed; returns `false` in that case so
/// the host avoids a redundant re-render.
pub fn close_scenario_breadcrumb(state: &mut OneCalcHostState) -> bool {
    if !state.global_ui_chrome.scenario_breadcrumb_open {
        return false;
    }
    state.global_ui_chrome.scenario_breadcrumb_open = false;
    true
}

/// Open the manage-formulas overlay. Idempotent: returns `false`
/// when the overlay is already open. Always clears the search
/// query so the next session starts with the full list visible.
pub fn open_manage_formulas(state: &mut OneCalcHostState) -> bool {
    if state.global_ui_chrome.manage_formulas_open {
        return false;
    }
    state.global_ui_chrome.manage_formulas_open = true;
    state.global_ui_chrome.manage_formulas_search_query.clear();
    true
}

/// Close the manage-formulas overlay. No-op when already closed.
/// Clears the search query on close so re-opening starts fresh.
pub fn close_manage_formulas(state: &mut OneCalcHostState) -> bool {
    if !state.global_ui_chrome.manage_formulas_open {
        return false;
    }
    state.global_ui_chrome.manage_formulas_open = false;
    state.global_ui_chrome.manage_formulas_search_query.clear();
    true
}

/// Update the manage-formulas search filter as the user types
/// into the overlay's input. The renderer subscribes to this so
/// the row list refilters reactively. Stays a no-op (returns
/// `false`) when the value is unchanged.
pub fn set_manage_formulas_search_query(
    state: &mut OneCalcHostState,
    query: impl Into<String>,
) -> bool {
    let query = query.into();
    if state.global_ui_chrome.manage_formulas_search_query == query {
        return false;
    }
    state.global_ui_chrome.manage_formulas_search_query = query;
    true
}

// -----------------------------------------------------------------
// Array-browser reducers (zoom, selection, resize, display options)
// -----------------------------------------------------------------

const ARRAY_BROWSER_ZOOM_MIN: f32 = 0.5;
const ARRAY_BROWSER_ZOOM_MAX: f32 = 3.0;
const ARRAY_BROWSER_ZOOM_STEP: f32 = 0.1;
const ARRAY_BROWSER_COL_MIN_REM: f32 = 1.5;
const ARRAY_BROWSER_COL_MAX_REM: f32 = 32.0;
const ARRAY_BROWSER_ROW_MIN_REM: f32 = 1.0;
const ARRAY_BROWSER_ROW_MAX_REM: f32 = 12.0;

/// Set the active formula's array-browser zoom factor. Clamped
/// to `[0.5, 3.0]`. Returns `true` when the value actually
/// changed.
pub fn set_active_array_browser_zoom(state: &mut OneCalcHostState, zoom: f32) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    let next = zoom.clamp(ARRAY_BROWSER_ZOOM_MIN, ARRAY_BROWSER_ZOOM_MAX);
    if (formula_space.array_browser.zoom - next).abs() < f32::EPSILON {
        return false;
    }
    formula_space.array_browser.zoom = next;
    true
}

/// Step the zoom up by one notch (0.1).
pub fn zoom_in_active_array_browser(state: &mut OneCalcHostState) -> bool {
    let current = active_formula_space_mut(state)
        .map(|space| space.array_browser.zoom)
        .unwrap_or(1.0);
    set_active_array_browser_zoom(state, current + ARRAY_BROWSER_ZOOM_STEP)
}

/// Step the zoom down by one notch (0.1).
pub fn zoom_out_active_array_browser(state: &mut OneCalcHostState) -> bool {
    let current = active_formula_space_mut(state)
        .map(|space| space.array_browser.zoom)
        .unwrap_or(1.0);
    set_active_array_browser_zoom(state, current - ARRAY_BROWSER_ZOOM_STEP)
}

/// Reset zoom to 1.0.
pub fn reset_active_array_browser_zoom(state: &mut OneCalcHostState) -> bool {
    set_active_array_browser_zoom(state, 1.0)
}

/// Set the active formula's array-browser block selection. Pass
/// `None` to clear. Idempotent.
pub fn set_active_array_browser_selection(
    state: &mut OneCalcHostState,
    selection: Option<crate::state::ArrayBlockSelection>,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if formula_space.array_browser.selection == selection {
        return false;
    }
    formula_space.array_browser.selection = selection;
    true
}

/// Compute the next block selection for a column-header click. With
/// `extend = false` (plain click), the result is a strip covering
/// rows `0..=last_row` in the chosen column. With `extend = true`
/// (Shift+click), the prior selection's anchor is preserved and
/// the focus moves to `(last_row, col)`, growing the selection
/// across multiple columns. Returns `None` when `preview_rows == 0`.
pub fn compute_column_header_selection(
    prior: Option<crate::state::ArrayBlockSelection>,
    col: usize,
    preview_rows: usize,
    extend: bool,
) -> Option<crate::state::ArrayBlockSelection> {
    if preview_rows == 0 {
        return None;
    }
    let last_row = preview_rows.saturating_sub(1);
    let next = if extend {
        match prior {
            Some(prior) => crate::state::ArrayBlockSelection::from_corners(
                prior.anchor_row,
                prior.anchor_col,
                last_row,
                col,
            ),
            None => crate::state::ArrayBlockSelection::from_corners(0, col, last_row, col),
        }
    } else {
        crate::state::ArrayBlockSelection::from_corners(0, col, last_row, col)
    };
    Some(next)
}

/// Compute the next block selection for a row-header click. Mirror
/// of `compute_column_header_selection`: plain click claims row
/// `row` columns `0..=last_col`; Shift+click preserves the anchor
/// and extends the focus right to `(row, last_col)`. Returns
/// `None` when `preview_cols == 0`.
pub fn compute_row_header_selection(
    prior: Option<crate::state::ArrayBlockSelection>,
    row: usize,
    preview_cols: usize,
    extend: bool,
) -> Option<crate::state::ArrayBlockSelection> {
    if preview_cols == 0 {
        return None;
    }
    let last_col = preview_cols.saturating_sub(1);
    let next = if extend {
        match prior {
            Some(prior) => crate::state::ArrayBlockSelection::from_corners(
                prior.anchor_row,
                prior.anchor_col,
                row,
                last_col,
            ),
            None => crate::state::ArrayBlockSelection::from_corners(row, 0, row, last_col),
        }
    } else {
        crate::state::ArrayBlockSelection::from_corners(row, 0, row, last_col)
    };
    Some(next)
}

/// Compute the "select all visible cells" block — the rectangle
/// covering rows `0..=preview_rows-1` and columns
/// `0..=preview_cols-1`. Returns `None` for an empty preview window.
pub fn compute_select_all_visible_selection(
    preview_rows: usize,
    preview_cols: usize,
) -> Option<crate::state::ArrayBlockSelection> {
    if preview_rows == 0 || preview_cols == 0 {
        return None;
    }
    Some(crate::state::ArrayBlockSelection::from_corners(
        0,
        0,
        preview_rows.saturating_sub(1),
        preview_cols.saturating_sub(1),
    ))
}

/// Override the displayed width (in rem) for a single column in
/// the active formula's array browser. Clamped so the column
/// stays useful.
pub fn set_active_array_browser_column_width(
    state: &mut OneCalcHostState,
    column: usize,
    width_rem: f32,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    let next = width_rem.clamp(ARRAY_BROWSER_COL_MIN_REM, ARRAY_BROWSER_COL_MAX_REM);
    let prior = formula_space
        .array_browser
        .column_widths_rem
        .insert(column, next);
    prior.is_none() || (prior.unwrap() - next).abs() >= f32::EPSILON
}

/// Override the displayed height (in rem) for a single row in
/// the active formula's array browser. Clamped to stay useful.
pub fn set_active_array_browser_row_height(
    state: &mut OneCalcHostState,
    row: usize,
    height_rem: f32,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    let next = height_rem.clamp(ARRAY_BROWSER_ROW_MIN_REM, ARRAY_BROWSER_ROW_MAX_REM);
    let prior = formula_space
        .array_browser
        .row_heights_rem
        .insert(row, next);
    prior.is_none() || (prior.unwrap() - next).abs() >= f32::EPSILON
}

/// Toggle a workspace-level array-browser display option. The
/// `which` enum picks which knob to toggle.
pub fn toggle_array_browser_display_option(
    state: &mut OneCalcHostState,
    which: ArrayBrowserDisplayToggle,
) -> bool {
    let target = match which {
        ArrayBrowserDisplayToggle::GridLines => {
            &mut state.global_ui_chrome.array_browser_display.show_grid_lines
        }
        ArrayBrowserDisplayToggle::AlternatingRows => {
            &mut state
                .global_ui_chrome
                .array_browser_display
                .show_alternating_rows
        }
        ArrayBrowserDisplayToggle::RowColumnHeaders => {
            &mut state
                .global_ui_chrome
                .array_browser_display
                .show_row_column_headers
        }
    };
    *target = !*target;
    true
}

#[derive(Debug, Clone, Copy)]
pub enum ArrayBrowserDisplayToggle {
    GridLines,
    AlternatingRows,
    RowColumnHeaders,
}

/// Toggle the command-palette overlay. Resets the filter query
/// and selected index to the empty / 0 state on every toggle so
/// the next open starts fresh.
pub fn toggle_command_palette(state: &mut OneCalcHostState) -> bool {
    state.global_ui_chrome.command_palette_open = !state.global_ui_chrome.command_palette_open;
    state.global_ui_chrome.command_palette_query.clear();
    state.global_ui_chrome.command_palette_selected_index = 0;
    true
}

/// Force the command palette closed (outside-click, Esc). No-op
/// when already closed.
pub fn close_command_palette(state: &mut OneCalcHostState) -> bool {
    if !state.global_ui_chrome.command_palette_open {
        return false;
    }
    state.global_ui_chrome.command_palette_open = false;
    state.global_ui_chrome.command_palette_query.clear();
    state.global_ui_chrome.command_palette_selected_index = 0;
    true
}

/// Update the command palette's filter input. Resets the selected
/// index to 0 because the prior selection's position in the
/// filtered list is no longer meaningful after the query changes.
pub fn set_command_palette_query(state: &mut OneCalcHostState, query: String) -> bool {
    if state.global_ui_chrome.command_palette_query == query {
        return false;
    }
    state.global_ui_chrome.command_palette_query = query;
    state.global_ui_chrome.command_palette_selected_index = 0;
    true
}

/// Move the command palette's selected index by `delta` (signed).
/// Wraps around `total` so Up at index 0 lands on the last entry
/// and Down at the last entry lands on 0. No-op when `total == 0`.
pub fn move_command_palette_selection(
    state: &mut OneCalcHostState,
    delta: isize,
    total: usize,
) -> bool {
    if total == 0 {
        return false;
    }
    let current = state.global_ui_chrome.command_palette_selected_index as isize;
    let next = current + delta;
    let total_signed = total as isize;
    let wrapped = ((next % total_signed) + total_signed) % total_signed;
    state.global_ui_chrome.command_palette_selected_index = wrapped as usize;
    true
}

/// Slice 5 — formatting-control mutations on the active formula's
/// FormulaFormattingState. Each setter returns `true` when the value
/// actually changed (so callers can short-circuit re-renders) and
/// `false` when the value is unchanged.
pub fn set_active_number_format_code(state: &mut OneCalcHostState, value: String) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if formula_space.formatting.number_format_code == value {
        return false;
    }
    formula_space.formatting.number_format_code = value;
    true
}

pub fn set_active_font_color(state: &mut OneCalcHostState, value: String) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if formula_space.formatting.font_color == value {
        return false;
    }
    formula_space.formatting.font_color = value;
    true
}

pub fn set_active_fill_color(state: &mut OneCalcHostState, value: String) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if formula_space.formatting.fill_color == value {
        return false;
    }
    formula_space.formatting.fill_color = value;
    true
}

pub fn set_active_date1904(state: &mut OneCalcHostState, value: bool) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if formula_space.formatting.date1904 == value {
        return false;
    }
    formula_space.formatting.date1904 = value;
    true
}

/// Set the active formula's scenario policy (Deterministic /
/// LiveRecalc). Drives whether the bridge passes fixed seeds for
/// `now_serial` plus a deterministic random provider stream, or
/// fresh clock/provider inputs per round-trip. Returns true when the
/// value actually changed.
pub fn set_active_scenario_policy(
    state: &mut OneCalcHostState,
    policy: crate::persistence::ScenarioPolicy,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if formula_space.formatting.scenario_policy == policy {
        return false;
    }
    formula_space.formatting.scenario_policy = policy;
    true
}

/// Switch the workspace's ambient locale preset to the given
/// BCP-47 language tag. Replaces the date / datetime / time format
/// codes and the `language_tag` field on `AmbientAppContext`. Each
/// subsequent bridge round-trip carries this tag through to
/// `live_bridge::build_runtime_locale_context`, which resolves it
/// to a `LocaleProfileId` and builds the matching runtime
/// `LocaleFormatContext` — month / weekday tables, separators,
/// currency symbol, and `General` rendering all flip with the
/// preset. Returns true when the value actually changed.
pub fn set_workspace_locale_preset(state: &mut OneCalcHostState, language_tag: String) -> bool {
    let next = crate::services::ambient_app_context::ambient_app_context_for_language_tag(Some(
        &language_tag,
    ));
    if state.ambient_app_context == next {
        return false;
    }
    state.ambient_app_context = next;
    true
}

/// Append a conditional-formatting rule to the active formula.
/// Returns the new rule's index in the list, or `None` when there
/// is no active formula space.
pub fn add_active_conditional_formatting_rule(
    state: &mut OneCalcHostState,
    rule: crate::state::FormulaConditionalFormattingRule,
) -> Option<usize> {
    let formula_space = active_formula_space_mut(state)?;
    formula_space
        .formatting
        .conditional_formatting_rules
        .push(rule);
    Some(formula_space.formatting.conditional_formatting_rules.len() - 1)
}

/// Remove a conditional-formatting rule by index. Returns true when
/// the rule existed and was removed.
pub fn remove_active_conditional_formatting_rule(
    state: &mut OneCalcHostState,
    index: usize,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if index >= formula_space.formatting.conditional_formatting_rules.len() {
        return false;
    }
    formula_space
        .formatting
        .conditional_formatting_rules
        .remove(index);
    true
}

/// Update an existing CF rule by index. Returns true when the rule
/// existed and the new value is different from the current value.
pub fn update_active_conditional_formatting_rule(
    state: &mut OneCalcHostState,
    index: usize,
    rule: crate::state::FormulaConditionalFormattingRule,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    let rules = &mut formula_space.formatting.conditional_formatting_rules;
    if index >= rules.len() {
        return false;
    }
    if rules[index] == rule {
        return false;
    }
    rules[index] = rule;
    true
}

pub fn apply_editor_overlay_measurement_to_active_formula_space(
    state: &mut OneCalcHostState,
    measurement_event: EditorOverlayMeasurementEvent,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };

    formula_space.editor_overlay_geometry = Some(measurement_event.snapshot);
    true
}

/// Set the popup's `selected_index` to the row matching `proposal_id`,
/// then accept it. Returns the acceptance (or `None` when the popup is
/// `Hidden` or the id is unknown). Used by the popup's click-to-accept
/// path so the home shell doesn't need to dispatch separate
/// "select-by-id then accept" reducer calls per click.
pub fn accept_completion_by_proposal_id_on_active_formula_space(
    state: &mut OneCalcHostState,
    proposal_id: &str,
) -> Option<crate::services::completion_popup::CompletionAcceptance> {
    let raw_text = active_formula_space_mut(state)?
        .raw_entered_cell_text
        .clone();
    let formula_space = active_formula_space_mut(state)?;
    if let crate::services::completion_popup::CompletionPopupState::Open {
        items,
        selected_index,
        ..
    } = &mut formula_space.completion_popup
    {
        if let Some(index) = items
            .iter()
            .position(|item| item.proposal_id == proposal_id)
        {
            *selected_index = index;
        } else {
            return None;
        }
    } else {
        return None;
    }
    let acceptance = popup_accept_selected(&mut formula_space.completion_popup, &raw_text)?;
    formula_space.completion_popup_suppressed_until_next_input = true;
    Some(acceptance)
}

/// Accept the popup's currently-selected completion (the keyboard
/// path: Tab / Enter from a popup-Open state). Mirrors the
/// proposal-id variant for the click path; both set the
/// `completion_popup_suppressed_until_next_input` flag so the next
/// bridge refresh does not re-open the popup over the just-accepted
/// proposal.
pub fn accept_selected_completion_with_suppression_on_active_formula_space(
    state: &mut OneCalcHostState,
) -> Option<crate::services::completion_popup::CompletionAcceptance> {
    let raw_text = active_formula_space_mut(state)?
        .raw_entered_cell_text
        .clone();
    let formula_space = active_formula_space_mut(state)?;
    let acceptance = popup_accept_selected(&mut formula_space.completion_popup, &raw_text)?;
    formula_space.completion_popup_suppressed_until_next_input = true;
    Some(acceptance)
}

/// Move the completion popup's selected index by `delta` (typically
/// `+1` for ArrowDown / `-1` for ArrowUp). Wraps at both ends. No-op
/// when the popup is `Hidden`. Returns `true` when state changed.
pub fn move_completion_popup_selection_on_active_formula_space(
    state: &mut OneCalcHostState,
    delta: i32,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    popup_move_selection(&mut formula_space.completion_popup, delta)
}

/// Force the completion popup to `Hidden`. No-op when already
/// `Hidden`. Returns `true` when state changed.
pub fn dismiss_completion_popup_on_active_formula_space(state: &mut OneCalcHostState) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    popup_dismiss(&mut formula_space.completion_popup)
}

/// Toggle the workspace's view-mode preference. Returns the new
/// value (User or Developer). Pure flip; if a caller needs an
/// explicit set, use [`set_view_mode_on_workspace`].
pub fn toggle_view_mode_on_workspace(state: &mut OneCalcHostState) -> crate::state::ViewMode {
    state.view_mode = state.view_mode.toggle();
    state.view_mode
}

/// Set the workspace's view-mode preference explicitly. Returns
/// `true` when the value changed. Used by the workspace settings
/// page (later bead) and by tests; the keyboard chord uses the
/// toggle entry above.
pub fn set_view_mode_on_workspace(
    state: &mut OneCalcHostState,
    mode: crate::state::ViewMode,
) -> bool {
    if state.view_mode == mode {
        return false;
    }
    state.view_mode = mode;
    true
}

/// Toggle the formula-drill-down panel on the active formula space.
/// Returns the new `formula_drill_open` value, or `false` when there
/// is no active formula space (caller can treat that as a no-op).
pub fn toggle_formula_drill_on_active_formula_space(state: &mut OneCalcHostState) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    formula_space.formula_drill_open = !formula_space.formula_drill_open;
    formula_space.formula_drill_open
}

/// Force-close the formula-drill-down panel. Used by tests and by
/// any future "exit drill-down on Esc" wiring. Returns `true` when
/// state changed.
pub fn close_formula_drill_on_active_formula_space(state: &mut OneCalcHostState) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if !formula_space.formula_drill_open {
        return false;
    }
    formula_space.formula_drill_open = false;
    true
}

/// Toggle the formatting-controls panel on the active formula space.
/// The panel sits between the formula drill-down and the result
/// section; when collapsed it renders as a single chip with the
/// number-format summary, when expanded it renders the full
/// formatting controls row. Returns the new `formatting_panel_open`
/// value, or `false` when there is no active formula space.
pub fn toggle_formatting_panel_on_active_formula_space(state: &mut OneCalcHostState) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    formula_space.formatting_panel_open = !formula_space.formatting_panel_open;
    formula_space.formatting_panel_open
}

/// Accept the popup's currently-selected completion. Returns the
/// `CompletionAcceptance` describing the splice the editor layer
/// should apply, plus transitions the popup to `Hidden`. Returns
/// `None` when the popup is `Hidden` (no acceptance to apply) or
/// when there is no active formula space.
pub fn accept_selected_completion_on_active_formula_space(
    state: &mut OneCalcHostState,
) -> Option<CompletionAcceptance> {
    let raw_text = active_formula_space_mut(state)?
        .raw_entered_cell_text
        .clone();
    let formula_space = active_formula_space_mut(state)?;
    popup_accept_selected(&mut formula_space.completion_popup, &raw_text)
}

/// Set or update the browser-measured textarea metrics on the active
/// formula space. Returns `true` when the metrics actually changed so
/// the home shell can short-circuit no-op re-renders. Increments the
/// monotonic `editor_box_metrics_tick` counter on every change so
/// browser tests can detect re-measurement even when the values are
/// numerically identical.
pub fn apply_editor_box_metrics_to_active_formula_space(
    state: &mut OneCalcHostState,
    metrics: TextareaMeasurementMetrics,
) -> bool {
    let Some(formula_space) = active_formula_space_mut(state) else {
        return false;
    };
    if formula_space.editor_box_metrics == Some(metrics) {
        return false;
    }
    formula_space.editor_box_metrics = Some(metrics);
    formula_space.editor_box_metrics_tick = formula_space.editor_box_metrics_tick.wrapping_add(1);
    true
}

pub fn open_retained_artifact_from_catalog(
    state: &mut OneCalcHostState,
    artifact_id: &str,
) -> bool {
    open_retained_artifact_by_id(state, artifact_id).is_ok()
}

pub fn open_retained_artifact_from_catalog_in_inspect(
    state: &mut OneCalcHostState,
    artifact_id: &str,
) -> bool {
    open_retained_artifact_in_inspect_by_id(state, artifact_id).is_ok()
}

pub fn import_manual_retained_artifact_into_active_formula_space(
    state: &mut OneCalcHostState,
    request: ManualRetainedArtifactImportRequest,
) -> bool {
    import_manual_artifact_for_active_formula_space(state, request).is_ok()
}

pub fn import_verification_bundle_report_into_workspace(
    state: &mut OneCalcHostState,
    request: VerificationBundleImportRequest,
) -> bool {
    import_verification_bundle_report_json(state, request).is_ok()
}

fn active_formula_space_mut(state: &mut OneCalcHostState) -> Option<&mut FormulaSpaceState> {
    let formula_space_id = state
        .workspace_shell
        .active_formula_space_id
        .clone()
        .or(state
            .active_formula_space_view
            .selected_formula_space_id
            .clone())?;
    state.formula_spaces.get_mut(&formula_space_id)
}

fn apply_local_editor_text_change(
    formula_space: &mut FormulaSpaceState,
    text: String,
    editor_surface_state: EditorSurfaceState,
) {
    formula_space.raw_entered_cell_text = text;
    formula_space.editor_surface_state = editor_surface_state;
    formula_space.editor_overlay_geometry = None;
    formula_space.editor_document = None;
    formula_space.completion_help = CompletionHelpState::default();
    formula_space.latest_evaluation_summary = None;
    formula_space.effective_display_summary = None;
}

fn apply_completion_command(
    formula_space: &mut FormulaSpaceState,
    command: &EditorCommand,
) -> bool {
    match command {
        EditorCommand::SelectPreviousCompletion => {
            let proposal_count = formula_space
                .editor_document
                .as_ref()
                .map(|document| document.completion_proposals.len())
                .unwrap_or(0);
            if proposal_count == 0 {
                return false;
            }
            formula_space.editor_surface_state.completion_selected_index =
                cycle_completion_selection(
                    formula_space.editor_surface_state.completion_selected_index,
                    proposal_count,
                    -1,
                );
            true
        }
        EditorCommand::SelectNextCompletion => {
            let proposal_count = formula_space
                .editor_document
                .as_ref()
                .map(|document| document.completion_proposals.len())
                .unwrap_or(0);
            if proposal_count == 0 {
                return false;
            }
            formula_space.editor_surface_state.completion_selected_index =
                cycle_completion_selection(
                    formula_space.editor_surface_state.completion_selected_index,
                    proposal_count,
                    1,
                );
            true
        }
        EditorCommand::SelectCompletionByIndex(index) => {
            let proposal_count = formula_space
                .editor_document
                .as_ref()
                .map(|document| document.completion_proposals.len())
                .unwrap_or(0);
            if proposal_count == 0 {
                return false;
            }
            formula_space.editor_surface_state.completion_selected_index =
                Some((*index).min(proposal_count.saturating_sub(1)));
            true
        }
        EditorCommand::AcceptSelectedCompletion => {
            let Some(document) = formula_space.editor_document.as_ref() else {
                return false;
            };
            let proposal_count = document.completion_proposals.len();
            if proposal_count == 0 {
                return false;
            }
            let selected_index = formula_space
                .editor_surface_state
                .completion_selected_index
                .unwrap_or(0)
                .min(proposal_count.saturating_sub(1));
            let proposal = &document.completion_proposals[selected_index];
            let (selection_start, selection_end) = proposal
                .replacement_span
                .map(|span| (span.start, span.start + span.len))
                .unwrap_or((
                    formula_space.editor_surface_state.selection.start(),
                    formula_space.editor_surface_state.selection.end(),
                ));
            let replacement_state = EditorSurfaceState {
                selection: crate::ui::editor::state::EditorSelection {
                    anchor: selection_start,
                    focus: selection_end,
                },
                caret: crate::ui::editor::state::EditorCaret {
                    offset: selection_end,
                },
                scroll_window: formula_space.editor_surface_state.scroll_window.clone(),
                completion_anchor_offset: formula_space
                    .editor_surface_state
                    .completion_anchor_offset,
                completion_selected_index: formula_space
                    .editor_surface_state
                    .completion_selected_index,
                signature_help_anchor_offset: formula_space
                    .editor_surface_state
                    .signature_help_anchor_offset,
            };
            let result = apply_editor_command(
                &formula_space.raw_entered_cell_text,
                &replacement_state,
                EditorCommand::InsertText(proposal.insert_text.clone()),
            );
            apply_local_editor_text_change(formula_space, result.text, result.state);
            true
        }
        EditorCommand::AcceptCompletionByIndex(index) => {
            let proposal_count = formula_space
                .editor_document
                .as_ref()
                .map(|document| document.completion_proposals.len())
                .unwrap_or(0);
            if proposal_count == 0 {
                return false;
            }
            formula_space.editor_surface_state.completion_selected_index =
                Some((*index).min(proposal_count.saturating_sub(1)));
            apply_completion_command(formula_space, &EditorCommand::AcceptSelectedCompletion)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::FormulaSpaceState;
    use crate::ui::editor::commands::EditorInputKind;
    use crate::ui::editor::geometry::{
        EditorMeasuredOverlayBox, EditorOverlayGeometrySnapshot, EditorOverlayMeasurementEvent,
    };
    use crate::ui::editor::state::EditorSelection;
    use crate::{
        domain::ids::FormulaSpaceId,
        services::{
            programmatic_testing::{
                ProgrammaticArtifactCatalogEntry, ProgrammaticComparisonStatus,
                ProgrammaticOpenModeHint,
            },
            retained_artifacts::RetainedArtifactImportRequest,
        },
    };

    fn fresh_state_with_active_space(id: &str) -> OneCalcHostState {
        let formula_space_id = FormulaSpaceId::new(id);
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .workspace_shell
            .open_formula_space_order
            .push(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, ""));
        state
    }

    #[test]
    fn manage_formulas_open_close_round_trip() {
        let mut state = fresh_state_with_active_space("space-1");
        assert!(open_manage_formulas(&mut state));
        assert!(state.global_ui_chrome.manage_formulas_open);
        // Idempotent on a second open.
        assert!(!open_manage_formulas(&mut state));
        // Search query persists while the overlay is open.
        assert!(set_manage_formulas_search_query(&mut state, "xlookup"));
        assert_eq!(
            state.global_ui_chrome.manage_formulas_search_query,
            "xlookup"
        );
        // Closing clears the search query so the next open is fresh.
        assert!(close_manage_formulas(&mut state));
        assert!(!state.global_ui_chrome.manage_formulas_open);
        assert!(state
            .global_ui_chrome
            .manage_formulas_search_query
            .is_empty());
        // Idempotent on second close.
        assert!(!close_manage_formulas(&mut state));
    }

    #[test]
    fn manage_formulas_search_query_is_idempotent() {
        let mut state = fresh_state_with_active_space("space-1");
        assert!(set_manage_formulas_search_query(&mut state, "abc"));
        assert!(!set_manage_formulas_search_query(&mut state, "abc"));
        assert!(set_manage_formulas_search_query(&mut state, "abcd"));
    }

    #[test]
    fn vba_host_browser_module_source_records_deferred_integration() {
        let mut state = OneCalcHostState::default();

        let runtime = load_vba_module_source_for_host_context(
            &mut state,
            VbaModuleSourceLoadRequest::browser_file(
                "SimpleVba.bas".to_string(),
                concat!(
                    "Public Function AddThem(a As Double, b As Double) As Double\n",
                    "AddThem = a + b\n",
                    "End Function\n"
                )
                .to_string(),
            ),
        );

        assert!(runtime.is_none());
        let association = state
            .vba_host_context
            .associations
            .last()
            .expect("association should be recorded");
        assert_eq!(association.display_name, "SimpleVba.bas");
        assert_eq!(association.source_ref, "browser-file:SimpleVba.bas");
        match &association.load_status {
            VbaHostAssociationLoadStatus::Failed(reason) => {
                assert!(reason.contains("OxVba integration is deferred"));
            }
            other => panic!("expected deferred OxVba failure, got {other:?}"),
        }
        assert_eq!(association.admitted_udf_count, 0);
        assert_eq!(association.rejected_candidate_count, 0);
        assert!(association.admitted_udfs.is_empty());
        assert!(association.rejected_candidates.is_empty());
    }

    #[test]
    fn vba_host_native_module_source_records_real_path_and_deferred_integration() {
        let mut state = OneCalcHostState::default();

        let runtime = load_vba_module_source_for_host_context(
            &mut state,
            VbaModuleSourceLoadRequest::native_file(
                "SimpleVba.bas".to_string(),
                "C:\\Temp\\SimpleVba.bas".to_string(),
                concat!(
                    "Public Function AddThem(a As Double, b As Double) As Double\n",
                    "AddThem = a + b\n",
                    "End Function\n"
                )
                .to_string(),
                Vec::new(),
            ),
        );

        assert!(runtime.is_none());
        let association = state
            .vba_host_context
            .associations
            .last()
            .expect("association should be recorded");
        assert_eq!(association.display_name, "SimpleVba.bas");
        assert_eq!(association.source_ref, "C:\\Temp\\SimpleVba.bas");
        match &association.load_status {
            VbaHostAssociationLoadStatus::Failed(reason) => {
                assert!(reason.contains("OxVba integration is deferred"));
            }
            other => panic!("expected deferred OxVba failure, got {other:?}"),
        }
        assert_eq!(association.admitted_udf_count, 0);
        assert_eq!(association.rejected_candidate_count, 0);
    }

    #[test]
    fn vba_host_native_module_source_records_command_failure() {
        let mut state = OneCalcHostState::default();

        let runtime = load_vba_module_source_for_host_context(
            &mut state,
            VbaModuleSourceLoadRequest::native_file(
                "Missing.bas".to_string(),
                "C:\\Temp\\Missing.bas".to_string(),
                String::new(),
                vec!["failed to read VBA module source".to_string()],
            ),
        );

        assert!(runtime.is_none());
        let association = state
            .vba_host_context
            .associations
            .last()
            .expect("failed association should be recorded");
        assert_eq!(association.source_ref, "C:\\Temp\\Missing.bas");
        assert_eq!(
            association.load_status,
            VbaHostAssociationLoadStatus::Failed("failed to read VBA module source".to_string())
        );
    }

    #[test]
    fn array_browser_zoom_reducers_clamp_to_bounds() {
        let mut state = fresh_state_with_active_space("space-zoom");
        // Default zoom is 1.0.
        let zoom = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .map(|space| space.array_browser.zoom)
            .unwrap_or(0.0);
        assert!((zoom - 1.0).abs() < f32::EPSILON);

        // Step up several times — clamped at 3.0.
        for _ in 0..50 {
            let _ = zoom_in_active_array_browser(&mut state);
        }
        let zoom = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .map(|space| space.array_browser.zoom)
            .unwrap_or(0.0);
        assert!(zoom <= 3.0 + f32::EPSILON);
        assert!(zoom >= 3.0 - f32::EPSILON);

        // Reset returns to 1.0.
        let _ = reset_active_array_browser_zoom(&mut state);
        let zoom = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .map(|space| space.array_browser.zoom)
            .unwrap_or(0.0);
        assert!((zoom - 1.0).abs() < f32::EPSILON);

        // Step down several times — clamped at 0.5.
        for _ in 0..50 {
            let _ = zoom_out_active_array_browser(&mut state);
        }
        let zoom = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .map(|space| space.array_browser.zoom)
            .unwrap_or(0.0);
        assert!(zoom <= 0.5 + f32::EPSILON);
        assert!(zoom >= 0.5 - f32::EPSILON);
    }

    #[test]
    fn array_browser_selection_reducer_round_trips() {
        use crate::state::ArrayBlockSelection;
        let mut state = fresh_state_with_active_space("space-sel");
        let sel = ArrayBlockSelection::from_corners(0, 0, 2, 3);
        assert!(set_active_array_browser_selection(&mut state, Some(sel)));
        let stored = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .and_then(|space| space.array_browser.selection)
            .expect("stored selection");
        assert_eq!(stored.normalized(), (0, 2, 0, 3));
        // Idempotent: setting the same selection returns false.
        assert!(!set_active_array_browser_selection(&mut state, Some(sel)));
        // Clearing returns true.
        assert!(set_active_array_browser_selection(&mut state, None));
    }

    #[test]
    fn compute_column_header_selection_replaces_when_not_extending() {
        // Plain click on column 2 with a 5-row preview should claim
        // exactly column 2, all 5 rows, with the anchor at (0, 2).
        let next = compute_column_header_selection(None, 2, 5, false).expect("selection");
        assert_eq!(next.normalized(), (0, 4, 2, 2));
        assert_eq!(next.anchor_row, 0);
        assert_eq!(next.anchor_col, 2);
        // A prior selection has no influence on a non-extending click.
        let prior = crate::state::ArrayBlockSelection::from_corners(1, 0, 3, 1);
        let next = compute_column_header_selection(Some(prior), 2, 5, false).expect("selection");
        assert_eq!(next.normalized(), (0, 4, 2, 2));
    }

    #[test]
    fn compute_column_header_selection_extends_from_prior_anchor() {
        let prior = crate::state::ArrayBlockSelection::from_corners(0, 1, 2, 1);
        let next = compute_column_header_selection(Some(prior), 4, 5, true).expect("selection");
        // Anchor stays at (0, 1) from the prior; focus moves to (4, 4).
        assert_eq!(next.anchor_row, 0);
        assert_eq!(next.anchor_col, 1);
        assert_eq!(next.focus_row, 4);
        assert_eq!(next.focus_col, 4);
    }

    #[test]
    fn compute_column_header_selection_extend_without_prior_acts_as_replace() {
        let next = compute_column_header_selection(None, 3, 5, true).expect("selection");
        assert_eq!(next.normalized(), (0, 4, 3, 3));
    }

    #[test]
    fn compute_row_header_selection_replaces_when_not_extending() {
        let next = compute_row_header_selection(None, 2, 4, false).expect("selection");
        assert_eq!(next.normalized(), (2, 2, 0, 3));
        assert_eq!(next.anchor_row, 2);
        assert_eq!(next.anchor_col, 0);
    }

    #[test]
    fn compute_row_header_selection_extends_from_prior_anchor() {
        let prior = crate::state::ArrayBlockSelection::from_corners(1, 0, 1, 2);
        let next = compute_row_header_selection(Some(prior), 4, 4, true).expect("selection");
        assert_eq!(next.anchor_row, 1);
        assert_eq!(next.anchor_col, 0);
        assert_eq!(next.focus_row, 4);
        assert_eq!(next.focus_col, 3);
    }

    #[test]
    fn compute_select_all_visible_selection_covers_whole_preview() {
        let next = compute_select_all_visible_selection(3, 4).expect("selection");
        assert_eq!(next.normalized(), (0, 2, 0, 3));
    }

    #[test]
    fn compute_helpers_return_none_for_empty_preview() {
        assert!(compute_column_header_selection(None, 0, 0, false).is_none());
        assert!(compute_row_header_selection(None, 0, 0, false).is_none());
        assert!(compute_select_all_visible_selection(0, 5).is_none());
        assert!(compute_select_all_visible_selection(5, 0).is_none());
    }

    #[test]
    fn array_browser_column_width_clamps_to_min() {
        let mut state = fresh_state_with_active_space("space-col");
        // 0.1rem is below the minimum 1.5rem clamp.
        let _ = set_active_array_browser_column_width(&mut state, 2, 0.1);
        let stored = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|id| state.formula_spaces.get(id))
            .map(|space| space.array_browser.column_widths_rem.get(&2).copied())
            .flatten()
            .expect("column width stored");
        assert!(stored >= 1.5 - f32::EPSILON);
    }

    #[test]
    fn array_browser_display_toggles_flip_settings() {
        let mut state = fresh_state_with_active_space("space-display");
        let initial = state.global_ui_chrome.array_browser_display;
        let _ = toggle_array_browser_display_option(
            &mut state,
            ArrayBrowserDisplayToggle::AlternatingRows,
        );
        assert_ne!(
            initial.show_alternating_rows,
            state
                .global_ui_chrome
                .array_browser_display
                .show_alternating_rows
        );
        let _ =
            toggle_array_browser_display_option(&mut state, ArrayBrowserDisplayToggle::GridLines);
        assert_ne!(
            initial.show_grid_lines,
            state.global_ui_chrome.array_browser_display.show_grid_lines
        );
    }

    #[test]
    fn input_event_updates_raw_text_and_editor_state_for_active_formula_space() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

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
            .get(&FormulaSpaceId::new("space-1"))
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=SUM(1,2,3)");
        assert_eq!(active.editor_surface_state.caret.offset, 11);
    }

    #[test]
    fn input_event_preserves_selection_metadata_when_provided() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

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
            .get(&FormulaSpaceId::new("space-1"))
            .expect("space exists");
        assert_eq!(active.editor_surface_state.selection.anchor, 2);
        assert_eq!(active.editor_surface_state.selection.focus, 5);
        assert_eq!(active.editor_surface_state.caret.offset, 5);
    }

    #[test]
    fn command_updates_editor_state_and_clears_stale_analysis() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id, "SUM(\n1,\n2)");
        formula_space.editor_surface_state.selection = EditorSelection {
            anchor: 0,
            focus: formula_space.raw_entered_cell_text.chars().count(),
        };
        formula_space.latest_evaluation_summary = Some("Number".to_string());
        formula_space.effective_display_summary = Some("3".to_string());
        state.formula_spaces.insert(formula_space);

        let changed = apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::IndentWithSpaces,
        );

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&FormulaSpaceId::new("space-1"))
            .expect("space exists");
        assert!(active.raw_entered_cell_text.starts_with("    SUM("));
        assert!(active.latest_evaluation_summary.is_none());
        assert!(active.effective_display_summary.is_none());
    }

    /// §11.2 invariant 2: `apply_editor_command_to_active_formula_space`
    /// returns `false` when no active formula space exists (and is not a
    /// UI chrome command). Pins the guard in reducer.rs.
    #[test]
    fn apply_editor_command_returns_false_when_no_active_formula_space() {
        let mut state = OneCalcHostState::default();
        assert!(state.workspace_shell.active_formula_space_id.is_none());

        let changed = apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::IndentWithSpaces,
        );
        assert!(!changed);

        let changed =
            apply_editor_command_to_active_formula_space(&mut state, EditorCommand::CommitEntry);
        assert!(!changed);
    }

    /// §11.2 invariant 3: UI-chrome commands succeed even when no active
    /// formula space exists. `ToggleConfigureDrawer`,
    /// `ToggleEditorSettingsPopover`, and `UpdateEditorSetting(_)` mutate
    /// workspace-level state and must return `true` against an empty
    /// workspace.
    #[test]
    fn ui_chrome_commands_succeed_with_no_active_formula_space() {
        use crate::ui::editor::state::EditorSettingUpdate;
        let mut state = OneCalcHostState::default();

        assert!(apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::ToggleConfigureDrawer,
        ));
        assert!(state.global_ui_chrome.configure_drawer_open);

        assert!(apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::ToggleEditorSettingsPopover,
        ));
        assert!(state.global_ui_chrome.editor_settings_popover_open);

        assert!(apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::UpdateEditorSetting(EditorSettingUpdate::ToggleHighlightBracketPairs),
        ));
        // The setting must have changed even though there's no formula space.
        assert!(
            !state
                .global_ui_chrome
                .editor_settings
                .highlight_bracket_pairs
        );
    }

    #[test]
    fn commit_entry_records_committed_text_and_transitions_live_state() {
        use crate::ui::editor::state::EditorLiveState;
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        assert!(apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::CommitEntry,
        ));
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.committed_cell_text.as_deref(), Some("=SUM(1,2)"));
        assert_eq!(active.live_state(), EditorLiveState::Committed);

        apply_editor_input_to_active_formula_space(
            &mut state,
            EditorInputEvent {
                text: "=SUM(1,2,3)".to_string(),
                selection_start: None,
                selection_end: None,
                input_kind: EditorInputKind::Other,
                inserted_text: None,
            },
        );
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.live_state(), EditorLiveState::EditingLive);

        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::RequestProof);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.live_state(), EditorLiveState::ProofedScratch);

        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::CancelEntry);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=SUM(1,2)");
        assert_eq!(active.live_state(), EditorLiveState::Committed);
    }

    #[test]
    fn editor_settings_popover_toggle_and_update_apply_to_global_chrome() {
        use crate::ui::editor::state::{CompletionAggressiveness, EditorSettingUpdate};
        let mut state = OneCalcHostState::default();
        apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::ToggleEditorSettingsPopover,
        );
        assert!(state.global_ui_chrome.editor_settings_popover_open);

        apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::UpdateEditorSetting(EditorSettingUpdate::ToggleHighlightBracketPairs),
        );
        assert!(
            !state
                .global_ui_chrome
                .editor_settings
                .highlight_bracket_pairs
        );

        apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::UpdateEditorSetting(EditorSettingUpdate::SetCompletionAggressiveness(
                CompletionAggressiveness::Always,
            )),
        );
        assert_eq!(
            state
                .global_ui_chrome
                .editor_settings
                .completion_aggressiveness,
            CompletionAggressiveness::Always
        );
    }

    /// §11.5 invariant 9: F4 with a single-cell reference under the
    /// caret rotates one step in the cycle and the selection covers the
    /// rewritten reference. Walks all four steps of the cycle through the
    /// reducer entry point so the integration between
    /// `reference_cycle::cycle_reference_form` and the reducer's caret /
    /// selection update is pinned end-to-end.
    #[test]
    fn cycle_reference_form_rewrites_cell_and_selects_new_span() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=A1+B2");
        formula_space.editor_surface_state.caret =
            crate::ui::editor::state::EditorCaret { offset: 1 };
        formula_space.editor_surface_state.selection = EditorSelection::collapsed(1);
        state.formula_spaces.insert(formula_space);

        // Step 1: A1 → $A$1
        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::CycleReferenceForm);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=$A$1+B2");
        assert_eq!(active.editor_surface_state.selection.anchor, 1);
        assert_eq!(active.editor_surface_state.selection.focus, 5);

        // Step 2: $A$1 → A$1
        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::CycleReferenceForm);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=A$1+B2");
        assert_eq!(active.editor_surface_state.selection.anchor, 1);
        assert_eq!(active.editor_surface_state.selection.focus, 4);

        // Step 3: A$1 → $A1
        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::CycleReferenceForm);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=$A1+B2");
        assert_eq!(active.editor_surface_state.selection.anchor, 1);
        assert_eq!(active.editor_surface_state.selection.focus, 4);

        // Step 4: $A1 → A1 (back to the starting form)
        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::CycleReferenceForm);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert_eq!(active.raw_entered_cell_text, "=A1+B2");
        assert_eq!(active.editor_surface_state.selection.anchor, 1);
        assert_eq!(active.editor_surface_state.selection.focus, 3);
    }

    /// §11.5 invariant 8: accepting a completion proposal (via
    /// `AcceptCompletionByIndex(0)`) replaces the proposal's
    /// `replacement_span` with the `insert_text` and lands the caret at
    /// the end of the inserted text.
    #[test]
    fn accept_completion_from_tab_replaces_anchor_span_and_lands_caret_at_end() {
        use crate::adapters::oxfml::{
            CompletionProposal, CompletionProposalKind, EditorDocument, FormulaEditReuseSummary,
            FormulaTextSpan,
        };

        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SU");
        formula_space.editor_surface_state.caret =
            crate::ui::editor::state::EditorCaret { offset: 3 };
        formula_space.editor_surface_state.selection = EditorSelection::collapsed(3);
        formula_space.editor_document = Some(EditorDocument {
            source_text: "=SU".to_string(),
            text_change_range: None,
            editor_syntax_snapshot: crate::test_support::make_editor_syntax_snapshot(
                "formula-1",
                "green-1",
                vec![],
            ),
            live_diagnostics: crate::test_support::empty_live_diagnostic_snapshot(),
            reuse_summary: FormulaEditReuseSummary {
                reused_green_tree: false,
                reused_red_projection: false,
                reused_bound_formula: false,
            },
            signature_help: None,
            function_help: None,
            completion_proposals: vec![CompletionProposal {
                proposal_id: "completion-SUM".to_string(),
                proposal_kind: CompletionProposalKind::Function,
                display_text: "SUM".to_string(),
                insert_text: "SUM(".to_string(),
                replacement_span: Some(FormulaTextSpan { start: 1, len: 2 }),
                documentation_ref: None,
                profile_payload: None,
                requires_revalidation: true,
            }],
            formula_walk: vec![],
            parse_summary: None,
            bind_summary: None,
            eval_summary: None,
            provenance_summary: None,
            value_presentation: None,
        });
        state.formula_spaces.insert(formula_space);

        let changed = apply_editor_command_to_active_formula_space(
            &mut state,
            EditorCommand::AcceptCompletionByIndex(0),
        );

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        // Replacement span [1, 3) covers "SU"; after insert_text = "SUM("
        // the text becomes "=SUM(" and the caret lands at offset 5.
        assert_eq!(active.raw_entered_cell_text, "=SUM(");
        assert_eq!(active.editor_surface_state.caret.offset, 5);
    }

    #[test]
    fn dismiss_completion_clears_anchor_and_selected_index() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "=SU");
        formula_space.editor_surface_state.completion_anchor_offset = Some(1);
        formula_space.editor_surface_state.completion_selected_index = Some(2);
        state.formula_spaces.insert(formula_space);

        apply_editor_command_to_active_formula_space(&mut state, EditorCommand::DismissCompletion);

        let active = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("space exists");
        assert!(active
            .editor_surface_state
            .completion_anchor_offset
            .is_none());
        assert!(active
            .editor_surface_state
            .completion_selected_index
            .is_none());
    }

    // ----------------------------------------------------------------
    // Browser-measured caret-box metrics (bead dno-xcq.22)
    // ----------------------------------------------------------------

    #[test]
    fn editor_box_metrics_default_to_none_with_zero_tick() {
        let formula_space = FormulaSpaceState::new(FormulaSpaceId::new("u-1"), "=SUM(1,2)");
        assert!(formula_space.editor_box_metrics.is_none());
        assert_eq!(formula_space.editor_box_metrics_tick, 0);
    }

    #[test]
    fn applying_editor_box_metrics_sets_state_and_increments_tick() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        let metrics = TextareaMeasurementMetrics {
            char_width_px: 9,
            line_height_px: 22,
            scroll_top_px: 0,
            scroll_left_px: 0,
        };
        let changed = apply_editor_box_metrics_to_active_formula_space(&mut state, metrics);
        assert!(changed, "first metric application changes state");

        let active = state.formula_spaces.get(&formula_space_id).expect("space");
        assert_eq!(active.editor_box_metrics, Some(metrics));
        assert_eq!(active.editor_box_metrics_tick, 1);
    }

    #[test]
    fn applying_identical_editor_box_metrics_is_a_noop_and_does_not_increment_tick() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        let metrics = TextareaMeasurementMetrics {
            char_width_px: 9,
            line_height_px: 22,
            scroll_top_px: 0,
            scroll_left_px: 0,
        };
        let _ = apply_editor_box_metrics_to_active_formula_space(&mut state, metrics);
        let changed_again = apply_editor_box_metrics_to_active_formula_space(&mut state, metrics);
        assert!(!changed_again, "identical metric is a no-op");

        let active = state.formula_spaces.get(&formula_space_id).expect("space");
        assert_eq!(
            active.editor_box_metrics_tick, 1,
            "tick stays at 1 across identical re-applications",
        );
    }

    #[test]
    fn changing_editor_box_metrics_increments_tick_and_updates_state() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        let initial = TextareaMeasurementMetrics {
            char_width_px: 9,
            line_height_px: 22,
            scroll_top_px: 0,
            scroll_left_px: 0,
        };
        let resized = TextareaMeasurementMetrics {
            char_width_px: 8,
            line_height_px: 20,
            scroll_top_px: 22,
            scroll_left_px: 0,
        };
        let _ = apply_editor_box_metrics_to_active_formula_space(&mut state, initial);
        let changed = apply_editor_box_metrics_to_active_formula_space(&mut state, resized);
        assert!(changed);

        let active = state.formula_spaces.get(&formula_space_id).expect("space");
        assert_eq!(active.editor_box_metrics, Some(resized));
        assert_eq!(active.editor_box_metrics_tick, 2);
    }

    #[test]
    fn applying_editor_box_metrics_without_an_active_space_returns_false() {
        let mut state = OneCalcHostState::default();
        let changed = apply_editor_box_metrics_to_active_formula_space(
            &mut state,
            TextareaMeasurementMetrics {
                char_width_px: 9,
                line_height_px: 22,
                scroll_top_px: 0,
                scroll_left_px: 0,
            },
        );
        assert!(!changed, "no active space -> no change reported");
    }

    #[test]
    fn overlay_measurement_event_updates_geometry_on_active_formula_space() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let changed = apply_editor_overlay_measurement_to_active_formula_space(
            &mut state,
            EditorOverlayMeasurementEvent {
                snapshot: EditorOverlayGeometrySnapshot {
                    caret_box: Some(EditorMeasuredOverlayBox {
                        top_px: 40,
                        left_px: 64,
                        width_px: 2,
                        height_px: 22,
                        line_index: 0,
                        column_index: 4,
                    }),
                    selection_box: None,
                    completion_anchor_box: None,
                    signature_help_anchor_box: None,
                    completion_popup_box: None,
                    signature_help_popup_box: None,
                },
            },
        );

        assert!(changed);
        let active = state
            .formula_spaces
            .get(&FormulaSpaceId::new("space-1"))
            .expect("space exists");
        assert_eq!(
            active
                .editor_overlay_geometry
                .as_ref()
                .and_then(|geometry| geometry.caret_box.as_ref())
                .map(|box_geometry| box_geometry.left_px),
            Some(64)
        );
    }

    #[test]
    fn open_retained_artifact_routes_shell_to_workbench_context() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        crate::services::retained_artifacts::import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id: formula_space_id.clone(),
                catalog_entry: ProgrammaticArtifactCatalogEntry {
                    artifact_id: "artifact-1".to_string(),
                    case_id: "case-1".to_string(),
                    comparison_status: ProgrammaticComparisonStatus::Mismatched,
                    open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                },
                discrepancy_summary: Some("dna=3 excel=4".to_string()),
            },
        );

        let opened = open_retained_artifact_from_catalog(&mut state, "artifact-1");

        assert!(opened);
        assert_eq!(
            state.retained_artifacts.open_artifact_id.as_deref(),
            Some("artifact-1")
        );
        assert_eq!(
            state.workspace_shell.active_formula_space_id.as_ref(),
            Some(&formula_space_id)
        );
        assert_eq!(
            state.active_formula_space_view.active_mode,
            crate::state::AppMode::Workbench
        );
    }

    #[test]
    fn importing_manual_retained_artifact_routes_shell_to_workbench_context() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, "=SUM(1,2)"));

        let imported = import_manual_retained_artifact_into_active_formula_space(
            &mut state,
            crate::services::retained_artifacts::ManualRetainedArtifactImportRequest {
                artifact_id: "artifact-2".to_string(),
                case_id: "case-2".to_string(),
                comparison_status: ProgrammaticComparisonStatus::Blocked,
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        );

        assert!(imported);
        assert_eq!(
            state.retained_artifacts.open_artifact_id.as_deref(),
            Some("artifact-2")
        );
        assert_eq!(
            state.active_formula_space_view.active_mode,
            crate::state::AppMode::Workbench
        );
    }

    #[test]
    fn open_retained_artifact_in_inspect_routes_shell_to_inspect_context() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        crate::services::retained_artifacts::import_programmatic_artifact(
            &mut state,
            RetainedArtifactImportRequest {
                formula_space_id: formula_space_id.clone(),
                catalog_entry: ProgrammaticArtifactCatalogEntry {
                    artifact_id: "artifact-inspect".to_string(),
                    case_id: "case-inspect".to_string(),
                    comparison_status: ProgrammaticComparisonStatus::Blocked,
                    open_mode_hint: ProgrammaticOpenModeHint::Workbench,
                },
                discrepancy_summary: Some("excel lane unavailable".to_string()),
            },
        );

        let opened = open_retained_artifact_from_catalog_in_inspect(&mut state, "artifact-inspect");

        assert!(opened);
        assert_eq!(
            state.retained_artifacts.open_artifact_id.as_deref(),
            Some("artifact-inspect")
        );
        assert_eq!(
            state.workspace_shell.active_formula_space_id.as_ref(),
            Some(&formula_space_id)
        );
        assert_eq!(
            state.active_formula_space_view.active_mode,
            crate::state::AppMode::Inspect
        );
    }

    #[test]
    fn view_mode_default_is_user() {
        let state = OneCalcHostState::default();
        assert_eq!(state.view_mode, crate::state::ViewMode::User);
    }

    #[test]
    fn toggle_view_mode_flips_and_returns_new_value() {
        let mut state = OneCalcHostState::default();
        let after_first = toggle_view_mode_on_workspace(&mut state);
        assert_eq!(after_first, crate::state::ViewMode::Developer);
        assert_eq!(state.view_mode, crate::state::ViewMode::Developer);
        let after_second = toggle_view_mode_on_workspace(&mut state);
        assert_eq!(after_second, crate::state::ViewMode::User);
        assert_eq!(state.view_mode, crate::state::ViewMode::User);
    }

    #[test]
    fn set_view_mode_returns_true_when_changed_false_when_unchanged() {
        let mut state = OneCalcHostState::default();
        assert!(set_view_mode_on_workspace(
            &mut state,
            crate::state::ViewMode::Developer
        ));
        assert_eq!(state.view_mode, crate::state::ViewMode::Developer);
        assert!(!set_view_mode_on_workspace(
            &mut state,
            crate::state::ViewMode::Developer
        ));
        assert!(set_view_mode_on_workspace(
            &mut state,
            crate::state::ViewMode::User
        ));
    }

    #[test]
    fn toggle_formula_drill_flips_state_and_returns_new_value() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id.clone(), ""));

        // Default is closed.
        let formula_space = state
            .formula_spaces
            .get(&formula_space_id)
            .expect("formula space");
        assert!(!formula_space.formula_drill_open);

        // First toggle opens.
        let now_open = toggle_formula_drill_on_active_formula_space(&mut state);
        assert!(now_open);
        assert!(
            state
                .formula_spaces
                .get(&formula_space_id)
                .unwrap()
                .formula_drill_open
        );

        // Second toggle closes.
        let now_open = toggle_formula_drill_on_active_formula_space(&mut state);
        assert!(!now_open);
        assert!(
            !state
                .formula_spaces
                .get(&formula_space_id)
                .unwrap()
                .formula_drill_open
        );
    }

    #[test]
    fn toggle_formula_drill_no_op_when_no_active_formula_space() {
        let mut state = OneCalcHostState::default();
        // No active formula space — toggle returns false (treated as
        // a no-op by the caller).
        let result = toggle_formula_drill_on_active_formula_space(&mut state);
        assert!(!result);
    }

    #[test]
    fn close_formula_drill_returns_false_when_already_closed() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id, ""));

        let changed = close_formula_drill_on_active_formula_space(&mut state);
        assert!(!changed);
    }

    #[test]
    fn close_formula_drill_closes_open_panel() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        let mut formula_space = FormulaSpaceState::new(formula_space_id.clone(), "");
        formula_space.formula_drill_open = true;
        state.formula_spaces.insert(formula_space);

        let changed = close_formula_drill_on_active_formula_space(&mut state);
        assert!(changed);
        assert!(
            !state
                .formula_spaces
                .get(&formula_space_id)
                .unwrap()
                .formula_drill_open
        );
    }

    #[test]
    fn skin_one_formula_edit_intent_updates_target_formula_space() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state.formula_spaces.insert(FormulaSpaceState::new(
            formula_space_id.clone(),
            "=SUM(1,2)",
        ));

        let changed = apply_skin_intent_to_host_state(
            &mut state,
            dnacalc_skin_ir::SkinIntent::OneFormula(dnacalc_skin_ir::OneFormulaIntent::EditText {
                formula_space_id: formula_space_id.as_str().to_string(),
                text: "=SUM(1,2,3)".to_string(),
                caret_offset: 11,
            }),
        );

        assert!(changed);
        let formula_space = state.formula_spaces.get(&formula_space_id).unwrap();
        assert_eq!(formula_space.raw_entered_cell_text, "=SUM(1,2,3)");
        assert_eq!(formula_space.editor_surface_state.caret.offset, 11);
    }

    #[test]
    fn skin_one_formula_format_and_policy_intents_update_existing_state() {
        let formula_space_id = FormulaSpaceId::new("space-1");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(formula_space_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(formula_space_id.clone(), "=NOW()"));

        assert!(apply_skin_intent_to_host_state(
            &mut state,
            dnacalc_skin_ir::SkinIntent::OneFormula(
                dnacalc_skin_ir::OneFormulaIntent::SetNumberFormat {
                    formula_space_id: formula_space_id.as_str().to_string(),
                    number_format_code: Some("yyyy-mm-dd".to_string()),
                },
            ),
        ));
        assert!(apply_skin_intent_to_host_state(
            &mut state,
            dnacalc_skin_ir::SkinIntent::OneFormula(
                dnacalc_skin_ir::OneFormulaIntent::SetScenarioPolicy {
                    formula_space_id: formula_space_id.as_str().to_string(),
                    policy: dnacalc_skin_ir::ScenarioPolicyProjection::Deterministic,
                },
            ),
        ));

        let formula_space = state.formula_spaces.get(&formula_space_id).unwrap();
        assert_eq!(formula_space.formatting.number_format_code, "yyyy-mm-dd");
        assert_eq!(
            formula_space.formatting.scenario_policy,
            crate::persistence::ScenarioPolicy::Deterministic
        );
    }

    #[test]
    fn skin_one_formula_toggle_drill_intent_uses_target_formula_space() {
        let first_id = FormulaSpaceId::new("space-1");
        let second_id = FormulaSpaceId::new("space-2");
        let mut state = OneCalcHostState::default();
        state.workspace_shell.active_formula_space_id = Some(first_id.clone());
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(first_id.clone(), "=1"));
        state
            .formula_spaces
            .insert(FormulaSpaceState::new(second_id.clone(), "=2"));

        assert!(apply_skin_intent_to_host_state(
            &mut state,
            dnacalc_skin_ir::SkinIntent::OneFormula(
                dnacalc_skin_ir::OneFormulaIntent::ToggleFormulaDrill {
                    formula_space_id: second_id.as_str().to_string(),
                },
            ),
        ));

        assert!(
            !state
                .formula_spaces
                .get(&first_id)
                .unwrap()
                .formula_drill_open
        );
        assert!(
            state
                .formula_spaces
                .get(&second_id)
                .unwrap()
                .formula_drill_open
        );
        assert_eq!(
            state.workspace_shell.active_formula_space_id.as_ref(),
            Some(&first_id)
        );
    }

    #[test]
    fn tree_workspace_skin_intent_is_no_op_in_onecalc_host() {
        let mut state = OneCalcHostState::default();

        let changed = apply_skin_intent_to_host_state(
            &mut state,
            dnacalc_skin_ir::SkinIntent::TreeWorkspace(
                dnacalc_skin_ir::WorkspaceIntent::SelectNode(None),
            ),
        );

        assert!(!changed);
    }

    #[test]
    fn skin_persistence_intents_route_to_host_adapter_state() {
        let mut state = OneCalcHostState::default();
        assert!(apply_skin_intent_to_host_state(
            &mut state,
            dnacalc_skin_ir::SkinIntent::Shell(dnacalc_skin_ir::SkinShellIntent::SaveAs {
                suggested_path: Some("case.dnaone".into()),
            }),
        ));
        assert_eq!(
            state.workspace_shell.pending_persistence_intent,
            Some(crate::state::WorkspacePersistenceIntent::SaveAs {
                suggested_path: Some("case.dnaone".into())
            })
        );
    }

    #[test]
    fn skin_open_recent_reopens_the_real_closed_formula() {
        let mut state = OneCalcHostState::default();
        let id = FormulaSpaceId::new("recent-1");
        state
            .workspace_shell
            .recent_formula_space_order
            .push(id.clone());
        state.workspace_shell.recent_formula_spaces.insert(
            id.clone(),
            crate::state::ClosedFormulaSpaceRecord {
                formula_space: FormulaSpaceState::new(id.clone(), "=SUM(1,2)"),
                last_active_mode: crate::state::AppMode::Explore,
            },
        );
        assert!(apply_skin_intent_to_host_state(
            &mut state,
            dnacalc_skin_ir::SkinIntent::Shell(dnacalc_skin_ir::SkinShellIntent::OpenRecent {
                document_id: id.as_str().into(),
            }),
        ));
        assert!(state.formula_spaces.spaces.contains_key(&id));
    }
}
