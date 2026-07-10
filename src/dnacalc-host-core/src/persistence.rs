use std::path::{Path, PathBuf};

use dnacalc_skin_ir::{
    PersistedSkinStateRecord, SkinStatePersistenceError, SkinStatePersistenceKey,
    SkinStatePersistenceStore,
};

/// Native host adapter for persisting otherwise host-neutral skin state.
pub struct LocalFileSkinStatePersistenceStore {
    root: PathBuf,
}

impl LocalFileSkinStatePersistenceStore {
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn path_for(&self, key: &SkinStatePersistenceKey) -> PathBuf {
        self.root
            .join(safe_path_component(&key.skin_id))
            .join(key.slot.stable_id())
            .join(format!("{}.json", safe_path_component(&key.workspace_id)))
    }
}

impl SkinStatePersistenceStore for LocalFileSkinStatePersistenceStore {
    fn load(
        &self,
        key: &SkinStatePersistenceKey,
    ) -> Result<Option<PersistedSkinStateRecord>, SkinStatePersistenceError> {
        let path = self.path_for(key);
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .map(Some)
                .map_err(|error| SkinStatePersistenceError::Deserialize(error.to_string())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(SkinStatePersistenceError::Store {
                operation: "reading local skin state",
                detail: error.to_string(),
            }),
        }
    }

    fn save(
        &self,
        key: &SkinStatePersistenceKey,
        record: &PersistedSkinStateRecord,
    ) -> Result<(), SkinStatePersistenceError> {
        let path = self.path_for(key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| SkinStatePersistenceError::Store {
                operation: "creating local skin state directory",
                detail: error.to_string(),
            })?;
        }
        let text = serde_json::to_string_pretty(record)
            .map_err(|error| SkinStatePersistenceError::Serialize(error.to_string()))?;
        std::fs::write(path, text).map_err(|error| SkinStatePersistenceError::Store {
            operation: "writing local skin state",
            detail: error.to_string(),
        })
    }
}

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

#[cfg(test)]
mod tests {
    use dnacalc_skin_ir::{SkinId, SkinMountSlot};

    use super::*;

    #[test]
    fn roundtrips_records_and_sanitizes_path_components() {
        let root = std::env::temp_dir().join(format!("dnacalc-skin-state-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let store = LocalFileSkinStatePersistenceStore::new(&root);
        let key = SkinStatePersistenceKey::new(
            SkinId::new("persisted/test"),
            SkinMountSlot::RightInspector,
            "workspace:file",
        );
        let record = PersistedSkinStateRecord::new(7, serde_json::json!({ "ok": true }));

        store.save(&key, &record).expect("save record");
        assert_eq!(store.load(&key).expect("load record"), Some(record));
        assert!(root.join("persisted_test").exists());
        std::fs::remove_dir_all(root).expect("cleanup store");
    }
}
