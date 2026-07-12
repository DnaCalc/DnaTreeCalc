use std::collections::{BTreeMap, BTreeSet};

use oxfml_core::{
    LibraryAvailabilityState, LibraryContextSnapshot, LibraryContextSnapshotEntry,
    RegistrationSourceKind,
};

const SNAPSHOT_EXPORT: &str = include_str!(
    "../../../../OxFunc/docs/function-lane/OXFUNC_LIBRARY_CONTEXT_SNAPSHOT_EXPORT_V1.csv"
);
const W50_INVENTORY: &str = include_str!(
    "../../../../OxFunc/docs/function-lane/W50_DEFERRED_CURRENT_VERSION_INVENTORY.csv"
);
const W51_INVENTORY: &str = include_str!(
    "../../../../OxFunc/docs/function-lane/W51_IN_SCOPE_CURRENT_VERSION_NOT_COMPLETE_INVENTORY.csv"
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdmissionCategory {
    Supported,
    Preview,
    Experimental,
    Deferred,
    CatalogOnly,
}

impl AdmissionCategory {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Preview => "preview",
            Self::Experimental => "experimental",
            Self::Deferred => "deferred",
            Self::CatalogOnly => "catalog_only",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSurfaceEntry {
    pub canonical_surface_name: String,
    pub surface_stable_id: String,
    pub name_resolution_table_ref: Option<String>,
    pub semantic_trait_profile_ref: Option<String>,
    pub gating_profile_ref: Option<String>,
    pub category: String,
    pub metadata_status: String,
    pub special_interface_kind: String,
    pub admission_interface_kind: String,
    pub preparation_owner: Option<String>,
    pub runtime_boundary_kind: Option<String>,
    pub interface_contract_ref: Option<String>,
    pub registration_source_kind: RegistrationSourceKind,
    pub admission_category: AdmissionCategory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceLabelSummary {
    pub supported: usize,
    pub preview: usize,
    pub experimental: usize,
    pub deferred: usize,
    pub catalog_only: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionSurfaceCatalog {
    snapshot_id: String,
    snapshot_version: String,
    entries: BTreeMap<String, FunctionSurfaceEntry>,
}

impl FunctionSurfaceCatalog {
    pub fn load_current() -> Self {
        let deferred = load_deferred_inventory();
        let w51 = load_w51_inventory();

        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(SNAPSHOT_EXPORT.as_bytes());

        let mut snapshot_id = None;
        let mut snapshot_version = None;
        let mut entries = BTreeMap::new();
        for row in reader.deserialize::<SnapshotRow>() {
            let row = row.expect("snapshot export rows should deserialize");
            snapshot_id.get_or_insert_with(|| row.snapshot_id.clone());
            snapshot_version.get_or_insert_with(|| row.snapshot_generation.clone());
            let admission_category = if deferred.contains(&row.canonical_surface_name) {
                AdmissionCategory::Deferred
            } else if let Some(notes) = w51.get(&row.canonical_surface_name) {
                if notes_indicate_real_runtime(notes) {
                    AdmissionCategory::Preview
                } else {
                    AdmissionCategory::Experimental
                }
            } else if row.metadata_status == "catalog_only" {
                AdmissionCategory::CatalogOnly
            } else {
                AdmissionCategory::Supported
            };

            entries.insert(
                row.canonical_surface_name.clone(),
                FunctionSurfaceEntry {
                    canonical_surface_name: row.canonical_surface_name,
                    surface_stable_id: row.surface_stable_id,
                    name_resolution_table_ref: some_if_non_empty(&row.name_resolution_table_ref),
                    semantic_trait_profile_ref: some_if_non_empty(&row.semantic_trait_profile_ref),
                    gating_profile_ref: some_if_non_empty(&row.gating_profile_ref),
                    category: row.category,
                    metadata_status: row.metadata_status,
                    special_interface_kind: row.special_interface_kind,
                    admission_interface_kind: row.admission_interface_kind,
                    preparation_owner: some_if_non_empty(&row.preparation_owner),
                    runtime_boundary_kind: some_if_non_empty(&row.runtime_boundary_kind),
                    interface_contract_ref: some_if_non_empty(&row.interface_contract_ref),
                    registration_source_kind: parse_registration_source_kind(
                        &row.registration_source_kind,
                    ),
                    admission_category,
                },
            );
        }

        Self {
            snapshot_id: snapshot_id.expect("snapshot export should have at least one row"),
            snapshot_version: snapshot_version
                .expect("snapshot export should have at least one row"),
            entries,
        }
    }

    pub fn get(&self, canonical_surface_name: &str) -> Option<&FunctionSurfaceEntry> {
        self.entries.get(canonical_surface_name)
    }

    pub fn label_summary(&self) -> SurfaceLabelSummary {
        let mut summary = SurfaceLabelSummary {
            supported: 0,
            preview: 0,
            experimental: 0,
            deferred: 0,
            catalog_only: 0,
        };

        for entry in self.entries.values() {
            match entry.admission_category {
                AdmissionCategory::Supported => summary.supported += 1,
                AdmissionCategory::Preview => summary.preview += 1,
                AdmissionCategory::Experimental => summary.experimental += 1,
                AdmissionCategory::Deferred => summary.deferred += 1,
                AdmissionCategory::CatalogOnly => summary.catalog_only += 1,
            }
        }

        summary
    }

    pub fn admitted_execution_snapshot(&self) -> LibraryContextSnapshot {
        LibraryContextSnapshot {
            snapshot_id: self.snapshot_id.clone(),
            snapshot_version: self.snapshot_version.clone(),
            entries: self
                .entries
                .values()
                .filter(|entry| entry.is_execution_admitted())
                .map(FunctionSurfaceEntry::to_snapshot_entry)
                .collect(),
        }
    }
}

impl FunctionSurfaceEntry {
    pub const fn is_execution_admitted(&self) -> bool {
        matches!(
            self.admission_category,
            AdmissionCategory::Supported | AdmissionCategory::Preview
        )
    }

    fn to_snapshot_entry(&self) -> LibraryContextSnapshotEntry {
        LibraryContextSnapshotEntry {
            surface_name: self.canonical_surface_name.clone(),
            canonical_id: Some(self.surface_stable_id.clone()),
            surface_stable_id: Some(self.surface_stable_id.clone()),
            name_resolution_table_ref: self.name_resolution_table_ref.clone(),
            semantic_trait_profile_ref: self.semantic_trait_profile_ref.clone(),
            gating_profile_ref: self.gating_profile_ref.clone(),
            metadata_status: Some(self.metadata_status.clone()),
            special_interface_kind: Some(self.special_interface_kind.clone()),
            admission_interface_kind: Some(self.admission_interface_kind.clone()),
            preparation_owner: self.preparation_owner.clone(),
            runtime_boundary_kind: self.runtime_boundary_kind.clone(),
            arity_shape_note: None,
            interface_contract_ref: self.interface_contract_ref.clone(),
            registration_source_kind: self.registration_source_kind,
            parse_bind_state: LibraryAvailabilityState::CatalogKnown,
            semantic_plan_state: LibraryAvailabilityState::CatalogKnown,
            runtime_capability_state: Some(LibraryAvailabilityState::CatalogKnown),
            post_dispatch_state: Some(LibraryAvailabilityState::CatalogKnown),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct SnapshotRow {
    snapshot_id: String,
    snapshot_generation: String,
    registration_source_kind: String,
    surface_stable_id: String,
    canonical_surface_name: String,
    name_resolution_table_ref: String,
    semantic_trait_profile_ref: String,
    gating_profile_ref: String,
    category: String,
    metadata_status: String,
    special_interface_kind: String,
    admission_interface_kind: String,
    preparation_owner: String,
    runtime_boundary_kind: String,
    interface_contract_ref: String,
}

#[derive(Debug, serde::Deserialize)]
struct W50Row {
    entry_name: String,
}

#[derive(Debug, serde::Deserialize)]
struct W51Row {
    entry_name: String,
    notes: String,
}

fn load_deferred_inventory() -> BTreeSet<String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(W50_INVENTORY.as_bytes());

    reader
        .deserialize::<W50Row>()
        .map(|row| {
            row.expect("W50 inventory rows should deserialize")
                .entry_name
        })
        .collect()
}

fn load_w51_inventory() -> BTreeMap<String, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(W51_INVENTORY.as_bytes());

    reader
        .deserialize::<W51Row>()
        .map(|row| {
            let row = row.expect("W51 inventory rows should deserialize");
            (row.entry_name, row.notes)
        })
        .collect()
}

fn notes_indicate_real_runtime(notes: &str) -> bool {
    let notes = notes.to_ascii_lowercase();
    notes.contains("runtime/formal/evidence slice is real")
        || notes.contains("runtime now has")
        || notes.contains("coverage is now real")
        || notes.contains("coverage are real")
        || notes.contains("typed request normalization")
}

fn some_if_non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn parse_registration_source_kind(value: &str) -> RegistrationSourceKind {
    match value {
        "built_in_catalog_function" | "built_in_operator_export" | "doc_modeled_operator" => {
            RegistrationSourceKind::BuiltIn
        }
        other => panic!("unsupported registration source kind {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_catalog_derives_labels_from_snapshot_and_overlays() {
        let catalog = FunctionSurfaceCatalog::load_current();

        assert_eq!(
            catalog.get("ABS").map(|entry| entry.admission_category),
            Some(AdmissionCategory::Supported)
        );
        assert_eq!(
            catalog
                .get("ENCODEURL")
                .map(|entry| entry.admission_category),
            Some(AdmissionCategory::Deferred)
        );
        assert!(catalog.label_summary().supported > 0);
    }

    #[test]
    fn w51_note_without_real_runtime_phrase_is_experimental() {
        assert!(!notes_indicate_real_runtime(
            "Remaining work is open boundary characterization only."
        ));
    }
}
