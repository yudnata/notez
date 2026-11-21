# Menggunakan Notvas

Pilih Canvas, Notes, atau Checklist dari header. Menu Dokumen mencari judul dan isi pada mode aktif. Dropdown tetap terbuka saat kolom pencarian diketik; pilih hasil, klik di luar, atau tekan Esc untuk menutupnya.

## Canvas

Pilih alat di panel kiri, lalu gambar pada kertas. Pena dan stabilo menghasilkan stroke bebas. Kotak, oval, belah ketupat, sticky, teks, gambar, garis, dan panah dapat dipilih serta diubah melalui panel kanan. Double-click atau F2 mengedit teks di shape dan label panah.

Shape geometris baru memakai tinta hitam 2 px dengan isi transparan. Pilihan tinta dan isi ada di bagian Gaya. Mengubah objek terpilih tidak otomatis mengubah gaya objek baru.

Panah bisa lurus, siku, atau lengkung. Dekatkan ujungnya ke batas shape untuk menyambung; penanda menunjukkan titik yang dipakai. Sambungan menyesuaikan ketika shape digeser atau diperbesar. Tarik ujung untuk menyambung ulang, tarik handle untuk mengubah jalur, atau tahan Alt untuk melepas snap. Jalur menghindari kedua shape terhubung; pencarian jalur mengitari objek lain belum tersedia.

Penghapus memakai ukuran kuas 8–120 px dengan preview pada kursor. Shape dan gambar terhapus sebagai objek utuh. Pena/stabilo terpotong pada bagian yang dilewati kuas. Satu sapuan dapat dibatalkan dengan satu Undo; objek terkunci dilewati.

Impor PNG/JPEG melalui tombol Gambar atau drop file. Gambar mempertahankan rasio saat di-resize. Batas impor adalah 20 MB dan 40 megapiksel per gambar. Canvas dapat diekspor ke PNG/SVG; SVG menyertakan font Patrick Hand dan lisensinya. PNG dibatasi 8192 px per sisi dan 24 megapiksel.

## Notes dan Checklist

Kedua mode memakai papan kartu 208 × 160 dengan enam pilihan warna. Judul tetap terlihat; pencarian mencocokkan judul tanpa membedakan huruf besar/kecil. Saat mencari, hasil dirapatkan tanpa mengubah posisi yang tersimpan. Kosongkan pencarian untuk melihat susunan semula.

Klik kartu untuk membuka isinya. Drag kiri menyusun kartu; Rapikan posisi mengembalikan tata letak otomatis. Klik kanan membuka pengaturan warna dan hapus dengan konfirmasi. Tombol Kembali ke papan atau klik tab mode aktif menampilkan papan lagi.

Notes mendukung ukuran teks 12–48 px, bold, italic, underline, jarak baris, dan undo/redo format. Gambar disisipkan pada posisi tulisan. Layout teks menyediakan ruang di antara paragraf; Overlay dapat digeser di atas tulisan. Resize menjaga rasio gambar. Ekspor catatan ke HTML atau TXT melalui menu Ekspor.

Kartu Checklist menampilkan progres dan tiga cuplikan tugas. Daftar penuh memakai checkbox besar; tugas selesai dicoret. Teks tugas tetap bisa diedit. Tambah tugas melalui kolom input, lalu klik Tambah atau tekan Enter.

## Geser dan zoom

Tahan klik kanan lalu seret, atau gunakan swipe trackpad/gesture dua jari. Canvas dapat bergeser bebas. Papan dan editor lain mengikuti batas area gulir. Klik kanan singkat pada kartu tetap membuka menu.

Ctrl+scroll memperbesar atau memperkecil isi tanpa mengubah ukuran header. Kontrol zoom juga ada di toolbar. Fit menampilkan seluruh isi Canvas; pada Notes/Checklist tombol tersebut mengembalikan zoom ke 100%.

## Penyimpanan dan backup

Autosave berjalan 500 ms setelah berhenti mengetik atau menghapus, dan paling lambat setiap 5 detik selama mengetik terus-menerus. Perubahan tugas dan aksi lain yang bersifat diskret dijadwalkan segera. Berpindah dokumen, kehilangan fokus, dan menutup aplikasi menyelesaikan penulisan tertunda.

Penulisan menggunakan berkas sementara dan rename. Penulisan berikutnya mencadangkan versi sebelumnya sebagai `workspace.notvas.bak`. File yang gagal dibaca tidak ditimpa otomatis. Jika penulisan gagal, perubahan tetap di memori dan aplikasi mencoba lagi; penutupan ditahan sampai data berhasil ditulis atau pengguna kembali ke editor.

File awal di Windows adalah `%APPDATA%\Notvas\workspace.notvas`. Development memakai `.dev-data/workspace.notvas`. Menu File dapat membuka workspace lain. Lokasi alternatif juga bisa dipilih melalui argumen:

```powershell
.\Notvas.exe --data-dir .\data-pribadi
```

Jangan membuka workspace yang sama pada dua jendela: penulisan terakhir akan menjadi versi aktif. Workspace, backup, dan ekspor belum dienkripsi. Lihat [PRIVACY.md](../PRIVACY.md) sebelum membagikan berkas.

## Shortcut

| Shortcut | Fungsi |
| --- | --- |
| Ctrl+N | Dokumen baru pada mode aktif |
| Ctrl+O | Buka workspace |
| Ctrl+scroll atas / bawah | Zoom in / out di Canvas, papan/editor Notes, dan Checklist |
| Ctrl+0 | Reset zoom ke 100% (di Canvas, klik area gambar terlebih dahulu) |
| Ctrl+B / Ctrl+I / Ctrl+U | Format teks di editor Notes |
| Ctrl+Z / Ctrl+Y | Undo / redo di editor Notes |
| Tab / Shift+Tab | Pindah fokus kontrol |
| Escape | Batalkan konfirmasi |

Shortcut berikut aktif saat canvas memiliki fokus (klik area canvas), dan tidak mengambil alih pengetikan teks:

| Shortcut | Fungsi canvas |
| --- | --- |
| V / L / P / H / E / N / T / S | Pilih / lasso / pena / stabilo / penghapus / sticky / teks / kotak |
| A | Panah |
| Space+drag / middle-drag / tahan klik kanan+drag | Pan |
| Swipe trackpad / dua jari | Geser canvas |
| Ctrl+wheel / pinch | Zoom di sekitar pointer |
| Ctrl+0 / Ctrl++ / Ctrl+- | Reset / perbesar / perkecil |
| Ctrl+Shift+F | Fit seluruh objek |
| Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z | Undo / redo |
| Ctrl+C / Ctrl+X / Ctrl+V / Ctrl+D | Copy / cut / paste / duplicate |
| Ctrl+A / Delete | Pilih semua / hapus pilihan |
| F2 / double-click | Edit teks shape atau label panah |
| Arrow keys / Shift+arrow | Geser pilihan 1 / 10 unit |
| Ctrl+Shift+] / Ctrl+Shift+[ | Ke depan / ke belakang |
| Ctrl+R | Penggaris |
| Ctrl+N / Ctrl+O / Ctrl+S | Dokumen baru / buka workspace / ekspor salinan canvas `.notvas` |
| Alt saat menarik ujung panah | Lepaskan smart snap |

## Batas yang perlu diketahui

Undo menyimpan sampai 100 perubahan pada dokumen aktif dan direset saat berpindah dokumen. PDF/PPT/Word, `.excalidraw`, rotasi objek, tabel rich text, dan deteksi khusus stylus belum didukung. Ukuran jendela minimum 380 × 520 px. Menu Kanban sudah dihapus; data Kanban lama tetap dipertahankan dalam workspace.
