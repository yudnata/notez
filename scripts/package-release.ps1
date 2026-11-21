param(
    [string]$ExecutablePath = (Join-Path $PSScriptRoot '..\target\release\notvas.exe'),
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '..\dist')
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot
$executable = (Resolve-Path -LiteralPath $ExecutablePath).Path
$version = [regex]::Match([IO.File]::ReadAllText((Join-Path $projectRoot 'Cargo.toml')), '(?m)^version = "([^"]+)"').Groups[1].Value
if (-not $version) { throw 'Versi tidak ditemukan di Cargo.toml.' }
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
$packageName = "notvas-$version-windows-x64"
$folder = Join-Path $outputPath $packageName
$archive = Join-Path $outputPath ($packageName + '.zip')
if ((Test-Path -LiteralPath $folder) -or (Test-Path -LiteralPath $archive)) {
    throw "Paket sudah ada. Pilih -OutputDirectory lain: $outputPath"
}
$required = @('LICENSE', 'NOTICE.md', 'THIRD_PARTY_NOTICES.md', 'README.md', 'PRIVACY.md', 'docs/licenses/THIRD_PARTY_LICENSES.txt', 'docs/licenses/dependencies.json', 'assets/fonts/OFL-PatrickHand.txt', 'docs/USAGE.md')
foreach ($relative in $required) {
    if (-not (Test-Path -LiteralPath (Join-Path $projectRoot $relative))) { throw "Berkas paket belum ada: $relative" }
}
New-Item -ItemType Directory -Force $folder | Out-Null
Copy-Item -LiteralPath $executable -Destination (Join-Path $folder 'Notvas.exe')
foreach ($relative in $required) {
    $destination = Join-Path $folder $relative
    New-Item -ItemType Directory -Force (Split-Path $destination) | Out-Null
    Copy-Item -LiteralPath (Join-Path $projectRoot $relative) -Destination $destination
}
$licenses = Join-Path $folder 'docs/licenses/upstream'
Copy-Item -LiteralPath (Join-Path $projectRoot 'docs/licenses/upstream') -Destination $licenses -Recurse
Compress-Archive -LiteralPath $folder -DestinationPath $archive
Write-Output $archive
