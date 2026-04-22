param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $CargoArgs
)

$ErrorActionPreference = "Stop"

if (-not $CargoArgs -or $CargoArgs.Count -eq 0) {
    Write-Error "usage: .\\scripts\\cargo-gnu.ps1 <cargo-args...>"
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$manifestPath = Join-Path $repoRoot "Cargo.toml"

$rustupHome = if ($env:RUSTUP_HOME) {
    $env:RUSTUP_HOME
} else {
    Join-Path $env:USERPROFILE ".rustup"
}

$gnuBin = Join-Path $rustupHome "toolchains\\stable-x86_64-pc-windows-gnu\\lib\\rustlib\\x86_64-pc-windows-gnu\\bin"
$gnuSelf = Join-Path $gnuBin "self-contained"
$gnuLinker = Join-Path $gnuSelf "x86_64-w64-mingw32-gcc.exe"

if (-not (Test-Path $gnuLinker)) {
    Write-Error @"
AX could not find the Rust GNU linker wrapper:
  $gnuLinker

Install it with:
  rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal -c rustfmt
"@
}

$env:PATH = "$gnuSelf;$gnuBin;$env:PATH"
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = $gnuLinker

$cargoCommand = $CargoArgs[0]
$cargoRest = if ($CargoArgs.Count -gt 1) {
    $CargoArgs[1..($CargoArgs.Count - 1)]
} else {
    @()
}

& cargo +stable-x86_64-pc-windows-gnu $cargoCommand --manifest-path $manifestPath @cargoRest
exit $LASTEXITCODE
