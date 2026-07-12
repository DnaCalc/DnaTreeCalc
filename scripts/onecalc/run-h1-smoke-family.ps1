# STALE (dtc-tsc.2): the smoke CLI flags this script drives were removed from the
# host bin upstream before the D5 import (bin now exposes verify-*/audit-formula-drill
# only), so every invocation exits 2 at 'unknown command'. Kept for the marker record.
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

Push-Location (Join-Path $PSScriptRoot "..\..")
try {
    $checks = @(
        @{
            Name = "h1-smoke"
            Command = @("cargo", "run", "-p", "dnacalc-bench-host", "--", "--h1-smoke")
            RequiredMarkers = @(
                "dnacalc-bench-host h1 smoke"
                "host_profile=OC-H1"
                "edit_accept=trigger:edit_accept;packet_kind:edit_accept_recalc"
                "manual_recalc=trigger:manual;packet_kind:manual_recalc"
                "forced_recalc=trigger:forced;packet_kind:forced_recalc"
                "worksheet_value:Number(6)"
            )
        }
        @{
            Name = "h1-retained-smoke"
            Command = @("cargo", "run", "-p", "dnacalc-bench-host", "--", "--h1-retained-smoke")
            RequiredMarkers = @(
                "dnacalc-bench-host h1 retained smoke"
                "scenario_id=scenario-onecalc-h1-retained"
                "scenario_run_id=scenario-run-onecalc-h1-retained-edit-accept-recalc-"
                "reopened=host_profile:OC-H1;formula_text_version:2;worksheet_value:Number(6)"
            )
        }
        @{
            Name = "h1-compare-smoke"
            Command = @("cargo", "run", "-p", "dnacalc-bench-host", "--", "--h1-compare-smoke")
            RequiredMarkers = @(
                "dnacalc-bench-host h1 compare smoke"
                "comparison=same_scenario:true"
                "formula_version_changed:true"
                "formula_text_changed:true"
                "worksheet_value_match:false"
                "reliability:direct"
            )
        }
    )

    foreach ($check in $checks) {
        $output = & $check.Command[0] $check.Command[1..($check.Command.Length - 1)] 2>&1 | Out-String
        foreach ($marker in $check.RequiredMarkers) {
            if ($output -notmatch [regex]::Escape($marker)) {
                throw "run-h1-smoke-family: missing marker '$marker' in $($check.Name)`n--- output ---`n$output"
            }
        }

        Write-Host "run-h1-smoke-family: $($check.Name) ok"
        Write-Host $output.TrimEnd()
    }
}
finally {
    Pop-Location
}
