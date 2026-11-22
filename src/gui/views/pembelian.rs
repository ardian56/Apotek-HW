use super::components::*;
use crate::gui::theme::AppTheme as T;
use fltk::{button::Button, frame::Frame, input::Input, menu::Choice, prelude::*, window::Window};
use mysql::Pool;

pub struct PembelianView {
    pub win: Window,
    pub input_faktur: Input,
    pub input_tgl: Input,
    pub input_discount: Input,
    pub input_tax: Input,
    pub choice_suplier: Choice,
    pub choice_bayar: Choice,
    pub input_tempo: Input,
    pub btn_tambah_item: Button,
    pub btn_simpan: Button,
    pub btn_baru: Button,
    pub btn_back: Button,
    pub btn_remove: Button,
    pub btn_pending: Button,
    pub btn_restore: Button,
    pub grid: Grid,
    pub lbl_subtotal: Frame,
    pub lbl_grand_total: Frame,
    pub status: Frame,
}

impl PembelianView {
    pub fn new(_pool: Option<Pool>) -> Self {
        let mut win = Window::new(0, 108, 1200, 612, None);
        win.set_color(T::BG_APP);
        page_header(
            "Pembelian baru",
            "Catat faktur suplier dan obat yang masuk ke apotek.",
        );
        let btn_tambah_item = button(968, 24, 204, "Tambah obat", false);
        let input_faktur = input(28, 88, 176, "Nomor faktur", "");
        let input_tgl = input(
            220,
            88,
            148,
            "Tanggal faktur",
            &chrono::Local::now().date_naive().to_string(),
        );
        text(384, 88, 284, 18, "Suplier", 12, false);
        let mut choice_suplier = Choice::new(384, 112, 284, 34, None);
        style_choice(&mut choice_suplier);
        choice_suplier.set_text_size(14);
        text(684, 88, 156, 18, "Pembayaran", 12, false);
        let mut choice_bayar = Choice::new(684, 112, 156, 34, None);
        style_choice(&mut choice_bayar);
        choice_bayar.set_text_size(14);
        choice_bayar.add_choice("Tunai|Kredit");
        choice_bayar.set_value(0);
        let input_tempo = input(
            28,
            158,
            176,
            "Jatuh tempo kredit",
            &(chrono::Local::now().date_naive() + chrono::Duration::days(30)).to_string(),
        );
        text(
            220,
            182,
            620,
            28,
            "Batch dan tanggal kedaluwarsa dicatat pada setiap obat.",
            12,
            false,
        );
        let mut grid = Grid::new(
            28,
            234,
            812,
            290,
            &[
                "Kode",
                "Obat",
                "Qty",
                "Satuan",
                "Harga (Rp)",
                "Diskon (%)",
                "Subtotal (Rp)",
            ],
        );
        for (i, w) in [112, 246, 48, 76, 102, 94, 116].iter().enumerate() {
            grid.table.set_col_width(i as i32, *w);
        }
        panel(864, 88, 308, 436);
        text(884, 108, 264, 20, "Subtotal barang", 13, false);
        let lbl_subtotal = text(884, 136, 264, 32, "Rp 0", 22, true);
        let input_discount = input(884, 194, 264, "Potongan faktur (Rp)", "0");
        let input_tax = input(884, 268, 264, "Pajak (%)", "0");
        text(884, 342, 264, 20, "Total pembelian", 13, false);
        let mut lbl_grand_total = text(884, 372, 264, 44, "Rp 0", 30, true);
        lbl_grand_total.set_label_color(T::PRIMARY);
        text(
            884,
            440,
            264,
            36,
            "Stok bertambah setelah disimpan.",
            12,
            false,
        );
        let btn_back = button(28, 552, 196, "Daftar pembelian", false);
        let btn_pending = button(240, 552, 144, "Pending [F6]", false);
        let btn_restore = button(400, 552, 164, "Buka pending [F7]", false);
        let btn_baru = button(580, 552, 144, "Bersihkan [F5]", false);
        let btn_remove = button(740, 552, 132, "Hapus item", false);
        let btn_simpan = button(1028, 552, 144, "Simpan [F3]", true);
        let status = text(28, 592, 1144, 18, "", 12, false);
        win.end();
        Self {
            win,
            input_faktur,
            input_tgl,
            input_discount,
            input_tax,
            choice_suplier,
            choice_bayar,
            input_tempo,
            btn_tambah_item,
            btn_simpan,
            btn_baru,
            btn_back,
            btn_remove,
            btn_pending,
            btn_restore,
            grid,
            lbl_subtotal,
            lbl_grand_total,
            status,
        }
    }
}
