# STALE (dtc-tsc.2): the `verify-vba-udf` bin command this script drives was removed
# from the host upstream before the D5 import (bin dispatch: verify-formula /
# verify-xml-cell / verify-batch / audit-formula-drill); every run exits 2.
param(
    [string]$OutputRoot = "target\onecalc-verification\vba-udf"
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
Push-Location $repoRoot
try {
    cargo run -p dnacalc-bench-host -- verify-vba-udf `
        --case-id VBA-UDF-T001 `
        --formula "=AddThem(2,3)" `
        --excel-oracle-ref "OxXlPlay/states/excel/xlplay_vba_udf_addthem_001/views/normalized-replay.json" `
        --output-root $OutputRoot
} finally {
    Pop-Location
}
