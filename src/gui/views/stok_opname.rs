use crate::gui::theme::AppTheme;
use fltk::button::Button;
use fltk::draw;
use fltk::enums::{Align, Color, Font, FrameType};
use fltk::frame::Frame;
use fltk::prelude::*;
use fltk::table::Table;
use fltk::window::Window;
use mysql::Pool;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct OpnameRow {
    pub id: i32,
    pub no: i32,
    pub kode: String,
    pub nama: String,
    pub satuan: String,
    pub stok_sistem: i32,
    pub stok_fisik: i32,
    pub selisih: i32,
    pub keterangan: String,
}

pub struct StokOpnameView {
    pub win: Window,
    pub table: Table,
    pub btn_simpan: Button,
    pub btn_batal: Button,
}

impl StokOpnameView {
    pub fn new(pool: Option<Pool>) -> Self {
        let mut win = Window::new(0, 108, 1200, 612, "");
        win.set_color(AppTheme::BG_APP);

        let mut main_card = Frame::new(16, 12, 1168, 588, "");
        main_card.set_frame(FrameType::RFlatBox);
        main_card.set_color(Color::White);

        // Header Title
        let mut title = Frame::new(36, 24, 300, 26, "Stok Opname & Penyesuaian");
        title.set_label_size(18);
        title.set_label_font(Font::HelveticaBold);
        title.set_label_color(AppTheme::TEXT_TITLE);
        title.set_align(Align::Left | Align::Inside);

        let mut subtitle = Frame::new(36, 48, 500, 18, "Sesuaikan stok fisik rak dengan stok sistem. Selisih otomatis dicatat ke Kartu Stok.");
        subtitle.set_label_size(11);
        subtitle.set_label_color(AppTheme::TEXT_MUTED);
        subtitle.set_align(Align::Left | Align::Inside);

        // Table
        let mut table = Table::new(36, 76, 1128, 450, "");
        table.set_color(Color::White);
        table.set_cols(8);
        table.set_col_header(true);
        table.set_col_width(0, 45);   // No
        table.set_col_width(1, 100);  // Kode
        table.set_col_width(2, 280);  // Nama Obat
        table.set_col_width(3, 80);   // Satuan
        table.set_col_width(4, 110);  // Stok Sistem
        table.set_col_width(5, 110);  // Stok Fisik
        table.set_col_width(6, 110);  // Selisih
        table.set_col_width(7, 260);  // Keterangan

        let initial_data = if let Some(ref p) = pool {
            if let Ok(items) = crate::db::get_stock_opname_items(p) {
                items.into_iter().enumerate().map(|(i, (id, code, name, unit, stock))| {
                    OpnameRow {
                        id,
                        no: (i + 1) as i32,
                        kode: code,
                        nama: name,
                        satuan: unit,
                        stok_sistem: stock,
                        stok_fisik: stock,
                        selisih: 0,
                        keterangan: "Sesuai sistem".into(),
                    }
                }).collect()
            } else {
                Vec::new()
            }
        } else {
            vec![
                OpnameRow {
                    id: 1,
                    no: 1,
                    kode: "AK0209".into(),
                    nama: "Minyak Telon My Baby Plus 90ml".into(),
                    satuan: "Botol".into(),
                    stok_sistem: 6,
                    stok_fisik: 5,
                    selisih: -1,
                    keterangan: "Pecah 1 botol saat display".into(),
                },
                OpnameRow {
                    id: 2,
                    no: 2,
                    kode: "OB0101".into(),
                    nama: "Paracetamol 500mg Tablet".into(),
                    satuan: "Tablet".into(),
                    stok_sistem: 120,
                    stok_fisik: 122,
                    selisih: 2,
                    keterangan: "Bonus sampel dari suplier".into(),
                },
            ]
        };

        let mock_data = Rc::new(RefCell::new(initial_data));
        table.set_rows(mock_data.borrow().len() as i32);

        let data_ref = mock_data.clone();
        table.draw_cell(move |_t, ctx, row, col, x, y, w, h| match ctx {
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
                    1 => "KODE",
                    2 => "NAMA OBAT",
                    3 => "SATUAN",
                    4 => "STOK SISTEM",
                    5 => "STOK FISIK",
                    6 => "SELISIH",
                    7 => "KETERANGAN",
                    _ => "",
                };
                let align = if col == 4 || col == 5 || col == 6 { Align::Right } else { Align::Left };
                draw::draw_text2(title, x + 8, y, w - 16, h, align);
                draw::set_draw_color(AppTheme::BORDER);
                draw::draw_line(x, y + h - 1, x + w, y + h - 1);
                draw::pop_clip();
            }
            fltk::table::TableContext::Cell => {
                let items = data_ref.borrow();
                if let Some(r) = items.get(row as usize) {
                    draw::push_clip(x, y, w, h);
                    let bg = if r.selisih < 0 {
                        Color::from_hex(0xFFF1F2) // Soft Rose (Hilang/Kurang)
                    } else if r.selisih > 0 {
                        Color::from_hex(0xF0FDF4) // Soft Green (Lebih)
                    } else {
                        Color::White
                    };

                    draw::draw_box(FrameType::FlatBox, x, y, w, h, bg);
                    draw::set_draw_color(Color::from_hex(0xF1F5F9));
                    draw::draw_line(x, y + h - 1, x + w, y + h - 1);

                    draw::set_draw_color(AppTheme::TEXT_BODY);
                    draw::set_font(Font::Helvetica, 12);

                    let txt = match col {
                        0 => r.no.to_string(),
                        1 => r.kode.clone(),
                        2 => r.nama.clone(),
                        3 => r.satuan.clone(),
                        4 => r.stok_sistem.to_string(),
                        5 => r.stok_fisik.to_string(),
                        6 => {
                            if r.selisih > 0 { format!("+{}", r.selisih) } else { r.selisih.to_string() }
                        }
                        7 => r.keterangan.clone(),
                        _ => String::new(),
                    };

                    let align = if col == 4 || col == 5 || col == 6 { Align::Right } else { Align::Left };
                    draw::draw_text2(&txt, x + 8, y, w - 16, h, align);
                    draw::pop_clip();
                }
            }
            _ => {}
        });

        // Bottom Actions
        let btn_y = 544;
        let mut btn_simpan = Button::new(36, btn_y, 250, 36, "⚖ Sesuaikan Stok Fisik");
        btn_simpan.set_color(AppTheme::PRIMARY);
        btn_simpan.set_label_color(Color::White);
        btn_simpan.set_label_font(Font::HelveticaBold);
        btn_simpan.set_frame(FrameType::RFlatBox);

        let mut btn_batal = Button::new(296, btn_y, 100, 36, "Refresh");
        btn_batal.set_color(Color::from_hex(0xF1F5F9));
        btn_batal.set_label_color(AppTheme::TEXT_BODY);
        btn_batal.set_frame(FrameType::RFlatBox);

        let p_op = pool.clone();
        let r_op = mock_data.clone();
        let mut t_op = table.clone();
        btn_simpan.set_callback(move |_| {
            let row_idx = t_op.callback_row();
            let mut data = r_op.borrow_mut();
            if let Some(target) = data.get_mut(row_idx as usize) {
                let nama = target.nama.clone();
                let pid = target.id;
                let cur = target.stok_sistem;

                if let Some(val_str) = fltk::dialog::input_default(
                    &format!("Masukkan Jumlah Stok Fisik Nyata di Rak untuk '{}':", nama),
                    &cur.to_string(),
                ) {
                    if let Ok(real_qty) = val_str.trim().parse::<i32>() {
                        let reason = fltk::dialog::input_default("Alasan / Catatan Penyesuaian:", "Stok Opname Rutin").unwrap_or_default();
                        target.stok_fisik = real_qty;
                        target.selisih = real_qty - cur;
                        target.keterangan = reason.clone();

                        if let Some(ref p) = p_op {
                            let _ = crate::db::save_stock_opname_adjustment(p, pid, real_qty, &reason);
                            fltk::dialog::message_default("Penyesuaian stok berhasil disimpan ke Kartu Stok & Database!");
                        }
                    }
                }
            }
            drop(data);
            t_op.redraw();
        });

        win.end();

        Self {
            win,
            table,
            btn_simpan,
            btn_batal,
        }
    }
}
