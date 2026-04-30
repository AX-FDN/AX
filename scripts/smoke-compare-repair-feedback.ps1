param(
    [string] $ManifestPath = "benchmarks\\repair-cases-smoke.json",
    [string] $SharedSourceDir = "benchmarks\\repair-candidates\\smoke",
    [string] $BaseSourceDir = "benchmarks\\repair-candidates\\compare\\base",
    [string] $AiSourceDir = "",
    [string] $BenchmarkDir = ".ax-ai\\repair-benchmark\\ci-compare",
    [string] $OutputDir = ".ax-ai\\repair-comparisons\\ci-smoke",
    [switch] $SkipBuild
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$exportScript = Join-Path $PSScriptRoot "export-repair-benchmark.ps1"
$compareScript = Join-Path $PSScriptRoot "compare-repair-feedback.ps1"
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

if (-not (Test-Path $baseSourceDir)) {
    Write-Error "Base replay source directory not found: $baseSourceDir"
}

if ($aiSourceDir -and (-not (Test-Path $aiSourceDir))) {
    Write-Error "AI replay source directory not found: $aiSourceDir"
}

Remove-RepoDirectoryIfExists -Path $BenchmarkDir
Remove-RepoDirectoryIfExists -Path $OutputDir

& $exportScript -ManifestPath $manifestPath -OutputDir $benchmarkDir -SkipBuild:$SkipBuild | Out-Null

$runnerExtraArgs = @(
    "-SourceDir", $sharedSourceDir,
    "-SourceDirBase", $baseSourceDir
)

if ($aiSourceDir) {
    $runnerExtraArgs += @("-SourceDirAi", $aiSourceDir)
}

& $compareScript `
    -BenchmarkDir $benchmarkDir `
    -RunnerScript $replayAdapter `
    -RunnerExtraArgs $runnerExtraArgs `
    -OutputDir $outputDir `
    -SkipBuild:$SkipBuild | Out-Null

$comparisonPath = Join-Path $outputDir "comparison.json"
if (-not (Test-Path $comparisonPath)) {
    Write-Error "Compare smoke did not produce comparison.json at $comparisonPath"
}

$comparison = Get-Content $comparisonPath -Raw -Encoding utf8 | ConvertFrom-Json

Assert-Equal -Label "schema_version" -Actual ([int] $comparison.schema_version) -Expected 1
Assert-Equal -Label "comparison.total_cases" -Actual ([int] $comparison.comparison.total_cases) -Expected 12
Assert-Equal -Label "comparison.base_passed" -Actual ([int] $comparison.comparison.base_passed) -Expected 6
Assert-Equal -Label "comparison.ai_passed" -Actual ([int] $comparison.comparison.ai_passed) -Expected 12
Assert-Equal -Label "comparison.absolute_lift_cases" -Actual ([int] $comparison.comparison.absolute_lift_cases) -Expected 6
Assert-Equal -Label "comparison.absolute_lift_pp" -Actual ([double] $comparison.comparison.absolute_lift_pp) -Expected 50
Assert-Equal -Label "base.invocation_totals.ok" -Actual ([int] $comparison.modes.base.invocation_totals.ok) -Expected 12
Assert-Equal -Label "ai.invocation_totals.ok" -Actual ([int] $comparison.modes.ai.invocation_totals.ok) -Expected 12
Assert-Equal -Label "base.score_totals.failed" -Actual ([int] $comparison.modes.base.score_totals.failed) -Expected 6
Assert-Equal -Label "ai.score_totals.failed" -Actual ([int] $comparison.modes.ai.score_totals.failed) -Expected 0
Assert-Equal -Label "base.timed_out" -Actual ([bool] $comparison.modes.base.timed_out) -Expected $false
Assert-Equal -Label "ai.timed_out" -Actual ([bool] $comparison.modes.ai.timed_out) -Expected $false
Assert-StringArray -Label "comparison.improved_cases" -Actual @($comparison.comparison.improved_cases) -Expected @(
    "type_mismatch_bool_from_int",
    "missing_struct_literal_field",
    "match_struct_pattern_missing_field",
    "slice_assignment_read_only",
    "index_out_of_bounds_runtime",
    "division_by_zero_runtime"
)
Assert-StringArray -Label "comparison.regressed_cases" -Actual @($comparison.comparison.regressed_cases) -Expected @()

$semanticCategory = @($comparison.categories | Where-Object { [string] $_.category -eq "semantic" })
Assert-Equal -Label "semantic category count" -Actual $semanticCategory.Count -Expected 1
Assert-Equal -Label "semantic.total" -Actual ([int] $semanticCategory[0].total) -Expected 7
Assert-Equal -Label "semantic.base_passed" -Actual ([int] $semanticCategory[0].base_passed) -Expected 3
Assert-Equal -Label "semantic.ai_passed" -Actual ([int] $semanticCategory[0].ai_passed) -Expected 7
Assert-Equal -Label "semantic.improved" -Actual ([int] $semanticCategory[0].improved) -Expected 4
Assert-Equal -Label "semantic.regressed" -Actual ([int] $semanticCategory[0].regressed) -Expected 0

$runtimeCategory = @($comparison.categories | Where-Object { [string] $_.category -eq "runtime" })
Assert-Equal -Label "runtime category count" -Actual $runtimeCategory.Count -Expected 1
Assert-Equal -Label "runtime.total" -Actual ([int] $runtimeCategory[0].total) -Expected 2
Assert-Equal -Label "runtime.base_passed" -Actual ([int] $runtimeCategory[0].base_passed) -Expected 0
Assert-Equal -Label "runtime.ai_passed" -Actual ([int] $runtimeCategory[0].ai_passed) -Expected 2
Assert-Equal -Label "runtime.improved" -Actual ([int] $runtimeCategory[0].improved) -Expected 2
Assert-Equal -Label "runtime.regressed" -Actual ([int] $runtimeCategory[0].regressed) -Expected 0

Write-Host "Compare smoke passed. Stable comparison.json contract verified at $comparisonPath"
