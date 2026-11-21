# Atribusi dan cakupan lisensi

Kode, dokumentasi, serta aset Notvas yang dibuat untuk proyek ini memakai [MIT](LICENSE). Dependensi dan font pihak ketiga tetap memakai lisensinya masing-masing. Lisensi proyek tidak mengganti lisensi komponen tersebut.

## Font

Patrick Hand Regular dibuat oleh Patrick Wagesreiter, copyright 2010–2012. File font dibundel di `assets/fonts/PatrickHand-Regular.ttf` dengan [SIL Open Font License 1.1](assets/fonts/OFL-PatrickHand.txt). Salinan lisensi harus ikut ketika font atau aplikasi yang memuat font didistribusikan. Berkas SVG hasil ekspor memuat font ini beserta atribusi dan teks lisensinya di metadata.

egui menyertakan font default melalui `epaint_default_fonts`. Pemberitahuan untuk Ubuntu, Noto Emoji, Hack, Bitstream Vera, dan emoji-icon-font tercantum dalam [teks lisensi dependensi](docs/licenses/THIRD_PARTY_LICENSES.txt), termasuk lisensi font dan nama yang dicadangkan upstream.

Notvas dapat memakai Segoe UI dari instalasi Windows untuk antarmuka. Berkas font Windows tersebut tidak disalin ke repository atau paket Notvas.

## Dependensi Rust

[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) mencatat nama, versi, lisensi, dan sumber dependensi normal serta build untuk target Windows GNU. [THIRD_PARTY_LICENSES.txt](docs/licenses/THIRD_PARTY_LICENSES.txt) memuat salinan teks lisensi dan pemberitahuan upstream.

Sebagian crate tidak menyertakan teks lisensinya dalam arsip Cargo. Salinan tambahannya diambil dari commit upstream yang dicatat dalam [sources.json](docs/licenses/upstream/sources.json). Generator berjalan secara offline dan tidak mengunduh atau menjalankan kode upstream.

## Logo dan gambar dokumentasi

Logo N dibuat untuk Notvas menggunakan ImageGen. Asal aset dan prompt pembuatannya tercatat dalam [assets/logo-prompt.md](assets/logo-prompt.md). Ikon aplikasi diturunkan dari gambar tersebut.

Gambar antarmuka di `docs/screenshots` berasal dari frame UI Notvas yang dirender dengan workspace demo. Tidak ada catatan pribadi di dalamnya. Metode dan cara membuat ulang gambar dijelaskan di [dokumentasi gambar](docs/screenshots/README.md).

## Distribusi

Sertakan `LICENSE`, dokumen ini, `THIRD_PARTY_NOTICES.md`, teks lisensi dependensi, dan lisensi Patrick Hand bersama paket aplikasi. `scripts/package-release.ps1` menyalin berkas tersebut ke paket portable. Perbarui inventaris ketika `Cargo.lock`, target, fitur Cargo, atau aset pihak ketiga berubah.

Panduan penggunaan lisensi: [GitHub Docs](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository) dan [SIL Open Font License](https://openfontlicense.org/).
