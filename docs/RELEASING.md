# Membuat paket Windows

Source terbaru dan EXE lama dapat memiliki fitur berbeda. Buat paket dari build yang sudah diperiksa. Versi package belum menyatakan bahwa release dengan versi tersebut sudah diterbitkan.

## Periksa source dan lisensi

Jalankan formatter, tes, dan build sesuai [panduan development](DEVELOPMENT.md). Perbarui versi di `Cargo.toml`, `Cargo.lock`, serta [CHANGELOG.md](../CHANGELOG.md) bila membuat versi baru.

```powershell
.\scripts\update-third-party-notices.ps1
```

Inventaris yang disertakan sekarang ditujukan untuk `x86_64-pc-windows-gnu`, dengan dependensi normal dan build. Untuk target lain, jalankan generator memakai `-Target` yang sesuai dan tinjau perubahan lisensinya. Salinan upstream yang tidak cocok dengan commit crate harus diperbarui sebelum distribusi.

Patrick Hand tetap OFL-1.1. Font default egui memiliki pemberitahuan terpisah; lisensinya ikut dalam bundle. Font Windows seperti Segoe UI tidak disalin dari sistem ke paket ini.

## Build

```powershell
.\run.ps1 -Release -BuildOnly
```

Hasilnya ada di `target/release/notvas.exe`. Untuk memperbarui paket portable lokal di root, tutup release yang sedang berjalan lalu salin hasil build tersebut ke `Notvas.exe`. File EXE dan hasil distribusi diabaikan Git.

## Paket portable

```powershell
.\scripts\package-release.ps1
```

Script menyalin EXE, panduan penggunaan, dokumen privasi, lisensi MIT, atribusi, inventaris, teks lisensi dependensi, dan lisensi Patrick Hand ke `dist/notvas-<versi>-windows-x64`, lalu membuat ZIP. Script menolak menimpa paket yang sudah ada. `-ExecutablePath` dan `-OutputDirectory` dapat dipakai untuk memeriksa build lain di folder terpisah.

Sertakan pemberitahuan lisensi yang diperlukan saat mendistribusikan EXE. Lihat [NOTICE.md](../NOTICE.md).

## Periksa paket

Buka EXE dari hasil ekstraksi di folder kosong dan gunakan workspace demo. Periksa navigasi, pencarian, gambar, ekspor, autosave, backup, dan buka ulang data. Uji keyboard, perubahan ukuran jendela, serta layar dengan skala Windows berbeda. Pastikan paket tidak memuat workspace, log pribadi, `.tools`, atau cache build.

Periksa bahwa ekspor SVG dengan font tertanam memuat metadata lisensi Patrick Hand. Buka PNG/SVG/HTML hasil ekspor dan pastikan isinya benar. Buat SHA-256 ZIP untuk catatan rilis:

```powershell
Get-FileHash .\dist\notvas-<versi>-windows-x64.zip -Algorithm SHA256
```

Publikasikan paket dan catatan versi setelah pemeriksaan selesai. Workflow CI dalam repository hanya memeriksa source; tidak membuat atau memublikasikan release secara otomatis.
