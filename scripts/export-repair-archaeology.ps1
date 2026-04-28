param(
    [string] $ComparisonPath = "",
    [string] $OutputDir = "",
    [int] $MaxCases = 0,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $CaseIds = @()
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$normalizedCaseIds = New-Object System.Collections.Generic.List[string]
foreach ($caseIdValue in @($CaseIds)) {
    foreach ($caseIdPart in ([string] $caseIdValue -split ",")) {
        $trimmedCaseId = $caseIdPart.Trim()
        if (-not [string]::IsNullOrWhiteSpace($trimmedCaseId)) {
            $normalizedCaseIds.Add($trimmedCaseId)
        }
    }
}
$CaseIds = $normalizedCaseIds.ToArray()

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

function Resolve-RepoPath {
    param([string] $Path)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $Path
    }

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return Join-Path $repoRoot $Path
}

function Resolve-ComparisonPath {
    param([string] $InputPath)

    if (-not [string]::IsNullOrWhiteSpace($InputPath)) {
        $resolved = Resolve-RepoPath -Path $InputPath
        if ((Test-Path $resolved) -and (Get-Item $resolved).PSIsContainer) {
            $resolved = Join-Path $resolved "comparison.json"
        }

        if (-not (Test-Path $resolved)) {
            Write-Error "Comparison artifact not found: $resolved"
        }

        return $resolved
    }

    $comparisonRoot = Join-Path $repoRoot ".ax-ai\repair-comparisons"
    if (-not (Test-Path $comparisonRoot)) {
        Write-Error "No repair comparison root found: $comparisonRoot. Run compare-repair-feedback.ps1 first or pass -ComparisonPath."
    }

    $latest = Get-ChildItem $comparisonRoot -Recurse -File -Filter "comparison.json" |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1

    if (-not $latest) {
        Write-Error "No comparison.json found under $comparisonRoot. Run compare-repair-feedback.ps1 first or pass -ComparisonPath."
    }

    return $latest.FullName
}

function Convert-ToArtifactPath {
    param([object] $Value)

    if ($null -eq $Value) {
        return $null
    }

    $path = [string] $Value
    if ([string]::IsNullOrWhiteSpace($path)) {
        return $null
    }

    if (-not [System.IO.Path]::IsPathRooted($path)) {
        return $path.Replace('\', '/')
    }

    $normalizedRepo = [System.IO.Path]::GetFullPath($repoRoot).TrimEnd('\', '/')
    $normalizedPath = [System.IO.Path]::GetFullPath($path)
    $comparison = [System.StringComparison]::OrdinalIgnoreCase

    if ($normalizedPath.Equals($normalizedRepo, $comparison)) {
        return "."
    }

    $repoPrefix = $normalizedRepo + [System.IO.Path]::DirectorySeparatorChar
    if ($normalizedPath.StartsWith($repoPrefix, $comparison)) {
        return $normalizedPath.Substring($repoPrefix.Length).Replace('\', '/')
    }

    return $path.Replace('\', '/')
}

function New-MapById {
    param([object[]] $Items)

    $map = @{}
    foreach ($item in @($Items)) {
        if ($null -ne $item -and $null -ne $item.PSObject.Properties["id"]) {
            $map[[string] $item.id] = $item
        }
    }
    return $map
}

function New-JsonArray {
    param([object[]] $Values = @())

    $list = [System.Collections.ArrayList]::new()
    foreach ($value in @($Values)) {
        [void] $list.Add($value)
    }

    return ,$list
}

function Join-ArtifactPath {
    param(
        [string] $Root,
        [object] $RelativePath
    )

    if ($null -eq $RelativePath) {
        return $null
    }

    $relative = [string] $RelativePath
    if ([string]::IsNullOrWhiteSpace($relative)) {
        return $null
    }

    if ([System.IO.Path]::IsPathRooted($relative)) {
        return $relative
    }

    return Join-Path $Root $relative
}

function Get-FirstValue {
    param([object[]] $Values = @())

    foreach ($value in @($Values)) {
        if ($null -ne $value -and -not [string]::IsNullOrWhiteSpace([string] $value)) {
            return [string] $value
        }
    }

    return $null
}

function Read-OptionalJsonFile {
    param([string] $Path)

    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path $Path)) {
        return $null
    }

    return Read-JsonFile -Path $Path -Label $Path
}

function Get-ContextInfo {
    param(
        [object] $ExportCase,
        [string] $BenchmarkRoot
    )

    $bundlePath = Join-ArtifactPath -Root $BenchmarkRoot -RelativePath $ExportCase.artifacts.ai_bundle
    $bundle = Read-OptionalJsonFile -Path $bundlePath

    if ($null -eq $bundle -or $null -eq $bundle.PSObject.Properties["context_bundle"]) {
        return [ordered]@{
            included      = $false
            symbol        = $null
            views         = New-JsonArray
            artifact_path = $null
        }
    }

    $contextBundle = $bundle.context_bundle
    $views = @()
    if ($null -ne $contextBundle.views) {
        $views = @($contextBundle.views.PSObject.Properties | ForEach-Object { [string] $_.Name })
    }

    return [ordered]@{
        included      = $true
        symbol        = [string] $contextBundle.symbol
        views         = New-JsonArray -Values $views
        artifact_path = Convert-ToArtifactPath -Value $bundlePath
    }
}

function Get-DeltaClassification {
    param([string] $Delta)

    switch ($Delta) {
        "improved" { return "ai_feedback_lift" }
        "regressed" { return "regression" }
        "both_pass" { return "stable_repair" }
        "both_fail" { return "unresolved" }
        default { return "not_comparable" }
    }
}

function Get-DeltaInterpretation {
    param(
        [string] $Delta,
        [string] $BaseRemaining,
        [string] $AiRemaining
    )

    switch ($Delta) {
        "improved" {
            return "AI-enhanced feedback passed where base feedback did not. Remaining diagnostics changed from '$BaseRemaining' to '$AiRemaining'."
        }
        "regressed" {
            return "AI-enhanced feedback regressed relative to base feedback. This case must be inspected before making public lift claims."
        }
        "both_pass" {
            return "Base and AI feedback both produced valid candidates in deterministic replay."
        }
        "both_fail" {
            return "Neither compared mode produced a passing candidate in deterministic replay."
        }
        default {
            return "The compared modes were not fully available for this case."
        }
    }
}

function Format-ListText {
    param([object[]] $Values = @())

    $items = @($Values | ForEach-Object { [string] $_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    if ($items.Count -eq 0) {
        return "(none)"
    }

    return ($items -join ", ")
}

function Escape-MarkdownCell {
    param([object] $Value)

    if ($null -eq $Value) {
        return "(none)"
    }

    $text = [string] $Value
    if ([string]::IsNullOrWhiteSpace($text)) {
        return "(none)"
    }

    return $text.Replace("|", "\|").Replace("`r", " ").Replace("`n", " ")
}

function New-ModeArtifact {
    param(
        [string] $Name,
        [object] $RunCase,
        [object] $ScoreCase,
        [object] $ExportCase,
        [string] $BenchmarkRoot,
        [string] $ScoreSummaryPath
    )

    $bundleProperty = "${Name}_bundle"
    $promptProperty = "${Name}_prompt"
    $bundlePath = $null
    $promptPath = $null

    if ($RunCase) {
        $bundlePath = [string] $RunCase.bundle_path
        $promptPath = [string] $RunCase.prompt_path
    }

    if ([string]::IsNullOrWhiteSpace($bundlePath) -and $ExportCase.artifacts.PSObject.Properties[$bundleProperty]) {
        $bundlePath = Join-ArtifactPath -Root $BenchmarkRoot -RelativePath $ExportCase.artifacts.$bundleProperty
    }
    if ([string]::IsNullOrWhiteSpace($promptPath) -and $ExportCase.artifacts.PSObject.Properties[$promptProperty]) {
        $promptPath = Join-ArtifactPath -Root $BenchmarkRoot -RelativePath $ExportCase.artifacts.$promptProperty
    }

    $diagnosticsPath = $null
    if ($ScoreSummaryPath) {
        $scoreRoot = Split-Path -Parent $ScoreSummaryPath
        $diagnosticsPath = Join-Path (Join-Path $scoreRoot ([string] $ExportCase.id)) "diagnostics.json"
    }

    $runInfo = $null
    if ($ScoreCase -and $ScoreCase.PSObject.Properties["run"]) {
        $runInfo = $ScoreCase.run
    }

    return [ordered]@{
        name       = $Name
        input      = [ordered]@{
            bundle_path = Convert-ToArtifactPath -Value $bundlePath
            prompt_path = Convert-ToArtifactPath -Value $promptPath
        }
        candidate  = [ordered]@{
            status               = if ($RunCase) { [string] $RunCase.status } else { "missing" }
            path                 = if ($RunCase) { Convert-ToArtifactPath -Value $RunCase.output_path } else { $null }
            invocation_exit_code = if ($RunCase) { $RunCase.exit_code } else { $null }
            timed_out            = if ($RunCase) { [bool] $RunCase.timed_out } else { $false }
        }
        validation = [ordered]@{
            status           = if ($ScoreCase) { [string] $ScoreCase.status } else { "missing" }
            success          = if ($ScoreCase) { [bool] $ScoreCase.success } else { $false }
            check_exit_code  = if ($ScoreCase) { $ScoreCase.check_exit_code } else { $null }
            remaining_codes  = if ($ScoreCase) { New-JsonArray -Values @($ScoreCase.remaining_codes) } else { New-JsonArray }
            diagnostics_path = Convert-ToArtifactPath -Value $diagnosticsPath
            run              = $runInfo
        }
    }
}

function Build-MarkdownReport {
    param([object] $Artifact)

    $lines = New-Object System.Collections.Generic.List[string]
    $null = $lines.Add("# Repair Archaeology: $($Artifact.case.id)")
    $null = $lines.Add("")
    $null = $lines.Add("## Summary")
    $null = $lines.Add("")
    $null = $lines.Add("- Category: $($Artifact.case.category)")
    $null = $lines.Add("- Subject: $($Artifact.subject.kind) ``$($Artifact.subject.file)``")
    if ($Artifact.subject.project) {
        $null = $lines.Add("- Project: ``$($Artifact.subject.project)``")
        $null = $lines.Add("- Project target: ``$($Artifact.subject.project_target_relative_path)``")
    }
    $null = $lines.Add("- Diagnostic command: ``axc $($Artifact.case.diagnostic_command) --json``")
    $null = $lines.Add("- Outcome: $($Artifact.comparison.delta)")
    $null = $lines.Add("- Claim boundary: deterministic replay, not live-model evidence")
    $null = $lines.Add("")
    $null = $lines.Add("## Initial Diagnostic")
    $null = $lines.Add("")
    $null = $lines.Add("- Expected codes: $(Format-ListText -Values @($Artifact.initial_diagnostic.expected_codes))")
    $null = $lines.Add("- Observed codes: $(Format-ListText -Values @($Artifact.initial_diagnostic.observed_codes))")
    $null = $lines.Add("- AI rule ids: $(Format-ListText -Values @($Artifact.initial_diagnostic.observed_ai_rule_ids))")
    $null = $lines.Add("- Repair goal: $($Artifact.case.repair_goal)")
    $null = $lines.Add("")
    $null = $lines.Add("## Context")
    $null = $lines.Add("")
    $null = $lines.Add("- Included: $($Artifact.context.included)")
    $null = $lines.Add("- Views: $(Format-ListText -Values @($Artifact.context.views))")
    $null = $lines.Add("- Symbol: $(if ($Artifact.context.symbol) { $Artifact.context.symbol } else { '(none)' })")
    $null = $lines.Add("")
    $null = $lines.Add("## Timeline")
    $null = $lines.Add("")
    $null = $lines.Add("| Mode | Candidate | Validation | Remaining diagnostics |")
    $null = $lines.Add("| --- | --- | --- | --- |")
    foreach ($mode in @($Artifact.modes)) {
        $remaining = Format-ListText -Values @($mode.validation.remaining_codes)
        $candidate = "$($mode.candidate.status): $($mode.candidate.path)"
        $validation = "$($mode.validation.status): success=$($mode.validation.success)"
        $null = $lines.Add("| $($mode.name) | $(Escape-MarkdownCell -Value $candidate) | $(Escape-MarkdownCell -Value $validation) | $(Escape-MarkdownCell -Value $remaining) |")
    }
    $null = $lines.Add("")
    $null = $lines.Add("## What Changed")
    $null = $lines.Add("")
    foreach ($fact in @($Artifact.archaeology_summary.facts)) {
        $null = $lines.Add("- $fact")
    }
    $null = $lines.Add("- Interpretation: $($Artifact.archaeology_summary.interpretation)")
    $null = $lines.Add("")
    $null = $lines.Add("## Failure / Regression Notes")
    $null = $lines.Add("")
    if ($Artifact.comparison.delta -eq "regressed") {
        $null = $lines.Add("- This case regressed and should block any stronger public claim until inspected.")
    } elseif ($Artifact.comparison.delta -eq "both_fail") {
        $null = $lines.Add("- This case remains unresolved in both compared modes.")
    } else {
        $null = $lines.Add("- No failure or regression in the compared final mode.")
    }
    if (-not [bool] $Artifact.context.included) {
        $null = $lines.Add("- No context bundle was included for this artifact.")
    }
    $null = $lines.Add("")
    $null = $lines.Add("## Reproduce")
    $null = $lines.Add("")
    $null = $lines.Add('```powershell')
    foreach ($command in @($Artifact.reproducibility.commands)) {
        $null = $lines.Add($command)
    }
    $null = $lines.Add('```')
    $null = $lines.Add("")
    $null = $lines.Add("## Artifacts")
    $null = $lines.Add("")
    $null = $lines.Add("- Benchmark index: ``$($Artifact.reproducibility.benchmark_index)``")
    $null = $lines.Add("- Comparison: ``$($Artifact.reproducibility.comparison_path)``")
    foreach ($mode in @($Artifact.modes)) {
        $null = $lines.Add("- $($mode.name) bundle: ``$($mode.input.bundle_path)``")
        $null = $lines.Add("- $($mode.name) candidate: ``$($mode.candidate.path)``")
        $null = $lines.Add("- $($mode.name) diagnostics: ``$($mode.validation.diagnostics_path)``")
    }

    return ($lines -join "`n") + "`n"
}

$ComparisonPath = Resolve-ComparisonPath -InputPath $ComparisonPath
$comparison = Read-JsonFile -Path $ComparisonPath -Label "comparison artifact"

if ($null -eq $comparison.modes -or $null -eq $comparison.modes.base -or $null -eq $comparison.modes.ai) {
    Write-Error "Repair Archaeology v0 currently requires a base -> ai comparison.json from compare-repair-feedback.ps1."
}

$benchmarkIndexPath = Resolve-RepoPath -Path ([string] $comparison.benchmark_index)
$benchmarkIndex = Read-JsonFile -Path $benchmarkIndexPath -Label "benchmark index"
$benchmarkRoot = Split-Path -Parent $benchmarkIndexPath

$baseRunSummaryPath = Resolve-RepoPath -Path ([string] $comparison.modes.base.run_summary_path)
$aiRunSummaryPath = Resolve-RepoPath -Path ([string] $comparison.modes.ai.run_summary_path)
$baseScoreSummaryPath = Resolve-RepoPath -Path ([string] $comparison.modes.base.score_summary_path)
$aiScoreSummaryPath = Resolve-RepoPath -Path ([string] $comparison.modes.ai.score_summary_path)

$baseRunSummary = Read-JsonFile -Path $baseRunSummaryPath -Label "base run summary"
$aiRunSummary = Read-JsonFile -Path $aiRunSummaryPath -Label "ai run summary"
$baseScoreSummary = Read-JsonFile -Path $baseScoreSummaryPath -Label "base score summary"
$aiScoreSummary = Read-JsonFile -Path $aiScoreSummaryPath -Label "ai score summary"

if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $OutputDir = Join-Path $repoRoot ".ax-ai\repair-archaeology\$timestamp"
} else {
    $OutputDir = Resolve-RepoPath -Path $OutputDir
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$casesDir = Join-Path $OutputDir "cases"
New-Item -ItemType Directory -Force -Path $casesDir | Out-Null

$exportCaseMap = New-MapById -Items @($benchmarkIndex.cases)
$comparisonCaseMap = New-MapById -Items @($comparison.cases)
$baseRunMap = New-MapById -Items @($baseRunSummary.cases)
$aiRunMap = New-MapById -Items @($aiRunSummary.cases)
$baseScoreMap = New-MapById -Items @($baseScoreSummary.cases)
$aiScoreMap = New-MapById -Items @($aiScoreSummary.cases)

$selectedComparisonCases = @($comparison.cases)
if ($CaseIds.Count -gt 0) {
    $wanted = @{}
    foreach ($caseId in @($CaseIds)) {
        $wanted[[string] $caseId] = $true
    }
    $selectedComparisonCases = @($selectedComparisonCases | Where-Object { $wanted.ContainsKey([string] $_.id) })

    foreach ($caseId in @($CaseIds)) {
        if (-not $comparisonCaseMap.ContainsKey([string] $caseId)) {
            Write-Error "Case '$caseId' was not found in comparison artifact $ComparisonPath"
        }
    }
}

if ($MaxCases -gt 0) {
    $selectedComparisonCases = @($selectedComparisonCases | Select-Object -First $MaxCases)
}

if ($selectedComparisonCases.Count -eq 0) {
    Write-Error "No cases selected for Repair Archaeology export."
}

$exportedCaseSummaries = New-Object System.Collections.Generic.List[object]

foreach ($comparisonCase in @($selectedComparisonCases)) {
    $caseId = [string] $comparisonCase.id
    if (-not $exportCaseMap.ContainsKey($caseId)) {
        Write-Error "Case '$caseId' exists in comparison but not in benchmark index."
    }

    $exportCase = $exportCaseMap[$caseId]
    $baseRunCase = $baseRunMap[$caseId]
    $aiRunCase = $aiRunMap[$caseId]
    $baseScoreCase = $baseScoreMap[$caseId]
    $aiScoreCase = $aiScoreMap[$caseId]
    $contextInfo = Get-ContextInfo -ExportCase $exportCase -BenchmarkRoot $benchmarkRoot

    $observedCodes = @()
    if ($exportCase.observed -and $exportCase.observed.ai_codes) {
        $observedCodes = @($exportCase.observed.ai_codes)
    } elseif ($exportCase.observed -and $exportCase.observed.base_codes) {
        $observedCodes = @($exportCase.observed.base_codes)
    }

    $observedAiRuleIds = @()
    if ($exportCase.observed -and $exportCase.observed.ai_rule_ids) {
        $observedAiRuleIds = @($exportCase.observed.ai_rule_ids)
    }

    $baseRemaining = Format-ListText -Values @($comparisonCase.base_remaining_codes)
    $aiRemaining = Format-ListText -Values @($comparisonCase.ai_remaining_codes)
    $delta = [string] $comparisonCase.delta
    $classification = Get-DeltaClassification -Delta $delta
    $interpretation = Get-DeltaInterpretation -Delta $delta -BaseRemaining $baseRemaining -AiRemaining $aiRemaining

    $artifact = [ordered]@{
        schema_version     = 1
        generated_at       = (Get-Date).ToString("o")
        case               = [ordered]@{
            id                 = $caseId
            category           = [string] $exportCase.category
            diagnostic_command = [string] $exportCase.diagnostic_command
            repair_goal        = [string] $exportCase.repair_goal
            notes              = [string] $exportCase.notes
        }
        subject            = [ordered]@{
            kind                         = if ($exportCase.PSObject.Properties["project"]) { "project" } else { "file" }
            file                         = [string] $exportCase.file
            project                      = if ($exportCase.PSObject.Properties["project"]) { [string] $exportCase.project } else { $null }
            project_target_relative_path = if ($exportCase.PSObject.Properties["project_target_relative_path"]) { [string] $exportCase.project_target_relative_path } else { $null }
        }
        initial_diagnostic = [ordered]@{
            expected_codes       = New-JsonArray -Values @($exportCase.expected_codes)
            observed_codes       = New-JsonArray -Values $observedCodes
            expected_ai_rule_ids = New-JsonArray -Values @($exportCase.expected_ai_rule_ids)
            observed_ai_rule_ids = New-JsonArray -Values $observedAiRuleIds
            primary_code         = Get-FirstValue -Values $observedCodes
            primary_rule_id      = Get-FirstValue -Values $observedAiRuleIds
            primary_repair_goal  = [string] $exportCase.repair_goal
        }
        repair_contract    = [ordered]@{
            feedback_modes             = New-JsonArray -Values @("base", "ai")
            pass_condition             = "check has no diagnostics"
            runtime_pass_condition     = "run cases must not emit runtime diagnostics"
            candidate_budget_per_mode  = 1
        }
        context            = $contextInfo
        modes              = New-JsonArray -Values @(
            (New-ModeArtifact -Name "base" -RunCase $baseRunCase -ScoreCase $baseScoreCase -ExportCase $exportCase -BenchmarkRoot $benchmarkRoot -ScoreSummaryPath $baseScoreSummaryPath),
            (New-ModeArtifact -Name "ai" -RunCase $aiRunCase -ScoreCase $aiScoreCase -ExportCase $exportCase -BenchmarkRoot $benchmarkRoot -ScoreSummaryPath $aiScoreSummaryPath)
        )
        comparison         = [ordered]@{
            delta           = $delta
            base_success    = [bool] $comparisonCase.base_success
            ai_success      = [bool] $comparisonCase.ai_success
            cold_success    = $null
            improved_modes  = if ($delta -eq "improved") { New-JsonArray -Values @("ai") } else { New-JsonArray }
            regressed_modes = if ($delta -eq "regressed") { New-JsonArray -Values @("ai") } else { New-JsonArray }
        }
        archaeology_summary = [ordered]@{
            classification = $classification
            facts          = New-JsonArray -Values @(
                "base status: $([string] $comparisonCase.base_status); remaining diagnostics: $baseRemaining",
                "ai status: $([string] $comparisonCase.ai_status); remaining diagnostics: $aiRemaining"
            )
            interpretation = $interpretation
        }
        reproducibility    = [ordered]@{
            benchmark_index    = Convert-ToArtifactPath -Value $benchmarkIndexPath
            comparison_path    = Convert-ToArtifactPath -Value $ComparisonPath
            run_summary_paths  = [ordered]@{
                base = Convert-ToArtifactPath -Value $baseRunSummaryPath
                ai   = Convert-ToArtifactPath -Value $aiRunSummaryPath
            }
            score_summary_paths = [ordered]@{
                base = Convert-ToArtifactPath -Value $baseScoreSummaryPath
                ai   = Convert-ToArtifactPath -Value $aiScoreSummaryPath
            }
            commands          = New-JsonArray -Values @(
                "powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\export-repair-benchmark.ps1 -ManifestPath benchmarks\repair-cases.json -OutputDir <benchmark-output> -SkipBuild",
                "powershell -NoProfile -ExecutionPolicy Bypass -Command `"& { .\scripts\compare-repair-feedback.ps1 -BenchmarkDir '<benchmark-output>' -RunnerScript '.\scripts\replay-repair-adapter.ps1' -RunnerExtraArgs @('-SourceDir', '.\benchmarks\repair-candidates\compare\shared', '-SourceDirBase', '.\benchmarks\repair-candidates\compare\base') -OutputDir '<comparison-output>' -SkipBuild }`"",
                "powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\export-repair-archaeology.ps1 -ComparisonPath '$((Convert-ToArtifactPath -Value $ComparisonPath))' -OutputDir '$((Convert-ToArtifactPath -Value $OutputDir))'"
            )
        }
        provenance         = [ordered]@{
            repo_relative    = $true
            source_kind      = "deterministic_replay"
            live_model_claim = $false
        }
    }

    $caseJsonPath = Join-Path $casesDir "$caseId.json"
    $caseMarkdownPath = Join-Path $casesDir "$caseId.md"
    Write-Utf8File -Path $caseJsonPath -Text (Format-JsonText -Value $artifact)
    Write-Utf8File -Path $caseMarkdownPath -Text (Build-MarkdownReport -Artifact $artifact)

    $exportedCaseSummaries.Add([pscustomobject][ordered]@{
        id       = $caseId
        category = [string] $exportCase.category
        delta    = $delta
        json     = "cases/$caseId.json"
        markdown = "cases/$caseId.md"
    })
}

$caseCount = [int] $exportedCaseSummaries.Count
$totals = [ordered]@{
    total          = $caseCount
    improved       = [int] @($exportedCaseSummaries | Where-Object { $_.delta -eq "improved" }).Count
    regressed      = [int] @($exportedCaseSummaries | Where-Object { $_.delta -eq "regressed" }).Count
    both_pass      = [int] @($exportedCaseSummaries | Where-Object { $_.delta -eq "both_pass" }).Count
    both_fail      = [int] @($exportedCaseSummaries | Where-Object { $_.delta -eq "both_fail" }).Count
    not_comparable = [int] @($exportedCaseSummaries | Where-Object { $_.delta -eq "not_comparable" }).Count
}

$index = [ordered]@{
    schema_version   = 1
    generated_at     = (Get-Date).ToString("o")
    source_kind      = "deterministic_replay"
    live_model_claim = $false
    benchmark_index  = Convert-ToArtifactPath -Value $benchmarkIndexPath
    comparison_path  = Convert-ToArtifactPath -Value $ComparisonPath
    output_dir       = Convert-ToArtifactPath -Value $OutputDir
    totals           = $totals
    cases            = $exportedCaseSummaries
}

$indexPath = Join-Path $OutputDir "index.json"
Write-Utf8File -Path $indexPath -Text (Format-JsonText -Value $index)

Write-Host ""
Write-Host "Repair Archaeology export:"
$exportedCaseSummaries |
    Select-Object `
        @{ Name = "Id"; Expression = { $_.id } }, `
        @{ Name = "Category"; Expression = { $_.category } }, `
        @{ Name = "Delta"; Expression = { $_.delta } }, `
        @{ Name = "Json"; Expression = { $_.json } }, `
        @{ Name = "Markdown"; Expression = { $_.markdown } } |
    Format-Table -AutoSize

Write-Host ""
Write-Host "Archaeology index written to $indexPath"
