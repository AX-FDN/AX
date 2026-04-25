param(
    [string] $OutputDir = ".ax-smoke\\project-representatives"
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoScript = Join-Path $PSScriptRoot "cargo-gnu.ps1"
$repoCargoConfig = Join-Path $repoRoot ".cargo\\config.toml"

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

function Ensure-AxcBinary {
    $binary = Resolve-AxcBinary
    if (Test-Path $binary) {
        return $binary
    }

    & $cargoScript build --bin axc | Out-Null

    $binary = Resolve-AxcBinary
    if (-not (Test-Path $binary)) {
        Write-Error "Could not find compiled AX binary after build: $binary"
    }

    return $binary
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

function Assert-PathExists {
    param(
        [string] $Label,
        [string] $Path
    )

    if (-not (Test-Path $Path)) {
        Write-Error "$Label missing at $Path"
    }
}

function Join-RelativePath {
    param(
        [string] $Root,
        [string] $RelativePath
    )

    $path = $Root
    foreach ($segment in ($RelativePath -split '/')) {
        $path = Join-Path $path $segment
    }
    return $path
}

$sharedFoundation = @(
    "external/foundation/cli.ax",
    "external/foundation/file_kind.ax",
    "external/foundation/report.ax",
    "external/foundation/search.ax",
    "external/foundation/text.ax",
    "external/foundation/workspace.ax"
)

$cases = @(
    [pscustomobject]@{
        Id = "project_release_promote"
        Tier = "core"
        Path = "examples/project_release_promote"
        ExpectedSources = @($sharedFoundation + @(
                "lib/receipt.ax",
                "src/main.ax"
            ))
    },
    [pscustomobject]@{
        Id = "project_directory_index"
        Tier = "core"
        Path = "examples/project_directory_index"
        ExpectedSources = @($sharedFoundation + @(
                "lib/index_totals.ax",
                "lib/report.ax",
                "lib/scan.ax",
                "src/main.ax"
            ))
    },
    [pscustomobject]@{
        Id = "project_text_normalize"
        Tier = "core"
        Path = "examples/project_text_normalize"
        ExpectedSources = @($sharedFoundation + @(
                "lib/normalize.ax",
                "lib/report.ax",
                "src/main.ax"
            ))
    },
    [pscustomobject]@{
        Id = "project_command_capture"
        Tier = "host"
        Path = "examples/project_command_capture"
        ExpectedSources = @($sharedFoundation + @(
                "src/main.ax"
            ))
    },
    [pscustomobject]@{
        Id = "project_command_batch"
        Tier = "host"
        Path = "examples/project_command_batch"
        ExpectedSources = @($sharedFoundation + @(
                "lib/report.ax",
                "src/main.ax"
            ))
    }
)

$outputRoot = Resolve-RepoPath -Path $OutputDir
Remove-RepoDirectoryIfExists -Path $OutputDir
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

$axcBinary = Ensure-AxcBinary

$coreCount = 0
$hostCount = 0

foreach ($case in $cases) {
    $examplePath = Resolve-RepoPath -Path $case.Path
    $caseOutDir = Join-Path $outputRoot $case.Id

    & $axcBinary build $examplePath --out-dir $caseOutDir | Out-Null

    $manifestPath = Join-Path $caseOutDir "build-manifest.json"
    $projectSourcesRoot = Join-Path $caseOutDir "project-sources"

    Assert-PathExists -Label "$($case.Id) build manifest" -Path $manifestPath
    Assert-PathExists -Label "$($case.Id) copied manifest" -Path (Join-Path $caseOutDir "AX.toml")
    Assert-PathExists -Label "$($case.Id) source copy" -Path (Join-Path $caseOutDir "source.ax")
    Assert-PathExists -Label "$($case.Id) project-sources" -Path $projectSourcesRoot

    $manifest = Get-Content $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
    Assert-StringArray `
        -Label "$($case.Id) project_sources" `
        -Actual @($manifest.artifacts.project_sources) `
        -Expected $case.ExpectedSources

    foreach ($source in $case.ExpectedSources) {
        Assert-PathExists `
            -Label "$($case.Id) copied source $source" `
            -Path (Join-RelativePath -Root $projectSourcesRoot -RelativePath $source)
    }

    if ([string] $case.Tier -eq "core") {
        $coreCount += 1
    } else {
        $hostCount += 1
    }
}

Assert-Equal -Label "core representative count" -Actual $coreCount -Expected 3
Assert-Equal -Label "host validation count" -Actual $hostCount -Expected 2

Write-Host "Representative project smoke passed. Verified $($cases.Count) builds ($coreCount core, $hostCount host-validation) at $outputRoot"
