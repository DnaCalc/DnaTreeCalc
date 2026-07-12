$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

Push-Location (Join-Path $PSScriptRoot "..")
try {
    $checks = @(
        "tests::observation_artifact_persists_from_upstream_source_bundle"
        "tests::twin_compare_artifact_persists_and_opens_on_real_run_and_observation"
        "tests::compare_regression_family_uses_retained_oxxlplay_fixtures_and_keeps_live_capture_gate_explicit"
        "tests::widening_request_handoff_emits_from_real_compare_state"
    )

    foreach ($check in $checks) {
        Write-Host "run-host-compare-regression: $check"
        cargo test -p dnaonecalc-host $check -- --exact
    }

    Write-Host "run-host-compare-regression: live Windows capture remains a separate gate"
    Write-Host "run-host-compare-regression: ok"
}
finally {
    Pop-Location
}
