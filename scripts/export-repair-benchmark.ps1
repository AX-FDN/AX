param(
    [string] $ManifestPath = "benchmarks\\repair-cases.json",
    [string] $OutputDir = "",
    [switch] $SkipBuild
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoScript = Join-Path $PSScriptRoot "cargo-gnu.ps1"
$repoCargoConfig = Join-Path $repoRoot ".cargo\\config.toml"

if (-not [System.IO.Path]::IsPathRooted($ManifestPath)) {
    $ManifestPath = Join-Path $repoRoot $ManifestPath
}

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $OutputDir = Join-Path $repoRoot ".ax-ai\\repair-benchmark\\$timestamp"
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

function Read-Manifest {
    param([string] $Path)

    if (-not (Test-Path $Path)) {
        Write-Error "Repair benchmark manifest not found: $Path"
    }

    $manifest = Get-Content $Path -Raw -Encoding utf8 | ConvertFrom-Json
    if ($manifest.version -ne 1) {
        Write-Error "Unsupported repair benchmark manifest version: $($manifest.version)"
    }

    $cases = @($manifest.cases)
    if ($cases.Count -eq 0) {
        Write-Error "Repair benchmark manifest contains no cases: $Path"
    }

    return $manifest
}

function Read-DiagnosticsJson {
    param(
        [string] $CaseId,
        [string] $Mode,
        [string] $JsonText,
        [string] $StdErr
    )

    if ([string]::IsNullOrWhiteSpace($JsonText)) {
        $details = if ($StdErr) { "`nstderr:`n$StdErr" } else { "" }
        Write-Error "Compiler produced no JSON for case '$CaseId' in mode '$Mode'.$details"
    }

    try {
        return @($JsonText | ConvertFrom-Json)
    } catch {
        $details = if ($StdErr) { "`nstderr:`n$StdErr" } else { "" }
        Write-Error "Failed to parse compiler JSON for case '$CaseId' in mode '$Mode': $($_.Exception.Message)$details"
    }
}

function Assert-ExactSequence {
    param(
        [string] $CaseId,
        [string] $FieldName,
        [string[]] $Expected,
        [string[]] $Actual
    )

    $expectedValues = @($Expected | ForEach-Object { [string] $_ })
    $actualValues = @($Actual | ForEach-Object { [string] $_ })

    if ($expectedValues.Count -ne $actualValues.Count) {
        Write-Error "Case '$CaseId' expected $FieldName [$($expectedValues -join ', ')] but observed [$($actualValues -join ', ')]"
    }

    for ($i = 0; $i -lt $expectedValues.Count; $i++) {
        if ($expectedValues[$i] -ne $actualValues[$i]) {
            Write-Error "Case '$CaseId' expected $FieldName [$($expectedValues -join ', ')] but observed [$($actualValues -join ', ')]"
        }
    }
}

$manifest = Read-Manifest -Path $ManifestPath

if (-not $SkipBuild) {
    & $cargoScript build | Out-Null
}

$targetDir = Resolve-TargetDir
$binary = Join-Path $targetDir "debug\\axc.exe"
if (-not (Test-Path $binary)) {
    Write-Error "Could not find compiled AX binary at $binary"
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$exportedCases = New-Object System.Collections.Generic.List[object]

foreach ($case in @($manifest.cases)) {
    $caseId = [string] $case.id
    $relativeFile = [string] $case.file
    $sourcePath = Join-Path $repoRoot $relativeFile

    if ([string]::IsNullOrWhiteSpace($caseId)) {
        Write-Error "Encountered repair benchmark case with empty id."
    }

    if (-not (Test-Path $sourcePath)) {
        Write-Error "Repair benchmark input not found for case '$caseId': $relativeFile"
    }

    $caseDir = Join-Path $OutputDir $caseId
    New-Item -ItemType Directory -Force -Path $caseDir | Out-Null

    $sourceText = Get-Content $sourcePath -Raw -Encoding utf8

    $baseResult = Invoke-Axc -BinaryPath $binary -Arguments @("check", $relativeFile, "--json")
    if ($baseResult.ExitCode -ne 0 -and $baseResult.ExitCode -ne 1) {
        Write-Error "Base diagnostics failed for case '$caseId' with exit code $($baseResult.ExitCode)."
    }

    $aiResult = Invoke-Axc -BinaryPath $binary -Arguments @("check", $relativeFile, "--json", "--ai")
    if ($aiResult.ExitCode -ne 0 -and $aiResult.ExitCode -ne 1) {
        Write-Error "AI diagnostics failed for case '$caseId' with exit code $($aiResult.ExitCode)."
    }

    $baseDiagnostics = Read-DiagnosticsJson -CaseId $caseId -Mode "base" -JsonText $baseResult.StdOut -StdErr $baseResult.StdErr
    $aiDiagnostics = Read-DiagnosticsJson -CaseId $caseId -Mode "ai" -JsonText $aiResult.StdOut -StdErr $aiResult.StdErr

    $expectedCodes = @($case.expected_codes | ForEach-Object { [string] $_ })
    $observedBaseCodes = @($baseDiagnostics | ForEach-Object { [string] $_.code })
    $observedAiCodes = @($aiDiagnostics | ForEach-Object { [string] $_.code })
    Assert-ExactSequence -CaseId $caseId -FieldName "expected_codes" -Expected $expectedCodes -Actual $observedBaseCodes
    Assert-ExactSequence -CaseId $caseId -FieldName "expected_codes" -Expected $expectedCodes -Actual $observedAiCodes

    $expectedAiRuleIds = @($case.expected_ai_rule_ids | ForEach-Object { [string] $_ })
    $observedAiRuleIds = @($aiDiagnostics | ForEach-Object {
        if ($_.ai) {
            [string] $_.ai.rule_id
        }
    })
    if ($expectedAiRuleIds.Count -gt 0) {
        Assert-ExactSequence -CaseId $caseId -FieldName "expected_ai_rule_ids" -Expected $expectedAiRuleIds -Actual $observedAiRuleIds
    }

    $sourceArtifact = Join-Path $caseDir "source.ax"
    $baseArtifact = Join-Path $caseDir "diagnostics.base.json"
    $aiArtifact = Join-Path $caseDir "diagnostics.ai.json"
    $caseArtifact = Join-Path $caseDir "case.json"

    Write-Utf8File -Path $sourceArtifact -Text $sourceText
    Write-Utf8File -Path $baseArtifact -Text ($baseResult.StdOut.TrimEnd() + "`n")
    Write-Utf8File -Path $aiArtifact -Text ($aiResult.StdOut.TrimEnd() + "`n")

    $caseSummary = [ordered]@{
        id                   = $caseId
        file                 = $relativeFile
        category             = [string] $case.category
        repair_goal          = [string] $case.repair_goal
        notes                = [string] $case.notes
        expected_codes       = $expectedCodes
        expected_ai_rule_ids = $expectedAiRuleIds
        artifacts            = [ordered]@{
            source           = "$caseId/source.ax"
            base_diagnostics = "$caseId/diagnostics.base.json"
            ai_diagnostics   = "$caseId/diagnostics.ai.json"
        }
        observed             = [ordered]@{
            base_exit_code = $baseResult.ExitCode
            ai_exit_code   = $aiResult.ExitCode
            base_codes     = $observedBaseCodes
            ai_codes       = $observedAiCodes
            ai_rule_ids    = $observedAiRuleIds
        }
    }

    Write-Utf8File -Path $caseArtifact -Text (($caseSummary | ConvertTo-Json -Depth 100) + "`n")
    $exportedCases.Add($caseSummary)
}

$index = [ordered]@{
    schema_version = 1
    generated_at   = (Get-Date).ToString("o")
    manifest_path  = $ManifestPath
    binary_path    = $binary
    output_dir     = $OutputDir
    cases          = $exportedCases
}

$indexPath = Join-Path $OutputDir "index.json"
Write-Utf8File -Path $indexPath -Text (($index | ConvertTo-Json -Depth 100) + "`n")

Write-Host ""
Write-Host "Exported repair benchmark artifacts:"
$exportedCases |
    Select-Object `
        @{ Name = "Id"; Expression = { $_.id } }, `
        @{ Name = "Category"; Expression = { $_.category } }, `
        @{ Name = "Codes"; Expression = { ($_.observed.base_codes -join ", ") } }, `
        @{ Name = "AI Rules"; Expression = { ($_.observed.ai_rule_ids -join ", ") } } |
    Format-Table -AutoSize

Write-Host ""
Write-Host "Index written to $indexPath"
