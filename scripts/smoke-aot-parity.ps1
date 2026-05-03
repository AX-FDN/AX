param(
    [string[]] $SourcePath = @(
        "examples/aot_return.ax",
        "examples/aot_math.ax",
        "examples/aot_control_flow.ax",
        "examples/aot_loop.ax",
        "examples/consts.ax",
        "examples/modulo.ax",
        "examples/for_loop.ax",
        "examples/break_loop.ax",
        "examples/continue.ax",
        "examples/for_in.ax",
        "examples/aot_slice_range.ax",
        "examples/aot_slice_for_in.ax",
        "examples/aot_bool_logic.ax",
        "examples/aot_comparisons.ax",
        "examples/aot_nested_calls.ax",
        "examples/aot_print.ax",
        "examples/aot_print_string.ax",
        "examples/aot_string_values.ax",
        "examples/aot_string_len_compare.ax",
        "examples/aot_string_runtime.ax",
        "examples/aot_string_predicates.ax",
        "examples/aot_string_replace.ax",
        "examples/aot_string_split_lines.ax",
        "examples/aot_string_split_lines_for_in.ax",
        "examples/aot_string_trim.ax",
        "examples/string_match.ax",
        "examples/aot_array_read.ax",
        "examples/aot_array_write.ax",
        "examples/aot_array_to_string.ax",
        "examples/aot_struct_read.ax",
        "examples/aot_struct_write.ax",
        "examples/aot_struct_to_string.ax",
        "examples/match_struct_pattern.ax",
        "examples/aot_enum_unit.ax",
        "examples/aot_enum_match.ax",
        "examples/aot_payload_enum.ax",
        "examples/aot_payload_enum_equality.ax",
        "examples/aot_enum_to_string.ax",
        "examples/aot_enum_print.ax",
        "examples/aot_generic_enum_print.ax",
        "examples/aot_match_expression.ax",
        "examples/match_range.ax",
        "examples/match_or.ax",
        "examples/match_guard.ax",
        "examples/aot_result_option.ax",
        "examples/aot_result_try.ax"
    ),
    [string] $OutputRoot = ""
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoScript = Join-Path $PSScriptRoot "cargo-gnu.ps1"
$manifestPath = Join-Path $repoRoot "Cargo.toml"
$cargoConfigPath = Join-Path (Join-Path $repoRoot ".cargo") "config.toml"

function Resolve-RepoPath {
    param([string] $Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return Join-Path $repoRoot $Path
}

function Resolve-PowerShell {
    $pwsh = Get-Command pwsh -ErrorAction SilentlyContinue
    if ($pwsh) {
        return [string] $pwsh.Source
    }

    $powershell = Get-Command powershell -ErrorAction SilentlyContinue
    if ($powershell) {
        return [string] $powershell.Source
    }

    Write-Error "PowerShell was not found."
}

function Resolve-CargoTargetDir {
    if (-not [string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
        $targetDir = [string] $env:CARGO_TARGET_DIR
        if ([System.IO.Path]::IsPathRooted($targetDir)) {
            return $targetDir
        }
        return (Join-Path $repoRoot $targetDir)
    }

    if (Test-Path $cargoConfigPath) {
        $configText = Get-Content $cargoConfigPath -Raw -Encoding utf8
        if ($configText -match 'target-dir\s*=\s*"([^"]+)"') {
            $targetDir = [string] $matches[1]
            if ([System.IO.Path]::IsPathRooted($targetDir)) {
                return $targetDir
            }
            return (Join-Path $repoRoot $targetDir)
        }
    }

    return (Join-Path $repoRoot "target")
}

function Resolve-AxcBinary {
    if (-not [string]::IsNullOrWhiteSpace($env:AXC_BINARY)) {
        return [string] $env:AXC_BINARY
    }

    $suffix = if ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
            [System.Runtime.InteropServices.OSPlatform]::Windows
        )) {
        ".exe"
    } else {
        ""
    }

    $targetDir = Resolve-CargoTargetDir
    $candidate = Join-Path (Join-Path $targetDir "debug") "axc$suffix"
    if (Test-Path $candidate) {
        return $candidate
    }

    $fallback = Join-Path (Join-Path (Join-Path $repoRoot "target") "debug") "axc$suffix"
    if (Test-Path $fallback) {
        return $fallback
    }

    return ""
}

function Invoke-Process {
    param(
        [string] $FilePath,
        [string[]] $Arguments = @(),
        [hashtable] $Environment = @{}
    )

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $FilePath
    $startInfo.WorkingDirectory = $repoRoot
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.Arguments = Join-ProcessArguments $Arguments

    foreach ($name in $Environment.Keys) {
        $startInfo.Environment[$name] = [string] $Environment[$name]
    }

    $process = [System.Diagnostics.Process]::Start($startInfo)
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()

    [pscustomobject] @{
        ExitCode = $process.ExitCode
        Stdout = $stdout
        Stderr = $stderr
    }
}

function Join-ProcessArguments {
    param([string[]] $Arguments)

    $quoted = @()
    foreach ($argument in $Arguments) {
        $text = [string] $argument
        if ($text.Length -eq 0) {
            $quoted += '""'
        } elseif ($text -match '[\s"]') {
            $quoted += '"' + $text.Replace('"', '\"') + '"'
        } else {
            $quoted += $text
        }
    }

    return ($quoted -join " ")
}

function Build-AxcIfNeeded {
    $axc = Resolve-AxcBinary
    if (-not [string]::IsNullOrWhiteSpace($axc)) {
        return $axc
    }

    Write-Host "Building axc for AOT parity smoke..."
    $isWindows = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
        [System.Runtime.InteropServices.OSPlatform]::Windows
    )

    if ($isWindows) {
        $powershell = Resolve-PowerShell
        $args = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $cargoScript, "build", "--quiet")
        $build = Invoke-Process -FilePath $powershell -Arguments $args
    } else {
        $build = Invoke-Process -FilePath "cargo" -Arguments @(
            "build",
            "--quiet",
            "--locked",
            "--manifest-path",
            $manifestPath
        )
    }

    if ($build.ExitCode -ne 0) {
        Write-Error "failed to build axc for AOT parity smoke`nstdout:`n$($build.Stdout)`nstderr:`n$($build.Stderr)"
    }

    $axc = Resolve-AxcBinary
    if ([string]::IsNullOrWhiteSpace($axc)) {
        Write-Error "axc binary was not found after build."
    }

    return $axc
}

function Resolve-Clang {
    if (-not [string]::IsNullOrWhiteSpace($env:AX_LLVM_CLANG)) {
        return [string] $env:AX_LLVM_CLANG
    }

    $command = Get-Command clang -ErrorAction SilentlyContinue
    if ($command) {
        return [string] $command.Source
    }

    Write-Error "clang was not found. Install clang or set AX_LLVM_CLANG before running the AOT parity smoke."
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

function Normalize-Text {
    param([string] $Text)
    return $Text.Replace("`r`n", "`n")
}

function Assert-ProcessResultEqual {
    param(
        [string] $CaseLabel,
        $Interpreter,
        $Executable
    )

    Assert-Equal -Label "$CaseLabel exit code" -Actual ([int] $Executable.ExitCode) -Expected ([int] $Interpreter.ExitCode)
    Assert-Equal -Label "$CaseLabel stdout" -Actual (Normalize-Text $Executable.Stdout) -Expected (Normalize-Text $Interpreter.Stdout)
    Assert-Equal -Label "$CaseLabel stderr" -Actual (Normalize-Text $Executable.Stderr) -Expected (Normalize-Text $Interpreter.Stderr)
}

function Invoke-Axc {
    param(
        [string] $Axc,
        [string[]] $Arguments,
        [hashtable] $Environment = @{}
    )

    Invoke-Process -FilePath $Axc -Arguments $Arguments -Environment $Environment
}

function Format-Blockers {
    param($Blockers)

    if ($null -eq $Blockers) {
        return "<none>"
    }

    $items = @()
    foreach ($blocker in @($Blockers)) {
        $code = [string] $blocker.code
        $action = [string] $blocker.resolution.agent_action
        $message = [string] $blocker.message
        $items += "${code}/${action}: $message"
    }
    return ($items -join "`n")
}

$axc = Build-AxcIfNeeded
$clang = Resolve-Clang

Write-Host "Using axc for AOT parity: $axc"
Write-Host "Using clang for AOT parity: $clang"

if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
    $tempRoot = if (-not [string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) {
        [string] $env:RUNNER_TEMP
    } else {
        [System.IO.Path]::GetTempPath()
    }
    $OutputRoot = Join-Path $tempRoot "ax-aot-parity-smoke"
} else {
    $OutputRoot = Resolve-RepoPath -Path $OutputRoot
}

if (Test-Path $OutputRoot) {
    Remove-Item -LiteralPath $OutputRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $OutputRoot | Out-Null

$buildEnv = @{
    AX_LLVM_AOT_LINK = "1"
    AX_LLVM_CLANG = $clang
}

foreach ($source in $SourcePath) {
    $resolvedSource = Resolve-RepoPath -Path $source
    if (-not (Test-Path $resolvedSource)) {
        Write-Error "AOT parity source not found: $resolvedSource"
    }

    $caseName = [System.IO.Path]::GetFileNameWithoutExtension($resolvedSource)
    $caseOutDir = Join-Path $OutputRoot $caseName

    $check = Invoke-Axc -Axc $axc -Arguments @("check", $resolvedSource)
    Assert-Equal -Label "$source check exit code" -Actual ([int] $check.ExitCode) -Expected 0

    $interpreter = Invoke-Axc -Axc $axc -Arguments @("run", $resolvedSource)

    $build = Invoke-Axc -Axc $axc -Arguments @(
        "build",
        $resolvedSource,
        "--out-dir",
        $caseOutDir,
        "--json"
    ) -Environment $buildEnv
    Assert-Equal -Label "$source build exit code" -Actual ([int] $build.ExitCode) -Expected 0

    try {
        $manifest = $build.Stdout | ConvertFrom-Json
    } catch {
        Write-Error "AOT parity build stdout was not valid manifest JSON for $source.`nstdout:`n$($build.Stdout)`nstderr:`n$($build.Stderr)"
    }

    Assert-Equal -Label "$source manifest schema_version" -Actual ([int] $manifest.schema_version) -Expected 9
    Assert-Equal -Label "$source aot_readiness.schema_version" -Actual ([int] $manifest.aot_readiness.schema_version) -Expected 3
    Assert-Equal -Label "$source user_code_valid" -Actual ([bool] $manifest.user_code_valid) -Expected $true
    Assert-Equal -Label "$source interpreter_supported" -Actual ([bool] $manifest.interpreter_supported) -Expected $true
    Assert-Equal -Label "$source aot_supported" -Actual ([bool] $manifest.aot_supported) -Expected $true
    Assert-Equal -Label "$source backend.kind" -Actual ([string] $manifest.backend.kind) -Expected "llvm-aot"
    Assert-Equal -Label "$source backend.status" -Actual ([string] $manifest.backend.status) -Expected "built"

    $executableArtifact = [string] $manifest.artifacts.executable
    if ([string]::IsNullOrWhiteSpace($executableArtifact)) {
        $blockers = Format-Blockers $manifest.aot_readiness.blockers
        Write-Error "AOT parity build did not produce an executable for $source. Blockers:`n$blockers"
    }

    $executablePath = Join-Path $caseOutDir $executableArtifact
    if (-not (Test-Path $executablePath)) {
        Write-Error "AOT parity executable is missing for $source`: $executablePath"
    }

    $executable = Invoke-Process -FilePath $executablePath
    Assert-ProcessResultEqual -CaseLabel $source -Interpreter $interpreter -Executable $executable

    Write-Host "AOT parity passed: $source exit=$($executable.ExitCode) exe=$executablePath"
}

Write-Host "LLVM AOT parity smoke passed for $($SourcePath.Count) case(s)."
