#!/usr/bin/env pwsh
# validate-corpus.ps1 - well-formedness + invariant checker for the DNA TreeCalc test corpus.
#
# Local-tier acceptance check (OPERATIONS.md section 6) for corpus-only beads, until the Rust runner
# that binds cases through the OxCalcTree bridge exists. Tooling rule: pwsh, no Python.
#
# Each theme file declares the owning `workset` and an activation `status` (pending|active) so the
# corpus traces onto the work plan and can be progressively activated. This script prints a coverage
# matrix grouped by workset + status. The future runner selects status=active themes; this validator
# checks well-formedness of every theme regardless of status.
#
# Two passes: load all workspaces first (order-independent), then validate cases.
# Exit 0 = clean; exit 1 = one or more problems.

$ErrorActionPreference = 'Stop'
$corpusRoot = Split-Path -Parent $PSScriptRoot
$knownProfiles = @('treecalc-v1', 'strict-excel')
$knownCycleProfiles = @('cycle.non_iterative_stage1', 'cycle.excel_match_iterative', 'cycle.iterative_deterministic_v0')
$knownKinds = @('resolution', 'classification', 'profile', 'syntax', 'import', 'cycle', 'dynamic',
  'membership', 'constant', 'edit', 'template', 'format', 'value_equivalence')
$knownStatuses = @('pending', 'active')

$errors = [System.Collections.Generic.List[string]]::new()
$workspaces = @{}
$seenIds = [System.Collections.Generic.HashSet[string]]::new()
$caseDocs = [System.Collections.Generic.List[object]]::new()
$themeRows = [System.Collections.Generic.List[object]]::new()
$counts = @{ files = 0; workspaces = 0; cases = 0 }
$byKind = @{}

function Add-Err([string]$m) { $script:errors.Add($m) }
function RelOf([string]$full) { $full.Substring($corpusRoot.Length).TrimStart('\', '/') }
function Category([string]$full) { ((RelOf $full) -split '[\\/]')[0] }
function InWs($set, $id) { $set.Contains([string]$id) }

# ---- Pass 1: parse every file; build the workspace map; stash case docs ----
$files = Get-ChildItem -Path $corpusRoot -Recurse -Filter *.json -File
foreach ($f in $files) {
  $counts.files++
  $rel = RelOf $f.FullName
  try { $doc = Get-Content -Raw -LiteralPath $f.FullName | ConvertFrom-Json -ErrorAction Stop }
  catch { Add-Err "PARSE  ${rel}: $($_.Exception.Message)"; continue }

  switch (Category $f.FullName) {
    'schema' { }
    'workspaces' {
      if ($doc.schema_version -ne 'treecalc-workspace-v1') { Add-Err "WS     ${rel}: schema_version must be 'treecalc-workspace-v1'" }
      if (-not $doc.workspace_id) { Add-Err "WS     ${rel}: missing workspace_id"; continue }
      $set = [System.Collections.Generic.HashSet[string]]::new()
      foreach ($n in $doc.nodes) { if (-not $n.node_id) { Add-Err "WS     ${rel}: a node is missing node_id"; continue }; [void]$set.Add([string]$n.node_id) }
      if ($workspaces.ContainsKey([string]$doc.workspace_id)) { Add-Err "WS     ${rel}: duplicate workspace_id '$($doc.workspace_id)'" }
      $workspaces[[string]$doc.workspace_id] = $set
      $counts.workspaces++
    }
    default {
      if ($doc.schema_version -ne 'treecalc-corpus-v1') { Add-Err "CASE   ${rel}: schema_version must be 'treecalc-corpus-v1'" }
      if (-not ($doc.workset -match '^W\d{3}$')) { Add-Err "CASE   ${rel}: missing/!valid 'workset' (expect W###)" }
      if ($doc.status -notin $knownStatuses) { Add-Err "CASE   ${rel}: 'status' must be one of $($knownStatuses -join '/')" }
      if ($null -eq $doc.cases) { Add-Err "CASE   ${rel}: missing cases[]"; continue }
      $caseDocs.Add([pscustomobject]@{ rel = $rel; doc = $doc })
    }
  }
}

# ---- Pass 2: validate cases against the complete workspace map ----
foreach ($entry in $caseDocs) {
  $rel = $entry.rel
  $doc = $entry.doc
  $themeCount = 0
  foreach ($c in $doc.cases) {
    $counts.cases++; $themeCount++
    $cid = if ($c.id) { $c.id } else { '<no-id>' }
    foreach ($req in 'id', 'name', 'spec', 'kind') { if (-not $c.$req) { Add-Err "CASE   ${rel} [$cid]: missing '$req'" } }
    if ($c.id -and -not $seenIds.Add([string]$c.id)) { Add-Err "CASE   ${rel} [$cid]: duplicate id" }
    if ($c.kind -and ($c.kind -notin $knownKinds)) { Add-Err "CASE   ${rel} [$cid]: unknown kind '$($c.kind)'" }
    if ($c.kind) { $byKind[$c.kind] = ([int]$byKind[$c.kind]) + 1 }

    $ws = if ($c.workspace) { $c.workspace } else { $null }
    $set = if ($ws -and $workspaces.ContainsKey([string]$ws)) { $workspaces[[string]$ws] } else { $null }
    if ($ws -and -not $set) { Add-Err "CASE   ${rel} [$cid]: unknown workspace '$ws'" }

    switch ($c.kind) {
      'resolution' {
        foreach ($req in 'workspace', 'caller', 'reference') { if (-not $c.$req) { Add-Err "CASE   ${rel} [$cid]: resolution missing '$req'" } }
        if ($c.expect.outcome -notin 'resolved', 'unresolved', 'reject', 'error') { Add-Err "CASE   ${rel} [$cid]: bad expect.outcome '$($c.expect.outcome)'" }
        if ($set) {
          if ($c.caller -and -not (InWs $set $c.caller)) { Add-Err "CASE   ${rel} [$cid]: caller '$($c.caller)' not in '$ws'" }
          if ($c.expect.outcome -eq 'resolved') {
            $t = $c.expect.target
            $tws = if ($c.expect.target_workspace) { [string]$c.expect.target_workspace } else { [string]$ws }
            if (-not $t) { Add-Err "CASE   ${rel} [$cid]: resolved case needs expect.target" }
            elseif ($t -ne '/') {
              if (-not $workspaces.ContainsKey($tws)) { Add-Err "CASE   ${rel} [$cid]: unknown target_workspace '$tws'" }
              elseif (-not (InWs $workspaces[$tws] $t)) { Add-Err "CASE   ${rel} [$cid]: target '$t' not in '$tws'" }
            }
          }
        }
      }
      'membership' {
        foreach ($req in 'workspace', 'caller', 'reference') { if (-not $c.$req) { Add-Err "CASE   ${rel} [$cid]: membership missing '$req'" } }
        if ($c.expect.outcome -notin 'resolved', 'unresolved') { Add-Err "CASE   ${rel} [$cid]: bad membership expect.outcome '$($c.expect.outcome)'" }
        if ($set) {
          if ($c.caller -and -not (InWs $set $c.caller)) { Add-Err "CASE   ${rel} [$cid]: caller '$($c.caller)' not in '$ws'" }
          if ($null -eq $c.expect.members) { Add-Err "CASE   ${rel} [$cid]: membership needs expect.members[]" }
          else { foreach ($m in $c.expect.members) { if (-not (InWs $set $m)) { Add-Err "CASE   ${rel} [$cid]: member '$m' not in '$ws'" } } }
        }
      }
      'classification' {
        if (-not $c.reference) { Add-Err "CASE   ${rel} [$cid]: classification missing 'reference'" }
        if ($c.expect.cardinality -notin 'single', 'set', 'value') { Add-Err "CASE   ${rel} [$cid]: bad expect.cardinality '$($c.expect.cardinality)'" }
      }
      'profile' {
        if (-not $c.reference) { Add-Err "CASE   ${rel} [$cid]: profile missing 'reference'" }
        if ($null -eq $c.profiles) { Add-Err "CASE   ${rel} [$cid]: profile missing 'profiles'" }
        else {
          $names = @($c.profiles.PSObject.Properties.Name)
          if ($names.Count -eq 0) { Add-Err "CASE   ${rel} [$cid]: 'profiles' has no entries" }
          foreach ($pn in $names) {
            if ($pn -notin $knownProfiles) { Add-Err "CASE   ${rel} [$cid]: unknown profile id '$pn'" }
            $v = $c.profiles.$pn; $verdict = if ($v -is [string]) { $v } else { $v.verdict }
            if ($verdict -notin 'accept', 'reject') { Add-Err "CASE   ${rel} [$cid]: profile '$pn' bad verdict '$verdict'" }
          }
        }
      }
      'syntax' {
        if (-not $c.reference) { Add-Err "CASE   ${rel} [$cid]: syntax missing 'reference'" }
        if ($c.expect.parse -notin 'accept', 'reject') { Add-Err "CASE   ${rel} [$cid]: bad expect.parse '$($c.expect.parse)'" }
      }
      'import' {
        if ($null -eq $c.excel) { Add-Err "CASE   ${rel} [$cid]: import missing 'excel'" }
        if ($null -eq $c.expect) { Add-Err "CASE   ${rel} [$cid]: import missing 'expect'" }
        elseif ($c.expect.nodes) { foreach ($n in $c.expect.nodes) { if (-not $n.node_id) { Add-Err "CASE   ${rel} [$cid]: an expect.nodes entry is missing node_id" } } }
        elseif ($c.expect.outcome) { if ($c.expect.outcome -notin 'out-of-scope', 'eval-error') { Add-Err "CASE   ${rel} [$cid]: bad import expect.outcome '$($c.expect.outcome)'" } }
        else { Add-Err "CASE   ${rel} [$cid]: import expect needs either nodes[] or outcome" }
      }
      'cycle' {
        if (-not $c.workspace) { Add-Err "CASE   ${rel} [$cid]: cycle missing 'workspace'" }
        if ($set) {
          if (-not $c.members) { Add-Err "CASE   ${rel} [$cid]: cycle missing 'members'" }
          else { foreach ($m in $c.members) { if (-not (InWs $set $m)) { Add-Err "CASE   ${rel} [$cid]: cycle member '$m' not in '$ws'" } } }
        }
        if ($c.config.profile -notin $knownCycleProfiles) { Add-Err "CASE   ${rel} [$cid]: bad config.profile '$($c.config.profile)'" }
        if ($c.expect.outcome -notin 'cycle_blocked', 'published', 'rejected') { Add-Err "CASE   ${rel} [$cid]: bad cycle expect.outcome '$($c.expect.outcome)'" }
      }
      'dynamic' {
        foreach ($req in 'workspace', 'caller', 'reference') { if (-not $c.$req) { Add-Err "CASE   ${rel} [$cid]: dynamic missing '$req'" } }
        if ($c.expect.outcome -notin 'resolved', 'unresolved', 'error', 'cycle_blocked') { Add-Err "CASE   ${rel} [$cid]: bad dynamic expect.outcome '$($c.expect.outcome)'" }
        if ($set) {
          if ($c.caller -and -not (InWs $set $c.caller)) { Add-Err "CASE   ${rel} [$cid]: caller '$($c.caller)' not in '$ws'" }
          if ($c.expect.outcome -eq 'resolved') {
            $t = $c.expect.target
            if (-not $t) { Add-Err "CASE   ${rel} [$cid]: resolved dynamic case needs expect.target" }
            elseif ($t -ne '/' -and -not (InWs $set $t)) { Add-Err "CASE   ${rel} [$cid]: target '$t' not in '$ws'" }
          }
          if ($c.given) { foreach ($k in $c.given.PSObject.Properties.Name) { if (-not (InWs $set $k)) { Add-Err "CASE   ${rel} [$cid]: given key '$k' not in '$ws'" } } }
          if ($c.members) { foreach ($m in $c.members) { if (-not (InWs $set $m)) { Add-Err "CASE   ${rel} [$cid]: member '$m' not in '$ws'" } } }
        }
      }
      'constant' {
        if ($null -eq $c.input) { Add-Err "CASE   ${rel} [$cid]: constant missing 'input'" }
        if ($c.expect.value_type -notin 'empty', 'number', 'logical', 'text', 'formula', 'error') { Add-Err "CASE   ${rel} [$cid]: bad constant expect.value_type '$($c.expect.value_type)'" }
      }
      'edit' {
        if (-not $c.workspace) { Add-Err "CASE   ${rel} [$cid]: edit missing 'workspace'" }
        if ($null -eq $c.edit) { Add-Err "CASE   ${rel} [$cid]: edit missing 'edit'" }
        elseif ($c.edit.op -notin 'rename', 'move', 'delete', 'insert') { Add-Err "CASE   ${rel} [$cid]: bad edit.op '$($c.edit.op)'" }
        if ($set) {
          if ($c.edit.target -and -not (InWs $set $c.edit.target)) { Add-Err "CASE   ${rel} [$cid]: edit.target '$($c.edit.target)' not in '$ws'" }
          if ($c.caller -and -not (InWs $set $c.caller)) { Add-Err "CASE   ${rel} [$cid]: caller '$($c.caller)' not in '$ws'" }
        }
        if ($c.expect.outcome -notin 'resolved', 'unresolved', 'rebound', 'error') { Add-Err "CASE   ${rel} [$cid]: bad edit expect.outcome '$($c.expect.outcome)'" }
      }
      'template' {
        if (-not $c.workspace) { Add-Err "CASE   ${rel} [$cid]: template missing 'workspace'" }
        if ($c.op -notin 'promote', 'instantiate', 'edit', 'sync', 'override', 'detach', 'fit_check') { Add-Err "CASE   ${rel} [$cid]: bad template op '$($c.op)'" }
      }
      'format' {
        if (-not $c.workspace) { Add-Err "CASE   ${rel} [$cid]: format missing 'workspace'" }
        if ($set -and $c.node -and -not (InWs $set $c.node)) { Add-Err "CASE   ${rel} [$cid]: node '$($c.node)' not in '$ws'" }
      }
      'value_equivalence' {
        if (-not $c.workspace) { Add-Err "CASE   ${rel} [$cid]: value_equivalence missing 'workspace'" }
        if ($set -and $c.expect.excel_values) {
          foreach ($k in $c.expect.excel_values.PSObject.Properties.Name) { if (-not (InWs $set $k)) { Add-Err "CASE   ${rel} [$cid]: excel_values key '$k' not in '$ws'" } }
        }
      }
    }
  }
  $themeRows.Add([pscustomobject]@{ workset = [string]$doc.workset; status = [string]$doc.status; theme = [string]$doc.theme; count = $themeCount })
}

# ---- Report ----
Write-Host ""
Write-Host "DNA TreeCalc test corpus validation"
Write-Host ("=" * 60)
Write-Host ("files {0}   workspaces {1}   cases {2}" -f $counts.files, $counts.workspaces, $counts.cases)
Write-Host ""
Write-Host "Coverage by workset (theme : cases : status):"
foreach ($wkset in ($themeRows.workset | Sort-Object -Unique)) {
  $rows = $themeRows | Where-Object { $_.workset -eq $wkset }
  $wsum = ($rows | Measure-Object count -Sum).Sum
  Write-Host ("  {0}  [{1} cases]" -f $wkset, $wsum)
  foreach ($r in ($rows | Sort-Object theme)) { Write-Host ("      {0,-34} {1,3}  {2}" -f $r.theme, $r.count, $r.status) }
}
Write-Host ""
$active = ($themeRows | Where-Object { $_.status -eq 'active' } | Measure-Object count -Sum).Sum
$pending = ($themeRows | Where-Object { $_.status -eq 'pending' } | Measure-Object count -Sum).Sum
Write-Host ("activation: {0} active / {1} pending" -f ([int]$active), ([int]$pending))
Write-Host ("=" * 60)

if ($errors.Count -gt 0) {
  Write-Host ("FAIL - {0} problem(s):" -f $errors.Count)
  foreach ($e in $errors) { Write-Host "  $e" }
  exit 1
}
Write-Host "OK - corpus is well-formed and internally consistent."
exit 0
