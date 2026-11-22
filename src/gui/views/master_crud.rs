use crate::gui::theme::AppTheme;
use fltk::button::Button;
use fltk::draw;
use fltk::enums::{Align, Color, Font, FrameType};
use fltk::frame::Frame;
use fltk::input::Input;
use fltk::prelude::*;
use fltk::table::Table;
use fltk::window::Window;
use mysql::Pool;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct SimpleMasterConfig {
    pub title: &'static str,
    pub count_str: String,
    pub col_names: Vec<&'static str>,
    pub col_widths: Vec<i32>,
    pub initial_rows: Vec<Vec<String>>,
}

pub struct MasterSimpleView {
    pub win: Window,
    pub table: Table,
    pub input_search: Input,
    pub btn_tambah: Button,
    pub btn_edit: Button,
    pub btn_hapus: Button,
    pub rows: Rc<RefCell<Vec<Vec<String>>>>,
}

impl MasterSimpleView {
    pub fn new(config: SimpleMasterConfig) -> Self {
        let mut win = Window::new(0, 108, 1200, 612, "");
        win.set_color(AppTheme::BG_APP);

        let mut main_card = Frame::new(16, 12, 1168, 588, "");
        main_card.set_frame(FrameType::RFlatBox);
        main_card.set_color(Color::White);

        // Title & Count Badge
        let mut title = Frame::new(36, 24, 260, 26, config.title);
        title.set_label_size(18);
        title.set_label_font(Font::HelveticaBold);
        title.set_label_color(AppTheme::TEXT_TITLE);
        title.set_align(Align::Left | Align::Inside);

        let mut badge_count = Frame::new(300, 26, 100, 22, "");
        badge_count.set_label(&config.count_str);
        badge_count.set_frame(FrameType::RFlatBox);
        badge_count.set_color(Color::from_hex(0xF1F5F9));
        badge_count.set_label_color(AppTheme::TEXT_MUTED);
        badge_count.set_label_size(11);
        badge_count.set_label_font(Font::HelveticaBold);

        // Table
        let cols_count = config.col_names.len() as i32;
        let mut table = Table::new(36, 72, 1128, 455, "");
        table.set_color(Color::White);
        table.set_cols(cols_count);
        table.set_col_header(true);

        for (i, w) in config.col_widths.iter().enumerate() {
            table.set_col_width(i as i32, *w);
        }

        let rows = Rc::new(RefCell::new(config.initial_rows));
        table.set_rows(rows.borrow().len() as i32);

        let r_ref = rows.clone();
        let headers = config.col_names.clone();

        table.draw_cell(move |_t, ctx, row, col, x, y, w, h| match ctx {
            fltk::table::TableContext::StartPage => {
                fltk::app::set_font(Font::Helvetica);
            }
            fltk::table::TableContext::ColHeader => {
                draw::push_clip(x, y, w, h);
                draw::draw_box(FrameType::FlatBox, x, y, w, h, AppTheme::TABLE_HEADER);
                draw::set_draw_color(AppTheme::TEXT_MUTED);
                draw::set_font(Font::HelveticaBold, 11);
                if let Some(t) = headers.get(col as usize) {
                    draw::draw_text2(t, x + 8, y, w - 16, h, Align::Left);
                }
                draw::set_draw_color(AppTheme::BORDER);
                draw::draw_line(x, y + h - 1, x + w, y + h - 1);
                draw::pop_clip();
            }
            fltk::table::TableContext::Cell => {
                let data = r_ref.borrow();
                if let Some(r) = data.get(row as usize) {
                    draw::push_clip(x, y, w, h);
                    let bg = if row % 2 == 0 { Color::White } else { Color::from_hex(0xF8FAFC) };
                    draw::draw_box(FrameType::FlatBox, x, y, w, h, bg);

                    draw::set_draw_color(Color::from_hex(0xF1F5F9));
                    draw::draw_line(x, y + h - 1, x + w, y + h - 1);

                    draw::set_draw_color(AppTheme::TEXT_BODY);
                    draw::set_font(Font::Helvetica, 12);
                    if let Some(val) = r.get(col as usize) {
                        draw::draw_text2(val, x + 8, y, w - 16, h, Align::Left);
                    }
                    draw::pop_clip();
                }
            }
            _ => {}
        });

        // Bottom Actions
        let btn_y = 544;
        let mut btn_tambah = Button::new(36, btn_y, 130, 36, "+ Tambah Baru");
        btn_tambah.set_color(AppTheme::PRIMARY);
        btn_tambah.set_label_color(Color::White);
        btn_tambah.set_label_font(Font::HelveticaBold);
        btn_tambah.set_frame(FrameType::RFlatBox);

        let mut btn_edit = Button::new(174, btn_y, 75, 36, "Edit");
        btn_edit.set_color(Color::from_hex(0xF1F5F9));
        btn_edit.set_label_color(AppTheme::TEXT_BODY);
        btn_edit.set_frame(FrameType::RFlatBox);

        let mut btn_hapus = Button::new(256, btn_y, 75, 36, "Hapus");
        btn_hapus.set_color(Color::from_hex(0xFEE2E2));
        btn_hapus.set_label_color(AppTheme::DANGER_TEXT);
        btn_hapus.set_frame(FrameType::RFlatBox);

        let mut input_search = Input::new(340, btn_y, 280, 36, "");
        input_search.set_frame(FrameType::RFlatBox);
        input_search.set_color(Color::from_hex(0xF8FAFC));
        input_search.set_text_size(13);
        input_search.set_value("");

        let mut _lbl_page = Frame::new(635, btn_y, 160, 36, "◀   Halaman 1 dari 1   ▶");
        _lbl_page.set_label_size(12);
        _lbl_page.set_label_color(AppTheme::TEXT_MUTED);

        let mut btn_refresh = Button::new(1070, btn_y, 94, 36, "🔄 Refresh");
        btn_refresh.set_frame(FrameType::RFlatBox);
        btn_refresh.set_color(Color::from_hex(0xF1F5F9));

        win.end();

        Self {
            win,
            table,
            input_search,
            btn_tambah,
            btn_edit,
            btn_hapus,
            rows,
        }
    }

    // Builder presets untuk setiap Master Entity
    pub fn suplier(pool: Option<Pool>) -> Self {
        let initial_rows = if let Some(ref p) = pool {
            if let Ok(sups) = crate::db::get_suppliers(p) {
                sups.into_iter().enumerate().map(|(i, (id, name, phone, addr))| {
                    vec![
                        (i + 1).to_string(),
                        format!("SUP{:03}", id),
                        name,
                        addr,
                        phone,
                        "-".into(),
                        "-".into(),
                    ]
                }).collect()
            } else {
                Vec::new()
            }
        } else {
            vec![
                vec!["1".into(), "01 SP BSP".into(), "PT Bina San Prima".into(), "Jl. Letjen Sutoyo No. 138".into(), "(0331) 332928".into(), "bsp@prima.com".into(), "BCA 4650274771".into()],
                vec!["2".into(), "02 SP BMJB".into(), "PT Bina Mitra Jaya Bersama".into(), "Kawasan Pergudangan Blok C".into(), "0812-3344-5566".into(), "sales@bmjb.co.id".into(), "Mandiri 14200192".into()],
            ]
        };

        let count = initial_rows.len();
        let mut view = Self::new(SimpleMasterConfig {
            title: "Master Data Suplier",
            count_str: format!("{} Suplier", count),
            col_names: vec!["NO", "KODE", "NAMA SUPLIER", "ALAMAT", "NO. HP / TELP", "EMAIL", "REKENING"],
            col_widths: vec![45, 100, 240, 280, 140, 160, 160],
            initial_rows,
        });

        let p_add = pool.clone();
        let r_add = view.rows.clone();
        let mut t_add = view.table.clone();
        view.btn_tambah.set_callback(move |_| {
            if let Some(nama) = fltk::dialog::input_default("Masukkan Nama Suplier:", "") {
                if nama.trim().is_empty() { return; }
                let phone = fltk::dialog::input_default("Masukkan No Telp / HP Suplier:", "").unwrap_or_default();
                let addr = fltk::dialog::input_default("Masukkan Alamat Suplier:", "").unwrap_or_default();

                if let Some(ref p) = p_add {
                    if let Ok(new_id) = crate::db::insert_supplier(p, &nama, &phone, &addr) {
                        let mut data = r_add.borrow_mut();
                        let no = (data.len() + 1).to_string();
                        data.push(vec![no, format!("SUP{:03}", new_id), nama, addr, phone, "-".into(), "-".into()]);
                        t_add.set_rows(data.len() as i32);
                        t_add.redraw();
                        fltk::dialog::message_default("Suplier baru berhasil disimpan ke Database!");
                    }
                }
            }
        });

        view
    }

    pub fn pelanggan(pool: Option<Pool>) -> Self {
        let initial_rows = if let Some(ref p) = pool {
            if let Ok(custs) = crate::db::get_customers(p) {
                custs.into_iter().enumerate().map(|(i, (id, name, phone, addr))| {
                    vec![
                        (i + 1).to_string(),
                        format!("PLG{:03}", id),
                        name,
                        phone,
                        addr,
                        "Reguler".into(),
                    ]
                }).collect()
            } else {
                Vec::new()
            }
        } else {
            vec![
                vec!["1".into(), "PLG001".into(), "Bpk. Budi Santoso".into(), "0812-1111-2222".into(), "Jl. Kenanga No. 8".into(), "Reguler".into()],
                vec!["2".into(), "PLG002".into(), "Ibu Siti Rahmawati".into(), "0813-2222-3333".into(), "Jl. Melati No. 14".into(), "VIP (Diskon 5%)".into()],
            ]
        };

        let count = initial_rows.len();
        let mut view = Self::new(SimpleMasterConfig {
            title: "Master Data Pelanggan / Pasien",
            count_str: format!("{} Pasien", count),
            col_names: vec!["NO", "KODE", "NAMA PELANGGAN", "NO. HP", "ALAMAT", "STATUS MEMBER"],
            col_widths: vec![45, 100, 260, 150, 380, 150],
            initial_rows,
        });

        let p_add = pool.clone();
        let r_add = view.rows.clone();
        let mut t_add = view.table.clone();
        view.btn_tambah.set_callback(move |_| {
            if let Some(nama) = fltk::dialog::input_default("Masukkan Nama Pasien / Pelanggan:", "") {
                if nama.trim().is_empty() { return; }
                let phone = fltk::dialog::input_default("Masukkan No HP Pasien:", "").unwrap_or_default();
                let addr = fltk::dialog::input_default("Masukkan Alamat:", "").unwrap_or_default();

                if let Some(ref p) = p_add {
                    if let Ok(new_id) = crate::db::insert_customer(p, &nama, &phone, &addr) {
                        let mut data = r_add.borrow_mut();
                        let no = (data.len() + 1).to_string();
                        data.push(vec![no, format!("PLG{:03}", new_id), nama, phone, addr, "Reguler".into()]);
                        t_add.set_rows(data.len() as i32);
                        t_add.redraw();
                        fltk::dialog::message_default("Pelanggan baru berhasil disimpan ke Database!");
                    }
                }
            }
        });

        view
    }

    pub fn dokter(pool: Option<Pool>) -> Self {
        let initial_rows = if let Some(ref p) = pool {
            if let Ok(docs) = crate::db::get_doctors(p) {
                docs.into_iter().enumerate().map(|(i, (id, name, sip))| {
                    vec![
                        (i + 1).to_string(),
                        format!("DOC{:02}", id),
                        name,
                        sip,
                        "Dokter Rekanan".into(),
                        "-".into(),
                    ]
                }).collect()
            } else {
                Vec::new()
            }
        } else {
            vec![
                vec!["1".into(), "DOC01".into(), "dr. H. Rahmat Hidayat, Sp.PD".into(), "SIP/440/012/2023".into(), "Penyakit Dalam".into(), "0811-9988-7711".into()],
                vec!["2".into(), "DOC02".into(), "dr. Siti Aminah, Sp.A".into(), "SIP/440/098/2024".into(), "Spesialis Anak".into(), "0812-4455-6677".into()],
            ]
        };

        let count = initial_rows.len();
        let mut view = Self::new(SimpleMasterConfig {
            title: "Master Data Dokter Rekanan",
            count_str: format!("{} Dokter", count),
            col_names: vec!["NO", "KODE", "NAMA DOKTER", "NOMOR SIP", "SPESIALISASI", "NO. HP"],
            col_widths: vec![45, 90, 280, 220, 220, 180],
            initial_rows,
        });

        let p_add = pool.clone();
        let r_add = view.rows.clone();
        let mut t_add = view.table.clone();
        view.btn_tambah.set_callback(move |_| {
            if let Some(nama) = fltk::dialog::input_default("Masukkan Nama Dokter:", "dr. ") {
                if nama.trim().is_empty() { return; }
                let sip = fltk::dialog::input_default("Masukkan Nomor SIP:", "SIP/440/").unwrap_or_default();

                if let Some(ref p) = p_add {
                    if let Ok(new_id) = crate::db::insert_doctor(p, &nama, &sip) {
                        let mut data = r_add.borrow_mut();
                        let no = (data.len() + 1).to_string();
                        data.push(vec![no, format!("DOC{:02}", new_id), nama, sip, "Dokter Rekanan".into(), "-".into()]);
                        t_add.set_rows(data.len() as i32);
                        t_add.redraw();
                        fltk::dialog::message_default("Dokter baru berhasil disimpan ke Database!");
                    }
                }
            }
        });

        view
    }

    pub fn karyawan(pool: Option<Pool>) -> Self {
        let initial_rows = if let Some(ref p) = pool {
            if let Ok(users) = crate::db::get_users_list(p) {
                users.into_iter().enumerate().map(|(i, (_id, uname, fname, role, act))| {
                    let status = if act == 1 { "Aktif" } else { "Non-Aktif" };
                    vec![
                        (i + 1).to_string(),
                        uname,
                        fname,
                        role,
                        status.into(),
                    ]
                }).collect()
            } else {
                Vec::new()
            }
        } else {
            vec![
                vec!["1".into(), "admin".into(), "Administrator Utama".into(), "ADMINISTRATOR (Full)".into(), "Aktif".into()],
                vec!["2".into(), "kasir01".into(), "Siti Fatimah (Kasir Pagi)".into(), "KASIR (POS Saja)".into(), "Aktif".into()],
            ]
        };

        let count = initial_rows.len();
        let view = Self::new(SimpleMasterConfig {
            title: "Master Data Karyawan & Operator",
            count_str: format!("{} Akun", count),
            col_names: vec!["NO", "USERNAME", "NAMA LENGKAP", "HAK AKSES (ROLE)", "STATUS"],
            col_widths: vec![45, 140, 320, 200, 160],
            initial_rows,
        });

        view
    }
}
