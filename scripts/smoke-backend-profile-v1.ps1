param(
    [string] $OutputRoot = "build\backend-profile-v1-smoke"
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoScript = Join-Path $PSScriptRoot "cargo-gnu.ps1"
$parityScript = Join-Path $PSScriptRoot "smoke-aot-parity.ps1"
$repoCargoConfig = Join-Path $repoRoot ".cargo\config.toml"

function Resolve-RepoPath {
    param([string] $Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return Join-Path $repoRoot $Path
}

function Resolve-RelativePath {
    param(
        [string] $FromDirectory,
        [string] $ToPath
    )

    $from = [System.IO.Path]::GetFullPath($FromDirectory)
    if (-not $from.EndsWith([System.IO.Path]::DirectorySeparatorChar)) {
        $from += [System.IO.Path]::DirectorySeparatorChar
    }
    $to = [System.IO.Path]::GetFullPath($ToPath)
    $fromUri = [System.Uri]::new($from)
    $toUri = [System.Uri]::new($to)
    return [System.Uri]::UnescapeDataString($fromUri.MakeRelativeUri($toUri).ToString()).Replace("/", "\")
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

function Resolve-TargetDir {
    if ($env:CARGO_TARGET_DIR) {
        return [string] $env:CARGO_TARGET_DIR
    }

    if (Test-Path $repoCargoConfig) {
        $configText = Get-Content $repoCargoConfig -Raw -Encoding utf8
        if ($configText -match 'target-dir\s*=\s*"([^"]+)"') {
            return [string] $matches[1]
        }
    }

    return Join-Path $repoRoot "target"
}

function Resolve-AxcBinary {
    if (-not [string]::IsNullOrWhiteSpace($env:AXC_BINARY) -and (Test-Path $env:AXC_BINARY)) {
        return [string] $env:AXC_BINARY
    }

    $targetDir = Resolve-TargetDir
    return Join-Path $targetDir "debug\axc.exe"
}

function Ensure-AxcBinary {
    $binary = Resolve-AxcBinary
    if (Test-Path $binary) {
        return $binary
    }

    & $cargoScript build --bin axc --quiet | Out-Null
    $binary = Resolve-AxcBinary
    if (-not (Test-Path $binary)) {
        Write-Error "Could not find compiled AX binary after build: $binary"
    }
    return $binary
}

function Write-Utf8NoBom {
    param(
        [string] $Path,
        [string] $Text
    )

    $encoding = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Text, $encoding)
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

    foreach ($key in $Environment.Keys) {
        $startInfo.Environment[$key] = [string] $Environment[$key]
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

function Assert-TextContains {
    param(
        [string] $Label,
        [string] $Text,
        [string] $Expected
    )

    if (-not $Text.Contains($Expected)) {
        Write-Error "$Label expected to contain '$Expected'."
    }
}

$outputRootPath = Resolve-RepoPath -Path $OutputRoot
if (Test-Path $outputRootPath) {
    Remove-Item -LiteralPath $outputRootPath -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $outputRootPath | Out-Null

$parityOutput = Join-Path $outputRootPath "parity"
$profileCases = @(
    "examples/aot_return.ax",
    "examples/aot_result_try.ax",
    "examples/aot_string_runtime.ax",
    "examples/aot_fs_read.ax",
    "examples/aot_fs_read_dir.ax",
    "examples/aot_process_runtime.ax",
    "examples/project_file_result",
    "examples/project_process_result"
)

& $parityScript -SourcePath $profileCases -OutputRoot $parityOutput

$anchorRoot = Join-Path $outputRootPath "host-result-anchor"
New-Item -ItemType Directory -Force -Path (Join-Path $anchorRoot "src") | Out-Null

$stdPath = (Resolve-RelativePath -FromDirectory $anchorRoot -ToPath (Join-Path $repoRoot "std")).Replace("\", "/")
$manifestText = @"
manifest_version = 1

[package]
name = "backend_profile_host_result_anchor"
entry = "src/main.ax"
sources = ["$stdPath"]
"@

$sourceText = @'
import std.fs;
import std.process;
import std.result;

fn main() -> i32 {
    let file_text: std.result.Result<string, string> = std.fs.try_read_to_string("README.md");
    let entries: std.result.Result<[string], string> = std.fs.try_read_dir("examples");
    let file_size: std.result.Result<i32, string> = std.fs.try_file_size("README.md");
    let run_code: std.result.Result<i32, string> = std.process.try_run("exit 0");
    let status: std.result.Result<std.process.ProcessStatus, string> = std.process.try_status("exit 0");
    let run_in: std.result.Result<i32, string> = std.process.try_run_in(".", "exit 0");
    let status_in: std.result.Result<std.process.ProcessStatus, string> = std.process.try_status_in(".", "exit 0");

    let status_value: std.process.ProcessStatus = std.result.unwrap_or(status, std.process.status_from_code(-1));
    let status_in_value: std.process.ProcessStatus = std.result.unwrap_or(status_in, std.process.status_from_code(-1));

    return string_len(std.result.unwrap_or(file_text, ""))
        + match (entries) { std.result.Result.Ok(items) => len(items), std.result.Result.Err(message) => string_len(message) }
        + std.result.unwrap_or(file_size, 0)
        + std.result.unwrap_or(run_code, 99)
        + status_value.code
        + std.result.unwrap_or(run_in, 99)
        + status_in_value.code;
}
'@

Write-Utf8NoBom -Path (Join-Path $anchorRoot "AX.toml") -Text $manifestText
Write-Utf8NoBom -Path (Join-Path $anchorRoot "src\main.ax") -Text $sourceText

$axcBinary = Ensure-AxcBinary
$anchorBuildOutput = Join-Path $anchorRoot "build"
$build = Invoke-Process -FilePath $axcBinary -Arguments @(
    "build",
    $anchorRoot,
    "--emit",
    "ir",
    "--no-link",
    "--out-dir",
    $anchorBuildOutput
)

if ($build.ExitCode -ne 0) {
    Write-Host $build.Stdout
    Write-Host $build.Stderr
    Write-Error "Backend Profile v1 host Result anchor build failed."
}

$manifestPath = Join-Path $anchorBuildOutput "build-manifest.json"
if (-not (Test-Path $manifestPath)) {
    Write-Error "Backend Profile v1 anchor build did not produce build-manifest.json."
}

$manifest = Get-Content $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json
Assert-Equal -Label "anchor schema_version" -Actual ([int] $manifest.schema_version) -Expected 10
Assert-Equal -Label "anchor aot_readiness.schema_version" -Actual ([int] $manifest.aot_readiness.schema_version) -Expected 3
Assert-Equal -Label "anchor user_code_valid" -Actual ([bool] $manifest.user_code_valid) -Expected $true
Assert-Equal -Label "anchor interpreter_supported" -Actual ([bool] $manifest.interpreter_supported) -Expected $true
Assert-Equal -Label "anchor backend.status" -Actual ([string] $manifest.backend.status) -Expected "ir_generated"
Assert-Equal -Label "anchor aot_readiness.status" -Actual ([string] $manifest.aot_readiness.status) -Expected "ir_generated"
Assert-Equal -Label "anchor aot_readiness blocker count" -Actual @($manifest.aot_readiness.blockers).Count -Expected 0

$llvmIrArtifact = [string] $manifest.artifacts.llvm_ir
if ([string]::IsNullOrWhiteSpace($llvmIrArtifact)) {
    Write-Error "Backend Profile v1 anchor manifest should include artifacts.llvm_ir."
}

$llvmIrPath = Join-Path $anchorBuildOutput $llvmIrArtifact
if (-not (Test-Path $llvmIrPath)) {
    Write-Error "Backend Profile v1 anchor LLVM IR artifact is missing: $llvmIrPath"
}

$llvmIr = Get-Content $llvmIrPath -Raw -Encoding utf8
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "define private { i32, ptr } @ax_host_error_ok()"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "define private { i32, ptr } @ax_host_error_new(i32 %code, ptr %message)"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "define private i1 @ax_host_error_is_ok({ i32, ptr } %error)"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "define private ptr @ax_host_error_message_or_default({ i32, ptr } %error)"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "define %ax_enum_std_result_Result_string__string_ @ax_std_fs_try_read_to_string"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "define %ax_enum_std_result_Result___string__string_ @ax_std_fs_try_read_dir"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "define %ax_enum_std_result_Result_i32__string_ @ax_std_fs_try_file_size"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "define %ax_enum_std_result_Result_i32__string_ @ax_std_process_try_run"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "define %ax_enum_std_result_Result_std_process_ProcessStatus__string_ @ax_std_process_try_status"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "define %ax_enum_std_result_Result_i32__string_ @ax_std_process_try_run_in"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "define %ax_enum_std_result_Result_std_process_ProcessStatus__string_ @ax_std_process_try_status_in"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "call i1 @ax_fs_is_file(ptr"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "call i1 @ax_fs_is_dir(ptr"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "call ptr @ax_fs_read_to_string(ptr"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "call { ptr, i32 } @ax_fs_read_dir(ptr"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "call i32 @ax_fs_file_size(ptr"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "call i32 @ax_process_run(ptr"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "call i32 @ax_process_run_in(ptr"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "readable file does not exist"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "readable directory does not exist"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "sized file does not exist"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "command must not be empty"
Assert-TextContains -Label "host Result anchor IR" -Text $llvmIr -Expected "working directory does not exist"

Write-Host "Backend Profile v1 smoke passed at $outputRootPath"
