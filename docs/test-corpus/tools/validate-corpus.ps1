#!/usr/bin/env pwsh
# validate-corpus.ps1 - well-formedness + invariant checker for the DNA TreeCalc test corpus.
#
# This is the local-tier acceptance check (OPERATIONS.md section 6) for corpus-only beads, until
# the Rust runner that binds cases through the OxCalcTree bridge exists (a later W002 bead).
# Tooling rule: pwsh convenience layer, no Python (OPERATIONS.md section 6).
#
# Checks:
#   - every *.json under docs/test-corpus parses
#   - workspaces declare workspace_id + nodes[].node_id
#   - every case has id (corpus-unique) / name / spec / kind, kind is known
#   - resolution: workspace exists; caller exists in it; resolved target is '/' or an existing node
#   - classification / profile / syntax / import: per-kind required fields + known verdicts
#
# Two passes: load all workspaces first (order-independent), then validate cases.
# Exit 0 = clean (prints counts); exit 1 = one or more problems (prints them).

$ErrorActionPreference = 'Stop'
$corpusRoot = Split-Path -Parent $PSScriptRoot           # docs/test-corpus
$knownProfiles = @('treecalc-v1', 'strict-excel')
$knownCycleProfiles = @('cycle.non_iterative_stage1', 'cycle.excel_match_iterative', 'cycle.iterative_deterministic_v0')
$knownKinds    = @('resolution', 'classification', 'profile', 'syntax', 'import', 'cycle')

$errors     = [System.Collections.Generic.List[string]]::new()
$workspaces = @{}                                        # workspace_id -> HashSet[string] of node_ids
$seenIds    = [System.Collections.Generic.HashSet[string]]::new()
$caseDocs   = [System.Collections.Generic.List[object]]::new()
$counts     = @{ files = 0; workspaces = 0; cases = 0 }
$byKind     = @{}

function Add-Err([string]$m) { $script:errors.Add($m) }
function RelOf([string]$full) { $full.Substring($corpusRoot.Length).TrimStart('\', '/') }
function Category([string]$full) { ((RelOf $full) -split '[\\/]')[0] }

# ---- Pass 1: parse every file; build the workspace map; stash case docs ----
$files = Get-ChildItem -Path $corpusRoot -Recurse -Filter *.json -File
foreach ($f in $files) {
  $counts.files++
  $rel = RelOf $f.FullName
  try {
    $doc = Get-Content -Raw -LiteralPath $f.FullName | ConvertFrom-Json -ErrorAction Stop
  }
  catch {
    Add-Err "PARSE  ${rel}: $($_.Exception.Message)"
    continue
  }

  switch (Category $f.FullName) {
    'schema' { }                                         # JSON Schema docs: parse-checked only
    'workspaces' {
      if ($doc.schema_version -ne 'treecalc-workspace-v1') { Add-Err "WS     ${rel}: schema_version must be 'treecalc-workspace-v1'" }
      if (-not $doc.workspace_id) { Add-Err "WS     ${rel}: missing workspace_id"; continue }
      $set = [System.Collections.Generic.HashSet[string]]::new()
      foreach ($n in $doc.nodes) {
        if (-not $n.node_id) { Add-Err "WS     ${rel}: a node is missing node_id"; continue }
        [void]$set.Add([string]$n.node_id)
      }
      if ($workspaces.ContainsKey([string]$doc.workspace_id)) { Add-Err "WS     ${rel}: duplicate workspace_id '$($doc.workspace_id)'" }
      $workspaces[[string]$doc.workspace_id] = $set
      $counts.workspaces++
    }
    default {                                            # references / profiles / import = case files
      if ($doc.schema_version -ne 'treecalc-corpus-v1') { Add-Err "CASE   ${rel}: schema_version must be 'treecalc-corpus-v1'" }
      if ($null -eq $doc.cases) { Add-Err "CASE   ${rel}: missing cases[]"; continue }
      $caseDocs.Add([pscustomobject]@{ rel = $rel; doc = $doc })
    }
  }
}

# ---- Pass 2: validate cases against the complete workspace map ----
foreach ($entry in $caseDocs) {
  $rel = $entry.rel
  foreach ($c in $entry.doc.cases) {
    $counts.cases++
    $cid = if ($c.id) { $c.id } else { '<no-id>' }
    foreach ($req in 'id', 'name', 'spec', 'kind') {
      if (-not $c.$req) { Add-Err "CASE   ${rel} [$cid]: missing '$req'" }
    }
    if ($c.id -and -not $seenIds.Add([string]$c.id)) { Add-Err "CASE   ${rel} [$cid]: duplicate id" }
    if ($c.kind -and ($c.kind -notin $knownKinds)) { Add-Err "CASE   ${rel} [$cid]: unknown kind '$($c.kind)'" }
    if ($c.kind) { $byKind[$c.kind] = ([int]$byKind[$c.kind]) + 1 }

    switch ($c.kind) {
      'resolution' {
        foreach ($req in 'workspace', 'caller', 'reference') {
          if (-not $c.$req) { Add-Err "CASE   ${rel} [$cid]: resolution missing '$req'" }
        }
        $out = $c.expect.outcome
        if ($out -notin 'resolved', 'unresolved', 'reject', 'error') { Add-Err "CASE   ${rel} [$cid]: bad expect.outcome '$out'" }
        if ($c.workspace) {
          if (-not $workspaces.ContainsKey([string]$c.workspace)) {
            Add-Err "CASE   ${rel} [$cid]: unknown workspace '$($c.workspace)'"
          }
          else {
            $set = $workspaces[[string]$c.workspace]
            if ($c.caller -and -not $set.Contains([string]$c.caller)) { Add-Err "CASE   ${rel} [$cid]: caller '$($c.caller)' not in workspace '$($c.workspace)'" }
            if ($out -eq 'resolved') {
              $tgt = $c.expect.target
              if (-not $tgt) { Add-Err "CASE   ${rel} [$cid]: resolved case needs expect.target" }
              elseif ($tgt -ne '/' -and -not $set.Contains([string]$tgt)) { Add-Err "CASE   ${rel} [$cid]: target '$tgt' not in workspace '$($c.workspace)'" }
            }
          }
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
            $v = $c.profiles.$pn
            $verdict = if ($v -is [string]) { $v } else { $v.verdict }
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
        elseif ($c.expect.nodes) {
          foreach ($n in $c.expect.nodes) {
            if (-not $n.node_id) { Add-Err "CASE   ${rel} [$cid]: an expect.nodes entry is missing node_id" }
          }
        }
        elseif ($c.expect.outcome) {
          if ($c.expect.outcome -notin 'out-of-scope', 'eval-error') { Add-Err "CASE   ${rel} [$cid]: bad import expect.outcome '$($c.expect.outcome)'" }
        }
        else {
          Add-Err "CASE   ${rel} [$cid]: import expect needs either nodes[] or outcome"
        }
      }
      'cycle' {
        if (-not $c.workspace) { Add-Err "CASE   ${rel} [$cid]: cycle missing 'workspace'" }
        elseif (-not $workspaces.ContainsKey([string]$c.workspace)) { Add-Err "CASE   ${rel} [$cid]: unknown workspace '$($c.workspace)'" }
        else {
          $set = $workspaces[[string]$c.workspace]
          if (-not $c.members) { Add-Err "CASE   ${rel} [$cid]: cycle missing 'members'" }
          foreach ($m in $c.members) { if (-not $set.Contains([string]$m)) { Add-Err "CASE   ${rel} [$cid]: cycle member '$m' not in workspace '$($c.workspace)'" } }
        }
        if ($c.config.profile -notin $knownCycleProfiles) { Add-Err "CASE   ${rel} [$cid]: bad config.profile '$($c.config.profile)'" }
        if ($c.expect.outcome -notin 'cycle_blocked', 'published', 'rejected') { Add-Err "CASE   ${rel} [$cid]: bad cycle expect.outcome '$($c.expect.outcome)'" }
      }
    }
  }
}

Write-Host ""
Write-Host "DNA TreeCalc test corpus validation"
Write-Host ("-" * 42)
Write-Host ("files parsed : {0}" -f $counts.files)
Write-Host ("workspaces   : {0}" -f $counts.workspaces)
Write-Host ("cases        : {0}" -f $counts.cases)
foreach ($k in ($byKind.Keys | Sort-Object)) { Write-Host ("  {0,-14}: {1}" -f $k, $byKind[$k]) }
Write-Host ("-" * 42)

if ($errors.Count -gt 0) {
  Write-Host ("FAIL - {0} problem(s):" -f $errors.Count)
  foreach ($e in $errors) { Write-Host "  $e" }
  exit 1
}
Write-Host "OK - corpus is well-formed and internally consistent."
exit 0
