param(
    [switch]$PrintPath
)

$ErrorActionPreference = "Stop"

# Resolve the installed Edge browser version, then ensure a matching
# msedgedriver lives under .tools\edgedriver\<version>\msedgedriver.exe.
# Edge auto-updates faster than a manually-installed PATH msedgedriver
# can keep up with; the WebDriver protocol enforces matching majors, so
# the wasm-bindgen browser tests fail outright when they drift apart.
# This script keeps a self-healing version-matched cache in the repo so
# `cargo test-browser` works without manual driver maintenance.

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

function Get-EdgeVersion {
    $candidates = @(
        "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        "C:\Program Files\Microsoft\Edge\Application\msedge.exe"
    )
    foreach ($candidate in $candidates) {
        if (Test-Path $candidate) {
            return (Get-Item $candidate).VersionInfo.ProductVersion
        }
    }
    throw "Could not locate Microsoft Edge install; expected msedge.exe under Program Files."
}

$edgeVersion = Get-EdgeVersion
$cacheRoot = Join-Path $repoRoot ".tools\edgedriver\$edgeVersion"
$driverExe = Join-Path $cacheRoot "msedgedriver.exe"

if (-not (Test-Path $driverExe)) {
    if (-not (Test-Path $cacheRoot)) {
        New-Item -ItemType Directory -Path $cacheRoot -Force | Out-Null
    }
    $zipPath = Join-Path $cacheRoot "edgedriver_win64.zip"
    $candidateUrls = @(
        "https://msedgedriver.microsoft.com/$edgeVersion/edgedriver_win64.zip",
        "https://msedgedriver.azureedge.net/$edgeVersion/edgedriver_win64.zip",
        "https://msedgewebdriverstorage.blob.core.windows.net/edgewebdriver/$edgeVersion/edgedriver_win64.zip"
    )

    $downloaded = $false
    foreach ($url in $candidateUrls) {
        try {
            Write-Host "ensure-edgedriver: downloading $url"
            Invoke-WebRequest -Uri $url -OutFile $zipPath -UseBasicParsing -TimeoutSec 60
            $downloaded = $true
            break
        }
        catch {
            Write-Host "  -> failed: $($_.Exception.Message)"
        }
    }
    if (-not $downloaded) {
        throw "Could not fetch msedgedriver $edgeVersion from any of the known mirrors."
    }

    Expand-Archive -Path $zipPath -DestinationPath $cacheRoot -Force
    Remove-Item $zipPath -Force -ErrorAction SilentlyContinue
}

if (-not (Test-Path $driverExe)) {
    throw "msedgedriver.exe not found under $cacheRoot after extraction."
}

if ($PrintPath) {
    $driverExe
}
