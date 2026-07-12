param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$RunnerArgs
)

$ErrorActionPreference = "Stop"

function Get-EdgeBrowserInfo {
    $candidates = @(
        "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        "C:\Program Files\Microsoft\Edge\Application\msedge.exe"
    )

    foreach ($candidate in $candidates) {
        if (Test-Path $candidate) {
            $item = Get-Item $candidate
            return [PSCustomObject]@{
                Path = $candidate
                Version = $item.VersionInfo.ProductVersion
            }
        }
    }

    return $null
}

function Get-EdgeDriverInfo {
    $driver = Get-Command msedgedriver.exe -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $driver) {
        return $null
    }

    $item = Get-Item $driver.Source
    return [PSCustomObject]@{
        Path = $driver.Source
        Version = $item.VersionInfo.ProductVersion
    }
}

function Get-VersionPrefix([string]$version) {
    if (-not $version) {
        return $null
    }

    return (($version -split '\.')[0..2] -join '.')
}

$edgeBrowser = Get-EdgeBrowserInfo
$edgeDriver = Get-EdgeDriverInfo
if ($edgeBrowser -and $edgeDriver) {
    $browserPrefix = Get-VersionPrefix $edgeBrowser.Version
    $driverPrefix = Get-VersionPrefix $edgeDriver.Version
    if ($browserPrefix -ne $driverPrefix) {
        throw "Edge WebDriver mismatch: browser $($edgeBrowser.Version) at $($edgeBrowser.Path), driver $($edgeDriver.Version) at $($edgeDriver.Path). Update or remove the PATH driver before running browser-mounted wasm tests."
    }
}

$runnerPath = & (Join-Path $PSScriptRoot "ensure-wasm-bindgen-runner.ps1") -PrintPath
& $runnerPath @RunnerArgs
exit $LASTEXITCODE
