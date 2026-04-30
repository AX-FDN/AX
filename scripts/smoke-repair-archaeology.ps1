param(
    [string] $ManifestPath = "benchmarks\repair-cases-smoke.json",
    [string] $SharedSourceDir = "benchmarks\repair-candidates\smoke",
    [string] $BaseSourceDir = "benchmarks\repair-candidates\compare\base",
    [string] $BenchmarkDir = ".ax-ai\repair-benchmark\ci-archaeology",
    [string] $ComparisonDir = ".ax-ai\repair-comparisons\ci-archaeology",
    [string] $OutputDir = ".ax-ai\repair-archaeology\ci-smoke",
    [switch] $SkipBuild
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$exportScript = Join-Path $PSScriptRoot "export-repair-benchmark.ps1"
$compareScript = Join-Path $PSScriptRoot "compare-repair-feedback.ps1"
$archaeologyScript = Join-Path $PSScriptRoot "export-repair-archaeology.ps1"
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

function Assert-FileExists {
    param(
        [string] $Label,
        [string] $Path
    )

    if (-not (Test-Path $Path)) {
        Write-Error "$Label not found: $Path"
    }
}

function Read-JsonFile {
    param(
        [string] $Path,
        [string] $Label
    )

    Assert-FileExists -Label $Label -Path $Path
    try {
        return Get-Content $Path -Raw -Encoding utf8 | ConvertFrom-Json
    } catch {
        Write-Error "Failed to parse ${Label}: $($_.Exception.Message)"
    }
}

$manifestPath = Resolve-RepoPath -Path $ManifestPath
$sharedSourceDir = Resolve-RepoPath -Path $SharedSourceDir
$baseSourceDir = Resolve-RepoPath -Path $BaseSourceDir
$benchmarkDir = Resolve-RepoPath -Path $BenchmarkDir
$comparisonDir = Resolve-RepoPath -Path $ComparisonDir
$outputDir = Resolve-RepoPath -Path $OutputDir

Assert-FileExists -Label "Repair archaeology smoke manifest" -Path $manifestPath
Assert-FileExists -Label "Shared replay source directory" -Path $sharedSourceDir
Assert-FileExists -Label "Base replay source directory" -Path $baseSourceDir

Remove-RepoDirectoryIfExists -Path $BenchmarkDir
Remove-RepoDirectoryIfExists -Path $ComparisonDir
Remove-RepoDirectoryIfExists -Path $OutputDir

& $exportScript `
    -ManifestPath $manifestPath `
    -OutputDir $benchmarkDir `
    -SkipBuild:$SkipBuild | Out-Null

& $compareScript `
    -BenchmarkDir $benchmarkDir `
    -RunnerScript $replayAdapter `
    -RunnerExtraArgs @(
        "-SourceDir", $sharedSourceDir,
        "-SourceDirBase", $baseSourceDir
    ) `
    -OutputDir $comparisonDir `
    -SkipBuild:$SkipBuild | Out-Null

$comparisonPath = Join-Path $comparisonDir "comparison.json"
$comparison = Read-JsonFile -Path $comparisonPath -Label "Repair archaeology comparison"

Assert-Equal -Label "comparison.schema_version" -Actual ([int] $comparison.schema_version) -Expected 1
Assert-Equal -Label "comparison.total_cases" -Actual ([int] $comparison.comparison.total_cases) -Expected 12
Assert-Equal -Label "comparison.base_passed" -Actual ([int] $comparison.comparison.base_passed) -Expected 6
Assert-Equal -Label "comparison.ai_passed" -Actual ([int] $comparison.comparison.ai_passed) -Expected 12

$caseIds = @(
    "missing_semicolon_basic",
    "type_mismatch_bool_from_int",
    "slice_assignment_read_only"
)

& $archaeologyScript `
    -ComparisonPath $comparisonPath `
    -OutputDir $outputDir `
    -CaseIds $caseIds | Out-Null

$indexPath = Join-Path $outputDir "index.json"
$index = Read-JsonFile -Path $indexPath -Label "Repair archaeology index"

Assert-Equal -Label "index.schema_version" -Actual ([int] $index.schema_version) -Expected 1
Assert-Equal -Label "index.source_kind" -Actual ([string] $index.source_kind) -Expected "deterministic_replay"
Assert-Equal -Label "index.live_model_claim" -Actual ([bool] $index.live_model_claim) -Expected $false
Assert-Equal -Label "index.totals.total" -Actual ([int] $index.totals.total) -Expected 3
Assert-Equal -Label "index.totals.both_pass" -Actual ([int] $index.totals.both_pass) -Expected 1
Assert-Equal -Label "index.totals.improved" -Actual ([int] $index.totals.improved) -Expected 2
Assert-Equal -Label "index.cases count" -Actual (@($index.cases).Count) -Expected 3

foreach ($case in @($index.cases)) {
    $caseId = [string] $case.id
    $caseJsonPath = Join-Path $outputDir ([string] $case.json)
    $caseMarkdownPath = Join-Path $outputDir ([string] $case.markdown)
    $caseJson = Read-JsonFile -Path $caseJsonPath -Label "Repair archaeology case JSON $caseId"
    Assert-FileExists -Label "Repair archaeology case Markdown $caseId" -Path $caseMarkdownPath

    Assert-Equal -Label "$caseId.schema_version" -Actual ([int] $caseJson.schema_version) -Expected 1
    Assert-Equal -Label "$caseId.provenance.source_kind" -Actual ([string] $caseJson.provenance.source_kind) -Expected "deterministic_replay"
    Assert-Equal -Label "$caseId.provenance.live_model_claim" -Actual ([bool] $caseJson.provenance.live_model_claim) -Expected $false
    Assert-Equal -Label "$caseId.case.id" -Actual ([string] $caseJson.case.id) -Expected $caseId

    $markdown = Get-Content $caseMarkdownPath -Raw -Encoding utf8
    if ($markdown -notmatch "Claim boundary: deterministic replay, not live-model evidence") {
        Write-Error "Repair archaeology Markdown for '$caseId' does not include the claim boundary."
    }
}

Write-Host "Repair archaeology smoke passed. Stable index and case JSON/Markdown artifacts verified at $outputDir"
