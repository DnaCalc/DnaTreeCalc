<#
.SYNOPSIS
    Run the S0 exit gates (SHELL_SPEC.md §10 item 5) in one recorded sequence.

.DESCRIPTION
    The four S0 dependency/contrast gates, all green, are the S0 exit
    acceptance's item 5. This script runs them as one auditable pass:

      * F-gate  - the Bench app + host + desktop graphs carry no `oxcalc*`
                  (with the OxFml positive control, and the Calc-app
                  TC-adjacency inverted control).
      * P-gate  - every TP crate (shell, bridge, strand, skin-leptos) carries
                  no `ox*` crate at all.
      * T0-gate - `dnacalc-skin-ir` carries no Leptos and no engine.
                  (F/P/T0 + the permanent inverted control all live in
                   `dnacalc-arch-gates`.)
      * Strand contrast law (STRAND §2.1) - `dnacalc-strand` tests.

    Each step's exit code is captured; the script prints a PASS/FAIL summary
    and exits non-zero if any gate fails, so CI (and a human) get one honest
    signal.

.NOTES
    Bead dtc-tsc.9. Engines are READ-ONLY; this script only reads the resolved
    dependency graph via `cargo tree` (inside the arch-gates tests) and runs
    the strand unit tests. It never builds or mutates any Ox* repo.
#>

[CmdletBinding()]
param(
    # Forwarded to `cargo test` (e.g. -CargoArgs '--offline').
    [string[]] $CargoArgs = @()
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    $steps = @(
        @{ Name = 'F/P/T0 dependency-tier gates'; Package = 'dnacalc-arch-gates' },
        @{ Name = 'Strand contrast law';          Package = 'dnacalc-strand' }
    )

    $results = @()
    foreach ($step in $steps) {
        Write-Host ''
        Write-Host "=== S0 gate: $($step.Name)  (cargo test -p $($step.Package)) ===" -ForegroundColor Cyan
        & cargo test -p $step.Package @CargoArgs
        $code = $LASTEXITCODE
        $results += [pscustomobject]@{
            Gate    = $step.Name
            Package = $step.Package
            Passed  = ($code -eq 0)
            Code    = $code
        }
    }

    Write-Host ''
    Write-Host '================ S0 GATE SUMMARY ================' -ForegroundColor Cyan
    foreach ($result in $results) {
        $status = if ($result.Passed) { 'PASS' } else { "FAIL ($($result.Code))" }
        $color = if ($result.Passed) { 'Green' } else { 'Red' }
        Write-Host ("  {0,-4}  {1}" -f $status, $result.Gate) -ForegroundColor $color
    }
    Write-Host '=================================================' -ForegroundColor Cyan

    if ($results | Where-Object { -not $_.Passed }) {
        Write-Host 'S0 gates: FAILED' -ForegroundColor Red
        exit 1
    }
    Write-Host 'S0 gates: ALL GREEN' -ForegroundColor Green
    exit 0
}
finally {
    Pop-Location
}
