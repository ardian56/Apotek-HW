# Adaptasi referensi video ke desktop Windows

Referensi: https://www.youtube.com/watch?v=LvgDavrJEz0 — “Test Upload”, durasi 1 menit 57 detik.

Analisis visual menggunakan storyboard per detik, dengan gambar 277 × 180 piksel. Nama modul dan alur utama dapat diidentifikasi; detail teks kecil dan aturan bisnis yang tidak diperlihatkan membutuhkan referensi tambahan jika harus sama persis.

Pilihan pengguna: mempertahankan proyek desktop Windows `kasir_apotek`, memakai tampilan bersih dan ringan, serta menggunakan LAN dan cadangan lokal.

## Modul yang tersedia

| Modul | Perilaku desktop |
| --- | --- |
| Dashboard | Penjualan dan laba kotor harian, penjualan bulanan, modal stok, peringatan stok, grafik penjualan, obat terlaris, pelanggan teratas. Angka berasal dari database. |
| Penjualan | Pencarian nama/barcode, pilihan satuan, jumlah, harga, diskon, tunai/kredit, pelanggan, jatuh tempo, pending, dan struk thermal. |
| Pembelian | Faktur suplier, tanggal, satuan, batch, kedaluwarsa, diskon barang/faktur, pajak yang dapat diisi, tunai/kredit, pending, dan penambahan stok. |
| Hutang dan piutang | Daftar sisa tagihan, pembayaran sebagian/penuh, dan riwayat pembayaran. |
| Obat | Tambah/edit/nonaktifkan, multi-satuan, stok minimum, filter stok/kedaluwarsa, impor dan ekspor Excel. |
| Master pendukung | Tambah/edit/nonaktifkan suplier, pelanggan/pasien, dokter, serta operator dan role. Password operator baru memakai bcrypt. |
| Stok opname | Penyesuaian hasil hitung fisik, alasan, dan pencatatan kartu stok. Penyesuaian mengurangi beberapa batch bila perlu. |
| Retur | Retur satu faktur utuh, pengembalian stok sesuai alokasi batch, nominal uang kembali, dan pencegahan retur ganda. |
| Laporan | Penjualan, pembelian, ringkasan harian, persediaan, kartu stok, pembayaran, retur, obat terlaris, serta pelanggan teratas. Mendukung tanggal, pencarian, dan ekspor Excel. |
| Pengaturan | Identitas apotek, header/footer struk, printer, kertas 58/80 mm, serta cadangan SQL. |
| LAN | Mekanisme server/client dan pencarian server yang sudah ada tetap digunakan. Sinkronisasi cloud eksternal tidak diperlukan menurut pilihan pengguna. |

Navigasi utama memakai tab horizontal dan submenu sesuai kelompok. Warna terang, aksen teal, ruang antarkontrol, dan indikator fokus keyboard menggantikan sidebar hijau/kuning pada referensi. Tampilan menggunakan widget FLTK native.

## Pratinjau

Dari folder proyek:

```bash
cargo run -- --preview
```

Pratinjau tidak membutuhkan MariaDB dan menampilkan keadaan tanpa data. Penyimpanan yang memerlukan database dinonaktifkan. Jalankan aplikasi normal untuk login dan memakai database.

Saat memperbarui instalasi, jalankan aplikasi server lebih dahulu untuk migrasi skema tambahan. Migrasi tidak mengisi ulang data contoh maupun menimpa pengaturan pada setiap pembukaan aplikasi.

![Dashboard desktop](screenshots/dashboard.png)

![Kasir desktop](screenshots/kasir.png)

![Pembelian desktop](screenshots/pembelian.png)

## Pemeriksaan

- Build Linux dan uji validasi angka/tanggal lulus.
- Ekspor lalu impor master Excel mempertahankan barcode dengan nol di awal dan harga desimal.
- Uji MariaDB terisolasi lulus untuk migrasi berulang, login/password, FEFO, stok tidak cukup, batch kedaluwarsa, kredit, pembayaran, pending, retur, stok opname lintas batch, impor atomik, dan cadangan SQL.
- Cadangan SQL dipulihkan ke database uji kosong; pengaturan dan transaksi terverifikasi.
- Pratinjau pada layar virtual 1366 × 768 dapat membuka navigasi utama. Pengukuran awal RSS Linux sekitar 35 MiB, tanpa database dan tanpa data transaksi; angka ini bukan pengukuran Windows.

```bash
cargo test --offline
```

Uji database dijalankan terpisah, hanya pada MariaDB sementara dengan socket di `/tmp/apotek-test-db-*`:

```bash
APOTEK_TEST_SOCKET=/tmp/apotek-test-db-XXXX/mysql.sock cargo test --offline database_transaction_workflow -- --ignored
```

## Batas yang perlu diketahui

Retur saat ini berlaku untuk satu faktur utuh. Retur sebagian membutuhkan aturan dan UI per item. Pencarian obat menampilkan paling banyak 200 hasil; tabel laporan menampilkan paling banyak 500 baris per filter. Persempit pencarian atau periode untuk data lainnya. Impor Excel menambahkan master baru secara atomik dan tidak mengimpor stok/batch historis.

Build dan pengujian perangkat Windows, kompatibilitas Windows 7/32-bit, printer thermal nyata, dan koneksi dua PC LAN masih membutuhkan perangkat target. Pemeriksaan ini menggunakan Linux dan MariaDB lokal yang terisolasi.
