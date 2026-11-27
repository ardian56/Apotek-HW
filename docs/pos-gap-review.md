# Perbandingan POS dan audit penggunaan tanpa mouse

Tanggal: 27 November 2026. Kode yang diperiksa: `9b46d5c`.

Aplikasi sudah mempunyai fondasi POS apotek: penjualan tunai/kredit, rincian obat per faktur, stok batch dan ED, FEFO, konversi satuan, pembelian, pending, pembayaran piutang/hutang, retur, struk native, barcode massal, laporan, akun karyawan, serta cadangan lokal. Prioritas berikutnya adalah menyelesaikan akses keyboard, koreksi transaksi, dan beberapa kelemahan pada pencatatan stok.

Kebutuhan proyek tetap aplikasi desktop Windows, database LAN, tampilan ringan, dan cadangan lokal. Setiap fungsi harus dapat diselesaikan melalui keyboard. Keberadaan shortcut F1/F2/F3 saja belum membuktikan seluruh alur memenuhi kebutuhan tersebut.

Koreksi kebutuhan pengguna setelah audit: satuan saat input transaksi terkunci mengikuti master. Penjualan memakai satuan dasar dan pembelian memakai satuan beli; perubahan satuan dilakukan di Master data obat. Pengembangan edit item keranjang mengubah jumlah/harga/diskon sesuai hak akses, dengan satuan tetap otomatis dari master.

## Referensi yang diperiksa

- [Odoo 19 — Workflow POS](https://www.odoo.com/documentation/19.0/applications/sales/point_of_sale/use.html): perubahan jumlah/harga/diskon barang yang sudah masuk keranjang, riwayat pesanan, retur berdasarkan barang dan jumlah, kas masuk/keluar, serta penutupan kas dengan perbandingan kas yang diharapkan dan kas fisik.
- [Odoo 19 — Payment methods](https://www.odoo.com/documentation/19.0/applications/sales/point_of_sale/payment_methods.html): pencatatan metode tunai, kartu, QR, dan akun pelanggan secara terpisah.
- [ERPNext — POS Profile](https://docs.frappe.io/erpnext/pos-profile): metode pembayaran, aturan perubahan harga/diskon, pemeriksaan stok saat simpan, penanganan transaksi belum selesai, dan pengaturan cetak saat transaksi selesai.
- [W3C — Keyboard](https://www.w3.org/WAI/WCAG22/Understanding/keyboard.html): padanan keyboard untuk fungsi yang dijalankan melalui pointer. Prinsip interaksi ini dipakai untuk audit desktop; pemeriksaan ini tidak menyatakan sertifikasi WCAG.
- [Referensi desktop iB dari pengguna](ib-reference.md): baris input kasir, pilihan obat, batch/ED, barcode massal, dan pratinjau struk.

Odoo dan ERPNext dipakai sebagai pembanding perilaku operasional. Arsitektur web, fitur restoran, layanan cloud, loyalty, dan modul akuntansi lengkap tidak menjadi kebutuhan otomatis proyek ini.

## Kekurangan dan prioritas

| Prioritas | Bagian | Keadaan sekarang | Perbaikan yang dibutuhkan |
| --- | --- | --- | --- |
| P0 | Hak akses perubahan stok | Kasir ditolak oleh menu Stok opname, tetapi bisa menjalankan opname melalui klik kanan Master Obat. | Periksa peran pada semua jalur perubahan stok dan catat operator. |
| P1 | Keyboard menyeluruh | Kasir mempunyai shortcut; menu baris obat dan pilihan baris terpisah pada Grid belum mempunyai padanan keyboard. | Menu/Shift+F10, Ctrl+Space, urutan Tab yang jelas, serta pemulihan fokus yang konsisten. |
| P1 | Edit item dalam keranjang | Jumlah/harga/diskon pada baris input menyiapkan item baru; satuan otomatis dari master. Item yang sudah masuk hanya bisa dihapus dan ditambahkan ulang. | Ubah jumlah/harga/diskon item terpilih melalui keyboard, hitung ulang total, lalu periksa stok kembali. |
| P1 | Opname per batch/ED | Hanya jumlah total obat yang dimasukkan; pembagian selisih ke batch berdasarkan urutan ID. | Pilih batch yang dihitung, masukkan stok fisik batch tersebut, dan catat alasannya. |
| P1 | Validasi pembelian | Konversi dari draf dipakai tanpa pemeriksaan ulang terhadap master satuan. | Tolak atau minta koreksi jika satuan/konversi berubah sebelum pembelian disimpan. |
| P1 | Rekap/tutup kas | Laporan penjualan tersedia; saldo awal, kas masuk/keluar, kas fisik, dan selisih kas belum tersedia. | Rekap akhir hari atau per operator yang membandingkan kas seharusnya dengan hasil hitung fisik. |
| P1–P2 | Retur | Retur berlaku untuk seluruh faktur dan mengembalikan barang ke batch asal. | Retur per obat/jumlah. Jika menerima barang rusak, pisahkan dari stok layak jual. |
| P2 | Metode pembayaran | Hanya Tunai/Kredit; QRIS, transfer, dan kartu belum dicatat sebagai metode tersendiri. | Pencatatan metode dan nomor referensi secara lokal sesuai metode yang diterima apotek. Kredit tetap menyatakan tagihan. |
| P2 | Pengelolaan pending | Upsert tersedia; daftar dibatasi 100 tanpa halaman dan tidak menyediakan batal/hapus draf. | Cari/paginasi dan pembatalan draf dengan konfirmasi. |
| P2 | Cadangan dan pemulihan | Backup SQL tersedia; restore di aplikasi, jadwal, retensi, dan status cadangan terakhir belum tersedia. | Pemulihan yang terkontrol dan cadangan otomatis lokal dengan status yang mudah diperiksa. |

P0 perlu ditangani sebelum mengandalkan pembatasan peran untuk operasional. P1 menjadi prioritas pengembangan berikutnya. P2 mengikuti cara kerja dan metode pembayaran apotek.

## Bukti temuan penting

### Menu dan pilihan tabel belum lengkap melalui keyboard

[Menu master obat](../src/gui/views/master_obat.rs#L763) hanya dibuka pada `Event::Push` dengan tombol mouse 3. Tidak ada penanganan tombol Menu atau Shift+F10. Sebagian aksi mempunyai jalur lain: barcode dapat diubah melalui Edit obat, dan opname mempunyai halaman sendiri. Rincian Batch/ED obat terpilih tetap bergantung pada menu tersebut.

[Pilihan khusus Grid](../src/gui/views/components.rs#L369) diperbarui melalui klik/drag. FLTK Table menyediakan navigasi panah dan pemilihan rentang, tetapi belum ada Ctrl+Space untuk menambah atau membatalkan baris terpisah. Hal ini memengaruhi Master Obat dan antrean barcode. Pemilih barcode yang memakai MultiBrowser sudah mempunyai navigasi/pilihan native melalui panah, Space, dan Shift; masalahnya tidak berlaku untuk semua jenis tabel.

Sidebar, tombol, pilihan satuan, dialog berkas, dan beberapa dialog konfirmasi mempunyai jalur native Tab/Shift+Tab/Space/panah. Tidak adanya shortcut khusus tidak otomatis berarti sebuah fitur membutuhkan mouse. Pengujian urutan interaksi lengkap tetap diperlukan.

### Hak akses opname dapat dilewati

[Pembatasan sidebar](../src/gui/views/app_shell.rs#L60) menolak Stok opname untuk Kasir. Namun, [aksi menu obat](../src/gui/views/master_obat.rs#L826) memanggil `adjust_stock` tanpa pemeriksaan peran pengguna. Ini merupakan ketidakkonsistenan akses yang nyata, bukan kebutuhan menambah menu.

[Pencatatan opname](../src/db/queries.rs#L845) dan [skema kartu stok](../sql/schema.sql#L188) belum menyimpan operator yang melakukan penyesuaian. Faktur, pembayaran, dan retur sudah menyimpan operator, tetapi belum ada jejak perubahan sebelum/sesudah untuk edit master, harga, dan pengaturan.

Penonaktifan akun juga baru membatasi login berikutnya. [Pemeriksaan operator saat checkout](../src/db/operations.rs#L710) memeriksa keberadaan ID, sehingga sesi yang sudah login belum otomatis kehilangan akses setelah akun dinonaktifkan.

### Total stok bisa benar sementara batch/ED keliru

[Opname](../src/db/queries.rs#L864) memilih batch berdasarkan ID menurun. Selisih positif ditambahkan ke batch terbaru; selisih negatif dipotong dari urutan tersebut.

Contoh: sistem mencatat batch A=10 dan B=10. Hasil fisik A=5 dan B=10. Memasukkan total 15 justru dapat membuat sistem A=10 dan B=5. Total obat sesuai, tetapi tanggal ED dan alokasi FEFO tidak mencerminkan barang fisik.

Ini berbeda dari penjualan: [checkout FEFO](../src/db/queries.rs#L247) sudah memeriksa produk aktif, konversi, batch layak jual, penguncian stok, dan rollback.

### Pembelian perlu memeriksa perubahan master

[Pembelian](../src/db/operations.rs#L200) menghitung stok dengan `line.qty * line.conversion`. Konversi tersebut berasal dari draf. Jika master satuan berubah saat draf terbuka atau pending, stok masuk masih memakai konversi lama. Penjualan sudah mempunyai pemeriksaan ulang konversi terhadap master; pembelian perlu perlindungan yang sama.

### Retur, pending, dan pembayaran

[Retur](../src/db/operations.rs#L602) memproses seluruh alokasi faktur dan menambah stok kembali ke batch asal. Belum ada jumlah retur per item atau klasifikasi barang baik/rusak. Kebijakan penerimaan retur apotek menentukan kapan klasifikasi tersebut diperlukan.

[Daftar pending](../src/db/operations.rs#L814) mengambil 100 identitas terakhir tanpa paginasi. Data lama tidak dihapus, tetapi tidak muncul selama batas daftar terisi oleh draf yang lebih baru.

Pembayaran kredit lanjutan sudah memperbarui `paid_amount` faktur dan mencatat pembayaran secara atomik ([operations.rs](../src/db/operations.rs#L578)). Retur membaca nilai kumulatif tersebut. Tidak ditemukan masalah pengabaian cicilan pada alur refund ini.

## Kriteria penggunaan tanpa mouse

Pengujian harus dimulai dengan tangan hanya pada keyboard/scanner dan menyelesaikan fungsi berikut:

1. Login, pindah halaman, buka/tutup kelompok sidebar, dan keluar akun.
2. F1 pada daftar penjualan membuka kasir; F1 di kasir membuka obat master. Panah dan Enter harus bekerja pada pencarian kosong, hasil pencarian, dan obat yang dipilih.
3. Tambah lewat scanner, scan berulang, ubah item yang sudah masuk, kurangi jumlah, hapus item, dan pindah ke keranjang tanpa kehilangan fokus.
4. F2 mengisi pembayaran; Enter/F3 menyimpan. Pilihan cetak, pembatalan printer, dan kegagalan cetak mengembalikan fokus ke tempat yang tepat tanpa transaksi ganda.
5. F6 pending, F7 melanjutkan, serta mencari dan membatalkan draf.
6. Pilih faktur, baca rincian, cetak ulang, dan proses retur dengan keyboard.
7. Master obat: tambah/edit, buka Batch/ED dan menu baris, pilih baris terpisah/rentang, serta atur dan cetak barcode beberapa obat.
8. Pembelian, opname, pengaturan, akun karyawan, impor/ekspor, dan cadangan harus selesai dengan Tab/Shift+Tab/panah/Enter/Space/Esc.

F1/F2/F3/F5/F6/F7 yang sudah dipakai kasir tetap menjadi acuan. Menu/Shift+F10 untuk menu baris dan Ctrl+Space untuk pilihan terpisah adalah tambahan yang disarankan. Enter perlu konsisten pada aksi biasa dan tombol yang terfokus; pada footer multiline, Enter tetap membuat baris baru. Escape membatalkan langkah yang dapat dibatalkan, dan operasi penyimpanan tidak boleh terpicu dua kali.

Status “tanpa mouse” baru layak diberikan setelah seluruh skenario tersebut lolos, termasuk dialog printer/berkas pada Windows. Skrip GUI saat ini masih menggunakan klik pada banyak tahap dan belum membuktikan penggunaan menyeluruh melalui keyboard.

## Bagian yang mengikuti kebutuhan usaha

Barcode berbeda per kemasan/satuan, pembayaran campuran, pembulatan tunai, laci kas otomatis, pemulihan draf setelah aplikasi berhenti, dan transaksi saat LAN terputus dapat dipertimbangkan setelah alur utama stabil. Dokter, cloud, loyalty, reservasi, serta akuntansi lengkap tidak ditambahkan sebagai persyaratan audit ini. Operasi apotek tetap memakai LAN dan cadangan lokal.

Angka laporan juga perlu definisi yang jelas: obat terlaris menjumlahkan subtotal barang sebelum diskon faktur, sedangkan laporan per barang mengalokasikan diskon faktur. Perbedaan ini perlu dijelaskan atau disamakan bila keduanya disebut penjualan neto. Kebijakan diskon faktur yang menghasilkan laba negatif perlu ditentukan sebelum menerapkan larangan tambahan.

## Verifikasi dan batas pemeriksaan

- `env -u DISPLAY cargo test --offline`: 19 uji lulus; satu uji database diabaikan pada suite ini.
- `python3 tests/database_workflow.py`: satu uji alur database lulus pada MariaDB sementara, hanya melalui socket lokal di `/tmp`. Database apotek asli tidak dipakai.
- Pembentukan struk, barcode, validasi, dan alur database yang sudah dicakup uji tidak membuktikan semua temuan baru sudah diperbaiki.
- Interaksi GUI lengkap, scanner/printer fisik, dan dua PC Windows/LAN belum diverifikasi pada audit ini. Eksekusi Xvfb sebelumnya ditolak automatic approval review.
- Audit ini menambahkan laporan; perilaku aplikasi belum diubah untuk menutup kekurangan yang ditemukan.
