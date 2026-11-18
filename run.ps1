param([switch]$Release, [switch]$BuildOnly, [switch]$Test)
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$portableCargo = Join-Path $PSScriptRoot '.tools\cargo\bin\cargo.exe'
if (Test-Path $portableCargo) {
    $env:RUSTUP_HOME = Join-Path $PSScriptRoot '.tools\rustup'
    $env:CARGO_HOME = Join-Path $PSScriptRoot '.tools\cargo'
    $env:PATH = "$(Join-Path $PSScriptRoot '.tools\cargo\bin');$env:PATH"
    $nativeTools = Join-Path $PSScriptRoot '.tools\rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained'
    if (Test-Path $nativeTools) {
        $env:PATH = "$nativeTools;$env:PATH"
        $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $nativeTools 'x86_64-w64-mingw32-gcc.exe'
    }
    $binutils = Join-Path $PSScriptRoot '.tools\mingw64\bin'
    if (Test-Path $binutils) { $env:PATH = "$binutils;$env:PATH" }
    $cargoExe = $portableCargo
} else { $cargoExe = 'cargo' }
$arguments = @('run')
if ($BuildOnly) { $arguments = @('build') }
if ($Test) { $arguments = @('test') }
if ($Release) { $arguments += '--release' }
& $cargoExe @arguments
exit $LASTEXITCODE
