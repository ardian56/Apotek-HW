use crate::gui::theme::AppTheme;
use crate::models::Product;
use fltk::button::Button;
use fltk::draw;
use fltk::enums::{Align, Color, Font, FrameType};
use fltk::frame::Frame;
use fltk::group::{Flex, Group};
use fltk::input::Input;
use fltk::prelude::*;
use fltk::table::Table;
use fltk::window::Window;
use std::cell::RefCell;
use std::rc::Rc;

pub struct MasterObatView {
    pub win: Window,
    pub input_search: Input,
    pub btn_tambah: Button,
    pub btn_import_excel: Button,
    pub btn_export_excel: Button,
    pub btn_refresh: Button,
    pub table: Table,
    pub products: Rc<RefCell<Vec<Product>>>,
}

impl MasterObatView {
    pub fn new() -> Self {
        let mut win = Window::default()
            .with_size(1024, 640)
            .with_label("Master Data Obat & Inventori Apotek");
        win.set_color(AppTheme::BG_APP);

        let mut root = Flex::default().size_of_parent().column();
        root.set_pad(10);
        root.set_margin(12);

        // Header Action Bar
        let mut action_bar = Group::default().with_size(0, 48);
        action_bar.set_color(AppTheme::BG_CARD);
        action_bar.set_frame(FrameType::FlatBox);

        let mut lbl_search = Frame::new(action_bar.x() + 12, action_bar.y() + 10, 80, 30, "Cari Obat:");
        lbl_search.set_label_font(Font::HelveticaBold);
        lbl_search.set_align(Align::Left | Align::Inside);

        let mut input_search = Input::new(lbl_search.x() + lbl_search.w() + 5, action_bar.y() + 8, 240, 32, "");
        input_search.set_frame(FrameType::FlatBox);
        input_search.set_color(Color::from_hex(0xF0F4F8));

        let mut btn_refresh = Button::new(input_search.x() + input_search.w() + 10, action_bar.y() + 8, 90, 32, "Refresh");
        btn_refresh.set_color(Color::from_hex(0x607D8B));
        btn_refresh.set_label_color(Color::White);
        btn_refresh.set_frame(FrameType::FlatBox);

        let mut btn_tambah = Button::new(btn_refresh.x() + btn_refresh.w() + 15, action_bar.y() + 8, 120, 32, "+ Tambah Obat");
        btn_tambah.set_color(AppTheme::PRIMARY);
        btn_tambah.set_label_color(Color::White);
        btn_tambah.set_label_font(Font::HelveticaBold);
        btn_tambah.set_frame(FrameType::FlatBox);

        let mut btn_import_excel = Button::new(btn_tambah.x() + btn_tambah.w() + 10, action_bar.y() + 8, 110, 32, "Import Excel");
        btn_import_excel.set_color(AppTheme::ACCENT);
        btn_import_excel.set_label_color(Color::White);
        btn_import_excel.set_frame(FrameType::FlatBox);

        let mut btn_export_excel = Button::new(btn_import_excel.x() + btn_import_excel.w() + 10, action_bar.y() + 8, 110, 32, "Export Excel");
        btn_export_excel.set_color(Color::from_hex(0x4CAF50));
        btn_export_excel.set_label_color(Color::White);
        btn_export_excel.set_frame(FrameType::FlatBox);

        action_bar.end();
        root.fixed(&action_bar, 48);

        // Keterangan Indikator Warna
        let mut legend_bar = Group::default().with_size(0, 24);
        let mut leg_yellow = Frame::new(legend_bar.x() + 10, legend_bar.y(), 180, 24, "■ Stok <= Minimal (Kuning)");
        leg_yellow.set_label_color(Color::from_hex(0xE65100));
        leg_yellow.set_label_size(12);

        let mut leg_red = Frame::new(leg_yellow.x() + 190, legend_bar.y(), 220, 24, "■ Stok Habis / Expired (Merah)");
        leg_red.set_label_color(AppTheme::DANGER_TEXT);
        leg_red.set_label_size(12);
        legend_bar.end();
        root.fixed(&legend_bar, 24);

        // Tabel Master Obat
        let mut table = Table::default();
        table.set_color(Color::White);
        table.set_rows(0);
        table.set_cols(8);
        table.set_col_header(true);
        table.set_col_width(0, 50);   // ID
        table.set_col_width(1, 130);  // Barcode
        table.set_col_width(2, 280);  // Nama Obat
        table.set_col_width(3, 110);  // Kategori
        table.set_col_width(4, 90);   // Satuan
        table.set_col_width(5, 80);   // Min
        table.set_col_width(6, 80);   // Total Stok
        table.set_col_width(7, 120);  // Harga Beli

        let products = Rc::new(RefCell::new(Vec::<Product>::new()));
        let prods_ref = products.clone();

        // Custom Draw Callback untuk highlight baris Kuning / Merah
        table.draw_cell(move |_t, ctx, row, col, x, y, w, h| match ctx {
            fltk::table::TableContext::StartPage => {
                fltk::app::set_font(Font::Helvetica);
            }
            fltk::table::TableContext::ColHeader => {
                draw::push_clip(x, y, w, h);
                draw::draw_box(FrameType::FlatBox, x, y, w, h, Color::from_hex(0xE0E0E0));
                draw::set_draw_color(Color::Black);
                draw::set_font(Font::HelveticaBold, 12);
                let title = match col {
                    0 => "ID",
                    1 => "Kode / Barcode",
                    2 => "Nama Obat",
                    3 => "Kategori",
                    4 => "Satuan Dasar",
                    5 => "Min",
                    6 => "Stok",
                    7 => "Harga Beli",
                    _ => "",
                };
                draw::draw_text2(title, x + 4, y, w - 8, h, Align::Left);
                draw::pop_clip();
            }
            fltk::table::TableContext::Cell => {
                let prods = prods_ref.borrow();
                if let Some(p) = prods.get(row as usize) {
                    draw::push_clip(x, y, w, h);

                    // Warna background berdasarkan status stok/expired
                    let bg_color = if p.total_stock <= 0 {
                        AppTheme::DANGER_BG
                    } else if p.total_stock <= p.min_stock {
                        AppTheme::WARNING_BG
                    } else {
                        Color::White
                    };

                    draw::draw_box(FrameType::FlatBox, x, y, w, h, bg_color);
                    draw::set_draw_color(Color::Black);
                    draw::set_font(Font::Helvetica, 12);

                    let txt = match col {
                        0 => p.id.to_string(),
                        1 => p.code.clone(),
                        2 => p.name.clone(),
                        3 => p.category.clone(),
                        4 => p.base_unit.clone(),
                        5 => p.min_stock.to_string(),
                        6 => p.total_stock.to_string(),
                        7 => format!("Rp {:.0}", p.buy_price),
                        _ => String::new(),
                    };

                    let align = if col >= 5 { Align::Right } else { Align::Left };
                    draw::draw_text2(&txt, x + 4, y, w - 8, h, align);
                    draw::pop_clip();
                }
            }
            _ => {}
        });

        root.end();
        win.end();

        Self {
            win,
            input_search,
            btn_tambah,
            btn_import_excel,
            btn_export_excel,
            btn_refresh,
            table,
            products,
        }
    }
}
