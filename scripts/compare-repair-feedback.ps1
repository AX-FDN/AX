param(
    [string] $BenchmarkDir = "",
    [string] $RunnerScript = "",
    [string[]] $RunnerExtraArgs = @(),
    [string] $OutputDir = "",
    [switch] $RefreshBenchmark,
    [switch] $SkipBuild,
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

if ([string]::IsNullOrWhiteSpace($RunnerScript)) {
    Write-Error "RunnerScript is required. Example: .\\scripts\\compare-repair-feedback.ps1 -RunnerScript .\\scripts\\codex-repair-adapter.ps1 -RunnerExtraArgs @('-Model', 'gpt-5.4')"
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
    $OutputDir = Join-Path $repoRoot ".ax-ai\\repair-comparisons\\$timestamp"
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
        [switch] $Refresh,
        [switch] $SkipBuild
    )

    if ($Refresh) {
        $exportDir = Join-Path $OutputDir "benchmark"
        & $exportScript -OutputDir $exportDir -SkipBuild:$SkipBuild | Out-Null
        return (Join-Path $exportDir "index.json")
    }

    $existing = Resolve-BenchmarkIndexPath -InputPath $BenchmarkDir
    if ($existing) {
        return $existing
    }

    $exportDir = Join-Path $OutputDir "benchmark"
    & $exportScript -OutputDir $exportDir -SkipBuild:$SkipBuild | Out-Null
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

    if ($SkipBuild) {
        $parts += "-SkipBuild"
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

function Build-MarkdownReport {
    param(
        [object] $ComparisonSummary
    )

    $lines = New-Object System.Collections.Generic.List[string]
    $null = $lines.Add("# AX Repair Feedback Comparison")
    $null = $lines.Add("")
    $null = $lines.Add("- Generated at: $($ComparisonSummary.generated_at)")
    $null = $lines.Add("- Benchmark index: $($ComparisonSummary.benchmark_index)")
    $null = $lines.Add("- Runner script: $($ComparisonSummary.runner_script)")

    if (@($ComparisonSummary.runner_extra_args).Count -gt 0) {
        $null = $lines.Add("- Runner extra args: $(@($ComparisonSummary.runner_extra_args) -join ' ')")
    }

    $null = $lines.Add("")
    $null = $lines.Add("## Overall")
    $null = $lines.Add("")
    $null = $lines.Add("| Mode | Candidates ok | Timed out | Passed | Failed | Missing | Pass rate |")
    $null = $lines.Add("| --- | ---: | ---: | ---: | ---: | ---: | ---: |")
    $null = $lines.Add("| base | $($ComparisonSummary.modes.base.invocation_totals.ok) | $($ComparisonSummary.modes.base.invocation_totals.timed_out) | $($ComparisonSummary.modes.base.score_totals.passed) | $($ComparisonSummary.modes.base.score_totals.failed) | $($ComparisonSummary.modes.base.score_totals.missing) | $($ComparisonSummary.comparison.base_pass_rate)% |")
    $null = $lines.Add("| ai | $($ComparisonSummary.modes.ai.invocation_totals.ok) | $($ComparisonSummary.modes.ai.invocation_totals.timed_out) | $($ComparisonSummary.modes.ai.score_totals.passed) | $($ComparisonSummary.modes.ai.score_totals.failed) | $($ComparisonSummary.modes.ai.score_totals.missing) | $($ComparisonSummary.comparison.ai_pass_rate)% |")
    $null = $lines.Add("")
    $null = $lines.Add("## Lift")
    $null = $lines.Add("")
    $null = $lines.Add("- Absolute lift: $($ComparisonSummary.comparison.absolute_lift_cases) case(s)")
    $null = $lines.Add("- Absolute lift: $($ComparisonSummary.comparison.absolute_lift_pp) percentage points")

    if ($null -ne $ComparisonSummary.comparison.relative_lift_pct) {
        $null = $lines.Add("- Relative lift over base: $($ComparisonSummary.comparison.relative_lift_pct)%")
    }

    $improvedCases = @($ComparisonSummary.comparison.improved_cases)
    $regressedCases = @($ComparisonSummary.comparison.regressed_cases)
    $null = $lines.Add("- Improved cases: $(if ($improvedCases.Count -gt 0) { $improvedCases -join ', ' } else { '(none)' })")
    $null = $lines.Add("- Regressed cases: $(if ($regressedCases.Count -gt 0) { $regressedCases -join ', ' } else { '(none)' })")
    $null = $lines.Add("")
    $null = $lines.Add("## Categories")
    $null = $lines.Add("")
    $null = $lines.Add("| Category | Total | Base passed | AI passed | Lift (pp) | Improved | Regressed |")
    $null = $lines.Add("| --- | ---: | ---: | ---: | ---: | ---: | ---: |")

    foreach ($category in @($ComparisonSummary.categories)) {
        $null = $lines.Add("| $($category.category) | $($category.total) | $($category.base_passed) | $($category.ai_passed) | $($category.absolute_lift_pp) | $($category.improved) | $($category.regressed) |")
    }

    return ($lines -join "`n") + "`n"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$benchmarkIndexPath = Ensure-BenchmarkIndex -BenchmarkDir $BenchmarkDir -Refresh:$RefreshBenchmark -SkipBuild:$SkipBuild
$benchmarkIndex = Read-JsonFile -Path $benchmarkIndexPath -Label "benchmark index"
$caseCount = @($benchmarkIndex.cases).Count
$outerTimeoutSeconds = [math]::Max(300, ($caseCount * $TimeoutSeconds) + 120)
$powerShellExe = Get-PowerShellExecutable

$modeResults = @{}
foreach ($mode in @("base", "ai")) {
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

$baseMode = $modeResults["base"]
$aiMode = $modeResults["ai"]
$baseCases = Get-CaseStatusMap -ScoreSummary $baseMode.score_summary
$aiCases = Get-CaseStatusMap -ScoreSummary $aiMode.score_summary

$caseComparisons = New-Object System.Collections.Generic.List[object]

foreach ($case in @($benchmarkIndex.cases)) {
    $id = [string] $case.id
    $category = [string] $case.category
    $baseCase = $baseCases[$id]
    $aiCase = $aiCases[$id]

    $baseSuccess = if ($baseCase) { [bool] $baseCase.success } else { $false }
    $aiSuccess = if ($aiCase) { [bool] $aiCase.success } else { $false }
    $delta = if ((-not $baseSuccess) -and $aiSuccess) {
        "improved"
    } elseif ($baseSuccess -and (-not $aiSuccess)) {
        "regressed"
    } elseif ($baseSuccess -and $aiSuccess) {
        "both_pass"
    } else {
        "both_fail"
    }

    $baseRemainingCodes = if ($baseCase) {
        @($baseCase.remaining_codes)
    } else {
        @()
    }
    $aiRemainingCodes = if ($aiCase) {
        @($aiCase.remaining_codes)
    } else {
        @()
    }

    $caseComparisons.Add([pscustomobject][ordered]@{
        id                   = $id
        category             = $category
        repair_goal          = [string] $case.repair_goal
        base_status          = if ($baseCase) { [string] $baseCase.status } else { "missing" }
        ai_status            = if ($aiCase) { [string] $aiCase.status } else { "missing" }
        base_success         = $baseSuccess
        ai_success           = $aiSuccess
        delta                = $delta
        base_remaining_codes = New-JsonArray -Values $baseRemainingCodes
        ai_remaining_codes   = New-JsonArray -Values $aiRemainingCodes
    })
}

$basePassed = [int] $baseMode.score_summary.totals.passed
$aiPassed = [int] $aiMode.score_summary.totals.passed
$totalCases = [int] $benchmarkIndex.cases.Count
$improvedCases = @($caseComparisons | Where-Object { $_.delta -eq "improved" } | ForEach-Object { $_.id })
$regressedCases = @($caseComparisons | Where-Object { $_.delta -eq "regressed" } | ForEach-Object { $_.id })

$categorySummaries = @(
    $caseComparisons |
        Group-Object category |
        Sort-Object Name |
        ForEach-Object {
            $groupCases = @($_.Group)
            $categoryTotal = [int] $groupCases.Count
            $categoryBasePassed = [int] @($groupCases | Where-Object { $_.base_success }).Count
            $categoryAiPassed = [int] @($groupCases | Where-Object { $_.ai_success }).Count
            [pscustomobject][ordered]@{
                category          = $_.Name
                total             = $categoryTotal
                base_passed       = $categoryBasePassed
                ai_passed         = $categoryAiPassed
                base_pass_rate    = Get-Percent -Numerator $categoryBasePassed -Denominator $categoryTotal
                ai_pass_rate      = Get-Percent -Numerator $categoryAiPassed -Denominator $categoryTotal
                absolute_lift_pp  = [math]::Round(
                    (Get-Percent -Numerator $categoryAiPassed -Denominator $categoryTotal) -
                    (Get-Percent -Numerator $categoryBasePassed -Denominator $categoryTotal),
                    2
                )
                improved          = [int] @($groupCases | Where-Object { $_.delta -eq "improved" }).Count
                regressed         = [int] @($groupCases | Where-Object { $_.delta -eq "regressed" }).Count
                improved_case_ids = New-JsonArray -Values (@($groupCases | Where-Object { $_.delta -eq "improved" } | ForEach-Object { $_.id }))
                regressed_case_ids = New-JsonArray -Values (@($groupCases | Where-Object { $_.delta -eq "regressed" } | ForEach-Object { $_.id }))
            }
        }
)

$comparisonSummary = [ordered]@{
    schema_version   = 1
    generated_at     = (Get-Date).ToString("o")
    benchmark_index  = $benchmarkIndexPath
    runner_script    = $RunnerScript
    runner_extra_args = New-JsonArray -Values $RunnerExtraArgs
    output_dir       = $OutputDir
    modes            = [ordered]@{
        base = [ordered]@{
            exit_code         = $baseMode.exit_code
            timed_out         = $baseMode.timed_out
            stdout_log        = $baseMode.stdout_log
            stderr_log        = $baseMode.stderr_log
            run_summary_path  = $baseMode.run_summary_path
            score_summary_path = $baseMode.score_summary_path
            invocation_totals = $baseMode.run_summary.totals
            score_totals      = $baseMode.score_summary.totals
        }
        ai = [ordered]@{
            exit_code         = $aiMode.exit_code
            timed_out         = $aiMode.timed_out
            stdout_log        = $aiMode.stdout_log
            stderr_log        = $aiMode.stderr_log
            run_summary_path  = $aiMode.run_summary_path
            score_summary_path = $aiMode.score_summary_path
            invocation_totals = $aiMode.run_summary.totals
            score_totals      = $aiMode.score_summary.totals
        }
    }
    comparison       = [ordered]@{
        total_cases         = $totalCases
        base_passed         = $basePassed
        ai_passed           = $aiPassed
        base_pass_rate      = Get-Percent -Numerator $basePassed -Denominator $totalCases
        ai_pass_rate        = Get-Percent -Numerator $aiPassed -Denominator $totalCases
        absolute_lift_cases = [int] ($aiPassed - $basePassed)
        absolute_lift_pp    = [math]::Round(
            (Get-Percent -Numerator $aiPassed -Denominator $totalCases) -
            (Get-Percent -Numerator $basePassed -Denominator $totalCases),
            2
        )
        relative_lift_pct   = if ($basePassed -gt 0) {
            [math]::Round((($aiPassed - $basePassed) / $basePassed) * 100, 2)
        } else {
            $null
        }
        improved_cases      = New-JsonArray -Values $improvedCases
        regressed_cases     = New-JsonArray -Values $regressedCases
        unchanged_cases     = New-JsonArray -Values (@($caseComparisons | Where-Object { $_.delta -eq "both_pass" -or $_.delta -eq "both_fail" } | ForEach-Object { $_.id }))
    }
    categories       = $categorySummaries
    cases            = $caseComparisons
}

$comparisonJsonPath = Join-Path $OutputDir "comparison.json"
$comparisonMarkdownPath = Join-Path $OutputDir "comparison.md"
Write-Utf8File -Path $comparisonJsonPath -Text (Format-JsonText -Value $comparisonSummary)
Write-Utf8File -Path $comparisonMarkdownPath -Text (Build-MarkdownReport -ComparisonSummary $comparisonSummary)

Write-Host ""
Write-Host "Repair feedback comparison:"
@(
    [pscustomobject]@{
        Mode          = "base"
        Passed        = $comparisonSummary.modes.base.score_totals.passed
        Failed        = $comparisonSummary.modes.base.score_totals.failed
        Missing       = $comparisonSummary.modes.base.score_totals.missing
        PassRatePct   = $comparisonSummary.comparison.base_pass_rate
    }
    [pscustomobject]@{
        Mode          = "ai"
        Passed        = $comparisonSummary.modes.ai.score_totals.passed
        Failed        = $comparisonSummary.modes.ai.score_totals.failed
        Missing       = $comparisonSummary.modes.ai.score_totals.missing
        PassRatePct   = $comparisonSummary.comparison.ai_pass_rate
    }
) | Format-Table -AutoSize

Write-Host ""
Write-Host "Absolute lift: $($comparisonSummary.comparison.absolute_lift_cases) case(s), $($comparisonSummary.comparison.absolute_lift_pp) percentage points"
Write-Host "Comparison JSON written to $comparisonJsonPath"
Write-Host "Comparison Markdown written to $comparisonMarkdownPath"
