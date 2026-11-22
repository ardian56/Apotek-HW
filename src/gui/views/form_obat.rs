use super::components::*;
use crate::{gui::theme::AppTheme as T, models::Product};
use fltk::{button::Button, input::Input, menu::Choice, prelude::*, window::Window};

#[derive(Clone)]
pub struct FormObatModal {
    pub win: Window,
    pub input_kode: Input,
    pub btn_barcode: Button,
    pub input_nama: Input,
    pub choice_kategori: Choice,
    pub choice_suplier: Choice,
    pub input_satuan_dasar: Input,
    pub input_harga_beli: Input,
    pub input_min_stok: Input,
    pub input_harga_jual_1: Input,
    pub input_nama_satuan_2: Input,
    pub input_konversi_2: Input,
    pub input_harga_jual_2: Input,
    pub input_nama_satuan_3: Input,
    pub input_konversi_3: Input,
    pub input_harga_jual_3: Input,
    pub input_no_batch: Input,
    pub input_expired: Input,
    pub input_stok_awal: Input,
    pub btn_simpan: Button,
}
impl FormObatModal {
    pub fn new(existing: Option<&Product>) -> Self {
        let title = if existing.is_some() {
            "Edit obat"
        } else {
            "Tambah obat"
        };
        let mut win = Window::default().with_size(680, 660).with_label(title);
        win.set_color(T::BG_APP);
        text(16, 20, 648, 32, title, 22, true);
        text(
            16,
            58,
            648,
            20,
            "Isi barcode kemasan atau pilih Buat barcode jika belum ada.",
            12,
            false,
        );
        let mut input_kode = input(16, 88, 180, "Kode / barcode", "");
        let btn_barcode = button(212, 112, 132, "Buat barcode", false);
        let mut input_nama = input(360, 88, 304, "Nama obat dan dosis", "");
        text(16, 162, 180, 18, "Kategori", 12, false);
        let mut choice_kategori = Choice::new(16, 186, 180, 34, None);
        style_choice(&mut choice_kategori);
        choice_kategori.add_choice("Obat bebas|Obat keras|Alat kesehatan|Vitamin|Lainnya");
        choice_kategori.set_value(0);
        text(212, 162, 216, 18, "Suplier utama", 12, false);
        let mut choice_suplier = Choice::new(212, 186, 216, 34, None);
        style_choice(&mut choice_suplier);
        choice_suplier.add_choice("Tanpa suplier");
        choice_suplier.set_value(0);
        let mut input_satuan_dasar = input(444, 162, 104, "Satuan dasar", "Tablet");
        let mut input_min_stok = input(564, 162, 100, "Stok minimum", "10");
        let mut input_harga_beli = input(16, 236, 180, "Harga beli dasar (Rp)", "0");
        let mut input_harga_jual_1 = input(212, 236, 216, "Harga jual dasar (Rp)", "0");
        let mut input_nama_satuan_2 = input(16, 310, 200, "Satuan kedua", "");
        let mut input_konversi_2 = input(232, 310, 120, "Isi satuan dasar", "1");
        let mut input_harga_jual_2 = input(368, 310, 296, "Harga jual satuan kedua (Rp)", "0");
        let mut input_nama_satuan_3 = input(16, 384, 200, "Satuan ketiga", "");
        let mut input_konversi_3 = input(232, 384, 120, "Isi satuan dasar", "1");
        let mut input_harga_jual_3 = input(368, 384, 296, "Harga jual satuan ketiga (Rp)", "0");
        let mut input_no_batch = input(16, 458, 180, "Nomor batch awal", "");
        let mut input_expired = input(212, 458, 216, "Kedaluwarsa (YYYY-MM-DD)", "");
        let mut input_stok_awal = input(444, 458, 220, "Stok awal (satuan dasar)", "0");
        let mut btn_batal = button(400, 588, 110, "Batal", false);
        let btn_simpan = button(526, 588, 138, "Simpan obat", true);
        let mut close = win.clone();
        btn_batal.set_callback(move |_| close_modal(&mut close));
        if let Some(p) = existing {
            input_kode.set_value(&p.code);
            input_nama.set_value(&p.name);
            input_satuan_dasar.set_value(&p.base_unit);
            input_min_stok.set_value(&p.min_stock.to_string());
            input_harga_beli.set_value(&p.buy_price.to_string());
            if let Some(u) = p.units.iter().find(|u| u.conversion_qty == 1) {
                input_harga_jual_1.set_value(&u.sell_price.to_string());
            }
            if let Some(u) = p.units.get(1) {
                input_nama_satuan_2.set_value(&u.unit_name);
                input_konversi_2.set_value(&u.conversion_qty.to_string());
                input_harga_jual_2.set_value(&u.sell_price.to_string());
            }
            if let Some(u) = p.units.get(2) {
                input_nama_satuan_3.set_value(&u.unit_name);
                input_konversi_3.set_value(&u.conversion_qty.to_string());
                input_harga_jual_3.set_value(&u.sell_price.to_string());
            }
            if let Some(b) = p.batches.first() {
                input_no_batch.set_value(&b.batch_no);
                input_expired.set_value(b.expired_date.as_deref().unwrap_or(""));
                input_stok_awal.set_value(&b.stock.to_string());
            }
        }
        win.end();
        win.make_modal(true);
        win.set_callback(close_modal);
        Self {
            win,
            input_kode,
            btn_barcode,
            input_nama,
            choice_kategori,
            choice_suplier,
            input_satuan_dasar,
            input_harga_beli,
            input_min_stok,
            input_harga_jual_1,
            input_nama_satuan_2,
            input_konversi_2,
            input_harga_jual_2,
            input_nama_satuan_3,
            input_konversi_3,
            input_harga_jual_3,
            input_no_batch,
            input_expired,
            input_stok_awal,
            btn_simpan,
        }
    }
}
