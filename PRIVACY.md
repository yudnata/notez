# Data dan privasi

Notvas menyimpan workspace pada disk lokal. Aplikasi tidak memiliki akun, telemetri, atau layanan unggah. Dialog pemilih file membuka lokasi yang dipilih pengguna; folder tersebut bisa berada dalam layanan sinkronisasi yang dikelola di luar Notvas.

Lokasi awal di Windows adalah `%APPDATA%\Notvas\workspace.notvas`. Development memakai `.dev-data/workspace.notvas`. Argumen `--data-dir` dapat memilih folder lain.

Workspace memuat teks, format, tugas, warna/posisi kartu, objek Canvas, dan gambar sebagai base64. Gambar disalin ke workspace saat diimpor. Berkas sumber tidak diperlukan lagi untuk menampilkan gambar tersebut.

Penulisan berikutnya menyimpan versi sebelumnya sebagai `.notvas.bak`. Menghapus catatan dari aplikasi tidak otomatis menghapus salinan di backup atau ekspor. Untuk menghapus data dari disk, tutup Notvas lalu hapus workspace, backup, dan ekspor yang terkait.

Ekspor HTML dan SVG dapat memuat teks serta gambar tertanam. Periksa isinya sebelum membagikan berkas. Berkas `.notvas`, `.notvas.bak`, `.dev-data`, dan `test-data` diabaikan Git agar tidak ikut ditambahkan secara tidak sengaja.

Workspace belum dienkripsi. Hak akses folder dan perlindungan perangkat mengikuti pengaturan Windows.
