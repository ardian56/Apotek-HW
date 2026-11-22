use crate::gui::theme::AppTheme;
use crate::models::Product;
use fltk::button::Button;
use fltk::draw;
use fltk::enums::{Align, Color, Font, FrameType};
use fltk::frame::Frame;
use fltk::input::Input;
use fltk::menu::Choice;
use fltk::prelude::*;
use fltk::table::Table;
use fltk::window::Window;
use mysql::Pool;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone)]
pub struct MockObatRow {
    pub id: i32,
    pub no: i32,
    pub kode: String,
    pub nama: String,
    pub kategori: String,
    pub stok: i32,
    pub satuan: String,
    pub harga_beli: f64,
    pub harga_jual: f64,
    pub suplier: String,
    pub expired: String,
    pub status_badge: &'static str, // "Tersedia", "Kritis", "Exp Segera", "Habis"
    pub status_type: i32,           // 0 = Normal/Green, 1 = Warning/Amber, 2 = Critical/Red
}

pub struct MasterObatView {
    pub win: Window,
    pub input_search: Input,
    pub btn_tambah: Button,
    pub btn_edit: Button,
    pub btn_hapus: Button,
    pub btn_refresh: Button,
    pub btn_import: Button,
    pub btn_export: Button,
    pub table: Table,
    pub products: Rc<RefCell<Vec<Product>>>,
}

impl MasterObatView {
    pub fn new(pool: Option<Pool>) -> Self {
        let mut win = Window::new(0, 108, 1200, 612, "");
        win.set_color(AppTheme::BG_APP);

        // ==========================================
        // KARTU UTAMA ELEGAN (White Card Background)
        // ==========================================
        let mut main_card = Frame::new(16, 12, 1168, 588, "");
        main_card.set_frame(FrameType::RFlatBox);
        main_card.set_color(Color::White);

        // ==========================================
        // 1. FILTER & HEADER SECTION
        // ==========================================
        let mut title_obat = Frame::new(36, 24, 220, 26, "Katalog Obat & Inventori");
        title_obat.set_label_size(18);
        title_obat.set_label_font(Font::HelveticaBold);
        title_obat.set_label_color(AppTheme::TEXT_TITLE);
        title_obat.set_align(Align::Left | Align::Inside);

        let mut badge_count = Frame::new(265, 26, 110, 22, "0 Item");
        badge_count.set_frame(FrameType::RFlatBox);
        badge_count.set_color(Color::from_hex(0xF1F5F9));
        badge_count.set_label_color(AppTheme::TEXT_MUTED);
        badge_count.set_label_size(11);
        badge_count.set_label_font(Font::HelveticaBold);

        // Filter Dropdowns
        let create_filter = |x: i32, label: &'static str, options: &'static str| -> Choice {
            let mut lbl = Frame::new(x, 16, 120, 16, label);
            lbl.set_label_size(11);
            lbl.set_label_color(AppTheme::TEXT_MUTED);
            lbl.set_align(Align::Left | Align::Inside);

            let mut c = Choice::new(x, 34, 120, 28, "");
            c.add_choice(options);
            c.set_value(0);
            c.set_frame(FrameType::RFlatBox);
            c.set_color(Color::from_hex(0xF8FAFC));
            c
        };

        let _choice_kategori = create_filter(670, "Kategori", "Semua Kategori|Alat Kesehatan|Minuman|Obat Bebas|Obat Keras");
        let _choice_suplier = create_filter(800, "Suplier", "Semua Suplier|SP Zorela|SP GEHAEL|SP KFTD|SP lhs");
        let _choice_stok = create_filter(930, "Status Stok", "Semua Status|Stok Tersedia|Stok Kritis|Stok Habis");
        let _choice_expired = create_filter(1060, "Kadaluarsa", "Semua|Aman|Exp Segera (< 30 Hari)|Expired");

        // ==========================================
        // 2. TABEL MODERN (Clean Headers & Pastel Badges)
        // ==========================================
        let mut table = Table::new(36, 72, 1128, 455, "");
        table.set_color(Color::White);
        table.set_cols(11);
        table.set_col_header(true);
        table.set_col_width(0, 45);   // No
        table.set_col_width(1, 100);  // Kode
        table.set_col_width(2, 280);  // Nama Obat
        table.set_col_width(3, 115);  // Kategori
        table.set_col_width(4, 55);   // Stok
        table.set_col_width(5, 75);   // Satuan
        table.set_col_width(6, 95);   // Harga Beli
        table.set_col_width(7, 95);   // Harga Jual
        table.set_col_width(8, 120);  // Suplier
        table.set_col_width(9, 90);   // Expired
        table.set_col_width(10, 95);  // Status Badge

        let rows = Rc::new(RefCell::new(Vec::<MockObatRow>::new()));
        let r_ref = rows.clone();

        table.draw_cell(move |_t, ctx, row, col, x, y, w, h| match ctx {
            fltk::table::TableContext::StartPage => {
                fltk::app::set_font(Font::Helvetica);
            }
            fltk::table::TableContext::ColHeader => {
                draw::push_clip(x, y, w, h);
                draw::draw_box(FrameType::FlatBox, x, y, w, h, AppTheme::TABLE_HEADER);
                draw::set_draw_color(AppTheme::TEXT_MUTED);
                draw::set_font(Font::HelveticaBold, 11);

                let header_text = match col {
                    0 => "NO",
                    1 => "KODE",
                    2 => "NAMA OBAT & DOSIS",
                    3 => "KATEGORI",
                    4 => "STOK",
                    5 => "SATUAN",
                    6 => "H. BELI (RP)",
                    7 => "H. JUAL (RP)",
                    8 => "SUPLIER",
                    9 => "EXPIRED",
                    10 => "STATUS",
                    _ => "",
                };

                let align = if col == 4 || col == 6 || col == 7 { Align::Right } else { Align::Left };
                draw::draw_text2(header_text, x + 8, y, w - 16, h, align);

                draw::set_draw_color(AppTheme::BORDER);
                draw::draw_line(x, y + h - 1, x + w, y + h - 1);
                draw::pop_clip();
            }
            fltk::table::TableContext::Cell => {
                let data = r_ref.borrow();
                if let Some(r) = data.get(row as usize) {
                    draw::push_clip(x, y, w, h);

                    let bg_color = if row % 2 == 0 { Color::White } else { Color::from_hex(0xF8FAFC) };
                    draw::draw_box(FrameType::FlatBox, x, y, w, h, bg_color);

                    draw::set_draw_color(Color::from_hex(0xF1F5F9));
                    draw::draw_line(x, y + h - 1, x + w, y + h - 1);

                    if col == 10 {
                        let (badge_bg, badge_fg) = match r.status_type {
                            0 => (AppTheme::BADGE_GREEN_BG, AppTheme::BADGE_GREEN_TEXT),
                            1 => (AppTheme::BADGE_AMBER_BG, AppTheme::BADGE_AMBER_TEXT),
                            _ => (AppTheme::BADGE_RED_BG, AppTheme::BADGE_RED_TEXT),
                        };

                        let pill_w = w - 16;
                        let pill_h = 22;
                        let pill_x = x + 8;
                        let pill_y = y + (h - pill_h) / 2;

                        draw::draw_box(FrameType::RFlatBox, pill_x, pill_y, pill_w, pill_h, badge_bg);
                        draw::set_draw_color(badge_fg);
                        draw::set_font(Font::HelveticaBold, 11);
                        draw::draw_text2(r.status_badge, pill_x, pill_y, pill_w, pill_h, Align::Center);
                    } else {
                        draw::set_draw_color(AppTheme::TEXT_BODY);
                        draw::set_font(Font::Helvetica, 12);

                        let txt = match col {
                            0 => r.no.to_string(),
                            1 => r.kode.clone(),
                            2 => r.nama.clone(),
                            3 => r.kategori.clone(),
                            4 => r.stok.to_string(),
                            5 => r.satuan.clone(),
                            6 => format!("{:.0}", r.harga_beli),
                            7 => format!("{:.0}", r.harga_jual),
                            8 => r.suplier.clone(),
                            9 => r.expired.clone(),
                            _ => String::new(),
                        };

                        let align = if col == 4 || col == 6 || col == 7 { Align::Right } else { Align::Left };
                        draw::draw_text2(&txt, x + 8, y, w - 16, h, align);
                    }

                    draw::pop_clip();
                }
            }
            _ => {}
        });

        // ==========================================
        // 3. BOTTOM ACTION & PAGINATION BAR
        // ==========================================
        let btn_y = 544;

        let mut btn_tambah = Button::new(36, btn_y, 110, 36, "+ Tambah Obat");
        btn_tambah.set_color(AppTheme::PRIMARY);
        btn_tambah.set_label_color(Color::White);
        btn_tambah.set_label_font(Font::HelveticaBold);
        btn_tambah.set_frame(FrameType::RFlatBox);

        let mut btn_edit = Button::new(154, btn_y, 75, 36, "Edit");
        btn_edit.set_color(Color::from_hex(0xF1F5F9));
        btn_edit.set_label_color(AppTheme::TEXT_BODY);
        btn_edit.set_frame(FrameType::RFlatBox);

        let mut btn_hapus = Button::new(236, btn_y, 75, 36, "Hapus");
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

        let mut btn_refresh = Button::new(880, btn_y, 85, 36, "🔄 Refresh");
        btn_refresh.set_frame(FrameType::RFlatBox);
        btn_refresh.set_color(Color::from_hex(0xF1F5F9));

        let mut btn_import = Button::new(972, btn_y, 92, 36, "📥 Import");
        btn_import.set_color(Color::from_hex(0xF0FDFA));
        btn_import.set_label_color(AppTheme::PRIMARY);
        btn_import.set_frame(FrameType::RFlatBox);

        let mut btn_export = Button::new(1070, btn_y, 94, 36, "📤 Export");
        btn_export.set_color(Color::from_hex(0xF0FDF4));
        btn_export.set_label_color(Color::from_hex(0x15803D));
        btn_export.set_frame(FrameType::RFlatBox);

        // Logika Refresh & Load Data dari Pool MariaDB
        let pool_for_load = pool.clone();
        let r_load = rows.clone();
        let mut t_load = table.clone();
        let mut badge_load = badge_count.clone();

        let refresh_data = Rc::new(RefCell::new(move |search_filter: Option<String>| {
            if let Some(ref p) = pool_for_load {
                if let Ok(prods) = crate::db::get_products(p, search_filter.as_deref()) {
                    let mut new_rows = Vec::with_capacity(prods.len());
                    for (i, prod) in prods.iter().enumerate() {
                        let harga_jual = prod.units.first().map(|u| u.sell_price).unwrap_or(0.0);
                        let expired = prod
                            .batches
                            .first()
                            .and_then(|b| b.expired_date.clone())
                            .unwrap_or_else(|| "-".into());
                        let (badge, st_type) = if prod.total_stock <= 0 {
                            ("Habis", 2)
                        } else if prod.total_stock <= prod.min_stock {
                            ("Kritis", 1)
                        } else {
                            ("Tersedia", 0)
                        };

                        new_rows.push(MockObatRow {
                            id: prod.id,
                            no: (i + 1) as i32,
                            kode: prod.code.clone(),
                            nama: prod.name.clone(),
                            kategori: prod.category.clone(),
                            stok: prod.total_stock,
                            satuan: prod.base_unit.clone(),
                            harga_beli: prod.buy_price,
                            harga_jual,
                            suplier: prod
                                .supplier_id
                                .map(|id| format!("Suplier #{}", id))
                                .unwrap_or_else(|| "Umum".into()),
                            expired,
                            status_badge: badge,
                            status_type: st_type,
                        });
                    }
                    badge_load.set_label(&format!("{} Item", new_rows.len()));
                    *r_load.borrow_mut() = new_rows;
                    t_load.set_rows(r_load.borrow().len() as i32);
                    t_load.redraw();
                    return;
                }
            }

            // Fallback preview dummy
            let fallback = vec![
                MockObatRow {
                    id: 1,
                    no: 1,
                    kode: "AK0209".to_string(),
                    nama: "Minyak Telon My Baby Plus 90ml".to_string(),
                    kategori: "Alat Kesehatan".to_string(),
                    stok: 6,
                    satuan: "Botol".to_string(),
                    harga_beli: 23100.0,
                    harga_jual: 25400.0,
                    suplier: "SP Zorela".to_string(),
                    expired: "28-05-2026".to_string(),
                    status_badge: "Tersedia",
                    status_type: 0,
                },
                MockObatRow {
                    id: 2,
                    no: 2,
                    kode: "OB0101".to_string(),
                    nama: "Paracetamol 500mg Tablet".to_string(),
                    kategori: "Obat Bebas".to_string(),
                    stok: 120,
                    satuan: "Tablet".to_string(),
                    harga_beli: 450.0,
                    harga_jual: 1000.0,
                    suplier: "Kimia Farma".to_string(),
                    expired: "15-12-2026".to_string(),
                    status_badge: "Tersedia",
                    status_type: 0,
                },
            ];
            badge_load.set_label(&format!("{} Item", fallback.len()));
            *r_load.borrow_mut() = fallback;
            t_load.set_rows(r_load.borrow().len() as i32);
            t_load.redraw();
        }));

        // Load awal data
        (refresh_data.borrow_mut())(None);

        // Tombol Refresh
        let ref_btn = refresh_data.clone();
        btn_refresh.set_callback(move |_| {
            (ref_btn.borrow_mut())(None);
        });

        // Search Callback
        let ref_srch = refresh_data.clone();
        let mut inp_c = input_search.clone();
        input_search.set_callback(move |_| {
            let q = inp_c.value();
            let filter = if q.trim().is_empty() { None } else { Some(q.trim().to_string()) };
            (ref_srch.borrow_mut())(filter);
        });

        // Tombol Tambah Obat
        let pool_for_add = pool.clone();
        let ref_after_add = refresh_data.clone();
        btn_tambah.set_callback(move |_| {
            let mut form = crate::gui::views::FormObatModal::new(None);
            form.win.show();

            let pool_save = pool_for_add.clone();
            let mut win_close = form.win.clone();
            let ref_cb = ref_after_add.clone();

            let in_kode = form.input_kode.clone();
            let in_nama = form.input_nama.clone();
            let in_satuan = form.input_satuan_dasar.clone();
            let in_beli = form.input_harga_beli.clone();
            let in_min = form.input_min_stok.clone();
            let in_jual1 = form.input_harga_jual_1.clone();
            let in_sat2 = form.input_nama_satuan_2.clone();
            let in_conv2 = form.input_konversi_2.clone();
            let in_jual2 = form.input_harga_jual_2.clone();
            let in_sat3 = form.input_nama_satuan_3.clone();
            let in_conv3 = form.input_konversi_3.clone();
            let in_jual3 = form.input_harga_jual_3.clone();
            let in_batch = form.input_no_batch.clone();
            let in_exp = form.input_expired.clone();
            let in_stok = form.input_stok_awal.clone();

            form.btn_simpan.set_callback(move |_| {
                let kode = in_kode.value().trim().to_string();
                let nama = in_nama.value().trim().to_string();
                let satuan = in_satuan.value().trim().to_string();

                if kode.is_empty() || nama.is_empty() || satuan.is_empty() {
                    fltk::dialog::alert_default("Kode Obat, Nama Obat, dan Satuan Dasar wajib diisi!");
                    return;
                }

                let buy_price: f64 = in_beli.value().trim().parse().unwrap_or(0.0);
                let min_stock: i32 = in_min.value().trim().parse().unwrap_or(0);
                let jual1: f64 = in_jual1.value().trim().parse().unwrap_or(0.0);
                let stok_awal: i32 = in_stok.value().trim().parse().unwrap_or(0);

                let mut units = vec![(satuan.clone(), 1, jual1)];

                let sat2 = in_sat2.value().trim().to_string();
                if !sat2.is_empty() {
                    let conv2: i32 = in_conv2.value().trim().parse().unwrap_or(1);
                    let jual2: f64 = in_jual2.value().trim().parse().unwrap_or(0.0);
                    units.push((sat2, conv2, jual2));
                }

                let sat3 = in_sat3.value().trim().to_string();
                if !sat3.is_empty() {
                    let conv3: i32 = in_conv3.value().trim().parse().unwrap_or(1);
                    let jual3: f64 = in_jual3.value().trim().parse().unwrap_or(0.0);
                    units.push((sat3, conv3, jual3));
                }

                let expired = {
                    let exp_str = in_exp.value().trim().to_string();
                    if exp_str.is_empty() { None } else { Some(exp_str) }
                };

                let new_prod = crate::db::NewProductInput {
                    code: kode,
                    name: nama,
                    category: "Umum".to_string(),
                    supplier_id: None,
                    base_unit: satuan,
                    min_stock,
                    buy_price,
                    units,
                    batch_no: in_batch.value().trim().to_string(),
                    expired_date: expired,
                    initial_stock: stok_awal,
                };

                if let Some(ref p) = pool_save {
                    match crate::db::insert_product(p, &new_prod) {
                        Ok(_) => {
                            fltk::dialog::message_default("Obat baru berhasil disimpan ke Database!");
                            win_close.hide();
                            (ref_cb.borrow_mut())(None);
                        }
                        Err(e) => {
                            fltk::dialog::alert_default(&format!("Gagal menyimpan obat:\n{}", e));
                        }
                    }
                } else {
                    fltk::dialog::message_default("Data tersimpan (Mode Preview)");
                    win_close.hide();
                }
            });
        });

        // Tombol Hapus Obat
        let pool_for_del = pool.clone();
        let rows_del = rows.clone();
        let table_del = table.clone();
        let ref_after_del = refresh_data.clone();
        btn_hapus.set_callback(move |_| {
            let row_idx = table_del.callback_row();
            let data = rows_del.borrow();
            if let Some(target) = data.get(row_idx as usize) {
                let id = target.id;
                let nama = target.nama.clone();
                drop(data);

                let choice = fltk::dialog::choice2_default(
                    &format!("Apakah Anda yakin ingin menghapus obat '{}'?", nama),
                    "Batal",
                    "Hapus",
                    "",
                );

                if choice == Some(1) {
                    if let Some(ref p) = pool_for_del {
                        if let Err(e) = crate::db::delete_product(p, id) {
                            fltk::dialog::alert_default(&format!("Gagal menghapus obat: {}", e));
                            return;
                        }
                    }
                    (ref_after_del.borrow_mut())(None);
                    fltk::dialog::message_default("Obat berhasil dihapus.");
                }
            }
        });

        win.end();

        Self {
            win,
            input_search,
            btn_tambah,
            btn_edit,
            btn_hapus,
            btn_refresh,
            btn_import,
            btn_export,
            table,
            products: Rc::new(RefCell::new(Vec::new())),
        }
    }
}
