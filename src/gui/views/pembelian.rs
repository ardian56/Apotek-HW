use crate::gui::theme::AppTheme;
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
pub struct PembelianItemRow {
    pub no: i32,
    pub barcode: String,
    pub nama: String,
    pub satuan: String,
    pub harga_beli: f64,
    pub qty: i32,
    pub diskon_persen: f64,
    pub total: f64,
}

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
    pub btn_cetak: Button,
    pub btn_baru: Button,
    pub table: Table,
    pub lbl_subtotal: Frame,
    pub lbl_grand_total: Frame,
    pub items: Rc<RefCell<Vec<PembelianItemRow>>>,
}

impl PembelianView {
    pub fn new(_pool: Option<Pool>) -> Self {
        let mut win = Window::new(0, 108, 1200, 612, "");
        win.set_color(AppTheme::BG_APP);

        // ==========================================
        // 1. TOP CARD: INFO FAKTUR & SUPLIER
        // ==========================================
        let mut top_card = Frame::new(20, 12, 1160, 100, "");
        top_card.set_frame(FrameType::RFlatBox);
        top_card.set_color(Color::White);

        // Baris 1: No Faktur, Tanggal, Pembayaran
        let mut _lbl_f = Frame::new(36, 20, 90, 16, "No. Faktur:");
        _lbl_f.set_label_size(11);
        _lbl_f.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_f.set_align(Align::Left | Align::Inside);

        let mut input_faktur = Input::new(36, 38, 180, 30, "");
        input_faktur.set_frame(FrameType::RFlatBox);
        input_faktur.set_color(Color::from_hex(0xF8FAFC));
        input_faktur.set_text_font(Font::HelveticaBold);
        input_faktur.set_value("PBL2611-001");

        let mut _lbl_t = Frame::new(230, 20, 90, 16, "Tgl Faktur:");
        _lbl_t.set_label_size(11);
        _lbl_t.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_t.set_align(Align::Left | Align::Inside);

        let mut input_tgl = Input::new(230, 38, 130, 30, "");
        input_tgl.set_frame(FrameType::RFlatBox);
        input_tgl.set_color(Color::from_hex(0xF8FAFC));
        input_tgl.set_value(&chrono::Local::now().date_naive().to_string());

        let mut _lbl_byr = Frame::new(375, 20, 90, 16, "Pembayaran:");
        _lbl_byr.set_label_size(11);
        _lbl_byr.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_byr.set_align(Align::Left | Align::Inside);

        let mut choice_bayar = Choice::new(375, 38, 120, 30, "");
        choice_bayar.add_choice("Tunai|Kredit");
        choice_bayar.set_value(0);
        choice_bayar.set_frame(FrameType::RFlatBox);
        choice_bayar.set_color(Color::from_hex(0xF8FAFC));

        let mut _lbl_tempo = Frame::new(510, 20, 90, 16, "Tempo:");
        _lbl_tempo.set_label_size(11);
        _lbl_tempo.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_tempo.set_align(Align::Left | Align::Inside);

        let mut input_tempo = Input::new(510, 38, 100, 30, "");
        input_tempo.set_frame(FrameType::RFlatBox);
        input_tempo.set_color(Color::from_hex(0xF8FAFC));
        input_tempo.set_value("30 Hari");

        let mut _lbl_sup = Frame::new(625, 20, 90, 16, "Nama Suplier (PBF):");
        _lbl_sup.set_label_size(11);
        _lbl_sup.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_sup.set_align(Align::Left | Align::Inside);

        let mut choice_suplier = Choice::new(625, 38, 240, 30, "");
        choice_suplier.add_choice(
            "PT Bina San Prima|PT Kediri Jaya Farma|PT Kimia Farma|Distributor Mandiri",
        );
        choice_suplier.set_value(0);
        choice_suplier.set_frame(FrameType::RFlatBox);
        choice_suplier.set_color(Color::from_hex(0xF8FAFC));

        // Action Buttons di Kanan Atas
        let mut btn_simpan = Button::new(880, 34, 130, 36, "Simpan [F3]");
        btn_simpan.set_color(AppTheme::PRIMARY);
        btn_simpan.set_label_color(Color::White);
        btn_simpan.set_label_font(Font::HelveticaBold);
        btn_simpan.set_frame(FrameType::RFlatBox);

        let mut btn_baru = Button::new(1020, 34, 85, 36, "Baru [F5]");
        btn_baru.set_color(Color::from_hex(0xF1F5F9));
        btn_baru.set_label_color(AppTheme::TEXT_BODY);
        btn_baru.set_frame(FrameType::RFlatBox);

        let mut btn_cetak = Button::new(1115, 34, 55, 36, "[F4]");
        btn_cetak.set_color(Color::from_hex(0xF1F5F9));
        btn_cetak.set_label_color(AppTheme::TEXT_BODY);
        btn_cetak.set_frame(FrameType::RFlatBox);

        // ==========================================
        // 2. TABEL DAFTAR BARANG PEMBELIAN
        // ==========================================
        let table_y = 124;
        let mut table = Table::new(20, table_y, 845, 470, "");
        table.end();
        table.set_row_height_all(34);
        table.set_col_header_height(32);
        table.set_color(Color::White);
        table.set_cols(8);
        table.set_col_header(true);
        table.set_col_width(0, 40); // No
        table.set_col_width(1, 110); // Barcode
        table.set_col_width(2, 285); // Nama Barang
        table.set_col_width(3, 80); // Satuan
        table.set_col_width(4, 90); // Harga Beli
        table.set_col_width(5, 60); // Qty
        table.set_col_width(6, 60); // Disc %
        table.set_col_width(7, 100); // Subtotal

        let items_data = Rc::new(RefCell::new(vec![
            PembelianItemRow {
                no: 1,
                barcode: "8993478112448".into(),
                nama: "ACTIFED COUGH MERAH 25 ML".into(),
                satuan: "Botol".into(),
                harga_beli: 25042.0,
                qty: 5,
                diskon_persen: 0.0,
                total: 125210.0,
            },
            PembelianItemRow {
                no: 2,
                barcode: "8999909001".into(),
                nama: "PARACETAMOL 500MG TABLET".into(),
                satuan: "Box".into(),
                harga_beli: 45000.0,
                qty: 10,
                diskon_persen: 5.0,
                total: 427500.0,
            },
        ]));

        table.set_rows(items_data.borrow().len() as i32);
        let items_ref = items_data.clone();

        table.draw_cell(move |_t, ctx, row, col, x, y, w, h| match ctx {
            fltk::table::TableContext::StartPage => {
                fltk::app::set_font(Font::Helvetica);
            }
            fltk::table::TableContext::ColHeader => {
                draw::push_clip(x, y, w, h);
                draw::draw_box(FrameType::FlatBox, x, y, w, h, AppTheme::TABLE_HEADER);
                draw::set_draw_color(AppTheme::TEXT_MUTED);
                draw::set_font(Font::HelveticaBold, 11);

                let header_title = match col {
                    0 => "NO",
                    1 => "KODE / BARCODE",
                    2 => "NAMA OBAT & DOSIS",
                    3 => "SATUAN",
                    4 => "HARGA BELI",
                    5 => "QTY",
                    6 => "DISC %",
                    7 => "TOTAL (RP)",
                    _ => "",
                };
                let align = if col == 4 || col == 5 || col == 6 || col == 7 {
                    Align::Right
                } else {
                    Align::Left
                };
                draw::draw_text2(header_title, x + 8, y, w - 16, h, align);
                draw::set_draw_color(AppTheme::BORDER);
                draw::draw_line(x, y + h - 1, x + w, y + h - 1);
                draw::pop_clip();
            }
            fltk::table::TableContext::Cell => {
                let data = items_ref.borrow();
                if let Some(r) = data.get(row as usize) {
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
                        0 => r.no.to_string(),
                        1 => r.barcode.clone(),
                        2 => r.nama.clone(),
                        3 => r.satuan.clone(),
                        4 => format!("{:.0}", r.harga_beli),
                        5 => r.qty.to_string(),
                        6 => format!("{:.0}%", r.diskon_persen),
                        7 => format!("{:.0}", r.total),
                        _ => String::new(),
                    };

                    let align = if col == 4 || col == 5 || col == 6 || col == 7 {
                        Align::Right
                    } else {
                        Align::Left
                    };
                    draw::draw_text2(&txt, x + 8, y, w - 16, h, align);
                    draw::pop_clip();
                }
            }
            _ => {}
        });

        // ==========================================
        // 3. CARD SUMMARY PEMBAYARAN (Di Sisi Kanan)
        // ==========================================
        let summary_x = 880;
        let mut sum_card = Frame::new(summary_x, table_y, 300, 470, "");
        sum_card.set_frame(FrameType::RFlatBox);
        sum_card.set_color(Color::White);

        let mut btn_tambah_item = Button::new(
            summary_x + 16,
            table_y + 16,
            268,
            40,
            "+ Tambah Barang (F1)",
        );
        btn_tambah_item.set_color(Color::from_hex(0xECFDF5));
        btn_tambah_item.set_label_color(Color::from_hex(0x065F46));
        btn_tambah_item.set_label_font(Font::HelveticaBold);
        btn_tambah_item.set_frame(FrameType::RFlatBox);

        let mut _lbl_sub_t = Frame::new(summary_x + 20, table_y + 80, 120, 20, "Subtotal Barang:");
        _lbl_sub_t.set_label_size(12);
        _lbl_sub_t.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_sub_t.set_align(Align::Left | Align::Inside);

        let mut lbl_subtotal = Frame::new(summary_x + 140, table_y + 80, 140, 20, "Rp 552.710");
        lbl_subtotal.set_label_size(13);
        lbl_subtotal.set_label_font(Font::HelveticaBold);
        lbl_subtotal.set_label_color(AppTheme::TEXT_TITLE);
        lbl_subtotal.set_align(Align::Right | Align::Inside);

        let mut _lbl_disc_t =
            Frame::new(summary_x + 20, table_y + 115, 120, 20, "Potongan Faktur:");
        _lbl_disc_t.set_label_size(12);
        _lbl_disc_t.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_disc_t.set_align(Align::Left | Align::Inside);

        let mut input_discount = Input::new(summary_x + 160, table_y + 112, 120, 26, "");
        input_discount.set_frame(FrameType::BorderBox);
        input_discount.set_value("0");

        let mut _lbl_ppn_t = Frame::new(summary_x + 20, table_y + 150, 120, 20, "Pajak (%):");
        _lbl_ppn_t.set_label_size(12);
        _lbl_ppn_t.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_ppn_t.set_align(Align::Left | Align::Inside);

        let mut _lbl_ppn_val = Frame::new(summary_x + 140, table_y + 150, 140, 20, "Rp 0");
        _lbl_ppn_val.set_label_size(13);
        _lbl_ppn_val.set_label_font(Font::HelveticaBold);
        _lbl_ppn_val.set_label_color(AppTheme::TEXT_TITLE);
        _lbl_ppn_val.set_align(Align::Right | Align::Inside);
        _lbl_ppn_val.hide();
        let mut input_tax = Input::new(summary_x + 160, table_y + 147, 120, 26, None);
        input_tax.set_frame(FrameType::BorderBox);
        input_tax.set_value("0");

        // Divider
        let mut _d = Frame::new(summary_x + 16, table_y + 185, 268, 1, "");
        _d.set_color(AppTheme::BORDER);
        _d.set_frame(FrameType::FlatBox);

        // Giant Total Card
        let mut total_box = Frame::new(summary_x + 16, table_y + 205, 268, 90, "");
        total_box.set_frame(FrameType::RFlatBox);
        total_box.set_color(Color::from_hex(0xF8FAFC));

        let mut _lbl_gt_title =
            Frame::new(summary_x + 24, table_y + 215, 200, 18, "TOTAL PEMBELIAN");
        _lbl_gt_title.set_label_size(11);
        _lbl_gt_title.set_label_font(Font::HelveticaBold);
        _lbl_gt_title.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_gt_title.set_align(Align::Left | Align::Inside);

        let mut lbl_grand_total = Frame::new(summary_x + 24, table_y + 240, 252, 36, "Rp 552.710");
        lbl_grand_total.set_label_size(24);
        lbl_grand_total.set_label_font(Font::HelveticaBold);
        lbl_grand_total.set_label_color(AppTheme::PRIMARY);
        lbl_grand_total.set_align(Align::Left | Align::Inside);

        let mut btn_pending = Button::new(summary_x + 16, table_y + 315, 268, 36, "Pending Faktur");
        btn_pending.set_color(Color::from_hex(0xFFFBEB));
        btn_pending.set_label_color(Color::from_hex(0xB45309));
        btn_pending.set_label_font(Font::HelveticaBold);
        btn_pending.set_frame(FrameType::RFlatBox);
        btn_pending.hide();

        // Tombol Tambah Barang Pembelian (Buka modal input obat seperti di video frame 008)
        let it_clone = items_data.clone();
        let mut tbl_clone = table.clone();
        btn_tambah_item.set_callback(move |_| {
            let mut form = crate::gui::views::FormObatModal::new(None);
            form.win.show();

            let it_save = it_clone.clone();
            let mut tbl_c = tbl_clone.clone();
            let mut win_c = form.win.clone();

            let in_k = form.input_kode.clone();
            let in_n = form.input_nama.clone();
            let in_s = form.input_satuan_dasar.clone();
            let in_b = form.input_harga_beli.clone();
            let in_stk = form.input_stok_awal.clone();

            form.btn_simpan.set_callback(move |_| {
                let kode = in_k.value().trim().to_string();
                let nama = in_n.value().trim().to_string();
                let sat = in_s.value().trim().to_string();
                let hrg: f64 = in_b.value().trim().parse().unwrap_or(0.0);
                let qty: i32 = in_stk.value().trim().parse().unwrap_or(1);

                if kode.is_empty() || nama.is_empty() {
                    fltk::dialog::alert_default("Kode dan Nama Obat wajib diisi!");
                    return;
                }

                let mut data = it_save.borrow_mut();
                let no = (data.len() + 1) as i32;
                let tot = hrg * qty as f64;
                data.push(PembelianItemRow {
                    no,
                    barcode: kode,
                    nama,
                    satuan: sat,
                    harga_beli: hrg,
                    qty,
                    diskon_persen: 0.0,
                    total: tot,
                });
                tbl_c.set_rows(data.len() as i32);
                tbl_c.redraw();
                win_c.hide();
                fltk::dialog::message_default("Item berhasil ditambahkan ke keranjang pembelian!");
            });
        });

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
            btn_cetak,
            btn_baru,
            table,
            lbl_subtotal,
            lbl_grand_total,
            items: items_data,
        }
    }
}
