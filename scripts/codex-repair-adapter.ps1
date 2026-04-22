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
    [string] $CodexCommand = "codex",
    [string] $Model = "",
    [string] $Profile = "",
    [string[]] $ConfigOverride = @()
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot

function Resolve-AbsolutePath {
    param([string] $Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return Join-Path $repoRoot $Path
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

function Invoke-ExternalCommandWithInput {
    param(
        [string] $CommandPath,
        [string[]] $Arguments,
        [string] $WorkingDirectory,
        [string] $StdInText
    )

    $stdoutPath = Join-Path $env:TEMP ("ax-codex-stdout-" + [guid]::NewGuid().ToString("N") + ".txt")
    $stderrPath = Join-Path $env:TEMP ("ax-codex-stderr-" + [guid]::NewGuid().ToString("N") + ".txt")

    try {
        Push-Location $WorkingDirectory
        try {
            $previousErrorActionPreference = $ErrorActionPreference
            try {
                $ErrorActionPreference = "Continue"
                $StdInText | & $CommandPath @Arguments 1> $stdoutPath 2> $stderrPath
                $exitCode = $LASTEXITCODE
            } finally {
                $ErrorActionPreference = $previousErrorActionPreference
            }
        } finally {
            Pop-Location
        }

        $stdout = if (Test-Path $stdoutPath) {
            Get-Content $stdoutPath -Raw -Encoding utf8
        } else {
            ""
        }

        $stderr = if (Test-Path $stderrPath) {
            Get-Content $stderrPath -Raw -Encoding utf8
        } else {
            ""
        }

        return [pscustomobject]@{
            ExitCode = $exitCode
            StdOut   = $stdout
            StdErr   = $stderr
        }
    } finally {
        if (Test-Path $stdoutPath) {
            Remove-Item -LiteralPath $stdoutPath -Force
        }

        if (Test-Path $stderrPath) {
            Remove-Item -LiteralPath $stderrPath -Force
        }
    }
}

function Read-JsonFile {
    param(
        [string] $Path,
        [string] $Label
    )

    try {
        return Get-Content $Path -Raw -Encoding utf8 | ConvertFrom-Json
    } catch {
        Write-Error "Failed to parse ${Label}: $($_.Exception.Message)"
    }
}

function Build-RepairPrompt {
    param(
        [string] $PromptText,
        [object] $Bundle,
        [string] $CaseId,
        [string] $FeedbackMode
    )

    $repairGoal = if ($Bundle.repair_goal) {
        [string] $Bundle.repair_goal
    } else {
        "Repair the AX program so axc check succeeds."
    }

    $notes = if ($Bundle.notes) {
        [string] $Bundle.notes
    } else {
        ""
    }

    $bundleJson = ($Bundle | ConvertTo-Json -Depth 100)
    $notesBlock = ""
    if (-not [string]::IsNullOrWhiteSpace($notes)) {
        $notesBlock = "- notes: $notes`n"
    }

    @"
You are running inside the AX repair benchmark through the Codex adapter.

Final response contract:
- Return a JSON object that matches the provided schema.
- Put the full repaired AX source code in the `repaired_source` field.
- Do not use markdown fences.
- Do not explain the fix.

Repair strategy:
- Prefer the smallest valid source edit.
- Stay inside the AX prototype syntax implemented in this repository.
- Keep explicit type annotations.
- Do not invent unsupported features.

Benchmark metadata:
- case_id: $CaseId
- feedback_mode: $FeedbackMode
- repair_goal: $repairGoal
$notesBlock
Provider-neutral repair prompt:
$PromptText

Structured repair bundle:
~~~json
$bundleJson
~~~
"@
}

function Get-RepairedSource {
    param([string] $ResponseText)

    $trimmed = $ResponseText.Trim()
    if ([string]::IsNullOrWhiteSpace($trimmed)) {
        Write-Error "Codex returned an empty response."
    }

    try {
        $payload = $trimmed | ConvertFrom-Json
        if ($payload.repaired_source -and -not [string]::IsNullOrWhiteSpace([string] $payload.repaired_source)) {
            return [string] $payload.repaired_source
        }
    } catch {
    }

    if ($trimmed -match '(?s)```(?:json)?\s*(\{.*?\})\s*```') {
        try {
            $payload = $matches[1] | ConvertFrom-Json
            if ($payload.repaired_source -and -not [string]::IsNullOrWhiteSpace([string] $payload.repaired_source)) {
                return [string] $payload.repaired_source
            }
        } catch {
        }
    }

    if ($trimmed -match '(?s)```(?:ax)?\s*(.*?)\s*```') {
        return [string] $matches[1]
    }

    return $trimmed
}

$PromptPath = Resolve-AbsolutePath -Path $PromptPath
$BundlePath = Resolve-AbsolutePath -Path $BundlePath
$OutputPath = Resolve-AbsolutePath -Path $OutputPath

if (-not (Test-Path $PromptPath)) {
    Write-Error "Prompt file not found: $PromptPath"
}

if (-not (Test-Path $BundlePath)) {
    Write-Error "Bundle file not found: $BundlePath"
}

$codex = Get-Command $CodexCommand -ErrorAction SilentlyContinue
if (-not $codex) {
    Write-Error "Could not find Codex CLI command '$CodexCommand'. Run `codex --help` to confirm installation."
}

$commandPath = if ($codex.Source) { [string] $codex.Source } else { [string] $codex.Definition }
$commandPrefixArguments = @()
if (
    $commandPath.EndsWith(".ps1", [System.StringComparison]::OrdinalIgnoreCase) -or
    $commandPath.EndsWith(".cmd", [System.StringComparison]::OrdinalIgnoreCase)
) {
    $shimDir = Split-Path -Parent $commandPath
    $codexJsPath = Join-Path $shimDir "node_modules\\@openai\\codex\\bin\\codex.js"
    $node = Get-Command node -ErrorAction SilentlyContinue

    if ($node -and (Test-Path $codexJsPath)) {
        $commandPath = if ($node.Source) { [string] $node.Source } else { [string] $node.Definition }
        $commandPrefixArguments = @($codexJsPath)
    }
}

$bundle = Read-JsonFile -Path $BundlePath -Label "repair bundle"
if ($bundle.case_id -and ([string] $bundle.case_id) -ne $CaseId) {
    Write-Error "Bundle case_id '$($bundle.case_id)' does not match requested CaseId '$CaseId'."
}

$promptText = Get-Content $PromptPath -Raw -Encoding utf8
$adapterPrompt = Build-RepairPrompt -PromptText $promptText -Bundle $bundle -CaseId $CaseId -FeedbackMode $FeedbackMode

$tempRoot = Join-Path $env:TEMP ("ax-codex-repair-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $tempRoot | Out-Null

$schemaPath = Join-Path $tempRoot "response.schema.json"
$responsePath = Join-Path $tempRoot "response.json"

$schemaText = @"
{
  "type": "object",
  "properties": {
    "repaired_source": {
      "type": "string"
    }
  },
  "required": ["repaired_source"],
  "additionalProperties": false
}
"@

Write-Utf8File -Path $schemaPath -Text $schemaText

$arguments = @("-a", "never", "exec")

if (-not [string]::IsNullOrWhiteSpace($Profile)) {
    $arguments += @("-p", $Profile)
}

if (-not [string]::IsNullOrWhiteSpace($Model)) {
    $arguments += @("-m", $Model)
}

foreach ($override in @($ConfigOverride)) {
    $arguments += @("-c", [string] $override)
}

$arguments += @(
    "-C",
    $repoRoot,
    "-s",
    "read-only",
    "--skip-git-repo-check",
    "--color",
    "never",
    "--output-schema",
    $schemaPath,
    "-o",
    $responsePath,
    "-"
)

try {
    $result = Invoke-ExternalCommandWithInput `
        -CommandPath $commandPath `
        -Arguments ($commandPrefixArguments + $arguments) `
        -WorkingDirectory $repoRoot `
        -StdInText $adapterPrompt

    if ($result.ExitCode -ne 0) {
        $details = @()
        if ($result.StdOut) {
            $details += "stdout:`n$($result.StdOut.TrimEnd())"
        }
        if ($result.StdErr) {
            $details += "stderr:`n$($result.StdErr.TrimEnd())"
        }
        $suffix = if ($details.Count -gt 0) { "`n`n" + ($details -join "`n`n") } else { "" }
        Write-Error "Codex adapter failed for case '$CaseId' with exit code $($result.ExitCode).$suffix"
    }

    $rawResponse = if (Test-Path $responsePath) {
        Get-Content $responsePath -Raw -Encoding utf8
    } else {
        $result.StdOut
    }

    $repairedSource = Get-RepairedSource -ResponseText $rawResponse
    Write-Utf8File -Path $OutputPath -Text ($repairedSource.TrimEnd() + "`n")
} finally {
    if (Test-Path $tempRoot) {
        Remove-Item -LiteralPath $tempRoot -Recurse -Force
    }
}

Write-Output "Generated repair for $CaseId with Codex ($FeedbackMode)."
