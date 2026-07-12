$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

Push-Location (Join-Path $PSScriptRoot "..")
try {
    $registerPath = "docs/WORKSET_REGISTER.md"
    if (-not (Test-Path $registerPath)) {
        throw "validate-workset-register: missing $registerPath"
    }

    if (-not (Test-Path ".beads")) {
        throw "validate-workset-register: missing .beads workspace"
    }

    $content = Get-Content $registerPath -Raw

    $worksetMatches = [regex]::Matches($content, '^###\s+(WS-\d{2}(?:-\d{2})?)\s+', [System.Text.RegularExpressions.RegexOptions]::Multiline)
    if ($worksetMatches.Count -eq 0) {
        throw "validate-workset-register: no WS workset headings found"
    }

    $worksetIds = @($worksetMatches | ForEach-Object { $_.Groups[1].Value })
    $worksetDuplicate = $worksetIds | Group-Object | Where-Object { $_.Count -gt 1 } | Select-Object -First 1
    if ($worksetDuplicate) {
        throw "validate-workset-register: duplicate workset id '$($worksetDuplicate.Name)'"
    }

    foreach ($id in $worksetIds) {
        $purposePattern = '(?ms)^###\s+' + [regex]::Escape($id) + '\s+.*?1\.\s+purpose:'
        $dependsPattern = '(?ms)^###\s+' + [regex]::Escape($id) + '\s+.*?2\.\s+depends_on:'
        $closurePattern = '(?ms)^###\s+' + [regex]::Escape($id) + '\s+.*?5\.\s+closure_condition:'
        $epicsPattern = '(?ms)^###\s+' + [regex]::Escape($id) + '\s+.*?6\.\s+initial_epic_lanes:'

        if ($content -notmatch $purposePattern) {
            throw "validate-workset-register: missing purpose field for $id"
        }
        if ($content -notmatch $dependsPattern) {
            throw "validate-workset-register: missing depends_on field for $id"
        }
        if ($content -notmatch $closurePattern) {
            throw "validate-workset-register: missing closure_condition field for $id"
        }
        if ($content -notmatch $epicsPattern) {
            throw "validate-workset-register: missing initial_epic_lanes field for $id"
        }
    }

    $beadSummaryText = "beads=unavailable"
    try {
        $statsJson = br stats --robot 2>$null
        if ($LASTEXITCODE -eq 0 -and $statsJson) {
            $stats = $statsJson | ConvertFrom-Json
            $summary = $stats.summary
            if ($summary) {
                $beadSummaryText = @(
                    "beads",
                    "total=$($summary.total_issues)",
                    "open=$($summary.open_issues)",
                    "in_progress=$($summary.in_progress_issues)",
                    "ready=$($summary.ready_issues)",
                    "blocked=$($summary.blocked_issues)",
                    "deferred=$($summary.deferred_issues)",
                    "draft=$($summary.draft_issues)",
                    "closed=$($summary.closed_issues)"
                ) -join ", "
            }
        }
    }
    catch {
        # Keep the validator usable as a register shape check even if br stats is unavailable.
    }

    Write-Host "check-worksets: ok (worksets=$($worksetIds.Count); $beadSummaryText)"
}
finally {
    Pop-Location
}
