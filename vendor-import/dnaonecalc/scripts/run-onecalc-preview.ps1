param(
    [int]$Port = 0,
    [switch]$BuildOnly,
    [switch]$NoOpen
)

$ErrorActionPreference = "Stop"

function Get-FreeTcpPort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    try {
        return ([System.Net.IPEndPoint]$listener.LocalEndpoint).Port
    }
    finally {
        $listener.Stop()
    }
}

function Get-ContentType([string]$path) {
    switch ([System.IO.Path]::GetExtension($path).ToLowerInvariant()) {
        ".html" { return "text/html; charset=utf-8" }
        ".js" { return "text/javascript; charset=utf-8" }
        ".wasm" { return "application/wasm" }
        ".css" { return "text/css; charset=utf-8" }
        ".json" { return "application/json; charset=utf-8" }
        default { return "application/octet-stream" }
    }
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$hostCrateRoot = Join-Path $repoRoot "src\dnaonecalc-host"
$previewRoot = Join-Path $repoRoot "target\onecalc-preview"
$bindgenPath = & (Join-Path $PSScriptRoot "ensure-wasm-bindgen-runner.ps1") -PrintCliPath

cargo build --lib --target wasm32-unknown-unknown -p dnaonecalc-host
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed"
}

$wasmPath = Join-Path $repoRoot "target\wasm32-unknown-unknown\debug\dnaonecalc_host.wasm"
if (-not (Test-Path $wasmPath)) {
    throw "Expected wasm output not found: $wasmPath"
}

if (Test-Path $previewRoot) {
    Remove-Item -Recurse -Force $previewRoot
}
New-Item -ItemType Directory -Path $previewRoot -Force | Out-Null

& $bindgenPath `
    --target web `
    --out-dir $previewRoot `
    --out-name onecalc_preview `
    $wasmPath
if ($LASTEXITCODE -ne 0) {
    throw "wasm-bindgen generation failed"
}

$html = @"
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>DNA OneCalc Preview</title>
    <style>
      html, body {
        margin: 0;
        padding: 0;
        background: #f3efe5;
      }
      #onecalc-root {
        min-height: 100vh;
      }
    </style>
  </head>
  <body>
    <div id="onecalc-root"></div>
    <script type="module">
      import init, { mount_onecalc_preview } from "./onecalc_preview.js";

      await init();
      mount_onecalc_preview("onecalc-root");
    </script>
  </body>
</html>
"@

$indexPath = Join-Path $previewRoot "index.html"
Set-Content -Path $indexPath -Value $html -NoNewline

Write-Host "Preview assets generated at $previewRoot"
Write-Host "Open $indexPath directly only for inspection; interactive wasm should be served over HTTP."

if ($BuildOnly) {
    exit 0
}

if ($Port -le 0) {
    $Port = Get-FreeTcpPort
}

$listener = [System.Net.HttpListener]::new()
$prefix = "http://127.0.0.1:$Port/"
$listener.Prefixes.Add($prefix)
$listener.Start()
$previewStopRequested = $false
$originalTreatControlCAsInput = $null

if ([Console]::IsInputRedirected -eq $false) {
    $originalTreatControlCAsInput = [Console]::TreatControlCAsInput
    [Console]::TreatControlCAsInput = $true
}

if (-not $NoOpen) {
    Start-Process $prefix | Out-Null
}

Write-Host "DNA OneCalc preview available at $prefix"
Write-Host "Press Ctrl+C to stop the preview server."

try {
    while ((-not $previewStopRequested) -and $listener.IsListening) {
        if (($originalTreatControlCAsInput -ne $null) -and [Console]::KeyAvailable) {
            $pressedKey = [Console]::ReadKey($true)
            if (
                (($pressedKey.Modifiers -band [ConsoleModifiers]::Control) -ne 0) -and
                ($pressedKey.Key -eq [ConsoleKey]::C)
            ) {
                $previewStopRequested = $true
                Write-Host "Stopping preview server..."
                break
            }
        }

        $getContextTask = $listener.GetContextAsync()
        while ((-not $previewStopRequested) -and $listener.IsListening -and (-not $getContextTask.Wait(200))) {
            if (($originalTreatControlCAsInput -ne $null) -and [Console]::KeyAvailable) {
                $pressedKey = [Console]::ReadKey($true)
                if (
                    (($pressedKey.Modifiers -band [ConsoleModifiers]::Control) -ne 0) -and
                    ($pressedKey.Key -eq [ConsoleKey]::C)
                ) {
                    $previewStopRequested = $true
                    Write-Host "Stopping preview server..."
                    break
                }
            }
        }

        if ($previewStopRequested -or (-not $listener.IsListening)) {
            break
        }

        try {
            $context = $getContextTask.GetAwaiter().GetResult()
        }
        catch [System.ObjectDisposedException] {
            break
        }
        catch [System.Net.HttpListenerException] {
            if ($previewStopRequested) {
                break
            }
            throw
        }

        $requestPath = $context.Request.Url.AbsolutePath.TrimStart('/')
        if ([string]::IsNullOrWhiteSpace($requestPath)) {
            $requestPath = "index.html"
        }

        $localPath = Join-Path $previewRoot $requestPath
        if ((-not (Test-Path $localPath)) -or (Get-Item $localPath).PSIsContainer) {
            $context.Response.StatusCode = 404
            $buffer = [System.Text.Encoding]::UTF8.GetBytes("Not found")
            $context.Response.OutputStream.Write($buffer, 0, $buffer.Length)
            $context.Response.Close()
            continue
        }

        $bytes = [System.IO.File]::ReadAllBytes($localPath)
        $context.Response.StatusCode = 200
        $context.Response.ContentType = Get-ContentType $localPath
        $context.Response.ContentLength64 = $bytes.Length
        $context.Response.OutputStream.Write($bytes, 0, $bytes.Length)
        $context.Response.Close()
    }
}
finally {
    if ($listener.IsListening) {
        $listener.Stop()
    }
    $listener.Close()
    if ($originalTreatControlCAsInput -ne $null) {
        [Console]::TreatControlCAsInput = $originalTreatControlCAsInput
    }
}
