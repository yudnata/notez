# Pemeriksaan Notvas development

Tanggal: 19 November 2026, Asia/Makassar.

## Build dan development

- Rust stable 1.100.0, target x86_64-pc-windows-gnu; eframe/egui 0.33.3.
- Source diformat dengan rustfmt. Build debug untuk tema sketchbook, canvas, dan papan Notes berhasil.
- 61 tes lulus. Pengujian input menggunakan event egui tanpa layar; bukan klik pada jendela Windows.
- Profil development memakai incremental compilation tanpa debug symbols besar. Build setelah perubahan kode terakhir sekitar 6,7 detik; build tanpa perubahan 0,6–0,7 detik pada mesin ini. Perubahan dependensi tetap memerlukan kompilasi lebih lama.
- `dev.ps1 -Watch` diuji dengan perubahan waktu modifikasi aset: proses aplikasi sebelumnya berakhir, build berhasil, lalu proses baru terbuka. Proses baru `Responding=True` dan stderr aplikasi kosong. Watcher meminta penutupan melalui aplikasi agar autosave selesai sebelum restart; tidak memakai penghentian paksa.
- Data development berada di `.dev-data`, terpisah dari workspace pengguna. Log watcher pengujian ada di `test-data/dev-watch-out.log` dan `test-data/dev-watch-err.log`.
- `Notvas.exe` di root masih release sebelumnya: smart autosave, tema monokrom gelap, dan logo N. Fitur baru dijalankan melalui `Notvas-dev.cmd` atau `dev.ps1`; belum dipaketkan sebagai release baru.
- Resource ikon pada release sebelumnya sudah diperiksa: tujuh RT_ICON dan satu RT_GROUP_ICON. Ekstraksi ikon EXE menampilkan logo N pada `test-data/embedded-exe-icon.png`.

## Pengujian otomatis

| Alur | Hasil |
| --- | --- |
| Sisip/hapus karakter Unicode, format seleksi, undo/redo Notes | Konten dan format karakter dipertahankan |
| Input keyboard Notes | Menulis, memilih semua, bold, dan undo mengubah model sesuai harapan |
| Ekspor HTML | Format dan jarak baris dipertahankan; markup teks di-escape |
| Simpan/buka semua mode | Notes, checklist, kanban, canvas, gambar, serta warna/posisi kartu kembali sama |
| Workspace lama | Field canvas dan kartu yang belum ada mendapat nilai default |
| Simpan ulang dan file rusak | Backup tersedia; file rusak menghasilkan error tanpa ditimpa |
| Empat mode dari 380×520 hingga 1280×820 | Frame kosong/terisi, modal, dan error menghasilkan mesh dengan koordinat valid |
| Header, aksi papan, dan panel canvas saat resize | Kontrol tetap di dalam clip dan jendela pada 380, 640, 800, 940, dan 1280 px; panel ikon berada di kiri canvas dengan dua ikon per baris; panel pengaturan berada di kanan, termasuk editor nama canvas saat terbuka penuh |
| Klik dan gulir panel ikon/pengaturan | Klik ikon memilih alat; ikon paling bawah dapat dicapai dengan gulir pada 380×520, tanpa menggeser atau menggambar di canvas; ikon kanan membuka popup pengaturan pada lebar kecil |
| Klik kanan Notes lalu hapus | Konfirmasi menghapus catatan yang diklik, termasuk saat catatan lain sebelumnya dipilih |
| Pena/stabilo | Goresan selesai tanpa pilihan otomatis atau kotak seleksi |
| Drag klik kanan dan swipe | Offset berubah satu kali per gerakan pada sumbu yang didukung, berhenti saat dilepas, dan dibatasi ukuran isi; gesture dua jari menggeser area gulir dan canvas |
| Geser tanpa mengedit | Drag kanan saat pena aktif tidak membuat stroke; drag pada kartu Notes tidak membuka atau memindahkan catatan; drag handle Kanban tidak membuat payload atau memindahkan tugas |
| Kontras teks dan permukaan | Teks utama/sekunder memenuhi 4.5:1 pada permukaan tema yang diuji; teks hitam pada keenam warna kartu memenuhi 7:1; teks tombol memenuhi 4.5:1 pada permukaan normal/hover/aktif. Tombol polos memakai border 0,7–0,8 px sesuai permintaan terbaru |
| Autosave | Jeda 500 ms diulang setiap edit, batas 5 detik saat mengetik terus, penghapusan hingga kosong tersimpan |
| Gagal menulis | Data tetap di memori, retry setelah 2 detik, kemudian tersimpan saat disk tersedia |
| Kehilangan fokus dan penutupan | Perubahan tertunda ditulis; penutupan ditahan jika disk gagal |
| Folder pemulihan | Membuat nama baru tanpa menimpa workspace yang sudah ada |
| Binding panah | Ujung mengikuti shape yang dipindah; penghapusan target melepas binding pada posisi terakhir |
| Penyesuaian sambungan setelah transformasi | Drag shape melewati target mengubah kedua sisi sambungan; Undo memulihkan anchor awal dan Redo memulihkan hasil. Resize terdeteksi melalui geometri shape. Sebanyak 48 kombinasi kotak/oval/belah ketupat/sticky, panah lurus/siku/lengkung, serta empat posisi diuji: sampel sepanjang garis berada di luar isi kedua shape terhubung. Jalur hasil tetap sama setelah roundtrip JSON |
| Saran sambungan mengikuti kursor | Empat sisi kotak, oval, dan belah ketupat memakai titik terdekat dengan kursor; sudut kosong oval/belah ketupat tidak tersambung dari jauh. Preview dan hasil sambungan sama pada event hover, drag panah, dan pemindahan ujung panah di zoom 50/100/200%. Posisi sambungan mengikuti move/resize/duplicate serta roundtrip JSON. Binding workspace lama tanpa anchor tetap dimuat; anchor tidak valid ditolak |
| Duplikasi kelompok | Binding diarahkan ke salinan shape dalam kelompok |
| Pointer dan keyboard canvas | Drag membuat shape; F2 dan input teks menambahkan teks di dalamnya |
| Editor teks Canvas | Memulai edit dan mengetik tidak menggeser panel pengaturan; input yang melewati tepi Canvas tidak mengambil area klik panel kanan. Posisi dan lebar teks memakai area yang sama saat tampil dan diedit |
| Keluar dari edit teks Canvas | Klik pertama dan double-click area kosong mengakhiri edit teks sekaligus menghapus seleksi. Double-click di dalam teks tetap mengedit; isi shape dan riwayat undo tidak berubah saat seleksi dilepas |
| Default shape | Kotak, oval, belah ketupat, panah, dan garis baru tetap hitam 2 px transparan setelah memilih sticky atau stabilo; gaya seleksi terpisah dari gaya objek baru |
| Teks kosong | Frame memuat penanda ketika teks kosong/spasi, lalu menghilangkannya saat diisi. Penanda tidak ikut SVG; klik objek kosong lalu Delete menghapusnya |
| Penghapus | Pemotongan di tengah stroke jarang dan berbelok mempertahankan bagian di luar kuas. Sapuan cepat tidak berlubang; pena/stabilo tersimpan kembali sebagai jalur yang valid. Shape terhapus utuh dan binding panah dilepas, objek terkunci tetap ada |
| Preview dan Undo penghapus | Event drag pada zoom 200% menghasilkan lingkaran 40 px sesuai potongan jalur; satu Undo memulihkan seluruh sapuan dan Redo mengembalikannya. Sapuan area kosong tidak menambah riwayat. Rail ukuran terlihat selebar lebih dari 180 px; klik sisi kanan slider mengubah ukuran menjadi lebih dari 100 px tanpa menambah Undo |
| Header jendela terpadu | Tombol mengirim perintah minimize/maximize/tutup; drag/double-click area kosong dan resize sudut mengirim perintah viewport yang sesuai. Tombol tutup menyimpan perubahan sebelum menutup dan menahan penutupan ketika penyimpanan gagal. Interaksi jendela Windows secara langsung belum diverifikasi |
| Ctrl+scroll semua mode | Scroll atas/bawah memperbesar/memperkecil Canvas, papan/editor Notes, Checklist, dan Kanban. Header tetap 100%, zoom Notes/Checklist/Kanban tidak mengubah data dokumen, dan Ctrl+0 mereset tampilannya |
| Pointer setelah zoom | Checkbox tetap bisa diklik pada koordinat tampilannya; drag kanan 90 px pada zoom 150% menggeser isi 60 unit |
| Impor gambar dan riwayat | Bytes gambar dipertahankan melalui JSON base64 dan undo/redo |
| Gambar Notes | Default mengikuti alur teks; baris berikutnya berada di bawah gambar. Resize menjaga rasio dan gambar muat pada Notes sempit. Overlay bisa diseret beberapa gerakan berturut-turut tanpa mengubah alur teks |
| Anchor gambar dan penyimpanan | Sisipan Unicode menggeser anchor, penghapusan marker menghapus gambar; kedua layout dan bytes tertanam kembali sama setelah simpan/buka. Workspace lama tanpa gambar tetap bisa dimuat |
| Ekspor SVG/PNG | Teks di-escape, font tertanam, PNG valid, template mempertahankan label dan sambungan |
| Papan 10 Notes | Warna bervariasi; klik kartu membuka Notes terkait; ukuran dasar kartu tetap 208 × 160 |
| Pencarian judul Notes | Input Unicode dan huruf besar/kecil mencocokkan judul saja; catatan yang tidak cocok tidak tampil. Hasil dipadatkan dan pencarian kosong menampilkan kembali semua kartu tanpa mengubah posisi yang tersimpan |
| Pembungkus editor Notes | Judul putih dan isi gelap memakai permukaan berbeda; header mengikuti keenam warna kartu. Kontras teks putih pada keadaan normal/hover/aktif memenuhi 4.5:1; pembungkus tetap muat pada lebar 380 dan 1280 px |
| Baris Checklist | Tiga kartu tetap sejajar pada lebar 380/640/1280 px; tombol hapus berada di dalam kartu. Klik checkbox 44 × 44 px mencoret teks dan menyimpan status; Space membatalkan centang; hapus menghilangkan tugas yang benar |

## Pemeriksaan visual dan batas verifikasi

Ekspor canvas template diperiksa secara visual pada `test-data/new-theme/whiteboard-preview.png`: kertas berkotak, tulisan Patrick Hand, shape bergaris sketsa, panah, serta sticky notes pastel terlihat. SVG pendamping ada pada folder yang sama. Ini hasil ekspor canvas, bukan screenshot jendela aplikasi.

Computer Use belum tersambung (`Computer Use native pipe is unavailable`, os error 2). Karena itu tata letak native, seluruh interaksi pointer/keyboard, dialog file Windows, dan drag-drop fisik belum diperiksa langsung. Pengujian event egui dan inspeksi source tidak menggantikan pemeriksaan tersebut.

Kontrol canvas mencakup pena/stabilo, penghapus dengan potongan jalur, lasso, shape berteks, panah lurus/siku/lengkung dengan smart snap, gambar PNG/JPEG, template, ekspor, serta shortcut yang tercatat di README. Notes memiliki papan kartu berwarna yang bisa disusun dan editor penuh. Tema terang mengikuti referensi terbaru pengguna; alasan desain ada di `DESIGN.md`.

## Pemeriksaan perubahan shape dan penghapus

- Hard Gate PASS dalam pengujian otomatis: 51 tes lulus, termasuk frame responsif yang ada; penanda #646464 di atas kertas terang memiliki kontras lebih dari 4,5:1. Kontrol memakai widget keyboard egui dan alat tetap memiliki shortcut E. Pemeriksaan jendela native tetap memiliki batas yang dicatat di atas.
- Purpose-Gate PASS: garis hitam/transparan mengikuti permintaan pengguna; penanda menemukan objek tak terlihat; lingkaran menjelaskan area hapus. Alasan dan aturan gaya tercatat di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: kertas berkotak, tulisan tangan, dan aksen seleksi tetap mengikuti ENERGY 1 / RHYTHM 2 / MOTION 1, tanpa dekorasi tambahan.
- Craftsmanship PASS dalam pengujian otomatis: default shape, marker kosong, geometri penghapus, preview saat zoom, objek terkunci, penyimpanan hasil potongan, serta Undo/Redo memiliki bukti tes di atas. Build debug dan rustfmt diperiksa ulang.

## Pemeriksaan bingkai aplikasi

- Hard Gate PASS dalam pengujian otomatis: 54 tes lulus, termasuk semua mode kosong/berisi pada lebar 380, 640, 800, 940, dan 1280 px serta kontrol navigasi saat resize. Status footer memakai teks #636363 di atas #f7f7f7; teks panjang dipotong dengan tooltip lengkap. Verifikasi native tetap terbatas seperti dicatat di atas.
- Purpose-Gate PASS: sesuai sketsa terbaru, bingkai luar dan kotak main di dalamnya memakai dua border abu 0,8 px dengan sudut kecil 6 px. Main memiliki margin 6 px; header dan footer berada di luar kotak main, dalam bingkai luar. Aturan dan alasan tercatat di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: motif kertas dan tulisan tangan tetap menjadi fokus dengan ENERGY 1 / RHYTHM 2 / MOTION 1; bingkai memakai abu netral tanpa bayangan tambahan.
- Craftsmanship PASS dalam pengujian otomatis: tes resize memeriksa main terletak di antara header/footer, sudut kecil, toolbar/pengaturan berada di dalam border, footer maksimal 36 px, dan bingkai luar mengelilingi semuanya. Frame juga harus melukis latar terang opak seluas jendela agar celah tidak hitam; clear color eframe disamakan dengan latar. Kontrol jendela, navigasi, zoom, editor Notes/Canvas, dan Checklist/Kanban tetap lulus. Rustfmt serta build debug diperiksa ulang.

## Pemeriksaan saran sambungan

- Hard Gate PASS dalam pengujian otomatis: 54 tes lulus, termasuk event pointer untuk preview dan sambungan pada tiga tingkat zoom. Tampilan dan shortcut tidak berubah; batas verifikasi native tetap dicatat di atas.
- Purpose-Gate PASS: titik ungu menunjukkan tempat sambungan sebenarnya pada sisi terdekat dengan kursor; alasan perilaku tercatat di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: penanda yang sama mempertahankan aksen seleksi dan motif sketsa sesuai ENERGY 1 / RHYTHM 2 / MOTION 1.
- Craftsmanship PASS dalam pengujian otomatis: saran empat sisi, bentuk oval/belah ketupat, hasil sambungan, move/resize/duplicate, penyimpanan anchor, dan kompatibilitas binding lama teruji. Rustfmt dan build debug diperiksa ulang.

## Pemeriksaan kontrol ukuran penghapus

- Hard Gate PASS dalam pengujian otomatis: 54 tes lulus; rail #919191 terlihat di atas putih dengan kontras lebih dari 3:1. Slider memakai widget keyboard egui, terhubung ke label Ukuran penghapus; rentang dan nilai px terlihat.
- Purpose-Gate PASS: rail 4 px dan pegangan bulat membedakan pengatur ukuran dari tombol. Nilai bisa diketik dan batas 8–120 px tercantum; aturan visual tercatat di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: kontrol tetap monokrom dengan ENERGY 1 / RHYTHM 2 / MOTION 1 dan tidak menambah panel atau dekorasi.
- Craftsmanship PASS dalam pengujian otomatis: tes penghapus memeriksa rail yang terlihat dan perubahan ukuran lewat event klik slider; Undo canvas tetap sama. Rustfmt dan build debug berhasil. Verifikasi native tetap terbatas seperti dicatat di atas.

## Pemeriksaan panah setelah shape dipindahkan

- Hard Gate PASS dalam pengujian otomatis: 56 tes lulus, termasuk event drag dan pemeriksaan geometri jalur; tampilan, fokus, dan shortcut tidak berubah. Batas verifikasi native tetap dicatat di atas.
- Purpose-Gate PASS: kedua sambungan beralih otomatis setelah transformasi shape supaya panah mengikuti posisi baru; preview awal tetap memakai sisi terdekat kursor. Alasan perilaku tercatat di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: garis dan penanda memakai motif sketsa serta aksen yang sama, dengan ENERGY 1 / RHYTHM 2 / MOTION 1.
- Craftsmanship PASS dalam pengujian otomatis: 48 kombinasi geometri, drag melewati target, resize, Undo/Redo, dan penyimpanan hasil lulus. Rustfmt serta build debug diperiksa ulang. Pengujian menghindari isi kedua shape terhubung; belum mencakup pencarian jalur mengitari objek lain yang tidak terhubung atau shape yang sengaja ditumpuk.

## Pemeriksaan latar main putih

- Hard Gate PASS dalam pengujian otomatis: 59 tes lulus, termasuk frame responsif dan pemeriksaan kontras teks; tidak ada perubahan kontrol atau shortcut.
- Purpose-Gate PASS: main, padding, celah antarpanel, kertas Canvas, dan ekspor memakai putih #ffffff sesuai permintaan. Dasar grid papan Notes, Checklist, dan Kanban juga putih, termasuk grid yang digambar ulang di papan Notes, sehingga menyatu dengan padding main. Grid Canvas hanya digambar di area gambar tengah sehingga tidak bocor di sela panel; aturan tercatat di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: motif kertas berkotak dan tulisan tangan tetap mengikuti ENERGY 1 / RHYTHM 2 / MOTION 1.
- Craftsmanship PASS dalam pengujian otomatis: semua mode kosong/berisi, tata letak panel, serta ekspor SVG/PNG tetap lulus. Rustfmt, build debug, dan pemeriksaan whitespace berhasil. Verifikasi native tetap terbatas seperti dicatat di atas.

## Pemeriksaan header sesuai referensi editor diagram

- Hard Gate PASS dalam pengujian otomatis: 57 tes lulus. Pada lebar 380/640/800/940/1280 px seluruh menu dan aksi toolbar berada dalam header; judul berada di atas menu, menu di atas toolbar. Kontras teks tetap lulus. Computer Use dicoba ulang tetapi native pipe masih tidak tersedia; screenshot dan interaksi jendela Windows belum diverifikasi langsung.
- Purpose-Gate PASS: tiga baris memisahkan identitas/judul, menu teks, dan aksi toolbar sesuai referensi pengguna. Ikon tanpa kotak dan pemisah antarkelompok meniru susunan editor diagram dengan logo N yang sama. Aturan tercatat di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: main tetap putih dan memakai motif kertas/tulisan tangan; header abu terang, judul 18 px, dan mode aktif bold mempertahankan ENERGY 1 / RHYTHM 2 / MOTION 1.
- Craftsmanship PASS dalam pengujian otomatis: event pointer membuka menu File dan membuat dokumen, mengedit judul, menggambar shape, menjalankan Undo/Redo dan zoom dari toolbar, serta membuka konfirmasi hapus tanpa langsung menghapus dokumen. Tes close/autosave dan resize tetap lulus. Menu File memanggil ekspor dan dialog workspace yang sudah ada; Edit terhubung ke history Canvas/Notes, Tampilan ke mode/zoom, Workspace ke lokasi lokal, Bantuan menampilkan shortcut. Rustfmt serta build debug diperiksa ulang.

## Pemeriksaan arah panah siku

- Hard Gate PASS dalam pengujian otomatis: 59 tes lulus, termasuk arah segmen awal/akhir dan pencegahan perpotongan isi shape terhubung. Tidak ada perubahan kontrol, warna teks, atau shortcut.
- Purpose-Gate PASS: jalur mengikuti sisi sambungan alih-alih selalu berbelok pada koordinat x tengah. Ruang sebelum belokan dan panjang segmen yang seimbang mencegah ujung menyusuri tepi shape. Aturan tercatat di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: arah panah yang jelas dan belokan ringkas mempertahankan motif sketsa, ENERGY 1 / RHYTHM 2 / MOTION 1.
- Craftsmanship PASS: 192 kombinasi sisi sambungan/posisi/kotak-oval-belah ketupat, jarak sempit, handle pada jalur, pengaturan siku manual, dan roundtrip JSON lulus. Kasus dari screenshot diperiksa secara visual melalui hasil ekspor `test-data/elbow-routing/elbow-routing.png`: panah masuk vertikal dari bawah dan berbelok di ruang antarshape. Ini ekspor Canvas, bukan screenshot jendela Windows. Rustfmt serta build debug diperiksa ulang; batas verifikasi native tetap dicatat di atas.

## Pemeriksaan pengaturan panah sesuai seleksi

- Hard Gate PASS dalam pengujian otomatis: 59 tes lulus; tidak ada kontrol, shortcut, atau warna baru.
- Purpose-Gate PASS: pilihan lurus/siku/lengkung hanya terlihat untuk alat Panah atau seleksi yang seluruhnya panah, sehingga shape tidak memuat pengaturan yang tidak berlaku. Aturan tercatat di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: panel tetap memakai susunan dan motif yang sama dengan ENERGY 1 / RHYTHM 2 / MOTION 1.
- Craftsmanship PASS dalam pengujian otomatis: regresi preview panah pada zoom 50/100/200% juga memeriksa kontrol tersembunyi untuk kotak, garis biasa, dan seleksi campuran, terlihat saat alat Panah aktif, serta menampilkan Panah siku saat panah terpilih memang siku tanpa mengubah data. Rustfmt dan build debug berhasil; batas verifikasi native tetap dicatat di atas.

## Pemeriksaan perbaikan layout header

- Hard Gate PASS dalam pengujian otomatis: 59 tes lulus, termasuk menu, history, zoom, autosave sebelum tutup, serta resize jendela. Tidak ada perubahan penyimpanan atau isi dokumen.
- Purpose-Gate PASS: ruang judul mempertahankan lebarnya agar kontrol jendela tetap di pojok kanan; padding menu mengembalikan jarak baca. Dropdown turun baris secara utuh dan status menu disembunyikan pada lebar kecil. Arahan tercatat di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: susunan tiga baris, tombol polos, warna netral, serta ENERGY 1 / RHYTHM 2 / MOTION 1 tetap mengikuti referensi.
- Craftsmanship PASS dalam pengujian otomatis: regresi awal mereproduksi tombol Tutup pada x=560–598 alih-alih pojok kanan di lebar 1280 px. Tes sekarang memeriksa tepi kanan kontrol jendela, padding menu, batas header dan jarak judul pada lebar 380/640/800/940/1280/1920 px. Rustfmt dan build development diperiksa. Ini pemeriksaan geometri frame egui dan event input; screenshot jendela Windows belum diverifikasi langsung karena batas native yang dicatat di atas.

## Pemeriksaan ukuran logo header

- Hard Gate PASS dalam pengujian otomatis: 59 tes lulus, termasuk menu, edit judul, tombol jendela, serta layout responsif.
- Purpose-Gate PASS: logo lama ditampilkan 56 × 56 px melintasi area judul dan menu untuk mengisi ruang bawah; indent menu mengikuti lebar logo. Alasan tercatat di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: monogram N, palet, dan ENERGY 1 / RHYTHM 2 / MOTION 1 tetap sama; tidak ada aset atau dekorasi baru.
- Craftsmanship PASS dalam pengujian otomatis: batas header, padding menu, serta kontrol jendela di pojok kanan tetap lulus pada lebar 380–1920 px yang diuji. Rustfmt dan build development berhasil. Batas pemeriksaan screenshot native tetap seperti dicatat di atas.

## Pemeriksaan perataan judul Notes dan Kanban

- Hard Gate PASS dalam pengujian otomatis: 59 tes lulus, termasuk pengeditan judul, navigasi, resize, dan kontrol jendela.
- Purpose-Gate PASS: label judul statis memakai layout rata kiri tanpa pembungkus yang memusatkan isinya; judul editable juga menetapkan rata kiri. Ini menyamakan posisi judul papan Notes dan mode tanpa dokumen dengan judul Kanban/Canvas. Aturan tercatat di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: font, logo, warna, serta ENERGY 1 / RHYTHM 2 / MOTION 1 tetap sama.
- Craftsmanship PASS dalam pengujian otomatis: tes frame responsif memeriksa posisi teks Papan catatan yang benar-benar digambar terhadap sisi kiri menu pada lebar 380/640/800/940/1280/1920 px. Rustfmt dan build development berhasil. Batas verifikasi screenshot native tetap seperti dicatat di atas.

## Pemeriksaan header lebih compact

- Hard Gate PASS dalam pengujian otomatis: 59 tes lulus, termasuk klik menu, judul editable, kontrol jendela, dan resize.
- Purpose-Gate PASS: padding vertikal menjadi 2 px, baris judul/kontrol 28 px, menu 20 px, dan jarak antarkedua baris dihapus. Logo 44 px mengikuti tinggi area identitas yang lebih padat sesuai arahan terbaru di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: font judul 18 px, monogram N, palet, serta ENERGY 1 / RHYTHM 2 / MOTION 1 tetap sama.
- Craftsmanship PASS dalam pengujian otomatis: tes perataan judul, batas header/menu, posisi tombol jendela di kanan, serta event tutup/drag/resize tetap lulus. Rustfmt dan build development berhasil. Batas verifikasi native tetap seperti dicatat di atas.

## Pemeriksaan papan Checklist seperti Notes

- Hard Gate PASS dalam pengujian otomatis: 60 tes lulus. Tidak ada format penyimpanan baru; dokumen Checklist lama tetap memakai data tugas yang sama.
- Purpose-Gate PASS: komponen papan Notes dipakai bersama untuk Checklist agar ukuran kartu, pencarian, drag, warna, dan menu konteks konsisten. Cuplikan tugas dan progres menjelaskan isi daftar sebelum dibuka. Arahan tercatat di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: enam pastel dengan teks hitam, judul tulisan tangan, grid, serta ENERGY 1 / RHYTHM 2 / MOTION 1 mengikuti Notes. Tidak ada aset baru.
- Craftsmanship PASS dalam pengujian otomatis: event klik membuka kartu yang benar, mengubah centang, kembali ke papan, memilih warna Sage, mencari judul tanpa mengubah posisi asli, merapikan tanpa menyentuh posisi Notes, menyeret kartu, serta membuat daftar baru. Tes pemuatan ulang file memeriksa tugas, warna, dan posisi; menu konteks kedua jenis papan menghapus dokumen yang diklik hanya setelah konfirmasi. Papan diperiksa pada lebar 380/640/1280 px dan Ctrl+scroll diuji di papan maupun daftar penuh. Rustfmt, build development, dan whitespace diperiksa; batas verifikasi screenshot native tetap seperti dicatat di atas.

## Pemeriksaan penghapusan menu Kanban

- Hard Gate PASS dalam pengujian otomatis: 60 tes lulus; format dan data Kanban lama tetap didukung oleh penyimpanan.
- Purpose-Gate PASS: tab navigasi dan menu Tampilan memakai daftar mode yang sama, berisi Canvas, Notes, dan Checklist. Dokumen Kanban dilewati saat pemilihan awal agar mode yang dihapus tidak muncul kembali. Arahan tercatat di DESIGN.md.
- Liveliness PASS pada lingkup perubahan: susunan header lebih pendek dengan font, warna, dan ENERGY 1 / RHYTHM 2 / MOTION 1 yang sama.
- Craftsmanship PASS dalam pengujian otomatis: tes frame responsif memastikan ketiga tab aktif tersedia dan label Kanban tidak tampil. Tes penyimpanan lintas mode tetap lulus. Rustfmt, build development, dan whitespace diperiksa; batas verifikasi screenshot native tetap seperti dicatat di atas.

## Pemeriksaan bentuk kontrol Gaya

- Hard Gate PASS: 60 tes lulus; perilaku pemilihan tinta/isi dan pengaturan objek tetap sama.
- Purpose-Gate PASS: swatch tinta/isi memakai ukuran persegi 28 px dan radius 4 px agar tidak lonjong; aturan tercatat di DESIGN.md.
- Liveliness PASS: palet dan penanda pilihan tetap sama, dengan ENERGY 1 / RHYTHM 2 / MOTION 1.
- Craftsmanship PASS dalam pengujian otomatis: layout panel responsif dan pengaturan objek tetap lulus. Gaya sudut dibatasi pada bagian Gaya. Rustfmt, build development, dan whitespace diperiksa; batas verifikasi screenshot native tetap seperti dicatat di atas.

## Pemeriksaan celah putih editor Notes

- Hard Gate PASS dalam pengujian otomatis: 60 tes lulus, termasuk editor, gambar, zoom, navigasi, dan kontras enam warna catatan.
- Purpose-Gate PASS: saat editor Notes dibuka, padding putih luar main dihapus, dasar main mengikuti warna catatan, dan grid di belakang editor tidak digambar. Sudut Notes mengikuti main 6 px agar bingkai menyatu; padding tulisan 16 px tetap ada. Arahan tercatat di DESIGN.md.
- Liveliness PASS: palet pastel, header senada dengan teks putih, tulisan tangan, serta ENERGY 1 / RHYTHM 2 / MOTION 1 tetap sama.
- Craftsmanship PASS dalam pengujian otomatis: tes enam warna pada lebar 380 dan 1280 px memastikan bidang catatan mencapai tepi kiri/kanan/bawah bingkai main, header tetap berada di dalam catatan, dan teks memakai warna yang benar. Rustfmt, build development, dan whitespace diperiksa; batas verifikasi screenshot native tetap seperti dicatat di atas.

## Pemeriksaan pencarian Dokumen tetap terbuka

- Hard Gate PASS dalam pengujian otomatis: 61 tes lulus; penyimpanan, navigasi, dan isi dokumen tidak berubah.
- Purpose-Gate PASS: dropdown Dokumen hanya menutup lewat pemilihan dokumen, klik luar, atau Esc; klik kolom pencarian tidak lagi dianggap pemilihan. Perilaku tercatat di DESIGN.md.
- Liveliness PASS: tampilan dan ENERGY 1 / RHYTHM 2 / MOTION 1 tetap sama, tanpa elemen visual baru.
- Craftsmanship PASS dalam pengujian otomatis: event pointer membuka dropdown dan mengklik pencarian, mengetik pencarian Unicode tanpa membedakan huruf besar/kecil, memilih dokumen yang cocok, lalu memeriksa penutupan setelah memilih, Esc, dan klik luar. Alur dijalankan di Canvas/Notes/Checklist pada lebar 380 dan 1280 px. Rustfmt, build development, dan whitespace diperiksa; batas verifikasi screenshot native tetap seperti dicatat di atas.

Belum ada impor PDF/PPT/Word atau `.excalidraw`, rotasi, deteksi khusus stylus, atau kolaborasi. Transisi kartu Notes/Checklist membuka editor penuh tanpa animasi zoom. Batas fitur lain tercatat di README.
