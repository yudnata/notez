# Pemeriksaan konsep Notez

Tanggal: 18 November 2026, Asia/Makassar.

## Build dan peluncuran

- Rust stable 1.100.0, target x86_64-pc-windows-gnu; eframe/egui 0.33.3.
- Build debug dan release berhasil. Source diformat dengan rustfmt.
- Peluncuran native debug: proses menampilkan judul jendela Notez, `Responding=True`, stderr kosong.
- Build release mencakup revisi smart autosave dan tema monokrom. Salinan siap jalan tersedia di root proyek sebagai `Notez.exe`.
- Peluncuran release revisi: judul jendela Notez, `Responding=True`, stderr kosong. Hash SHA-256 salinan root sama dengan hasil build release.
- Tiga belas unit test lulus. Pengujian frame memakai egui tanpa layar; ini bukan pengujian screenshot atau klik pada jendela Windows.
- Revisi ikon: logo N dimuat pada jendela serta header sidebar. Resource EXE berisi tujuh RT_ICON dan satu RT_GROUP_ICON; inspeksi resource selesai tanpa error. Ekstraksi ikon langsung dari hasil EXE menghasilkan logo N, diperiksa secara visual pada `test-data/embedded-exe-icon.png`.

## Pengujian otomatis

| Alur | Hasil |
| --- | --- |
| Sisip/hapus karakter Unicode di tengah teks berformat | Format karakter tetangga tetap benar |
| Ekspor HTML | Format dan jarak baris dipertahankan, markup teks di-escape |
| Simpan/buka workspace Notes, checklist, kanban | Data kembali sama, termasuk status dan format |
| Simpan ulang | Versi sebelumnya tersedia di `.notez.bak` |
| Baca file rusak | Menghasilkan error tanpa mengubah file asal |
| Format seleksi, undo, redo | Konten dan format kembali benar; bold mempertahankan ukuran teks lain |
| Input editor lewat event keyboard egui | Menulis, Ctrl+A, Ctrl+B, dan Ctrl+Z mengubah model sesuai harapan |
| Ketiga mode pada 940×620 dan 1280×820 | Frame kosong dan terisi, modal, serta error menghasilkan mesh dengan koordinat valid |
| Kontras teks utama dan sekunder | Memenuhi rasio WCAG AA 4.5:1 pada latar yang diuji |
| Warna permukaan, seleksi, hover, dan active | Semua channel RGB sama; tidak ada aksen hijau |
| Jeda autosave 500 ms | Setiap edit mengulang jeda; tidak menyimpan sebelum batas waktu |
| Mengetik terus-menerus | Penyimpanan jatuh tempo paling lambat setelah 5 detik |
| Perubahan disk setelah mengetik dan menghapus | Isi terbaru, termasuk teks kosong, terbaca kembali dari file |
| Disk gagal kemudian tersedia lagi | Perubahan tetap di memori, retry ditunda 2 detik, lalu tersimpan otomatis |
| Kehilangan fokus | Teks yang masih menunggu jeda langsung ditulis |
| Menutup jendela ketika disk gagal | Perintah CancelClose dikirim, teks tetap tersedia, modal ditampilkan |
| Pemilihan folder pemulihan | Nama file baru tidak menimpa workspace yang sudah ada |

## Kontrol yang diperiksa melalui source

| Kontrol | Implementasi |
| --- | --- |
| Notes / Checklist / Kanban | Memfilter daftar dan memilih dokumen pada mode terkait |
| Buat baru / Ctrl+N | Membuat dokumen sesuai mode aktif |
| Pencarian | Memfilter judul, isi catatan, dan teks tugas |
| Judul | Editable; menandai workspace untuk autosave |
| B / I / U / ukuran | Mengubah format seleksi atau format ketikan berikutnya |
| Jarak baris | Mengubah seluruh catatan dan ekspor HTML |
| Undo / Redo | Riwayat konten dan format pada Notes |
| Tambah tugas / Enter | Menambah tugas jika teks tidak kosong |
| Checkbox | Menjadwalkan penyimpanan langsung pada frame yang sama |
| Judul/rincian kartu | Editable dan disimpan |
| Status kartu / drag handle | Memindahkan kartu ke kolom tujuan |
| Hapus tugas / kartu | Menghapus tugas terkait |
| Hapus dokumen | Modal konfirmasi; Batal dan Escape menutup modal |
| Simpan / Simpan sebagai / Ctrl+S | Dihapus; penyimpanan otomatis melalui perubahan data |
| Pilih folder penyimpanan lain | Hanya muncul saat ada masalah; memindahkan autosave ke file baru |
| Buka workspace / Ctrl+O | Menyimpan perubahan aktif lebih dulu, lalu memuat dan memvalidasi file pilihan |
| Ekspor HTML / TXT | Dialog file native dan penulisan hasil ekspor |
| Tutup jendela saat belum tersimpan | Menyelesaikan autosave; jika gagal, menahan penutupan dan mencoba lagi otomatis |

## Batas verifikasi

Layanan Computer Use tidak tersambung: `Computer Use native pipe is unavailable ... os error 2`. Percobaan melalui dua antarmuka kontrol memberikan kendala yang sama. Karena itu screenshot, pemeriksaan visual, klik seluruh kontrol, drag-drop fisik, urutan Tab, dan dialog file Windows belum terverifikasi secara langsung. Aplikasi ini diserahkan sebagai konsep awal yang dapat dijalankan; audit visual penuh belum dinyatakan lulus.

Empty state dan error state tersedia. Data lokal dimuat secara sinkron sebelum UI muncul, sehingga tidak ada indikator loading buatan. Tema yang disediakan hanya dark mode sesuai permintaan. Logo dibuat dengan ImageGen atas permintaan pengguna; tidak ada statistik buatan. Alasan desain dan arah visual tercatat di `DESIGN.md`.
