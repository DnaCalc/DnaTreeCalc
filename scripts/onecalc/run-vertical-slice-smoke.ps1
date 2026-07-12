# STALE (dtc-tsc.2): the smoke CLI flags this script drives were removed from the
# host bin upstream before the D5 import (bin now exposes verify-*/audit-formula-drill
# only), so every invocation exits 2 at 'unknown command'. Kept for the marker record.
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

Push-Location (Join-Path $PSScriptRoot "..\..")
try {
    $output = cargo run -p dnacalc-bench-host -- --shell-smoke 2>&1 | Out-String

    $requiredMarkers = @(
        "shell_regions=formula,result,diagnostics",
        "edit_packet=",
        "evaluation_truth=",
        "worksheet_value:Number(6)",
        "payload_summary:Number",
        "returned_surface:OrdinaryValue",
        "effective_display:none",
        "commit_decision:accepted"
    )

    foreach ($marker in $requiredMarkers) {
        if ($output -notmatch [regex]::Escape($marker)) {
            throw "run-vertical-slice-smoke: missing marker '$marker'`n--- output ---`n$output"
        }
    }

    Write-Host "run-vertical-slice-smoke: ok"
    Write-Host $output.TrimEnd()
}
finally {
    Pop-Location
}
