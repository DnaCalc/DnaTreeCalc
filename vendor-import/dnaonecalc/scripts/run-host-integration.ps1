$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

Push-Location (Join-Path $PSScriptRoot "..")
try {
    $checks = @(
        "tests::retained_host_integration_family_carries_one_scenario_across_recalc_reopen_persistence_and_capsule_transport"
        "tests::h1_runs_persist_scenario_and_scenario_run_and_reopen_through_runtime"
        "tests::retained_run_xray_and_diff_surfaces_open_on_real_retained_data"
        "tests::retained_runs_map_onto_current_oxreplay_diff_and_explain_inputs_without_local_reinterpretation"
        "tests::replay_substrate_proving_family_tracks_reopened_runs_and_operation_boundaries"
        "tests::semantic_logging_boundary_model_makes_owner_split_and_seam_gaps_explicit"
        "tests::replay_witnesses_generate_structured_upstream_pressure_packets_for_seam_gaps"
        "tests::retained_witness_generation_uses_real_diff_state_and_keeps_blocked_dimensions_explicit"
        "tests::handoff_packets_are_generated_from_retained_evidence_and_gated_by_capability_truth"
        "tests::observation_artifact_persists_from_upstream_source_bundle"
        "tests::twin_compare_artifact_persists_and_opens_on_real_run_and_observation"
        "tests::compare_regression_family_uses_retained_oxxlplay_fixtures_and_keeps_live_capture_gate_explicit"
        "tests::spreadsheetml_document_round_trip_reopens_into_the_h1_host"
        "tests::scenario_capsule_export_and_intake_preserve_lineage_and_capability_refs"
        "tests::workspace_manifest_groups_multiple_isolated_documents_without_merging_them"
    )

    foreach ($check in $checks) {
        Write-Host "run-host-integration: $check"
        cargo test -p dnaonecalc-host $check -- --exact
    }

    Write-Host "run-host-integration: ok"
}
finally {
    Pop-Location
}
