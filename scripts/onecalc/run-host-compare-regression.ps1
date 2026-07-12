# STALE (dtc-tsc.2): every test name in this script's filter list was removed from
# the host upstream before the D5 import (dno-uh9y rework), so each `cargo test
# <name> -- --exact` matches ZERO tests and exits 0 - the script's 'ok' is vacuous.
# Re-point the filters at live tests before trusting it again.
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

Push-Location (Join-Path $PSScriptRoot "..\..")
try {
    $checks = @(
        "tests::observation_artifact_persists_from_upstream_source_bundle"
        "tests::twin_compare_artifact_persists_and_opens_on_real_run_and_observation"
        "tests::compare_regression_family_uses_retained_oxxlplay_fixtures_and_keeps_live_capture_gate_explicit"
        "tests::widening_request_handoff_emits_from_real_compare_state"
    )

    foreach ($check in $checks) {
        Write-Host "run-host-compare-regression: $check"
        cargo test -p dnacalc-bench-host $check -- --exact
    }

    Write-Host "run-host-compare-regression: live Windows capture remains a separate gate"
    Write-Host "run-host-compare-regression: ok"
}
finally {
    Pop-Location
}
