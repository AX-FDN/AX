param(
    [string] $OutputDir = "target\host-network-runtime-smoke"
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoScript = Join-Path $PSScriptRoot "cargo-gnu.ps1"
$repoCargoConfig = Join-Path $repoRoot ".cargo\config.toml"

function Resolve-RepoPath {
    param([string] $Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return Join-Path $repoRoot $Path
}

function Resolve-TargetDir {
    if ($env:CARGO_TARGET_DIR) {
        return $env:CARGO_TARGET_DIR
    }

    if (Test-Path $repoCargoConfig) {
        $configText = Get-Content $repoCargoConfig -Raw -Encoding utf8
        if ($configText -match 'target-dir\s*=\s*"([^"]+)"') {
            return $matches[1]
        }
    }

    return Join-Path $repoRoot "target"
}

function Resolve-AxcBinary {
    if (-not [string]::IsNullOrWhiteSpace($env:AXC_BINARY) -and (Test-Path $env:AXC_BINARY)) {
        return [string] $env:AXC_BINARY
    }

    $targetDir = Resolve-TargetDir
    return Join-Path $targetDir "debug\axc.exe"
}

function Ensure-AxcBinary {
    $binary = Resolve-AxcBinary
    if (Test-Path $binary) {
        return $binary
    }

    & $cargoScript build --bin axc | Out-Null
    $binary = Resolve-AxcBinary
    if (-not (Test-Path $binary)) {
        Write-Error "Could not find compiled AX binary after build: $binary"
    }
    return $binary
}

function Write-Utf8NoBom {
    param(
        [string] $Path,
        [string] $Text
    )

    $encoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Text, $encoding)
}

function Assert-Equal {
    param(
        [string] $Label,
        $Actual,
        $Expected
    )

    if ($Actual -ne $Expected) {
        Write-Error "$Label expected '$Expected' but got '$Actual'."
    }
}

function Get-FreeTcpPort {
    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, 0)
    $listener.Start()
    try {
        return [int] $listener.LocalEndpoint.Port
    } finally {
        $listener.Stop()
    }
}

$outputRoot = Resolve-RepoPath -Path $OutputDir
if (Test-Path $outputRoot) {
    Remove-Item -LiteralPath $outputRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path (Join-Path $outputRoot "src") | Out-Null

$port = Get-FreeTcpPort
$serverJob = Start-Job -ArgumentList $port -ScriptBlock {
    param([int] $Port)

    $listener = [System.Net.Sockets.TcpListener]::new([System.Net.IPAddress]::Loopback, $Port)
    $listener.Start()
    try {
        for ($index = 0; $index -lt 2; $index += 1) {
            $client = $listener.AcceptTcpClient()
            try {
                $client.ReceiveTimeout = 3000
                $stream = $client.GetStream()
                $buffer = New-Object byte[] 4096
                $request = ""
                while ($true) {
                    try {
                        $read = $stream.Read($buffer, 0, $buffer.Length)
                    } catch {
                        break
                    }
                    if ($read -le 0) {
                        break
                    }
                    $request += [System.Text.Encoding]::UTF8.GetString($buffer, 0, $read)
                    if ($request -match "(\r?\n){2}") {
                        break
                    }
                }

                $body = "AX_HTTP_OK:$index"
                $response = "HTTP/1.0 200 OK`r`nContent-Length: $($body.Length)`r`nConnection: close`r`n`r`n$body"
                $bytes = [System.Text.Encoding]::UTF8.GetBytes($response)
                $stream.Write($bytes, 0, $bytes.Length)
            } finally {
                $client.Close()
            }
        }
    } finally {
        $listener.Stop()
    }
}

Start-Sleep -Milliseconds 300

try {
    $manifestText = @'
manifest_version = 1

[package]
name = "host_network_runtime_smoke"
entry = "src/main.ax"
sources = ["../../std"]
'@

    $sourceText = @"
import std.http;
import std.net;

fn main() -> i32 {
    let url: string = "http://127.0.0.1:$port/hello";
    let response: std.http.HttpResponse = std.http.get(url);
    println(std.http.status_text(response));
    println(response.body);

    let tcp: std.net.TcpResponse = std.net.tcp_exchange("127.0.0.1", $port, "PING");
    println(std.net.status_text(tcp));

    if (response.ok && tcp.ok && string_contains(response.body, "AX_HTTP_OK:0") && string_contains(tcp.data, "AX_HTTP_OK:1")) {
        return 0;
    }
    return 1;
}
"@

    Write-Utf8NoBom -Path (Join-Path $outputRoot "AX.toml") -Text $manifestText
    Write-Utf8NoBom -Path (Join-Path $outputRoot "src\main.ax") -Text $sourceText

    $axcBinary = Ensure-AxcBinary

    & $axcBinary check $outputRoot
    Assert-Equal -Label "axc check exit code" -Actual $LASTEXITCODE -Expected 0

    $runOutput = & $axcBinary run $outputRoot
    Assert-Equal -Label "axc run exit code" -Actual $LASTEXITCODE -Expected 0

    $actualOutput = @($runOutput | ForEach-Object { [string] $_ })
    $expectedOutput = @("ok:200", "AX_HTTP_OK:0", "ok")
    Assert-Equal -Label "run output line count" -Actual $actualOutput.Count -Expected $expectedOutput.Count
    for ($index = 0; $index -lt $expectedOutput.Count; $index += 1) {
        Assert-Equal -Label "run output[$index]" -Actual $actualOutput[$index] -Expected $expectedOutput[$index]
    }

    $buildOutput = Join-Path $outputRoot "build"
    & $axcBinary build $outputRoot --emit ir --no-link --out-dir $buildOutput | Out-Null
    Assert-Equal -Label "axc build --emit ir --no-link exit code" -Actual $LASTEXITCODE -Expected 0

    $manifestPath = Join-Path $buildOutput "build-manifest.json"
    if (-not (Test-Path $manifestPath)) {
        Write-Error "build manifest was not produced at $manifestPath"
    }
    $manifest = Get-Content $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
    $features = @($manifest.aot_readiness.required_backend_features | ForEach-Object { [string] $_ })
    if (-not $features.Contains("host_http")) {
        Write-Error "AOT readiness did not report host_http for std.http."
    }
    if (-not $features.Contains("host_net")) {
        Write-Error "AOT readiness did not report host_net for std.net."
    }
    $blockerCodes = @($manifest.aot_readiness.blockers | ForEach-Object { [string] $_.code })
    if (-not $blockerCodes.Contains("AOT0301")) {
        Write-Error "AOT readiness did not report AOT0301 for host network runtime ABI."
    }

    Write-Host "Host network runtime smoke passed at $outputRoot"
} finally {
    if ($serverJob.State -eq "Running") {
        Stop-Job $serverJob | Out-Null
    }
    Receive-Job $serverJob -ErrorAction SilentlyContinue | Out-Null
    Remove-Job $serverJob -Force -ErrorAction SilentlyContinue
}
