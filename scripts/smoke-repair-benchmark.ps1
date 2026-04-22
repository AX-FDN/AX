param(
    [string] $ManifestPath = "benchmarks\\repair-cases-smoke.json",
    [string] $SourceDir = "benchmarks\\repair-candidates\\smoke",
    [string] $BenchmarkDir = ".ax-ai\\repair-benchmark\\ci-smoke",
    [string] $OutputDir = ".ax-ai\\repair-runs\\ci-smoke"
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$exportScript = Join-Path $PSScriptRoot "export-repair-benchmark.ps1"
$runScript = Join-Path $PSScriptRoot "run-repair-benchmark.ps1"
$replayAdapter = Join-Path $PSScriptRoot "replay-repair-adapter.ps1"

function Resolve-RepoPath {
    param([string] $Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return Join-Path $repoRoot $Path
}

function Remove-RepoDirectoryIfExists {
    param([string] $Path)

    $resolved = Resolve-RepoPath -Path $Path
    if (-not (Test-Path $resolved)) {
        return
    }

    Remove-Item -LiteralPath $resolved -Recurse -Force
}

$manifestPath = Resolve-RepoPath -Path $ManifestPath
$sourceDir = Resolve-RepoPath -Path $SourceDir
$benchmarkDir = Resolve-RepoPath -Path $BenchmarkDir
$outputDir = Resolve-RepoPath -Path $OutputDir

if (-not (Test-Path $manifestPath)) {
    Write-Error "Smoke manifest not found: $manifestPath"
}

if (-not (Test-Path $sourceDir)) {
    Write-Error "Smoke replay source directory not found: $sourceDir"
}

Remove-RepoDirectoryIfExists -Path $BenchmarkDir
Remove-RepoDirectoryIfExists -Path $OutputDir

& $exportScript -ManifestPath $manifestPath -OutputDir $benchmarkDir | Out-Null
& $runScript `
    -BenchmarkDir $benchmarkDir `
    -RunnerScript $replayAdapter `
    -RunnerExtraArgs @("-SourceDir", $sourceDir) `
    -FeedbackMode ai `
    -OutputDir $outputDir
