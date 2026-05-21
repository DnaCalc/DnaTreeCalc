#!/usr/bin/env pwsh
# validate-ux-matrix.ps1 - well-formedness + coverage checker for ux-trace-manifest.json.
#
# The UX analog of docs/test-corpus/tools/validate-corpus.ps1: UX trace slices + scenarios are tagged
# to a workset with a pending/active activation status, so "tests <-> work areas, progressively
# activated" is uniform across the model test corpus and the UX matrix. Prints a by-workset coverage
# matrix and a pending/active tally. Exit 0 = clean; exit 1 = problems.

$ErrorActionPreference = 'Stop'
$manifest = Join-Path (Split-Path -Parent $PSScriptRoot) 'ux-trace-manifest.json'
$knownStatuses = @('pending', 'active')
$errors = [System.Collections.Generic.List[string]]::new()
function Add-Err([string]$m) { $script:errors.Add($m) }

try { $doc = Get-Content -Raw -LiteralPath $manifest | ConvertFrom-Json -ErrorAction Stop }
catch { Write-Host "PARSE $($manifest): $($_.Exception.Message)"; exit 1 }

$sliceIds = [System.Collections.Generic.HashSet[string]]::new()
foreach ($s in $doc.slices) {
  if (-not ($s.id -match '^UX-[A-Z]{2}-\d{3}$')) { Add-Err "slice bad id '$($s.id)'" }
  elseif (-not $sliceIds.Add([string]$s.id)) { Add-Err "slice duplicate id '$($s.id)'" }
  if (-not ($s.workset -match '^W\d{3}$')) { Add-Err "slice $($s.id): bad workset '$($s.workset)'" }
  if ($s.status -notin $knownStatuses) { Add-Err "slice $($s.id): bad status '$($s.status)'" }
  if (-not $s.title) { Add-Err "slice $($s.id): missing title" }
}

$scenIds = [System.Collections.Generic.HashSet[string]]::new()
foreach ($sc in $doc.scenarios) {
  if (-not $sc.id) { Add-Err "scenario missing id" }
  elseif (-not $scenIds.Add([string]$sc.id)) { Add-Err "scenario duplicate id '$($sc.id)'" }
  if (-not ($sc.workset -match '^W\d{3}$')) { Add-Err "scenario $($sc.id): bad workset '$($sc.workset)'" }
  if ($sc.status -notin $knownStatuses) { Add-Err "scenario $($sc.id): bad status '$($sc.status)'" }
  foreach ($t in $sc.trace_ids) { if (-not $sliceIds.Contains([string]$t)) { Add-Err "scenario $($sc.id): unknown trace id '$t'" } }
}

foreach ($h in $doc.harnesses) {
  if (-not $h.name) { Add-Err "harness missing name" }
  if (-not ($h.first_workset -match '^W\d{3}$')) { Add-Err "harness '$($h.name)': bad first_workset '$($h.first_workset)'" }
}

Write-Host ""
Write-Host "DNA TreeCalc UX matrix validation"
Write-Host ("=" * 56)
Write-Host ("slices {0}   scenarios {1}   harnesses {2}" -f @($doc.slices).Count, @($doc.scenarios).Count, @($doc.harnesses).Count)
Write-Host ""
Write-Host "Coverage by workset:"
$worksets = @($doc.slices.workset) + @($doc.scenarios.workset) | Sort-Object -Unique
foreach ($w in $worksets) {
  $sl = @($doc.slices | Where-Object { $_.workset -eq $w })
  $sc = @($doc.scenarios | Where-Object { $_.workset -eq $w })
  Write-Host ("  {0}  ({1} slices, {2} scenarios)" -f $w, $sl.Count, $sc.Count)
  if ($sl.Count) { Write-Host ("      slices:    {0}" -f (($sl | ForEach-Object { $_.id }) -join ', ')) }
  if ($sc.Count) { Write-Host ("      scenarios: {0}" -f (($sc | ForEach-Object { $_.id }) -join ', ')) }
}
Write-Host ""
$activeS = @($doc.slices | Where-Object { $_.status -eq 'active' }).Count
$pendS = @($doc.slices | Where-Object { $_.status -eq 'pending' }).Count
Write-Host ("activation (slices): {0} active / {1} pending" -f $activeS, $pendS)
Write-Host ("=" * 56)

if ($errors.Count -gt 0) {
  Write-Host ("FAIL - {0} problem(s):" -f $errors.Count)
  foreach ($e in $errors) { Write-Host "  $e" }
  exit 1
}
Write-Host "OK - UX matrix manifest is well-formed and internally consistent."
exit 0
