param(
    [string] $OutputDir = "target\bytes-runtime-smoke"
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
    $overrideCandidates = @()
    if (-not [string]::IsNullOrWhiteSpace($env:AXC_BINARY)) {
        $overrideCandidates += [string] $env:AXC_BINARY
    }
    if (-not [string]::IsNullOrWhiteSpace($env:CARGO_BIN_EXE_axc)) {
        $overrideCandidates += [string] $env:CARGO_BIN_EXE_axc
    }

    foreach ($candidate in $overrideCandidates) {
        if (Test-Path $candidate) {
            return $candidate
        }
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

$outputRoot = Resolve-RepoPath -Path $OutputDir
if (Test-Path $outputRoot) {
    Remove-Item -LiteralPath $outputRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path (Join-Path $outputRoot "src") | Out-Null

$manifestText = @'
manifest_version = 1

[package]
name = "bytes_runtime_smoke"
entry = "src/main.ax"
sources = ["../../std"]
'@

$sourceText = @'
import std.bytes;

fn main() -> i32 {
    let data: bytes = std.bytes.from_string("AX");
    let more: bytes = std.bytes.push(data, 33);
    println(std.bytes.to_hex(more));
    println(std.bytes.to_string_lossy(more));
    println(std.bytes.get(more, 0));
    return std.bytes.length(more);
}
'@

Write-Utf8NoBom -Path (Join-Path $outputRoot "AX.toml") -Text $manifestText
Write-Utf8NoBom -Path (Join-Path $outputRoot "src\main.ax") -Text $sourceText

$axcBinary = Ensure-AxcBinary

$check = Invoke-Process -FilePath $axcBinary -Arguments @("check", $outputRoot)
Assert-Equal -Label "axc check exit code" -Actual ([int] $check.ExitCode) -Expected 0

$run = Invoke-Process -FilePath $axcBinary -Arguments @("run", $outputRoot)
Assert-Equal -Label "axc run exit code" -Actual ([int] $run.ExitCode) -Expected 3

$actualOutput = @(($run.Stdout -split "\r?\n") | Where-Object { $_ -ne "" } | ForEach-Object { [string] $_ })
$expectedOutput = @("415821", "AX!", "65")
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
Assert-Equal -Label "bytes fixture aot_supported" -Actual ([bool] $manifest.aot_supported) -Expected $false
Assert-Equal -Label "backend status" -Actual ([string] $manifest.backend.status) -Expected "ir_generated"
if (-not $manifest.artifacts.llvm_ir) {
    Write-Error "AOT IR artifact was not produced for bytes runtime smoke."
}

$features = @($manifest.aot_readiness.required_backend_features | ForEach-Object { [string] $_ })
if (-not $features.Contains("bytes_runtime")) {
    Write-Error "AOT readiness did not report bytes_runtime for std.bytes."
}

if (@($manifest.aot_readiness.blockers | Where-Object { [string] $_.code -eq "AOT0303" }).Count -ne 0) {
    Write-Error "AOT readiness still reported AOT0303 after bytes runtime helpers landed."
}

if (@($manifest.aot_readiness.blockers | Where-Object { [string] $_.code -eq "AOT1001" }).Count -ne 0) {
    Write-Error "IR-only bytes smoke should not require AOT1001."
}
Write-Host "Bytes runtime smoke passed at $outputRoot"
