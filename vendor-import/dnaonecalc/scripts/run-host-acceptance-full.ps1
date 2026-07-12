$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

Push-Location (Join-Path $PSScriptRoot "..")
try {
    cargo test -p dnaonecalc-host
    Write-Host "run-host-acceptance-full: ok"
}
finally {
    Pop-Location
}
