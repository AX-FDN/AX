param(
    [string] $ManifestPath = "benchmarks\\repair-cases-smoke.json",
    [string] $SourceDir = "benchmarks\\repair-candidates\\smoke",
    [string] $BenchmarkDir = ".ax-ai\\repair-benchmark\\ci-smoke",
    [string] $OutputDir = ".ax-ai\\repair-runs\\ci-smoke",
    [switch] $SkipBuild
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

& $exportScript -ManifestPath $manifestPath -OutputDir $benchmarkDir -SkipBuild:$SkipBuild | Out-Null
& $runScript `
    -BenchmarkDir $benchmarkDir `
    -RunnerScript $replayAdapter `
    -RunnerExtraArgs @("-SourceDir", $sourceDir) `
    -FeedbackMode ai `
    -OutputDir $outputDir `
    -SkipBuild:$SkipBuild

$runSummaryPath = Join-Path $outputDir "run-summary.json"
if (-not (Test-Path $runSummaryPath)) {
    Write-Error "Repair smoke did not produce run-summary.json at $runSummaryPath"
}

$scoreSummaryPath = Join-Path $outputDir "score\\summary.json"
if (-not (Test-Path $scoreSummaryPath)) {
    Write-Error "Repair smoke did not produce score summary at $scoreSummaryPath"
}

$runSummary = Get-Content $runSummaryPath -Raw -Encoding utf8 | ConvertFrom-Json
$scoreSummary = Get-Content $scoreSummaryPath -Raw -Encoding utf8 | ConvertFrom-Json

Assert-Equal -Label "runSummary.schema_version" -Actual ([int] $runSummary.schema_version) -Expected 1
Assert-Equal -Label "runSummary.feedback_mode" -Actual ([string] $runSummary.feedback_mode) -Expected "ai"
Assert-Equal -Label "runSummary.totals.total" -Actual ([int] $runSummary.totals.total) -Expected 11
Assert-Equal -Label "runSummary.totals.ok" -Actual ([int] $runSummary.totals.ok) -Expected 11
Assert-Equal -Label "runSummary.totals.failed" -Actual ([int] $runSummary.totals.failed) -Expected 0
Assert-Equal -Label "runSummary.totals.timed_out" -Actual ([int] $runSummary.totals.timed_out) -Expected 0
Assert-Equal -Label "runSummary.cases count" -Actual (@($runSummary.cases).Count) -Expected 11
Assert-Equal -Label "runSummary.score.skipped" -Actual ([bool] $runSummary.score.skipped) -Expected $false
if ($null -eq $runSummary.score.exit_code) {
    Write-Error "Repair smoke should record a non-null score exit code."
}
Assert-Equal -Label "runSummary.score.exit_code" -Actual ([int] $runSummary.score.exit_code) -Expected 0

if (-not (Test-Path ([string] $runSummary.score.summary_path))) {
    Write-Error "Repair smoke reported missing score summary path: $($runSummary.score.summary_path)"
}

Assert-Equal -Label "scoreSummary.schema_version" -Actual ([int] $scoreSummary.schema_version) -Expected 1
Assert-Equal -Label "scoreSummary.totals.total" -Actual ([int] $scoreSummary.totals.total) -Expected 11
Assert-Equal -Label "scoreSummary.totals.passed" -Actual ([int] $scoreSummary.totals.passed) -Expected 11
Assert-Equal -Label "scoreSummary.totals.failed" -Actual ([int] $scoreSummary.totals.failed) -Expected 0
Assert-Equal -Label "scoreSummary.totals.missing" -Actual ([int] $scoreSummary.totals.missing) -Expected 0
Assert-Equal -Label "scoreSummary.cases count" -Actual (@($scoreSummary.cases).Count) -Expected 11

$runtimeCases = @($scoreSummary.cases | Where-Object { [string] $_.diagnostic_command -eq "run" })
$checkCases = @($scoreSummary.cases | Where-Object { [string] $_.diagnostic_command -eq "check" })

Assert-Equal -Label "runtime case count" -Actual $runtimeCases.Count -Expected 2
Assert-Equal -Label "check case count" -Actual $checkCases.Count -Expected 9
Assert-StringArray -Label "runtime case ids" -Actual @($runtimeCases | ForEach-Object { [string] $_.id }) -Expected @(
    "index_out_of_bounds_runtime",
    "division_by_zero_runtime"
)

foreach ($runtimeCase in $runtimeCases) {
    $caseId = [string] $runtimeCase.id
    if ($null -eq $runtimeCase.run) {
        Write-Error "Runtime smoke case '$caseId' should include run validation details."
    }

    Assert-Equal -Label "runtime[$caseId].status" -Actual ([string] $runtimeCase.status) -Expected "passed"
    Assert-Equal -Label "runtime[$caseId].run.command" -Actual ([string] $runtimeCase.run.command) -Expected "run --json"
    Assert-Equal -Label "runtime[$caseId].run.command_exit_code" -Actual ([int] $runtimeCase.run.command_exit_code) -Expected 0
    Assert-Equal -Label "runtime[$caseId].run.parsed_diagnostics" -Actual ([bool] $runtimeCase.run.parsed_diagnostics) -Expected $false
    Assert-Equal -Label "runtime[$caseId].run.remaining_codes count" -Actual (@($runtimeCase.run.remaining_codes).Count) -Expected 0
}

Write-Host "Repair smoke passed. Stable run-summary.json and score/summary.json contracts verified at $outputDir"
