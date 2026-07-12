pub mod artifact;
pub mod capsule;
pub mod conditional_formatting;
pub mod document;
pub mod extension;
pub mod function_surface;
pub mod observation;
pub mod retained;
pub mod runtime;
pub mod shell;
#[cfg(test)]
pub(crate) mod test_support;
pub mod workspace;

use oxfml_core::{parse_formula, FormulaChannelKind, FormulaSourceRecord, ParseRequest};
use oxfunc_core::functions::sum::eval_sum_surface;
use oxfunc_core::resolver::{RefResolutionError, ReferenceResolver, ResolverCapabilities};
use oxfunc_core::value::{CallArgValue, EvalValue, ReferenceLike};
use oxreplay_abstractions::{LaneId, RegistryRef};
use oxreplay_core::{is_replay_ready, ReplayEvent, ReplayScenario};

pub use artifact::{
    stable_hash, ArtifactAttachmentRef, ArtifactEnvelope, ArtifactKind, ArtifactLineageRef,
    StableArtifactRef,
};
pub use capsule::{
    ImportedScenarioCapsule, PersistedScenarioCapsule, ScenarioCapsuleArtifactEntry,
    ScenarioCapsuleAttachmentEntry, ScenarioCapsuleManifest,
};
pub use conditional_formatting::{
    validate_isolated_conditional_formatting_carrier, ConditionalFormattingCarrierSummary,
    IsolatedConditionalFormattingCarrier,
};
pub use document::{
    read_spreadsheetml_document, write_spreadsheetml_document, DocumentArtifactIndexEntry,
    DocumentViewStateRecord, OneCalcDocumentRecord, PersistedOneCalcDocument,
};
pub use extension::{
    activate_windows_rtd_topic, admitted_extension_abi, advance_rtd_topic,
    extension_root_runtime_truth, invoke_extension_provider, load_extension_root,
    validate_extension_manifest, ActivatedRtdTopicSession, ExtensionAbiContract,
    ExtensionCapabilityTruth, ExtensionInvocationArgument, ExtensionInvocationSummary,
    ExtensionManifestLoadFailure, ExtensionProviderEntrypoint, ExtensionProviderManifest,
    ExtensionProviderRuntimeTruth, ExtensionRootLoadSummary, ExtensionRootRuntimeTruthSummary,
    ExtensionValidationResult, LinuxRtdRegistryEntry, LinuxRtdRegistrySummary,
    LoadedExtensionProvider, RegisteredExtensionBehavior, RegisteredExtensionFunction,
    RegisteredRtdTopic, RtdTopicUpdateSummary,
};
pub use function_surface::{
    AdmissionCategory, FunctionSurfaceCatalog, FunctionSurfaceEntry, SurfaceLabelSummary,
};
pub use observation::{
    invoke_live_windows_capture, load_observation_source_bundle, LoadedObservationSourceBundle,
    ObservationBridgePayload, ObservationCapturePayload, ObservationInterpretation,
    ObservationProvenancePayload, ObservationSurfaceDescriptor, ObservationSurfaceValue,
};
pub use retained::{
    CapabilityLedgerSnapshotRecord, CapabilityModeAvailabilityRecord, ComparisonMismatchRecord,
    ComparisonRecord, HandoffPacketRecord, HandoffReadinessRecord, ObservationRecord,
    PersistedCapabilitySnapshot, PersistedComparison, PersistedHandoffPacket, PersistedObservation,
    PersistedReplayCapture, PersistedScenarioRun, PersistedWitness, ReopenedScenarioRun,
    ReplayCaptureRecord, RetainedProvenanceRecord, RetainedRecalcContextRecord,
    RetainedScenarioStore, ScenarioRecord, ScenarioRunRecord, WitnessRecord,
};
pub use runtime::{
    AcceptanceMatrix, AcceptanceMatrixRow, ArrayPreviewSummary, CapabilitySnapshotDiffSummary,
    CompletionProposalSummary, DocumentRoundTripInvariantReport, DrivenRecalcSummary,
    DrivenRunComparison, DrivenSingleFormulaHost, FormulaEditPacketSummary, FormulaEditorSession,
    FormulaEvaluationSummary, FunctionHelpSummary, HostPacketKind, OneCalcHostProfile,
    OpenedCapabilitySnapshotSummary, OpenedHandoffPacketSummary, OpenedOneCalcWorkspace,
    OpenedReplayCaptureSummary, OpenedTwinCompareSummary, OpenedWitnessSummary, OpenedXRaySummary,
    PlatformGate, PromotedScenarioIndex, PromotedScenarioIndexRow,
    PromotedScenarioRegressionSummary, RecalcContext, RecalcTriggerKind,
    ReopenedDrivenSingleFormulaRun, ReopenedOneCalcDocument, ReplayAwareOperationKind,
    ReplayAwareOperationSummary, RetainedRunDiffSummary, RetainedRunReopenInvariantReport,
    SemanticLoggingBoundarySummary,
    RetainedRunXRaySummary, RuntimeAdapter, ScenarioLibraryFilter, ScenarioLibrarySavedView,
    ScenarioLineageRef, ScenarioSelectionAction, ScenarioSelectionDetail, UpstreamPressurePacket,
    XRayBindSummary, XRayEvalSummary, XRayParseSummary, XRayProvenanceSummary, XRayTraceSummary,
};
pub use shell::{launch_shell, launch_shell_with_formula, OneCalcShellApp};
pub use workspace::{
    read_workspace_manifest, write_workspace_manifest, OneCalcWorkspaceManifest,
    PersistedOneCalcWorkspace, WorkspaceDocumentEntry,
};

#[derive(Debug, Clone, PartialEq)]
pub struct DependencyProbeReport {
    pub formula_token: String,
    pub parse_token_count: usize,
    pub parse_diagnostic_count: usize,
    pub sum_result: f64,
    pub replay_ready: bool,
    pub replay_registry_ref_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyProbeError {
    SumDidNotReturnNumber,
}

struct NoReferenceResolver;

impl ReferenceResolver for NoReferenceResolver {
    fn capabilities(&self) -> ResolverCapabilities {
        ResolverCapabilities::permissive_local()
    }

    fn resolve_reference(
        &self,
        reference: &ReferenceLike,
    ) -> Result<EvalValue, RefResolutionError> {
        Err(RefResolutionError::UnresolvedReference {
            target: reference.target.clone(),
        })
    }
}

pub fn run_dependency_probe() -> Result<DependencyProbeReport, DependencyProbeError> {
    // This is a bounded seam probe, not ordinary host execution.
    let source = FormulaSourceRecord::new("onecalc.probe", 1, "=SUM(1,2,3)")
        .with_formula_channel_kind(FormulaChannelKind::WorksheetA1);
    let formula_token = source.formula_token().0;

    let parse = parse_formula(ParseRequest { source });
    let parse_token_count = parse.green_tree.full_fidelity_tokens.len();
    let parse_diagnostic_count = parse.green_tree.diagnostics.len();

    let args = [
        CallArgValue::Eval(EvalValue::Number(1.0)),
        CallArgValue::Eval(EvalValue::Number(2.0)),
        CallArgValue::Eval(EvalValue::Number(3.0)),
    ];
    let sum_result = match eval_sum_surface(&args, &NoReferenceResolver) {
        Ok(EvalValue::Number(number)) => number,
        Ok(_) | Err(_) => return Err(DependencyProbeError::SumDidNotReturnNumber),
    };

    let replay = ReplayScenario {
        scenario_id: "onecalc.probe.sum".to_string(),
        lane_id: LaneId("onecalc".to_string()),
        events: vec![ReplayEvent {
            event_id: "event-001".to_string(),
            source_label: "sum_probe".to_string(),
            normalized_family: "evaluation.sum".to_string(),
        }],
        source_metadata: None,
        registry_refs: vec![RegistryRef {
            family: "probe".to_string(),
            version: "v1".to_string(),
        }],
    };

    Ok(DependencyProbeReport {
        formula_token,
        parse_token_count,
        parse_diagnostic_count,
        sum_result,
        replay_ready: is_replay_ready(&replay),
        replay_registry_ref_count: replay.registry_refs.len(),
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::test_support::{
        adapter_for, persist_observation_fixture, promoted_formatting_scenarios,
        promoted_formula_scenario, promoted_formula_scenarios, promoted_observation_scenario,
        promoted_observation_scenarios, DrivenHostFixture, FixtureRoot, FormulaScenarioFamily,
        ObservationScenarioFamily,
    };

    fn retained_xray_golden_lines(xray: &RetainedRunXRaySummary) -> Vec<String> {
        vec![
            format!(
                "packet_kind={}",
                xray.provenance.latest_host_driving_packet_kind
            ),
            format!(
                "worksheet_value={}",
                xray.evaluation
                    .as_ref()
                    .map(|value| value.worksheet_value_summary.as_str())
                    .unwrap_or("none")
            ),
            format!(
                "payload={}",
                xray.evaluation
                    .as_ref()
                    .map(|value| value.payload_summary.as_str())
                    .unwrap_or("none")
            ),
            format!(
                "effective_display={}",
                xray.evaluation
                    .as_ref()
                    .map(|value| value.effective_display_status.as_str())
                    .unwrap_or("none")
            ),
            format!(
                "formatting_truth_plane={}",
                xray.provenance.formatting_truth_plane
            ),
            format!(
                "conditional_formatting_scope={}",
                xray.provenance.conditional_formatting_scope
            ),
            format!(
                "blocked_dimensions={}",
                xray.provenance.blocked_dimensions.join("|")
            ),
            format!(
                "replay_floor={}",
                xray.trace
                    .as_ref()
                    .and_then(|value| value.replay_floor.as_deref())
                    .unwrap_or("none")
            ),
            format!(
                "replay_projection_family={}",
                xray.trace
                    .as_ref()
                    .and_then(|value| value.replay_projection_source_artifact_family.as_deref())
                    .unwrap_or("none")
            ),
            format!(
                "replay_projection_phase={}",
                xray.trace
                    .as_ref()
                    .and_then(|value| value.replay_projection_phase.as_deref())
                    .unwrap_or("none")
            ),
        ]
    }

    fn capability_snapshot_golden_lines(
        snapshot: &OpenedCapabilitySnapshotSummary,
        diff: &CapabilitySnapshotDiffSummary,
    ) -> Vec<String> {
        vec![
            format!("host_kind={}", snapshot.host_kind),
            format!("runtime_class={}", snapshot.runtime_class),
            format!("capability_floor={}", snapshot.capability_floor),
            format!(
                "packet_kind_register={}",
                snapshot.packet_kind_register.join("|")
            ),
            format!(
                "mode_availability={}",
                snapshot
                    .mode_availability
                    .iter()
                    .map(|mode| format!("{}:{}", mode.mode_id, mode.state))
                    .collect::<Vec<_>>()
                    .join("|")
            ),
            format!(
                "function_surface_policy={}",
                snapshot.function_surface_policy_id
            ),
            format!(
                "diff_base={}",
                snapshot.diff_base_snapshot_id.as_deref().unwrap_or("none")
            ),
            format!("diff_floor={}", diff.diff_floor),
            format!("diff_mode_changes={}", diff.mode_changes.join("|")),
        ]
    }

    fn document_roundtrip_golden_lines(
        document: &PersistedOneCalcDocument,
        invariants: &DocumentRoundTripInvariantReport,
    ) -> Vec<String> {
        vec![
            format!("scope={}", document.document.document_scope),
            format!("format={}", document.document.persistence_format_id),
            format!("host_profile={}", document.document.host_profile_id),
            format!("scenario_slug={}", document.document.scenario_slug),
            format!("host_session_id={}", document.document.host_session_id),
            format!(
                "host_recalc_sequence={}",
                document.document.host_recalc_sequence
            ),
            format!(
                "governing_capability_snapshot={}",
                document
                    .document
                    .governing_capability_snapshot_id
                    .as_deref()
                    .unwrap_or("none")
            ),
            format!("packet_kind={}", document.document.host_driving_packet_kind),
            format!("effective_display={}", document.document.effective_display_status),
            format!("artifact_index_count={}", document.document.artifact_index.len()),
            format!(
                "artifact_kinds={}",
                document
                    .document
                    .artifact_index
                    .iter()
                    .map(|entry| entry.artifact_kind.as_str())
                    .collect::<Vec<_>>()
                    .join("|")
            ),
            format!(
                "invariants=document_id:{};formula_identity:{};structure_context:{};session_state:{};library_context_snapshot_ref:{};governing_capability_snapshot:{};artifact_index:{};effective_display_status:{}",
                invariants.document_id_preserved,
                invariants.formula_identity_preserved,
                invariants.structure_context_preserved,
                invariants.session_state_preserved,
                invariants.library_context_snapshot_ref_preserved,
                invariants.governing_capability_snapshot_preserved,
                invariants.artifact_index_preserved,
                invariants.effective_display_status_preserved
            ),
        ]
    }

    fn twin_compare_golden_lines(compare: &OpenedTwinCompareSummary) -> Vec<String> {
        vec![
            format!("reliability={}", compare.reliability_badge),
            format!("envelope={}", compare.comparison_envelope.join("|")),
            format!("mismatches={}", compare.mismatch_lines.join("|")),
            format!(
                "projection_limitations={}",
                compare.projection_limitations.join("|")
            ),
        ]
    }

    fn replay_operation_golden_lines(operation: &ReplayAwareOperationSummary) -> Vec<String> {
        vec![
            format!("operation_id={}", operation.operation_id),
            format!(
                "packet_kind={}",
                operation.packet_kind.as_deref().unwrap_or("none")
            ),
            format!(
                "trigger_kind={}",
                operation.trigger_kind.as_deref().unwrap_or("none")
            ),
            format!("operation_class={}", operation.operation_class),
            format!("replay_readiness={}", operation.replay_readiness),
            format!("retained_consequence={}", operation.retained_consequence),
            format!("semantic_log_boundary={}", operation.semantic_log_boundary),
            format!(
                "reproducibility_contract={}",
                operation.reproducibility_contract
            ),
            format!("non_assumptions={}", operation.non_assumptions.join("|")),
        ]
    }

    fn semantic_logging_boundary_golden_lines(
        boundary: &SemanticLoggingBoundarySummary,
    ) -> Vec<String> {
        vec![
            format!("boundary_id={}", boundary.boundary_id),
            format!("operation_id={}", boundary.operation_id),
            format!(
                "semantic_fact_owners={}",
                boundary.semantic_fact_owners.join("|")
            ),
            format!(
                "upstream_semantic_facts={}",
                boundary.upstream_semantic_facts.join("|")
            ),
            format!("host_owned_facts={}", boundary.host_owned_facts.join("|")),
            format!(
                "shared_replay_inputs={}",
                boundary.shared_replay_inputs.join("|")
            ),
            format!("seam_gaps={}", boundary.seam_gaps.join("|")),
            format!("status={}", boundary.status),
        ]
    }

    fn replay_operation_for_packet_kind(
        adapter: &RuntimeAdapter,
        packet_kind: &str,
    ) -> ReplayAwareOperationSummary {
        adapter
            .replay_operation_model()
            .into_iter()
            .find(|operation| operation.packet_kind.as_deref() == Some(packet_kind))
            .expect("replay-aware operation should exist for packet kind")
    }

    #[test]
    fn dependency_probe_uses_real_upstream_libraries() {
        let report = run_dependency_probe().expect("dependency probe should succeed");

        assert!(!report.formula_token.is_empty());
        assert!(report.parse_token_count > 0);
        assert_eq!(report.parse_diagnostic_count, 0);
        assert_eq!(report.sum_result, 6.0);
        assert!(report.replay_ready);
        assert_eq!(report.replay_registry_ref_count, 1);
    }

    #[test]
    fn runtime_adapter_exposes_profile_and_packet_register() {
        let adapter = adapter_for(OneCalcHostProfile::OcH0);

        assert_eq!(adapter.host_profile(), OneCalcHostProfile::OcH0);
        assert_eq!(adapter.host_profile().id(), "OC-H0");
        assert_eq!(
            adapter.packet_kinds(),
            &[
                HostPacketKind::FormulaEdit,
                HostPacketKind::EditAcceptRecalc,
                HostPacketKind::ReplayCapture,
            ]
        );
    }

    #[test]
    fn runtime_adapter_evaluates_admitted_formula_through_upstream_host() {
        let adapter = adapter_for(OneCalcHostProfile::OcH0);
        let summary = adapter
            .evaluate_formula(promoted_formula_scenario(FormulaScenarioFamily::ExplorerSum).formula)
            .expect("admitted SUM formula should evaluate");

        assert!(!summary.formula_token.is_empty());
        assert_eq!(summary.worksheet_value_summary, "Number(6)");
        assert_eq!(summary.array_preview, None);
        assert_eq!(summary.payload_summary, "Number");
        assert_eq!(summary.returned_presentation_hint_status, "none");
        assert_eq!(summary.host_style_state_status, "none");
        assert_eq!(summary.effective_display_status, "none");
        assert_eq!(summary.commit_decision_kind, "accepted");
    }

    #[test]
    fn runtime_adapter_evaluates_array_formula_with_bounded_preview() {
        let adapter = adapter_for(OneCalcHostProfile::OcH0);
        let summary = adapter
            .evaluate_formula("=SEQUENCE(8,7)")
            .expect("admitted array formula should evaluate");

        assert_eq!(summary.worksheet_value_summary, "Array(8x7)");
        let preview = summary
            .array_preview
            .expect("array-valued results should include a bounded preview");
        assert_eq!(preview.row_count, 8);
        assert_eq!(preview.column_count, 7);
        assert_eq!(preview.hidden_row_count, 2);
        assert_eq!(preview.hidden_column_count, 1);
        assert_eq!(preview.rows.len(), 6);
        assert_eq!(preview.rows[0], vec!["1", "2", "3", "4", "5", "6"]);
        assert_eq!(preview.rows[5], vec!["36", "37", "38", "39", "40", "41"]);
    }

    #[test]
    fn promoted_scenario_corpus_covers_main_product_planes() {
        assert!(promoted_formula_scenarios()
            .iter()
            .any(|scenario| scenario.plane_tags.contains(&"explorer")));
        assert!(promoted_formula_scenarios()
            .iter()
            .any(|scenario| scenario.plane_tags.contains(&"diagnostics")));
        assert!(promoted_formatting_scenarios()
            .iter()
            .any(|scenario| scenario.plane_tags.contains(&"formatting")));
        assert!(promoted_formula_scenarios()
            .iter()
            .any(|scenario| scenario.plane_tags.contains(&"replay")));
        assert!(promoted_observation_scenarios()
            .iter()
            .any(|scenario| scenario.plane_tags.contains(&"observation")));
    }

    #[test]
    fn extension_abi_contract_and_validation_keep_scope_narrow() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let contract = adapter.extension_abi_contract();
        let admitted_manifest = ExtensionProviderManifest {
            provider_id: "demo.sum.provider".to_string(),
            display_name: "Demo Sum Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["host_managed_function_registration".to_string()],
            entrypoint: "providers/demo_sum".to_string(),
        };
        let blocked_manifest = ExtensionProviderManifest {
            provider_id: "demo.rtd.provider".to_string(),
            display_name: "Demo RTD Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["rtd_provider".to_string()],
            entrypoint: "providers/demo_rtd".to_string(),
        };

        let admitted = adapter.validate_extension_manifest(&admitted_manifest);
        let blocked = adapter.validate_extension_manifest(&blocked_manifest);

        assert_eq!(contract.abi_id, "dnaonecalc.desktop.extension_abi");
        assert!(contract
            .admitted_capabilities
            .contains(&"host_managed_function_registration".to_string()));
        assert!(contract
            .excluded_capabilities
            .contains(&"rtd_provider".to_string()));
        assert!(admitted.admitted);
        assert_eq!(
            admitted.admitted_capabilities,
            vec!["host_managed_function_registration".to_string()]
        );
        assert!(!blocked.admitted);
        assert!(blocked
            .blocked_reasons
            .iter()
            .any(|reason| reason.contains("rtd_provider")));
    }

    #[test]
    fn extension_root_loading_surfaces_admitted_and_rejected_providers() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-extension-root-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("demo-sum")).expect("demo sum dir should create");
        fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

        let admitted_manifest = ExtensionProviderManifest {
            provider_id: "demo.sum.provider".to_string(),
            display_name: "Demo Sum Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["host_managed_function_registration".to_string()],
            entrypoint: "providers/demo_sum".to_string(),
        };
        let blocked_manifest = ExtensionProviderManifest {
            provider_id: "demo.rtd.provider".to_string(),
            display_name: "Demo RTD Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["rtd_provider".to_string()],
            entrypoint: "providers/demo_rtd".to_string(),
        };

        fs::write(
            root.join("demo-sum").join("provider.json"),
            serde_json::to_string_pretty(&admitted_manifest)
                .expect("admitted manifest should serialize"),
        )
        .expect("admitted manifest should write");
        fs::write(
            root.join("demo-sum").join("functions.json"),
            serde_json::to_string_pretty(&ExtensionProviderEntrypoint {
                registered_functions: vec![RegisteredExtensionFunction {
                    function_name: "DEMOADD".to_string(),
                    behavior: RegisteredExtensionBehavior::SumNumbers,
                }],
                rtd_topics: Vec::new(),
            })
            .expect("entrypoint should serialize"),
        )
        .expect("entrypoint should write");
        fs::write(
            root.join("demo-rtd").join("provider.json"),
            serde_json::to_string_pretty(&blocked_manifest)
                .expect("blocked manifest should serialize"),
        )
        .expect("blocked manifest should write");

        let loaded = adapter
            .load_extension_root(&root)
            .expect("extension root should load");

        assert_eq!(loaded.discovered_manifest_count, 2);
        assert_eq!(loaded.admitted_providers.len(), 1);
        assert_eq!(loaded.rejected_providers.len(), 1);
        assert!(loaded.malformed_manifests.is_empty());
        assert_eq!(
            loaded.admitted_providers[0].manifest.provider_id,
            "demo.sum.provider"
        );
        assert_eq!(
            loaded.admitted_providers[0]
                .validation
                .admitted_capabilities,
            vec!["host_managed_function_registration".to_string()]
        );
        assert_eq!(
            loaded.rejected_providers[0].manifest.provider_id,
            "demo.rtd.provider"
        );
        assert!(loaded.rejected_providers[0]
            .validation
            .blocked_reasons
            .iter()
            .any(|reason| reason.contains("rtd_provider")));
    }

    #[test]
    fn extension_provider_invocation_keeps_failures_explicit() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-extension-provider-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("demo-sum")).expect("demo sum dir should create");
        fs::create_dir_all(root.join("demo-fail")).expect("demo fail dir should create");
        fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

        let admitted_manifest = ExtensionProviderManifest {
            provider_id: "demo.sum.provider".to_string(),
            display_name: "Demo Sum Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["host_managed_function_registration".to_string()],
            entrypoint: "functions.json".to_string(),
        };
        let failing_manifest = ExtensionProviderManifest {
            provider_id: "demo.fail.provider".to_string(),
            display_name: "Demo Fail Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["host_managed_function_registration".to_string()],
            entrypoint: "functions.json".to_string(),
        };
        let blocked_manifest = ExtensionProviderManifest {
            provider_id: "demo.rtd.provider".to_string(),
            display_name: "Demo RTD Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["rtd_provider".to_string()],
            entrypoint: "functions.json".to_string(),
        };

        fs::write(
            root.join("demo-sum").join("provider.json"),
            serde_json::to_string_pretty(&admitted_manifest)
                .expect("admitted manifest should serialize"),
        )
        .expect("admitted manifest should write");
        fs::write(
            root.join("demo-sum").join("functions.json"),
            serde_json::to_string_pretty(&ExtensionProviderEntrypoint {
                registered_functions: vec![RegisteredExtensionFunction {
                    function_name: "DEMOADD".to_string(),
                    behavior: RegisteredExtensionBehavior::SumNumbers,
                }],
                rtd_topics: Vec::new(),
            })
            .expect("sum entrypoint should serialize"),
        )
        .expect("sum entrypoint should write");

        fs::write(
            root.join("demo-fail").join("provider.json"),
            serde_json::to_string_pretty(&failing_manifest)
                .expect("failing manifest should serialize"),
        )
        .expect("failing manifest should write");
        fs::write(
            root.join("demo-fail").join("functions.json"),
            serde_json::to_string_pretty(&ExtensionProviderEntrypoint {
                registered_functions: vec![RegisteredExtensionFunction {
                    function_name: "DEMOFAIL".to_string(),
                    behavior: RegisteredExtensionBehavior::AlwaysError {
                        message: "provider execution failed".to_string(),
                    },
                }],
                rtd_topics: Vec::new(),
            })
            .expect("failing entrypoint should serialize"),
        )
        .expect("failing entrypoint should write");

        fs::write(
            root.join("demo-rtd").join("provider.json"),
            serde_json::to_string_pretty(&blocked_manifest)
                .expect("blocked manifest should serialize"),
        )
        .expect("blocked manifest should write");

        let sum = adapter
            .invoke_extension_provider(
                &root,
                "demo.sum.provider",
                "DEMOADD",
                &[
                    ExtensionInvocationArgument::Number(1.0),
                    ExtensionInvocationArgument::Number(2.0),
                    ExtensionInvocationArgument::Number(3.0),
                ],
            )
            .expect("admitted provider should invoke");
        let fail = adapter
            .invoke_extension_provider(&root, "demo.fail.provider", "DEMOFAIL", &[])
            .expect("failing provider should still return explicit state");
        let blocked = adapter
            .invoke_extension_provider(&root, "demo.rtd.provider", "RTDDEMO", &[])
            .expect("blocked provider should still return explicit state");

        assert_eq!(sum.provider_state, "admitted");
        assert_eq!(sum.invocation_state, "returned");
        assert_eq!(sum.value_summary.as_deref(), Some("Number(6)"));
        assert_eq!(sum.failure_reason, None);

        assert_eq!(fail.provider_state, "admitted");
        assert_eq!(fail.invocation_state, "provider_error");
        assert_eq!(
            fail.failure_reason.as_deref(),
            Some("provider execution failed")
        );

        assert_eq!(blocked.provider_state, "rejected");
        assert_eq!(blocked.invocation_state, "blocked");
        assert!(blocked
            .failure_reason
            .as_deref()
            .expect("blocked provider should report a reason")
            .contains("rtd_provider"));
    }

    #[test]
    fn extension_runtime_truth_keeps_declared_rtd_state_visible() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-extension-rtd-truth-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("demo-sum")).expect("demo sum dir should create");
        fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

        let admitted_manifest = ExtensionProviderManifest {
            provider_id: "demo.sum.provider".to_string(),
            display_name: "Demo Sum Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["host_managed_function_registration".to_string()],
            entrypoint: "functions.json".to_string(),
        };
        let rtd_manifest = ExtensionProviderManifest {
            provider_id: "demo.rtd.provider".to_string(),
            display_name: "Demo RTD Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["rtd_provider".to_string()],
            entrypoint: "functions.json".to_string(),
        };

        fs::write(
            root.join("demo-sum").join("provider.json"),
            serde_json::to_string_pretty(&admitted_manifest)
                .expect("admitted manifest should serialize"),
        )
        .expect("admitted manifest should write");
        fs::write(
            root.join("demo-rtd").join("provider.json"),
            serde_json::to_string_pretty(&rtd_manifest).expect("rtd manifest should serialize"),
        )
        .expect("rtd manifest should write");

        let truth = adapter
            .extension_root_runtime_truth(&root)
            .expect("extension truth should load");
        let sum_provider = truth
            .provider_truths
            .iter()
            .find(|provider| provider.provider_id == "demo.sum.provider")
            .expect("sum provider should be present");
        let rtd_provider = truth
            .provider_truths
            .iter()
            .find(|provider| provider.provider_id == "demo.rtd.provider")
            .expect("rtd provider should be present");

        assert_eq!(sum_provider.provider_state, "admitted");
        assert_eq!(sum_provider.capability_truths[0].runtime_state, "admitted");
        assert_eq!(
            rtd_provider.provider_state,
            "declared_with_blocked_capabilities"
        );
        assert_eq!(
            rtd_provider.capability_truths[0].capability_id,
            "rtd_provider"
        );
        if std::env::consts::OS == "windows" {
            assert_eq!(
                rtd_provider.capability_truths[0].runtime_state,
                "declared_but_not_yet_admitted"
            );
        } else {
            assert_eq!(
                rtd_provider.capability_truths[0].runtime_state,
                "blocked_by_platform"
            );
        }
    }

    #[test]
    fn windows_rtd_activation_runs_the_admitted_in_process_subset() {
        if std::env::consts::OS != "windows" {
            return;
        }

        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-windows-rtd-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

        let rtd_manifest = ExtensionProviderManifest {
            provider_id: "demo.rtd.provider".to_string(),
            display_name: "Demo RTD Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["rtd_provider".to_string()],
            entrypoint: "functions.json".to_string(),
        };

        fs::write(
            root.join("demo-rtd").join("provider.json"),
            serde_json::to_string_pretty(&rtd_manifest).expect("rtd manifest should serialize"),
        )
        .expect("rtd manifest should write");
        fs::write(
            root.join("demo-rtd").join("functions.json"),
            serde_json::to_string_pretty(&ExtensionProviderEntrypoint {
                registered_functions: Vec::new(),
                rtd_topics: vec![RegisteredRtdTopic {
                    topic_id: "PRICE".to_string(),
                    initial_value: "100.0".to_string(),
                    updates: vec!["101.5".to_string(), "103.0".to_string()],
                }],
            })
            .expect("rtd entrypoint should serialize"),
        )
        .expect("rtd entrypoint should write");

        let truth = adapter
            .extension_root_runtime_truth(&root)
            .expect("extension truth should load");
        let rtd_provider = truth
            .provider_truths
            .iter()
            .find(|provider| provider.provider_id == "demo.rtd.provider")
            .expect("rtd provider should be present");
        assert_eq!(
            rtd_provider.capability_truths[0].runtime_state,
            "admitted_windows_subset"
        );

        let mut session = adapter
            .activate_windows_rtd_topic(&root, "demo.rtd.provider", "PRICE")
            .expect("windows rtd topic should activate");
        assert_eq!(session.current_value, "100.0");
        assert_eq!(session.lifecycle_state, "active");

        let first = adapter.advance_rtd_topic(&mut session);
        let second = adapter.advance_rtd_topic(&mut session);
        let third = adapter.advance_rtd_topic(&mut session);

        assert_eq!(first.current_value, "101.5");
        assert_eq!(first.remaining_update_count, 1);
        assert_eq!(second.current_value, "103.0");
        assert_eq!(second.remaining_update_count, 0);
        assert_eq!(second.lifecycle_state, "active_final_value");
        assert_eq!(third.current_value, "103.0");
        assert_eq!(third.lifecycle_state, "active_no_pending_updates");
    }

    #[test]
    fn linux_rtd_registry_truth_keeps_cross_platform_gate_explicit() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-linux-rtd-registry-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("demo-rtd")).expect("demo rtd dir should create");

        let rtd_manifest = ExtensionProviderManifest {
            provider_id: "demo.rtd.provider".to_string(),
            display_name: "Demo RTD Provider".to_string(),
            abi_version: "v1".to_string(),
            host_profile_ids: vec!["OC-H1".to_string()],
            platform_gate_ids: vec!["desktop_native_only".to_string()],
            declared_capabilities: vec!["rtd_provider".to_string()],
            entrypoint: "functions.json".to_string(),
        };

        fs::write(
            root.join("demo-rtd").join("provider.json"),
            serde_json::to_string_pretty(&rtd_manifest).expect("rtd manifest should serialize"),
        )
        .expect("rtd manifest should write");
        fs::write(
            root.join("demo-rtd").join("functions.json"),
            serde_json::to_string_pretty(&ExtensionProviderEntrypoint {
                registered_functions: Vec::new(),
                rtd_topics: vec![RegisteredRtdTopic {
                    topic_id: "PRICE".to_string(),
                    initial_value: "100.0".to_string(),
                    updates: vec!["101.5".to_string()],
                }],
            })
            .expect("rtd entrypoint should serialize"),
        )
        .expect("rtd entrypoint should write");

        let summary = adapter
            .linux_rtd_registry_truth(&root)
            .expect("linux rtd registry truth should load");

        assert_eq!(summary.entries.len(), 1);
        assert_eq!(summary.entries[0].provider_id, "demo.rtd.provider");
        assert_eq!(summary.entries[0].topic_ids, vec!["PRICE".to_string()]);
        if std::env::consts::OS == "linux" {
            assert_eq!(summary.gate_state, "admitted_design_floor");
        } else {
            assert_eq!(summary.gate_state, "blocked_on_host_platform");
        }
    }

    #[test]
    fn promoted_scenario_index_links_live_retained_evidence() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.index", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let recalc_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let recalc_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", recalc_context.clone())
            .expect("edit-and-accept recalc should succeed");

        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-scenario-index-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(&root);
        let persisted = adapter
            .persist_driven_scenario_run(
                &store,
                &host,
                &recalc_context,
                &recalc_summary,
                "index-smoke",
            )
            .expect("retained run should persist");
        let replay = adapter
            .emit_replay_capture_for_run(&store, &persisted.run.scenario_run_id)
            .expect("replay capture should persist");

        let run_ref = persisted.run.envelope.stable_ref();
        let capability_snapshot_ref = persisted.capability_snapshot.snapshot.envelope.stable_ref();
        let witness_ref = StableArtifactRef {
            artifact_kind: ArtifactKind::Witness.id().to_string(),
            logical_id: "witness-index-smoke".to_string(),
            content_hash: None,
        };
        let comparison = ComparisonRecord {
            envelope: ArtifactEnvelope {
                schema_id: "dnaonecalc.artifact.comparison".to_string(),
                schema_version: "v1".to_string(),
                artifact_kind: ArtifactKind::Comparison.id().to_string(),
                logical_id: "comparison-index-smoke".to_string(),
                content_hash: stable_hash(&(
                    "comparison-index-smoke",
                    persisted.run.scenario_run_id.as_str(),
                )),
                created_at_unix_ms: 1,
                created_by_build: "dnaonecalc-host@test".to_string(),
                host_profile_id: "OC-H1".to_string(),
                packet_kind: "twin_compare".to_string(),
                seam_pin_set_id: "onecalc:test".to_string(),
                capability_floor: "OC-H1".to_string(),
                provisionality_state: "stable".to_string(),
                lineage_refs: Vec::new(),
                attachment_refs: Vec::new(),
                capability_snapshot_ref: Some(capability_snapshot_ref.clone()),
            },
            comparison_id: "comparison-index-smoke".to_string(),
            left_artifact_ref: run_ref.clone(),
            right_artifact_ref: StableArtifactRef {
                artifact_kind: ArtifactKind::Observation.id().to_string(),
                logical_id: "observation-index-smoke".to_string(),
                content_hash: None,
            },
            comparison_envelope: vec!["value_surface".to_string()],
            mismatches: Vec::new(),
            reliability_badge: "narrow".to_string(),
            projection_limitations: vec!["observation_envelope_narrow".to_string()],
            explanation_refs: Vec::new(),
            witness_candidate_refs: vec![witness_ref.clone()],
        };
        let witness = WitnessRecord {
            envelope: ArtifactEnvelope {
                schema_id: "dnaonecalc.artifact.witness".to_string(),
                schema_version: "v1".to_string(),
                artifact_kind: ArtifactKind::Witness.id().to_string(),
                logical_id: witness_ref.logical_id.clone(),
                content_hash: stable_hash(&(
                    "witness-index-smoke",
                    persisted.run.scenario_run_id.as_str(),
                )),
                created_at_unix_ms: 2,
                created_by_build: "dnaonecalc-host@test".to_string(),
                host_profile_id: "OC-H1".to_string(),
                packet_kind: "explain".to_string(),
                seam_pin_set_id: "onecalc:test".to_string(),
                capability_floor: "OC-H1".to_string(),
                provisionality_state: "stable".to_string(),
                lineage_refs: Vec::new(),
                attachment_refs: Vec::new(),
                capability_snapshot_ref: Some(capability_snapshot_ref.clone()),
            },
            witness_id: witness_ref.logical_id.clone(),
            scenario_id: persisted.scenario.scenario_id.clone(),
            left_run_ref: run_ref.clone(),
            right_run_ref: run_ref.clone(),
            explain_floor: "retained_diff".to_string(),
            explanation_lines: vec!["stable".to_string()],
            blocked_dimensions: vec!["excel_observation".to_string()],
            replay_diff_equivalent: false,
            replay_mismatch_count: 1,
            replay_explain_query_id: "explain-index-smoke".to_string(),
            replay_explain_summary: "stable".to_string(),
            semantic_log_boundary_ids: vec![
                "oxfml_runtime_result_plus_host_retained_artifacts".to_string(),
            ],
            seam_gaps: vec!["no_cross_library_semantic_log_contract_exists".to_string()],
            emitted_at_unix_ms: 2,
        };
        let handoff = HandoffPacketRecord {
            envelope: ArtifactEnvelope {
                schema_id: "dnaonecalc.artifact.handoff_packet".to_string(),
                schema_version: "v1".to_string(),
                artifact_kind: ArtifactKind::HandoffPacket.id().to_string(),
                logical_id: "handoff-index-smoke".to_string(),
                content_hash: stable_hash(&(
                    "handoff-index-smoke",
                    persisted.run.scenario_run_id.as_str(),
                )),
                created_at_unix_ms: 3,
                created_by_build: "dnaonecalc-host@test".to_string(),
                host_profile_id: "OC-H1".to_string(),
                packet_kind: "handoff".to_string(),
                seam_pin_set_id: "onecalc:test".to_string(),
                capability_floor: "OC-H1".to_string(),
                provisionality_state: "stable".to_string(),
                lineage_refs: Vec::new(),
                attachment_refs: Vec::new(),
                capability_snapshot_ref: Some(capability_snapshot_ref.clone()),
            },
            handoff_id: "handoff-index-smoke".to_string(),
            scenario_id: persisted.scenario.scenario_id.clone(),
            source_run_ref: run_ref.clone(),
            witness_ref: witness_ref.clone(),
            capability_snapshot_ref: capability_snapshot_ref.clone(),
            requested_action_kind: "widen_compare".to_string(),
            target_lane: "OxXlPlay".to_string(),
            expected_behavior: "stable".to_string(),
            observed_behavior: "stable".to_string(),
            supporting_artifact_refs: vec![replay.capture.envelope.stable_ref()],
            reliability_state: "narrow".to_string(),
            blocked_dimensions: vec!["excel_observation".to_string()],
            replay_explain_query_id: "explain-index-smoke".to_string(),
            replay_explain_summary: "stable".to_string(),
            semantic_log_boundary_ids: vec![
                "oxfml_runtime_result_plus_host_retained_artifacts".to_string(),
            ],
            seam_gaps: vec!["no_cross_library_semantic_log_contract_exists".to_string()],
            status: "ready".to_string(),
            readiness: vec![HandoffReadinessRecord {
                item_id: "capability_snapshot".to_string(),
                satisfied: true,
            }],
            emitted_at_unix_ms: 3,
        };

        store
            .persist_comparison(&comparison)
            .expect("comparison should persist");
        store
            .persist_witness(&witness)
            .expect("witness should persist");
        store
            .persist_handoff_packet(&handoff)
            .expect("handoff should persist");

        let index = adapter
            .build_promoted_scenario_index(&store)
            .expect("promoted scenario index should build");

        assert_eq!(index.rows.len(), 1);
        let row = &index.rows[0];
        assert_eq!(
            row.row_id,
            format!("promoted-scenario:{}", persisted.scenario.scenario_id)
        );
        assert_eq!(row.scenario_id, persisted.scenario.scenario_id);
        assert_eq!(row.latest_run_id, persisted.run.scenario_run_id);
        assert_eq!(
            row.replay_capture_ids,
            vec![replay.capture.replay_capture_id.clone()]
        );
        assert_eq!(
            row.comparison_ids,
            vec!["comparison-index-smoke".to_string()]
        );
        assert_eq!(row.witness_ids, vec!["witness-index-smoke".to_string()]);
        assert_eq!(row.handoff_ids, vec!["handoff-index-smoke".to_string()]);
    }

    #[test]
    fn scenario_library_filters_and_saved_views_use_promoted_index_fields() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let index = PromotedScenarioIndex {
            rows: vec![
                PromotedScenarioIndexRow {
                    row_id: "promoted-scenario:one".to_string(),
                    scenario_id: "scenario-one".to_string(),
                    scenario_slug: "one".to_string(),
                    latest_run_id: "run-one".to_string(),
                    host_profile_id: "OC-H1".to_string(),
                    runtime_platform: std::env::consts::OS.to_string(),
                    formula_text: "=SUM(1,2,3)".to_string(),
                    worksheet_value_summary: "Number(6)".to_string(),
                    replay_capture_ids: vec!["replay-one".to_string()],
                    comparison_ids: vec!["comparison-one".to_string()],
                    witness_ids: vec!["witness-one".to_string()],
                    handoff_ids: vec!["handoff-one".to_string()],
                },
                PromotedScenarioIndexRow {
                    row_id: "promoted-scenario:two".to_string(),
                    scenario_id: "scenario-two".to_string(),
                    scenario_slug: "two".to_string(),
                    latest_run_id: "run-two".to_string(),
                    host_profile_id: "OC-H1".to_string(),
                    runtime_platform: std::env::consts::OS.to_string(),
                    formula_text: "=ABS(-3)".to_string(),
                    worksheet_value_summary: "Number(3)".to_string(),
                    replay_capture_ids: vec!["replay-two".to_string()],
                    comparison_ids: Vec::new(),
                    witness_ids: Vec::new(),
                    handoff_ids: Vec::new(),
                },
            ],
        };
        let filter = ScenarioLibraryFilter {
            host_profile_ids: vec!["OC-H1".to_string()],
            runtime_platform: Some(std::env::consts::OS.to_string()),
            replay_required: Some(true),
            comparison_required: Some(true),
            witness_required: Some(true),
            handoff_required: Some(true),
        };
        let filtered = adapter.apply_scenario_library_filter(&index, &filter);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].scenario_id, "scenario-one");

        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-scenario-library-view-test-{}.json",
            std::process::id()
        ));
        let view = ScenarioLibrarySavedView {
            view_id: "with-evidence".to_string(),
            display_name: "With Evidence".to_string(),
            filter: filter.clone(),
        };
        adapter
            .save_scenario_library_view(&root, &view)
            .expect("saved view should persist");
        let reopened = adapter
            .read_scenario_library_view(&root)
            .expect("saved view should reopen");

        assert_eq!(reopened, view);
    }

    #[test]
    fn scenario_selection_detail_exposes_only_real_lineage_actions() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let row = PromotedScenarioIndexRow {
            row_id: "promoted-scenario:one".to_string(),
            scenario_id: "scenario-one".to_string(),
            scenario_slug: "one".to_string(),
            latest_run_id: "run-one".to_string(),
            host_profile_id: "OC-H1".to_string(),
            runtime_platform: std::env::consts::OS.to_string(),
            formula_text: "=SUM(1,2,3)".to_string(),
            worksheet_value_summary: "Number(6)".to_string(),
            replay_capture_ids: vec!["replay-one".to_string()],
            comparison_ids: vec!["comparison-one".to_string()],
            witness_ids: vec!["witness-one".to_string()],
            handoff_ids: vec!["handoff-one".to_string()],
        };

        let detail = adapter.build_scenario_selection_detail(&row);

        assert_eq!(detail.row_id, row.row_id);
        assert_eq!(detail.scenario_id, row.scenario_id);
        assert_eq!(detail.latest_run_id, row.latest_run_id);
        assert!(detail
            .lineage
            .iter()
            .any(|item| item.relation == "replay_capture" && item.artifact_id == "replay-one"));
        assert!(detail
            .available_actions
            .iter()
            .any(|item| item.action_id == "open_replay" && item.target_id == "replay-one"));
        assert!(detail
            .available_actions
            .iter()
            .any(|item| item.action_id == "open_compare" && item.target_id == "comparison-one"));
        assert!(detail
            .available_actions
            .iter()
            .any(|item| item.action_id == "open_witness" && item.target_id == "witness-one"));
        assert!(detail
            .available_actions
            .iter()
            .any(|item| item.action_id == "open_handoff" && item.target_id == "handoff-one"));
    }

    #[test]
    fn acceptance_matrix_rows_are_derived_from_promoted_rows_and_capability_truth() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.acceptance", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let recalc_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let recalc_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", recalc_context.clone())
            .expect("edit-and-accept recalc should succeed");

        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-acceptance-matrix-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(&root);
        let persisted = adapter
            .persist_driven_scenario_run(
                &store,
                &host,
                &recalc_context,
                &recalc_summary,
                "acceptance-smoke",
            )
            .expect("retained run should persist");
        adapter
            .emit_replay_capture_for_run(&store, &persisted.run.scenario_run_id)
            .expect("replay capture should persist");

        let matrix = adapter
            .build_acceptance_matrix(&store)
            .expect("acceptance matrix should build");

        assert_eq!(matrix.rows.len(), 1);
        let row = &matrix.rows[0];
        assert_eq!(
            row.row_id,
            format!("promoted-scenario:{}", persisted.scenario.scenario_id)
        );
        assert_eq!(row.latest_run_id, persisted.run.scenario_run_id);
        assert_eq!(row.capability_floor, "OC-H1");
        assert_eq!(row.replay_status, "available");
        assert_eq!(row.comparison_status, "missing");
        assert_eq!(row.witness_status, "missing");
        assert_eq!(row.handoff_status, "missing");
    }

    #[test]
    fn regression_summary_and_upstream_pressure_packets_follow_acceptance_rows() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let matrix = AcceptanceMatrix {
            rows: vec![
                AcceptanceMatrixRow {
                    row_id: "promoted-scenario:one".to_string(),
                    scenario_id: "scenario-one".to_string(),
                    latest_run_id: "run-one".to_string(),
                    capability_snapshot_id: "cap-one".to_string(),
                    capability_floor: "OC-H1".to_string(),
                    runtime_class: "desktop_native_only".to_string(),
                    replay_status: "available".to_string(),
                    comparison_status: "missing".to_string(),
                    witness_status: "missing".to_string(),
                    handoff_status: "missing".to_string(),
                },
                AcceptanceMatrixRow {
                    row_id: "promoted-scenario:two".to_string(),
                    scenario_id: "scenario-two".to_string(),
                    latest_run_id: "run-two".to_string(),
                    capability_snapshot_id: "cap-two".to_string(),
                    capability_floor: "OC-H1".to_string(),
                    runtime_class: "desktop_native_only".to_string(),
                    replay_status: "available".to_string(),
                    comparison_status: "available".to_string(),
                    witness_status: "available".to_string(),
                    handoff_status: "available".to_string(),
                },
            ],
        };

        let summary = adapter.build_promoted_scenario_regression_summary(&matrix);
        let packets = adapter.build_upstream_pressure_packets(&matrix);
        let path = std::env::temp_dir().join(format!(
            "dnaonecalc-regression-summary-test-{}.json",
            std::process::id()
        ));
        adapter
            .save_promoted_scenario_regression_summary(&path, &summary)
            .expect("regression summary should persist");
        let reopened = adapter
            .read_promoted_scenario_regression_summary(&path)
            .expect("regression summary should reopen");

        assert_eq!(summary.total_rows, 2);
        assert_eq!(summary.replay_ready_rows, 2);
        assert_eq!(summary.compare_ready_rows, 1);
        assert_eq!(summary.witness_ready_rows, 1);
        assert_eq!(summary.handoff_ready_rows, 1);
        assert_eq!(reopened, summary);

        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].scenario_id, "scenario-one");
        assert_eq!(packets[0].packet_kind, "acceptance_gap");
        assert!(packets[0]
            .blocker_ids
            .contains(&"comparison_missing".to_string()));
        assert_eq!(packets[0].target_lane, "OxXlPlay");
        assert_eq!(
            packets[0].evidence_artifact_ids,
            vec!["run-one".to_string(), "cap-one".to_string()]
        );
        assert!(packets[0]
            .summary
            .contains("acceptance row promoted-scenario:one is blocked"));
    }

    #[test]
    fn h1_driven_host_runs_edit_accept_manual_and_forced_recalc_with_explicit_context() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");

        let edit_summary = adapter
            .edit_accept_recalc(
                &mut host,
                "=SUM(1,2,3)",
                RecalcContext::edit_accept(Some(46_000.0), Some(0.25)),
            )
            .expect("edit-and-accept recalc should succeed");
        assert_eq!(edit_summary.host_profile_id, "OC-H1");
        assert_eq!(
            edit_summary.host_session_id,
            host.session_state().session_id
        );
        assert_eq!(edit_summary.host_recalc_sequence, 1);
        assert_eq!(edit_summary.replay_operation_id, "edit_accept_recalc");
        assert_eq!(
            edit_summary.replay_operation_readiness,
            "recalc_projection_ready"
        );
        assert_eq!(
            edit_summary.replay_retained_consequence,
            "persists_formula_version_and_retained_run_when_requested"
        );
        assert_eq!(edit_summary.trigger_kind, "edit_accept");
        assert_eq!(edit_summary.packet_kind, "edit_accept_recalc");
        assert_eq!(edit_summary.formula_text_version, 2);
        assert_eq!(
            edit_summary.structure_context_version,
            "onecalc:single_formula:h1"
        );
        assert_eq!(edit_summary.evaluation.worksheet_value_summary, "Number(6)");

        let manual_summary = adapter
            .manual_recalc(&mut host, RecalcContext::manual(Some(46_000.0), Some(0.25)))
            .expect("manual recalc should succeed");
        assert_eq!(manual_summary.host_session_id, edit_summary.host_session_id);
        assert_eq!(manual_summary.host_recalc_sequence, 2);
        assert_eq!(manual_summary.replay_operation_id, "manual_recalc");
        assert_eq!(
            manual_summary.replay_retained_consequence,
            "reuses_current_formula_state_with_explicit_context"
        );
        assert_eq!(manual_summary.trigger_kind, "manual");
        assert_eq!(manual_summary.packet_kind, "manual_recalc");
        assert_eq!(manual_summary.formula_text_version, 2);
        assert_eq!(
            manual_summary.evaluation.worksheet_value_summary,
            "Number(6)"
        );

        let forced_summary = adapter
            .forced_recalc(&mut host, RecalcContext::forced(Some(46_000.0), Some(0.25)))
            .expect("forced recalc should succeed");
        assert_eq!(forced_summary.host_session_id, edit_summary.host_session_id);
        assert_eq!(forced_summary.host_recalc_sequence, 3);
        assert_eq!(forced_summary.replay_operation_id, "forced_recalc");
        assert_eq!(
            forced_summary.replay_retained_consequence,
            "reuses_current_formula_state_with_forced_provisionality"
        );
        assert_eq!(forced_summary.trigger_kind, "forced");
        assert_eq!(forced_summary.packet_kind, "forced_recalc");
        assert_eq!(forced_summary.formula_text_version, 2);
        assert_eq!(
            forced_summary.evaluation.worksheet_value_summary,
            "Number(6)"
        );
    }

    #[test]
    fn replay_operation_model_makes_current_and_future_host_operations_explicit() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let model = adapter.replay_operation_model();

        assert_eq!(model.len(), ReplayAwareOperationKind::ALL.len());
        assert_eq!(
            replay_operation_golden_lines(&model[0]),
            vec![
                "operation_id=edit_accept_recalc".to_string(),
                "packet_kind=edit_accept_recalc".to_string(),
                "trigger_kind=edit_accept".to_string(),
                "operation_class=driven_recalc".to_string(),
                "replay_readiness=recalc_projection_ready".to_string(),
                "retained_consequence=persists_formula_version_and_retained_run_when_requested"
                    .to_string(),
                "semantic_log_boundary=oxfml_runtime_result_plus_host_retained_artifacts"
                    .to_string(),
                "reproducibility_contract=requires_explicit_now_serial_and_random_value"
                    .to_string(),
                "non_assumptions=does_not_define_undo_inverse|does_not_imply_macro_capture_stream"
                    .to_string(),
            ]
        );
        assert_eq!(
            replay_operation_golden_lines(
                model.last().expect("macro capture operation should exist")
            ),
            vec![
                "operation_id=macro_capture".to_string(),
                "packet_kind=none".to_string(),
                "trigger_kind=none".to_string(),
                "operation_class=future_host_operation".to_string(),
                "replay_readiness=not_admitted_yet".to_string(),
                "retained_consequence=no_current_retained_artifact".to_string(),
                "semantic_log_boundary=future_macro_capture_boundary_not_designed".to_string(),
                "reproducibility_contract=macro_capture_pipeline_not_proven".to_string(),
                "non_assumptions=no_host_macro_dsl_exists|no_cross_library_semantic_log_contract_exists"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn semantic_logging_boundary_model_makes_owner_split_and_seam_gaps_explicit() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let boundaries = adapter.semantic_logging_boundaries();

        assert_eq!(boundaries.len(), ReplayAwareOperationKind::ALL.len());
        assert_eq!(
            semantic_logging_boundary_golden_lines(&boundaries[0]),
            vec![
                "boundary_id=oxfml_runtime_result_plus_host_retained_artifacts".to_string(),
                "operation_id=edit_accept_recalc".to_string(),
                "semantic_fact_owners=OxFml|OxFunc|OxReplay|DnaOneCalcHost".to_string(),
                "upstream_semantic_facts=oxfml_formula_runtime_result|oxfunc_function_semantics_transitively_consumed_via_oxfml|oxreplay_bundle_replay_diff_explain_runtime".to_string(),
                "host_owned_facts=recalc_context_inputs|retained_artifact_envelopes|capability_snapshot_linkage|workspace_document_capsule_persistence".to_string(),
                "shared_replay_inputs=oxfml_v1_replay_projection|replay.bundle.v1|replay_diff_report|explain_record".to_string(),
                "seam_gaps=no_cross_library_semantic_log_contract_exists|oxfunc_direct_replay_intake_not_admitted|replay_owned_retained_artifact_contract_not_defined".to_string(),
                "status=provisional_current_floor".to_string(),
            ]
        );
        assert_eq!(
            semantic_logging_boundary_golden_lines(
                boundaries.last().expect("macro capture boundary should exist")
            ),
            vec![
                "boundary_id=future_macro_capture_boundary_not_designed".to_string(),
                "operation_id=macro_capture".to_string(),
                "semantic_fact_owners=OxReplay|DnaOneCalcHost|OxFml|OxFunc".to_string(),
                "upstream_semantic_facts=replay_action_stream_capture|formula_semantic_effect_classification|function_side_effect_and_observation_classification".to_string(),
                "host_owned_facts=macro_recording_session_controls|macro_capture_retention_policy".to_string(),
                "shared_replay_inputs=future_macro_capture_bundle|future_macro_explain_query".to_string(),
                "seam_gaps=macro_capture_pipeline_not_proven|no_host_macro_dsl_exists|no_cross_library_semantic_log_contract_exists".to_string(),
                "status=not_designed".to_string(),
            ]
        );
    }

    #[test]
    fn h0_profile_rejects_h1_driven_host_model() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH0);
        let error = adapter
            .new_driven_single_formula_host("onecalc.h1", "=SUM(1,2,3)")
            .expect_err("OC-H0 should reject the driven host model");

        assert!(error.contains("does not admit the driven single-formula host model"));
    }

    #[test]
    fn h1_runs_persist_scenario_and_scenario_run_and_reopen_through_runtime() {
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let (recalc_context, recalc_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumBaseline, 46_000.0, 0.25);
        let fixture_root = FixtureRoot::new("h1-retained");
        let store = fixture_root.retained_store();
        let persisted = fixture.persist_run(
            &store,
            &recalc_context,
            &recalc_summary,
            FormulaScenarioFamily::RetainedSumBaseline,
        );

        assert!(persisted.scenario_path.exists());
        assert!(persisted.run_path.exists());
        assert!(persisted.capability_snapshot.snapshot_path.exists());
        assert_eq!(persisted.scenario.scenario_slug, "sum-baseline");
        assert_eq!(persisted.scenario.host_profile_id, "OC-H1");
        assert_eq!(persisted.run.scenario_id, persisted.scenario.scenario_id);
        assert_eq!(persisted.scenario.envelope.artifact_kind, "scenario");
        assert_eq!(persisted.run.envelope.artifact_kind, "scenario_run");
        assert_eq!(
            persisted
                .capability_snapshot
                .snapshot
                .envelope
                .artifact_kind,
            "capability_ledger_snapshot"
        );
        assert_eq!(
            persisted.run.scenario_ref.logical_id,
            persisted.scenario.scenario_id
        );
        assert_eq!(persisted.run.envelope.lineage_refs.len(), 1);
        assert_eq!(
            persisted.run.envelope.lineage_refs[0]
                .artifact_ref
                .logical_id,
            persisted.scenario.scenario_id
        );
        assert_eq!(
            persisted
                .run
                .envelope
                .capability_snapshot_ref
                .as_ref()
                .expect("run should point to the capability snapshot")
                .logical_id,
            persisted
                .capability_snapshot
                .snapshot
                .capability_snapshot_id
        );
        assert_eq!(
            persisted
                .scenario
                .envelope
                .capability_snapshot_ref
                .as_ref()
                .expect("scenario should point to the capability snapshot")
                .logical_id,
            persisted
                .capability_snapshot
                .snapshot
                .capability_snapshot_id
        );

        let mut reopened = fixture
            .adapter
            .reopen_driven_scenario_run(&store, &persisted.run.scenario_run_id)
            .expect("retained run should reopen");
        assert_eq!(
            reopened.retained.scenario.formula_text,
            persisted.scenario.formula_text
        );
        assert_eq!(
            reopened.driven_host.formula_text(),
            persisted.scenario.formula_text
        );
        assert_eq!(
            reopened.driven_host.formula_text_version(),
            recalc_summary.formula_text_version
        );
        assert_eq!(
            reopened.driven_host.session_state().session_id,
            persisted.run.host_session_id
        );
        assert_eq!(
            reopened.driven_host.session_state().recalc_sequence,
            persisted.run.host_recalc_sequence
        );

        let reopened_summary = fixture
            .adapter
            .manual_recalc(
                &mut reopened.driven_host,
                RecalcContext::manual(Some(46_000.0), Some(0.25)),
            )
            .expect("reopened driven host should recalc");
        assert_eq!(reopened_summary.host_profile_id, "OC-H1");
        assert_eq!(
            reopened_summary.host_session_id,
            persisted.run.host_session_id
        );
        assert_eq!(
            reopened_summary.host_recalc_sequence,
            persisted.run.host_recalc_sequence + 1
        );
        assert_eq!(
            reopened_summary.evaluation.worksheet_value_summary,
            "Number(6)"
        );
    }

    #[test]
    fn retained_run_reopen_invariants_hold_for_active_and_reopened_h1_flow() {
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let (recalc_context, recalc_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumBaseline, 46_000.0, 0.25);
        let fixture_root = FixtureRoot::new("retained-reopen-invariants");
        let store = fixture_root.retained_store();
        let persisted = fixture.persist_run(
            &store,
            &recalc_context,
            &recalc_summary,
            FormulaScenarioFamily::RetainedSumBaseline,
        );

        let report = fixture
            .adapter
            .verify_reopened_driven_scenario_run_invariants(&store, &persisted.run.scenario_run_id)
            .expect("retained reopen invariants should hold");

        assert!(report.scenario_ref_preserved);
        assert!(report.formula_identity_preserved);
        assert!(report.structure_context_preserved);
        assert!(report.session_state_preserved);
        assert!(report.capability_snapshot_links_preserved);
        assert!(report.replay_projection_links_preserved);
        assert!(report.all_preserved());
    }

    #[test]
    fn retained_run_reopen_invariant_regressions_surface_when_persisted_state_is_tampered() {
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let (recalc_context, recalc_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumBaseline, 46_000.0, 0.25);
        let fixture_root = FixtureRoot::new("retained-reopen-regression");
        let store = fixture_root.retained_store();
        let persisted = fixture.persist_run(
            &store,
            &recalc_context,
            &recalc_summary,
            FormulaScenarioFamily::RetainedSumBaseline,
        );

        let mut tampered_run = store
            .read_run(&persisted.run.scenario_run_id)
            .expect("run should read for tamper setup");
        tampered_run.host_recalc_sequence += 7;
        fs::write(
            &persisted.run_path,
            serde_json::to_string_pretty(&tampered_run).expect("tampered run should serialize"),
        )
        .expect("tampered run should write");

        let error = fixture
            .adapter
            .verify_reopened_driven_scenario_run_invariants(&store, &persisted.run.scenario_run_id)
            .expect_err("tampered retained run should fail reopen invariants");

        assert!(error.contains("session_state"));
    }

    #[test]
    fn retained_runs_emit_replay_capture_outputs_and_open_them_through_oxreplay() {
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let (recalc_context, recalc_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumBaseline, 46_000.0, 0.25);
        let fixture_root = FixtureRoot::new("replay-capture");
        let store = fixture_root.retained_store();
        let persisted = fixture.persist_run(
            &store,
            &recalc_context,
            &recalc_summary,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let replay_capture = fixture.emit_replay_capture(&store, &persisted);

        assert!(replay_capture.capture_path.exists());
        assert!(replay_capture.bundle_manifest_path.exists());
        assert!(replay_capture.replay_path.exists());
        assert_eq!(
            replay_capture.capture.envelope.artifact_kind,
            "replay_capture"
        );
        assert_eq!(
            replay_capture.capture.scenario_run_ref.logical_id,
            persisted.run.scenario_run_id
        );

        let reopened_run = store
            .read_run(&persisted.run.scenario_run_id)
            .expect("run should be rewritten with replay capture ref");
        assert_eq!(
            reopened_run
                .replay_capture_ref
                .as_ref()
                .expect("replay capture ref should be set")
                .logical_id,
            replay_capture.capture.replay_capture_id
        );

        let opened = fixture
            .adapter
            .open_replay_capture(&store, &replay_capture.capture.replay_capture_id)
            .expect("replay capture should open");
        assert!(opened.replay_ready);
        assert_eq!(opened.bundle_validation_status, "valid");
        assert_eq!(opened.bundle_projection_status, "lossless");
        assert_eq!(opened.bundle_capture_loss, "none");
        assert!(opened.bundle_manifest_path.ends_with(".bundle.json"));
        assert_eq!(
            opened.replay_floor,
            "cap.C1.replay_valid (normalized_replay_open)"
        );
        assert!(opened.event_count >= 2);
        assert_eq!(opened.view_family, "normalized_replay");
        assert_eq!(
            opened.projection_source_artifact_family,
            "runtime_formula_result"
        );
        assert_eq!(
            opened.projection_phase.as_deref(),
            Some("CommittedOrRejected")
        );
        assert!(opened.projection_alias.is_none());

        let bundle_report = fixture
            .adapter
            .validate_replay_capture_bundle(&store, &replay_capture.capture.replay_capture_id)
            .expect("replay bundle should validate");
        assert_eq!(bundle_report.status, oxreplay_bundle::ValidationStatus::Valid);
        assert_eq!(
            bundle_report.bundle_id.as_deref(),
            Some(replay_capture.capture.replay_capture_id.as_str())
        );
        assert_eq!(
            bundle_report.scenario_id.as_deref(),
            Some(opened.scenario_id.as_str())
        );
    }

    #[test]
    fn replay_substrate_proving_family_tracks_reopened_runs_and_operation_boundaries() {
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let fixture_root = FixtureRoot::new("replay-substrate-proving");
        let store = fixture_root.retained_store();

        let (edit_context, edit_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumBaseline, 46_000.0, 0.25);
        let edit = fixture.persist_run(
            &store,
            &edit_context,
            &edit_summary,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let edit_capture = fixture.emit_replay_capture(&store, &edit);

        let manual_context = RecalcContext::manual(Some(46_000.0), Some(0.25));
        let manual_summary = fixture
            .adapter
            .manual_recalc(&mut fixture.host, manual_context.clone())
            .expect("manual recalc should succeed");
        let manual = fixture.persist_run(
            &store,
            &manual_context,
            &manual_summary,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let manual_capture = fixture.emit_replay_capture(&store, &manual);

        let mut reopened = fixture
            .adapter
            .reopen_driven_scenario_run(&store, &manual.run.scenario_run_id)
            .expect("manual retained run should reopen");
        let forced_context = RecalcContext::forced(Some(46_000.0), Some(0.25));
        let forced_summary = fixture
            .adapter
            .forced_recalc(&mut reopened.driven_host, forced_context.clone())
            .expect("forced recalc should succeed on reopened host");
        let forced = fixture
            .adapter
            .persist_driven_scenario_run(
                &store,
                &reopened.driven_host,
                &forced_context,
                &forced_summary,
                promoted_formula_scenario(FormulaScenarioFamily::RetainedSumBaseline).scenario_slug,
            )
            .expect("forced retained run should persist");
        let forced_capture = fixture
            .adapter
            .emit_replay_capture_for_run(&store, &forced.run.scenario_run_id)
            .expect("forced retained run should emit replay capture");

        assert_eq!(edit.run.envelope.packet_kind, "edit_accept_recalc");
        assert_eq!(manual.run.envelope.packet_kind, "manual_recalc");
        assert_eq!(forced.run.envelope.packet_kind, "forced_recalc");
        assert_eq!(manual.run.host_recalc_sequence, edit.run.host_recalc_sequence + 1);
        assert_eq!(forced.run.host_recalc_sequence, manual.run.host_recalc_sequence + 1);

        let edit_operation =
            replay_operation_for_packet_kind(&fixture.adapter, &edit.run.envelope.packet_kind);
        let manual_operation =
            replay_operation_for_packet_kind(&fixture.adapter, &manual.run.envelope.packet_kind);
        let forced_operation =
            replay_operation_for_packet_kind(&fixture.adapter, &forced.run.envelope.packet_kind);
        assert_eq!(edit_operation.operation_id, edit_summary.replay_operation_id);
        assert_eq!(
            manual_operation.operation_id,
            manual_summary.replay_operation_id
        );
        assert_eq!(
            forced_operation.operation_id,
            forced_summary.replay_operation_id
        );
        assert_eq!(
            manual_operation.retained_consequence,
            manual_summary.replay_retained_consequence
        );
        assert_eq!(
            forced_operation.retained_consequence,
            forced_summary.replay_retained_consequence
        );

        let reopened_manual = store
            .reopen_run(&manual.run.scenario_run_id)
            .expect("manual run should reopen from retained store");
        let reopened_forced = store
            .reopen_run(&forced.run.scenario_run_id)
            .expect("forced run should reopen from retained store");
        assert_eq!(reopened_manual.run.envelope.packet_kind, "manual_recalc");
        assert_eq!(reopened_forced.run.envelope.packet_kind, "forced_recalc");
        assert_eq!(
            reopened_manual
                .run
                .replay_capture_ref
                .as_ref()
                .expect("manual run should point to replay capture")
                .logical_id,
            manual_capture.capture.replay_capture_id
        );
        assert_eq!(
            reopened_forced
                .run
                .replay_capture_ref
                .as_ref()
                .expect("forced run should point to replay capture")
                .logical_id,
            forced_capture.capture.replay_capture_id
        );

        let manual_xray = fixture
            .adapter
            .open_retained_run_xray(&store, &manual.run.scenario_run_id)
            .expect("manual xray should open");
        let forced_xray = fixture
            .adapter
            .open_retained_run_xray(&store, &forced.run.scenario_run_id)
            .expect("forced xray should open");
        assert_eq!(
            manual_xray.provenance.latest_host_driving_packet_kind,
            "manual_recalc"
        );
        assert_eq!(
            forced_xray.provenance.latest_host_driving_packet_kind,
            "forced_recalc"
        );
        assert_eq!(
            manual_xray
                .trace
                .as_ref()
                .and_then(|trace| trace.replay_projection_phase.as_deref()),
            Some("CommittedOrRejected")
        );
        assert_eq!(
            forced_xray
                .trace
                .as_ref()
                .and_then(|trace| trace.replay_projection_phase.as_deref()),
            Some("CommittedOrRejected")
        );

        let replay_diff = fixture
            .adapter
            .diff_retained_run_replay_inputs(
                &store,
                &manual.run.scenario_run_id,
                &forced.run.scenario_run_id,
            )
            .expect("manual and forced retained runs should map onto replay diff inputs");
        assert!(replay_diff.equivalent);
        assert!(replay_diff.mismatches.is_empty());

        let witness = fixture
            .adapter
            .generate_retained_witness(
                &store,
                &manual.run.scenario_run_id,
                &forced.run.scenario_run_id,
            )
            .expect("replay witness should generate from reopened retained runs");
        let opened_witness = fixture
            .adapter
            .open_witness(&store, &witness.witness.witness_id)
            .expect("witness should reopen");
        assert!(opened_witness.replay_diff_equivalent);
        assert_eq!(opened_witness.replay_mismatch_count, 0);
        assert!(opened_witness
            .blocked_dimensions
            .contains(&"host_retained_diff_exceeds_current_oxreplay_diff_surface".to_string()));
        assert!(opened_witness
            .semantic_log_boundary_ids
            .contains(&"oxfml_runtime_result_plus_host_retained_artifacts".to_string()));
        assert!(opened_witness
            .seam_gaps
            .contains(&"no_cross_library_semantic_log_contract_exists".to_string()));

        let handoff = fixture
            .adapter
            .generate_handoff_packet(&store, &witness.witness.witness_id)
            .expect("handoff should generate from replay witness");
        let opened_handoff = fixture
            .adapter
            .open_handoff_packet(&store, &handoff.handoff.handoff_id)
            .expect("handoff should reopen");
        assert_eq!(opened_handoff.status, "ready");
        assert_eq!(
            opened_handoff.replay_projection_phase.as_deref(),
            Some("CommittedOrRejected")
        );
        assert!(opened_handoff
            .blocked_dimensions
            .contains(&"host_retained_diff_exceeds_current_oxreplay_diff_surface".to_string()));
        assert!(opened_handoff
            .semantic_log_boundary_ids
            .contains(&"oxfml_runtime_result_plus_host_retained_artifacts".to_string()));
        assert!(opened_handoff
            .seam_gaps
            .contains(&"no_cross_library_semantic_log_contract_exists".to_string()));

        let packets = fixture
            .adapter
            .build_replay_upstream_pressure_packets(&store)
            .expect("replay pressure packets should build from retained witness evidence");
        assert!(packets.iter().any(|packet| {
            packet.target_lane == "OxReplay"
                && packet
                    .evidence_artifact_ids
                    .contains(&manual.run.scenario_run_id)
                && packet
                    .evidence_artifact_ids
                    .contains(&forced.run.scenario_run_id)
                && packet
                    .evidence_artifact_ids
                    .contains(&witness.witness.witness_id)
        }));
        assert!(edit_capture.bundle_manifest_path.exists());
        assert!(manual_capture.bundle_manifest_path.exists());
        assert!(forced_capture.bundle_manifest_path.exists());
    }

    #[test]
    fn retained_run_xray_and_diff_surfaces_open_on_real_retained_data() {
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let fixture_root = FixtureRoot::new("xray-diff");
        let store = fixture_root.retained_store();

        let (first_context, first_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumBaseline, 46_000.0, 0.25);
        let first = fixture.persist_run(
            &store,
            &first_context,
            &first_summary,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let first_replay = fixture.emit_replay_capture(&store, &first);

        let (second_context, second_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumShifted, 46_001.0, 0.25);
        let second = fixture.persist_run(
            &store,
            &second_context,
            &second_summary,
            FormulaScenarioFamily::RetainedSumShifted,
        );
        let second_replay = fixture.emit_replay_capture(&store, &second);

        let xray = fixture
            .adapter
            .open_retained_run_xray(&store, &first.run.scenario_run_id)
            .expect("retained X-Ray should open");
        assert_eq!(
            xray.provenance.scenario_id.as_deref(),
            Some(first.scenario.scenario_id.as_str())
        );
        assert_eq!(
            xray.provenance.scenario_run_id.as_deref(),
            Some(first.run.scenario_run_id.as_str())
        );
        assert_eq!(
            xray.trace
                .as_ref()
                .and_then(|value| value.replay_capture_id.as_deref()),
            Some(first_replay.capture.replay_capture_id.as_str())
        );
        assert_eq!(
            xray.trace
                .as_ref()
                .and_then(|value| value.replay_floor.as_deref()),
            Some("cap.C1.replay_valid (normalized_replay_open)")
        );
        assert_eq!(
            xray.trace
                .as_ref()
                .and_then(|value| { value.replay_projection_source_artifact_family.as_deref() }),
            Some("runtime_formula_result")
        );
        assert_eq!(
            xray.trace
                .as_ref()
                .and_then(|value| value.replay_projection_phase.as_deref()),
            Some("CommittedOrRejected")
        );
        assert!(xray
            .trace
            .as_ref()
            .and_then(|value| value.replay_projection_alias.as_ref())
            .is_none());
        assert_eq!(
            xray.provenance.formatting_truth_plane,
            "returned_presentation_hint+host_style_state=>effective_display"
        );
        assert!(xray
            .provenance
            .conditional_formatting_scope
            .contains("Conditional Formatting: admitted="));
        assert!(xray
            .provenance
            .blocked_dimensions
            .contains(&"conditional_formatting_rules_not_attached_to_retained_run".to_string()));
        assert_eq!(
            retained_xray_golden_lines(&xray),
            vec![
                "packet_kind=edit_accept_recalc".to_string(),
                "worksheet_value=Number(6)".to_string(),
                "payload=Number".to_string(),
                "effective_display=none".to_string(),
                "formatting_truth_plane=returned_presentation_hint+host_style_state=>effective_display".to_string(),
                "conditional_formatting_scope=Conditional Formatting: admitted=fill_color|font_color|bold|italic|underline|simple_border|number_format_override|local_icon_set blocked=data_bars|two_color_scale|three_color_scale|rich_icon_sets|multi_range_priority_graph|stop_if_true_graph|workbook_global_scope".to_string(),
                "blocked_dimensions=conditional_formatting_rules_not_attached_to_retained_run".to_string(),
                "replay_floor=cap.C1.replay_valid (normalized_replay_open)".to_string(),
                "replay_projection_family=runtime_formula_result".to_string(),
                "replay_projection_phase=CommittedOrRejected".to_string(),
            ]
        );

        let diff = fixture
            .adapter
            .diff_retained_run_xray(
                &store,
                &first.run.scenario_run_id,
                &second.run.scenario_run_id,
            )
            .expect("retained diff should open");
        assert!(diff.same_scenario);
        assert!(diff.formula_text_changed);
        assert!(!diff.worksheet_value_match);
        assert!(diff.capability_snapshot_changed);
        assert!(diff.replay_pair_openable);
        assert_eq!(
            diff.formatting_truth_plane,
            "returned_presentation_hint+host_style_state=>effective_display"
        );
        assert!(diff
            .conditional_formatting_scope
            .contains("Conditional Formatting: admitted="));
        assert!(diff
            .blocked_dimensions
            .contains(&"conditional_formatting_rules_not_attached_to_retained_run".to_string()));
        assert_eq!(diff.diff_floor, "retained_artifact_direct_diff");
        assert!(!first_replay.capture.replay_capture_id.is_empty());
        assert!(!second_replay.capture.replay_capture_id.is_empty());
    }

    #[test]
    fn retained_runs_map_onto_current_oxreplay_diff_and_explain_inputs_without_local_reinterpretation(
    ) {
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let fixture_root = FixtureRoot::new("replay-contracts");
        let store = fixture_root.retained_store();

        let (left_context, left_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumBaseline, 46_000.0, 0.25);
        let left = fixture.persist_run(
            &store,
            &left_context,
            &left_summary,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        fixture.emit_replay_capture(&store, &left);

        let (right_context, right_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumShifted, 46_001.0, 0.25);
        let right = fixture.persist_run(
            &store,
            &right_context,
            &right_summary,
            FormulaScenarioFamily::RetainedSumShifted,
        );
        fixture.emit_replay_capture(&store, &right);

        let replay_diff = fixture
            .adapter
            .diff_retained_run_replay_inputs(
                &store,
                &left.run.scenario_run_id,
                &right.run.scenario_run_id,
            )
            .expect("retained runs should map onto the current oxreplay diff surface");
        assert!(replay_diff.equivalent);
        assert!(replay_diff.mismatches.is_empty());

        let explain = fixture
            .adapter
            .explain_retained_run_replay_diff(
                &store,
                &left.run.scenario_run_id,
                &right.run.scenario_run_id,
            )
            .expect("retained runs should map onto the current oxreplay explain surface");
        assert_eq!(explain.query_id, "explain-equivalent");
        assert_eq!(
            explain.summary,
            "scenarios are equivalent on the current normalized replay surface"
        );

        let retained_diff = fixture
            .adapter
            .diff_retained_run_xray(
                &store,
                &left.run.scenario_run_id,
                &right.run.scenario_run_id,
            )
            .expect("host-local retained diff should remain available");
        assert!(retained_diff.formula_text_changed);
        assert!(!retained_diff.worksheet_value_match);
    }

    #[test]
    fn retained_witness_generation_uses_real_diff_state_and_keeps_blocked_dimensions_explicit() {
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let fixture_root = FixtureRoot::new("witness");
        let store = fixture_root.retained_store();

        let (left_context, left_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumBaseline, 46_000.0, 0.25);
        let left = fixture.persist_run(
            &store,
            &left_context,
            &left_summary,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        fixture.emit_replay_capture(&store, &left);

        let (right_context, right_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumShifted, 46_001.0, 0.25);
        let right = fixture.persist_run(
            &store,
            &right_context,
            &right_summary,
            FormulaScenarioFamily::RetainedSumShifted,
        );
        fixture.emit_replay_capture(&store, &right);

        let persisted = fixture
            .adapter
            .generate_retained_witness(
                &store,
                &left.run.scenario_run_id,
                &right.run.scenario_run_id,
            )
            .expect("witness should generate");
        assert!(persisted.witness_path.exists());
        assert_eq!(persisted.witness.envelope.artifact_kind, "witness");
        assert_eq!(
            persisted.witness.left_run_ref.logical_id,
            left.run.scenario_run_id
        );
        assert_eq!(
            persisted.witness.right_run_ref.logical_id,
            right.run.scenario_run_id
        );

        let opened = fixture
            .adapter
            .open_witness(&store, &persisted.witness.witness_id)
            .expect("witness should open");
        assert_eq!(
            opened.explain_floor,
            "retained_replay_floor_plus_host_diff_summary"
        );
        assert!(opened
            .explanation_lines
            .iter()
            .any(|line| line.contains("replay_equivalent=true")));
        assert_eq!(opened.replay_explain_query_id, "explain-equivalent");
        assert_eq!(
            opened.replay_explain_summary,
            "scenarios are equivalent on the current normalized replay surface"
        );
        assert!(opened
            .explanation_lines
            .iter()
            .any(|line| line.contains("formula_text_changed=true")));
        assert!(opened
            .blocked_dimensions
            .contains(&"distill_not_integrated".to_string()));
        assert!(opened
            .blocked_dimensions
            .contains(&"replay_explain_surface_is_summary_only".to_string()));
        assert!(opened
            .blocked_dimensions
            .contains(&"host_retained_diff_exceeds_current_oxreplay_diff_surface".to_string()));
        assert!(opened
            .semantic_log_boundary_ids
            .contains(&"oxfml_runtime_result_plus_host_retained_artifacts".to_string()));
        assert!(opened
            .seam_gaps
            .contains(&"no_cross_library_semantic_log_contract_exists".to_string()));
        assert!(opened.replay_projection_aliases.is_empty());
    }

    #[test]
    fn handoff_packets_are_generated_from_retained_evidence_and_gated_by_capability_truth() {
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let fixture_root = FixtureRoot::new("handoff");
        let store = fixture_root.retained_store();

        let (left_context, left_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumBaseline, 46_000.0, 0.25);
        let left = fixture.persist_run(
            &store,
            &left_context,
            &left_summary,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        fixture.emit_replay_capture(&store, &left);

        let (right_context, right_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumShifted, 46_001.0, 0.25);
        let right = fixture.persist_run(
            &store,
            &right_context,
            &right_summary,
            FormulaScenarioFamily::RetainedSumShifted,
        );
        fixture.emit_replay_capture(&store, &right);

        let witness = fixture
            .adapter
            .generate_retained_witness(
                &store,
                &left.run.scenario_run_id,
                &right.run.scenario_run_id,
            )
            .expect("witness should generate");
        let handoff = fixture
            .adapter
            .generate_handoff_packet(&store, &witness.witness.witness_id)
            .expect("handoff should generate");
        assert!(handoff.handoff_path.exists());
        assert_eq!(handoff.handoff.envelope.artifact_kind, "handoff_packet");
        assert_eq!(handoff.handoff.status, "ready");

        let opened = fixture
            .adapter
            .open_handoff_packet(&store, &handoff.handoff.handoff_id)
            .expect("handoff should open");
        assert_eq!(opened.target_lane, "OxReplay/DnaOneCalc");
        assert_eq!(opened.requested_action_kind, "clarify_contract");
        assert_eq!(opened.status, "ready");
        assert!(opened.readiness.iter().all(|item| item.satisfied));
        assert_eq!(
            opened.capability_snapshot_id,
            left.run
                .envelope
                .capability_snapshot_ref
                .as_ref()
                .expect("left run should have capability snapshot")
                .logical_id
        );
        assert!(opened.replay_projection_alias.is_none());
        assert_eq!(opened.replay_explain_query_id, "explain-equivalent");
        assert_eq!(
            opened.replay_explain_summary,
            "scenarios are equivalent on the current normalized replay surface"
        );
        assert!(opened
            .blocked_dimensions
            .contains(&"host_retained_diff_exceeds_current_oxreplay_diff_surface".to_string()));
        assert!(opened
            .semantic_log_boundary_ids
            .contains(&"oxfml_runtime_result_plus_host_retained_artifacts".to_string()));
        assert!(opened
            .seam_gaps
            .contains(&"no_cross_library_semantic_log_contract_exists".to_string()));
        assert_eq!(
            opened.replay_projection_phase.as_deref(),
            Some("CommittedOrRejected")
        );
    }

    #[test]
    fn replay_witnesses_generate_structured_upstream_pressure_packets_for_seam_gaps() {
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let fixture_root = FixtureRoot::new("replay-upstream-pressure");
        let store = fixture_root.retained_store();

        let (left_context, left_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumBaseline, 46_000.0, 0.25);
        let left = fixture.persist_run(
            &store,
            &left_context,
            &left_summary,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let left_replay = fixture.emit_replay_capture(&store, &left);

        let (right_context, right_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumShifted, 46_001.0, 0.25);
        let right = fixture.persist_run(
            &store,
            &right_context,
            &right_summary,
            FormulaScenarioFamily::RetainedSumShifted,
        );
        let right_replay = fixture.emit_replay_capture(&store, &right);

        fixture
            .adapter
            .generate_retained_witness(
                &store,
                &left.run.scenario_run_id,
                &right.run.scenario_run_id,
            )
            .expect("witness should generate");

        let packets = fixture
            .adapter
            .build_replay_upstream_pressure_packets(&store)
            .expect("replay pressure packets should build");

        assert_eq!(packets.len(), 2);
        let oxreplay = packets
            .iter()
            .find(|packet| packet.target_lane == "OxReplay")
            .expect("oxreplay pressure packet should exist");
        assert_eq!(oxreplay.packet_kind, "replay_seam_gap");
        assert!(oxreplay
            .blocker_ids
            .contains(&"host_retained_diff_exceeds_current_oxreplay_diff_surface".to_string()));
        assert!(oxreplay
            .blocker_ids
            .contains(&"replay_owned_retained_artifact_contract_not_defined".to_string()));
        assert!(oxreplay
            .evidence_artifact_ids
            .contains(&left.run.scenario_run_id));
        assert!(oxreplay
            .evidence_artifact_ids
            .contains(&right.run.scenario_run_id));
        assert!(oxreplay
            .evidence_artifact_ids
            .contains(&left_replay.capture.replay_capture_id));
        assert!(oxreplay
            .evidence_artifact_ids
            .contains(&right_replay.capture.replay_capture_id));

        let oxfunc = packets
            .iter()
            .find(|packet| packet.target_lane == "OxFunc")
            .expect("oxfunc pressure packet should exist");
        assert_eq!(oxfunc.packet_kind, "replay_seam_gap");
        assert_eq!(
            oxfunc.blocker_ids,
            vec!["oxfunc_direct_replay_intake_not_admitted".to_string()]
        );
        assert!(oxfunc.summary.contains("OxFunc"));
    }

    #[test]
    fn runtime_adapter_emits_capability_snapshot_from_executable_truth() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let snapshot = adapter
            .emit_capability_snapshot("edit_accept_recalc", None)
            .expect("capability snapshot should emit");

        assert_eq!(
            snapshot.envelope.artifact_kind,
            "capability_ledger_snapshot"
        );
        assert_eq!(snapshot.host_kind, "dnaonecalc-host");
        assert_eq!(snapshot.capability_floor, "OC-H1");
        assert!(snapshot
            .packet_kind_register
            .contains(&"edit_accept_recalc".to_string()));
        assert!(snapshot
            .dependency_set
            .contains(&"oxreplay_core".to_string()));
        assert!(snapshot
            .dependency_set
            .contains(&"oxreplay_abstractions".to_string()));
        assert!(snapshot
            .mode_availability
            .iter()
            .any(|mode| mode.mode_id == "DNA-only" && mode.state == "available"));
        assert!(snapshot
            .mode_availability
            .iter()
            .any(|mode| mode.mode_id == "Excel-observed" && mode.state == "available"));
        assert!(snapshot
            .mode_availability
            .iter()
            .any(|mode| mode.mode_id == "Twin compare" && mode.state == "available"));
        assert!(snapshot
            .mode_availability
            .iter()
            .any(|mode| mode.mode_id == "Replay" && mode.state == "available"));
        assert!(snapshot
            .mode_availability
            .iter()
            .any(|mode| mode.mode_id == "Diff" && mode.state == "available"));
        assert!(!snapshot
            .provisional_seams
            .contains(&"observation_path_not_integrated".to_string()));
    }

    #[test]
    fn capability_snapshot_open_and_diff_read_persisted_immutable_truth() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-capability-diff-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(&root);

        let left = adapter
            .persist_capability_snapshot(&store, "formula_edit", None)
            .expect("left snapshot should persist");
        let right = adapter
            .persist_capability_snapshot(
                &store,
                "edit_accept_recalc",
                Some(&left.snapshot.capability_snapshot_id),
            )
            .expect("right snapshot should persist");

        let opened = adapter
            .open_capability_snapshot(&store, &right.snapshot.capability_snapshot_id)
            .expect("right snapshot should open");
        let diff = adapter
            .diff_capability_snapshots(
                &store,
                &left.snapshot.capability_snapshot_id,
                &right.snapshot.capability_snapshot_id,
            )
            .expect("snapshot diff should open");

        assert_eq!(
            opened.diff_base_snapshot_id,
            Some(left.snapshot.capability_snapshot_id.clone())
        );
        assert!(opened
            .mode_availability
            .iter()
            .any(|mode| mode.mode_id == "Replay"));
        assert!(diff.packet_kinds_added.is_empty());
        assert!(!diff.function_surface_policy_changed);
        assert_eq!(diff.diff_floor, "immutable_capability_snapshot_diff");
        assert_eq!(
            capability_snapshot_golden_lines(&opened, &diff),
            vec![
                "host_kind=dnaonecalc-host".to_string(),
                "runtime_class=desktop_native_only".to_string(),
                "capability_floor=OC-H1".to_string(),
                "packet_kind_register=formula_edit|edit_accept_recalc|manual_recalc|forced_recalc|replay_capture|observation_capture".to_string(),
                "mode_availability=DNA-only:available|Excel-observed:available|Twin compare:available|Replay:available|Diff:available|Explain:available|Distill:blocked|Handoff:available".to_string(),
                "function_surface_policy=onecalc:admitted_execution:supported=517::preview=0::experimental=0::deferred=17::catalog_only=0".to_string(),
                format!("diff_base={}", left.snapshot.capability_snapshot_id),
                "diff_floor=immutable_capability_snapshot_diff".to_string(),
                "diff_mode_changes=".to_string(),
            ]
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn observation_artifact_persists_from_upstream_source_bundle() {
        let adapter = adapter_for(OneCalcHostProfile::OcH1);
        let fixture_root = FixtureRoot::new("observation");
        let store = fixture_root.retained_store();
        let persisted = persist_observation_fixture(
            &adapter,
            &store,
            ObservationScenarioFamily::XlPlayCaptureValuesFormulae001,
        );
        let scenario = promoted_observation_scenario(
            ObservationScenarioFamily::XlPlayCaptureValuesFormulae001,
        );

        assert!(persisted.observation_path.exists());
        assert_eq!(persisted.observation.envelope.artifact_kind, "observation");
        assert_eq!(
            persisted.observation.capture.surfaces[0].surface.surface_id,
            scenario.expected_value_surface_id
        );
        assert_eq!(
            persisted.observation.source_schema_id,
            "oxxlplay.replay_bundle_seed.v1"
        );
        assert_eq!(
            persisted.observation.capture_mode,
            "excel_black_box_observation"
        );
        assert_eq!(persisted.observation.projection_status, "lossy");
        assert_eq!(
            persisted.observation.replay_bundle_id.as_deref(),
            Some("oxxlplay-xlplay_capture_values_formulae_001")
        );
        assert_eq!(
            persisted
                .observation
                .replay_manifest_validation_status
                .as_deref(),
            Some("valid")
        );
        assert_eq!(
            persisted.observation.replay_capture_loss_status.as_deref(),
            Some("none")
        );
        assert!(persisted
            .observation
            .lossiness
            .contains(&"normalized_replay_projection_is_lossy".to_string()));
        assert!(persisted.observation.source_artifact_ref.content_hash.is_some());
        assert!(persisted.observation.provenance_ref.content_hash.is_some());
        assert!(persisted.observation.capture_loss_ref.content_hash.is_some());
        assert!(persisted
            .observation
            .replay_manifest_ref
            .as_ref()
            .and_then(|artifact| artifact.content_hash.as_ref())
            .is_some());
        assert!(persisted
            .observation
            .normalized_replay_ref
            .as_ref()
            .and_then(|artifact| artifact.content_hash.as_ref())
            .is_some());
        assert_eq!(persisted.observation.envelope.lineage_refs.len(), 5);
        assert!(persisted
            .observation
            .envelope
            .lineage_refs
            .iter()
            .any(|entry| entry.relation == "replay_manifest"));
        assert!(persisted
            .observation
            .envelope
            .lineage_refs
            .iter()
            .any(|entry| entry.relation == "normalized_replay"));
        assert_eq!(
            persisted.observation.provenance.bridge.bridge_kind,
            "external_process"
        );

        let reopened = store
            .read_observation(&persisted.observation.observation_id)
            .expect("observation artifact should reopen");
        assert_eq!(reopened.scenario_id, scenario.expected_scenario_id);
        assert_eq!(reopened.projection_status, "lossy");
        assert_eq!(
            reopened.replay_manifest_validation_status.as_deref(),
            Some("valid")
        );
    }

    #[test]
    fn twin_compare_artifact_persists_and_opens_on_real_run_and_observation() {
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::ObservationCompareSum,
        );
        let fixture_root = FixtureRoot::new("twin-compare");
        let store = fixture_root.retained_store();
        let (context, summary) =
            fixture.edit_accept(FormulaScenarioFamily::ObservationCompareSum, 46_000.0, 0.25);
        let retained = fixture.persist_run(
            &store,
            &context,
            &summary,
            FormulaScenarioFamily::ObservationCompareSum,
        );
        let observation = persist_observation_fixture(
            &fixture.adapter,
            &store,
            ObservationScenarioFamily::XlPlayCaptureValuesFormulae001,
        );

        let comparison = fixture
            .adapter
            .compare_run_with_observation(
                &store,
                &retained.run.scenario_run_id,
                &observation.observation.observation_id,
            )
            .expect("comparison should persist");
        let opened = fixture
            .adapter
            .open_twin_compare(&store, &comparison.comparison.comparison_id)
            .expect("compare view should open");

        assert_eq!(opened.reliability_badge, "direct");
        assert_eq!(
            opened.comparison_envelope,
            vec!["worksheet_value".to_string(), "formula_text".to_string()]
        );
        assert!(opened
            .mismatch_lines
            .iter()
            .any(|line| line.contains("worksheet_value:match")));
        assert!(opened
            .mismatch_lines
            .iter()
            .any(|line| line.contains("formula_text:mismatch")));
        assert_eq!(
            twin_compare_golden_lines(&opened),
            vec![
                "reliability=direct".to_string(),
                "envelope=worksheet_value|formula_text".to_string(),
                "mismatches=worksheet_value:match:Number(42):Number(42):observation surface status=direct capture_loss=none|formula_text:mismatch:=SUM(10,20,12):=SUM(B1:B3):observation surface status=direct capture_loss=none".to_string(),
                "projection_limitations=display_state_not_in_current_observation_envelope|formatting_not_in_current_observation_envelope|conditional_formatting_not_in_current_observation_envelope".to_string(),
            ]
        );
    }

    #[test]
    fn compare_regression_family_uses_retained_oxxlplay_fixtures_and_keeps_live_capture_gate_explicit(
    ) {
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::ObservationCompareSum,
        );
        let fixture_root = FixtureRoot::new("compare-regression-family");
        let store = fixture_root.retained_store();
        let (context, summary) =
            fixture.edit_accept(FormulaScenarioFamily::ObservationCompareSum, 46_000.0, 0.25);
        let retained = fixture.persist_run(
            &store,
            &context,
            &summary,
            FormulaScenarioFamily::ObservationCompareSum,
        );
        let observation = persist_observation_fixture(
            &fixture.adapter,
            &store,
            ObservationScenarioFamily::XlPlayCaptureValuesFormulae001,
        );

        let comparison = fixture
            .adapter
            .compare_run_with_observation(
                &store,
                &retained.run.scenario_run_id,
                &observation.observation.observation_id,
            )
            .expect("comparison should persist from retained OxXlPlay fixtures");
        let opened_compare = fixture
            .adapter
            .open_twin_compare(&store, &comparison.comparison.comparison_id)
            .expect("compare view should open");
        let widening_handoff = fixture
            .adapter
            .generate_observation_widening_handoff(&store, &comparison.comparison.comparison_id)
            .expect("widening handoff should persist");
        let opened_handoff = fixture
            .adapter
            .open_handoff_packet(&store, &widening_handoff.handoff.handoff_id)
            .expect("handoff should open");

        assert_eq!(opened_compare.reliability_badge, "direct");
        assert_eq!(
            opened_compare.observation_id,
            observation.observation.observation_id
        );
        assert!(opened_compare
            .projection_limitations
            .contains(&"display_state_not_in_current_observation_envelope".to_string()));
        assert_eq!(opened_handoff.target_lane, "OxXlPlay/DnaOneCalc");
        assert_eq!(opened_handoff.requested_action_kind, "widen_observation_envelope");
        assert_eq!(opened_handoff.status, "ready");

        if cfg!(windows) {
            assert_eq!(
                fixture
                    .adapter
                    .live_windows_capture_gate()
                    .expect("windows hosts should admit live capture gating"),
                "live_windows_capture_admitted"
            );
        } else {
            let error = fixture
                .adapter
                .live_windows_capture_gate()
                .expect_err("non-Windows hosts should keep live capture blocked");
            assert!(error.contains("Windows-only"));
            assert!(error.contains("retained OxXlPlay fixtures"));
        }
    }

    #[test]
    fn widening_request_handoff_emits_from_real_compare_state() {
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::ObservationCompareSum,
        );
        let fixture_root = FixtureRoot::new("widening");
        let store = fixture_root.retained_store();
        let (context, summary) =
            fixture.edit_accept(FormulaScenarioFamily::ObservationCompareSum, 46_000.0, 0.25);
        let retained = fixture.persist_run(
            &store,
            &context,
            &summary,
            FormulaScenarioFamily::ObservationCompareSum,
        );
        let observation = persist_observation_fixture(
            &fixture.adapter,
            &store,
            ObservationScenarioFamily::XlPlayCaptureValuesFormulae001,
        );
        let comparison = fixture
            .adapter
            .compare_run_with_observation(
                &store,
                &retained.run.scenario_run_id,
                &observation.observation.observation_id,
            )
            .expect("comparison should persist");

        let handoff = fixture
            .adapter
            .generate_observation_widening_handoff(&store, &comparison.comparison.comparison_id)
            .expect("widening handoff should persist");
        let opened = fixture
            .adapter
            .open_handoff_packet(&store, &handoff.handoff.handoff_id)
            .expect("handoff should open");

        assert_eq!(opened.target_lane, "OxXlPlay/DnaOneCalc");
        assert_eq!(opened.requested_action_kind, "widen_observation_envelope");
        assert_eq!(opened.status, "ready");
    }

    #[test]
    fn retained_h1_runs_compare_version_to_version_in_code() {
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let fixture_root = FixtureRoot::new("h1-compare");
        let store = fixture_root.retained_store();

        let (first_context, first_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumBaseline, 46_000.0, 0.25);
        let first = fixture.persist_run(
            &store,
            &first_context,
            &first_summary,
            FormulaScenarioFamily::RetainedSumBaseline,
        );

        let (second_context, second_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumShifted, 46_001.0, 0.25);
        let second = fixture.persist_run(
            &store,
            &second_context,
            &second_summary,
            FormulaScenarioFamily::RetainedSumShifted,
        );

        let comparison = fixture
            .adapter
            .compare_retained_driven_runs(
                &store,
                &first.run.scenario_run_id,
                &second.run.scenario_run_id,
            )
            .expect("retained driven runs should compare");

        assert!(comparison.same_scenario);
        assert!(comparison.formula_version_changed);
        assert!(comparison.formula_text_changed);
        assert!(!comparison.worksheet_value_match);
        assert_eq!(comparison.reliability_badge, "direct");
    }

    #[test]
    fn spreadsheetml_document_round_trip_reopens_into_the_h1_host() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1.document", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let recalc_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let recalc_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", recalc_context)
            .expect("document recalc should succeed");

        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-document-roundtrip-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(root.join("retained"));
        let persisted_run = adapter
            .persist_driven_scenario_run(
                &store,
                &host,
                &recalc_context,
                &recalc_summary,
                "SUM document",
            )
            .expect("retained run should persist");
        let document_path = root.join("sum-document.xml");
        let persisted_document = adapter
            .persist_isolated_document(
                &document_path,
                &host,
                &recalc_context,
                &recalc_summary,
                "SUM document",
                Some(&persisted_run),
            )
            .expect("isolated document should persist");

        assert!(persisted_document.document_path.exists());
        assert_eq!(
            persisted_document.document.document_scope,
            "isolated_single_formula_instance"
        );
        assert_eq!(
            persisted_document.document.persistence_format_id,
            "spreadsheetml2003.onecalc.single_instance.v1"
        );
        assert_eq!(persisted_document.document.artifact_index.len(), 3);

        let mut reopened = adapter
            .reopen_isolated_document(&persisted_document.document_path)
            .expect("isolated document should reopen");
        assert_eq!(
            reopened.document.formula_stable_id,
            persisted_document.document.formula_stable_id
        );
        assert_eq!(
            reopened.driven_host.formula_text(),
            persisted_document.document.formula_text
        );
        assert_eq!(
            reopened.driven_host.formula_text_version(),
            persisted_document.document.formula_text_version
        );
        assert_eq!(
            reopened.driven_host.session_state().session_id,
            persisted_document.document.host_session_id
        );
        assert_eq!(
            reopened.driven_host.session_state().recalc_sequence,
            persisted_document.document.host_recalc_sequence
        );

        let reopened_summary = adapter
            .manual_recalc(
                &mut reopened.driven_host,
                RecalcContext::manual(Some(46_000.0), Some(0.25)),
            )
            .expect("reopened document should recalc");
        assert_eq!(reopened_summary.host_profile_id, "OC-H1");
        assert_eq!(
            reopened_summary.host_session_id,
            persisted_document.document.host_session_id
        );
        assert_eq!(
            reopened_summary.host_recalc_sequence,
            persisted_document.document.host_recalc_sequence + 1
        );
        assert_eq!(
            reopened_summary.evaluation.worksheet_value_summary,
            "Number(6)"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn spreadsheetml_document_round_trip_preserves_identity_and_current_formatting_invariants() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1.document.invariants", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let recalc_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let recalc_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", recalc_context)
            .expect("document recalc should succeed");

        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-document-invariants-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(root.join("retained"));
        let persisted_run = adapter
            .persist_driven_scenario_run(
                &store,
                &host,
                &recalc_context,
                &recalc_summary,
                "SUM document invariant",
            )
            .expect("retained run should persist");
        let persisted_document = adapter
            .persist_isolated_document(
                root.join("sum-document-invariants.xml"),
                &host,
                &recalc_context,
                &recalc_summary,
                "SUM document invariant",
                Some(&persisted_run),
            )
            .expect("isolated document should persist");

        let invariants = adapter
            .verify_isolated_document_roundtrip_invariants(&persisted_document)
            .expect("document invariants should survive round-trip");

        assert!(invariants.document_id_preserved);
        assert!(invariants.formula_identity_preserved);
        assert!(invariants.structure_context_preserved);
        assert!(invariants.session_state_preserved);
        assert!(invariants.library_context_snapshot_ref_preserved);
        assert!(invariants.governing_capability_snapshot_preserved);
        assert!(invariants.artifact_index_preserved);
        assert!(invariants.effective_display_status_preserved);
        assert_eq!(
            document_roundtrip_golden_lines(&persisted_document, &invariants),
            vec![
                "scope=isolated_single_formula_instance".to_string(),
                "format=spreadsheetml2003.onecalc.single_instance.v1".to_string(),
                "host_profile=OC-H1".to_string(),
                "scenario_slug=sum-document-invariant".to_string(),
                format!(
                    "host_session_id={}",
                    persisted_document.document.host_session_id
                ),
                "host_recalc_sequence=1".to_string(),
                format!(
                    "governing_capability_snapshot={}",
                    persisted_document
                        .document
                        .governing_capability_snapshot_id
                        .as_deref()
                        .unwrap_or("none")
                ),
                "packet_kind=edit_accept_recalc".to_string(),
                "effective_display=none".to_string(),
                "artifact_index_count=3".to_string(),
                "artifact_kinds=scenario|scenario_run|capability_ledger_snapshot".to_string(),
                "invariants=document_id:true;formula_identity:true;structure_context:true;session_state:true;library_context_snapshot_ref:true;governing_capability_snapshot:true;artifact_index:true;effective_display_status:true".to_string(),
            ]
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn scenario_capsule_export_and_intake_preserve_lineage_and_capability_refs() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let mut host = adapter
            .new_driven_single_formula_host("onecalc.h1.capsule", "=SUM(1,2,3)")
            .expect("OC-H1 should admit the driven host model");
        let recalc_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let recalc_summary = adapter
            .edit_accept_recalc(&mut host, "=SUM(1,2,3)", recalc_context)
            .expect("capsule source recalc should succeed");

        let root = std::env::temp_dir().join(format!(
            "dnaonecalc-scenario-capsule-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let export_store = RetainedScenarioStore::new(root.join("retained-export"));
        let import_store = RetainedScenarioStore::new(root.join("retained-import"));
        let persisted_run = adapter
            .persist_driven_scenario_run(
                &export_store,
                &host,
                &recalc_context,
                &recalc_summary,
                "SUM capsule",
            )
            .expect("retained run should persist");
        let replay_capture = adapter
            .emit_replay_capture_for_run(&export_store, &persisted_run.run.scenario_run_id)
            .expect("capsule replay capture should emit");
        let exported = adapter
            .export_scenario_capsule(
                &export_store,
                root.join("capsule"),
                &[&persisted_run.run.scenario_run_id],
            )
            .expect("ScenarioCapsule export should succeed");

        assert_eq!(
            exported.manifest.root_scenario_id,
            persisted_run.scenario.scenario_id
        );
        assert_eq!(exported.manifest.lineage_roots.len(), 1);
        assert_eq!(exported.manifest.capability_snapshot_refs.len(), 1);
        assert_eq!(exported.manifest.included_artifacts.len(), 4);
        assert_eq!(exported.manifest.included_attachments.len(), 1);
        assert_eq!(
            exported.manifest.capability_snapshot_refs[0].logical_id,
            persisted_run
                .capability_snapshot
                .snapshot
                .capability_snapshot_id
        );
        assert_eq!(
            exported.manifest.included_attachments[0].attachment_ref,
            replay_capture.capture.replay_capture_id
        );

        let imported = adapter
            .import_scenario_capsule(&import_store, &exported.capsule_root)
            .expect("ScenarioCapsule intake should succeed");

        assert_eq!(imported.imported_paths.len(), 5);
        assert!(imported.deduped_paths.is_empty());
        assert!(imported.conflict_paths.is_empty());
        assert!(imported.manifest_copy_path.exists());

        let imported_run_body = fs::read_to_string(
            import_store
                .root()
                .join("imports")
                .join("scenario-runs")
                .join(format!("{}.json", persisted_run.run.scenario_run_id)),
        )
        .expect("imported run should exist");
        let imported_run: ScenarioRunRecord =
            serde_json::from_str(&imported_run_body).expect("imported run should deserialize");
        assert_eq!(
            imported_run
                .envelope
                .capability_snapshot_ref
                .as_ref()
                .expect("imported run should preserve capability snapshot ref")
                .logical_id,
            persisted_run
                .capability_snapshot
                .snapshot
                .capability_snapshot_id
        );
        assert_eq!(imported_run.envelope.lineage_refs.len(), 1);
        assert_eq!(
            imported_run.envelope.lineage_refs[0]
                .artifact_ref
                .logical_id,
            persisted_run.scenario.scenario_id
        );
        assert!(import_store
            .root()
            .join("imports")
            .join("replay-captures")
            .join(format!("{}.json", replay_capture.capture.replay_capture_id))
            .exists());
        assert!(import_store
            .root()
            .join("imports")
            .join("replay-captures")
            .join(format!(
                "{}.replay.json",
                replay_capture.capture.replay_capture_id
            ))
            .exists());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn workspace_manifest_groups_multiple_isolated_documents_without_merging_them() {
        let adapter = RuntimeAdapter::new(OneCalcHostProfile::OcH1);
        let root =
            std::env::temp_dir().join(format!("dnaonecalc-workspace-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let store = RetainedScenarioStore::new(root.join("retained"));

        let mut first_host = adapter
            .new_driven_single_formula_host("onecalc.h1.workspace.left", "=SUM(1,2,3)")
            .expect("first workspace host should initialize");
        let first_context = RecalcContext::edit_accept(Some(46_000.0), Some(0.25));
        let first_summary = adapter
            .edit_accept_recalc(&mut first_host, "=SUM(1,2,3)", first_context)
            .expect("first workspace recalc should succeed");
        let first_retained = adapter
            .persist_driven_scenario_run(
                &store,
                &first_host,
                &first_context,
                &first_summary,
                "SUM workspace left",
            )
            .expect("first retained run should persist");
        let first_document = adapter
            .persist_isolated_document(
                root.join("left.xml"),
                &first_host,
                &first_context,
                &first_summary,
                "SUM workspace left",
                Some(&first_retained),
            )
            .expect("first document should persist");

        let mut second_host = adapter
            .new_driven_single_formula_host("onecalc.h1.workspace.right", "=SUM(1,2,4)")
            .expect("second workspace host should initialize");
        let second_context = RecalcContext::edit_accept(Some(46_001.0), Some(0.25));
        let second_summary = adapter
            .edit_accept_recalc(&mut second_host, "=SUM(1,2,4)", second_context)
            .expect("second workspace recalc should succeed");
        let second_retained = adapter
            .persist_driven_scenario_run(
                &store,
                &second_host,
                &second_context,
                &second_summary,
                "SUM workspace right",
            )
            .expect("second retained run should persist");
        let second_document = adapter
            .persist_isolated_document(
                root.join("right.xml"),
                &second_host,
                &second_context,
                &second_summary,
                "SUM workspace right",
                Some(&second_retained),
            )
            .expect("second document should persist");

        let persisted = adapter
            .persist_workspace_manifest(
                root.join("workspace.onecalc.json"),
                "Workspace Test",
                &[
                    &first_document.document_path,
                    &second_document.document_path,
                ],
            )
            .expect("workspace manifest should persist");
        let opened = adapter
            .open_workspace(&persisted.manifest_path)
            .expect("workspace should reopen");

        assert_eq!(opened.reopened_documents.len(), 2);
        assert_eq!(
            opened.manifest.active_document_id,
            first_document.document.document_id
        );
        assert_eq!(
            opened.manifest.document_entries[0].governing_capability_snapshot_id,
            first_document.document.governing_capability_snapshot_id
        );
        assert_eq!(
            opened.manifest.document_entries[1].governing_capability_snapshot_id,
            second_document.document.governing_capability_snapshot_id
        );
        assert_ne!(
            opened.reopened_documents[0].document.document_id,
            opened.reopened_documents[1].document.document_id
        );
        assert_eq!(
            opened.reopened_documents[0].document.formula_text,
            "=SUM(1,2,3)"
        );
        assert_eq!(
            opened.reopened_documents[1].document.formula_text,
            "=SUM(1,2,4)"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn retained_host_integration_family_carries_one_scenario_across_recalc_reopen_persistence_and_capsule_transport(
    ) {
        let fixture_root = FixtureRoot::new("retained-host-integration");
        let store = fixture_root.retained_store();
        let import_store = RetainedScenarioStore::new(fixture_root.join("retained-import"));
        let mut fixture = DrivenHostFixture::new(
            OneCalcHostProfile::OcH1,
            FormulaScenarioFamily::RetainedSumBaseline,
        );

        let (recalc_context, recalc_summary) =
            fixture.edit_accept(FormulaScenarioFamily::RetainedSumBaseline, 46_000.0, 0.25);
        assert_eq!(recalc_summary.evaluation.worksheet_value_summary, "Number(6)");
        assert_eq!(recalc_summary.host_recalc_sequence, 1);

        let persisted_run = fixture.persist_run(
            &store,
            &recalc_context,
            &recalc_summary,
            FormulaScenarioFamily::RetainedSumBaseline,
        );
        let replay_capture = fixture.emit_replay_capture(&store, &persisted_run);

        let mut reopened_run = fixture
            .adapter
            .reopen_driven_scenario_run(&store, &persisted_run.run.scenario_run_id)
            .expect("retained run should reopen in the promoted integration flow");
        assert_eq!(
            reopened_run.driven_host.session_state().session_id,
            persisted_run.run.host_session_id
        );
        assert_eq!(
            reopened_run.driven_host.session_state().recalc_sequence,
            persisted_run.run.host_recalc_sequence
        );

        let reopened_context = RecalcContext::manual(Some(46_000.0), Some(0.25));
        let reopened_summary = fixture
            .adapter
            .manual_recalc(&mut reopened_run.driven_host, reopened_context)
            .expect("reopened retained run should recalc");
        assert_eq!(reopened_summary.evaluation.worksheet_value_summary, "Number(6)");
        assert_eq!(
            reopened_summary.host_recalc_sequence,
            persisted_run.run.host_recalc_sequence + 1
        );

        let document_path = fixture_root.join("retained-integration.onecalc.xml");
        let persisted_document = fixture
            .adapter
            .persist_isolated_document(
                &document_path,
                &reopened_run.driven_host,
                &reopened_context,
                &reopened_summary,
                "SUM retained integration",
                Some(&persisted_run),
            )
            .expect("reopened retained run should persist as an isolated document");
        assert_eq!(
            persisted_document.document.governing_capability_snapshot_id,
            Some(
                persisted_run
                    .capability_snapshot
                    .snapshot
                    .capability_snapshot_id
                    .clone()
            )
        );

        let mut reopened_document = fixture
            .adapter
            .reopen_isolated_document(&persisted_document.document_path)
            .expect("isolated document should reopen");
        assert_eq!(
            reopened_document.driven_host.formula_text(),
            persisted_run.scenario.formula_text
        );
        assert_eq!(
            reopened_document.driven_host.session_state().session_id,
            persisted_document.document.host_session_id
        );

        let reopened_document_context = RecalcContext::manual(Some(46_000.0), Some(0.25));
        let reopened_document_summary = fixture
            .adapter
            .manual_recalc(
                &mut reopened_document.driven_host,
                reopened_document_context,
            )
            .expect("reopened document should recalc");
        assert_eq!(
            reopened_document_summary.evaluation.worksheet_value_summary,
            "Number(6)"
        );
        assert_eq!(
            reopened_document_summary.host_recalc_sequence,
            persisted_document.document.host_recalc_sequence + 1
        );

        let persisted_workspace = fixture
            .adapter
            .persist_workspace_manifest(
                fixture_root.join("retained-integration.workspace.json"),
                "Retained Integration Workspace",
                &[persisted_document.document_path.clone()],
            )
            .expect("workspace manifest should persist");
        let opened_workspace = fixture
            .adapter
            .open_workspace(&persisted_workspace.manifest_path)
            .expect("workspace manifest should reopen");
        assert_eq!(opened_workspace.reopened_documents.len(), 1);
        assert_eq!(
            opened_workspace.reopened_documents[0].document.document_id,
            persisted_document.document.document_id
        );
        assert_eq!(
            opened_workspace.reopened_documents[0]
                .document
                .governing_capability_snapshot_id,
            persisted_document.document.governing_capability_snapshot_id
        );

        let exported_capsule = fixture
            .adapter
            .export_scenario_capsule(
                &store,
                fixture_root.join("retained-integration-capsule"),
                &[&persisted_run.run.scenario_run_id],
            )
            .expect("scenario capsule export should succeed");
        let imported_capsule = fixture
            .adapter
            .import_scenario_capsule(&import_store, &exported_capsule.capsule_root)
            .expect("scenario capsule intake should succeed");

        assert_eq!(exported_capsule.manifest.included_artifacts.len(), 4);
        assert_eq!(exported_capsule.manifest.included_attachments.len(), 1);
        assert_eq!(imported_capsule.imported_paths.len(), 5);
        assert!(imported_capsule.deduped_paths.is_empty());
        assert!(imported_capsule.conflict_paths.is_empty());

        let imported_run_body = fs::read_to_string(
            import_store
                .root()
                .join("imports")
                .join("scenario-runs")
                .join(format!("{}.json", persisted_run.run.scenario_run_id)),
        )
        .expect("capsule intake should copy the retained run");
        let imported_run: ScenarioRunRecord =
            serde_json::from_str(&imported_run_body).expect("imported run should deserialize");
        assert_eq!(imported_run.scenario_id, persisted_run.run.scenario_id);
        assert_eq!(
            imported_run
                .envelope
                .capability_snapshot_ref
                .as_ref()
                .expect("imported run should preserve governing capability snapshot")
                .logical_id,
            persisted_run
                .capability_snapshot
                .snapshot
                .capability_snapshot_id
        );
        assert!(import_store
            .root()
            .join("imports")
            .join("replay-captures")
            .join(format!("{}.json", replay_capture.capture.replay_capture_id))
            .exists());
        assert!(import_store
            .root()
            .join("imports")
            .join("replay-captures")
            .join(format!(
                "{}.replay.json",
                replay_capture.capture.replay_capture_id
            ))
            .exists());
    }
}
