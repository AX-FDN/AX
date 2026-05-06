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

function Assert-TextContains {
    param(
        [string] $Label,
        [string] $Text,
        [string] $Expected
    )

    if (-not $Text.Contains($Expected)) {
        Write-Error "$Label expected to contain '$Expected'."
    }
}

function Join-ProcessArguments {
    param([string[]] $Arguments)

    $quoted = @()
    foreach ($argument in $Arguments) {
        $text = [string] $argument
        if ($text.Length -eq 0) {
            $quoted += '""'
        } elseif ($text -match '[\s"]') {
            $quoted += '"' + $text.Replace('"', '\"') + '"'
        } else {
            $quoted += $text
        }
    }

    return ($quoted -join " ")
}

function Invoke-Process {
    param(
        [string] $FilePath,
        [string[]] $Arguments = @()
    )

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FilePath
    $startInfo.WorkingDirectory = $repoRoot
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.Arguments = Join-ProcessArguments $Arguments

    $process = $null
    for ($attempt = 1; $attempt -le 5; $attempt += 1) {
        try {
            $process = [System.Diagnostics.Process]::Start($startInfo)
            break
        } catch [System.ComponentModel.Win32Exception] {
            $message = $_.Exception.Message
            if ($attempt -eq 5 -or $message -notmatch "being used by another process") {
                throw
            }
            Start-Sleep -Milliseconds (200 * $attempt)
        }
    }

    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()

    [pscustomobject] @{
        ExitCode = $process.ExitCode
        Stdout = $stdout
        Stderr = $stderr
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

    $check = Invoke-Process -FilePath $axcBinary -Arguments @("check", $outputRoot)
    Assert-Equal -Label "axc check exit code" -Actual ([int] $check.ExitCode) -Expected 0

    $run = Invoke-Process -FilePath $axcBinary -Arguments @("run", $outputRoot)
    Assert-Equal -Label "axc run exit code" -Actual ([int] $run.ExitCode) -Expected 0

    $actualOutput = @(($run.Stdout -split "\r?\n") | Where-Object { $_ -ne "" } | ForEach-Object { [string] $_ })
    $expectedOutput = @("ok:200", "AX_HTTP_OK:0", "ok")
    Assert-Equal -Label "run output line count" -Actual $actualOutput.Count -Expected $expectedOutput.Count
    for ($index = 0; $index -lt $expectedOutput.Count; $index += 1) {
        Assert-Equal -Label "run output[$index]" -Actual $actualOutput[$index] -Expected $expectedOutput[$index]
    }

    $buildOutput = Join-Path $outputRoot "build"
    $build = Invoke-Process -FilePath $axcBinary -Arguments @("build", $outputRoot, "--emit", "ir", "--no-link", "--out-dir", $buildOutput)
    Assert-Equal -Label "axc build --emit ir --no-link exit code" -Actual ([int] $build.ExitCode) -Expected 0

    $manifestPath = Join-Path $buildOutput "build-manifest.json"
    if (-not (Test-Path $manifestPath)) {
        Write-Error "build manifest was not produced at $manifestPath"
    }
    $manifest = Get-Content $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
    Assert-Equal -Label "manifest schema_version" -Actual ([int] $manifest.schema_version) -Expected 10
    Assert-Equal -Label "aot readiness schema_version" -Actual ([int] $manifest.aot_readiness.schema_version) -Expected 3
    Assert-Equal -Label "host fixture aot_supported" -Actual ([bool] $manifest.aot_supported) -Expected $false

    $features = @($manifest.aot_readiness.required_backend_features | ForEach-Object { [string] $_ })
    if (-not $features.Contains("host_http")) {
        Write-Error "AOT readiness did not report host_http for std.http."
    }
    if (-not $features.Contains("host_net")) {
        Write-Error "AOT readiness did not report host_net for std.net."
    }
    $hostBlocker = @($manifest.aot_readiness.blockers | Where-Object { [string] $_.code -eq "AOT0301" }) | Select-Object -First 1
    if ($null -eq $hostBlocker) {
        Write-Error "AOT readiness did not report AOT0301 for host network runtime ABI."
    }
    Assert-Equal -Label "AOT0301 category" -Actual ([string] $hostBlocker.category) -Expected "runtime"
    Assert-Equal -Label "AOT0301 ai layer" -Actual ([string] $hostBlocker.ai.layer) -Expected "runtime_abi"
    Assert-Equal -Label "AOT0301 ai action" -Actual ([string] $hostBlocker.ai.ai_action) -Expected "explain_unsupported"
    Assert-Equal -Label "AOT0301 safe_to_edit" -Actual ([bool] $hostBlocker.ai.safe_to_edit) -Expected $false
    Assert-Equal -Label "AOT0301 rule id" -Actual ([string] $hostBlocker.ai.rule_id) -Expected "aot_host_runtime_abi_pending"

    $anchorRoot = Join-Path $outputRoot "host-abi-anchor"
    New-Item -ItemType Directory -Force -Path (Join-Path $anchorRoot "src") | Out-Null
    Write-Utf8NoBom -Path (Join-Path $anchorRoot "AX.toml") -Text @'
manifest_version = 1

[package]
name = "host_abi_anchor"
entry = "src/main.ax"
'@
    Write-Utf8NoBom -Path (Join-Path $anchorRoot "src\main.ax") -Text @'
fn main() -> i32 {
    return 0;
}
'@

    $anchorBuildOutput = Join-Path $anchorRoot "build"
    $anchorBuild = Invoke-Process -FilePath $axcBinary -Arguments @("build", $anchorRoot, "--emit", "ir", "--no-link", "--out-dir", $anchorBuildOutput)
    Assert-Equal -Label "host ABI anchor build exit code" -Actual ([int] $anchorBuild.ExitCode) -Expected 0
    $anchorManifestPath = Join-Path $anchorBuildOutput "build-manifest.json"
    if (-not (Test-Path $anchorManifestPath)) {
        Write-Error "host ABI anchor build manifest was not produced at $anchorManifestPath"
    }
    $anchorManifest = Get-Content $anchorManifestPath -Raw -Encoding utf8 | ConvertFrom-Json
    $llvmIrArtifact = [string] $anchorManifest.artifacts.llvm_ir
    if ([string]::IsNullOrWhiteSpace($llvmIrArtifact)) {
        Write-Error "host ABI anchor expected artifacts.llvm_ir."
    }
    $llvmIrPath = Join-Path $anchorBuildOutput $llvmIrArtifact
    if (-not (Test-Path $llvmIrPath)) {
        Write-Error "host ABI anchor LLVM IR artifact is missing: $llvmIrPath"
    }
    $llvmIr = Get-Content $llvmIrPath -Raw -Encoding utf8
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "; host handle ABI: ax.host.handle_v0"
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "; host handle layout: header=16 kind_off=0 native_off=8"
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "; host error ABI: ax.host.error_v0"
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "define private { i32, ptr } @ax_host_error_ok()"
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "define private { i32, ptr } @ax_host_error_new(i32 %code, ptr %message)"
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "define private ptr @ax_host_handle_new(i32 %kind, ptr %native)"
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "define private i32 @ax_host_handle_kind(ptr %handle)"
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "define private void @ax_tcp_socket_release(ptr %socket)"
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "call void @free(ptr %socket)"
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "define private void @ax_tls_stream_release(ptr %stream)"
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "define private void @ax_http_client_release(ptr %client)"
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "define private void @ax_http_server_release(ptr %server)"
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "define private void @ax_db_connection_release(ptr %connection)"
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "define private void @ax_async_task_release(ptr %task)"
    Assert-TextContains -Label "host LLVM IR" -Text $llvmIr -Expected "define private void @ax_timer_release(ptr %timer)"

    Write-Host "Host network runtime smoke passed at $outputRoot"
} finally {
    if ($serverJob.State -eq "Running") {
        Stop-Job $serverJob | Out-Null
    }
    Receive-Job $serverJob -ErrorAction SilentlyContinue | Out-Null
    Remove-Job $serverJob -Force -ErrorAction SilentlyContinue
}
