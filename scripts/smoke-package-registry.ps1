param(
    [string] $OutputDir = "target\package-registry-smoke",
    [string] $Registry = "registry"
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
$registryRoot = Resolve-RepoPath -Path $Registry

if (Test-Path $outputRoot) {
    Remove-Item -LiteralPath $outputRoot -Recurse -Force
}

New-Item -ItemType Directory -Force -Path (Join-Path $outputRoot "src") | Out-Null

$manifestText = @'
manifest_version = 1

[package]
name = "package_registry_smoke"
entry = "src/main.ax"
'@

$sourceText = @'
import collection_tools.ints;
import config_rules.validate;
import markdown_tools.headings;
import math_rules.core;
import number_tools.core;
import report_tools.builder;
import result_tools.summary;
import text_tools.normalize;
import text_tools.stats;
import validation_tools.rules;

fn main() -> i32 {
    let normalized: string = text_tools.normalize.normalize_document("# Title\n\n\tbody");
    let summary: text_tools.stats.TextSummary = text_tools.stats.analyze(normalized);
    let score: i32 = math_rules.core.score(summary.nonempty);
    let values: [i32] = [score, summary.headings, summary.nonempty];
    let totals: collection_tools.ints.IntSummary = collection_tools.ints.summarize(values);
    let bounded: number_tools.core.RangeCheck = number_tools.core.range_check(totals.sum, 0, 20);
    let headings: markdown_tools.headings.HeadingSummary = markdown_tools.headings.summarize(normalized);
    let config_status: i32 = config_rules.validate.validate("host=localhost\nport=8080\n");
    let name_status: i32 = validation_tools.rules.require_prefix("AX-PKG", "AX");
    let mut report: string = "";
    report = report_tools.builder.section(report, "registry package preview");
    report = report_tools.builder.kv_i32(report, "score", score);
    report = report_tools.builder.kv_i32(report, "sum", totals.sum);
    report = report_tools.builder.kv_string(report, "band", number_tools.core.score_band(score));
    report = report_tools.builder.kv_bool(report, "range-ok", bounded.ok);
    report = report_tools.builder.kv_i32(report, "headings", headings.headings);
    report = report_tools.builder.kv_string(report, "name", validation_tools.rules.message(name_status, "name"));
    println(result_tools.summary.status_label(config_status, "ok"));
    println(report);
    return result_tools.summary.exit_code(config_status);
}
'@

Write-Utf8NoBom -Path (Join-Path $outputRoot "AX.toml") -Text $manifestText
Write-Utf8NoBom -Path (Join-Path $outputRoot "src\main.ax") -Text $sourceText

$axcBinary = Ensure-AxcBinary

& $axcBinary pkg check --registry $registryRoot
Assert-Equal -Label "axc pkg check exit code" -Actual $LASTEXITCODE -Expected 0

$packages = @(
    "collection_tools",
    "config_rules",
    "markdown_tools",
    "math_rules",
    "number_tools",
    "report_tools",
    "result_tools",
    "text_tools",
    "validation_tools"
)

foreach ($package in $packages) {
    & $axcBinary pkg add $package $outputRoot --registry $registryRoot
    Assert-Equal -Label "axc pkg add $package exit code" -Actual $LASTEXITCODE -Expected 0
}

& $axcBinary pkg install $outputRoot --registry $registryRoot
Assert-Equal -Label "axc pkg install exit code" -Actual $LASTEXITCODE -Expected 0

$lockfilePath = Join-Path $outputRoot "AX.lock"
if (-not (Test-Path $lockfilePath)) {
    Write-Error "Package registry smoke did not produce AX.lock at $lockfilePath"
}

$lockfile = Get-Content $lockfilePath -Raw -Encoding utf8 | ConvertFrom-Json
Assert-Equal -Label "AX.lock schema_version" -Actual ([int] $lockfile.schema_version) -Expected 2
Assert-Equal -Label "AX.lock dependency count" -Actual (@($lockfile.dependencies).Count) -Expected 9

& $axcBinary check $outputRoot
Assert-Equal -Label "axc check exit code" -Actual $LASTEXITCODE -Expected 0

$runOutput = & $axcBinary run $outputRoot
Assert-Equal -Label "axc run exit code" -Actual $LASTEXITCODE -Expected 0

$expectedOutput = @(
    "ok",
    "== registry package preview ==",
    "score: 9",
    "sum: 12",
    "band: excellent",
    "range-ok: true",
    "headings: 1",
    "name: name: ok"
)
$actualOutput = @($runOutput | ForEach-Object { [string] $_ })
while ($actualOutput.Count -gt 0 -and $actualOutput[$actualOutput.Count - 1] -eq "") {
    if ($actualOutput.Count -eq 1) {
        $actualOutput = @()
    } else {
        $actualOutput = @($actualOutput[0..($actualOutput.Count - 2)])
    }
}
Assert-Equal -Label "run output line count" -Actual $actualOutput.Count -Expected $expectedOutput.Count
for ($index = 0; $index -lt $expectedOutput.Count; $index += 1) {
    Assert-Equal -Label "run output[$index]" -Actual $actualOutput[$index] -Expected $expectedOutput[$index]
}

Write-Host "Package registry smoke passed. Verified 9 registry packages at $outputRoot"
