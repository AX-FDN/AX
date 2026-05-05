param(
    [string] $OutputDir = "target\package-registry-aot-smoke",
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
    if (-not [string]::IsNullOrWhiteSpace($env:AXC_BINARY) -and (Test-Path $env:AXC_BINARY)) {
        return [string] $env:AXC_BINARY
    }

    if (-not [string]::IsNullOrWhiteSpace($env:CARGO_BIN_EXE_axc) -and (Test-Path $env:CARGO_BIN_EXE_axc)) {
        return [string] $env:CARGO_BIN_EXE_axc
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

function Assert-Contains {
    param(
        [string] $Label,
        [string[]] $Values,
        [string] $Expected
    )

    if (-not $Values.Contains($Expected)) {
        Write-Error "$Label expected to contain '$Expected' but found: $($Values -join ', ')"
    }
}

function Assert-NotContains {
    param(
        [string] $Label,
        [string[]] $Values,
        [string] $Unexpected
    )

    if ($Values.Contains($Unexpected)) {
        Write-Error "$Label must not contain '$Unexpected'."
    }
}

function New-FixtureProject {
    param(
        [string] $Name,
        [string] $DependencyLine,
        [string] $SourceText,
        [bool] $IncludeStd
    )

    $projectRoot = Join-Path $outputRoot $Name
    New-Item -ItemType Directory -Force -Path (Join-Path $projectRoot "src") | Out-Null

    $sourcesLine = ""
    if ($IncludeStd) {
        $sourcesLine = 'sources = ["../../../std"]'
    }

    $manifestText = @"
manifest_version = 1

[package]
name = "$Name"
entry = "src/main.ax"
$sourcesLine

[dependencies]
$DependencyLine
"@

    Write-Utf8NoBom -Path (Join-Path $projectRoot "AX.toml") -Text $manifestText
    Write-Utf8NoBom -Path (Join-Path $projectRoot "src\main.ax") -Text $SourceText
    return $projectRoot
}

function Invoke-FixtureBuild {
    param(
        [string] $Name,
        [string] $DependencyLine,
        [string] $SourceText,
        [bool] $IncludeStd,
        [string[]] $ExpectedBlockers,
        [string[]] $UnexpectedBlockers,
        [string[]] $ExpectedFeatures
    )

    $projectRoot = New-FixtureProject `
        -Name $Name `
        -DependencyLine $DependencyLine `
        -SourceText $SourceText `
        -IncludeStd $IncludeStd

    & $axcBinary pkg install $projectRoot --registry $registryRoot | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Error "axc pkg install failed for $Name with exit code $LASTEXITCODE"
    }

    $buildRoot = Join-Path $outputRoot "$Name-build"
    & $axcBinary build $projectRoot --emit ir --out-dir $buildRoot --json | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Error "axc build failed for $Name with exit code $LASTEXITCODE"
    }

    $manifestPath = Join-Path $buildRoot "build-manifest.json"
    if (-not (Test-Path $manifestPath)) {
        Write-Error "build manifest was not produced for $Name at $manifestPath"
    }
    $manifest = Get-Content $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
    $blockerCodes = @($manifest.aot_readiness.blockers | ForEach-Object { [string] $_.code })
    $features = @($manifest.aot_readiness.required_backend_features | ForEach-Object { [string] $_ })
    $registryPackages = @($manifest.registry_packages)

    if ($registryPackages.Count -lt 1) {
        Write-Error "$Name expected registry_packages in build manifest"
    }

    foreach ($expected in $ExpectedBlockers) {
        Assert-Contains -Label "$Name blockers" -Values $blockerCodes -Expected $expected
    }
    foreach ($unexpected in $UnexpectedBlockers) {
        Assert-NotContains -Label "$Name blockers" -Values $blockerCodes -Unexpected $unexpected
    }
    foreach ($feature in $ExpectedFeatures) {
        Assert-Contains -Label "$Name features" -Values $features -Expected $feature
    }
}

$outputRoot = Resolve-RepoPath -Path $OutputDir
$registryRoot = Resolve-RepoPath -Path $Registry

if (Test-Path $outputRoot) {
    Remove-Item -LiteralPath $outputRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

$axcBinary = Ensure-AxcBinary

Invoke-FixtureBuild `
    -Name "registry_stable_pure_ax_aot" `
    -DependencyLine 'json_tools = { registry = "ax", version = "0.1.0" }' `
    -IncludeStd $false `
    -SourceText @'
import json_tools.encode;

fn main() -> i32 {
    let value: string = json_tools.encode.object1(json_tools.encode.field_string("status", "ok"));
    println(value);
    return 0;
}
'@ `
    -ExpectedBlockers @() `
    -UnexpectedBlockers @("AOT0104", "AOT0105") `
    -ExpectedFeatures @("registry_packages", "registry_package_stable_pure_ax")

Invoke-FixtureBuild `
    -Name "registry_host_boundary_aot" `
    -DependencyLine 'http_tools = { registry = "ax", version = "0.1.0" }' `
    -IncludeStd $true `
    -SourceText @'
import http_tools.client;

fn main() -> i32 {
    let label: string = http_tools.client.status_class(200);
    println(label);
    return 0;
}
'@ `
    -ExpectedBlockers @("AOT0104") `
    -UnexpectedBlockers @("AOT0105") `
    -ExpectedFeatures @("registry_packages", "registry_package_host_boundary_preview")

Invoke-FixtureBuild `
    -Name "registry_future_native_aot" `
    -DependencyLine 'auth_tools = { registry = "ax", version = "0.1.0" }' `
    -IncludeStd $false `
    -SourceText @'
import auth_tools.headers;

fn main() -> i32 {
    println(auth_tools.headers.safe_header_preview("Authorization", "secret-token"));
    return 0;
}
'@ `
    -ExpectedBlockers @("AOT0105") `
    -UnexpectedBlockers @("AOT0104") `
    -ExpectedFeatures @("registry_packages", "registry_package_future_native_preview")

Write-Host "Package registry AOT readiness smoke passed at $outputRoot"
