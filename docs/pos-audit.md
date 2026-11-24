# Audit fungsi POS — 24 November 2026

Proyek ini memiliki alur inti POS yang berjalan pada database: transaksi penjualan, penerimaan barang, stok per batch, kredit, pembayaran, retur, pending, dan struk. Pemeriksaan tidak hanya melihat keberadaan menu. Validasi dilakukan pada logika transaksi dan MariaDB sementara, tanpa memakai database apotek asli.

## Hasil pemeriksaan

| Area | Perilaku yang diperiksa | Hasil dan batas |
| --- | --- | --- |
| Penjualan | Total barang, diskon faktur, ongkos, pembayaran tunai/kredit, dan kembalian | Validasi menolak keranjang kosong, angka/konversi tidak valid, total yang tidak sesuai, serta uang tunai yang belum cukup. |
| Stok | Konversi satuan, FEFO, batch kedaluwarsa, stok tidak cukup, dan rollback | Penjualan mengurangi stok dasar saat disimpan. Dua kasir yang masing-masing mencoba membeli 4 dari stok 5 menghasilkan satu penjualan dan sisa stok 1. |
| Pending | Penyimpanan berulang/bersamaan, perubahan draf, dan penyelesaian transaksi | Enam penyimpanan bersamaan menghasilkan satu draf. Pending boleh belum dibayar dan tidak mengurangi stok. Faktur, stok, serta konsumsi pending disimpan secara atomik saat checkout. |
| Cetak | F3 simpan, F4 simpan/cetak, dan pengulangan setelah printer gagal | Pengulangan memakai faktur tersimpan tanpa transaksi atau pengurangan stok baru. Alur kegagalan diuji dengan simulasi printer; perangkat fisik belum diuji. |
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

## Memperbarui instalasi

Jalankan versi baru pada server terlebih dahulu untuk menambahkan kolom snapshot obat, lalu perbarui semua client. Migrasi tidak mengisi ulang data contoh atau pengaturan. Snapshot hanya tersedia untuk transaksi baru; identitas historis yang tidak pernah disimpan tidak dapat direkonstruksi. Login/sidebar otomatis membaca nama apotek yang sudah tersimpan pada database.
