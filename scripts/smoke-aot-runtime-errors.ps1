param(
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
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)

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

function Build-AxcIfNeeded {
    $axc = Resolve-AxcBinary
    if (-not [string]::IsNullOrWhiteSpace($axc)) {
        return $axc
    }

    Write-Host "Building axc for AOT runtime-error smoke..."
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
        Write-Error "failed to build axc for AOT runtime-error smoke`nstdout:`n$($build.Stdout)`nstderr:`n$($build.Stderr)"
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

    Write-Error "clang was not found. Install clang or set AX_LLVM_CLANG before running the AOT runtime-error smoke."
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

function Assert-StartsWith {
    param(
        [string] $Label,
        [string] $Actual,
        [string] $ExpectedPrefix
    )

    if (-not $Actual.StartsWith($ExpectedPrefix)) {
        Write-Error "$Label expected prefix '$ExpectedPrefix' but got '$Actual'."
    }
}

function Assert-SafeOutputRoot {
    param([string] $Path)

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $root = [System.IO.Path]::GetPathRoot($fullPath)
    if ($fullPath.TrimEnd('\', '/') -eq $root.TrimEnd('\', '/')) {
        Write-Error "Refusing to use filesystem root as AOT runtime-error smoke output root: $fullPath"
    }

    $leaf = Split-Path -Leaf $fullPath
    if (-not $leaf.StartsWith("ax-aot-runtime-error")) {
        Write-Error "OutputRoot leaf must start with 'ax-aot-runtime-error' because the script recreates it: $fullPath"
    }

    return $fullPath
}

function Format-Blockers {
    param($Blockers)

    if ($null -eq $Blockers) {
        return "(no blockers field)"
    }

    return ($Blockers | ConvertTo-Json -Depth 8)
}

$cases = @(
    @{
        Name = "add_overflow"
        ExpectedPrefix = "R0018:"
        Source = @'
fn main() -> i32 {
    let max: i32 = 2147483647;
    return max + 1;
}
'@
    },
    @{
        Name = "sub_overflow"
        ExpectedPrefix = "R0019:"
        Source = @'
fn main() -> i32 {
    let min: i32 = -2147483647 - 1;
    return min - 1;
}
'@
    },
    @{
        Name = "mul_overflow"
        ExpectedPrefix = "R0020:"
        Source = @'
fn main() -> i32 {
    let left: i32 = 50000;
    let right: i32 = 50000;
    return left * right;
}
'@
    },
    @{
        Name = "neg_overflow"
        ExpectedPrefix = "R0012:"
        Source = @'
fn main() -> i32 {
    let min: i32 = -2147483647 - 1;
    return -min;
}
'@
    },
    @{
        Name = "div_zero"
        ExpectedPrefix = "R0021:"
        Source = @'
fn main() -> i32 {
    let divisor: i32 = 0;
    return 8 / divisor;
}
'@
    },
    @{
        Name = "rem_zero"
        ExpectedPrefix = "R0021:"
        Source = @'
fn main() -> i32 {
    let divisor: i32 = 0;
    return 8 % divisor;
}
'@
    },
    @{
        Name = "div_overflow"
        ExpectedPrefix = "R0022:"
        Source = @'
fn main() -> i32 {
    let min: i32 = -2147483647 - 1;
    return min / -1;
}
'@
    },
    @{
        Name = "rem_overflow"
        ExpectedPrefix = "R0024:"
        Source = @'
fn main() -> i32 {
    let min: i32 = -2147483647 - 1;
    return min % -1;
}
'@
    },
    @{
        Name = "array_oob"
        ExpectedPrefix = "R0031:"
        Source = @'
fn main() -> i32 {
    let values: [i32; 2] = [1, 2];
    return values[2];
}
'@
    },
    @{
        Name = "slice_bound_oob"
        ExpectedPrefix = "R0032:"
        Source = @'
fn main() -> i32 {
    let values: [i32; 2] = [1, 2];
    let view: [i32] = values[0:3];
    return len(view);
}
'@
    },
    @{
        Name = "slice_order_invalid"
        ExpectedPrefix = "R0032:"
        Source = @'
fn main() -> i32 {
    let values: [i32; 2] = [1, 2];
    let view: [i32] = values[2:1];
    return len(view);
}
'@
    },
    @{
        Name = "argv_oob"
        ExpectedPrefix = "R0031:"
        Source = @'
fn main() -> i32 {
    return string_len(argv_get(0));
}
'@
    },
    @{
        Name = "env_missing"
        ExpectedPrefix = "R0053:"
        Source = @'
fn main() -> i32 {
    return string_len(env_get("AX_THIS_VARIABLE_SHOULD_NOT_EXIST_7A9F3D0C"));
}
'@
    }
)

if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
    $tempRoot = if (-not [string]::IsNullOrWhiteSpace($env:RUNNER_TEMP)) {
        [string] $env:RUNNER_TEMP
    } else {
        [System.IO.Path]::GetTempPath()
    }
    $OutputRoot = Join-Path $tempRoot "ax-aot-runtime-error-smoke"
} else {
    $OutputRoot = Resolve-RepoPath -Path $OutputRoot
}

$OutputRoot = Assert-SafeOutputRoot -Path $OutputRoot
if (Test-Path $OutputRoot) {
    Remove-Item -LiteralPath $OutputRoot -Recurse -Force
}
New-Item -ItemType Directory -Path $OutputRoot | Out-Null

$axc = Build-AxcIfNeeded
$clang = Resolve-Clang
Write-Host "Using axc for AOT runtime-error smoke: $axc"
Write-Host "Using clang for AOT runtime-error smoke: $clang"

$buildEnv = @{
    AX_LLVM_AOT_LINK = "1"
    AX_LLVM_CLANG = $clang
}

foreach ($case in $cases) {
    $caseName = [string] $case.Name
    $sourcePath = Join-Path $OutputRoot "$caseName.ax"
    $caseOutDir = Join-Path $OutputRoot $caseName
    [System.IO.File]::WriteAllText($sourcePath, [string] $case.Source, $utf8NoBom)

    $check = Invoke-Process -FilePath $axc -Arguments @("check", $sourcePath)
    Assert-Equal -Label "$caseName check exit code" -Actual ([int] $check.ExitCode) -Expected 0

    $build = Invoke-Process -FilePath $axc -Arguments @(
        "build",
        $sourcePath,
        "--out-dir",
        $caseOutDir,
        "--json"
    ) -Environment $buildEnv
    Assert-Equal -Label "$caseName build exit code" -Actual ([int] $build.ExitCode) -Expected 0

    try {
        $manifest = $build.Stdout | ConvertFrom-Json
    } catch {
        Write-Error "AOT runtime-error build stdout was not valid manifest JSON for $caseName.`nstdout:`n$($build.Stdout)`nstderr:`n$($build.Stderr)"
    }

    Assert-Equal -Label "$caseName manifest schema_version" -Actual ([int] $manifest.schema_version) -Expected 9
    Assert-Equal -Label "$caseName aot_readiness.schema_version" -Actual ([int] $manifest.aot_readiness.schema_version) -Expected 3
    Assert-Equal -Label "$caseName user_code_valid" -Actual ([bool] $manifest.user_code_valid) -Expected $true
    Assert-Equal -Label "$caseName interpreter_supported" -Actual ([bool] $manifest.interpreter_supported) -Expected $true
    Assert-Equal -Label "$caseName aot_supported" -Actual ([bool] $manifest.aot_supported) -Expected $true
    Assert-Equal -Label "$caseName backend.kind" -Actual ([string] $manifest.backend.kind) -Expected "llvm-aot"
    Assert-Equal -Label "$caseName backend.status" -Actual ([string] $manifest.backend.status) -Expected "built"

    $executableArtifact = [string] $manifest.artifacts.executable
    if ([string]::IsNullOrWhiteSpace($executableArtifact)) {
        $blockers = Format-Blockers $manifest.aot_readiness.blockers
        Write-Error "AOT runtime-error build did not produce an executable for $caseName. Blockers:`n$blockers"
    }

    $executablePath = Join-Path $caseOutDir $executableArtifact
    if (-not (Test-Path $executablePath)) {
        Write-Error "AOT runtime-error executable is missing for $caseName`: $executablePath"
    }

    $executable = Invoke-Process -FilePath $executablePath
    Assert-Equal -Label "$caseName executable exit code" -Actual ([int] $executable.ExitCode) -Expected 1
    Assert-StartsWith -Label "$caseName executable stderr" -Actual ([string] $executable.Stderr) -ExpectedPrefix ([string] $case.ExpectedPrefix)

    Write-Host "AOT runtime-error passed: $caseName exit=$($executable.ExitCode) stderr=$($executable.Stderr.Trim())"
}

Write-Host "LLVM AOT runtime-error smoke passed for $($cases.Count) case(s)."
