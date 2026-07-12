param(
    [switch]$Release
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$desktopCrateRoot = Join-Path $repoRoot "src\dnaonecalc-desktop"

Push-Location $desktopCrateRoot
try {
    if ($Release) {
        cargo tauri build
    } else {
        cargo tauri dev
    }
    if ($LASTEXITCODE -ne 0) {
        throw "cargo tauri failed with exit code $LASTEXITCODE"
    }
}
finally {
    Pop-Location
}
