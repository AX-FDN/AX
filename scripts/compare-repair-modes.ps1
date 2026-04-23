param(
    [string] $BenchmarkDir = "",
    [string] $RunnerScript = "",
    [string[]] $RunnerExtraArgs = @(),
    [string] $OutputDir = "",
    [switch] $RefreshBenchmark,
    [switch] $RunPrograms,
    [int] $TimeoutSeconds = 180
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$runScript = Join-Path $PSScriptRoot "run-repair-benchmark.ps1"
$exportScript = Join-Path $PSScriptRoot "export-repair-benchmark.ps1"
$modeOrder = @("cold", "base", "ai")

if ([string]::IsNullOrWhiteSpace($RunnerScript)) {
    Write-Error "RunnerScript is required. Example: .\\scripts\\compare-repair-modes.ps1 -RunnerScript .\\scripts\\codex-repair-adapter.ps1 -RunnerExtraArgs @('-Model', 'gpt-5.4')"
}

if ($TimeoutSeconds -lt 1) {
    Write-Error "TimeoutSeconds must be at least 1."
}

if (-not [System.IO.Path]::IsPathRooted($RunnerScript)) {
    $RunnerScript = Join-Path $repoRoot $RunnerScript
}

if (-not (Test-Path $RunnerScript)) {
    Write-Error "Runner script not found: $RunnerScript"
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $OutputDir = Join-Path $repoRoot ".ax-ai\\repair-mode-comparisons\\$timestamp"
} elseif (-not [System.IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir = Join-Path $repoRoot $OutputDir
}

function Write-Utf8File {
    param(
        [string] $Path,
        [string] $Text
    )

    $parent = Split-Path -Parent $Path
    if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }

    $encoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Text, $encoding)
}

function Format-JsonText {
    param([object] $Value)

    return (($Value | ConvertTo-Json -Depth 100).TrimEnd() + "`n")
}

function Resolve-BenchmarkIndexPath {
    param([string] $InputPath)

    if (-not [string]::IsNullOrWhiteSpace($InputPath)) {
        if (-not [System.IO.Path]::IsPathRooted($InputPath)) {
            $InputPath = Join-Path $repoRoot $InputPath
        }

        if ((Test-Path $InputPath) -and (Get-Item $InputPath).PSIsContainer) {
            $indexPath = Join-Path $InputPath "index.json"
        } else {
            $indexPath = $InputPath
        }

        if (-not (Test-Path $indexPath)) {
            Write-Error "Benchmark index not found: $indexPath"
        }

        return $indexPath
    }

    $benchmarkRoot = Join-Path $repoRoot ".ax-ai\\repair-benchmark"
    if (-not (Test-Path $benchmarkRoot)) {
        return $null
    }

    $latest = Get-ChildItem $benchmarkRoot -Directory |
        Where-Object { Test-Path (Join-Path $_.FullName "index.json") } |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1

    if (-not $latest) {
        return $null
    }

    return (Join-Path $latest.FullName "index.json")
}

function Ensure-BenchmarkIndex {
    param(
        [string] $BenchmarkDir,
        [switch] $Refresh
    )

    if ($Refresh) {
        $exportDir = Join-Path $OutputDir "benchmark"
        & $exportScript -OutputDir $exportDir | Out-Null
        return (Join-Path $exportDir "index.json")
    }

    $existing = Resolve-BenchmarkIndexPath -InputPath $BenchmarkDir
    if ($existing) {
        return $existing
    }

    $exportDir = Join-Path $OutputDir "benchmark"
    & $exportScript -OutputDir $exportDir | Out-Null
    return (Join-Path $exportDir "index.json")
}

function Get-PowerShellExecutable {
    $candidate = Join-Path $PSHOME "powershell.exe"
    if (Test-Path $candidate) {
        return $candidate
    }

    $command = Get-Command powershell.exe -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    Write-Error "Could not locate powershell.exe for child script execution."
}

function Invoke-ExternalProcess {
    param(
        [string] $FileName,
        [string[]] $Arguments,
        [int] $TimeoutSeconds
    )

    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $FileName
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.Arguments = ($Arguments | ForEach-Object {
        if ($_ -match '[\s"]') {
            '"' + $_.Replace('"', '\"') + '"'
        } else {
            $_
        }
    }) -join ' '

    $process = [System.Diagnostics.Process]::Start($startInfo)
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        try {
            $process.Kill()
        } catch {
        }

        return [pscustomobject]@{
            TimedOut = $true
            ExitCode = $null
            StdOut   = ""
            StdErr   = "Process timed out after $TimeoutSeconds seconds."
        }
    }

    return [pscustomobject]@{
        TimedOut = $false
        ExitCode = $process.ExitCode
        StdOut   = $process.StandardOutput.ReadToEnd()
        StdErr   = $process.StandardError.ReadToEnd()
    }
}

function Read-JsonFile {
    param(
        [string] $Path,
        [string] $Label
    )

    if (-not (Test-Path $Path)) {
        Write-Error "$Label not found: $Path"
    }

    try {
        return Get-Content $Path -Raw -Encoding utf8 | ConvertFrom-Json
    } catch {
        Write-Error "Failed to parse ${Label}: $($_.Exception.Message)"
    }
}

function Quote-PowerShellLiteral {
    param([string] $Value)

    return "'" + $Value.Replace("'", "''") + "'"
}

function Build-RunnerArgsLiteral {
    param([string[]] $Values)

    if (-not $Values -or $Values.Count -eq 0) {
        return "@()"
    }

    return "@(" + (($Values | ForEach-Object { Quote-PowerShellLiteral -Value ([string] $_) }) -join ", ") + ")"
}

function New-JsonArray {
    param([object[]] $Values = @())

    $list = [System.Collections.ArrayList]::new()
    foreach ($value in @($Values)) {
        [void] $list.Add($value)
    }

    return ,$list
}

function Build-RunCommandText {
    param(
        [string] $Mode,
        [string] $BenchmarkIndexPath,
        [string] $ModeOutputDir
    )

    $parts = @(
        "& " + (Quote-PowerShellLiteral -Value $runScript),
        "-BenchmarkDir " + (Quote-PowerShellLiteral -Value $BenchmarkIndexPath),
        "-RunnerScript " + (Quote-PowerShellLiteral -Value $RunnerScript),
        "-FeedbackMode " + $Mode,
        "-OutputDir " + (Quote-PowerShellLiteral -Value $ModeOutputDir),
        "-TimeoutSeconds " + $TimeoutSeconds
    )

    if ($RunnerExtraArgs.Count -gt 0) {
        $parts += "-RunnerExtraArgs " + (Build-RunnerArgsLiteral -Values $RunnerExtraArgs)
    }

    if ($RunPrograms) {
        $parts += "-RunPrograms"
    }

    return ($parts -join " ")
}

function Get-CaseStatusMap {
    param([object] $ScoreSummary)

    $map = @{}
    foreach ($case in @($ScoreSummary.cases)) {
        $map[[string] $case.id] = $case
    }
    return $map
}

function Get-Percent {
    param(
        [double] $Numerator,
        [double] $Denominator
    )

    if ($Denominator -le 0) {
        return 0
    }

    return [math]::Round(($Numerator / $Denominator) * 100, 2)
}

function Get-DeltaKind {
    param(
        [bool] $FromSuccess,
        [bool] $ToSuccess
    )

    if ((-not $FromSuccess) -and $ToSuccess) {
        return "improved"
    }

    if ($FromSuccess -and (-not $ToSuccess)) {
        return "regressed"
    }

    if ($FromSuccess -and $ToSuccess) {
        return "both_pass"
    }

    return "both_fail"
}

function Build-PairwiseComparison {
    param(
        [string] $FromMode,
        [string] $ToMode,
        [object[]] $Cases,
        [int] $FromPassed,
        [int] $ToPassed,
        [int] $TotalCases
    )

    $deltaProperty = "${FromMode}_to_${ToMode}_delta"
    $improvedCases = @($Cases | Where-Object { $_.$deltaProperty -eq "improved" } | ForEach-Object { $_.id })
    $regressedCases = @($Cases | Where-Object { $_.$deltaProperty -eq "regressed" } | ForEach-Object { $_.id })
    $unchangedCases = @($Cases | Where-Object { $_.$deltaProperty -eq "both_pass" -or $_.$deltaProperty -eq "both_fail" } | ForEach-Object { $_.id })

    return [ordered]@{
        from_mode           = $FromMode
        to_mode             = $ToMode
        from_passed         = $FromPassed
        to_passed           = $ToPassed
        from_pass_rate      = Get-Percent -Numerator $FromPassed -Denominator $TotalCases
        to_pass_rate        = Get-Percent -Numerator $ToPassed -Denominator $TotalCases
        absolute_lift_cases = [int] ($ToPassed - $FromPassed)
        absolute_lift_pp    = [math]::Round(
            (Get-Percent -Numerator $ToPassed -Denominator $TotalCases) -
            (Get-Percent -Numerator $FromPassed -Denominator $TotalCases),
            2
        )
        relative_lift_pct   = if ($FromPassed -gt 0) {
            [math]::Round((($ToPassed - $FromPassed) / $FromPassed) * 100, 2)
        } else {
            $null
        }
        improved_cases      = New-JsonArray -Values $improvedCases
        regressed_cases     = New-JsonArray -Values $regressedCases
        unchanged_cases     = New-JsonArray -Values $unchangedCases
    }
}

function Build-MarkdownReport {
    param([object] $Summary)

    $lines = New-Object System.Collections.Generic.List[string]
    $null = $lines.Add("# AX Repair Mode Comparison")
    $null = $lines.Add("")
    $null = $lines.Add("- Generated at: $($Summary.generated_at)")
    $null = $lines.Add("- Benchmark index: $($Summary.benchmark_index)")
    $null = $lines.Add("- Runner script: $($Summary.runner_script)")

    if (@($Summary.runner_extra_args).Count -gt 0) {
        $null = $lines.Add("- Runner extra args: $(@($Summary.runner_extra_args) -join ' ')")
    }

    $null = $lines.Add("")
    $null = $lines.Add("## Overall")
    $null = $lines.Add("")
    $null = $lines.Add("| Mode | Candidates ok | Timed out | Passed | Failed | Missing | Pass rate |")
    $null = $lines.Add("| --- | ---: | ---: | ---: | ---: | ---: | ---: |")
    foreach ($mode in $modeOrder) {
        $modeInfo = $Summary.modes.$mode
        $passRate = $Summary.summary."${mode}_pass_rate"
        $null = $lines.Add("| $mode | $($modeInfo.invocation_totals.ok) | $($modeInfo.invocation_totals.timed_out) | $($modeInfo.score_totals.passed) | $($modeInfo.score_totals.failed) | $($modeInfo.score_totals.missing) | ${passRate}% |")
    }

    $null = $lines.Add("")
    $null = $lines.Add("## Pairwise Lift")
    $null = $lines.Add("")
    $null = $lines.Add("| Pair | Lift (cases) | Lift (pp) | Relative lift | Improved | Regressed |")
    $null = $lines.Add("| --- | ---: | ---: | ---: | ---: | ---: |")
    foreach ($pairName in @("cold_to_base", "base_to_ai", "cold_to_ai")) {
        $pair = $Summary.summary.pairwise_comparisons.$pairName
        $relative = if ($null -ne $pair.relative_lift_pct) { "$($pair.relative_lift_pct)%" } else { "n/a" }
        $null = $lines.Add("| $($pair.from_mode) -> $($pair.to_mode) | $($pair.absolute_lift_cases) | $($pair.absolute_lift_pp) | $relative | $(@($pair.improved_cases).Count) | $(@($pair.regressed_cases).Count) |")
    }

    $null = $lines.Add("")
    $null = $lines.Add("## Categories")
    $null = $lines.Add("")
    $null = $lines.Add("| Category | Total | Cold | Base | AI | Cold->Base (pp) | Base->AI (pp) | Cold->AI (pp) |")
    $null = $lines.Add("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")

    foreach ($category in @($Summary.categories)) {
        $null = $lines.Add("| $($category.category) | $($category.total) | $($category.cold_passed) | $($category.base_passed) | $($category.ai_passed) | $($category.pairwise_lifts.cold_to_base.absolute_lift_pp) | $($category.pairwise_lifts.base_to_ai.absolute_lift_pp) | $($category.pairwise_lifts.cold_to_ai.absolute_lift_pp) |")
    }

    return ($lines -join "`n") + "`n"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$benchmarkIndexPath = Ensure-BenchmarkIndex -BenchmarkDir $BenchmarkDir -Refresh:$RefreshBenchmark
$benchmarkIndex = Read-JsonFile -Path $benchmarkIndexPath -Label "benchmark index"
$caseCount = @($benchmarkIndex.cases).Count
$outerTimeoutSeconds = [math]::Max(300, ($caseCount * $TimeoutSeconds) + 120)
$powerShellExe = Get-PowerShellExecutable

$modeResults = @{}
foreach ($mode in $modeOrder) {
    $modeOutputDir = Join-Path $OutputDir $mode
    $commandText = Build-RunCommandText -Mode $mode -BenchmarkIndexPath $benchmarkIndexPath -ModeOutputDir $modeOutputDir
    $invocation = Invoke-ExternalProcess `
        -FileName $powerShellExe `
        -Arguments @("-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", $commandText) `
        -TimeoutSeconds $outerTimeoutSeconds

    Write-Utf8File -Path (Join-Path $OutputDir "$mode.stdout.txt") -Text $invocation.StdOut
    Write-Utf8File -Path (Join-Path $OutputDir "$mode.stderr.txt") -Text $invocation.StdErr

    $runSummaryPath = Join-Path $modeOutputDir "run-summary.json"
    $scoreSummaryPath = Join-Path (Join-Path $modeOutputDir "score") "summary.json"
    $runSummary = Read-JsonFile -Path $runSummaryPath -Label "$mode run summary"
    $scoreSummary = Read-JsonFile -Path $scoreSummaryPath -Label "$mode score summary"

    $modeResults[$mode] = [pscustomobject]@{
        mode               = $mode
        exit_code          = $invocation.ExitCode
        timed_out          = $invocation.TimedOut
        stdout_log         = (Join-Path $OutputDir "$mode.stdout.txt")
        stderr_log         = (Join-Path $OutputDir "$mode.stderr.txt")
        run_summary_path   = $runSummaryPath
        score_summary_path = $scoreSummaryPath
        run_summary        = $runSummary
        score_summary      = $scoreSummary
    }
}

$coldCases = Get-CaseStatusMap -ScoreSummary $modeResults["cold"].score_summary
$baseCases = Get-CaseStatusMap -ScoreSummary $modeResults["base"].score_summary
$aiCases = Get-CaseStatusMap -ScoreSummary $modeResults["ai"].score_summary

$caseComparisons = New-Object System.Collections.Generic.List[object]

foreach ($case in @($benchmarkIndex.cases)) {
    $id = [string] $case.id
    $category = [string] $case.category
    $coldCase = $coldCases[$id]
    $baseCase = $baseCases[$id]
    $aiCase = $aiCases[$id]

    $coldSuccess = if ($coldCase) { [bool] $coldCase.success } else { $false }
    $baseSuccess = if ($baseCase) { [bool] $baseCase.success } else { $false }
    $aiSuccess = if ($aiCase) { [bool] $aiCase.success } else { $false }

    $coldRemainingCodes = if ($coldCase) { @($coldCase.remaining_codes) } else { @() }
    $baseRemainingCodes = if ($baseCase) { @($baseCase.remaining_codes) } else { @() }
    $aiRemainingCodes = if ($aiCase) { @($aiCase.remaining_codes) } else { @() }

    $caseComparisons.Add([pscustomobject][ordered]@{
        id                    = $id
        category              = $category
        repair_goal           = [string] $case.repair_goal
        cold_status           = if ($coldCase) { [string] $coldCase.status } else { "missing" }
        base_status           = if ($baseCase) { [string] $baseCase.status } else { "missing" }
        ai_status             = if ($aiCase) { [string] $aiCase.status } else { "missing" }
        cold_success          = $coldSuccess
        base_success          = $baseSuccess
        ai_success            = $aiSuccess
        cold_remaining_codes  = New-JsonArray -Values $coldRemainingCodes
        base_remaining_codes  = New-JsonArray -Values $baseRemainingCodes
        ai_remaining_codes    = New-JsonArray -Values $aiRemainingCodes
        cold_to_base_delta    = Get-DeltaKind -FromSuccess $coldSuccess -ToSuccess $baseSuccess
        base_to_ai_delta      = Get-DeltaKind -FromSuccess $baseSuccess -ToSuccess $aiSuccess
        cold_to_ai_delta      = Get-DeltaKind -FromSuccess $coldSuccess -ToSuccess $aiSuccess
    })
}

$totalCases = [int] $benchmarkIndex.cases.Count
$coldPassed = [int] $modeResults["cold"].score_summary.totals.passed
$basePassed = [int] $modeResults["base"].score_summary.totals.passed
$aiPassed = [int] $modeResults["ai"].score_summary.totals.passed

$coldToBase = Build-PairwiseComparison -FromMode "cold" -ToMode "base" -Cases $caseComparisons -FromPassed $coldPassed -ToPassed $basePassed -TotalCases $totalCases
$baseToAi = Build-PairwiseComparison -FromMode "base" -ToMode "ai" -Cases $caseComparisons -FromPassed $basePassed -ToPassed $aiPassed -TotalCases $totalCases
$coldToAi = Build-PairwiseComparison -FromMode "cold" -ToMode "ai" -Cases $caseComparisons -FromPassed $coldPassed -ToPassed $aiPassed -TotalCases $totalCases

$categorySummaries = @(
    $caseComparisons |
        Group-Object category |
        Sort-Object Name |
        ForEach-Object {
            $groupCases = @($_.Group)
            $categoryTotal = [int] $groupCases.Count
            $categoryColdPassed = [int] @($groupCases | Where-Object { $_.cold_success }).Count
            $categoryBasePassed = [int] @($groupCases | Where-Object { $_.base_success }).Count
            $categoryAiPassed = [int] @($groupCases | Where-Object { $_.ai_success }).Count

            [pscustomobject][ordered]@{
                category        = $_.Name
                total           = $categoryTotal
                cold_passed     = $categoryColdPassed
                base_passed     = $categoryBasePassed
                ai_passed       = $categoryAiPassed
                cold_pass_rate  = Get-Percent -Numerator $categoryColdPassed -Denominator $categoryTotal
                base_pass_rate  = Get-Percent -Numerator $categoryBasePassed -Denominator $categoryTotal
                ai_pass_rate    = Get-Percent -Numerator $categoryAiPassed -Denominator $categoryTotal
                pairwise_lifts  = [ordered]@{
                    cold_to_base = Build-PairwiseComparison -FromMode "cold" -ToMode "base" -Cases $groupCases -FromPassed $categoryColdPassed -ToPassed $categoryBasePassed -TotalCases $categoryTotal
                    base_to_ai   = Build-PairwiseComparison -FromMode "base" -ToMode "ai" -Cases $groupCases -FromPassed $categoryBasePassed -ToPassed $categoryAiPassed -TotalCases $categoryTotal
                    cold_to_ai   = Build-PairwiseComparison -FromMode "cold" -ToMode "ai" -Cases $groupCases -FromPassed $categoryColdPassed -ToPassed $categoryAiPassed -TotalCases $categoryTotal
                }
            }
        }
)

$comparisonSummary = [ordered]@{
    schema_version    = 1
    generated_at      = (Get-Date).ToString("o")
    benchmark_index   = $benchmarkIndexPath
    runner_script     = $RunnerScript
    runner_extra_args = New-JsonArray -Values $RunnerExtraArgs
    mode_order        = New-JsonArray -Values $modeOrder
    output_dir        = $OutputDir
    modes             = [ordered]@{
        cold = [ordered]@{
            exit_code          = $modeResults["cold"].exit_code
            timed_out          = $modeResults["cold"].timed_out
            stdout_log         = $modeResults["cold"].stdout_log
            stderr_log         = $modeResults["cold"].stderr_log
            run_summary_path   = $modeResults["cold"].run_summary_path
            score_summary_path = $modeResults["cold"].score_summary_path
            invocation_totals  = $modeResults["cold"].run_summary.totals
            score_totals       = $modeResults["cold"].score_summary.totals
        }
        base = [ordered]@{
            exit_code          = $modeResults["base"].exit_code
            timed_out          = $modeResults["base"].timed_out
            stdout_log         = $modeResults["base"].stdout_log
            stderr_log         = $modeResults["base"].stderr_log
            run_summary_path   = $modeResults["base"].run_summary_path
            score_summary_path = $modeResults["base"].score_summary_path
            invocation_totals  = $modeResults["base"].run_summary.totals
            score_totals       = $modeResults["base"].score_summary.totals
        }
        ai = [ordered]@{
            exit_code          = $modeResults["ai"].exit_code
            timed_out          = $modeResults["ai"].timed_out
            stdout_log         = $modeResults["ai"].stdout_log
            stderr_log         = $modeResults["ai"].stderr_log
            run_summary_path   = $modeResults["ai"].run_summary_path
            score_summary_path = $modeResults["ai"].score_summary_path
            invocation_totals  = $modeResults["ai"].run_summary.totals
            score_totals       = $modeResults["ai"].score_summary.totals
        }
    }
    summary           = [ordered]@{
        total_cases           = $totalCases
        cold_passed           = $coldPassed
        base_passed           = $basePassed
        ai_passed             = $aiPassed
        cold_pass_rate        = Get-Percent -Numerator $coldPassed -Denominator $totalCases
        base_pass_rate        = Get-Percent -Numerator $basePassed -Denominator $totalCases
        ai_pass_rate          = Get-Percent -Numerator $aiPassed -Denominator $totalCases
        pairwise_comparisons  = [ordered]@{
            cold_to_base = $coldToBase
            base_to_ai   = $baseToAi
            cold_to_ai   = $coldToAi
        }
    }
    categories        = $categorySummaries
    cases             = $caseComparisons
}

$comparisonJsonPath = Join-Path $OutputDir "comparison.json"
$comparisonMarkdownPath = Join-Path $OutputDir "comparison.md"
Write-Utf8File -Path $comparisonJsonPath -Text (Format-JsonText -Value $comparisonSummary)
Write-Utf8File -Path $comparisonMarkdownPath -Text (Build-MarkdownReport -Summary $comparisonSummary)

Write-Host ""
Write-Host "Repair mode comparison:"
@(
    [pscustomobject]@{
        Mode        = "cold"
        Passed      = $comparisonSummary.modes.cold.score_totals.passed
        Failed      = $comparisonSummary.modes.cold.score_totals.failed
        Missing     = $comparisonSummary.modes.cold.score_totals.missing
        PassRatePct = $comparisonSummary.summary.cold_pass_rate
    }
    [pscustomobject]@{
        Mode        = "base"
        Passed      = $comparisonSummary.modes.base.score_totals.passed
        Failed      = $comparisonSummary.modes.base.score_totals.failed
        Missing     = $comparisonSummary.modes.base.score_totals.missing
        PassRatePct = $comparisonSummary.summary.base_pass_rate
    }
    [pscustomobject]@{
        Mode        = "ai"
        Passed      = $comparisonSummary.modes.ai.score_totals.passed
        Failed      = $comparisonSummary.modes.ai.score_totals.failed
        Missing     = $comparisonSummary.modes.ai.score_totals.missing
        PassRatePct = $comparisonSummary.summary.ai_pass_rate
    }
) | Format-Table -AutoSize

Write-Host ""
Write-Host "cold -> base lift: $($comparisonSummary.summary.pairwise_comparisons.cold_to_base.absolute_lift_cases) case(s), $($comparisonSummary.summary.pairwise_comparisons.cold_to_base.absolute_lift_pp) percentage points"
Write-Host "base -> ai lift: $($comparisonSummary.summary.pairwise_comparisons.base_to_ai.absolute_lift_cases) case(s), $($comparisonSummary.summary.pairwise_comparisons.base_to_ai.absolute_lift_pp) percentage points"
Write-Host "cold -> ai lift: $($comparisonSummary.summary.pairwise_comparisons.cold_to_ai.absolute_lift_cases) case(s), $($comparisonSummary.summary.pairwise_comparisons.cold_to_ai.absolute_lift_pp) percentage points"
Write-Host "Comparison JSON written to $comparisonJsonPath"
Write-Host "Comparison Markdown written to $comparisonMarkdownPath"
