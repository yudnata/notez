# Gambar antarmuka

Gambar ini dibuat pada 19 November 2026 dari kode UI Notvas yang sama dengan aplikasi desktop, berukuran 1280 × 820 px. Isinya berasal dari workspace demo di `scripts/create-demo.ps1`.

Capture jendela Windows tidak tersedia pada lingkungan pembuatan dokumentasi: koneksi Computer Use gagal dan `PrintWindow` menghasilkan gambar hitam. Gambar di folder ini adalah render frame egui melalui rasterizer CPU di `src/docs_render.rs`, bukan tangkapan jendela Windows. Tata letak, teks, ikon, dan objek berasal dari fungsi `Notvas::draw`; hasil antialiasing dapat berbeda dari renderer OpenGL desktop. Tidak ada mockup yang digambar ulang atau konten pribadi.

| Berkas | Isi |
| --- | --- |
| [canvas.png](canvas.png) | Shape, panah terhubung, sticky, alat gambar, dan panel Gaya |
| [notes.png](notes.png) | Papan catatan berwarna dengan pencarian judul |
| [note-editor.png](note-editor.png) | Editor catatan dan toolbar format |
| [checklist.png](checklist.png) | Papan daftar dengan progres dan cuplikan tugas |
| [checklist-editor.png](checklist-editor.png) | Daftar penuh, checkbox, dan tugas selesai tercoret |

## Membuat ulang

Jalankan di Windows dari root repository:

```powershell
.\scripts\render-screenshots.ps1
```

Script membuat workspace demo jika belum ada, lalu menjalankan generator gambar yang ditandai `ignored` agar tes biasa tidak menulis aset dokumentasi. Folder data normal dan `.dev-data` tidak dibaca. `scripts/create-demo.ps1` menolak menimpa workspace yang sudah ada.

Untuk mencoba data yang sama di aplikasi:

```powershell
.\target\debug\notvas.exe --data-dir .\test-data\docs-demo\canvas
```

Lisensi gambar proyek mengikuti [MIT](../../LICENSE); atribusi font dan komponen tetap dijelaskan di [NOTICE.md](../../NOTICE.md).
