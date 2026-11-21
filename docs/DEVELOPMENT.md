# Development

Notvas memakai Rust edition 2024, egui/eframe, dan renderer OpenGL. Pengembangan saat ini dilakukan di Windows. Dependensi dipatok melalui `Cargo.lock`.

## Menjalankan source

Pasang Rust stable dan linker Windows untuk toolchain yang digunakan. Pada MSVC, gunakan C++ Build Tools; pada GNU, gunakan toolchain dan linker MinGW yang sesuai. Folder `.tools` membantu setup lokal dan tidak masuk Git.

```powershell
.\run.ps1
```

Script memilih Cargo portable dari `.tools` bila tersedia, atau `cargo` dari PATH. Script tidak mengubah PATH Windows secara permanen.

## Build dan restart otomatis

Klik `Notvas-dev.cmd`, atau jalankan:

```powershell
.\dev.ps1 -Watch
```

Watcher memantau `src`, `assets`, `Cargo.toml`, dan `build.rs`. Ketika berubah, proses aplikasi diminta menyelesaikan autosave, lalu build dan restart. Jika aplikasi tidak bisa menutup karena penyimpanan gagal, watcher tidak menghentikannya secara paksa. Tanpa `-Watch`, script hanya build dan membuka aplikasi sekali. Ctrl+C menghentikan watcher.

Data development ada di `.dev-data/workspace.notvas`; stderr aplikasi ada di `.dev-data/app-stderr.log`. Rust tetap memerlukan kompilasi. Profil dev memakai incremental compilation tanpa debug symbols besar. Auto-restart tidak mempertahankan state proses; data yang tersimpan dimuat lagi.

## Pemeriksaan

```powershell
.\run.ps1 -Test
```

Jika menggunakan Cargo langsung:

```powershell
cargo fmt --all -- --check
cargo test --locked
cargo build --locked
```

Tes UI memakai event egui tanpa jendela Windows. Tes tersebut memeriksa posisi, pointer/keyboard, seleksi, format, zoom, penyimpanan, dan ekspor. Pemeriksaan native tetap diperlukan sebelum rilis. Catatan hasil lokal ada di [QA.md](../QA.md).

## Struktur source

| Lokasi | Isi |
| --- | --- |
| `src/main.rs` | State aplikasi, editor, autosave, dan alur dokumen |
| `src/header.rs` | Judul, menu, navigasi, dan toolbar |
| `src/canvas` | Model objek, geometri, jalur panah, penghapus, UI, dan ekspor |
| `src/notes_board.rs` | Papan kartu Notes dan Checklist |
| `src/note_images.rs` | Gambar di dalam alur teks atau overlay |
| `src/model.rs`, `src/storage.rs` | Model workspace, validasi, penulisan, dan backup |
| `src/navigation.rs`, `src/zoom.rs` | Pan, gesture, dan skala isi |
| `src/window_chrome.rs` | Kontrol jendela dan resize |
| `src/docs_render.rs` | Generator frame UI untuk dokumentasi, hanya pada build tes |

## Data demo dan gambar dokumentasi

```powershell
.\scripts\render-screenshots.ps1
```

Generator memakai workspace demo terpisah dan menulis lima gambar ke `docs/screenshots`. Metodenya dijelaskan di [README gambar](screenshots/README.md). Jangan mengganti contoh dengan catatan pribadi.

## Logo dan ikon

Gambar sumber ada di `assets/logo-notvas.png`. Windows memakai `assets/notvas.ico`; header dan ikon jendela memakai `assets/notvas-icon-256.png`. Asal aset tercatat di [logo-prompt.md](../assets/logo-prompt.md).

Jika gambar sumber diganti, jalankan `scripts/make-icon.ps1`, lalu build ulang. `build.rs` menanam ikon ke EXE. Build GNU memakai converter resource .NET Framework atau `windres`; lokasi dapat ditentukan melalui `CVTRES`/`WINDRES`. Build MSVC menautkan resource langsung.

## Dependensi dan lisensi

Setelah mengubah dependensi, perbarui `Cargo.lock` dan inventaris lisensi:

```powershell
.\scripts\update-third-party-notices.ps1
```

Script membaca cache Cargo secara offline. Jalankan `cargo fetch --locked` jika cache belum ada. Bila crate tidak memuat teks lisensinya, simpan salinan dari commit upstream yang sesuai di `docs/licenses/upstream` beserta URL dalam `sources.json`. Generator memeriksa commit fallback dan berhenti bila salinannya sudah tidak cocok. Ikuti [panduan distribusi](RELEASING.md) sebelum membuat paket.
