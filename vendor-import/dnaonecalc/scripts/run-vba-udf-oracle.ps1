param(
    [string]$OutputRoot = "target\onecalc-verification\vba-udf"
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    cargo run -p dnaonecalc-host -- verify-vba-udf `
        --case-id VBA-UDF-T001 `
        --formula "=AddThem(2,3)" `
        --excel-oracle-ref "OxXlPlay/states/excel/xlplay_vba_udf_addthem_001/views/normalized-replay.json" `
        --output-root $OutputRoot
} finally {
    Pop-Location
}
