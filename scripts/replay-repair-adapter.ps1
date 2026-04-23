param(
    [Parameter(Mandatory = $true)]
    [string] $PromptPath,
    [Parameter(Mandatory = $true)]
    [string] $BundlePath,
    [Parameter(Mandatory = $true)]
    [string] $OutputPath,
    [Parameter(Mandatory = $true)]
    [string] $CaseId,
    [Parameter(Mandatory = $true)]
    [string] $FeedbackMode,
    [string] $SourceDir = "",
    [string] $SourceDirBase = "",
    [string] $SourceDirAi = ""
)

$ErrorActionPreference = "Stop"

function Resolve-SourceDirectory {
    param(
        [string] $Path,
        [string] $Label
    )

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $null
    }

    if (-not [System.IO.Path]::IsPathRooted($Path)) {
        $Path = Join-Path (Get-Location) $Path
    }

    if (-not (Test-Path $Path)) {
        Write-Error "$Label not found: $Path"
    }

    return $Path
}

function Get-CandidatePaths {
    param([string] $Root)

    return @(
        (Join-Path $Root "$CaseId.ax"),
        (Join-Path (Join-Path $Root $CaseId) "repaired.ax")
    )
}

$resolvedModeSourceDir = if ($FeedbackMode -eq "base") {
    Resolve-SourceDirectory -Path $SourceDirBase -Label "SourceDirBase"
} else {
    Resolve-SourceDirectory -Path $SourceDirAi -Label "SourceDirAi"
}
$resolvedDefaultSourceDir = Resolve-SourceDirectory -Path $SourceDir -Label "SourceDir"

$searchRoots = @()
if ($resolvedModeSourceDir) {
    $searchRoots += $resolvedModeSourceDir
}
if ($resolvedDefaultSourceDir) {
    $searchRoots += $resolvedDefaultSourceDir
}

if ($searchRoots.Count -eq 0) {
    Write-Error "At least one replay source directory must be provided. Use -SourceDir, -SourceDirBase, or -SourceDirAi."
}

$candidatePath = $null
foreach ($root in $searchRoots) {
    $candidatePath = Get-CandidatePaths -Root $root | Where-Object { Test-Path $_ } | Select-Object -First 1
    if ($candidatePath) {
        break
    }
}

if (-not $candidatePath) {
    $searchedRoots = $searchRoots -join ", "
    Write-Error "Replay candidate not found for case '$CaseId' in: $searchedRoots"
}

$parent = Split-Path -Parent $OutputPath
if ($parent) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

Copy-Item -LiteralPath $candidatePath -Destination $OutputPath -Force
Write-Output "Replayed $CaseId ($FeedbackMode) from $candidatePath"
