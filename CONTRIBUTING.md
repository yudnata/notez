# Berkontribusi

Laporkan bug dan usulkan perubahan melalui [Issues](https://github.com/yudnata/notvas/issues). Untuk perubahan besar, jelaskan kebutuhan dan contoh penggunaannya terlebih dahulu agar ruang lingkupnya jelas.

Saat melaporkan bug, sertakan versi atau commit, versi Windows, langkah untuk mengulang masalah, serta hasil yang diharapkan. Gunakan workspace demo untuk screenshot dan berkas contoh. Hapus informasi pribadi dari log.

## Menyiapkan perubahan

Ikuti [panduan development](docs/DEVELOPMENT.md). Buat branch untuk satu perubahan yang berkaitan. Hindari perubahan format, nama, atau dependensi yang tidak diperlukan untuk pekerjaan tersebut.

Jalankan formatter, tes yang relevan, dan build sebelum mengirim pull request. Untuk perubahan UI, sertakan gambar sebelum/sesudah dan periksa jendela kecil. Untuk bug input, gunakan event pointer atau keyboard dalam tes sehingga alur yang rusak benar-benar dijalankan.

PR perlu menjelaskan perilaku yang berubah, cara memeriksanya, dan keterbatasan yang masih ada. Jangan memasukkan `.tools`, `target`, `test-data`, workspace pribadi, atau hasil build.

## Lisensi kontribusi

Kontribusi kode dan dokumentasi diberikan dengan lisensi MIT proyek. Jika menambahkan kode, font, ikon, atau gambar dari pihak lain, sertakan sumber dan lisensinya. Perbarui [atribusi](NOTICE.md) serta inventaris dependensi jika diperlukan. Jangan memasukkan aset yang izin distribusinya tidak jelas.

Untuk laporan keamanan, ikuti [SECURITY.md](SECURITY.md).
