$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot)
if (-not (Test-Path 'test-data\docs-demo\canvas\workspace.notvas')) { & "$PSScriptRoot\create-demo.ps1" }
$portable = Join-Path $PWD '.tools\cargo\bin\cargo.exe'
if (Test-Path $portable) {
    $env:RUSTUP_HOME = Join-Path $PWD '.tools\rustup'
    $env:CARGO_HOME = Join-Path $PWD '.tools\cargo'
    $native = Join-Path $env:RUSTUP_HOME 'toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained'
    $env:PATH = "$native;$(Join-Path $PWD '.tools\mingw64\bin');$env:PATH"
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $native 'x86_64-w64-mingw32-gcc.exe'
    $cargo = $portable
} else { $cargo = 'cargo' }
& $cargo test --locked render_documentation_screenshots -- --ignored --nocapture
exit $LASTEXITCODE
