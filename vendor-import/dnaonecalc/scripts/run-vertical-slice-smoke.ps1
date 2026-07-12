$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

Push-Location (Join-Path $PSScriptRoot "..")
try {
    $output = cargo run -p dnaonecalc-host -- --shell-smoke 2>&1 | Out-String

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
