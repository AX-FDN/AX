param(
    [string] $SourcePath = "examples/aot_return.ax",
    [string] $OutputDir = "",
    [int] $ExpectedExitCode = 42
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoScript = Join-Path $PSScriptRoot "cargo-gnu.ps1"
$manifestPath = Join-Path $repoRoot "Cargo.toml"

function Resolve-RepoPath {
    param([string] $Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return Join-Path $repoRoot $Path
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

function Resolve-Clang {
    if (-not [string]::IsNullOrWhiteSpace($env:AX_LLVM_CLANG)) {
        return [string] $env:AX_LLVM_CLANG
    }

    $command = Get-Command clang -ErrorAction SilentlyContinue
    if ($command) {
        return [string] $command.Source
    }

    Write-Error "clang was not found. Install clang or set AX_LLVM_CLANG before running the AOT link smoke."
}

$sourcePath = Resolve-RepoPath -Path $SourcePath
if (-not (Test-Path $sourcePath)) {
    Write-Error "AOT smoke source not found: $sourcePath"
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $tempRoot = if (-not [string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) {
        [string] $env:RUNNER_TEMP
    } else {
        [System.IO.Path]::GetTempPath()
    }
    $OutputDir = Join-Path $tempRoot "ax-aot-link-smoke"
} else {
    $OutputDir = Resolve-RepoPath -Path $OutputDir
}

if (Test-Path $OutputDir) {
    Remove-Item -LiteralPath $OutputDir -Recurse -Force
}

$clang = Resolve-Clang
Write-Host "Using clang for AOT smoke: $clang"

$previousLink = $env:AX_LLVM_AOT_LINK
$previousClang = $env:AX_LLVM_CLANG
$env:AX_LLVM_AOT_LINK = "1"
$env:AX_LLVM_CLANG = $clang

try {
    $isWindows = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
        [System.Runtime.InteropServices.OSPlatform]::Windows
    )

    if ($isWindows) {
        & $cargoScript run -- build $sourcePath --out-dir $OutputDir
    } else {
        & cargo run --locked --manifest-path $manifestPath -- build $sourcePath --out-dir $OutputDir
    }

    Assert-Equal -Label "axc build exit code" -Actual $LASTEXITCODE -Expected 0
} finally {
    if ($null -eq $previousLink) {
        Remove-Item Env:\AX_LLVM_AOT_LINK -ErrorAction SilentlyContinue
    } else {
        $env:AX_LLVM_AOT_LINK = $previousLink
    }

    if ($null -eq $previousClang) {
        Remove-Item Env:\AX_LLVM_CLANG -ErrorAction SilentlyContinue
    } else {
        $env:AX_LLVM_CLANG = $previousClang
    }
}

$buildManifestPath = Join-Path $OutputDir "build-manifest.json"
if (-not (Test-Path $buildManifestPath)) {
    Write-Error "AOT smoke did not produce build-manifest.json at $buildManifestPath"
}

$manifest = Get-Content $buildManifestPath -Raw -Encoding utf8 | ConvertFrom-Json

Assert-Equal -Label "backend.kind" -Actual ([string] $manifest.backend.kind) -Expected "llvm-aot"
Assert-Equal -Label "backend.status" -Actual ([string] $manifest.backend.status) -Expected "built"
Assert-Equal -Label "aot_readiness.status" -Actual ([string] $manifest.aot_readiness.status) -Expected "built"
Assert-Equal -Label "aot_readiness.executable_emission" -Actual ([bool] $manifest.aot_readiness.executable_emission) -Expected $true

$llvmIrArtifact = [string] $manifest.artifacts.llvm_ir
if ([string]::IsNullOrWhiteSpace($llvmIrArtifact)) {
    Write-Error "AOT smoke manifest should include artifacts.llvm_ir."
}

$executableArtifact = [string] $manifest.artifacts.executable
if ([string]::IsNullOrWhiteSpace($executableArtifact)) {
    Write-Error "AOT smoke manifest should include artifacts.executable after linking."
}

$llvmIrPath = Join-Path $OutputDir $llvmIrArtifact
$executablePath = Join-Path $OutputDir $executableArtifact

if (-not (Test-Path $llvmIrPath)) {
    Write-Error "AOT smoke LLVM IR artifact is missing: $llvmIrPath"
}

if (-not (Test-Path $executablePath)) {
    Write-Error "AOT smoke executable artifact is missing: $executablePath"
}

& $executablePath
$actualExitCode = $LASTEXITCODE
Assert-Equal -Label "AOT executable exit code" -Actual $actualExitCode -Expected $ExpectedExitCode

Write-Host "LLVM AOT link smoke passed. Executable $executablePath exited with $actualExitCode."
