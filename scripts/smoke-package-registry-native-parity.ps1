param(
    [string] $OutputDir = "target\package-registry-native-parity",
    [string] $Registry = "registry"
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

    Write-Error "clang was not found. Install LLVM clang or set AX_LLVM_CLANG before running package native parity."
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

function Assert-Contains {
    param(
        [string] $Label,
        [string[]] $Values,
        [string] $Expected
    )

    if (-not $Values.Contains($Expected)) {
        Write-Error "$Label expected to contain '$Expected' but found: $($Values -join ', ')"
    }
}

$outputRoot = Resolve-RepoPath -Path $OutputDir
$registryRoot = Resolve-RepoPath -Path $Registry

if (Test-Path $outputRoot) {
    Remove-Item -LiteralPath $outputRoot -Recurse -Force
}
New-Item -ItemType Directory -Force -Path (Join-Path $outputRoot "src") | Out-Null

$manifestText = @'
manifest_version = 1

[package]
name = "registry_stable_pure_ax_native_parity"
entry = "src/main.ax"

[dependencies]
generic_tools = { registry = "ax", version = "0.1.0" }
json_tools = { registry = "ax", version = "0.1.0" }
'@

$sourceText = @'
import generic_tools.box;
import json_tools.encode;

fn main() -> i32 {
    let boxed: generic_tools.box.Box<i32> = generic_tools.box.wrap(9);
    let direct: i32 = boxed.get();
    let roundtrip: i32 = generic_tools.box.unwrap(boxed);
    let score: i32 = direct + roundtrip;
    let value: string = json_tools.encode.object3(
        json_tools.encode.field_string("service", "ax-pkg"),
        json_tools.encode.field_i32("score", score),
        json_tools.encode.field_bool("native", true)
    );
    println(value);
    return 0;
}
'@

Write-Utf8NoBom -Path (Join-Path $outputRoot "AX.toml") -Text $manifestText
Write-Utf8NoBom -Path (Join-Path $outputRoot "src\main.ax") -Text $sourceText

$axcBinary = Ensure-AxcBinary
$clang = Resolve-Clang

& $axcBinary pkg install $outputRoot --registry $registryRoot | Out-Null
Assert-Equal -Label "axc pkg install exit code" -Actual $LASTEXITCODE -Expected 0

$check = Invoke-Process -FilePath $axcBinary -Arguments @("check", $outputRoot)
Assert-Equal -Label "axc check exit code" -Actual ([int] $check.ExitCode) -Expected 0

$interpreter = Invoke-Process -FilePath $axcBinary -Arguments @("run", $outputRoot)
Assert-Equal -Label "axc run exit code" -Actual ([int] $interpreter.ExitCode) -Expected 0

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
Assert-Equal -Label "user_code_valid" -Actual ([bool] $manifest.user_code_valid) -Expected $true
Assert-Equal -Label "interpreter_supported" -Actual ([bool] $manifest.interpreter_supported) -Expected $true
Assert-Equal -Label "aot_supported" -Actual ([bool] $manifest.aot_supported) -Expected $true
Assert-Equal -Label "backend status" -Actual ([string] $manifest.backend.status) -Expected "built"

$registryPackages = @($manifest.registry_packages)
Assert-Equal -Label "registry package count" -Actual $registryPackages.Count -Expected 2
foreach ($package in $registryPackages) {
    Assert-Equal -Label "registry package maturity $($package.package)" -Actual ([string] $package.maturity) -Expected "stable_pure_ax"
}

$features = @($manifest.aot_readiness.required_backend_features | ForEach-Object { [string] $_ })
foreach ($feature in @(
    "registry_packages",
    "registry_package_stable_pure_ax",
    "generic_functions",
    "generic_impls",
    "generic_structs",
    "generic_type_instances",
    "impl_methods"
)) {
    Assert-Contains -Label "required_backend_features" -Values $features -Expected $feature
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

Write-Host "Package registry native parity smoke passed for stable_pure_ax json_tools + generic_tools generic/method coverage at $outputRoot"
