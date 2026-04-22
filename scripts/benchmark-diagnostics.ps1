param(
    [int] $Iterations = 10,
    [string] $ManifestPath = "benchmarks\\repair-cases.json",
    [string[]] $Files
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
$results |
    Group-Object Mode |
    ForEach-Object {
        $avg = ($_.Group | Measure-Object -Property AvgMs -Average).Average
        [pscustomobject]@{
            Mode  = $_.Name
            Files = $_.Count
            AvgMs = [math]::Round($avg, 2)
        }
    } |
    Sort-Object Mode |
    Format-Table -AutoSize
