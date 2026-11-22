use crate::gui::theme::AppTheme;
use crate::services::pos::CartItem;
use fltk::button::Button;
use fltk::enums::{Align, Color, Font, FrameType, Shortcut};
use fltk::frame::Frame;
use fltk::group::{Flex, Group};
use fltk::input::Input;
use fltk::menu::Choice;
use fltk::prelude::*;
use fltk::table::Table;
use fltk::window::Window;

pub struct KasirView {
    pub win: Window,
    pub input_barcode: Input,
    pub input_qty: Input,
    pub choice_unit: Choice,
    pub btn_add_item: Button,
    pub cart_table: Table,
    pub lbl_grand_total: Frame,
    pub input_bayar: Input,
    pub lbl_kembali: Frame,
    pub btn_simpan: Button,
    pub btn_cetak: Button,
    pub btn_baru: Button,
    pub btn_pending: Button,
    pub btn_buka_pending: Button,
    pub lbl_network_status: Frame,
    pub lbl_warning_rugi: Frame,
    pub cart_items: Vec<CartItem>,
}

impl KasirView {
    pub fn new() -> Self {
        let mut win = Window::default()
            .with_size(1024, 680)
            .with_label("POS Kasir Apotek - [F1 Cari | F2 Bayar | F3 Simpan | F4 Cetak | F5 Baru]");
        win.set_color(AppTheme::BG_APP);

        let mut root = Flex::default().size_of_parent().column();
        root.set_pad(8);
        root.set_margin(10);

        // ==========================================
        // 1. TOP BAR (Info Toko, Operator, Indikator LAN)
        // ==========================================
        let mut top_bar = Group::default().with_size(0, 48);
        top_bar.set_color(AppTheme::BG_CARD);
        top_bar.set_frame(FrameType::FlatBox);

        let mut lbl_store = Frame::new(top_bar.x() + 15, top_bar.y() + 8, 250, 32, "APOTEK SEHAT SEJAHTERA");
        lbl_store.set_label_size(16);
        lbl_store.set_label_font(Font::HelveticaBold);
        lbl_store.set_label_color(AppTheme::PRIMARY);
        lbl_store.set_align(Align::Left | Align::Inside);

        let mut lbl_network_status = Frame::new(top_bar.w() - 220, top_bar.y() + 12, 200, 24, "● Terhubung ke Server");
        lbl_network_status.set_label_color(AppTheme::SUCCESS);
        lbl_network_status.set_label_font(Font::HelveticaBold);
        lbl_network_status.set_label_size(12);

        top_bar.end();
        root.fixed(&top_bar, 48);

        // ==========================================
        // 2. INPUT BARCODE & SHORTCUT BAR (F1)
        // ==========================================
        let mut input_bar = Group::default().with_size(0, 52);
        input_bar.set_color(AppTheme::BG_CARD);
        input_bar.set_frame(FrameType::FlatBox);

        let mut lbl_f1 = Frame::new(input_bar.x() + 12, input_bar.y() + 10, 110, 32, "[F1] Barcode:");
        lbl_f1.set_label_font(Font::HelveticaBold);
        lbl_f1.set_label_size(13);
        lbl_f1.set_align(Align::Left | Align::Inside);

        let mut input_barcode = Input::new(input_bar.x() + 125, input_bar.y() + 9, 280, 34, "");
        input_barcode.set_text_size(14);
        input_barcode.set_frame(FrameType::FlatBox);
        input_barcode.set_color(Color::from_hex(0xF0F4F8));

        let mut lbl_qty = Frame::new(input_barcode.x() + input_barcode.w() + 15, input_bar.y() + 10, 40, 32, "Qty:");
        lbl_qty.set_label_font(Font::HelveticaBold);

        let mut input_qty = Input::new(lbl_qty.x() + lbl_qty.w() + 5, input_bar.y() + 9, 60, 34, "");
        input_qty.set_value("1");
        input_qty.set_text_size(14);
        input_qty.set_frame(FrameType::FlatBox);
        input_qty.set_color(Color::from_hex(0xF0F4F8));

        let mut choice_unit = Choice::new(input_qty.x() + input_qty.w() + 10, input_bar.y() + 9, 100, 34, "");
        choice_unit.add_choice("Tablet|Strip|Box");
        choice_unit.set_value(0);

        let mut btn_add_item = Button::new(choice_unit.x() + choice_unit.w() + 15, input_bar.y() + 9, 110, 34, "+ Tambah");
        btn_add_item.set_color(AppTheme::PRIMARY);
        btn_add_item.set_label_color(Color::White);
        btn_add_item.set_label_font(Font::HelveticaBold);
        btn_add_item.set_frame(FrameType::FlatBox);

        // Warning Label Jika Rugi
        let mut lbl_warning_rugi = Frame::new(btn_add_item.x() + btn_add_item.w() + 15, input_bar.y() + 10, 240, 32, "");
        lbl_warning_rugi.set_label_color(AppTheme::DANGER_TEXT);
        lbl_warning_rugi.set_label_font(Font::HelveticaBold);
        lbl_warning_rugi.set_label_size(12);

        input_bar.end();
        root.fixed(&input_bar, 52);

        // ==========================================
        // 3. TABEL KERANJANG BELANJA (CART)
        // ==========================================
        let mut cart_table = Table::default();
        cart_table.set_color(Color::White);
        cart_table.set_rows(0);
        cart_table.set_cols(7);
        cart_table.set_col_header(true);
        cart_table.set_col_width(0, 50);   // No
        cart_table.set_col_width(1, 140);  // Barcode
        cart_table.set_col_width(2, 320);  // Nama Obat
        cart_table.set_col_width(3, 90);   // Satuan
        cart_table.set_col_width(4, 70);   // Qty
        cart_table.set_col_width(5, 120);  // Harga
        cart_table.set_col_width(6, 140);  // Subtotal

        // ==========================================
        // 4. BOTTOM PANEL: TOTAL, BAYAR, SHORTCUT BUTTONS
        // ==========================================
        let mut bottom_panel = Group::default().with_size(0, 160);
        bottom_panel.set_color(AppTheme::BG_CARD);
        bottom_panel.set_frame(FrameType::FlatBox);

        // Display Total Besar di kanan
        let mut lbl_total_title = Frame::new(bottom_panel.w() - 420, bottom_panel.y() + 10, 400, 24, "TOTAL PEMBAYARAN:");
        lbl_total_title.set_align(Align::Right | Align::Inside);
        lbl_total_title.set_label_size(13);
        lbl_total_title.set_label_font(Font::HelveticaBold);
        lbl_total_title.set_label_color(AppTheme::TEXT_MUTED);

        let mut lbl_grand_total = Frame::new(bottom_panel.w() - 420, bottom_panel.y() + 32, 400, 55, "Rp 0");
        lbl_grand_total.set_align(Align::Right | Align::Inside);
        lbl_grand_total.set_label_size(36);
        lbl_grand_total.set_label_font(Font::HelveticaBold);
        lbl_grand_total.set_label_color(AppTheme::PRIMARY);

        // Input Bayar (F2) & Kembali
        let mut lbl_bayar_title = Frame::new(bottom_panel.x() + 20, bottom_panel.y() + 15, 120, 30, "[F2] Bayar (Rp):");
        lbl_bayar_title.set_label_font(Font::HelveticaBold);
        lbl_bayar_title.set_label_size(14);
        lbl_bayar_title.set_align(Align::Left | Align::Inside);

        let mut input_bayar = Input::new(bottom_panel.x() + 145, bottom_panel.y() + 12, 180, 36, "");
        input_bayar.set_text_size(18);
        input_bayar.set_text_font(Font::HelveticaBold);
        input_bayar.set_frame(FrameType::FlatBox);
        input_bayar.set_color(Color::from_hex(0xEDE7F6));

        let mut lbl_kembali_title = Frame::new(bottom_panel.x() + 20, bottom_panel.y() + 55, 120, 30, "Kembalian:");
        lbl_kembali_title.set_label_font(Font::HelveticaBold);
        lbl_kembali_title.set_label_size(14);
        lbl_kembali_title.set_align(Align::Left | Align::Inside);

        let mut lbl_kembali = Frame::new(bottom_panel.x() + 145, bottom_panel.y() + 55, 180, 30, "Rp 0");
        lbl_kembali.set_label_size(18);
        lbl_kembali.set_label_font(Font::HelveticaBold);
        lbl_kembali.set_label_color(AppTheme::SUCCESS);
        lbl_kembali.set_align(Align::Left | Align::Inside);

        // Tombol Aksi F3 - F7
        let btn_y = bottom_panel.y() + 105;
        let mut btn_simpan = Button::new(bottom_panel.x() + 20, btn_y, 140, 42, "[F3] Simpan");
        btn_simpan.set_color(AppTheme::SUCCESS);
        btn_simpan.set_label_color(Color::White);
        btn_simpan.set_label_font(Font::HelveticaBold);
        btn_simpan.set_frame(FrameType::FlatBox);
        btn_simpan.set_shortcut(Shortcut::None);

        let mut btn_cetak = Button::new(btn_simpan.x() + 150, btn_y, 140, 42, "[F4] Cetak Struk");
        btn_cetak.set_color(AppTheme::PRIMARY);
        btn_cetak.set_label_color(Color::White);
        btn_cetak.set_label_font(Font::HelveticaBold);
        btn_cetak.set_frame(FrameType::FlatBox);

        let mut btn_baru = Button::new(btn_cetak.x() + 150, btn_y, 130, 42, "[F5] Baru");
        btn_baru.set_color(Color::from_hex(0x607D8B));
        btn_baru.set_label_color(Color::White);
        btn_baru.set_label_font(Font::HelveticaBold);
        btn_baru.set_frame(FrameType::FlatBox);

        let mut btn_pending = Button::new(btn_baru.x() + 140, btn_y, 140, 42, "[F6] Pending");
        btn_pending.set_color(Color::from_hex(0xFF9800));
        btn_pending.set_label_color(Color::White);
        btn_pending.set_label_font(Font::HelveticaBold);
        btn_pending.set_frame(FrameType::FlatBox);

        let mut btn_buka_pending = Button::new(btn_pending.x() + 150, btn_y, 150, 42, "[F7] Buka Pending");
        btn_buka_pending.set_color(Color::from_hex(0x795548));
        btn_buka_pending.set_label_color(Color::White);
        btn_buka_pending.set_label_font(Font::HelveticaBold);
        btn_buka_pending.set_frame(FrameType::FlatBox);

        bottom_panel.end();
        root.fixed(&bottom_panel, 160);

        root.end();
        win.end();

        Self {
            win,
            input_barcode,
            input_qty,
            choice_unit,
            btn_add_item,
            cart_table,
            lbl_grand_total,
            input_bayar,
            lbl_kembali,
            btn_simpan,
            btn_cetak,
            btn_baru,
            btn_pending,
            btn_buka_pending,
            lbl_network_status,
            lbl_warning_rugi,
            cart_items: Vec::new(),
        }
    }
}
