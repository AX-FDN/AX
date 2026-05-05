param(
    [string] $OutputDir = "target\http-helpers-smoke"
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoScript = Join-Path $PSScriptRoot "cargo-gnu.ps1"
$repoCargoConfig = Join-Path $repoRoot ".cargo\config.toml"

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
    if ($env:AXC_BINARY -and (Test-Path $env:AXC_BINARY)) {
        return $env:AXC_BINARY
    }
    if ($env:CARGO_BIN_EXE_axc -and (Test-Path $env:CARGO_BIN_EXE_axc)) {
        return $env:CARGO_BIN_EXE_axc
    }
    return Join-Path (Resolve-TargetDir) "debug\axc.exe"
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

$outputRoot = Join-Path $repoRoot $OutputDir
if ([System.IO.Path]::IsPathRooted($OutputDir)) {
    $outputRoot = $OutputDir
}
if (Test-Path $outputRoot) {
    Remove-Item -LiteralPath $outputRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path (Join-Path $outputRoot "src") | Out-Null

$manifestText = @'
manifest_version = 1

[package]
name = "http_helpers_smoke"
entry = "src/main.ax"
sources = ["../../std"]
'@

$sourceText = @'
import std.http;

fn main() -> i32 {
    let query: string = std.http.query_pair("page", "1");
    let url: string = std.http.append_query("/v1/items", query);
    println(to_string(std.http.is_success_status(204)));
    println(to_string(std.http.is_retryable_status(503)));
    println(std.http.status_class(404));
    println(url);
    println(std.http.append_query("/v1/items?tag=ax", query));
    println(std.http.request_key("GET", url));
    println(std.http.accept_json_header());
    return string_len(std.http.request_key("GET", url));
}
'@

Write-Utf8NoBom -Path (Join-Path $outputRoot "AX.toml") -Text $manifestText
Write-Utf8NoBom -Path (Join-Path $outputRoot "src\main.ax") -Text $sourceText

$axcBinary = Ensure-AxcBinary

& $axcBinary check $outputRoot
Assert-Equal -Label "axc check exit code" -Actual $LASTEXITCODE -Expected 0

$runOutput = & $axcBinary run $outputRoot
Assert-Equal -Label "axc run exit code" -Actual $LASTEXITCODE -Expected 20

$actualOutput = @($runOutput | ForEach-Object { [string] $_ })
$expectedOutput = @(
    "true",
    "true",
    "client-error",
    "/v1/items?page=1",
    "/v1/items?tag=ax&page=1",
    "GET /v1/items?page=1",
    "Accept: application/json"
)
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
if ($features.Contains("host_http")) {
    Write-Error "Pure std.http helpers should not require host_http."
}
if (-not $features.Contains("string_runtime")) {
    Write-Error "AOT readiness did not report string_runtime for std.http string helpers."
}

Write-Host "HTTP helpers smoke passed at $outputRoot"

