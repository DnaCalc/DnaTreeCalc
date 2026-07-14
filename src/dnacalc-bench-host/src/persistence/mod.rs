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

/// Shared serialization point for every test in this crate that redirects
/// `DNAONECALC_WORKSPACE_DIR` (a process-global env var) to a scratch
/// directory. `cargo test` runs test functions concurrently by default, so
/// two tests in *different* modules each holding their own private
/// `Mutex` would not actually serialize against each other and could race
/// on the same env var (each pointing it at a different scratch dir mid-
/// test). Every such test — in `persistence::workspace_storage` or
/// elsewhere (e.g. `adapters::skin_session`) — must hold this ONE lock for
/// the duration of its override.
#[cfg(test)]
pub(crate) static WORKSPACE_DIR_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
    open_workspace_from_content(state, &json, Some(path))
}

/// Apply a workspace.json *string* (not a filesystem path) to the host
/// state — the both-target Open seam the wasm Bench product uses (bead
/// dtc-lfz.9). wasm cannot `std::fs` a path, so `open_workspace_from_path`
/// above is the native/test-only seam; this one carries the file *content*
/// instead, resolved by an async picker (browser `<input type=file>` or the
/// Tauri desktop dialog) and handed in directly. `source_path` records the
/// origin for the mast's `current_path` when the picker knows it (Tauri: the
/// real path; browser: `None` — there is no addressable path). Mirrors the
/// `Open` handler in `adapters::skin_session`: on success it also clears any
/// pending persistence intent.
pub fn open_workspace_from_content(
    state: &mut crate::state::OneCalcHostState,
    json: &str,
    source_path: Option<&str>,
) -> Result<(), String> {
    let workspace = deserialize_workspace(json).map_err(|error| error.to_string())?;
    workspace
        .apply_to_state(state)
        .map_err(|error| error.to_string())?;
    state.workspace_shell.current_workspace_path = source_path.map(str::to_string);
    state.workspace_shell.pending_persistence_intent = None;
    Ok(())
}

impl dnacalc_bench_core::StatePersistence<crate::state::OneCalcHostState>
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
    open_workspace_via_file_input, open_xml_via_file_input, save_xml_via_download,
    suggested_filename_stem, OpenedFormulaFile, OpenedWorkspaceFile,
};
#[cfg(target_arch = "wasm32")]
pub use tauri_file_io::{
    open_workspace_via_tauri_dialog, tauri_command_bridge_available, TauriOpenedWorkspace,
};
