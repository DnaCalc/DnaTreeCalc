use std::collections::BTreeSet;

use crate::state::{
    CapabilityLedgerSnapshot, ModeAvailabilityFact, OneCalcHostState, OxFuncMetadataSnapshot,
};

pub fn build_capability_ledger_snapshot(state: &OneCalcHostState) -> CapabilityLedgerSnapshot {
    let mut snapshot = state.capability_and_environment.current_snapshot.clone();
    snapshot.oxfunc_metadata = build_oxfunc_metadata_snapshot();
    snapshot.rich_value_capabilities = snapshot
        .oxfunc_metadata
        .producer_capability_vocab
        .iter()
        .filter(|key| {
            key.starts_with("Indexable(")
                || key.starts_with("Shaped(")
                || key.starts_with("Materialisable(")
                || key.starts_with("Enumerable(")
        })
        .cloned()
        .collect();
    snapshot.mode_availability = vec![
        ModeAvailabilityFact::available("DNA-only"),
        ModeAvailabilityFact::available("Replay"),
        ModeAvailabilityFact::blocked("Excel-observed", "requires OxXlPlay live capture"),
        ModeAvailabilityFact::blocked("Twin compare", "requires retained Excel observation"),
        ModeAvailabilityFact::blocked("RTD", "desktop extension host not active"),
    ];
    snapshot.blocked_modes = snapshot
        .mode_availability
        .iter()
        .filter_map(|fact| {
            (!fact.available).then(|| crate::state::BlockedModeFact {
                mode: fact.mode.clone(),
                reason: fact.reason.clone().unwrap_or_default(),
            })
        })
        .collect();
    snapshot
}

pub fn build_oxfunc_metadata_snapshot() -> OxFuncMetadataSnapshot {
    let registry = oxfunc_core::registry::builtin_registry();
    let mut semantic_versions = BTreeSet::new();
    let mut arg_versions = BTreeSet::new();
    let mut producer_keys = BTreeSet::new();

    for entry in registry.iter() {
        semantic_versions.insert(entry.meta.semantic_kernel_metadata_version.clone());
        arg_versions.insert(entry.meta.arg_admission_metadata_version.clone());
        for key in &entry.meta.producer_capability_set_keys {
            producer_keys.insert(key.clone());
        }
    }

    OxFuncMetadataSnapshot {
        semantic_kernel_metadata_versions: semantic_versions.into_iter().collect(),
        arg_admission_metadata_versions: arg_versions.into_iter().collect(),
        producer_capability_vocab: producer_keys.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_snapshot_includes_oxfunc_metadata_versions_and_webimage_capabilities() {
        let snapshot = build_capability_ledger_snapshot(&OneCalcHostState::default());

        assert!(snapshot
            .oxfunc_metadata
            .semantic_kernel_metadata_versions
            .iter()
            .any(|version| version.contains("semantic_kernel_metadata.v1")));
        assert!(snapshot
            .oxfunc_metadata
            .arg_admission_metadata_versions
            .iter()
            .any(|version| version.contains("arg_admission_metadata.v1")));
        assert!(snapshot
            .rich_value_capabilities
            .iter()
            .any(|key| key.starts_with("Materialisable(")));
    }
}
