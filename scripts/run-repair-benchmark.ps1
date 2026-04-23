param(
    [string] $BenchmarkDir = "",
    [string] $RunnerScript = "",
    [string[]] $RunnerExtraArgs = @(),
    [ValidateSet("cold", "base", "ai")]
    [string] $FeedbackMode = "ai",
    [string] $OutputDir = "",
    [switch] $RefreshBenchmark,
    [switch] $SkipScore,
    [switch] $RunPrograms,
    [int] $TimeoutSeconds = 180
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$exportScript = Join-Path $PSScriptRoot "export-repair-benchmark.ps1"
$scoreScript = Join-Path $PSScriptRoot "score-repair-benchmark.ps1"

if ([string]::IsNullOrWhiteSpace($RunnerScript)) {
    Write-Error "RunnerScript is required. Example: .\\scripts\\run-repair-benchmark.ps1 -RunnerScript .\\scripts\\replay-repair-adapter.ps1 -RunnerExtraArgs @('-SourceDir', '.ax-ai\\repair-candidates\\smoke')"
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
    $OutputDir = Join-Path $repoRoot ".ax-ai\\repair-runs\\$timestamp"
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

$powerShellExe = Get-PowerShellExecutable
$benchmarkIndexPath = Ensure-BenchmarkIndex -BenchmarkDir $BenchmarkDir -Refresh:$RefreshBenchmark
$benchmarkIndex = Get-Content $benchmarkIndexPath -Raw -Encoding utf8 | ConvertFrom-Json
$benchmarkRoot = Split-Path -Parent $benchmarkIndexPath

$candidatesDir = Join-Path $OutputDir "candidates"
$invocationsDir = Join-Path $OutputDir "invocations"
New-Item -ItemType Directory -Force -Path $candidatesDir | Out-Null
New-Item -ItemType Directory -Force -Path $invocationsDir | Out-Null

$caseResults = @()

foreach ($case in @($benchmarkIndex.cases)) {
    $caseId = [string] $case.id
    $caseInvocationDir = Join-Path $invocationsDir $caseId
    New-Item -ItemType Directory -Force -Path $caseInvocationDir | Out-Null

    $promptArtifact = switch ($FeedbackMode) {
        "cold" { [string] $case.artifacts.cold_prompt }
        "base" { [string] $case.artifacts.base_prompt }
        "ai" { [string] $case.artifacts.ai_prompt }
    }

    $bundleArtifact = switch ($FeedbackMode) {
        "cold" { [string] $case.artifacts.cold_bundle }
        "base" { [string] $case.artifacts.base_bundle }
        "ai" { [string] $case.artifacts.ai_bundle }
    }

    $promptPath = Join-Path $benchmarkRoot $promptArtifact
    $bundlePath = Join-Path $benchmarkRoot $bundleArtifact
    $outputPath = Join-Path $candidatesDir "$caseId.ax"

    $arguments = @(
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        $RunnerScript,
        "-PromptPath",
        $promptPath,
        "-BundlePath",
        $bundlePath,
        "-OutputPath",
        $outputPath,
        "-CaseId",
        $caseId,
        "-FeedbackMode",
        $FeedbackMode
    ) + $RunnerExtraArgs

    $invocation = Invoke-ExternalProcess -FileName $powerShellExe -Arguments $arguments -TimeoutSeconds $TimeoutSeconds

    if ((-not (Test-Path $outputPath)) -and (-not [string]::IsNullOrWhiteSpace($invocation.StdOut))) {
        Write-Utf8File -Path $outputPath -Text $invocation.StdOut
    }

    Write-Utf8File -Path (Join-Path $caseInvocationDir "stdout.txt") -Text $invocation.StdOut
    Write-Utf8File -Path (Join-Path $caseInvocationDir "stderr.txt") -Text $invocation.StdErr

    $status = "failed"
    if ($invocation.TimedOut) {
        $status = "timed_out"
    } elseif (($invocation.ExitCode -eq 0) -and (Test-Path $outputPath)) {
        $status = "ok"
    } elseif (($invocation.ExitCode -eq 0) -and (-not (Test-Path $outputPath)) -and (-not [string]::IsNullOrWhiteSpace($invocation.StdOut))) {
        $status = "ok"
    }

    $caseResult = [pscustomobject][ordered]@{
        id              = $caseId
        feedback_mode   = $FeedbackMode
        prompt_path     = $promptPath
        bundle_path     = $bundlePath
        output_path     = $outputPath
        status          = $status
        timed_out       = $invocation.TimedOut
        exit_code       = $invocation.ExitCode
        stdout_log      = (Join-Path $caseInvocationDir "stdout.txt")
        stderr_log      = (Join-Path $caseInvocationDir "stderr.txt")
    }

    Write-Utf8File -Path (Join-Path $caseInvocationDir "invocation.json") -Text (Format-JsonText -Value $caseResult)
    $caseResults += $caseResult
}

$scoreSummaryPath = $null
$scoreExitCode = $null
if (-not $SkipScore) {
    $scoreOutputDir = Join-Path $OutputDir "score"
    $scoreArguments = @(
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        $scoreScript,
        "-BenchmarkDir",
        $benchmarkRoot,
        "-CandidatesDir",
        $candidatesDir,
        "-OutputDir",
        $scoreOutputDir
    )

    if ($RunPrograms) {
        $scoreArguments += "-RunPrograms"
    }

    $scoreInvocation = Invoke-ExternalProcess -FileName $powerShellExe -Arguments $scoreArguments -TimeoutSeconds $TimeoutSeconds
    $scoreExitCode = $scoreInvocation.ExitCode
    $scoreSummaryPath = Join-Path $scoreOutputDir "summary.json"

    Write-Utf8File -Path (Join-Path $OutputDir "score.stdout.txt") -Text $scoreInvocation.StdOut
    Write-Utf8File -Path (Join-Path $OutputDir "score.stderr.txt") -Text $scoreInvocation.StdErr
}

$okCount = @($caseResults | Where-Object { $_.status -eq "ok" }).Count
$failedCount = @($caseResults | Where-Object { $_.status -eq "failed" }).Count
$timedOutCount = @($caseResults | Where-Object { $_.status -eq "timed_out" }).Count

$summary = [ordered]@{
    schema_version   = 1
    generated_at     = (Get-Date).ToString("o")
    feedback_mode    = $FeedbackMode
    benchmark_index  = $benchmarkIndexPath
    benchmark_root   = $benchmarkRoot
    runner_script    = $RunnerScript
    runner_extra_args = $RunnerExtraArgs
    candidates_dir   = $candidatesDir
    output_dir       = $OutputDir
    totals           = [ordered]@{
        total      = [int] $caseResults.Count
        ok         = [int] $okCount
        failed     = [int] $failedCount
        timed_out  = [int] $timedOutCount
    }
    score           = [ordered]@{
        skipped      = [bool] $SkipScore
        summary_path = $scoreSummaryPath
        exit_code    = $scoreExitCode
    }
    cases           = $caseResults
}

$summaryPath = Join-Path $OutputDir "run-summary.json"
Write-Utf8File -Path $summaryPath -Text (Format-JsonText -Value $summary)

Write-Host ""
Write-Host "Repair runner results:"
$caseResults |
    Select-Object `
        @{ Name = "Id"; Expression = { $_.id } }, `
        @{ Name = "Mode"; Expression = { $_.feedback_mode } }, `
        @{ Name = "Status"; Expression = { $_.status } } |
    Format-Table -AutoSize

Write-Host ""
Write-Host "Run summary written to $summaryPath"

if ($failedCount -gt 0 -or $timedOutCount -gt 0) {
    exit 1
}

if (($null -ne $scoreExitCode) -and ($scoreExitCode -ne 0)) {
    exit $scoreExitCode
}
