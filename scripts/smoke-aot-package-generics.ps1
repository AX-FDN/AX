param(
    [string] $OutputDir = "target\aot-package-generics-smoke"
)

$ErrorActionPreference = "Stop"

if (Get-Variable -Name PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoScript = Join-Path $PSScriptRoot "cargo-gnu.ps1"
$repoCargoConfig = Join-Path $repoRoot ".cargo\config.toml"

function Resolve-RepoPath {
    param([string] $Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return Join-Path $repoRoot $Path
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
    if (-not [string]::IsNullOrWhiteSpace($env:AXC_BINARY) -and (Test-Path $env:AXC_BINARY)) {
        return [string] $env:AXC_BINARY
    }
    if (-not [string]::IsNullOrWhiteSpace($env:CARGO_BIN_EXE_axc) -and (Test-Path $env:CARGO_BIN_EXE_axc)) {
        return [string] $env:CARGO_BIN_EXE_axc
    }

    return Join-Path (Resolve-TargetDir) "debug\axc.exe"
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

function Resolve-Clang {
    if (-not [string]::IsNullOrWhiteSpace($env:AX_LLVM_CLANG) -and (Test-Path $env:AX_LLVM_CLANG)) {
        return [string] $env:AX_LLVM_CLANG
    }

    $command = Get-Command clang -ErrorAction SilentlyContinue
    if ($command) {
        return [string] $command.Source
    }

    $commonWindowsClang = "C:\Program Files\LLVM\bin\clang.exe"
    if (Test-Path $commonWindowsClang) {
        return $commonWindowsClang
    }

    Write-Error "clang was not found. Install LLVM clang or set AX_LLVM_CLANG before running package generic AOT parity."
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

$outputRoot = Resolve-RepoPath -Path $OutputDir

if (Test-Path $outputRoot) {
    Remove-Item -LiteralPath $outputRoot -Recurse -Force
}

$packageRoot = Join-Path $outputRoot "packages\generic_helpers"
New-Item -ItemType Directory -Force -Path (Join-Path $outputRoot "src") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $packageRoot "src") | Out-Null

Write-Utf8NoBom -Path (Join-Path $outputRoot "AX.toml") -Text @'
manifest_version = 1

[package]
name = "aot_package_generics_smoke"
entry = "src/main.ax"

[dependencies]
generic_helpers = { path = "packages/generic_helpers" }
'@

Write-Utf8NoBom -Path (Join-Path $packageRoot "AX.toml") -Text @'
manifest_version = 1

[package]
name = "generic_helpers"
sources = ["src"]
'@

Write-Utf8NoBom -Path (Join-Path $packageRoot "src\core.ax") -Text @'
module generic_helpers.core;

struct Box<T> {
    value: T,
}

fn identity<T>(value: T) -> T {
    return value;
}

fn box_value<T>(box: Box<T>) -> T {
    return box.value;
}
'@

Write-Utf8NoBom -Path (Join-Path $outputRoot "src\main.ax") -Text @'
import generic_helpers.core;

fn main() -> i32 {
    let left: i32 = generic_helpers.core.identity(4);
    let boxed: generic_helpers.core.Box<i32> = generic_helpers.core.Box { value: 6 };
    let right: i32 = generic_helpers.core.box_value(boxed);
    return left + right;
}
'@

$axcBinary = Ensure-AxcBinary
$clang = Resolve-Clang

$check = Invoke-Process -FilePath $axcBinary -Arguments @("check", $outputRoot)
Assert-Equal -Label "axc check exit code" -Actual ([int] $check.ExitCode) -Expected 0

$lock = Invoke-Process -FilePath $axcBinary -Arguments @("lock", $outputRoot)
Assert-Equal -Label "axc lock exit code" -Actual ([int] $lock.ExitCode) -Expected 0

$interpreter = Invoke-Process -FilePath $axcBinary -Arguments @("run", $outputRoot)

$buildRoot = Join-Path $outputRoot "build"
$build = Invoke-Process `
    -FilePath $axcBinary `
    -Arguments @("build", $outputRoot, "--out-dir", $buildRoot, "--json") `
    -Environment @{
        AX_LLVM_AOT_LINK = "1"
        AX_LLVM_CLANG = $clang
    }
Assert-Equal -Label "axc build exit code" -Actual ([int] $build.ExitCode) -Expected 0

try {
    $manifest = $build.Stdout | ConvertFrom-Json
} catch {
    Write-Error "axc build did not produce valid manifest JSON.`nstdout:`n$($build.Stdout)`nstderr:`n$($build.Stderr)"
}

Assert-Equal -Label "manifest schema_version" -Actual ([int] $manifest.schema_version) -Expected 10
Assert-Equal -Label "aot readiness schema_version" -Actual ([int] $manifest.aot_readiness.schema_version) -Expected 3
Assert-Equal -Label "aot_supported" -Actual ([bool] $manifest.aot_supported) -Expected $true
Assert-Equal -Label "backend status" -Actual ([string] $manifest.backend.status) -Expected "built"
Assert-Equal -Label "package graph ready" -Actual ([bool] $manifest.package_graph_readiness.aot_ready) -Expected $true

$features = @($manifest.aot_readiness.required_backend_features | ForEach-Object { [string] $_ })
foreach ($feature in @("generic_functions", "generic_structs", "generic_type_instances", "local_path_packages")) {
    if (-not $features.Contains($feature)) {
        Write-Error "required_backend_features expected to contain '$feature' but found: $($features -join ', ')"
    }
}

$executableArtifact = [string] $manifest.artifacts.executable
if ([string]::IsNullOrWhiteSpace($executableArtifact)) {
    Write-Error "build manifest did not include artifacts.executable"
}

$executablePath = Join-Path $buildRoot $executableArtifact
if (-not (Test-Path $executablePath)) {
    Write-Error "native executable was not produced: $executablePath"
}

$executable = Invoke-Process -FilePath $executablePath
Assert-Equal -Label "native exit code" -Actual ([int] $executable.ExitCode) -Expected ([int] $interpreter.ExitCode)
Assert-Equal -Label "native stdout" -Actual (Normalize-Text $executable.Stdout) -Expected (Normalize-Text $interpreter.Stdout)
Assert-Equal -Label "native stderr" -Actual (Normalize-Text $executable.Stderr) -Expected (Normalize-Text $interpreter.Stderr)

Write-Host "AOT package generics smoke passed at $outputRoot"
