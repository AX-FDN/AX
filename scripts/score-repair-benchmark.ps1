param(
    [string] $BenchmarkDir = "",
    [string] $CandidatesDir = "",
    [string] $OutputDir = "",
    [switch] $RunPrograms,
    [switch] $SkipBuild
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoScript = Join-Path $PSScriptRoot "cargo-gnu.ps1"
$repoCargoConfig = Join-Path $repoRoot ".cargo\\config.toml"

if ([string]::IsNullOrWhiteSpace($CandidatesDir)) {
    Write-Error "CandidatesDir is required. Example: .\\scripts\\score-repair-benchmark.ps1 -CandidatesDir .ax-ai\\repair-candidates\\demo"
}

if (-not [System.IO.Path]::IsPathRooted($CandidatesDir)) {
    $CandidatesDir = Join-Path $repoRoot $CandidatesDir
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $OutputDir = Join-Path $repoRoot ".ax-ai\\repair-results\\$timestamp"
} elseif (-not [System.IO.Path]::IsPathRooted($OutputDir)) {
    $OutputDir = Join-Path $repoRoot $OutputDir
}

function Resolve-TargetDir {
    if ($env:CARGO_TARGET_DIR) {
        return $env:CARGO_TARGET_DIR
    }

    if (Test-Path $repoCargoConfig) {
        $configText = Get-Content $repoCargoConfig -Raw -Encoding utf8
        if ($configText -match 'target-dir\s*=\s*"([^"]+)"') {
            return $matches[1]
        }
    }

    return Join-Path $repoRoot "target"
}

function Resolve-AxcBinary {
    $overrideCandidates = @()
    if (-not [string]::IsNullOrWhiteSpace($env:AXC_BINARY)) {
        $overrideCandidates += [string] $env:AXC_BINARY
    }
    if (-not [string]::IsNullOrWhiteSpace($env:CARGO_BIN_EXE_axc)) {
        $overrideCandidates += [string] $env:CARGO_BIN_EXE_axc
    }

    foreach ($candidate in $overrideCandidates) {
        if (Test-Path $candidate) {
            return $candidate
        }
    }

    $targetDir = Resolve-TargetDir
    return Join-Path $targetDir "debug\\axc.exe"
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
        Write-Error "No exported repair benchmark found under $benchmarkRoot. Run .\\scripts\\export-repair-benchmark.ps1 first."
    }

    $latest = Get-ChildItem $benchmarkRoot -Directory |
        Where-Object { Test-Path (Join-Path $_.FullName "index.json") } |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1

    if (-not $latest) {
        Write-Error "No repair benchmark export with index.json found under $benchmarkRoot."
    }

    return Join-Path $latest.FullName "index.json"
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

function Invoke-Axc {
    param(
        [string] $BinaryPath,
        [string[]] $Arguments
    )

    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $BinaryPath
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
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()

    return [pscustomobject]@{
        ExitCode = $process.ExitCode
        StdOut   = $stdout
        StdErr   = $stderr
    }
}

function Read-Json {
    param(
        [string] $Label,
        [string] $Text
    )

    try {
        return $Text | ConvertFrom-Json
    } catch {
        Write-Error "Failed to parse JSON for ${Label}: $($_.Exception.Message)"
    }
}

function Read-DiagnosticsArray {
    param(
        [string] $Label,
        [string] $Text
    )

    try {
        $value = $Text | ConvertFrom-Json
        if ($null -eq $value) {
            return
        }

        if ($value -is [System.Array]) {
            return $value
        }

        return $value
    } catch {
        Write-Error "Failed to parse diagnostics JSON for ${Label}: $($_.Exception.Message)"
    }
}

function Resolve-DiagnosticCommand {
    param([object] $Case)

    $diagnosticCommand = [string] $Case.diagnostic_command
    if ([string]::IsNullOrWhiteSpace($diagnosticCommand)) {
        return "check"
    }

    $diagnosticCommand = $diagnosticCommand.ToLowerInvariant()
    if ($diagnosticCommand -ne "check" -and $diagnosticCommand -ne "run") {
        Write-Error "Case '$([string] $Case.id)' uses unsupported diagnostic_command '$diagnosticCommand'. Expected 'check' or 'run'."
    }

    return $diagnosticCommand
}

function Try-ReadDiagnosticsArray {
    param([string] $Text)

    if ([string]::IsNullOrWhiteSpace($Text)) {
        return [pscustomobject]@{
            Parsed      = $false
            Diagnostics = @()
        }
    }

    try {
        $value = $Text | ConvertFrom-Json
    } catch {
        return [pscustomobject]@{
            Parsed      = $false
            Diagnostics = @()
        }
    }

    if ($null -eq $value) {
        return [pscustomobject]@{
            Parsed      = $true
            Diagnostics = @()
        }
    }

    if ($value -isnot [System.Array]) {
        return [pscustomobject]@{
            Parsed      = $false
            Diagnostics = @()
        }
    }

    $diagnostics = @($value)
    if ($diagnostics.Count -eq 0) {
        return [pscustomobject]@{
            Parsed      = $true
            Diagnostics = @()
        }
    }

    $looksLikeDiagnostics = $true
    foreach ($diagnostic in $diagnostics) {
        if ($null -eq $diagnostic.PSObject.Properties["code"]) {
            $looksLikeDiagnostics = $false
            break
        }
    }

    if (-not $looksLikeDiagnostics) {
        return [pscustomobject]@{
            Parsed      = $false
            Diagnostics = @()
        }
    }

    return [pscustomobject]@{
        Parsed      = $true
        Diagnostics = $diagnostics
    }
}

function Resolve-CandidatePath {
    param(
        [string] $Root,
        [string] $CaseId
    )

    $candidates = @(
        (Join-Path $Root "$CaseId.ax"),
        (Join-Path (Join-Path $Root $CaseId) "repaired.ax")
    )

    foreach ($candidate in $candidates) {
        if (Test-Path $candidate) {
            return $candidate
        }
    }

    return $null
}

$benchmarkIndexPath = Resolve-BenchmarkIndexPath -InputPath $BenchmarkDir
$benchmarkIndex = Read-Json -Label $benchmarkIndexPath -Text (Get-Content $benchmarkIndexPath -Raw -Encoding utf8)
$benchmarkDirPath = Split-Path -Parent $benchmarkIndexPath

if (-not $SkipBuild) {
    & $cargoScript build | Out-Null
}

$binary = Resolve-AxcBinary
if (-not (Test-Path $binary)) {
    Write-Error "Could not find compiled AX binary. Checked AXC_BINARY, CARGO_BIN_EXE_axc, and fallback path $binary"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$results = @()

foreach ($case in @($benchmarkIndex.cases)) {
    $caseId = [string] $case.id
    $diagnosticCommand = Resolve-DiagnosticCommand -Case $case
    $caseOutputDir = Join-Path $OutputDir $caseId
    New-Item -ItemType Directory -Force -Path $caseOutputDir | Out-Null

    $candidatePath = Resolve-CandidatePath -Root $CandidatesDir -CaseId $caseId
    if (-not $candidatePath) {
        $missing = [pscustomobject][ordered]@{
            id              = $caseId
            status          = "missing"
            success         = $false
            candidate_path  = $null
            remaining_codes = @()
            diagnostics     = @()
        }
        Write-Utf8File -Path (Join-Path $caseOutputDir "result.json") -Text (($missing | ConvertTo-Json -Depth 100) + "`n")
        $results += $missing
        continue
    }

    $candidateText = Get-Content $candidatePath -Raw -Encoding utf8
    Write-Utf8File -Path (Join-Path $caseOutputDir "candidate.ax") -Text $candidateText

    $checkResult = Invoke-Axc -BinaryPath $binary -Arguments @("check", $candidatePath, "--json")
    if ($checkResult.ExitCode -ne 0 -and $checkResult.ExitCode -ne 1) {
        Write-Error "Repair check failed for case '$caseId' with exit code $($checkResult.ExitCode)."
    }

    $diagnostics = @()
    if (-not [string]::IsNullOrWhiteSpace($checkResult.StdOut)) {
        $diagnostics = @(Read-DiagnosticsArray -Label "repair diagnostics for $caseId" -Text $checkResult.StdOut)
    }

    $remainingCodes = @($diagnostics | ForEach-Object { [string] $_.code })
    $success = ($checkResult.ExitCode -eq 0 -and $remainingCodes.Count -eq 0)
    $status = if ($success) { "passed" } else { "failed" }

    $runInfo = $null
    if ($diagnosticCommand -eq "run" -and $success) {
        $runResult = Invoke-Axc -BinaryPath $binary -Arguments @("run", $candidatePath, "--json")
        $runtimeDiagnosticsResult = Try-ReadDiagnosticsArray -Text $runResult.StdOut
        $runtimeDiagnostics = @($runtimeDiagnosticsResult.Diagnostics)
        $runtimeRemainingCodes = @($runtimeDiagnostics | ForEach-Object { [string] $_.code })

        $runInfo = [pscustomobject][ordered]@{
            command            = "run --json"
            command_exit_code  = $runResult.ExitCode
            stdout             = $runResult.StdOut.TrimEnd()
            stderr             = $runResult.StdErr.TrimEnd()
            parsed_diagnostics = [bool] $runtimeDiagnosticsResult.Parsed
            diagnostics        = $runtimeDiagnostics
            remaining_codes    = $runtimeRemainingCodes
        }

        if ($runResult.ExitCode -eq 2 -or ($runtimeDiagnosticsResult.Parsed -and $runtimeRemainingCodes.Count -gt 0)) {
            $success = $false
            $status = "failed"
        }
    } elseif ($RunPrograms -and $success) {
        $runResult = Invoke-Axc -BinaryPath $binary -Arguments @("run", $candidatePath)
        $runInfo = [pscustomobject][ordered]@{
            command_exit_code = $runResult.ExitCode
            stdout            = $runResult.StdOut.TrimEnd()
            stderr            = $runResult.StdErr.TrimEnd()
        }
    }

    Write-Utf8File -Path (Join-Path $caseOutputDir "diagnostics.json") -Text (($diagnostics | ConvertTo-Json -Depth 100) + "`n")

    $result = [pscustomobject][ordered]@{
        id              = $caseId
        diagnostic_command = $diagnosticCommand
        status          = $status
        success         = $success
        candidate_path  = $candidatePath
        benchmark_case  = $case
        remaining_codes = $remainingCodes
        diagnostics     = $diagnostics
        check_exit_code = $checkResult.ExitCode
    }

    if ($runInfo) {
        $result | Add-Member -NotePropertyName run -NotePropertyValue $runInfo
    }

    Write-Utf8File -Path (Join-Path $caseOutputDir "result.json") -Text (($result | ConvertTo-Json -Depth 100) + "`n")
    $results += $result
}

$passedCases = @($results | Where-Object { $_.status -eq "passed" })
$failedCases = @($results | Where-Object { $_.status -eq "failed" })
$missingCases = @($results | Where-Object { $_.status -eq "missing" })

$passed = [int] $passedCases.Count
$failed = [int] $failedCases.Count
$missing = [int] $missingCases.Count
$total = [int] $results.Count

$summary = [ordered]@{
    schema_version = 1
    generated_at   = (Get-Date).ToString("o")
    benchmark_dir  = $benchmarkDirPath
    benchmark_index = $benchmarkIndexPath
    candidates_dir = $CandidatesDir
    output_dir     = $OutputDir
    totals         = [ordered]@{
        total        = $total
        passed       = $passed
        failed       = $failed
        missing      = $missing
    }
    cases          = $results
}

$summaryPath = Join-Path $OutputDir "summary.json"
Write-Utf8File -Path $summaryPath -Text (($summary | ConvertTo-Json -Depth 100) + "`n")

Write-Host ""
Write-Host "Repair benchmark results:"
$results |
    Select-Object `
        @{ Name = "Id"; Expression = { $_.id } }, `
        @{ Name = "Status"; Expression = { $_.status } }, `
        @{ Name = "Remaining"; Expression = { ($_.remaining_codes -join ", ") } } |
    Format-Table -AutoSize

Write-Host ""
Write-Host "Pass rate: $passed / $total"
Write-Host "Summary written to $summaryPath"

if ($failed -gt 0 -or $missing -gt 0) {
    exit 1
}
