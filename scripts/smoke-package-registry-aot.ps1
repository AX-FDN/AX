param(
    [string] $OutputDir = "target\package-registry-aot-smoke",
    [string] $Registry = "registry"
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$packageSmoke = Join-Path $PSScriptRoot "smoke-package-registry.ps1"
$aotParitySmoke = Join-Path $PSScriptRoot "smoke-aot-parity.ps1"

function Resolve-RepoPath {
    param([string] $Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return Join-Path $repoRoot $Path
}

$outputRoot = Resolve-RepoPath -Path $OutputDir
$registryRoot = Resolve-RepoPath -Path $Registry
$projectRoot = Join-Path $outputRoot "project"
$aotRoot = Join-Path $outputRoot "aot"

if (Test-Path $outputRoot) {
    Remove-Item -LiteralPath $outputRoot -Recurse -Force
}

& powershell -NoProfile -ExecutionPolicy Bypass -File $packageSmoke `
    -OutputDir $projectRoot `
    -Registry $registryRoot

if ($LASTEXITCODE -ne 0) {
    Write-Error "package registry setup smoke failed with exit code $LASTEXITCODE"
}

& powershell -NoProfile -ExecutionPolicy Bypass -File $aotParitySmoke `
    -SourcePath $projectRoot `
    -OutputRoot $aotRoot

if ($LASTEXITCODE -ne 0) {
    Write-Error "package registry AOT parity smoke failed with exit code $LASTEXITCODE"
}

Write-Host "Package registry AOT parity smoke passed at $outputRoot"
