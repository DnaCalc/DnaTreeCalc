use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationSurfaceDescriptor {
    pub surface_id: String,
    pub surface_kind: String,
    pub locator: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationSurfaceValue {
    pub surface: ObservationSurfaceDescriptor,
    pub status: String,
    pub value_repr: Option<String>,
    pub capture_loss: String,
    pub uncertainty: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationInterpretation {
    pub bridge_influenced: bool,
    pub interpretation_limits: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationCapturePayload {
    pub surfaces: Vec<ObservationSurfaceValue>,
    pub interpretation: ObservationInterpretation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationBridgePayload {
    pub scenario_id: String,
    pub bridge_kind: String,
    pub bridge_version: String,
    pub executable_identity: String,
    pub command_channel: String,
    pub invocation_mode: String,
    pub interpretation_limits: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationProvenancePayload {
    pub scenario_id: String,
    pub run_id: String,
    pub workbook_ref: String,
    pub workbook_fingerprint: String,
    pub excel_version: String,
    pub excel_build: String,
    pub excel_channel: String,
    pub host_os: String,
    pub host_architecture: String,
    pub macro_mode: String,
    pub automation_policy: String,
    pub captured_at_utc: String,
    pub timezone: String,
    pub declared_surface_ids: Vec<String>,
    pub capture_loss_summary: Vec<String>,
    pub uncertainty_summary: Vec<String>,
    pub bridge: ObservationBridgePayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedObservationSourceBundle {
    pub source_root: PathBuf,
    pub capture_path: PathBuf,
    pub provenance_path: PathBuf,
    pub bundle_path: Option<PathBuf>,
    pub replay_manifest_path: Option<PathBuf>,
    pub normalized_replay_path: Option<PathBuf>,
    pub capture: ObservationCapturePayload,
    pub provenance: ObservationProvenancePayload,
}

pub fn invoke_live_windows_capture(
    output_root: impl AsRef<Path>,
) -> Result<LoadedObservationSourceBundle, String> {
    let oxxlplay_root = oxxlplay_repo_root();
    let scenario_path = oxxlplay_root
        .join("docs/test-corpus/excel/xlplay_capture_values_formulae_001/scenario.json");
    let output_root = output_root.as_ref();
    fs::create_dir_all(output_root).map_err(|error| error.to_string())?;

    let status = Command::new("cargo")
        .arg("run")
        .arg("-p")
        .arg("oxxlplay-cli")
        .arg("--")
        .arg("capture-run")
        .arg("--scenario")
        .arg(&scenario_path)
        .arg("--output-dir")
        .arg(output_root)
        .current_dir(&oxxlplay_root)
        .status()
        .map_err(|error| format!("failed to start OxXlPlay capture-run: {error}"))?;

    if !status.success() {
        return match status.code() {
            Some(code) => Err(format!("OxXlPlay capture-run exited with code {code}")),
            None => Err("OxXlPlay capture-run exited without a status code".to_string()),
        };
    }

    load_observation_source_bundle(output_root)
}

pub fn load_observation_source_bundle(
    source_root: impl AsRef<Path>,
) -> Result<LoadedObservationSourceBundle, String> {
    let source_root = source_root.as_ref().to_path_buf();
    let capture_path = source_root.join("capture.json");
    let provenance_path = source_root.join("provenance.json");
    let bundle_path = source_root.join("bundle.json");
    let replay_manifest_path = source_root.join("oxreplay-manifest.json");
    let normalized_replay_path = source_root.join("views").join("normalized-replay.json");
    let capture = read_json::<ObservationCapturePayload>(&capture_path)?;
    let provenance = read_json::<ObservationProvenancePayload>(&provenance_path)?;

    Ok(LoadedObservationSourceBundle {
        source_root,
        capture_path,
        provenance_path,
        bundle_path: bundle_path.is_file().then_some(bundle_path),
        replay_manifest_path: replay_manifest_path
            .is_file()
            .then_some(replay_manifest_path),
        normalized_replay_path: normalized_replay_path
            .is_file()
            .then_some(normalized_replay_path),
        capture,
        provenance,
    })
}

fn oxxlplay_repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("OxXlPlay")
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let body = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&body)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}
