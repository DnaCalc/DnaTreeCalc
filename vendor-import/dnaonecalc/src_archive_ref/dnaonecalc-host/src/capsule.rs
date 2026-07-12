use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::artifact::{stable_hash, StableArtifactRef};
use crate::retained::{
    CapabilityLedgerSnapshotRecord, ReplayCaptureRecord, RetainedScenarioStore, ScenarioRecord,
    ScenarioRunRecord,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioCapsuleArtifactEntry {
    pub artifact_kind: String,
    pub logical_id: String,
    pub relative_path: String,
    pub content_hash: String,
    pub integrity_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioCapsuleAttachmentEntry {
    pub attachment_kind: String,
    pub attachment_ref: String,
    pub relative_path: String,
    pub content_hash: String,
    pub integrity_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioCapsuleManifest {
    pub capsule_id: String,
    pub schema_id: String,
    pub schema_version: String,
    pub exporter_build_id: String,
    pub root_scenario_id: String,
    pub included_artifacts: Vec<ScenarioCapsuleArtifactEntry>,
    pub included_attachments: Vec<ScenarioCapsuleAttachmentEntry>,
    pub capability_snapshot_refs: Vec<StableArtifactRef>,
    pub lineage_roots: Vec<StableArtifactRef>,
    pub omission_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedScenarioCapsule {
    pub manifest: ScenarioCapsuleManifest,
    pub capsule_root: PathBuf,
    pub manifest_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedScenarioCapsule {
    pub manifest: ScenarioCapsuleManifest,
    pub imported_paths: Vec<PathBuf>,
    pub deduped_paths: Vec<PathBuf>,
    pub conflict_paths: Vec<PathBuf>,
    pub manifest_copy_path: PathBuf,
}

pub fn export_scenario_capsule(
    store: &RetainedScenarioStore,
    capsule_root: impl AsRef<Path>,
    selected_run_ids: &[&str],
) -> Result<PersistedScenarioCapsule, String> {
    if selected_run_ids.is_empty() {
        return Err("ScenarioCapsule export requires at least one selected run".to_string());
    }

    let capsule_root = capsule_root.as_ref();
    let runs = selected_run_ids
        .iter()
        .map(|run_id| store.read_run(run_id))
        .collect::<Result<Vec<_>, _>>()?;
    let scenario = store.read_scenario(&runs[0].scenario_id)?;
    if runs
        .iter()
        .any(|run| run.scenario_id != scenario.scenario_id)
    {
        return Err("ScenarioCapsule export requires runs from one scenario".to_string());
    }

    let mut capabilities = Vec::new();
    let mut replay_captures = Vec::new();
    for run in &runs {
        let snapshot_ref = run
            .envelope
            .capability_snapshot_ref
            .as_ref()
            .ok_or_else(|| {
                format!(
                    "run {} is missing a capability snapshot ref",
                    run.scenario_run_id
                )
            })?;
        let capability = store.read_capability_snapshot(&snapshot_ref.logical_id)?;
        if !capabilities
            .iter()
            .any(|existing: &CapabilityLedgerSnapshotRecord| {
                existing.capability_snapshot_id == capability.capability_snapshot_id
            })
        {
            capabilities.push(capability);
        }

        if let Some(replay_ref) = run.replay_capture_ref.as_ref() {
            let replay_capture = store.read_replay_capture(&replay_ref.logical_id)?;
            if !replay_captures
                .iter()
                .any(|existing: &ReplayCaptureRecord| {
                    existing.replay_capture_id == replay_capture.replay_capture_id
                })
            {
                replay_captures.push(replay_capture);
            }
        }
    }

    fs::create_dir_all(capsule_root).map_err(|error| error.to_string())?;
    for dir in [
        "scenario",
        "runs",
        "observations",
        "comparisons",
        "witnesses",
        "handoffs",
        "capabilities",
        "replay-captures",
        "attachments",
    ] {
        fs::create_dir_all(capsule_root.join(dir)).map_err(|error| error.to_string())?;
    }

    let mut inventory = Vec::new();
    let mut included_attachments = Vec::new();

    let scenario_relative_path = format!("scenario/{}.json", scenario.scenario_id);
    write_pretty_json(capsule_root.join(&scenario_relative_path), &scenario)?;
    inventory.push(build_inventory_entry(
        &scenario_relative_path,
        &scenario.envelope.artifact_kind,
        &scenario.scenario_id,
        &scenario.envelope.content_hash,
        &serde_json::to_string_pretty(&scenario).map_err(|error| error.to_string())?,
    ));

    for run in &runs {
        let relative_path = format!("runs/{}.json", run.scenario_run_id);
        write_pretty_json(capsule_root.join(&relative_path), run)?;
        inventory.push(build_inventory_entry(
            &relative_path,
            &run.envelope.artifact_kind,
            &run.scenario_run_id,
            &run.envelope.content_hash,
            &serde_json::to_string_pretty(run).map_err(|error| error.to_string())?,
        ));
    }

    for capability in &capabilities {
        let relative_path = format!("capabilities/{}.json", capability.capability_snapshot_id);
        write_pretty_json(capsule_root.join(&relative_path), capability)?;
        inventory.push(build_inventory_entry(
            &relative_path,
            &capability.envelope.artifact_kind,
            &capability.capability_snapshot_id,
            &capability.envelope.content_hash,
            &serde_json::to_string_pretty(capability).map_err(|error| error.to_string())?,
        ));
    }

    for replay_capture in &replay_captures {
        let relative_path = format!("replay-captures/{}.json", replay_capture.replay_capture_id);
        write_pretty_json(capsule_root.join(&relative_path), replay_capture)?;
        inventory.push(build_inventory_entry(
            &relative_path,
            &replay_capture.envelope.artifact_kind,
            &replay_capture.replay_capture_id,
            &replay_capture.envelope.content_hash,
            &serde_json::to_string_pretty(replay_capture).map_err(|error| error.to_string())?,
        ));

        let replay_projection_body = fs::read_to_string(replay_projection_path(
            store,
            &replay_capture.replay_capture_id,
        ))
        .map_err(|error| error.to_string())?;
        let replay_relative_path = format!(
            "attachments/{}.replay.json",
            replay_capture.replay_capture_id
        );
        fs::write(
            capsule_root.join(&replay_relative_path),
            &replay_projection_body,
        )
        .map_err(|error| error.to_string())?;
        included_attachments.push(build_attachment_entry(
            "oxfml_replay_projection",
            &replay_capture.replay_capture_id,
            &replay_relative_path,
            &stable_hash(&replay_projection_body),
            &replay_projection_body,
        ));
    }

    let manifest = ScenarioCapsuleManifest {
        capsule_id: format!(
            "capsule-{}-{}",
            scenario.scenario_slug,
            scenario.envelope.created_at_unix_ms
        ),
        schema_id: "dnaonecalc.scenario_capsule".to_string(),
        schema_version: "v1".to_string(),
        exporter_build_id: format!("dnaonecalc-host@{}", env!("CARGO_PKG_VERSION")),
        root_scenario_id: scenario.scenario_id.clone(),
        included_artifacts: inventory,
        included_attachments,
        capability_snapshot_refs: capabilities
            .iter()
            .map(|capability| capability.envelope.stable_ref())
            .collect(),
        lineage_roots: vec![scenario.envelope.stable_ref()],
        omission_notes: vec![
            "observations, comparisons, witnesses, and handoffs not yet exported in the current implementation".to_string(),
            "export includes the authored scenario, selected retained runs, governing capability snapshots, replay captures, and replay projection attachments".to_string(),
        ],
    };

    let manifest_path = capsule_root.join("capsule_manifest.json");
    write_pretty_json(&manifest_path, &manifest)?;

    Ok(PersistedScenarioCapsule {
        manifest,
        capsule_root: capsule_root.to_path_buf(),
        manifest_path,
    })
}

pub fn import_scenario_capsule(
    store: &RetainedScenarioStore,
    capsule_root: impl AsRef<Path>,
) -> Result<ImportedScenarioCapsule, String> {
    let capsule_root = capsule_root.as_ref();
    let manifest_path = capsule_root.join("capsule_manifest.json");
    let manifest = read_json::<ScenarioCapsuleManifest>(&manifest_path)?;

    let mut imported_paths = Vec::new();
    let mut deduped_paths = Vec::new();
    let mut conflict_paths = Vec::new();

    for artifact in &manifest.included_artifacts {
        let source_path = capsule_root.join(&artifact.relative_path);
        let body = fs::read_to_string(&source_path).map_err(|error| error.to_string())?;
        let actual_integrity_hash = stable_hash(&body);
        if actual_integrity_hash != artifact.integrity_hash {
            return Err(format!(
                "capsule integrity mismatch for {}",
                artifact.relative_path
            ));
        }

        validate_artifact_identity(artifact, &body)?;

        let destination_path = imported_artifact_path(store, artifact);
        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        if destination_path.exists() {
            let existing =
                fs::read_to_string(&destination_path).map_err(|error| error.to_string())?;
            let existing_hash = stable_hash(&existing);
            if existing_hash == artifact.integrity_hash {
                deduped_paths.push(destination_path);
                continue;
            }

            let conflict_path = conflict_artifact_path(store, artifact);
            if let Some(parent) = conflict_path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::write(&conflict_path, body).map_err(|error| error.to_string())?;
            conflict_paths.push(conflict_path);
            continue;
        }

        fs::write(&destination_path, body).map_err(|error| error.to_string())?;
        imported_paths.push(destination_path);
    }

    for attachment in &manifest.included_attachments {
        let source_path = capsule_root.join(&attachment.relative_path);
        let body = fs::read_to_string(&source_path).map_err(|error| error.to_string())?;
        let actual_integrity_hash = stable_hash(&body);
        if actual_integrity_hash != attachment.integrity_hash {
            return Err(format!(
                "capsule attachment integrity mismatch for {}",
                attachment.relative_path
            ));
        }

        let destination_path = imported_attachment_path(store, attachment);
        if let Some(parent) = destination_path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        if destination_path.exists() {
            let existing =
                fs::read_to_string(&destination_path).map_err(|error| error.to_string())?;
            let existing_hash = stable_hash(&existing);
            if existing_hash == attachment.integrity_hash {
                deduped_paths.push(destination_path);
                continue;
            }

            let conflict_path = conflict_attachment_path(store, attachment);
            if let Some(parent) = conflict_path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::write(&conflict_path, body).map_err(|error| error.to_string())?;
            conflict_paths.push(conflict_path);
            continue;
        }

        fs::write(&destination_path, body).map_err(|error| error.to_string())?;
        imported_paths.push(destination_path);
    }

    let manifest_copy_path = store
        .root()
        .join("imports")
        .join("capsules")
        .join(format!("{}.json", manifest.capsule_id));
    if let Some(parent) = manifest_copy_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    write_pretty_json(&manifest_copy_path, &manifest)?;

    Ok(ImportedScenarioCapsule {
        manifest,
        imported_paths,
        deduped_paths,
        conflict_paths,
        manifest_copy_path,
    })
}

fn build_inventory_entry(
    relative_path: &str,
    artifact_kind: &str,
    logical_id: &str,
    content_hash: &str,
    body: &str,
) -> ScenarioCapsuleArtifactEntry {
    ScenarioCapsuleArtifactEntry {
        artifact_kind: artifact_kind.to_string(),
        logical_id: logical_id.to_string(),
        relative_path: relative_path.to_string(),
        content_hash: content_hash.to_string(),
        integrity_hash: stable_hash(&body),
    }
}

fn build_attachment_entry(
    attachment_kind: &str,
    attachment_ref: &str,
    relative_path: &str,
    content_hash: &str,
    body: &str,
) -> ScenarioCapsuleAttachmentEntry {
    ScenarioCapsuleAttachmentEntry {
        attachment_kind: attachment_kind.to_string(),
        attachment_ref: attachment_ref.to_string(),
        relative_path: relative_path.to_string(),
        content_hash: content_hash.to_string(),
        integrity_hash: stable_hash(&body),
    }
}

fn imported_artifact_path(
    store: &RetainedScenarioStore,
    artifact: &ScenarioCapsuleArtifactEntry,
) -> PathBuf {
    store
        .root()
        .join("imports")
        .join(import_dir_name(&artifact.artifact_kind))
        .join(format!("{}.json", artifact.logical_id))
}

fn conflict_artifact_path(
    store: &RetainedScenarioStore,
    artifact: &ScenarioCapsuleArtifactEntry,
) -> PathBuf {
    store
        .root()
        .join("imports")
        .join("conflicts")
        .join(import_dir_name(&artifact.artifact_kind))
        .join(format!(
            "{}--{}.json",
            artifact.logical_id, artifact.integrity_hash
        ))
}

fn imported_attachment_path(
    store: &RetainedScenarioStore,
    attachment: &ScenarioCapsuleAttachmentEntry,
) -> PathBuf {
    store
        .root()
        .join("imports")
        .join(import_attachment_dir_name(&attachment.attachment_kind))
        .join(format!("{}.replay.json", attachment.attachment_ref))
}

fn conflict_attachment_path(
    store: &RetainedScenarioStore,
    attachment: &ScenarioCapsuleAttachmentEntry,
) -> PathBuf {
    store
        .root()
        .join("imports")
        .join("conflicts")
        .join(import_attachment_dir_name(&attachment.attachment_kind))
        .join(format!(
            "{}--{}.replay.json",
            attachment.attachment_ref, attachment.integrity_hash
        ))
}

fn import_dir_name(artifact_kind: &str) -> &'static str {
    match artifact_kind {
        "scenario" => "scenarios",
        "scenario_run" => "scenario-runs",
        "capability_ledger_snapshot" => "capability-snapshots",
        "replay_capture" => "replay-captures",
        _ => "other-artifacts",
    }
}

fn import_attachment_dir_name(attachment_kind: &str) -> &'static str {
    match attachment_kind {
        "oxfml_replay_projection" => "replay-captures",
        _ => "attachments",
    }
}

fn replay_projection_path(store: &RetainedScenarioStore, replay_capture_id: &str) -> PathBuf {
    store
        .root()
        .join("replay-captures")
        .join(format!("{replay_capture_id}.replay.json"))
}

fn validate_artifact_identity(
    artifact: &ScenarioCapsuleArtifactEntry,
    body: &str,
) -> Result<(), String> {
    match artifact.artifact_kind.as_str() {
        "scenario" => {
            let record =
                serde_json::from_str::<ScenarioRecord>(body).map_err(|error| error.to_string())?;
            if record.scenario_id != artifact.logical_id
                || record.envelope.content_hash != artifact.content_hash
            {
                return Err(format!(
                    "scenario identity mismatch for {}",
                    artifact.logical_id
                ));
            }
        }
        "scenario_run" => {
            let record = serde_json::from_str::<ScenarioRunRecord>(body)
                .map_err(|error| error.to_string())?;
            if record.scenario_run_id != artifact.logical_id
                || record.envelope.content_hash != artifact.content_hash
            {
                return Err(format!(
                    "scenario run identity mismatch for {}",
                    artifact.logical_id
                ));
            }
        }
        "capability_ledger_snapshot" => {
            let record = serde_json::from_str::<CapabilityLedgerSnapshotRecord>(body)
                .map_err(|error| error.to_string())?;
            if record.capability_snapshot_id != artifact.logical_id
                || record.envelope.content_hash != artifact.content_hash
            {
                return Err(format!(
                    "capability snapshot identity mismatch for {}",
                    artifact.logical_id
                ));
            }
        }
        "replay_capture" => {
            let record = serde_json::from_str::<ReplayCaptureRecord>(body)
                .map_err(|error| error.to_string())?;
            if record.replay_capture_id != artifact.logical_id
                || record.envelope.content_hash != artifact.content_hash
            {
                return Err(format!(
                    "replay capture identity mismatch for {}",
                    artifact.logical_id
                ));
            }
        }
        _ => {
            return Err(format!(
                "unsupported ScenarioCapsule artifact kind: {}",
                artifact.artifact_kind
            ))
        }
    }
    Ok(())
}

fn write_pretty_json<T: Serialize>(path: impl AsRef<Path>, value: &T) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let body = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, body).map_err(|error| error.to_string())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, String> {
    let body = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&body).map_err(|error| error.to_string())
}
