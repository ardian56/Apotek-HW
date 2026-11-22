use super::components::*;
use crate::gui::theme::AppTheme as T;
use fltk::{button::Button, frame::Frame, input::Input, menu::Choice, prelude::*, window::Window};

pub struct KasirView {
    pub win: Window,
    pub input_barcode: Input,
    pub btn_add_item: Button,
    pub grid: Grid,
    pub lbl_grand_total: Frame,
    pub input_bayar: Input,
    pub lbl_kembali: Frame,
    pub payment: Choice,
    pub customer: Choice,
    pub due: Input,
    pub due_label: Frame,
    pub btn_back: Button,
    pub btn_remove: Button,
    pub btn_simpan: Button,
    pub btn_cetak: Button,
    pub btn_baru: Button,
    pub btn_pending: Button,
    pub btn_buka_pending: Button,
    pub status: Frame,
}

impl KasirView {
    pub fn new() -> Self {
        let mut win = Window::new(0, 108, 1200, 612, None);
        win.set_color(T::BG_APP);
        page_header(
            "Penjualan baru",
            "Scan barcode + Enter menambah 1 satuan dasar. Gunakan pencarian manual untuk satuan atau jumlah lainnya.",
        );
        let btn_add_item = button(968, 24, 204, "Cari obat manual", false);
        let input_barcode = input(28, 88, 812, "Cari nama atau barcode [F1]", "");
        let mut grid = Grid::new(
            28,
            166,
            812,
            358,
            &[
                "Kode",
                "Obat",
                "Qty",
                "Satuan",
                "Harga (Rp)",
                "Diskon (Rp)",
                "Subtotal (Rp)",
            ],
        );
        for (i, w) in [112, 246, 48, 76, 102, 94, 116].iter().enumerate() {
            grid.table.set_col_width(i as i32, *w);
        }
        panel(864, 88, 308, 436);
        text(884, 108, 264, 20, "Total penjualan", 13, false);
        let mut lbl_grand_total = text(884, 136, 264, 44, "Rp 0", 30, true);
        lbl_grand_total.set_label_color(T::PRIMARY);
        text(884, 204, 264, 18, "Pembayaran", 12, false);
        let mut payment = Choice::new(884, 228, 264, 34, None);
        payment.add_choice("Tunai|Kredit");
        payment.set_value(0);
        style_choice(&mut payment);
        payment.set_text_size(14);
        text(884, 274, 264, 18, "Pelanggan", 12, false);
        let mut customer = Choice::new(884, 298, 264, 34, None);
        style_choice(&mut customer);
        customer.set_text_size(14);
        let input_bayar = input(884, 344, 264, "Uang diterima [F2]", "0");
        text(884, 410, 264, 18, "Kembalian", 12, false);
        let lbl_kembali = text(884, 432, 264, 26, "Rp 0", 20, true);
        let mut due_label = text(884, 464, 264, 18, "Jatuh tempo kredit", 12, false);
        let mut due = Input::new(884, 488, 264, 34, None);
        due.set_frame(fltk::enums::FrameType::BorderBox);
        due.set_text_size(14);
        due.hide();
        due_label.hide();
        let btn_back = button(28, 552, 196, "Daftar penjualan", false);
        let btn_pending = button(240, 552, 144, "Pending [F6]", false);
        let btn_buka_pending = button(400, 552, 164, "Buka pending [F7]", false);
        let btn_baru = button(580, 552, 144, "Bersihkan [F5]", false);
        let btn_remove = button(740, 552, 132, "Hapus item", false);
        let btn_cetak = button(888, 552, 124, "Cetak [F4]", false);
        let btn_simpan = button(1028, 552, 144, "Simpan [F3]", true);
        let status = text(28, 592, 1144, 18, "", 12, false);
        win.end();
        Self {
            win,
            input_barcode,
            btn_add_item,
            grid,
            lbl_grand_total,
            input_bayar,
            lbl_kembali,
            payment,
            customer,
            due,
            due_label,
            btn_back,
            btn_remove,
            btn_simpan,
            btn_cetak,
            btn_baru,
            btn_pending,
            btn_buka_pending,
            status,
        }
    }
}
