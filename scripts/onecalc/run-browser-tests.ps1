# NOTE (dtc-tsc.2): the 'cargo test-browser*' aliases lived in the old repo's
# .cargo/config.toml, which was intentionally not imported; the workspace harness
# (wasm-bindgen-test-runner + geckodriver, see .cargo/config.toml) is the supported path.
param(
    [switch]$Live
)

$ErrorActionPreference = "Stop"

& (Join-Path $PSScriptRoot "ensure-wasm-bindgen-runner.ps1") | Out-Null

# Pin a version-matched msedgedriver under .tools\edgedriver and
# prepend it to PATH so the wasm-bindgen test runner picks it up
# instead of any older driver the developer has on their global PATH.
# Edge auto-updates faster than the typical manual driver refresh
# cadence; without this, browser tests fail with a WebDriver mismatch
# the moment Edge ships a new major.
$driverPath = & (Join-Path $PSScriptRoot "ensure-edgedriver.ps1") -PrintPath
$driverDir = Split-Path -Parent $driverPath
$env:PATH = "$driverDir;$env:PATH"

if ($Live) {
    cargo test-browser-live -p dnacalc-bench-host
} else {
    cargo test-browser -p dnacalc-bench-host
}

exit $LASTEXITCODE
