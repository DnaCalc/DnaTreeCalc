<#
.SYNOPSIS
    Run the S0 exit gates (SHELL_SPEC.md §10 item 5) in one recorded sequence.

.DESCRIPTION
    SHELL_SPEC.md section 9 and REDESIGN_PROGRAM.md's S0 definition of done
    both require the parity CI job to cover *both* targets: "browser build +
    headless browser tests + Tauri build", not just the native host lane.
    This script runs every lane as one auditable pass:

      * F-gate  - the Bench app + host + desktop graphs carry no `oxcalc*`
                  (with the OxFml positive control, and the Calc-app
                  TC-adjacency inverted control).
      * P-gate  - every TP crate (shell, bridge, strand, skin-leptos) carries
                  no `ox*` crate at all.
      * T0-gate - `dnacalc-skin-ir` carries no Leptos and no engine.
                  (F/P/T0 + the permanent inverted control all live in
                   `dnacalc-arch-gates`.)
      * Strand contrast law (STRAND §2.1) - `dnacalc-strand` tests.
      * wasm compile gate - `cargo test --target wasm32-unknown-unknown
                  --no-run` for the four DOM acceptance suites
                  (`dnacalc-shell`, `dnacalc-bridge`, `dnacalc-app`,
                  `dnacalc-bench-app`). These crates' `tests/browser.rs`
                  suites are `#![cfg(target_arch = "wasm32")]`, so on the
                  native lane alone they compile to zero tests and report a
                  vacuous "ok" -- this gate is the only thing that actually
                  builds them for wasm32 on every commit, catching wasm-only
                  compile rot (stray imports, wasm-only cfg mistakes) even
                  on a machine with no WebDriver installed. ALWAYS runs.
      * Tauri build gate - `cargo build` for both desktop shells
                  (`dnacalc-bench-app-desktop`, `dnacalc-app-desktop`).
                  ALWAYS runs.
      * headless browser tests - the same four wasm crates, actually
                  executed in a live headless browser via
                  `wasm-bindgen-test-runner` (see `.cargo/config.toml`,
                  which pins `GECKODRIVER=geckodriver`). Runs only when a
                  WebDriver (`geckodriver` or `msedgedriver`) is found on
                  PATH; otherwise this lane is reported SKIPPED -- never
                  silently reported as if it ran and passed.

    Each step's exit code is captured; the script prints a PASS/SKIPPED/FAIL
    summary and exits non-zero if any *run* gate failed (a SKIPPED lane does
    not fail the script -- it is reported, not hidden), so CI (and a human)
    get one honest signal.

.NOTES
    Bead dtc-tsc.9 (original F/P/T0 + strand gates); extended under
    dtc-1tk.3 (S0 adversarial verification finding H3: the native-only gate
    never built wasm or Tauri, so nothing committed ever exercised those
    targets) to add the wasm compile gate, the Tauri build gate, and the
    conditional headless-browser lane. Engines are READ-ONLY; this script
    only reads the resolved dependency graph via `cargo tree` (inside the
    arch-gates tests), runs the strand unit tests, and builds/tests this
    repo's own crates for wasm32/desktop. It never builds or mutates any
    Ox* repo.
#>

[CmdletBinding()]
param(
    # Forwarded to every `cargo` invocation (e.g. -CargoArgs '--offline').
    [string[]] $CargoArgs = @()
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    # The four DOM acceptance suites (SHELL_SPEC.md §9/§10 item 5): each has
    # a `tests/browser.rs` that is `#![cfg(target_arch = "wasm32")]`, so both
    # the compile gate and the headless-run gate below target this exact
    # crate list.
    $wasmCrates = @('dnacalc-shell', 'dnacalc-bridge', 'dnacalc-app', 'dnacalc-bench-app')
    $wasmPackageArgs = $wasmCrates | ForEach-Object { '-p', $_ }

    # Each entry: a human Name, a Description shown in the section banner,
    # and the full argv (after `cargo`) to run. Native `&` invocations below
    # are deliberately bare statements (not captured into a variable) so
    # cargo's own stdout streams straight to the console instead of being
    # swept into $results -- capturing a native command's output inside a
    # helper function and returning a result object from that same function
    # pulls the command's stdout lines into the return stream too.
    $steps = @(
        @{ Name = 'F/P/T0 dependency-tier gates'; Argv = @('test', '-p', 'dnacalc-arch-gates') },
        @{ Name = 'Strand contrast law'; Argv = @('test', '-p', 'dnacalc-strand') },
        @{ Name = 'wasm compile (shell/bridge/app/bench-app)'; Argv = @('test', '--target', 'wasm32-unknown-unknown', '--no-run') + $wasmPackageArgs },
        @{ Name = 'Tauri build (bench-app-desktop/app-desktop)'; Argv = @('build', '-p', 'dnacalc-bench-app-desktop', '-p', 'dnacalc-app-desktop') }
    )

    $results = @()
    foreach ($step in $steps) {
        Write-Host ''
        Write-Host "=== S0 gate: $($step.Name)  (cargo $($step.Argv -join ' ')) ===" -ForegroundColor Cyan
        $stepArgv = $step.Argv + $CargoArgs
        & cargo @stepArgv
        $code = $LASTEXITCODE
        $results += [pscustomobject]@{
            Gate   = $step.Name
            Status = if ($code -eq 0) { 'PASS' } else { 'FAIL' }
            Code   = $code
        }
    }

    # ---- Headless browser tests (conditional on WebDriver; dtc-1tk.3 finding H3) ----
    $geckodriver = Get-Command geckodriver -ErrorAction SilentlyContinue
    $edgedriver = Get-Command msedgedriver -ErrorAction SilentlyContinue
    if ($geckodriver -or $edgedriver) {
        $driverName = if ($geckodriver) { $geckodriver.Name } else { $edgedriver.Name }
        $browserArgv = @('test', '--target', 'wasm32-unknown-unknown') + $wasmPackageArgs + $CargoArgs
        Write-Host ''
        Write-Host "=== S0 gate: headless browser tests (shell/bridge/app/bench-app)  ($driverName detected on PATH; cargo $($browserArgv -join ' ')) ===" -ForegroundColor Cyan
        & cargo @browserArgv
        $code = $LASTEXITCODE
        $results += [pscustomobject]@{
            Gate   = 'headless browser tests (shell/bridge/app/bench-app)'
            Status = if ($code -eq 0) { 'PASS' } else { 'FAIL' }
            Code   = $code
        }
    }
    else {
        Write-Host ''
        Write-Host '=== S0 gate: headless browser tests -- SKIPPED (no geckodriver/msedgedriver on PATH) ===' -ForegroundColor Yellow
        $results += [pscustomobject]@{
            Gate   = 'headless browser tests (shell/bridge/app/bench-app)'
            Status = 'SKIPPED'
            Code   = $null
        }
    }

    Write-Host ''
    Write-Host '================ S0 GATE SUMMARY ================' -ForegroundColor Cyan
    foreach ($result in $results) {
        $label = switch ($result.Status) {
            'PASS' { 'PASS' }
            'SKIPPED' { 'SKIP' }
            default { "FAIL ($($result.Code))" }
        }
        $color = switch ($result.Status) {
            'PASS' { 'Green' }
            'SKIPPED' { 'Yellow' }
            default { 'Red' }
        }
        Write-Host ("  {0,-9}  {1}" -f $label, $result.Gate) -ForegroundColor $color
    }
    Write-Host '=================================================' -ForegroundColor Cyan

    if ($results | Where-Object { $_.Status -eq 'FAIL' }) {
        Write-Host 'S0 gates: FAILED' -ForegroundColor Red
        exit 1
    }
    Write-Host 'S0 gates: ALL GREEN' -ForegroundColor Green
    exit 0
}
finally {
    Pop-Location
}
