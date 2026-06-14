use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

use dnatreecalc_skin_framework::NodeId;
use serde::{Deserialize, Serialize};

use super::preview::preview_accounts_workspace_session;
use super::session::{DnaTreeWorkspaceDocument, TreeWorkspaceSession};

const WORKSPACE_DOCUMENT_CATALOG_SCHEMA_VERSION: &str = "dnatreecalc-workspace-catalog-v1";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceDocumentCatalog {
    pub schema_version: String,
    pub workspace_ids: Vec<String>,
    pub active_workspace_id: Option<String>,
    /// Human display names per `workspace_id`. Optional and serde-defaulted, so
    /// `v1` catalogs written before rename existed load as an empty map; new
    /// code falls back to the `workspace_id` when an entry is absent.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub workspace_names: std::collections::BTreeMap<String, String>,
}

impl WorkspaceDocumentCatalog {
    #[must_use]
    pub fn new(workspace_ids: Vec<String>, active_workspace_id: Option<String>) -> Self {
        Self {
            schema_version: WORKSPACE_DOCUMENT_CATALOG_SCHEMA_VERSION.to_string(),
            workspace_ids,
            active_workspace_id,
            workspace_names: std::collections::BTreeMap::new(),
        }
    }

    /// Attach display names, keeping only entries for workspaces still present
    /// in `workspace_ids` so renames of deleted workspaces don't accumulate.
    #[must_use]
    pub fn with_workspace_names(
        mut self,
        workspace_names: &std::collections::BTreeMap<String, String>,
    ) -> Self {
        self.workspace_names = workspace_names
            .iter()
            .filter(|(workspace_id, _)| self.workspace_ids.contains(workspace_id))
            .map(|(workspace_id, name)| (workspace_id.clone(), name.clone()))
            .collect();
        self
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum WorkspaceDocumentStoreError {
    #[error("workspace document store failed while {operation}: {detail}")]
    Store {
        operation: &'static str,
        detail: String,
    },
    #[error("workspace document serialization failed: {0}")]
    Serialize(String),
    #[error("workspace document deserialization failed: {0}")]
    Deserialize(String),
    #[error("unsupported workspace document catalog schema {schema_version}")]
    UnsupportedCatalogSchema { schema_version: String },
    #[error("workspace document failed to open: {0}")]
    OpenDocument(String),
}

pub trait WorkspaceDocumentStore: Send + Sync {
    fn load_catalog(&self)
    -> Result<Option<WorkspaceDocumentCatalog>, WorkspaceDocumentStoreError>;

    fn save_catalog(
        &self,
        catalog: &WorkspaceDocumentCatalog,
    ) -> Result<(), WorkspaceDocumentStoreError>;

    fn load_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Option<DnaTreeWorkspaceDocument>, WorkspaceDocumentStoreError>;

    fn save_workspace(
        &self,
        workspace_id: &str,
        document: &DnaTreeWorkspaceDocument,
    ) -> Result<(), WorkspaceDocumentStoreError>;
}

#[derive(Default)]
pub struct InMemoryWorkspaceDocumentStore {
    catalog: std::sync::Mutex<Option<WorkspaceDocumentCatalog>>,
    documents: std::sync::Mutex<std::collections::BTreeMap<String, DnaTreeWorkspaceDocument>>,
}

impl InMemoryWorkspaceDocumentStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl WorkspaceDocumentStore for InMemoryWorkspaceDocumentStore {
    fn load_catalog(
        &self,
    ) -> Result<Option<WorkspaceDocumentCatalog>, WorkspaceDocumentStoreError> {
        Ok(self
            .catalog
            .lock()
            .map_err(|_| WorkspaceDocumentStoreError::Store {
                operation: "locking in-memory workspace catalog",
                detail: "mutex poisoned".to_string(),
            })?
            .clone())
    }

    fn save_catalog(
        &self,
        catalog: &WorkspaceDocumentCatalog,
    ) -> Result<(), WorkspaceDocumentStoreError> {
        *self
            .catalog
            .lock()
            .map_err(|_| WorkspaceDocumentStoreError::Store {
                operation: "locking in-memory workspace catalog",
                detail: "mutex poisoned".to_string(),
            })? = Some(catalog.clone());
        Ok(())
    }

    fn load_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Option<DnaTreeWorkspaceDocument>, WorkspaceDocumentStoreError> {
        Ok(self
            .documents
            .lock()
            .map_err(|_| WorkspaceDocumentStoreError::Store {
                operation: "locking in-memory workspace documents",
                detail: "mutex poisoned".to_string(),
            })?
            .get(workspace_id)
            .cloned())
    }

    fn save_workspace(
        &self,
        workspace_id: &str,
        document: &DnaTreeWorkspaceDocument,
    ) -> Result<(), WorkspaceDocumentStoreError> {
        self.documents
            .lock()
            .map_err(|_| WorkspaceDocumentStoreError::Store {
                operation: "locking in-memory workspace documents",
                detail: "mutex poisoned".to_string(),
            })?
            .insert(workspace_id.to_string(), document.clone());
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct LocalFileWorkspaceDocumentStore {
    root: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl LocalFileWorkspaceDocumentStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn catalog_path(&self) -> PathBuf {
        self.root.join("catalog.json")
    }

    fn workspace_path(&self, workspace_id: &str) -> PathBuf {
        self.root.join("workspaces").join(format!(
            "{}.dnatree.json",
            safe_path_component(workspace_id)
        ))
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl WorkspaceDocumentStore for LocalFileWorkspaceDocumentStore {
    fn load_catalog(
        &self,
    ) -> Result<Option<WorkspaceDocumentCatalog>, WorkspaceDocumentStoreError> {
        let path = self.catalog_path();
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                let catalog: WorkspaceDocumentCatalog = serde_json::from_str(&text)
                    .map_err(|error| WorkspaceDocumentStoreError::Deserialize(error.to_string()))?;
                if catalog.schema_version != WORKSPACE_DOCUMENT_CATALOG_SCHEMA_VERSION {
                    return Err(WorkspaceDocumentStoreError::UnsupportedCatalogSchema {
                        schema_version: catalog.schema_version,
                    });
                }
                Ok(Some(catalog))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(WorkspaceDocumentStoreError::Store {
                operation: "reading local workspace catalog",
                detail: error.to_string(),
            }),
        }
    }

    fn save_catalog(
        &self,
        catalog: &WorkspaceDocumentCatalog,
    ) -> Result<(), WorkspaceDocumentStoreError> {
        std::fs::create_dir_all(&self.root).map_err(|error| {
            WorkspaceDocumentStoreError::Store {
                operation: "creating local workspace catalog directory",
                detail: error.to_string(),
            }
        })?;
        let text = serde_json::to_string_pretty(catalog)
            .map_err(|error| WorkspaceDocumentStoreError::Serialize(error.to_string()))?;
        std::fs::write(self.catalog_path(), text).map_err(|error| {
            WorkspaceDocumentStoreError::Store {
                operation: "writing local workspace catalog",
                detail: error.to_string(),
            }
        })
    }

    fn load_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Option<DnaTreeWorkspaceDocument>, WorkspaceDocumentStoreError> {
        let path = self.workspace_path(workspace_id);
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .map(Some)
                .map_err(|error| WorkspaceDocumentStoreError::Deserialize(error.to_string())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(WorkspaceDocumentStoreError::Store {
                operation: "reading local workspace document",
                detail: error.to_string(),
            }),
        }
    }

    fn save_workspace(
        &self,
        workspace_id: &str,
        document: &DnaTreeWorkspaceDocument,
    ) -> Result<(), WorkspaceDocumentStoreError> {
        let path = self.workspace_path(workspace_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                WorkspaceDocumentStoreError::Store {
                    operation: "creating local workspace document directory",
                    detail: error.to_string(),
                }
            })?;
        }
        let text = serde_json::to_string_pretty(document)
            .map_err(|error| WorkspaceDocumentStoreError::Serialize(error.to_string()))?;
        std::fs::write(path, text).map_err(|error| WorkspaceDocumentStoreError::Store {
            operation: "writing local workspace document",
            detail: error.to_string(),
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn safe_path_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

pub fn workspace_session_from_document_store_or_default(
    store: &dyn WorkspaceDocumentStore,
) -> Result<(TreeWorkspaceSession, Option<NodeId>), WorkspaceDocumentStoreError> {
    let Some(catalog) = store.load_catalog()? else {
        return Ok((preview_accounts_workspace_session(), None));
    };
    if catalog.schema_version != WORKSPACE_DOCUMENT_CATALOG_SCHEMA_VERSION {
        return Err(WorkspaceDocumentStoreError::UnsupportedCatalogSchema {
            schema_version: catalog.schema_version,
        });
    }
    let mut candidates = Vec::new();
    if let Some(active) = &catalog.active_workspace_id {
        candidates.push(active.clone());
    }
    candidates.extend(
        catalog
            .workspace_ids
            .iter()
            .filter(|workspace_id| Some(*workspace_id) != catalog.active_workspace_id.as_ref())
            .cloned(),
    );
    for workspace_id in candidates {
        let Some(document) = store.load_workspace(&workspace_id)? else {
            continue;
        };
        return TreeWorkspaceSession::from_dnatree_document(document)
            .map_err(|error| WorkspaceDocumentStoreError::OpenDocument(error.to_string()));
    }
    Ok((preview_accounts_workspace_session(), None))
}

pub(crate) fn persist_workspace_sessions(
    store: &Arc<dyn WorkspaceDocumentStore>,
    sessions: &std::collections::BTreeMap<String, u64>,
    active_workspace_id: &str,
    selected_node: Option<&NodeId>,
    workspace_names: &std::collections::BTreeMap<String, String>,
) -> Result<(), WorkspaceDocumentStoreError> {
    let workspace_ids = sessions.keys().cloned().collect::<Vec<_>>();
    store.save_catalog(
        &WorkspaceDocumentCatalog::new(workspace_ids, Some(active_workspace_id.to_string()))
            .with_workspace_names(workspace_names),
    )?;
    super::dispatcher::with_host_sessions(|host_sessions| {
        for (workspace_id, session_id) in sessions {
            let Some(session) = host_sessions.get(session_id).cloned() else {
                continue;
            };
            let document = session
                .lock()
                .map_err(|_| WorkspaceDocumentStoreError::Store {
                    operation: "locking workspace session for persistence",
                    detail: "mutex poisoned".to_string(),
                })?
                .export_dnatree_document(
                    (workspace_id == active_workspace_id)
                        .then_some(selected_node)
                        .flatten(),
                )
                .map_err(|error| WorkspaceDocumentStoreError::OpenDocument(error.to_string()))?;
            store.save_workspace(workspace_id, &document)?;
        }
        Ok(())
    })
}
