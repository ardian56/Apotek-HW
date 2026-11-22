# Aplikasi Desktop POS & Manajemen Apotek (Low-Spec & Low-RAM)

Aplikasi desktop *Point of Sale* (POS) dan manajemen inventori apotek yang dirancang **super ringan, cepat, dan stabil** untuk spesifikasi komputer rendah (Intel Core 2 Duo, RAM 1 GB, HDD, Windows 7 SP1 32-bit hingga Windows 11).

Adaptasi modul demo video, pratinjau tampilan, dan batas pengujian tersedia di [panduan adaptasi desktop](docs/video-reference.md). Buka pratinjau berisi data contoh dengan `cargo run -- --preview`. Untuk menambahkan contoh ke database server, jalankan `cargo run -- --seed-demo` sekali; pengisian dapat diulang tanpa duplikasi.

---

## 1. Arsitektur & Teknologi

* **Bahasa:** Rust 1.75.0 (versi resmi terakhir dengan dukungan penuh Windows 7 SP1).
* **Target Binary:** 32-bit (`i686-pc-windows-msvc` atau `i686-pc-windows-gnu`). 1 binary berjalan di Windows 7 32-bit maupun Windows 64-bit.
* **GUI Framework:** `fltk 1.4` (Bundled, flat modern theme, footprint RAM hanya ~20–35 MB).
* **Database:** MariaDB 10.4 / 10.5 (Tuning low-RAM `~50–70 MB`).
* **Jaringan LAN:** Auto-Discovery 2 PC (Gudang & Kasir) menggunakan UDP Broadcast port `45454` tanpa perlu setting IP manual.
* **Printer Struk:** Direct RAW ESC/POS via Windows Print Spooler (Cepat tanpa lag rendering).

---

## 2. Struktur Proyek

```text
kasir_apotek/
├── Cargo.toml                  # Dependensi dan profil optimasi release (opt-level="z", lto=true)
├── rust-toolchain.toml         # Mengunci Rust ke 1.75.0
├── config/
│   ├── my.ini                  # Konfigurasi MariaDB low-RAM (32MB buffer pool, disabled perf schema)
│   └── install_service.bat     # Script 1-klik pasang service MariaDB di PC Gudang
├── sql/
│   ├── schema.sql              # Skema tabel MariaDB (lengkap dengan FEFO batch & kartu stok)
│   └── seed.sql                # Data awal: user admin/kasir, suplier, dokter, contoh obat
└── src/
    ├── main.rs                 # Entry point, routing mode Server/Client, event loop
    ├── db/                     # Koneksi MySQL pool sinkron & query FEFO transaksi
    ├── gui/                    # GUI FLTK (Theme, Login, Layar Kasir, Master Obat)
    ├── hardware/               # Driver cetak RAW ESC/POS struk thermal 58mm/80mm
    ├── models/                 # Struct data (Product, Batch, Unit, Sale, User)
    ├── network/                # Auto-discovery LAN UDP port 45454 (Server & Client)
    └── services/               # Impor dan ekspor Excel
```

---

## 3. Shortcut Keyboard Layar Kasir (Wajib)

| Shortcut | Fungsi |
|---|---|
| **F1** | Fokus ke input Kode / Barcode / Cari Obat |
| **F2** | Fokus ke input Bayar (Nominal Tunai) |
| **F3** | Simpan Transaksi Penjualan |
| **F4** | Cetak Struk ke Printer Thermal |
| **F5** | Transaksi Baru / Bersihkan Keranjang |
| **F6** | Simpan Transaksi Sementara (Pending) |
| **F7** | Buka Daftar Transaksi Pending |

---

## 4. Cara Setup & Kompilasi

### A. Persiapan Toolchain Rust 1.75.0
```bash
# Install Rust 1.75.0
rustup install 1.75.0
rustup default 1.75.0

# Tambahkan target 32-bit Windows
rustup target add i686-pc-windows-msvc
# atau untuk MinGW:
rustup target add i686-pc-windows-gnu
```

### B. Kompilasi Release Binary (Ukuran Minimal)
```bash
cargo build --release --target i686-pc-windows-msvc
```
Hasil file executable berada di `target/i686-pc-windows-msvc/release/kasir_apotek.exe`.

---

## 5. Panduan Instalasi di Apotek (2 Komputer)

### Komputer 1: PC Gudang (Bertindak sebagai Server Database)
1. Ekstrak folder MariaDB Portable 32-bit (versi 10.4 atau 10.5) ke `C:\Apotek\mariadb`.
2. Salin file `config/my.ini` ke `C:\Apotek\config\my.ini`.
3. Klik kanan `config/install_service.bat` -> **Run as Administrator**.
4. Import database:
   ```cmd
   mariadb -u root -p < sql/schema.sql
   mariadb -u root -p < sql/seed.sql
   ```
5. Jalankan aplikasi POS dalam **Mode Server**:
   ```cmd
   kasir_apotek.exe --server
   ```
   *(Aplikasi akan otomatis mengaktifkan UDP listener responder di background port 45454).*

### Komputer 2: PC Kasir (Client)
1. Sambungkan PC Kasir dan PC Gudang ke Switch / Router LAN yang sama.
2. Cukup salin `kasir_apotek.exe` ke Desktop PC Kasir. **TIDAK PERLU install MariaDB.**
3. Buka `kasir_apotek.exe`.
   * Aplikasi akan otomatis mem-broadcast dan mendeteksi IP PC Gudang dalam 1–2 detik.
   * Kasir siap digunakan dengan akun default:
     * **Username:** `kasir`
     * **Password:** `kasir123`
     *(atau akun `admin` / `admin123`)*.
