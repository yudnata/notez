param([string]$OutputDirectory = (Join-Path $PSScriptRoot '..\test-data\docs-demo'))
$ErrorActionPreference = 'Stop'
$outputPath = [IO.Path]::GetFullPath($OutputDirectory)
foreach ($mode in 'canvas', 'notes', 'checklist') {
    if (Test-Path -LiteralPath (Join-Path $outputPath "$mode\workspace.notvas")) {
        throw "Demo sudah ada. Pilih -OutputDirectory lain: $outputPath"
    }
}

$colors = @(@(248,225,151), @(247,213,184), @(240,203,213), @(221,212,240), @(199,220,238), @(210,225,199))
function New-Document($Id, $Kind, $Title, $Text, $ColorIndex) {
    @{
        id = $Id; kind = $Kind; title = $Title
        body = @{ text = $Text; styles = @(); images = @() }
        spacing = 1.5; tasks = @(); note_color = $colors[$ColorIndex]; note_position = $null
    }
}
function New-Element($Id, $Kind, $X, $Y, $Width, $Height, $Text, $Fill) {
    @{
        id = $Id; kind = $Kind; pos = @($X,$Y); size = @($Width,$Height); points = @(); text = $Text
        style = @{ stroke = @(0,0,0,255); fill = $Fill; width = 2.0; font_size = 24.0; rough = $true }
        route = 'Straight'; bend = $null; start_binding = $null; end_binding = $null; image = $null; locked = $false
    }
}
$canvas = New-Document 1 'Canvas' 'Dari ide ke rencana' '' 0
$elements = @(
    (New-Element 1 'Text' 65 45 650 55 'Mulai dari satu ide' @(255,255,255,0)),
    (New-Element 2 'Rectangle' 65 155 185 100 'Tulis masalah' @(195,222,246,255)),
    (New-Element 3 'Rectangle' 345 155 185 100 'Coba solusi' @(204,233,200,255)),
    (New-Element 4 'Rectangle' 625 155 185 100 'Catat hasil' @(239,231,248,255)),
    (New-Element 5 'Sticky' 65 350 205 150 "Apa yang ingin`nkita perbaiki?" @(255,228,129,255)),
    (New-Element 6 'Sticky' 345 350 205 150 "Buat sketsa`nsebelum mulai" @(247,201,210,255)),
    (New-Element 7 'Text' 625 370 190 100 "Satu langkah`nsetiap hari." @(255,255,255,0))
)
foreach ($pair in @(@(2,3), @(3,4))) {
    $arrow = New-Element ($elements.Count + 1) 'Arrow' 0 0 0 0 '' @(255,255,255,0)
    $arrow.points = @(@(250,205), @(345,205))
    $arrow.start_binding = @{ target = $pair[0] }; $arrow.end_binding = @{ target = $pair[1] }
    $elements += $arrow
}
$canvas.canvas = @{ next_id = 20; elements = $elements; paper = 'Grid'; pan = @(0,0); zoom = 0.85 }
$notes = @(
    (New-Document 2 'Note' 'Catatan proyek' "Tujuan minggu ini`n`nMerapikan ide dan memilih satu hal yang bisa selesai lebih dulu.`n`nYang perlu dicoba`n`nBuat sketsa alur di Canvas, tulis hasil percobaan, lalu pindahkan langkah berikutnya ke Checklist." 0),
    (New-Document 3 'Note' 'Ide akhir pekan' "Jalan pagi, baca satu bab, dan coba resep baru.`n`nSisakan waktu untuk istirahat." 4),
    (New-Document 4 'Note' 'Bahan bacaan' "Desain antarmuka`nStruktur data`nCatatan dari buku yang sedang dibaca" 5),
    (New-Document 5 'Note' 'Hal yang dipelajari' "Tulis masalahnya dengan jelas sebelum mencari solusi.`n`nSketsa kecil sering membantu melihat bagian yang terlewat." 2),
    (New-Document 6 'Note' 'Rencana minggu depan' "Pilih tiga prioritas yang masuk akal.`n`nSelesaikan pekerjaan yang tertunda sebelum menambah yang baru." 3),
    (New-Document 7 'Note' 'Tempat menyimpan ide' "Judul tetap terbaca di papan.`n`nKlik kartu untuk membuka catatan dan melanjutkan tulisan." 1)
)
$checklists = @(
    (New-Document 8 'Checklist' 'Persiapan proyek' '' 5),
    (New-Document 9 'Checklist' 'Belanja mingguan' '' 0),
    (New-Document 10 'Checklist' 'Sebelum berangkat' '' 4),
    (New-Document 11 'Checklist' 'Rutinitas belajar' '' 2)
)
$taskId = 100
$taskLists = @(
    @('Tulis ruang lingkup', 'Buat sketsa alur', 'Siapkan contoh data', 'Tinjau hasil percobaan'),
    @('Beras dan telur', 'Sayur untuk tiga hari', 'Buah', 'Kopi'),
    @('Isi botol minum', 'Bawa buku catatan', 'Periksa kunci rumah'),
    @('Baca satu bab', 'Tulis ringkasan', 'Coba satu latihan')
)
for ($i = 0; $i -lt $checklists.Count; $i++) {
    for ($j = 0; $j -lt $taskLists[$i].Count; $j++) {
        $checklists[$i].tasks += @{ id = $taskId; text = $taskLists[$i][$j]; detail = ''; done = ($j -eq 0); column = 0 }
        $taskId++
    }
}
$documents = @($canvas) + $notes + $checklists
foreach ($mode in 'canvas', 'notes', 'checklist') {
    $firstKind = @{ canvas = 'Canvas'; notes = 'Note'; checklist = 'Checklist' }[$mode]
    $ordered = @($documents | Where-Object kind -eq $firstKind) + @($documents | Where-Object kind -ne $firstKind)
    $folder = Join-Path $outputPath $mode
    New-Item -ItemType Directory -Force $folder | Out-Null
    $json = @{ version = 1; next_id = 1000; documents = $ordered } | ConvertTo-Json -Depth 30
    [IO.File]::WriteAllText((Join-Path $folder 'workspace.notvas'), $json, [Text.UTF8Encoding]::new($false))
}
Write-Output "Workspace demo dibuat di $outputPath"
