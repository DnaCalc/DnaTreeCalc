//! Workspace-level persistence for the browser host.
//!
//! Per `docs/PERSISTENCE_FORMAT_PLAN.md` §1A.2, `workspace.json`
//! lives at `localStorage["dnaonecalc.workspace.v1"]` on the
//! browser host (and at a per-user app-data path on the eventual
//! Tauri host). It carries app-state — pinned formula ids, the
//! active formula's last-edited content, the recents list — that
//! survives reload but is **not** part of any single user
//! document.
//!
//! Slice 1 of `SEAM-ONECALC-SCENARIO-PERSIST` ships the minimum
//! viable surface:
//!
//! - **Pinned ids** — survive reload.
//! - **Active formula snapshot** — last-edited text + formatting
//!   round-trip back into the workspace on the next load.
//!
//! Recents-with-full-state (the larger surface that carries N
//! recently-closed formulas across reload) is the obvious next
//! slice. Until then the recents list is in-memory only — it
//! tracks the current session, not history-across-reloads.
//!
//! The persistence format is JSON with a stable `version: 1` field
//! at the root. Inside the JSON, the active formula is itself
//! serialised as the same `.dnafml` XML the host already emits via
//! `write_formula_xml` — so workspace persistence reuses the
//! formula-file round-trip rather than re-stating its shape.

use crate::persistence::{
    apply_loaded_scenario_to_formula_space, formula_space_to_scenario, read_formula_xml,
    write_formula_xml, FormulaFileError,
};
use crate::state::{AppMode, ClosedFormulaSpaceRecord, FormulaSpaceState, OneCalcHostState};
use serde::{Deserialize, Serialize};

/// Stable storage key. Bumped only on incompatible schema changes
/// — the JSON's `version` field is the soft compatibility lever
/// for additive changes within a key generation.
pub const WORKSPACE_STORAGE_KEY: &str = "dnaonecalc.workspace.v1";

/// Top-level workspace.json envelope. Versioned so the loader can
/// reject futures it doesn't understand and tolerate older shapes.
///
/// Schema history:
/// - v1: pins + single active-formula XML.
/// - v2 (current): pins + open-formula list (each chip in the tab
///   strip survives reload) + recent-formula list (closed
///   formulas survive reload too) + which open formula was
///   active at save time.
///
/// The loader still accepts v1 envelopes — it treats
/// `active_formula_xml` as a single open-and-active formula.
/// Writers always emit v2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceJson {
    /// Schema version. The loader accepts `1` and `2`; writers
    /// always emit `2`.
    pub version: u32,
    /// Pinned formula ids in stable user-visible order. Reloading
    /// the workspace re-inserts each id into
    /// `workspace_shell.pinned_formula_space_ids`. Ids without a
    /// matching restored formula stay in the pinned set — the
    /// breadcrumb / tab-strip surfaces them with a placeholder
    /// label until the user opens them again.
    #[serde(default)]
    pub pinned_formula_space_ids: Vec<String>,
    /// Optional active-formula snapshot — v1-only. Carried
    /// forward in the struct so v1 envelopes still parse; v2
    /// writers emit `None` here and use `open_formulas` /
    /// `active_formula_id` instead.
    #[serde(default)]
    pub active_formula_xml: Option<String>,
    /// All open formulas in `workspace_shell.open_formula_space_order`
    /// position. Each entry round-trips through the same `.dnafml`
    /// XML the host emits for `Save as…`, so formatting / CF
    /// rules / scenario policy / drill-down state all survive.
    /// Empty in v1 envelopes.
    #[serde(default)]
    pub open_formulas: Vec<PersistedFormulaSnapshot>,
    /// Closed-but-recent formulas. Mirrors
    /// `workspace_shell.recent_formula_spaces` plus its
    /// `recent_formula_space_order`. Each carries its own
    /// `last_active_mode` so reopening lands the user back in the
    /// same surface.
    #[serde(default)]
    pub recent_formulas: Vec<PersistedRecentFormula>,
    /// Which open-formula id was active at save time. Restored
    /// after `open_formulas` is rehydrated. `None` falls back to
    /// the first open formula.
    #[serde(default)]
    pub active_formula_id: Option<String>,
}

/// One open / closed formula snapshot. The `.dnafml` XML carries
/// everything the host needs to reconstruct the formula's editor
/// state (raw text, committed text, formatting, CF rules,
/// scenario policy, drill-down expansion). The id stays alongside
/// because the XML's `<dna:Identity>` doesn't always carry the
/// synthetic `untitled-N` id back unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedFormulaSnapshot {
    pub formula_space_id: String,
    pub xml: String,
}

/// Recent (closed) formula. Adds the `last_active_mode` slot so
/// reopening lands the user in the same shell surface they had
/// when they closed the formula.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedRecentFormula {
    pub formula_space_id: String,
    pub xml: String,
    /// Stringified `AppMode` (`"Explore"` / `"Inspect"` /
    /// `"Workbench"`). Round-trips via
    /// [`AppMode::parse_persisted`]; an unknown string falls back
    /// to `Explore`.
    pub last_active_mode: String,
}

impl WorkspaceJson {
    /// Project the live host state into a workspace.json envelope
    /// (schema v2). Carries:
    /// - the pinned id list,
    /// - every open formula's `.dnafml` XML in stable
    ///   `open_formula_space_order` position,
    /// - every recent (closed) formula's XML + last-active mode in
    ///   `recent_formula_space_order` position,
    /// - the id of whichever open formula was active at save time.
    pub fn from_state(state: &OneCalcHostState) -> Self {
        let pinned_formula_space_ids = state
            .workspace_shell
            .pinned_formula_space_ids
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        let open_formulas: Vec<PersistedFormulaSnapshot> = state
            .workspace_shell
            .open_formula_space_order
            .iter()
            .filter_map(|id| {
                let space = state.formula_spaces.get(id)?;
                let scenario = formula_space_to_scenario(space, String::new(), String::new());
                Some(PersistedFormulaSnapshot {
                    formula_space_id: id.as_str().to_string(),
                    xml: write_formula_xml(&scenario),
                })
            })
            .collect();
        let recent_formulas: Vec<PersistedRecentFormula> = state
            .workspace_shell
            .recent_formula_space_order
            .iter()
            .filter_map(|id| {
                let record = state.workspace_shell.recent_formula_spaces.get(id)?;
                let scenario =
                    formula_space_to_scenario(&record.formula_space, String::new(), String::new());
                Some(PersistedRecentFormula {
                    formula_space_id: id.as_str().to_string(),
                    xml: write_formula_xml(&scenario),
                    last_active_mode: persist_app_mode(record.last_active_mode),
                })
            })
            .collect();
        let active_formula_id = state
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .map(|id| id.as_str().to_string());
        Self {
            version: 2,
            pinned_formula_space_ids,
            // v2 always emits `None` here — open / recent /
            // active_formula_id carry the per-formula state. Kept
            // in the struct for v1 backward-compat read.
            active_formula_xml: None,
            open_formulas,
            recent_formulas,
            active_formula_id,
        }
    }

    /// Apply the workspace.json envelope back onto the host
    /// state. Restores pins, every open formula's content, every
    /// recent formula's content, and the active-formula id.
    ///
    /// The host's mount path always boots with a fresh
    /// `untitled-1` open. This loader replaces that placeholder
    /// when v2 envelopes carry a non-empty `open_formulas` list;
    /// for v1 envelopes the single `active_formula_xml` is
    /// applied to whichever formula space is currently active
    /// (the boot placeholder).
    pub fn apply_to_state(self, state: &mut OneCalcHostState) -> Result<(), WorkspaceLoadError> {
        if self.version != 1 && self.version != 2 {
            return Err(WorkspaceLoadError::UnsupportedVersion(self.version));
        }
        // Re-insert pins.
        state.workspace_shell.pinned_formula_space_ids.clear();
        for id in self.pinned_formula_space_ids {
            state
                .workspace_shell
                .pinned_formula_space_ids
                .insert(crate::domain::ids::FormulaSpaceId::new(id));
        }

        // v2 path: rebuild the open-formula list from scratch so
        // the boot placeholder doesn't leak into the restored
        // workspace. v1 path: fall through to the legacy
        // single-active-XML restore below.
        if self.version == 2 && !self.open_formulas.is_empty() {
            // Drop the boot placeholder and any other open
            // formulas — we're going to repopulate from the
            // envelope.
            state.formula_spaces.spaces.clear();
            state.workspace_shell.open_formula_space_order.clear();
            state.workspace_shell.formula_space_modes.clear();
            for snapshot in self.open_formulas {
                let id = crate::domain::ids::FormulaSpaceId::new(snapshot.formula_space_id.clone());
                let loaded =
                    read_formula_xml(&snapshot.xml).map_err(WorkspaceLoadError::FormulaFile)?;
                let mut formula_space = FormulaSpaceState::new(id.clone(), "");
                apply_loaded_scenario_to_formula_space(&mut formula_space, loaded.scenario);
                state.formula_spaces.insert(formula_space);
                state
                    .workspace_shell
                    .open_formula_space_order
                    .push(id.clone());
                state
                    .workspace_shell
                    .formula_space_modes
                    .insert(id, AppMode::Explore);
            }
            // Restore active-id, falling back to the first open
            // formula when the envelope didn't name one (or the
            // named id no longer exists in the open set).
            let active_id = self
                .active_formula_id
                .map(crate::domain::ids::FormulaSpaceId::new)
                .filter(|id| state.workspace_shell.open_formula_space_order.contains(id))
                .or_else(|| {
                    state
                        .workspace_shell
                        .open_formula_space_order
                        .first()
                        .cloned()
                });
            state.workspace_shell.active_formula_space_id = active_id.clone();
            state.active_formula_space_view.selected_formula_space_id = active_id;

            // Recent (closed) formulas: rebuild
            // `recent_formula_spaces` map + its order vector.
            state.workspace_shell.recent_formula_spaces.clear();
            state.workspace_shell.recent_formula_space_order.clear();
            for recent in self.recent_formulas {
                let id = crate::domain::ids::FormulaSpaceId::new(recent.formula_space_id.clone());
                let loaded =
                    read_formula_xml(&recent.xml).map_err(WorkspaceLoadError::FormulaFile)?;
                let mut formula_space = FormulaSpaceState::new(id.clone(), "");
                apply_loaded_scenario_to_formula_space(&mut formula_space, loaded.scenario);
                let last_active_mode = parse_app_mode(&recent.last_active_mode);
                state.workspace_shell.recent_formula_spaces.insert(
                    id.clone(),
                    ClosedFormulaSpaceRecord {
                        formula_space,
                        last_active_mode,
                    },
                );
                state.workspace_shell.recent_formula_space_order.push(id);
            }
        } else if let Some(xml) = self.active_formula_xml {
            // v1 fallback: apply the single active-formula XML to
            // whatever target the host has open (typically the
            // boot placeholder). Pre-existing test corpus + any
            // workspaces saved before the v2 bump still load.
            let loaded = read_formula_xml(&xml).map_err(WorkspaceLoadError::FormulaFile)?;
            let target_id = state
                .workspace_shell
                .active_formula_space_id
                .clone()
                .or_else(|| {
                    state
                        .workspace_shell
                        .open_formula_space_order
                        .first()
                        .cloned()
                });
            let Some(target_id) = target_id else {
                return Err(WorkspaceLoadError::NoTargetFormulaSpace);
            };
            let Some(target) = state.formula_spaces.get_mut(&target_id) else {
                return Err(WorkspaceLoadError::NoTargetFormulaSpace);
            };
            apply_loaded_scenario_to_formula_space(target, loaded.scenario);
            state.workspace_shell.active_formula_space_id = Some(target_id);
        }
        Ok(())
    }
}

/// Round-trip helper for `AppMode` → `String` → `AppMode`. Kept
/// in lock-step with the on-disk shape used by v2 envelopes; an
/// unknown string falls back to `Explore` so older workspaces
/// keep loading even when the enum grows.
fn persist_app_mode(mode: AppMode) -> String {
    match mode {
        AppMode::Explore => "Explore",
        AppMode::Inspect => "Inspect",
        AppMode::Workbench => "Workbench",
    }
    .to_string()
}

fn parse_app_mode(raw: &str) -> AppMode {
    match raw {
        "Inspect" => AppMode::Inspect,
        "Workbench" => AppMode::Workbench,
        _ => AppMode::Explore,
    }
}

/// Errors the workspace loader can surface. `UnsupportedVersion`
/// fires when the stored JSON's `version` doesn't match what this
/// build understands; `FormulaFile` propagates a parse failure
/// from the embedded `.dnafml` XML; `Json` wraps a `serde_json`
/// failure on the envelope itself.
#[derive(Debug)]
pub enum WorkspaceLoadError {
    UnsupportedVersion(u32),
    FormulaFile(FormulaFileError),
    Json(serde_json::Error),
    NoTargetFormulaSpace,
    #[cfg(target_arch = "wasm32")]
    StorageUnavailable,
    #[cfg(not(target_arch = "wasm32"))]
    Io(std::io::Error),
}

impl core::fmt::Display for WorkspaceLoadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnsupportedVersion(v) => write!(f, "unsupported workspace version {v}"),
            Self::FormulaFile(e) => write!(f, "formula-file parse failure: {e}"),
            Self::Json(e) => write!(f, "workspace.json parse failure: {e}"),
            Self::NoTargetFormulaSpace => {
                write!(f, "workspace has no formula space to apply the snapshot to")
            }
            #[cfg(target_arch = "wasm32")]
            Self::StorageUnavailable => write!(f, "browser localStorage unavailable"),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Io(e) => write!(f, "workspace.json file IO failure: {e}"),
        }
    }
}

impl std::error::Error for WorkspaceLoadError {}

/// Serialize the host state into the workspace.json wire form.
pub fn serialize_workspace(state: &OneCalcHostState) -> Result<String, serde_json::Error> {
    let envelope = WorkspaceJson::from_state(state);
    serde_json::to_string(&envelope)
}

/// Parse a workspace.json string into the envelope.
pub fn deserialize_workspace(json: &str) -> Result<WorkspaceJson, serde_json::Error> {
    serde_json::from_str(json)
}

// ---------------------------------------------------------------
// Browser localStorage adapter (wasm32-only)
// ---------------------------------------------------------------

/// Read the workspace envelope from `localStorage`. Returns
/// `Ok(None)` when no entry has been written yet (fresh user) and
/// `Err(...)` for storage failures or schema-incompatibility.
#[cfg(target_arch = "wasm32")]
pub fn load_workspace_from_local_storage() -> Result<Option<WorkspaceJson>, WorkspaceLoadError> {
    let storage = web_sys::window()
        .and_then(|w| w.local_storage().ok())
        .flatten()
        .ok_or(WorkspaceLoadError::StorageUnavailable)?;
    let raw = match storage.get_item(WORKSPACE_STORAGE_KEY) {
        Ok(Some(value)) => value,
        Ok(None) => return Ok(None),
        Err(_) => return Err(WorkspaceLoadError::StorageUnavailable),
    };
    let envelope = deserialize_workspace(&raw).map_err(WorkspaceLoadError::Json)?;
    Ok(Some(envelope))
}

/// Write the workspace envelope into `localStorage`. Silent on
/// success; logs to the console on failure (storage quota, the
/// user disabling site data, etc.) so persistence failures don't
/// take the rest of the app down with them.
#[cfg(target_arch = "wasm32")]
pub fn save_workspace_to_local_storage(state: &OneCalcHostState) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(Some(storage)) = window.local_storage() else {
        return;
    };
    let json = match serialize_workspace(state) {
        Ok(json) => json,
        Err(error) => {
            web_sys::console::error_1(
                &format!("[onecalc] workspace.json serialise failed: {error}").into(),
            );
            return;
        }
    };
    if let Err(error) = storage.set_item(WORKSPACE_STORAGE_KEY, &json) {
        web_sys::console::error_1(
            &format!("[onecalc] workspace.json write failed: {error:?}").into(),
        );
    }
}

/// Apply the most recently saved workspace.json (if any) to the
/// host state. Called from the home-shell mount before the
/// component subscribes to the state signal so the user sees their
/// last-edited formula immediately.
#[cfg(target_arch = "wasm32")]
pub fn hydrate_state_from_local_storage(state: &mut OneCalcHostState) {
    match load_workspace_from_local_storage() {
        Ok(Some(envelope)) => {
            if let Err(error) = envelope.apply_to_state(state) {
                web_sys::console::warn_1(
                    &format!("[onecalc] workspace.json apply failed: {error}").into(),
                );
            }
        }
        Ok(None) => {
            // Fresh user — no prior workspace to restore. Not an
            // error.
        }
        Err(error) => {
            web_sys::console::warn_1(
                &format!("[onecalc] workspace.json load failed: {error}").into(),
            );
        }
    }
}

// ---------------------------------------------------------------
// Native (Tauri / desktop) file-backed adapter
// ---------------------------------------------------------------
//
// The browser host uses `localStorage`; the Tauri / desktop host
// stores `workspace.json` on disk under the platform-standard
// per-user app-data directory. Both adapters share the same
// envelope shape (`WorkspaceJson`) and the same auto-save /
// hydrate API; the home-shell mount calls
// `save_workspace_to_local_storage` and
// `hydrate_state_from_local_storage` regardless of target — the
// "local_storage" name is a slight misnomer on disk but kept for
// callsite stability across the wasm / native boundary. The
// actual paths:
//
// | Host | Path |
// |---|---|
// | Windows | `%APPDATA%\DnaOneCalc\workspace.json` |
// | macOS | `~/Library/Application Support/DnaOneCalc/workspace.json` |
// | Linux | `$XDG_CONFIG_HOME/DnaOneCalc/workspace.json` (falls back to `~/.config/...`) |
// | Browser | `localStorage["dnaonecalc.workspace.v1"]` (no file) |
//
// A `DNAONECALC_WORKSPACE_DIR` env var overrides the path —
// useful for tests and headless runs that want to write to a
// scratch dir.

#[cfg(not(target_arch = "wasm32"))]
const WORKSPACE_FILENAME: &str = "workspace.json";

#[cfg(not(target_arch = "wasm32"))]
const WORKSPACE_APP_DIR: &str = "DnaOneCalc";

/// Resolve the platform-appropriate `workspace.json` path. Honours
/// `DNAONECALC_WORKSPACE_DIR` (test override) before falling
/// through to the OS-standard user-config location. Returns `None`
/// when no usable directory could be located (rare; means
/// `$HOME` / `%APPDATA%` are all unset, in which case persistence
/// silently degrades to in-memory).
#[cfg(not(target_arch = "wasm32"))]
pub fn workspace_storage_path() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    if let Ok(dir) = std::env::var("DNAONECALC_WORKSPACE_DIR") {
        if !dir.is_empty() {
            return Some(PathBuf::from(dir).join(WORKSPACE_FILENAME));
        }
    }
    let base = if cfg!(target_os = "windows") {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var("HOME")
            .ok()
            .map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else {
        // Linux / BSDs: respect XDG, fall back to ~/.config.
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var("HOME")
                    .ok()
                    .map(|home| PathBuf::from(home).join(".config"))
            })
    }?;
    Some(base.join(WORKSPACE_APP_DIR).join(WORKSPACE_FILENAME))
}

/// Load the workspace.json file from disk. Returns `Ok(None)`
/// when no file exists (fresh user) and `Err` for read failures
/// or schema-incompatibility.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_workspace_from_disk() -> Result<Option<WorkspaceJson>, WorkspaceLoadError> {
    let Some(path) = workspace_storage_path() else {
        return Ok(None);
    };
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path).map_err(WorkspaceLoadError::Io)?;
    let envelope = deserialize_workspace(&raw).map_err(WorkspaceLoadError::Json)?;
    Ok(Some(envelope))
}

/// Write the workspace envelope to disk. Creates the parent
/// directory as needed; logs (via `eprintln!`) on failure rather
/// than panicking so persistence failures don't crash the app.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_workspace_to_local_storage(state: &OneCalcHostState) {
    let Some(path) = workspace_storage_path() else {
        return;
    };
    let json = match serialize_workspace(state) {
        Ok(json) => json,
        Err(error) => {
            eprintln!("[onecalc] workspace.json serialise failed: {error}");
            return;
        }
    };
    if let Some(parent) = path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            eprintln!(
                "[onecalc] workspace.json mkdir failed at `{}`: {error}",
                parent.display(),
            );
            return;
        }
    }
    if let Err(error) = std::fs::write(&path, json) {
        eprintln!(
            "[onecalc] workspace.json write failed at `{}`: {error}",
            path.display(),
        );
    }
}

/// Hydrate the host state from disk (mirror of the wasm
/// `hydrate_state_from_local_storage`). Called from the home-
/// shell mount before any subscriber sees the state signal.
#[cfg(not(target_arch = "wasm32"))]
pub fn hydrate_state_from_local_storage(state: &mut OneCalcHostState) {
    match load_workspace_from_disk() {
        Ok(Some(envelope)) => {
            if let Err(error) = envelope.apply_to_state(state) {
                eprintln!("[onecalc] workspace.json apply failed: {error}");
            }
        }
        Ok(None) => {
            // Fresh user — no prior workspace to restore. Not an
            // error.
        }
        Err(error) => {
            eprintln!("[onecalc] workspace.json load failed: {error}");
        }
    }
}

// Take an unused import to silence the warning when the wasm path
// isn't compiled.
#[cfg(not(target_arch = "wasm32"))]
fn _silence_unused(_: &FormulaSpaceState) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::case_lifecycle::{
        new_formula_space, pin_active_formula_space, toggle_pin_formula_space,
    };

    #[test]
    fn workspace_round_trips_pins_and_active_formula_text() {
        let mut original = OneCalcHostState::default();
        let id = new_formula_space(&mut original);
        // Mutate the formula a bit so we can assert restoration.
        let formula_space = original.formula_spaces.get_mut(&id).expect("active space");
        formula_space.raw_entered_cell_text = "=SUM(1,2,3)".to_string();
        formula_space.committed_cell_text = Some("=SUM(1,2,3)".to_string());
        formula_space.formatting.number_format_code = "0.00".to_string();
        // Pin the active formula.
        let _ = pin_active_formula_space(&mut original);

        // Round-trip through the JSON envelope.
        let json = serialize_workspace(&original).expect("serialise");
        let restored_envelope = deserialize_workspace(&json).expect("parse");

        // Boot a fresh host and apply the envelope on top.
        let mut restored = OneCalcHostState::default();
        let _ = new_formula_space(&mut restored);
        restored_envelope
            .apply_to_state(&mut restored)
            .expect("apply");

        // Pins survive verbatim.
        let pinned_ids: Vec<_> = restored
            .workspace_shell
            .pinned_formula_space_ids
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        assert_eq!(
            pinned_ids,
            vec![id.as_str().to_string()],
            "pinned ids must round-trip verbatim",
        );

        // Active formula text + formatting round-trips.
        let active = restored
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|aid| restored.formula_spaces.get(aid))
            .expect("active restored");
        assert_eq!(active.raw_entered_cell_text, "=SUM(1,2,3)");
        assert_eq!(active.formatting.number_format_code, "0.00");
    }

    #[test]
    fn workspace_envelope_rejects_future_versions() {
        let envelope = WorkspaceJson {
            version: 999,
            ..WorkspaceJson::default()
        };
        let mut state = OneCalcHostState::default();
        let _ = new_formula_space(&mut state);
        let result = envelope.apply_to_state(&mut state);
        assert!(matches!(
            result,
            Err(WorkspaceLoadError::UnsupportedVersion(999)),
        ));
    }

    impl Default for WorkspaceJson {
        fn default() -> Self {
            Self {
                version: 2,
                pinned_formula_space_ids: Vec::new(),
                active_formula_xml: None,
                open_formulas: Vec::new(),
                recent_formulas: Vec::new(),
                active_formula_id: None,
            }
        }
    }

    #[test]
    fn pin_toggle_persists_through_envelope() {
        let mut state = OneCalcHostState::default();
        let id = new_formula_space(&mut state);
        // Pin via toggle, then unpin via toggle.
        assert!(toggle_pin_formula_space(&mut state, id.as_str()));
        let json_pinned = serialize_workspace(&state).expect("serialise pinned");
        assert!(toggle_pin_formula_space(&mut state, id.as_str()));
        let json_unpinned = serialize_workspace(&state).expect("serialise unpinned");
        assert_ne!(
            json_pinned, json_unpinned,
            "envelope must reflect pin-toggle state",
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn workspace_disk_round_trip_via_env_override() {
        use std::sync::Mutex;
        // The override env var is process-global; serialise the
        // disk-path tests through a mutex so concurrent test
        // threads don't see each other's value.
        static ENV_LOCK: Mutex<()> = Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap();

        // Use a scratch directory under `target/` so the test
        // never touches the user's real config dir.
        let scratch =
            std::env::temp_dir().join(format!("dnaonecalc-workspace-test-{}", std::process::id(),));
        if scratch.exists() {
            let _ = std::fs::remove_dir_all(&scratch);
        }
        std::env::set_var("DNAONECALC_WORKSPACE_DIR", &scratch);

        let mut original = OneCalcHostState::default();
        let id = new_formula_space(&mut original);
        original
            .formula_spaces
            .get_mut(&id)
            .expect("space")
            .raw_entered_cell_text = "=42".to_string();
        // Save: writes the file to the scratch dir.
        save_workspace_to_local_storage(&original);
        let path = workspace_storage_path().expect("path resolves");
        assert!(path.exists(), "save must create the file at {path:?}");

        // Load: round-trips into a fresh state.
        let mut restored = OneCalcHostState::default();
        let _ = new_formula_space(&mut restored);
        hydrate_state_from_local_storage(&mut restored);
        let restored_active = restored
            .workspace_shell
            .active_formula_space_id
            .as_ref()
            .and_then(|aid| restored.formula_spaces.get(aid))
            .expect("active restored");
        assert_eq!(restored_active.raw_entered_cell_text, "=42");

        // Cleanup.
        let _ = std::fs::remove_dir_all(&scratch);
        std::env::remove_var("DNAONECALC_WORKSPACE_DIR");
    }

    #[test]
    fn workspace_v2_round_trips_multiple_open_formulas() {
        // Build a workspace with two open formulas + one closed
        // (recent) formula, mutate each one's text + formatting
        // so we can assert distinct round-trip.
        let mut original = OneCalcHostState::default();
        let id_a = new_formula_space(&mut original);
        let id_b = new_formula_space(&mut original);

        original
            .formula_spaces
            .get_mut(&id_a)
            .expect("space a")
            .raw_entered_cell_text = "=A1+B1".to_string();
        let formula_b = original.formula_spaces.get_mut(&id_b).expect("space b");
        formula_b.raw_entered_cell_text = "=SUM(C1:C10)".to_string();
        formula_b.formatting.number_format_code = "$#,##0.00".to_string();

        // Close formula_a so it lands in recents.
        let _ = crate::app::case_lifecycle::close_formula_space(&mut original, id_a.as_str());

        // Round-trip the envelope.
        let json = serialize_workspace(&original).expect("serialise");
        let envelope = deserialize_workspace(&json).expect("parse");

        // Apply onto a fresh host (which boots with one
        // placeholder open formula).
        let mut restored = OneCalcHostState::default();
        let _ = new_formula_space(&mut restored);
        envelope.apply_to_state(&mut restored).expect("apply");

        // formula_b is the surviving open formula; assert text +
        // formatting survived.
        let restored_b = restored.formula_spaces.get(&id_b).expect("restored b");
        assert_eq!(restored_b.raw_entered_cell_text, "=SUM(C1:C10)");
        assert_eq!(restored_b.formatting.number_format_code, "$#,##0.00");
        // The active id is the surviving open one (formula_a was
        // closed and now lives in recents).
        assert_eq!(
            restored.workspace_shell.active_formula_space_id.as_ref(),
            Some(&id_b),
        );
        // formula_a survives in `recent_formula_spaces`.
        let recent_a = restored
            .workspace_shell
            .recent_formula_spaces
            .get(&id_a)
            .expect("recent a");
        assert_eq!(recent_a.formula_space.raw_entered_cell_text, "=A1+B1");
    }
}
