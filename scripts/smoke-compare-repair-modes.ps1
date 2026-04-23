param(
    [string] $ManifestPath = "benchmarks\\repair-cases-smoke.json",
    [string] $SharedSourceDir = "benchmarks\\repair-candidates\\smoke",
    [string] $ColdSourceDir = "benchmarks\\repair-candidates\\compare\\cold",
    [string] $BaseSourceDir = "benchmarks\\repair-candidates\\compare\\base",
    [string] $AiSourceDir = "",
    [string] $BenchmarkDir = ".ax-ai\\repair-benchmark\\ci-compare-modes",
    [string] $OutputDir = ".ax-ai\\repair-mode-comparisons\\ci-smoke"
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$exportScript = Join-Path $PSScriptRoot "export-repair-benchmark.ps1"
$compareScript = Join-Path $PSScriptRoot "compare-repair-modes.ps1"
$replayAdapter = Join-Path $PSScriptRoot "replay-repair-adapter.ps1"

function Resolve-RepoPath {
    param([string] $Path)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $null
    }

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return Join-Path $repoRoot $Path
}

function Remove-RepoDirectoryIfExists {
    param([string] $Path)

    $resolved = Resolve-RepoPath -Path $Path
    if (-not $resolved -or -not (Test-Path $resolved)) {
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

function Assert-StringArray {
    param(
        [string] $Label,
        [object[]] $Actual,
        [string[]] $Expected
    )

    $actualStrings = @($Actual | ForEach-Object { [string] $_ })
    $expectedStrings = @($Expected)

    if ($actualStrings.Count -ne $expectedStrings.Count) {
        Write-Error "$Label expected $($expectedStrings.Count) item(s) but got $($actualStrings.Count): $($actualStrings -join ', ')"
    }

    for ($index = 0; $index -lt $expectedStrings.Count; $index += 1) {
        if ($actualStrings[$index] -ne $expectedStrings[$index]) {
            Write-Error "$Label expected '$($expectedStrings[$index])' at index $index but got '$($actualStrings[$index])'."
        }
    }
}

$manifestPath = Resolve-RepoPath -Path $ManifestPath
$sharedSourceDir = Resolve-RepoPath -Path $SharedSourceDir
$coldSourceDir = Resolve-RepoPath -Path $ColdSourceDir
$baseSourceDir = Resolve-RepoPath -Path $BaseSourceDir
$aiSourceDir = Resolve-RepoPath -Path $AiSourceDir
$benchmarkDir = Resolve-RepoPath -Path $BenchmarkDir
$outputDir = Resolve-RepoPath -Path $OutputDir

if (-not (Test-Path $manifestPath)) {
    Write-Error "Smoke compare manifest not found: $manifestPath"
}

if (-not (Test-Path $sharedSourceDir)) {
    Write-Error "Shared replay source directory not found: $sharedSourceDir"
}

if (-not (Test-Path $coldSourceDir)) {
    Write-Error "Cold replay source directory not found: $coldSourceDir"
}

if (-not (Test-Path $baseSourceDir)) {
    Write-Error "Base replay source directory not found: $baseSourceDir"
}

if ($aiSourceDir -and (-not (Test-Path $aiSourceDir))) {
    Write-Error "AI replay source directory not found: $aiSourceDir"
}

Remove-RepoDirectoryIfExists -Path $BenchmarkDir
Remove-RepoDirectoryIfExists -Path $OutputDir

& $exportScript -ManifestPath $manifestPath -OutputDir $benchmarkDir | Out-Null

$runnerExtraArgs = @(
    "-SourceDir", $sharedSourceDir,
    "-SourceDirCold", $coldSourceDir,
    "-SourceDirBase", $baseSourceDir
)

if ($aiSourceDir) {
    $runnerExtraArgs += @("-SourceDirAi", $aiSourceDir)
}

& $compareScript `
    -BenchmarkDir $benchmarkDir `
    -RunnerScript $replayAdapter `
    -RunnerExtraArgs $runnerExtraArgs `
    -OutputDir $outputDir | Out-Null

$comparisonPath = Join-Path $outputDir "comparison.json"
if (-not (Test-Path $comparisonPath)) {
    Write-Error "Mode compare smoke did not produce comparison.json at $comparisonPath"
}

$comparison = Get-Content $comparisonPath -Raw -Encoding utf8 | ConvertFrom-Json

Assert-Equal -Label "schema_version" -Actual ([int] $comparison.schema_version) -Expected 1
Assert-Equal -Label "summary.total_cases" -Actual ([int] $comparison.summary.total_cases) -Expected 5
Assert-Equal -Label "summary.cold_passed" -Actual ([int] $comparison.summary.cold_passed) -Expected 2
Assert-Equal -Label "summary.base_passed" -Actual ([int] $comparison.summary.base_passed) -Expected 3
Assert-Equal -Label "summary.ai_passed" -Actual ([int] $comparison.summary.ai_passed) -Expected 5
Assert-Equal -Label "cold score_totals.failed" -Actual ([int] $comparison.modes.cold.score_totals.failed) -Expected 3
Assert-Equal -Label "base score_totals.failed" -Actual ([int] $comparison.modes.base.score_totals.failed) -Expected 2
Assert-Equal -Label "ai score_totals.failed" -Actual ([int] $comparison.modes.ai.score_totals.failed) -Expected 0
Assert-Equal -Label "cold_to_base.absolute_lift_cases" -Actual ([int] $comparison.summary.pairwise_comparisons.cold_to_base.absolute_lift_cases) -Expected 1
Assert-Equal -Label "base_to_ai.absolute_lift_cases" -Actual ([int] $comparison.summary.pairwise_comparisons.base_to_ai.absolute_lift_cases) -Expected 2
Assert-Equal -Label "cold_to_ai.absolute_lift_cases" -Actual ([int] $comparison.summary.pairwise_comparisons.cold_to_ai.absolute_lift_cases) -Expected 3
Assert-Equal -Label "cold_to_base.absolute_lift_pp" -Actual ([double] $comparison.summary.pairwise_comparisons.cold_to_base.absolute_lift_pp) -Expected 20
Assert-Equal -Label "base_to_ai.absolute_lift_pp" -Actual ([double] $comparison.summary.pairwise_comparisons.base_to_ai.absolute_lift_pp) -Expected 40
Assert-Equal -Label "cold_to_ai.absolute_lift_pp" -Actual ([double] $comparison.summary.pairwise_comparisons.cold_to_ai.absolute_lift_pp) -Expected 60
Assert-StringArray -Label "cold_to_base.improved_cases" -Actual @($comparison.summary.pairwise_comparisons.cold_to_base.improved_cases) -Expected @(
    "unknown_type_missing"
)
Assert-StringArray -Label "base_to_ai.improved_cases" -Actual @($comparison.summary.pairwise_comparisons.base_to_ai.improved_cases) -Expected @(
    "type_mismatch_bool_from_int",
    "missing_struct_literal_field"
)
Assert-StringArray -Label "cold_to_ai.improved_cases" -Actual @($comparison.summary.pairwise_comparisons.cold_to_ai.improved_cases) -Expected @(
    "type_mismatch_bool_from_int",
    "unknown_type_missing",
    "missing_struct_literal_field"
)
Assert-StringArray -Label "cold_to_ai.regressed_cases" -Actual @($comparison.summary.pairwise_comparisons.cold_to_ai.regressed_cases) -Expected @()

$semanticCategory = @($comparison.categories | Where-Object { [string] $_.category -eq "semantic" })
Assert-Equal -Label "semantic category count" -Actual $semanticCategory.Count -Expected 1
Assert-Equal -Label "semantic.total" -Actual ([int] $semanticCategory[0].total) -Expected 3
Assert-Equal -Label "semantic.cold_passed" -Actual ([int] $semanticCategory[0].cold_passed) -Expected 0
Assert-Equal -Label "semantic.base_passed" -Actual ([int] $semanticCategory[0].base_passed) -Expected 1
Assert-Equal -Label "semantic.ai_passed" -Actual ([int] $semanticCategory[0].ai_passed) -Expected 3

Write-Host "Mode compare smoke passed. Stable three-mode comparison contract verified at $comparisonPath"
