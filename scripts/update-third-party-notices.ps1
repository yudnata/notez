param([string]$Target = 'x86_64-pc-windows-gnu')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot
Set-Location $projectRoot
$portable = Join-Path $projectRoot '.tools\cargo\bin\cargo.exe'
if (Test-Path $portable) {
    $env:RUSTUP_HOME = Join-Path $projectRoot '.tools\rustup'
    $env:CARGO_HOME = Join-Path $projectRoot '.tools\cargo'
    $cargo = $portable
} else { $cargo = 'cargo' }
$metadataText = & $cargo metadata --locked --offline --format-version 1 --filter-platform $Target
if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata gagal. Jalankan cargo fetch --locked terlebih dahulu.' }
$metadata = $metadataText | ConvertFrom-Json
$tree = & $cargo tree --locked --offline --target $Target --edges normal,build --prefix none --format '{p}'
if ($LASTEXITCODE -ne 0) { throw 'Cargo tree gagal.' }
$ids = @{}
foreach ($line in $tree) {
    if ($line -match '^([A-Za-z0-9_-]+) v([^ ]+)') { $ids[$Matches[1] + '@' + $Matches[2]] = $true }
}
$packages = @($metadata.packages | Where-Object { $_.name -ne 'notvas' -and $ids.ContainsKey($_.name + '@' + $_.version) } | Sort-Object name,version)
$fallback = @{
    accesskit = 'accesskit-a'; accesskit_consumer = 'accesskit-b'; accesskit_windows = 'accesskit-b'; accesskit_winit = 'accesskit-b'
    'clipboard-win' = 'clipboard-win'; ecolor = 'egui'; eframe = 'egui'; egui = 'egui'; 'egui-winit' = 'egui'; egui_glow = 'egui'
    emath = 'egui'; epaint = 'egui'; epaint_default_fonts = 'egui'; gl_generator = 'gl-generator'; khronos_api = 'khronos-api'
    profiling = 'profiling'; 'zune-core' = 'zune-core'; 'zune-jpeg' = 'zune-jpeg'
}
$sources = Get-Content -Raw 'docs/licenses/upstream/sources.json' | ConvertFrom-Json
$bundle = [Text.StringBuilder]::new()
[void]$bundle.AppendLine("Notvas third-party license texts`nTarget: $Target`nIncludes normal and build dependencies resolved from Cargo.lock.`n")
$index = [Collections.Generic.List[string]]::new()
$index.Add('# Third-party notices')
$index.Add('')
$index.Add("Dependency inventory for target ``$Target``. Includes build-time dependencies; this is not a list of separately shipped DLLs.")
$index.Add('')
$index.Add('Generated from `Cargo.lock` with `scripts/update-third-party-notices.ps1`. Full license texts and upstream notices are in [THIRD_PARTY_LICENSES.txt](docs/licenses/THIRD_PARTY_LICENSES.txt). Bundled fonts are described in [NOTICE.md](NOTICE.md).')
$index.Add('')
$index.Add('| Package | Version | Declared license | Source |')
$index.Add('| --- | --- | --- | --- |')
$inventory = @()
foreach ($package in $packages) {
    if (-not $package.license) { throw "Lisensi tidak dideklarasikan: $($package.name)" }
    $root = Split-Path $package.manifest_path
    $files = @(Get-ChildItem -LiteralPath $root -Recurse -File | Where-Object Name -Match '(licen[cs]e|copying|copyright|notice|^ofl|^ufl)' | ForEach-Object {
        @{ name = $_.FullName.Substring($root.Length + 1).Replace('\','/'); text = [IO.File]::ReadAllText($_.FullName) }
    })
    if ($package.name -eq 'epaint_default_fonts') {
        $file = Join-Path $root 'fonts\Hack-Regular.txt'
        $files += @{ name = 'fonts/Hack-Regular.txt'; text = [IO.File]::ReadAllText($file) }
    }
    if ($fallback.ContainsKey($package.name) -and ($files.Count -eq 0 -or $package.name -eq 'epaint_default_fonts')) {
        $key = $fallback[$package.name]
        $vcs = Get-Content -Raw (Join-Path $root '.cargo_vcs_info.json') | ConvertFrom-Json
        foreach ($source in $sources | Where-Object key -eq $key) {
            if (-not $source.url.Contains($vcs.git.sha1)) { throw "Commit lisensi upstream perlu diperbarui: $($package.name)" }
            $file = Join-Path $projectRoot ('docs/licenses/upstream/' + $key + '/' + $source.path)
            $files += @{ name = $source.url; text = [IO.File]::ReadAllText($file) }
        }
    }
    if ($package.name -eq 'khronos_api') {
        foreach ($file in Get-ChildItem -LiteralPath $root -Recurse -Filter '*.xml') {
            $text = [IO.File]::ReadAllText($file.FullName)
            if ($text -match '(?s)<comment>(.*?)</comment>') {
                $files += @{ name = $file.Name + ' registry notice'; text = $Matches[1].Trim() }
            }
        }
    }
    if ($files.Count -eq 0) { throw "Teks lisensi belum tersedia: $($package.name) $($package.version)" }
    $repository = if ($package.repository) { $package.repository } else { "https://crates.io/crates/$($package.name)/$($package.version)" }
    $index.Add("| $($package.name) | $($package.version) | $($package.license) | [upstream]($repository) |")
    [void]$bundle.AppendLine("`n========================================================================`n$($package.name) $($package.version)`nLicense: $($package.license)`nSource: $repository")
    foreach ($file in $files) {
        [void]$bundle.AppendLine("`n--- $($file.name) ---`n")
        [void]$bundle.AppendLine($file.text.TrimEnd())
    }
    $inventory += [ordered]@{ name = $package.name; version = $package.version; license = $package.license; repository = $repository }
}
$utf8 = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllText((Join-Path $projectRoot 'THIRD_PARTY_NOTICES.md'), ($index -join "`n") + "`n", $utf8)
[IO.File]::WriteAllText((Join-Path $projectRoot 'docs/licenses/THIRD_PARTY_LICENSES.txt'), $bundle.ToString().Replace("`r`n", "`n"), $utf8)
[IO.File]::WriteAllText((Join-Path $projectRoot 'docs/licenses/dependencies.json'), ($inventory | ConvertTo-Json -Depth 4) + "`n", $utf8)
Write-Output "$($packages.Count) dependensi dicatat untuk $Target."
