use crate::gui::theme::AppTheme;
use crate::services::pos::CartItem;
use fltk::button::Button;
use fltk::draw;
use fltk::enums::{Align, Color, Font, FrameType};
use fltk::frame::Frame;
use fltk::input::Input;
use fltk::menu::Choice;
use fltk::prelude::*;
use fltk::table::Table;
use fltk::window::Window;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct MockCartRow {
    pub no: i32,
    pub barcode: String,
    pub name: String,
    pub unit: String,
    pub qty: i32,
    pub price: f64,
    pub subtotal: f64,
}

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
    pub lbl_operator: Frame,
    pub lbl_warning_rugi: Frame,
    pub cart_items: Vec<CartItem>,
    pub rows_data: Rc<RefCell<Vec<MockCartRow>>>,
}

impl KasirView {
    pub fn new() -> Self {
        let mut win = Window::default().with_size(1024, 680).with_label(
            "Kasir Penjualan POS - [F1 Cari | F2 Bayar | F3 Simpan | F4 Cetak | F5 Baru]",
        );
        win.set_color(AppTheme::BG_APP);

        // ==========================================
        // 1. TOP HEADER (1004 x 46)
        // ==========================================
        let mut top_card = Frame::new(12, 10, 1000, 46, "");
        top_card.set_frame(FrameType::RFlatBox);
        top_card.set_color(Color::White);

        let mut lbl_store = Frame::new(26, 17, 300, 32, "Kasir penjualan");
        lbl_store.set_label_size(16);
        lbl_store.set_label_font(Font::HelveticaBold);
        lbl_store.set_label_color(AppTheme::PRIMARY);
        lbl_store.set_align(Align::Left | Align::Inside);

        let mut lbl_operator = Frame::new(340, 18, 300, 30, "Pilih obat, satuan, dan jumlah");
        lbl_operator.set_label_size(12);
        lbl_operator.set_label_color(AppTheme::TEXT_MUTED);
        lbl_operator.set_align(Align::Left | Align::Inside);

        let mut lbl_network_status = Frame::new(790, 18, 206, 30, "● Terhubung Server");
        lbl_network_status.set_label_color(AppTheme::SUCCESS);
        lbl_network_status.set_label_font(Font::HelveticaBold);
        lbl_network_status.set_label_size(12);
        lbl_network_status.set_align(Align::Right | Align::Inside);

        // ==========================================
        // 2. INPUT BARCODE & QTY (1000 x 50)
        // ==========================================
        let mut bar_card = Frame::new(12, 64, 1000, 50, "");
        bar_card.set_frame(FrameType::RFlatBox);
        bar_card.set_color(Color::White);

        let mut lbl_f1 = Frame::new(26, 74, 90, 30, "[F1] Barcode:");
        lbl_f1.set_label_font(Font::HelveticaBold);
        lbl_f1.set_label_size(12);
        lbl_f1.set_label_color(AppTheme::TEXT_TITLE);
        lbl_f1.set_align(Align::Left | Align::Inside);

        let mut input_barcode = Input::new(118, 73, 260, 32, "");
        input_barcode.set_value("8999909001");
        input_barcode.set_text_size(14);
        input_barcode.set_frame(FrameType::RFlatBox);
        input_barcode.set_color(Color::from_hex(0xF8FAFC));

        let mut lbl_qty = Frame::new(388, 74, 35, 30, "Qty:");
        lbl_qty.hide();
        lbl_qty.set_label_font(Font::HelveticaBold);
        lbl_qty.set_label_color(AppTheme::TEXT_TITLE);

        let mut input_qty = Input::new(425, 73, 50, 32, "");
        input_qty.set_value("2");
        input_qty.hide();
        input_qty.set_text_size(14);
        input_qty.set_frame(FrameType::RFlatBox);
        input_qty.set_color(Color::from_hex(0xF8FAFC));

        let mut choice_unit = Choice::new(485, 73, 90, 32, "");
        choice_unit.add_choice("Strip|Tablet|Box");
        choice_unit.set_value(0);
        choice_unit.hide();
        choice_unit.set_frame(FrameType::RFlatBox);
        choice_unit.set_color(Color::from_hex(0xF8FAFC));

        input_barcode.resize(118, 73, 440, 32);
        let mut btn_add_item = Button::new(585, 73, 260, 32, "Cari / tambah obat");
        btn_add_item.set_color(AppTheme::PRIMARY);
        btn_add_item.set_label_color(Color::White);
        btn_add_item.set_label_font(Font::HelveticaBold);
        btn_add_item.set_frame(FrameType::RFlatBox);

        let mut lbl_warning_rugi = Frame::new(700, 74, 300, 30, "");
        lbl_warning_rugi.set_label_color(AppTheme::DANGER_TEXT);
        lbl_warning_rugi.set_label_font(Font::HelveticaBold);
        lbl_warning_rugi.set_label_size(12);

        // ==========================================
        // 3. TABEL KERANJANG BELANJA (1000 x 360)
        // ==========================================
        let mut cart_table = Table::new(12, 122, 1000, 355, "");
        cart_table.end();
        cart_table.set_row_height_all(34);
        cart_table.set_col_header_height(32);
        cart_table.set_color(Color::White);
        cart_table.set_cols(7);
        cart_table.set_col_header(true);
        cart_table.set_col_width(0, 50); // No
        cart_table.set_col_width(1, 140); // Barcode
        cart_table.set_col_width(2, 300); // Nama Obat
        cart_table.set_col_width(3, 90); // Satuan
        cart_table.set_col_width(4, 70); // Qty
        cart_table.set_col_width(5, 120); // Harga Satuan
        cart_table.set_col_width(6, 150); // Subtotal

        let rows_data = Rc::new(RefCell::new(vec![
            MockCartRow {
                no: 1,
                barcode: "8999909001".to_string(),
                name: "Paracetamol 500mg Tablet".to_string(),
                unit: "Strip".to_string(),
                qty: 2,
                price: 9500.0,
                subtotal: 19000.0,
            },
            MockCartRow {
                no: 2,
                barcode: "8999909002".to_string(),
                name: "Amoxicillin 500mg Kapsul".to_string(),
                unit: "Strip".to_string(),
                qty: 1,
                price: 14000.0,
                subtotal: 14000.0,
            },
            MockCartRow {
                no: 3,
                barcode: "8999909003".to_string(),
                name: "Bodrex Extra Tablet".to_string(),
                unit: "Blister".to_string(),
                qty: 3,
                price: 4500.0,
                subtotal: 13500.0,
            },
        ]));

        cart_table.set_rows(rows_data.borrow().len() as i32);

        let r_clone = rows_data.clone();
        cart_table.draw_cell(move |_t, ctx, row, col, x, y, w, h| match ctx {
            fltk::table::TableContext::StartPage => {
                fltk::app::set_font(Font::Helvetica);
            }
            fltk::table::TableContext::ColHeader => {
                draw::push_clip(x, y, w, h);
                draw::draw_box(FrameType::FlatBox, x, y, w, h, AppTheme::TABLE_HEADER);
                draw::set_draw_color(AppTheme::TEXT_MUTED);
                draw::set_font(Font::HelveticaBold, 11);
                let title = match col {
                    0 => "NO",
                    1 => "KODE / BARCODE",
                    2 => "NAMA OBAT / BARANG",
                    3 => "SATUAN",
                    4 => "QTY",
                    5 => "HARGA (RP)",
                    6 => "SUBTOTAL (RP)",
                    _ => "",
                };
                let align = if col >= 4 { Align::Right } else { Align::Left };
                draw::draw_text2(title, x + 8, y, w - 16, h, align);
                draw::set_draw_color(AppTheme::BORDER);
                draw::draw_line(x, y + h - 1, x + w, y + h - 1);
                draw::pop_clip();
            }
            fltk::table::TableContext::Cell => {
                let items = r_clone.borrow();
                if let Some(item) = items.get(row as usize) {
                    draw::push_clip(x, y, w, h);
                    let bg = if row % 2 == 0 {
                        Color::White
                    } else {
                        Color::from_hex(0xF8FAFC)
                    };
                    draw::draw_box(FrameType::FlatBox, x, y, w, h, bg);
                    draw::set_draw_color(Color::from_hex(0xF1F5F9));
                    draw::draw_line(x, y + h - 1, x + w, y + h - 1);

                    draw::set_draw_color(AppTheme::TEXT_BODY);
                    draw::set_font(Font::Helvetica, 12);

                    let txt = match col {
                        0 => item.no.to_string(),
                        1 => item.barcode.clone(),
                        2 => item.name.clone(),
                        3 => item.unit.clone(),
                        4 => item.qty.to_string(),
                        5 => format!("{:.0}", item.price),
                        6 => format!("{:.0}", item.subtotal),
                        _ => String::new(),
                    };
                    let align = if col >= 4 { Align::Right } else { Align::Left };
                    draw::draw_text2(&txt, x + 8, y, w - 16, h, align);
                    draw::pop_clip();
                }
            }
            _ => {}
        });

        // ==========================================
        // 4. BOTTOM ELEVATED CHECKOUT CARD (1000 x 185)
        // ==========================================
        let mut btm_card = Frame::new(12, 485, 1000, 185, "");
        btm_card.set_frame(FrameType::RFlatBox);
        btm_card.set_color(Color::White);

        // Display Total Besar di kanan atas panel bawah
        let mut lbl_total_title = Frame::new(600, 495, 395, 18, "TOTAL TAGIHAN");
        lbl_total_title.set_align(Align::Right | Align::Inside);
        lbl_total_title.set_label_size(11);
        lbl_total_title.set_label_font(Font::HelveticaBold);
        lbl_total_title.set_label_color(AppTheme::TEXT_MUTED);

        let mut lbl_grand_total = Frame::new(600, 515, 395, 48, "Rp 46.500,-");
        lbl_grand_total.set_align(Align::Right | Align::Inside);
        lbl_grand_total.set_label_size(36);
        lbl_grand_total.set_label_font(Font::HelveticaBold);
        lbl_grand_total.set_label_color(Color::from_hex(0x0F172A)); // Modern Slate 900 bold total

        // Input Bayar (F2) & Kembali di kiri atas panel bawah
        let mut lbl_bayar_title = Frame::new(26, 502, 115, 32, "[F2] Bayar (Rp):");
        lbl_bayar_title.set_label_font(Font::HelveticaBold);
        lbl_bayar_title.set_label_size(13);
        lbl_bayar_title.set_label_color(AppTheme::TEXT_TITLE);
        lbl_bayar_title.set_align(Align::Left | Align::Inside);

        let mut input_bayar = Input::new(145, 500, 180, 36, "");
        input_bayar.set_value("50000");
        input_bayar.set_text_size(18);
        input_bayar.set_text_font(Font::HelveticaBold);
        input_bayar.set_frame(FrameType::RFlatBox);
        input_bayar.set_color(Color::from_hex(0xF0FDFA)); // Soft Teal

        let mut lbl_kembali_title = Frame::new(26, 546, 115, 30, "Kembalian:");
        lbl_kembali_title.set_label_font(Font::HelveticaBold);
        lbl_kembali_title.set_label_size(13);
        lbl_kembali_title.set_label_color(AppTheme::TEXT_MUTED);
        lbl_kembali_title.set_align(Align::Left | Align::Inside);

        let mut lbl_kembali = Frame::new(145, 546, 180, 30, "Rp 3.500");
        lbl_kembali.set_label_size(18);
        lbl_kembali.set_label_font(Font::HelveticaBold);
        lbl_kembali.set_label_color(AppTheme::SUCCESS);
        lbl_kembali.set_align(Align::Left | Align::Inside);

        // Tombol Baris Bawah (F3 - F7)
        let btn_y = 612;
        let mut btn_simpan = Button::new(26, btn_y, 140, 44, "[F3] Simpan");
        btn_simpan.set_color(AppTheme::SUCCESS);
        btn_simpan.set_label_color(Color::White);
        btn_simpan.set_label_font(Font::HelveticaBold);
        btn_simpan.set_frame(FrameType::RFlatBox);

        let mut btn_cetak = Button::new(176, btn_y, 140, 44, "[F4] Cetak Struk");
        btn_cetak.set_color(AppTheme::PRIMARY);
        btn_cetak.set_label_color(Color::White);
        btn_cetak.set_label_font(Font::HelveticaBold);
        btn_cetak.set_frame(FrameType::RFlatBox);

        let mut btn_baru = Button::new(326, btn_y, 125, 44, "[F5] Baru");
        btn_baru.set_color(Color::from_hex(0xF1F5F9));
        btn_baru.set_label_color(AppTheme::TEXT_BODY);
        btn_baru.set_label_font(Font::HelveticaBold);
        btn_baru.set_frame(FrameType::RFlatBox);

        let mut btn_pending = Button::new(461, btn_y, 125, 44, "[F6] Pending");
        btn_pending.set_color(Color::from_hex(0xFEF3C7));
        btn_pending.set_label_color(AppTheme::WARNING_TEXT);
        btn_pending.set_label_font(Font::HelveticaBold);
        btn_pending.set_frame(FrameType::RFlatBox);

        let mut btn_buka_pending = Button::new(596, btn_y, 150, 44, "[F7] Buka Pending");
        btn_buka_pending.set_color(Color::from_hex(0xF1F5F9));
        btn_buka_pending.set_label_color(AppTheme::TEXT_BODY);
        btn_buka_pending.set_label_font(Font::HelveticaBold);
        btn_buka_pending.set_frame(FrameType::RFlatBox);

        win.end();
        win.make_resizable(false);

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
            lbl_operator,
            lbl_warning_rugi,
            cart_items: Vec::new(),
            rows_data,
        }
    }
}
