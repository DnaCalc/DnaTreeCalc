use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::document::{read_spreadsheetml_document, OneCalcDocumentRecord};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDocumentEntry {
    pub document_id: String,
    pub document_path: String,
    pub formula_stable_id: String,
    pub scenario_slug: String,
    pub host_profile_id: String,
    pub governing_capability_snapshot_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneCalcWorkspaceManifest {
    pub workspace_id: String,
    pub workspace_name: String,
    pub active_document_id: String,
    pub document_entries: Vec<WorkspaceDocumentEntry>,
    pub omitted_scope_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedOneCalcWorkspace {
    pub manifest: OneCalcWorkspaceManifest,
    pub manifest_path: PathBuf,
}

pub fn write_workspace_manifest(
    manifest_path: impl AsRef<Path>,
    workspace_name: impl Into<String>,
    document_paths: &[impl AsRef<Path>],
) -> Result<PersistedOneCalcWorkspace, String> {
    if document_paths.is_empty() {
        return Err("workspace manifest requires at least one document".to_string());
    }

    let manifest_path = manifest_path.as_ref();
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let documents = document_paths
        .iter()
        .map(|path| {
            let path = path.as_ref().to_path_buf();
            let document = read_spreadsheetml_document(&path)?;
            Ok::<_, String>((path, document))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let document_entries = documents
        .iter()
        .map(|(path, document)| workspace_document_entry(path, document))
        .collect::<Vec<_>>();
    let workspace_name = workspace_name.into();
    let workspace_id = format!(
        "workspace-{}-{}",
        sanitize_slug(&workspace_name),
        document_entries.len()
    );
    let active_document_id = document_entries[0].document_id.clone();
    let manifest = OneCalcWorkspaceManifest {
        workspace_id,
        workspace_name,
        active_document_id,
        document_entries,
        omitted_scope_notes: vec![
            "workspace management groups isolated documents without introducing workbook semantics"
                .to_string(),
            "cross-instance recalc, shared dependency graphs, and workbook-global precedence remain out of scope"
                .to_string(),
        ],
    };

    let body = serde_json::to_string_pretty(&manifest).map_err(|error| error.to_string())?;
    fs::write(manifest_path, body).map_err(|error| error.to_string())?;

    Ok(PersistedOneCalcWorkspace {
        manifest,
        manifest_path: manifest_path.to_path_buf(),
    })
}

pub fn read_workspace_manifest(
    manifest_path: impl AsRef<Path>,
) -> Result<OneCalcWorkspaceManifest, String> {
    let body = fs::read_to_string(manifest_path).map_err(|error| error.to_string())?;
    serde_json::from_str(&body).map_err(|error| error.to_string())
}

fn workspace_document_entry(
    path: &Path,
    document: &OneCalcDocumentRecord,
) -> WorkspaceDocumentEntry {
    WorkspaceDocumentEntry {
        document_id: document.document_id.clone(),
        document_path: path.display().to_string(),
        formula_stable_id: document.formula_stable_id.clone(),
        scenario_slug: document.scenario_slug.clone(),
        host_profile_id: document.host_profile_id.clone(),
        governing_capability_snapshot_id: document.governing_capability_snapshot_id.clone(),
    }
}

fn sanitize_slug(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
