param(
    [string] $OutputDir = "target\encoding-runtime-smoke"
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

$outputRoot = Resolve-RepoPath -Path $OutputDir
if (Test-Path $outputRoot) {
    Remove-Item -LiteralPath $outputRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path (Join-Path $outputRoot "src") | Out-Null

$manifestText = @'
manifest_version = 1

[package]
name = "encoding_runtime_smoke"
entry = "src/main.ax"
sources = ["../../std"]
'@

$sourceText = @'
import std.bytes;
import std.encoding;

fn main() -> i32 {
    let decoded: std.encoding.DecodeResult = std.encoding.hex_decode("415821");
    println(std.encoding.hex_encode_text("AX"));
    println(std.encoding.base64_encode_text("AX"));
    println(std.encoding.base64_urlsafe_no_pad("AX?"));
    println(std.bytes.to_string_lossy(decoded.data));
    println(to_string(decoded.ok));
    return std.bytes.length(decoded.data);
}
'@

Write-Utf8NoBom -Path (Join-Path $outputRoot "AX.toml") -Text $manifestText
Write-Utf8NoBom -Path (Join-Path $outputRoot "src\main.ax") -Text $sourceText

$axcBinary = Ensure-AxcBinary

& $axcBinary check $outputRoot
Assert-Equal -Label "axc check exit code" -Actual $LASTEXITCODE -Expected 0

$runOutput = & $axcBinary run $outputRoot
Assert-Equal -Label "axc run exit code" -Actual $LASTEXITCODE -Expected 3

$actualOutput = @($runOutput | ForEach-Object { [string] $_ })
$expectedOutput = @("4158", "QVg=", "QVg_", "AX!", "true")
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
if (-not $features.Contains("bytes_runtime")) {
    Write-Error "AOT readiness did not report bytes_runtime for std.encoding."
}
if (-not $manifest.artifacts.llvm_ir) {
    Write-Error "AOT IR artifact was not produced for std.encoding bytes runtime smoke."
}
if (@($manifest.aot_readiness.blockers | Where-Object { [string] $_.code -eq "AOT0303" }).Count -ne 0) {
    Write-Error "AOT readiness still reported AOT0303 after std.encoding bytes lowering landed."
}

Write-Host "Encoding runtime smoke passed at $outputRoot"
