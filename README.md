# Notez

Aplikasi desktop native Rust dengan tema gelap. UI menggunakan egui/eframe dan OpenGL, tanpa browser atau WebView.

## Jalankan

Klik dua kali `Notez.exe` setelah build portable tersedia, atau jalankan dari PowerShell:

```powershell
.\run.ps1
```

Toolchain portable untuk mesin ini ada di `.tools` dan tidak mengubah PATH Windows. Untuk komputer lain, pasang Rust stable beserta linker untuk target Windows yang digunakan, lalu `cargo run --release`. Dependensi dipatok melalui `Cargo.lock`.

```powershell
.\run.ps1 -Release -BuildOnly
```

Hasil build ada di `target/release/notez.exe`. Salinan siap jalan ditempatkan di `Notez.exe` pada root proyek.

## Logo dan ikon aplikasi

Logo N dipasang pada ikon file EXE Windows, ikon jendela/taskbar, dan header sidebar. Gambar asli ada di `assets/logo-notez.png`. Ikon Windows memakai `assets/notez.ico` dengan ukuran 16, 24, 32, 48, 64, 128, dan 256 px; jendela memakai `assets/notez-icon-256.png`.

Jika gambar asli diganti, jalankan `scripts/make-icon.ps1`, lalu build ulang. `build.rs` menanam resource ikon ke EXE secara otomatis. Build GNU di Windows memakai converter resource .NET Framework bila tersedia, atau `windres` dari binutils. Lokasi converter bisa ditentukan lewat `CVTRES` atau `WINDRES`. Build MSVC menautkan resource langsung.

## Fitur konsep awal

- Notes: format seleksi teks dengan ukuran 12–48 px, bold, italic, underline; atur jarak baris untuk satu catatan. Format tanpa seleksi berlaku untuk tulisan berikutnya. Undo/redo menyertakan format teks, hingga 100 perubahan selama catatan aktif.
- Checklist: tambah tugas, edit teks, centang selesai, hapus tugas.
- Kanban: kolom Rencana, Dikerjakan, Selesai; edit judul dan rincian kartu, pindahkan lewat drag handle atau pilihan status.
- Dokumen terpisah untuk tiap mode, pencarian judul/isi, konfirmasi hapus dokumen.
- Smart autosave setelah 0,5 detik berhenti mengetik atau menghapus, tanpa tombol Simpan atau Simpan sebagai.
- Checklist, perubahan format, tambah/hapus tugas, dan perpindahan kartu tersimpan langsung. Perpindahan dokumen, kehilangan fokus jendela, dan penutupan aplikasi menyelesaikan penyimpanan tertunda.
- Tema monokrom: hitam, abu-abu netral, dan putih lembut, termasuk seleksi teks, hover, dan fokus.
- Ekspor Notes ke HTML dengan format atau TXT tanpa format.

## Penyimpanan

File awal di Windows: `%APPDATA%\Notez\workspace.notez`. File `.notez` berupa JSON dan berisi seluruh Notes, checklist, dan papan kanban. Semua perubahan tersimpan otomatis; Buka workspace memilih file kerja lain untuk sesi tersebut.

Setiap edit mengulang hitungan jeda 500 ms, termasuk Backspace, Delete, paste, dan undo. Selama mengetik terus-menerus, aplikasi juga menyimpan maksimal tiap 5 detik. Tidak ada penulisan ulang saat tidak ada perubahan. Jika penyimpanan gagal, perubahan tetap di memori dan aplikasi mencoba lagi tiap 2 detik sampai berhasil. Status bawah menampilkan menunggu autosave, tersimpan, atau mencoba kembali. Penutupan ditahan bila perubahan belum berhasil ditulis.

Penulisan memakai file sementara, flush ke disk, lalu rename. Versi sebelumnya dicadangkan sebagai `workspace.notez.bak` pada penyimpanan berikutnya. Cadangan bisa dibuka melalui Buka workspace. File rusak ditampilkan sebagai error dan tidak ditimpa otomatis. Saat terjadi masalah, pilihan folder penyimpanan lain membuat file baru tanpa menimpa workspace yang sudah ada di folder tujuan. Untuk membuka lokasi alternatif pada sesi selanjutnya, gunakan Buka workspace atau argumen `--data-dir`. Jangan membuka file workspace yang sama dalam dua jendela karena versi terakhir yang disimpan akan berlaku.

Untuk memakai folder data terpisah:

```powershell
.\Notez.exe --data-dir .\data-pribadi
```

## Shortcut

| Shortcut | Fungsi |
| --- | --- |
| Ctrl+N | Dokumen baru pada mode aktif |
| Ctrl+O | Buka workspace |
| Ctrl+B / Ctrl+I / Ctrl+U | Format teks di editor Notes |
| Ctrl+Z / Ctrl+Y | Undo / redo di editor Notes |
| Tab / Shift+Tab | Pindah fokus kontrol |
| Escape | Batalkan konfirmasi |

## Batas konsep

Belum ada gambar, tabel, PDF, sinkronisasi, enkripsi, dan interoperabilitas DOCX/RTF. HTML adalah ekspor satu arah. Ukuran jendela minimum 940 × 620; papan kanban bisa digulir horizontal. Riwayat undo catatan direset saat berpindah dokumen dan tidak disimpan ke disk.

## Pemeriksaan

Pengujian model meliputi penyisipan/penghapusan Unicode dengan format, escaping HTML, penyimpanan ketiga mode, backup, dan pembacaan file rusak.

```powershell
.\run.ps1 -Test
```

Gunakan environment portable dari `run.ps1` jika Cargo belum ada di PATH. Hasil pemeriksaan build dan UI dicatat di `QA.md`.
