//! Per-formula persistence.
//!
//! Slice 1 ships the in-memory `Scenario` shape + XML emitter + XML
//! parser for `.dnafml` / `.xml` files (`formula_file`). Slice 1b
//! adds the host-side projection (`scenario_projection`) and
//! browser-host file IO (`browser_file_io` — wasm32-only). Tauri
//! command-backed VBA file selection starts in WS-15; broader Tauri
//! file IO and the `<dna:CompareBundle>` merge are later slices.
//!
//! See `docs/PERSISTENCE_FORMAT_PLAN.md` §10 for the full seam ladder.

pub mod formula_file;
pub mod scenario_projection;
pub mod workspace_storage;

#[cfg(target_arch = "wasm32")]
pub mod browser_file_io;
#[cfg(target_arch = "wasm32")]
pub mod tauri_file_io;

pub use formula_file::{
    apply_bundle_retention_policy, read_formula_xml, write_formula_xml, BundleVerdict, CfRule,
    CompareBundle, Context, Entry, EntryMode, FormulaFileError, HostProfile, Identity,
    LoadDiagnostic, LoadedFormula, Locale, PublicationContext, Scenario, ScenarioPolicy,
    UiPreferences, DEFAULT_BUNDLE_RETENTION_CAP,
};
pub use scenario_projection::{
    apply_loaded_scenario_to_formula_space, apply_loaded_scenario_with_diagnostics,
    formula_space_to_scenario,
};
pub use workspace_storage::{
    deserialize_workspace, hydrate_state_from_local_storage, save_workspace_to_local_storage,
    serialize_workspace, WorkspaceJson, WorkspaceLoadError, WORKSPACE_STORAGE_KEY,
};

/// Platform adapter for the core-owned workspace lifecycle. The wire format
/// remains host-specific; the Leptos shell does not own hydration or saving.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalWorkspacePersistence;

#[cfg(not(target_arch = "wasm32"))]
pub fn save_workspace_to_path(
    state: &crate::state::OneCalcHostState,
    path: &str,
) -> Result<(), String> {
    let path = std::path::Path::new(path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let json = serialize_workspace(state).map_err(|error| error.to_string())?;
    std::fs::write(path, json).map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn open_workspace_from_path(
    state: &mut crate::state::OneCalcHostState,
    path: &str,
) -> Result<(), String> {
    let json = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let workspace = deserialize_workspace(&json).map_err(|error| error.to_string())?;
    workspace
        .apply_to_state(state)
        .map_err(|error| error.to_string())
}

impl dnaonecalc_core::StatePersistence<crate::state::OneCalcHostState>
    for LocalWorkspacePersistence
{
    fn hydrate(&mut self, state: &mut crate::state::OneCalcHostState) {
        hydrate_state_from_local_storage(state);
    }

    fn persist(&mut self, state: &crate::state::OneCalcHostState) {
        save_workspace_to_local_storage(state);
    }
}

#[cfg(target_arch = "wasm32")]
pub use browser_file_io::{
    open_xml_via_file_input, save_xml_via_download, suggested_filename_stem, OpenedFormulaFile,
};
