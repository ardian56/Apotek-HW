use super::components::*;
use crate::gui::theme::AppTheme as T;
use fltk::{
    button::Button,
    enums::{Color, FrameType},
    frame::Frame,
    input::Input,
    menu::Choice,
    prelude::*,
    window::Window,
};

pub struct KasirView {
    pub win: Window,
    pub input_barcode: Input,
    pub product: Choice,
    pub qty: Input,
    pub unit: Choice,
    pub price: Input,
    pub item_discount: Input,
    pub line_total: Frame,
    pub date: Input,
    pub invoice: Input,
    pub salesperson: Choice,
    pub sale_type_label: Frame,
    pub discount_percent: Input,
    pub discount: Input,
    pub shipping: Input,
    pub paper: Choice,
    pub btn_preview: Button,
    pub btn_add_item: Button,
    pub grid: Grid,
    pub lbl_grand_total: Frame,
    pub lbl_subtotal: Frame,
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

fn choice(x: i32, y: i32, w: i32, label: &str) -> Choice {
    text(x, y, w, 18, label, 12, false);
    let mut value = Choice::new(x, y + 24, w, 34, None);
    style_choice(&mut value);
    value
}

fn header_input(x: i32, y: i32, w: i32, label: &str, value: &str) -> Input {
    text(x, y, 84, 34, label, 12, false);
    let mut field = Input::new(x + 96, y, w, 34, None);
    field.set_frame(FrameType::BorderBox);
    field.set_color(Color::White);
    field.set_text_color(T::TEXT_BODY);
    field.set_text_size(14);
    field.set_value(value);
    field
}

fn header_choice(x: i32, y: i32, w: i32, label: &str) -> Choice {
    text(x, y, 84, 34, label, 12, false);
    let mut field = Choice::new(x + 96, y, w, 34, None);
    style_choice(&mut field);
    field
}

fn divider(y: i32) {
    let mut line = Frame::new(28, y, 1144, 1, None);
    line.set_frame(FrameType::FlatBox);
    line.set_color(T::BORDER);
}

impl KasirView {
    pub fn new() -> Self {
        let mut win = Window::new(0, 108, 1200, 612, None);
        win.set_color(T::BG_APP);
        page_header(
            "Penjualan baru",
            "Scan + Enter menambah obat. F3 simpan, F4 simpan dan cetak.",
        );
        let btn_back = button(984, 24, 188, "Riwayat penjualan", false);
        let date = header_input(
            28,
            84,
            220,
            "Tanggal",
            &chrono::Local::now().date_naive().to_string(),
        );
        let mut invoice = header_input(28, 130, 220, "No. faktur", "");
        invoice.set_readonly(true);
        invoice.set_tooltip("Nomor faktur dibuat otomatis");
        let mut payment = header_choice(392, 84, 200, "Pembayaran");
        payment.add_choice("Tunai|Kredit");
        payment.set_value(0);
        let mut sale_type_label = text(392, 130, 296, 34, "Penjualan biasa", 12, false);
        sale_type_label.set_label_color(T::TEXT_MUTED);
        let mut due_label = text(392, 130, 84, 34, "Jatuh tempo", 12, false);
        let mut due = header_input(392, 130, 200, "", "");
        due_label.hide();
        due.hide();
        let salesperson = header_choice(736, 84, 340, "Sales");
        let customer = header_choice(736, 130, 340, "Pelanggan");
        divider(184);
        let input_barcode = input(28, 196, 156, "Kode / cari [F1]", "");
        let product = choice(200, 196, 284, "Nama obat");
        let qty = input(500, 196, 64, "Jumlah", "1");
        let unit = choice(580, 196, 96, "Satuan");
        let price = input(692, 196, 112, "Harga (Rp)", "0");
        let item_discount = input(820, 196, 72, "Disc (%)", "0");
        let line_total = text(908, 220, 132, 34, "Rp 0", 14, true);
        let btn_add_item = button(1056, 220, 116, "Tambah", true);
        let mut grid = Grid::new(
            28,
            268,
            1144,
            184,
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
        for (i, width) in [180, 360, 64, 112, 144, 120, 144].iter().enumerate() {
            grid.table.set_col_width(i as i32, *width);
        }
        divider(458);
        let discount_percent = input(28, 468, 100, "Diskon (%)", "0");
        let discount = input(144, 468, 144, "Diskon (Rp)", "0");
        let shipping = input(304, 468, 148, "Ongkos (Rp)", "0");
        text(492, 468, 352, 18, "Total bayar", 13, false);
        let mut lbl_grand_total = text(492, 486, 352, 40, "Rp 0", 32, true);
        lbl_grand_total.set_label_color(T::PRIMARY);
        let mut lbl_subtotal = text(492, 528, 148, 18, "Subtotal Rp 0", 11, false);
        lbl_subtotal.set_label_color(T::TEXT_MUTED);
        let input_bayar = input(876, 468, 296, "Bayar [F2]", "0");
        text(876, 528, 96, 22, "Kembalian", 12, false);
        let lbl_kembali = text(984, 528, 188, 22, "Rp 0", 18, true);
        let btn_baru = button(28, 552, 144, "Baru [F5]", false);
        let btn_remove = button(188, 552, 116, "Hapus item", false);
        let btn_pending = button(320, 552, 140, "Pending [F6]", false);
        let btn_buka_pending = button(476, 552, 164, "Daftar pending [F7]", false);
        let mut paper = choice(656, 528, 116, "Struk");
        paper.add_choice("58 mm|80 mm");
        paper.set_value(0);
        let mut btn_preview = button(788, 552, 100, "Lihat struk", false);
        btn_preview.set_tooltip("Pratinjau struk; tidak menyimpan transaksi");
        let btn_cetak = button(904, 552, 128, "Cetak [F4]", false);
        let btn_simpan = button(1048, 552, 124, "Simpan [F3]", true);
        let status = text(28, 592, 1144, 18, "", 12, false);
        win.end();
        Self {
            win,
            input_barcode,
            product,
            qty,
            unit,
            price,
            item_discount,
            line_total,
            date,
            invoice,
            salesperson,
            sale_type_label,
            discount_percent,
            discount,
            shipping,
            paper,
            btn_preview,
            btn_add_item,
            grid,
            lbl_grand_total,
            lbl_subtotal,
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
