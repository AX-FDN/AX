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
    [Parameter(Mandatory = $true)]
    [string] $SourceDir
)

$ErrorActionPreference = "Stop"

if (-not [System.IO.Path]::IsPathRooted($SourceDir)) {
    $SourceDir = Join-Path (Get-Location) $SourceDir
}

$candidatePaths = @(
    (Join-Path $SourceDir "$CaseId.ax"),
    (Join-Path (Join-Path $SourceDir $CaseId) "repaired.ax")
)

$candidatePath = $candidatePaths | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $candidatePath) {
    Write-Error "Replay candidate not found for case '$CaseId' in $SourceDir"
}

$parent = Split-Path -Parent $OutputPath
if ($parent) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
}

Copy-Item -LiteralPath $candidatePath -Destination $OutputPath -Force
Write-Output "Replayed $CaseId ($FeedbackMode) from $candidatePath"
