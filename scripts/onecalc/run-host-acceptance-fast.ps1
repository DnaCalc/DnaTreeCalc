# STALE (dtc-tsc.2): every test name in this script's filter list was removed from
# the host upstream before the D5 import (dno-uh9y rework), so each `cargo test
# <name> -- --exact` matches ZERO tests and exits 0 - the script's 'ok' is vacuous.
# Re-point the filters at live tests before trusting it again.
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

Push-Location (Join-Path $PSScriptRoot "..\..")
try {
    $checks = @(
        "tests::promoted_scenario_corpus_covers_main_product_planes"
        "shell::tests::shell_interaction_harness_covers_keyboard_shortcuts_and_focus_routing"
        "shell::tests::shell_app_projects_structured_xray_model_from_runtime_truth"
        "tests::capability_snapshot_open_and_diff_read_persisted_immutable_truth"
        "tests::retained_runs_emit_replay_capture_outputs_and_open_them_through_oxreplay"
        "tests::twin_compare_artifact_persists_and_opens_on_real_run_and_observation"
    )

    foreach ($check in $checks) {
        Write-Host "run-host-acceptance-fast smoke: $check"
        cargo test -p dnacalc-bench-host $check -- --exact
    }

    Write-Host "run-host-acceptance-fast smoke: ok"
}
finally {
    Pop-Location
}
