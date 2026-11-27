# Audit fungsi POS — 24 November 2026

Proyek ini memiliki alur inti POS yang berjalan pada database: transaksi penjualan, penerimaan barang, stok per batch, kredit, pembayaran, retur, pending, dan struk. Pemeriksaan tidak hanya melihat keberadaan menu. Validasi dilakukan pada logika transaksi dan MariaDB sementara, tanpa memakai database apotek asli.

## Hasil pemeriksaan

| Area | Perilaku yang diperiksa | Hasil dan batas |
| --- | --- | --- |
| Penjualan | Total barang, diskon faktur, ongkos, pembayaran tunai/kredit, dan kembalian | Validasi menolak keranjang kosong, angka/konversi tidak valid, total yang tidak sesuai, serta uang tunai yang belum cukup. |
| Stok | Konversi satuan, FEFO, batch kedaluwarsa, stok tidak cukup, dan rollback | Penjualan mengurangi stok dasar saat disimpan. Dua kasir yang masing-masing mencoba membeli 4 dari stok 5 menghasilkan satu penjualan dan sisa stok 1. |
| Pending | Penyimpanan berulang/bersamaan, perubahan draf, dan penyelesaian transaksi | Enam penyimpanan bersamaan menghasilkan satu draf. Pending boleh belum dibayar dan tidak mengurangi stok. Faktur, stok, serta konsumsi pending disimpan secara atomik saat checkout. |
| Cetak | Simpan dahulu, pilihan cetak setelah transaksi berhasil, dan pengulangan dari riwayat | Pengulangan memakai faktur tersimpan tanpa transaksi atau pengurangan stok baru. Alur kegagalan diuji dengan simulasi printer; perangkat fisik belum diuji. |
| Kredit | Pelanggan/jatuh tempo, pembayaran sebagian, sisa tagihan, dan retur | Struk HTML/ESC/POS sekarang memuat sisa serta jatuh tempo. Faktur yang diretur utuh tidak menampilkan saldo yang masih harus dibayar. |
| Pembelian dan retur | Stok masuk, biaya per satuan dasar, retur sesuai alokasi batch, dan penolakan retur ganda | Retur penjualan mengembalikan batch asal. Retur pembelian ditolak jika stok batch sudah tidak cukup. Retur sebagian belum tersedia. |
| Riwayat obat | Identitas obat pada faktur setelah nama/barcode master berubah | Faktur baru menyimpan snapshot nama/kode dari master yang dikunci saat checkout. Struk langsung dan cetak ulang memakai data tersimpan. Faktur lama tanpa snapshot memakai master terbaru sebagai fallback. |
| Impor/ekspor | Barcode bernol awal, harga desimal, satuan/konversi, dan impor atomik | `tablet` dan `Tablet` diperlakukan sama. Satuan beli yang sama dengan satuan dasar harus memiliki isi 1. Membaca Excel dan menulis database berjalan di worker; putusnya worker mengaktifkan kembali kontrol. Impor obat mencakup master, bukan stok/batch historis atau relasi suplier. |
| Identitas apotek | Nama pada sidebar, login, judul jendela, struk, dan laporan | Semua memakai `store_name` dari Pengaturan. Menyimpan nama memperbarui sidebar/judul di sesi yang sama. Login memuat ulang nama saat ditampilkan kembali. Nama kosong memakai fallback `Apotek`. |
| Akses operator | Navigasi Admin, Gudang, dan Kasir | Pembatasan menu diuji. Koneksi LAN tetap merupakan akses database bersama, bukan layanan backend dengan otorisasi per operator. |

## Perbaikan dari audit

1. Menghapus nama aplikasi yang ditulis tetap pada sidebar/login dan menghubungkan identitas ke Pengaturan.
2. Mempertahankan pecahan rupiah pada struk thermal; sebelumnya harga dan total dibulatkan ke bilangan bulat saat dicetak.
3. Menampilkan sisa/jatuh tempo pada struk kredit, termasuk pembatalan saldo setelah retur utuh.
4. Menyimpan snapshot nama/barcode obat agar faktur baru tidak berubah saat master diedit. Checkout mengembalikan data faktur tersimpan untuk struk langsung.
5. Menyamakan huruf besar/kecil satuan Excel, menolak konversi dasar yang keliru, memindahkan pembacaan berkas ke worker, dan menangani worker yang terputus.
6. Membatasi lebar struk HTML ke area cetak halaman agar margin tidak membuat dokumen melebihi lebar kertas.

## Validasi

- `cargo build --offline`: lulus.
- `cargo test --offline`: 10 lulus; satu uji database dijalankan terpisah.
- `python3 tests/database_workflow.py`: lulus pada MariaDB sementara dengan socket di `/tmp`, tanpa TCP. Mencakup migrasi skema lama, cadangan/pemulihan, pengaturan nama, FEFO, pending, kredit, retur, impor, checkout/cetak ulang, snapshot obat, dan penjualan bersamaan.

Uji interaksi GUI untuk perubahan terbaru belum dijalankan karena automatic approval review sebelumnya menolak eksekusi Xvfb. Tampilan login/nama panjang, pembaruan label saat klik Simpan Pengaturan, pemulihan UI saat worker berhenti, printer Windows fisik, scanner fisik, dan koneksi dua PC tetap perlu diverifikasi pada perangkat target. Pengujian receipt generator membuktikan isi dokumen, bukan keluarnya kertas pada printer.

## Batas produk

- Barcode masih satu kode per obat. Scan menambah satu satuan dasar; barcode berbeda untuk tiap kemasan/satuan belum didukung.
- Retur berlaku untuk seluruh faktur. Riwayat stok/batch dan relasi suplier tidak dipulihkan oleh impor master obat.
- Dashboard/laporan memuat omzet dan laba kotor. Biaya, cashflow, rekonsiliasi kas/shift, serta laba bersih belum tersedia.
- Laporan barang mengalokasikan diskon secara proporsional dan membulatkan per baris; hasil alokasi dapat memiliki selisih pembulatan terhadap total invoice.
- Master obat memiliki navigasi 100 per halaman. Pilihan obat dibatasi 200 hasil; laporan umum 500 baris, laporan rincian barang 10.000 baris per periode.

Alur dan batas ini cukup untuk POS dasar apotek; belum merupakan sistem akuntansi atau seluruh fitur iB. [Pemetaan IB](ib-reference.md) tetap menjadi acuan, dengan kebutuhan pengguna menghilangkan dokter dari antarmuka.

## Penyesuaian barcode — 25 November 2026

Mengikuti gambar iB nomor 8, antrean label menyediakan Tambah obat dengan pemilih banyak obat, Pilih semua hasil pencarian, serta pratinjau lembar putih berbingkai di dalam aplikasi dengan zoom 75–200%. Alur utama adalah Lihat label, lalu Cetak dari pratinjau memakai dialog printer native. Ekspor HTML barcode dan pembentuk HTML label dihapus. Toolbar master obat memuat tiga tombol: Tambah obat, Cetak barcode, dan Lainnya. Jumlah tetap dapat diterapkan pada beberapa baris. Penambahan obat duplikat diabaikan dan perubahan yang melewati batas 200 obat atau 1.000 label ditolak sebelum mengubah antrean. Pratinjau dan cetak native memakai validasi ukuran/DPI yang sama. Harga desimal tetap dicetak.

Pratinjau struk pada kasir dan riwayat juga berada di dalam aplikasi. Draf ditandai dan belum dapat dicetak. Struk final memakai printer ESC/POS yang dikonfigurasi di Windows atau dialog printer native pada platform lain/tanpa printer langsung. Cetak native memakai ukuran sebenarnya, membagi dokumen ke halaman, serta menolak area cetak yang terlalu sempit. Pembatalan/kegagalan cetak tetap memakai alur checkout yang menyimpan faktur terlebih dahulu dan dapat mencetak ulang tanpa transaksi baru.

Susunan struk dirapikan menjadi nota thermal dengan identitas tengah, metadata ringkas, rincian barang, nominal rata kanan, pemisah putus-putus, TOTAL tebal, dan catatan penutup. Pratinjau dan ESC/POS memakai pembentuk baris yang sama. Diskon barang kini ikut pada ESC/POS; baris panjang dibungkus tanpa menyatukan nominal dengan rincian barang. Diskon/ongkos nol tidak ditampilkan. Contoh visual di `docs/examples` dibuat dari data layout uji, sementara pemeriksaan GUI tetap terpisah.

Log pengguna kemudian menunjukkan panic saat menekan Tambah obat: FLTK menolak karakter format NUL pada MultiBrowser. Pemilih sekarang memakai karakter kontrol ASCII non-NUL yang tidak muncul pada awal kode obat yang valid. Skrip GUI diperluas untuk membuka pemilih, mencari satu obat tertentu, menambahkannya ke antrean, lalu memeriksa kodenya pada pratinjau sebelum mencoba dialog printer. Pemeriksaan interaksi ini masih menunggu akses eksekusi GUI.

Pilihan tabel kini memakai satu penyimpanan baris terpilih dan titik awal Shift. Klik biasa mengganti pilihan; Shift memilih rentang dari klik sebelumnya; Ctrl menambah atau membuang baris terpisah. Penambahan obat mempertahankan pilihan lama; penggantian susunan daftar mereset titik awal agar tidak memakai posisi lama. Ubah jumlah dan hapus label memakai indeks baris yang benar-benar dipilih. Menu master obat juga mempertahankan pilihan Ctrl saat klik kanan pada baris terpilih; cetak barcode dan nonaktifkan tidak mengambil obat di antara dua pilihan terpisah.

- `cargo build --offline`: lulus.
- `cargo test --offline`: 16 lulus; uji database tetap terpisah. Uji mencakup titik awal/rentang Shift, pilihan/toggle Ctrl, reset pilihan, perubahan jumlah hanya pada pilihan terpisah, isi struk native/ESC-POS, snapshot nama, diskon barang dalam rupiah, harga desimal, teks panjang pada 32/48 kolom, kredit/retur, dan pembagian 1.000 label ke halaman. Perubahan cetak ini tidak mengubah database atau mutasi stok.
- `python3 tests/barcode_decode.py`: 11 kode, tiga preset ukuran/DPI, dan enam label dari tiga obat dalam satu dokumen terbaca persis oleh pembaca independen.
- Skrip GUI diperbarui untuk pemilih banyak obat, pratinjau struk/barcode native, perubahan jumlah, dialog cetak, dan penambahan ulang tanpa duplikasi. Uji GUI belum dijalankan karena automatic approval review sebelumnya menolak Xvfb; screenshot lama belum diperbarui. Pengujian layout tidak membuktikan dialog driver maupun hasil fisik cetak vektor.

## Pengaturan dan akun karyawan — 25 November 2026

Pengaturan menyediakan tombol Akun karyawan yang membuka pengelolaan akun Operator yang sama. Admin dapat menambah akun melalui Tambah akun, dengan username, nama lengkap, pilihan hak akses Kasir/Gudang/Admin, pilihan status Aktif/Nonaktif, dan password. Status pada daftar ditampilkan sebagai teks; edit mengembalikannya ke nilai database yang sama. Penanganan password bcrypt dan perlindungan admin terakhir tetap memakai alur yang ada.

Header/footer memakai input multiline native. Enter membuat baris baru tanpa tampilan karakter kontrol `^J`; pengguna tidak perlu mengetik kode pemformatan. Nilai disimpan sebagai teks asli. Baris kosong dipertahankan sampai ke pratinjau dan ESC/POS, dan karakter seperti `<3` tetap menjadi teks biasa. Susunan formulir disesuaikan agar kotak teks tidak bertumpuk dengan pilihan kertas dan tombol simpan.

Build offline dan 16 uji unit lulus; pemeriksaan tambahan pada uji struk memverifikasi nilai footer, newline, baris kosong, dan teks literal. Skrip GUI diperbarui untuk akses akun dari Pengaturan serta pengetikan dua baris footer. Uji GUI masih belum dijalankan karena penolakan automatic approval review terhadap Xvfb sebelumnya. Tidak ada perubahan skema atau pembuatan akun pada database apotek asli selama pengembangan.

## Ukuran jendela — 25 November 2026

Jendela utama dan pratinjau struk/barcode mendukung resize serta maximize melalui kontrol sistem. **F11** mengaktifkan/menonaktifkan fullscreen; **Esc** keluar dari fullscreen. Menu **Akun → Layar penuh (F11)** juga tersedia. Sidebar tetap selebar 188 piksel, area konten mengikuti jendela, dan lebar kolom tabel kembali tanpa pergeseran pembulatan saat ukuran dipulihkan. Dashboard menyesuaikan lebar konten; pratinjau menjaga ukuran dokumen dan skala cetak.

Build offline dan 18 uji unit lulus, termasuk tombol fullscreen, pemisahan shortcut POS, pemulihan lebar kolom, dan perubahan kolom pengguna. Skrip sidebar diperluas untuk resize hints, perubahan ukuran, F11, dan Esc. Pemeriksaan GUI dan tombol maximize pada perangkat target belum dijalankan karena penolakan Xvfb sebelumnya.

## Alur penjualan dengan keyboard — 26 November 2026

**F1** pada daftar Penjualan membuka kasir. **F1** di kasir membuka pemilih obat dari master. Ketik nama atau barcode lalu tekan **Enter** untuk mencari; gunakan panah untuk memilih obat, **Enter** untuk mengisi jumlah, **Tab** untuk berpindah ke harga/diskon, lalu **Enter** untuk menambah ke keranjang. Satuan otomatis mengikuti master. Dialog mengambil harga, satuan, stok, dan ED dari master; penjualan tidak meminta input batch atau ED baru. **Esc** menutup pemilih dan mengembalikan fokus ke kolom scanner. Scan barcode yang cocok tetap langsung menambah satu satuan dasar; scan berulang menggabungkan jumlah dan tetap memeriksa stok layak jual.

**F2** memfokuskan pembayaran dan memilih nilai yang ada sehingga kasir dapat langsung mengetik nominal pengganti. **Enter** pada pembayaran atau **F3** menyimpan transaksi, kemudian menanyakan apakah struk perlu dicetak. Pilihan tidak mencetak tetap mempertahankan faktur dan mutasi stok yang sudah tersimpan. Validasi atau penyimpanan yang gagal tidak boleh membuka pilihan cetak. Tombol Lihat struk dan Cetak terpisah serta shortcut F4 pada kasir dihapus. Pratinjau dan cetak ulang tersedia dari riwayat penjualan tanpa menyimpan transaksi atau mengurangi stok lagi.

Skrip `tests/pos_workflow.py` disesuaikan untuk F1 pada daftar dan kasir, pencarian/pemilihan/jumlah/satuan dengan keyboard, Esc kembali ke scanner, scan berulang, penggantian nominal melalui F2, serta penolakan checkout di mode contoh. Mode `--preview` sengaja tidak menyimpan transaksi; pemeriksaan urutan simpan dan cetak memakai uji Rust headless. Skrip tetap memeriksa struk tersimpan melalui riwayat.

Build offline, 18 uji unit, dan pemeriksaan sintaks skrip Python lulus; satu uji database tetap diabaikan pada suite unit. Uji checkout memeriksa urutan simpan, pilihan cetak, cetak gagal, dan cetak ulang tanpa penyimpanan baru. Pemeriksaan GUI terbaru belum dijalankan karena automatic approval review sebelumnya menolak eksekusi Xvfb. Skrip GUI yang diperbarui belum menjadi bukti interaksi keyboard pada perangkat target. Tidak ada pengujian pada database apotek asli.

Log pengguna kemudian menunjukkan bahwa F1 memicu panic saat membuat pemilih obat. Konfigurasi lebar kolom browser menyertakan nilai `0`, padahal binding FLTK menambahkan terminator tersebut sendiri dan menolak `0` di dalam daftar lebar. Nilai tambahan dihapus; empat lebar tetap memisahkan kode, nama, stok, dan satuan, sedangkan ED memakai sisa lebar. Penjualan dan pembelian memakai konfigurasi pemilih yang sama.

Uji regresi membuat browser native dengan konfigurasi produksi tanpa menampilkan jendela atau menambahkan baris yang memerlukan metrik font. Uji tersebut dan seluruh suite berjalan tanpa variabel `DISPLAY`: 19 uji lulus, satu uji database diabaikan. Build offline juga lulus. Pemeriksaan ini memverifikasi penerimaan konfigurasi oleh FLTK; interaksi popup lengkap tetap belum diuji lewat GUI.

Laporan pengguna berikutnya menunjukkan bahwa popup terbuka dengan fokus di pencarian, sehingga panah tidak memilih baris. Enter pada pencarian kosong juga terserap oleh FLTK karena trigger sebelumnya hanya memanggil callback bila teks berubah. Pencarian kini memakai `EnterKeyAlways`; panah dari pencarian berpindah ke daftar dan memilih baris. Teks yang baru diketik dicari terlebih dahulu. Enter tanpa perubahan pencarian menuju jumlah obat yang disorot. Enter pada daftar tetap menuju jumlah, dan Enter pada jumlah/harga/diskon menambah ke keranjang.

Uji konfigurasi native diperluas untuk pencarian kosong maupun terisi dengan status teks belum berubah, dan seluruh widget native diuji pada satu thread sesuai aturan FLTK. Build serta 19 uji headless lulus; satu uji database diabaikan. Skrip GUI ditambah untuk panah langsung setelah F1, Enter dari daftar, dan Enter pada pencarian kosong. Sintaks skrip lulus; interaksi GUI belum dijalankan karena penolakan Xvfb sebelumnya.

## Satuan transaksi terkunci — 27 November 2026

Satuan pada baris kasir dan popup pemilihan obat ditampilkan sebagai teks terkunci. Penjualan mengambil satuan dasar dari master; pembelian mengambil satuan beli, dengan fallback ke satuan dasar untuk data lama yang belum mengisi satuan beli. Tab melewati satuan langsung ke harga/diskon. Pemilihan manual dan scanner memakai pemetaan satuan master yang sama; konversi dan harga mengikuti data tersebut. Satuan/konversi master yang tidak valid ditolak dengan pesan untuk memperbaiki Master data obat.

Build offline dan 20 uji headless lulus; satu uji database diabaikan. Uji memeriksa pemetaan satuan dasar/satuan beli walaupun urutan daftar berubah, konversi pembelian, penolakan master tidak valid, dan field satuan yang tidak menerima fokus. Skrip GUI diperbarui untuk memastikan Tab/panah tidak mengubah satuan; interaksi GUI lengkap belum dijalankan karena penolakan Xvfb sebelumnya.

## Format nominal rupiah — 27 November 2026

Kolom Bayar, harga obat di kasir/pemilih/master, diskon faktur dalam rupiah, dan ongkos memakai pemisah ribuan otomatis. `110000` tampil sebagai `110.000`; pecahan memakai koma, misalnya `1.200,50`. Tanda titik merupakan pemisah ribuan pada kolom ini. Nilai program dari master dan pending diformat sebelum ditampilkan. Pembayaran hutang/piutang juga menerima format rupiah yang sama.

Formatter mempertahankan posisi kursor, pilihan F2/Ctrl+A, dan pecahan yang sedang diketik. Backspace/Delete di dekat pemisah ribuan menghapus digit yang dituju. Perhitungan membaca nilai numerik dari format lokal, sedangkan validasi angka internal, Excel, dan database tetap memakai angka desimal biasa. Angka negatif, bukan angka, pecahan lebih dari dua digit, serta nominal di luar batas ditolak.

Build offline dan 22 uji headless lulus; satu uji database diabaikan. Uji mencakup nominal/cents, nilai kembalian, perbedaan parser tampilan dan angka internal, pemetaan kursor, pilihan teks, penghapusan digit, pecahan sementara, dan konfigurasi field native. Skrip GUI ditambah untuk input nominal mentah/berpemisah, F2 mengganti nominal, dan Backspace dekat titik. Interaksi GUI lengkap belum dijalankan karena penolakan Xvfb sebelumnya.

## Escape dan fokus login — 27 November 2026

Esc di kasir kembali ke daftar Penjualan setelah proses simpan/cetak selesai. Keranjang yang belum disimpan dipertahankan pada halaman kasir selama sesi, tanpa dianggap sebagai pending yang tersimpan di database. Saat daftar Penjualan tampil, tabel mendapat fokus agar panah dapat langsung memilih faktur. Esc pada popup menutup popup tersebut; saat fullscreen aktif, Esc mengembalikan ukuran jendela terlebih dahulu.

Jendela utama, login, dan pengaturan awal mengabaikan alasan penutupan `Cancelled` yang dipakai FLTK untuk Escape. Penutupan melalui kontrol jendela tetap berjalan. Login langsung memfokuskan dan memilih username, termasuk setelah keluar akun. Tab atau Enter pada username menuju password; Enter biasa maupun Enter numpad pada password menjalankan login. Kesalahan atau kolom kosong mengembalikan fokus ke kolom yang perlu diisi.

Build dan 22 uji headless lulus; satu uji database diabaikan. Skrip GUI diperluas untuk Esc kembali ke riwayat, Esc berikutnya menjaga aplikasi terbuka, membuka ulang keranjang tanpa kehilangan item, dan Esc setelah keluar fullscreen. Interaksi login serta regresi GUI lengkap belum dijalankan karena penolakan Xvfb sebelumnya.

## Kembali otomatis setelah checkout — 27 November 2026

Setelah faktur berhasil disimpan dan proses pilihan/cetak selesai, aplikasi otomatis kembali ke daftar Penjualan. Memilih Tidak juga kembali ke daftar. Tabel riwayat dimuat ulang dan mendapat fokus, sehingga F1 membuka penjualan berikutnya. Penyimpanan yang gagal mempertahankan keranjang dan fokus pembayaran. Cetak yang tidak selesai menampilkan pesan bahwa faktur tetap tersimpan, lalu kembali ke riwayat setelah pesan ditutup; pencetakan ulang memakai faktur yang sama.

Build dan 22 uji headless lulus; satu uji database diabaikan. Uji checkout yang ada memverifikasi urutan simpan/pilihan cetak, penolakan pembayaran belum cukup, cetak gagal, dan cetak ulang tanpa penyimpanan ganda. Perpindahan halaman setelah checkout belum diuji melalui GUI karena penolakan Xvfb sebelumnya.

## Memperbarui instalasi

Jalankan versi baru pada server terlebih dahulu untuk menambahkan kolom snapshot obat, lalu perbarui semua client. Migrasi tidak mengisi ulang data contoh atau pengaturan. Snapshot hanya tersedia untuk transaksi baru; identitas historis yang tidak pernah disimpan tidak dapat direkonstruksi. Login/sidebar otomatis membaca nama apotek yang sudah tersimpan pada database.
