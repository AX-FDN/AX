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

    $process = $null
    for ($attempt = 1; $attempt -le 5; $attempt += 1) {
        try {
            $process = [System.Diagnostics.Process]::Start($startInfo)
            break
        } catch [System.ComponentModel.Win32Exception] {
            $message = $_.Exception.Message
            if ($attempt -eq 5 -or $message -notmatch "being used by another process") {
                throw
            }
            Start-Sleep -Milliseconds (200 * $attempt)
        }
    }

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

$axcBinary = Ensure-AxcBinary
$clang = Resolve-Clang

function Invoke-LocalPackageParity {
    param(
        [string] $Name,
        [string] $PackageName,
        [string] $ModuleFile,
        [string] $ModuleText,
        [string] $MainText,
        [string[]] $ExpectedFeatures
    )

    $fixtureRoot = Join-Path $outputRoot $Name
    $packageRoot = Join-Path $fixtureRoot "packages\$PackageName"
    New-Item -ItemType Directory -Force -Path (Join-Path $fixtureRoot "src") | Out-Null
    New-Item -ItemType Directory -Force -Path (Join-Path $packageRoot "src") | Out-Null

    Write-Utf8NoBom -Path (Join-Path $fixtureRoot "AX.toml") -Text @"
manifest_version = 1

[package]
name = "$Name"
entry = "src/main.ax"

[dependencies]
$PackageName = { path = "packages/$PackageName" }
"@

    Write-Utf8NoBom -Path (Join-Path $packageRoot "AX.toml") -Text @"
manifest_version = 1

[package]
name = "$PackageName"
sources = ["src"]
"@

    Write-Utf8NoBom -Path (Join-Path $packageRoot "src\$ModuleFile") -Text $ModuleText
    Write-Utf8NoBom -Path (Join-Path $fixtureRoot "src\main.ax") -Text $MainText

    $check = Invoke-Process -FilePath $axcBinary -Arguments @("check", $fixtureRoot)
    Assert-Equal -Label "$Name axc check exit code" -Actual ([int] $check.ExitCode) -Expected 0

    $lock = Invoke-Process -FilePath $axcBinary -Arguments @("lock", $fixtureRoot)
    Assert-Equal -Label "$Name axc lock exit code" -Actual ([int] $lock.ExitCode) -Expected 0

    $interpreter = Invoke-Process -FilePath $axcBinary -Arguments @("run", $fixtureRoot)

    $buildRoot = Join-Path $fixtureRoot "build"
    $build = Invoke-Process `
        -FilePath $axcBinary `
        -Arguments @("build", $fixtureRoot, "--out-dir", $buildRoot, "--json") `
        -Environment @{
            AX_LLVM_AOT_LINK = "1"
            AX_LLVM_CLANG = $clang
        }
    Assert-Equal -Label "$Name axc build exit code" -Actual ([int] $build.ExitCode) -Expected 0

    try {
        $manifest = $build.Stdout | ConvertFrom-Json
    } catch {
        Write-Error "$Name axc build did not produce valid manifest JSON.`nstdout:`n$($build.Stdout)`nstderr:`n$($build.Stderr)"
    }

    Assert-Equal -Label "$Name manifest schema_version" -Actual ([int] $manifest.schema_version) -Expected 10
    Assert-Equal -Label "$Name aot readiness schema_version" -Actual ([int] $manifest.aot_readiness.schema_version) -Expected 3
    Assert-Equal -Label "$Name aot_supported" -Actual ([bool] $manifest.aot_supported) -Expected $true
    Assert-Equal -Label "$Name backend status" -Actual ([string] $manifest.backend.status) -Expected "built"
    Assert-Equal -Label "$Name package graph ready" -Actual ([bool] $manifest.package_graph_readiness.aot_ready) -Expected $true

    $features = @($manifest.aot_readiness.required_backend_features | ForEach-Object { [string] $_ })
    foreach ($feature in $ExpectedFeatures) {
        if (-not $features.Contains($feature)) {
            Write-Error "$Name required_backend_features expected to contain '$feature' but found: $($features -join ', ')"
        }
    }

    $executableArtifact = [string] $manifest.artifacts.executable
    if ([string]::IsNullOrWhiteSpace($executableArtifact)) {
        Write-Error "$Name build manifest did not include artifacts.executable"
    }

    $executablePath = Join-Path $buildRoot $executableArtifact
    if (-not (Test-Path $executablePath)) {
        Write-Error "$Name native executable was not produced: $executablePath"
    }

    $executable = Invoke-Process -FilePath $executablePath
    Assert-Equal -Label "$Name native exit code" -Actual ([int] $executable.ExitCode) -Expected ([int] $interpreter.ExitCode)
    Assert-Equal -Label "$Name native stdout" -Actual (Normalize-Text $executable.Stdout) -Expected (Normalize-Text $interpreter.Stdout)
    Assert-Equal -Label "$Name native stderr" -Actual (Normalize-Text $executable.Stderr) -Expected (Normalize-Text $interpreter.Stderr)

    Write-Host "AOT local package parity passed: $Name"
}

Invoke-LocalPackageParity `
    -Name "aot_package_generics_smoke" `
    -PackageName "generic_helpers" `
    -ModuleFile "core.ax" `
    -ModuleText @'
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
'@ `
    -MainText @'
import generic_helpers.core;

fn main() -> i32 {
    let left: i32 = generic_helpers.core.identity(4);
    let boxed: generic_helpers.core.Box<i32> = generic_helpers.core.Box { value: 6 };
    let right: i32 = generic_helpers.core.box_value(boxed);
    return left + right;
}
'@ `
    -ExpectedFeatures @("generic_functions", "generic_structs", "generic_type_instances", "local_path_packages")

Invoke-LocalPackageParity `
    -Name "aot_package_methods_smoke" `
    -PackageName "method_helpers" `
    -ModuleFile "score.ax" `
    -ModuleText @'
module method_helpers.score;

struct Score {
    value: i32,
}

impl Score {
    fn add(self: Score, amount: i32) -> Score {
        return Score { value: self.value + amount };
    }

    fn clamp(self: Score, max: i32) -> Score {
        if (self.value > max) {
            return Score { value: max };
        }
        return self;
    }

    fn get(self: Score) -> i32 {
        return self.value;
    }
}

fn new(value: i32) -> Score {
    return Score { value: value };
}
'@ `
    -MainText @'
import method_helpers.score;

fn main() -> i32 {
    let score: method_helpers.score.Score = method_helpers.score.new(7);
    let raised: method_helpers.score.Score = score.add(5);
    let adjusted: method_helpers.score.Score = raised.clamp(10);
    return adjusted.get();
}
'@ `
    -ExpectedFeatures @("impl_methods", "local_path_packages", "module_imports", "structs")

Write-Host "AOT package generics/methods smoke passed at $outputRoot"
