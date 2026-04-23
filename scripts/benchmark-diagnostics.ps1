param(
    [int] $Iterations = 10,
    [string] $ManifestPath = "benchmarks\\repair-cases.json",
    [string[]] $Files,
    [string] $OutputDir = ""
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

if ($Iterations -lt 1) {
    Write-Error "Iterations must be at least 1."
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoScript = Join-Path $PSScriptRoot "cargo-gnu.ps1"
$repoCargoConfig = Join-Path $repoRoot ".cargo\config.toml"

if (-not [System.IO.Path]::IsPathRooted($ManifestPath)) {
    $ManifestPath = Join-Path $repoRoot $ManifestPath
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $OutputDir = Join-Path $repoRoot ".ax-ai\\diagnostics-benchmark\\$timestamp"
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

function Get-ManifestFiles {
    param([string] $Path)

    if (-not (Test-Path $Path)) {
        Write-Error "Benchmark manifest not found: $Path"
    }

    $manifest = Get-Content $Path -Raw -Encoding utf8 | ConvertFrom-Json
    $files = @($manifest.cases | ForEach-Object { [string] $_.file })
    if ($files.Count -eq 0) {
        Write-Error "Benchmark manifest contains no case files: $Path"
    }

    return $files
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
    $null = $process.StandardOutput.ReadToEnd()
    $null = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    return $process.ExitCode
}

function Build-MarkdownReport {
    param([object] $Summary)

    $lines = New-Object System.Collections.Generic.List[string]
    $null = $lines.Add("# AX Diagnostics Benchmark")
    $null = $lines.Add("")
    $null = $lines.Add("- Generated at: $($Summary.generated_at)")
    $null = $lines.Add("- Manifest path: $($Summary.manifest_path)")
    $null = $lines.Add("- Iterations per file/mode: $($Summary.iterations)")
    $null = $lines.Add("- Cases: $($Summary.total_cases)")
    $null = $lines.Add("- AX binary: $($Summary.binary_path)")
    $null = $lines.Add("")
    $null = $lines.Add("## Mode Summary")
    $null = $lines.Add("")
    $null = $lines.Add("| Mode | Cases | Avg ms | Min ms | Max ms | Total ms |")
    $null = $lines.Add("| --- | ---: | ---: | ---: | ---: | ---: |")

    foreach ($mode in @($Summary.mode_summary)) {
        $null = $lines.Add("| $($mode.mode) | $($mode.files) | $($mode.avg_ms) | $($mode.min_ms) | $($mode.max_ms) | $($mode.total_ms) |")
    }

    $null = $lines.Add("")
    $null = $lines.Add("## Pairwise Overhead")
    $null = $lines.Add("")
    $null = $lines.Add("| From | To | Avg overhead ms | Relative overhead % |")
    $null = $lines.Add("| --- | --- | ---: | ---: |")

    foreach ($pair in @($Summary.pairwise_overhead)) {
        $relative = if ($null -ne $pair.relative_overhead_pct) { "$($pair.relative_overhead_pct)%" } else { "n/a" }
        $null = $lines.Add("| $($pair.from_mode) | $($pair.to_mode) | $($pair.avg_overhead_ms) | $relative |")
    }

    $null = $lines.Add("")
    $null = $lines.Add("## Per-Case Timings")
    $null = $lines.Add("")
    $null = $lines.Add("| File | Mode | Iterations | Avg ms | Total ms |")
    $null = $lines.Add("| --- | --- | ---: | ---: | ---: |")

    foreach ($row in @($Summary.per_case_timings)) {
        $null = $lines.Add("| $($row.file) | $($row.mode) | $($row.iterations) | $($row.avg_ms) | $($row.total_ms) |")
    }

    return ($lines -join "`n") + "`n"
}

function Get-ModeSummaryByName {
    param(
        [object[]] $ModeSummary,
        [string] $ModeName
    )

    return @($ModeSummary | Where-Object { $_.Mode -eq $ModeName })[0]
}

function Build-PairwiseOverhead {
    param(
        [string] $FromMode,
        [string] $ToMode,
        [object[]] $Results,
        [object[]] $ModeSummary
    )

    $fromRows = @($Results | Where-Object { $_.Mode -eq $FromMode } | Sort-Object File)
    $toRows = @($Results | Where-Object { $_.Mode -eq $ToMode } | Sort-Object File)
    if ($fromRows.Count -ne $toRows.Count) {
        Write-Error "Cannot compare mode '$FromMode' to '$ToMode' because the row counts differ."
    }

    $deltas = New-Object System.Collections.Generic.List[double]
    for ($index = 0; $index -lt $fromRows.Count; $index += 1) {
        if ($fromRows[$index].File -ne $toRows[$index].File) {
            Write-Error "Cannot compare mode '$FromMode' to '$ToMode' because file ordering drifted."
        }

        $delta = [double] $toRows[$index].AvgMs - [double] $fromRows[$index].AvgMs
        $deltas.Add($delta)
    }

    $avgDelta = ($deltas | Measure-Object -Average).Average
    $fromSummary = Get-ModeSummaryByName -ModeSummary $ModeSummary -ModeName $FromMode
    $toSummary = Get-ModeSummaryByName -ModeSummary $ModeSummary -ModeName $ToMode
    $relative = if ([double] $fromSummary.AvgMs -gt 0) {
        [math]::Round((($toSummary.AvgMs - $fromSummary.AvgMs) / $fromSummary.AvgMs) * 100, 2)
    } else {
        $null
    }

    return [ordered]@{
        from_mode             = $FromMode
        to_mode               = $ToMode
        avg_from_ms           = [math]::Round([double] $fromSummary.AvgMs, 2)
        avg_to_ms             = [math]::Round([double] $toSummary.AvgMs, 2)
        avg_overhead_ms       = [math]::Round([double] $avgDelta, 2)
        relative_overhead_pct = $relative
    }
}

if (-not $Files -or $Files.Count -eq 0) {
    $Files = Get-ManifestFiles -Path $ManifestPath
}

& $cargoScript build | Out-Null

$targetDir = Resolve-TargetDir
$binary = Join-Path $targetDir "debug\axc.exe"
if (-not (Test-Path $binary)) {
    Write-Error "Could not find compiled AX binary at $binary"
}

$modes = @(
    @{ Name = "text"; Args = @("check") },
    @{ Name = "json"; Args = @("check", "--json") },
    @{ Name = "json_ai"; Args = @("check", "--json", "--ai") }
)

$results = New-Object System.Collections.Generic.List[object]

foreach ($relativeFile in $Files) {
    $resolvedFile = Join-Path $repoRoot $relativeFile
    if (-not (Test-Path $resolvedFile)) {
        Write-Error "Benchmark input not found: $relativeFile"
    }

    foreach ($mode in $modes) {
        $exitCode = Invoke-Axc -BinaryPath $binary -Arguments ($mode.Args + @($relativeFile))
        if ($exitCode -ne 0 -and $exitCode -ne 1) {
            Write-Error "Warm-up failed for mode '$($mode.Name)' on '$relativeFile' with exit code $exitCode"
        }

        $watch = [System.Diagnostics.Stopwatch]::StartNew()
        for ($i = 0; $i -lt $Iterations; $i++) {
            $exitCode = Invoke-Axc -BinaryPath $binary -Arguments ($mode.Args + @($relativeFile))
            if ($exitCode -ne 0 -and $exitCode -ne 1) {
                Write-Error "Benchmark failed for mode '$($mode.Name)' on '$relativeFile' with exit code $exitCode"
            }
        }
        $watch.Stop()

        $results.Add([pscustomobject]@{
            File       = $relativeFile
            Mode       = $mode.Name
            Iterations = $Iterations
            TotalMs    = [math]::Round($watch.Elapsed.TotalMilliseconds, 2)
            AvgMs      = [math]::Round($watch.Elapsed.TotalMilliseconds / $Iterations, 2)
        })
    }
}

Write-Host ""
Write-Host "Per-file timings"
$results | Sort-Object File, Mode | Format-Table -AutoSize

Write-Host ""
Write-Host "Mode summary"
$modeSummary = @(
    $results |
    Group-Object Mode |
    ForEach-Object {
        $avg = ($_.Group | Measure-Object -Property AvgMs -Average).Average
        $total = ($_.Group | Measure-Object -Property TotalMs -Sum).Sum
        $min = ($_.Group | Measure-Object -Property AvgMs -Minimum).Minimum
        $max = ($_.Group | Measure-Object -Property AvgMs -Maximum).Maximum
        [pscustomobject]@{
            Mode    = $_.Name
            Files   = $_.Count
            AvgMs   = [math]::Round($avg, 2)
            MinMs   = [math]::Round($min, 2)
            MaxMs   = [math]::Round($max, 2)
            TotalMs = [math]::Round($total, 2)
        }
    }
)

$modeSummary |
    Sort-Object Mode |
    Format-Table -AutoSize

$pairwiseOverhead = @(
    Build-PairwiseOverhead -FromMode "text" -ToMode "json" -Results $results -ModeSummary $modeSummary
    Build-PairwiseOverhead -FromMode "json" -ToMode "json_ai" -Results $results -ModeSummary $modeSummary
    Build-PairwiseOverhead -FromMode "text" -ToMode "json_ai" -Results $results -ModeSummary $modeSummary
)

Write-Host ""
Write-Host "Pairwise overhead"
$pairwiseOverhead | Format-Table -AutoSize

$summary = [ordered]@{
    schema_version   = 1
    generated_at     = (Get-Date).ToString("o")
    manifest_path    = $ManifestPath
    output_dir       = $OutputDir
    iterations       = $Iterations
    total_cases      = @($Files).Count
    target_dir       = $targetDir
    binary_path      = $binary
    mode_order       = @($modes | ForEach-Object { $_.Name })
    per_case_timings = @(
        $results |
            Sort-Object File, Mode |
            ForEach-Object {
                [ordered]@{
                    file       = $_.File
                    mode       = $_.Mode
                    iterations = $_.Iterations
                    total_ms   = $_.TotalMs
                    avg_ms     = $_.AvgMs
                }
            }
    )
    mode_summary     = @(
        $modeSummary |
            Sort-Object Mode |
            ForEach-Object {
                [ordered]@{
                    mode     = $_.Mode
                    files    = $_.Files
                    avg_ms   = $_.AvgMs
                    min_ms   = $_.MinMs
                    max_ms   = $_.MaxMs
                    total_ms = $_.TotalMs
                }
            }
    )
    pairwise_overhead = @($pairwiseOverhead)
}

$summaryJsonPath = Join-Path $OutputDir "summary.json"
$summaryMarkdownPath = Join-Path $OutputDir "summary.md"
Write-Utf8File -Path $summaryJsonPath -Text (Format-JsonText -Value $summary)
Write-Utf8File -Path $summaryMarkdownPath -Text (Build-MarkdownReport -Summary $summary)

Write-Host ""
Write-Host "Summary JSON written to $summaryJsonPath"
Write-Host "Summary Markdown written to $summaryMarkdownPath"
