param(
    [string] $ManifestPath = "benchmarks\\repair-cases-smoke.json",
    [string] $OutputDir = ".ax-ai\\diagnostics-benchmark\\ci-smoke",
    [int] $Iterations = 1
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$benchmarkScript = Join-Path $PSScriptRoot "benchmark-diagnostics.ps1"

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
$outputDir = Resolve-RepoPath -Path $OutputDir

if (-not (Test-Path $manifestPath)) {
    Write-Error "Diagnostics smoke manifest not found: $manifestPath"
}

Remove-RepoDirectoryIfExists -Path $OutputDir

& $benchmarkScript -ManifestPath $manifestPath -Iterations $Iterations -OutputDir $outputDir | Out-Null

$summaryPath = Join-Path $outputDir "summary.json"
if (-not (Test-Path $summaryPath)) {
    Write-Error "Diagnostics smoke did not produce summary.json at $summaryPath"
}

$summary = Get-Content $summaryPath -Raw -Encoding utf8 | ConvertFrom-Json

Assert-Equal -Label "schema_version" -Actual ([int] $summary.schema_version) -Expected 1
Assert-Equal -Label "iterations" -Actual ([int] $summary.iterations) -Expected $Iterations
Assert-Equal -Label "total_cases" -Actual ([int] $summary.total_cases) -Expected 5
Assert-StringArray -Label "mode_order" -Actual @($summary.mode_order) -Expected @("text", "json", "json_ai")
Assert-Equal -Label "per_case_timings count" -Actual (@($summary.per_case_timings).Count) -Expected 15
Assert-Equal -Label "mode_summary count" -Actual (@($summary.mode_summary).Count) -Expected 3

$textMode = @($summary.mode_summary | Where-Object { [string] $_.mode -eq "text" })
$jsonMode = @($summary.mode_summary | Where-Object { [string] $_.mode -eq "json" })
$jsonAiMode = @($summary.mode_summary | Where-Object { [string] $_.mode -eq "json_ai" })

Assert-Equal -Label "text mode rows" -Actual $textMode.Count -Expected 1
Assert-Equal -Label "json mode rows" -Actual $jsonMode.Count -Expected 1
Assert-Equal -Label "json_ai mode rows" -Actual $jsonAiMode.Count -Expected 1
Assert-Equal -Label "text mode files" -Actual ([int] $textMode[0].files) -Expected 5
Assert-Equal -Label "json mode files" -Actual ([int] $jsonMode[0].files) -Expected 5
Assert-Equal -Label "json_ai mode files" -Actual ([int] $jsonAiMode[0].files) -Expected 5

Write-Host "Diagnostics benchmark smoke passed. Stable summary contract verified at $summaryPath"
