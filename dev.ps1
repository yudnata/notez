param([switch]$Watch)
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$portableRustup = Join-Path $PSScriptRoot '.tools\rustup'
$portableCargoHome = Join-Path $PSScriptRoot '.tools\cargo'
$portableCargo = Join-Path $portableCargoHome 'bin\cargo.exe'
$cargoExe = if (Test-Path $portableCargo) { $portableCargo } else { 'cargo' }
$nativeTools = Join-Path $portableRustup 'toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained'
if (Test-Path $portableCargo) { $env:RUSTUP_HOME = $portableRustup; $env:CARGO_HOME = $portableCargoHome }
if (Test-Path $nativeTools) {
    $env:PATH = "$(Join-Path $env:CARGO_HOME 'bin');$(Join-Path $PSScriptRoot '.tools\mingw64\bin');$nativeTools;$env:PATH"
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $nativeTools 'x86_64-w64-mingw32-gcc.exe'
}
$env:CARGO_INCREMENTAL = '1'
$devData = Join-Path $PSScriptRoot '.dev-data'
$restartFile = Join-Path $devData 'restart.request'
$devExe = Join-Path $PSScriptRoot 'target\debug\notvas.exe'
$devProcess = $null

function SourceStamp {
    $files = @(Get-ChildItem src,assets -Recurse -File)
    $files += Get-Item Cargo.toml,build.rs
    return ($files | Sort-Object FullName | ForEach-Object { $_.FullName + ':' + $_.LastWriteTimeUtc.Ticks + ':' + $_.Length }) -join '|'
}

function RebuildAndRun {
    if ($null -ne $script:devProcess -and -not $script:devProcess.HasExited) {
        [System.IO.File]::WriteAllText($restartFile, 'restart')
        if (-not $script:devProcess.WaitForExit(10000)) {
            throw 'Tutup jendela Notvas development terlebih dahulu. Proses tidak dihentikan paksa agar perubahan tetap aman.'
        }
    }
    & $cargoExe build --bin notvas
    if ($LASTEXITCODE -eq 0) {
        New-Item -ItemType Directory -Force $devData | Out-Null
        if (Test-Path -LiteralPath $restartFile) { Remove-Item -LiteralPath $restartFile }
        $script:devProcess = Start-Process -FilePath $devExe -ArgumentList '--data-dir', ('"' + $devData + '"'), '--restart-file', ('"' + $restartFile + '"') -RedirectStandardError (Join-Path $devData 'app-stderr.log') -PassThru
        Write-Host 'Notvas development aktif. Data: .dev-data'
    } else { Write-Host 'Build gagal. Perbaiki source; mode watch akan mencoba lagi saat file berubah.' }
}

$stamp = SourceStamp
RebuildAndRun
if ($Watch) {
    Write-Host 'Menunggu perubahan source/aset. Ctrl+C untuk menghentikan watcher.'
    while ($true) {
        Start-Sleep -Milliseconds 600
        $current = SourceStamp
        if ($current -ne $stamp) {
            $stamp = $current
            RebuildAndRun
        }
    }
}
