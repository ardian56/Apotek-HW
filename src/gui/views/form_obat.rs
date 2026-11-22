use crate::gui::theme::AppTheme;
use crate::models::{Product, ProductBatch, ProductUnit};
use fltk::button::Button;
use fltk::enums::{Align, Color, Font, FrameType};
use fltk::frame::Frame;
use fltk::input::Input;
use fltk::menu::Choice;
use fltk::prelude::*;
use fltk::window::Window;

#[derive(Clone)]
pub struct FormObatModal {
    pub win: Window,
    pub input_kode: Input,
    pub input_nama: Input,
    pub choice_kategori: Choice,
    pub choice_suplier: Choice,
    pub input_satuan_dasar: Input,
    pub input_harga_beli: Input,
    pub input_min_stok: Input,

    // Multi-Satuan 1 (Dasar)
    pub input_harga_jual_1: Input,

    // Multi-Satuan 2 (Strip / Blister)
    pub input_nama_satuan_2: Input,
    pub input_konversi_2: Input,
    pub input_harga_jual_2: Input,

    // Multi-Satuan 3 (Box / Dus)
    pub input_nama_satuan_3: Input,
    pub input_konversi_3: Input,
    pub input_harga_jual_3: Input,

    // Batch & Expired Awal
    pub input_no_batch: Input,
    pub input_expired: Input,
    pub input_stok_awal: Input,

    pub btn_simpan: Button,
    pub btn_batal: Button,
}

impl FormObatModal {
    pub fn new(existing: Option<&Product>) -> Self {
        let is_edit = existing.is_some();
        let title_text = if is_edit {
            "Edit Data Obat"
        } else {
            "Tambah Obat Baru"
        };

        let mut win = Window::default().with_size(680, 660).with_label(title_text);
        win.set_color(AppTheme::BG_APP);

        // Header Modal
        let mut top_card = Frame::new(16, 12, 648, 50, "");
        top_card.set_frame(FrameType::RFlatBox);
        top_card.set_color(Color::White);

        let mut lbl_title = Frame::new(32, 18, 300, 24, title_text);
        lbl_title.set_label_size(16);
        lbl_title.set_label_font(Font::HelveticaBold);
        lbl_title.set_label_color(AppTheme::PRIMARY);
        lbl_title.set_align(Align::Left | Align::Inside);

        let mut lbl_sub = Frame::new(
            32,
            38,
            500,
            18,
            "Lengkapi data obat, satuan konversi harga jual, dan batch awal",
        );
        lbl_sub.set_label_size(11);
        lbl_sub.set_label_color(AppTheme::TEXT_MUTED);
        lbl_sub.set_align(Align::Left | Align::Inside);

        // ========================================================
        // SECTION 1: INFORMASI UTAMA OBAT
        // ========================================================
        let mut card1 = Frame::new(16, 68, 648, 175, "");
        card1.set_frame(FrameType::RFlatBox);
        card1.set_color(Color::White);

        let mut sec1_title = Frame::new(32, 76, 200, 20, "1. Informasi Dasar");
        sec1_title.set_label_font(Font::HelveticaBold);
        sec1_title.set_label_size(13);
        sec1_title.set_label_color(AppTheme::TEXT_TITLE);
        sec1_title.set_align(Align::Left | Align::Inside);

        // Baris 1: Kode & Nama
        let mut _lbl_kd = Frame::new(32, 102, 120, 16, "Kode / Barcode *");
        _lbl_kd.set_label_size(11);
        _lbl_kd.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_kd.set_align(Align::Left | Align::Inside);

        let mut input_kode = Input::new(32, 120, 200, 30, "");
        input_kode.set_frame(FrameType::RFlatBox);
        input_kode.set_color(Color::from_hex(0xF8FAFC));

        let mut _lbl_nm = Frame::new(242, 102, 120, 16, "Nama Obat & Dosis *");
        _lbl_nm.set_label_size(11);
        _lbl_nm.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_nm.set_align(Align::Left | Align::Inside);

        let mut input_nama = Input::new(242, 120, 406, 30, "");
        input_nama.set_frame(FrameType::RFlatBox);
        input_nama.set_color(Color::from_hex(0xF8FAFC));

        // Baris 2: Kategori, Suplier, Satuan Dasar
        let mut _lbl_cat = Frame::new(32, 158, 100, 16, "Kategori");
        _lbl_cat.set_label_size(11);
        _lbl_cat.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_cat.set_align(Align::Left | Align::Inside);

        let mut choice_kategori = Choice::new(32, 176, 140, 30, "");
        choice_kategori.add_choice("Obat Bebas|Obat Keras|Alat Kesehatan|Minuman|Jasa");
        choice_kategori.set_value(0);

        let mut _lbl_sup = Frame::new(182, 158, 100, 16, "Suplier Utama");
        _lbl_sup.set_label_size(11);
        _lbl_sup.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_sup.set_align(Align::Left | Align::Inside);

        let mut choice_suplier = Choice::new(182, 176, 170, 30, "");
        choice_suplier.add_choice("PT. Kimia Farma|PT. APL|SP Zorela|Distributor Mandiri");
        choice_suplier.set_value(0);

        let mut _lbl_base = Frame::new(362, 158, 130, 16, "Satuan Terkecil *");
        _lbl_base.set_label_size(11);
        _lbl_base.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_base.set_align(Align::Left | Align::Inside);

        let mut input_satuan_dasar = Input::new(362, 176, 130, 30, "");
        input_satuan_dasar.set_value("Tablet");
        input_satuan_dasar.set_frame(FrameType::RFlatBox);
        input_satuan_dasar.set_color(Color::from_hex(0xF8FAFC));

        let mut _lbl_min = Frame::new(502, 158, 100, 16, "Min. Stok");
        _lbl_min.set_label_size(11);
        _lbl_min.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_min.set_align(Align::Left | Align::Inside);

        let mut input_min_stok = Input::new(502, 176, 146, 30, "");
        input_min_stok.set_value("10");
        input_min_stok.set_frame(FrameType::RFlatBox);
        input_min_stok.set_color(Color::from_hex(0xF8FAFC));

        // ========================================================
        // SECTION 2: MULTI-SATUAN & HARGA JUAL
        // ========================================================
        let mut card2 = Frame::new(16, 250, 648, 185, "");
        card2.set_frame(FrameType::RFlatBox);
        card2.set_color(Color::White);

        let mut sec2_title =
            Frame::new(32, 258, 300, 20, "2. Multi-Satuan & Harga Jual Bertingkat");
        sec2_title.set_label_font(Font::HelveticaBold);
        sec2_title.set_label_size(13);
        sec2_title.set_label_color(AppTheme::TEXT_TITLE);
        sec2_title.set_align(Align::Left | Align::Inside);

        let mut _lbl_modal = Frame::new(32, 284, 160, 16, "Harga Beli / Modal Dasar (Rp)");
        _lbl_modal.set_label_size(11);
        _lbl_modal.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_modal.set_align(Align::Left | Align::Inside);

        let mut input_harga_beli = Input::new(32, 302, 180, 30, "");
        input_harga_beli.set_value("450");
        input_harga_beli.set_frame(FrameType::RFlatBox);
        input_harga_beli.set_color(Color::from_hex(0xF8FAFC));

        // Tingkat 1: Satuan Dasar (Eceran)
        let mut _lbl_t1 = Frame::new(230, 284, 180, 16, "Tingkat 1 (Eceran / Dasar)");
        _lbl_t1.set_label_size(11);
        _lbl_t1.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_t1.set_align(Align::Left | Align::Inside);

        let mut input_harga_jual_1 = Input::new(230, 302, 190, 30, "");
        input_harga_jual_1.set_value("1000");
        input_harga_jual_1.set_frame(FrameType::RFlatBox);
        input_harga_jual_1.set_color(Color::from_hex(0xF0FDFA)); // Soft Teal

        // Tingkat 2: Strip / Blister
        let mut _lbl_t2 = Frame::new(32, 342, 120, 16, "Satuan 2 (Strip/Blister)");
        _lbl_t2.set_label_size(11);
        _lbl_t2.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_t2.set_align(Align::Left | Align::Inside);

        let mut input_nama_satuan_2 = Input::new(32, 360, 110, 30, "");
        input_nama_satuan_2.set_value("Strip");
        input_nama_satuan_2.set_frame(FrameType::RFlatBox);
        input_nama_satuan_2.set_color(Color::from_hex(0xF8FAFC));

        let mut _lbl_k2 = Frame::new(150, 342, 90, 16, "Isi (Konversi)");
        _lbl_k2.set_label_size(11);
        _lbl_k2.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_k2.set_align(Align::Left | Align::Inside);

        let mut input_konversi_2 = Input::new(150, 360, 80, 30, "");
        input_konversi_2.set_value("10");
        input_konversi_2.set_frame(FrameType::RFlatBox);
        input_konversi_2.set_color(Color::from_hex(0xF8FAFC));

        let mut _lbl_h2 = Frame::new(240, 342, 150, 16, "Harga Jual Satuan 2 (Rp)");
        _lbl_h2.set_label_size(11);
        _lbl_h2.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_h2.set_align(Align::Left | Align::Inside);

        let mut input_harga_jual_2 = Input::new(240, 360, 180, 30, "");
        input_harga_jual_2.set_value("9500");
        input_harga_jual_2.set_frame(FrameType::RFlatBox);
        input_harga_jual_2.set_color(Color::from_hex(0xF0FDFA));

        // Tingkat 3: Box / Dus
        let mut _lbl_t3 = Frame::new(430, 342, 80, 16, "Satuan 3 (Box)");
        _lbl_t3.set_label_size(11);
        _lbl_t3.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_t3.set_align(Align::Left | Align::Inside);

        let mut input_nama_satuan_3 = Input::new(430, 360, 80, 30, "");
        input_nama_satuan_3.set_value("Box");
        input_nama_satuan_3.set_frame(FrameType::RFlatBox);
        input_nama_satuan_3.set_color(Color::from_hex(0xF8FAFC));

        let mut _lbl_k3 = Frame::new(518, 342, 50, 16, "Isi");
        _lbl_k3.set_label_size(11);
        _lbl_k3.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_k3.set_align(Align::Left | Align::Inside);

        let mut input_konversi_3 = Input::new(518, 360, 48, 30, "");
        input_konversi_3.set_value("100");
        input_konversi_3.set_frame(FrameType::RFlatBox);
        input_konversi_3.set_color(Color::from_hex(0xF8FAFC));

        let mut _lbl_h3 = Frame::new(574, 342, 80, 16, "Harga Satuan 3");
        _lbl_h3.set_label_size(11);
        _lbl_h3.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_h3.set_align(Align::Left | Align::Inside);

        let mut input_harga_jual_3 = Input::new(574, 360, 74, 30, "");
        input_harga_jual_3.set_value("90000");
        input_harga_jual_3.set_frame(FrameType::RFlatBox);
        input_harga_jual_3.set_color(Color::from_hex(0xF0FDFA));

        // ========================================================
        // SECTION 3: BATCH & KADALUARSA AWAL (FEFO)
        // ========================================================
        let mut card3 = Frame::new(16, 442, 648, 120, "");
        card3.set_frame(FrameType::RFlatBox);
        card3.set_color(Color::White);

        let mut sec3_title = Frame::new(32, 450, 300, 20, "3. Batch & Stok Masuk Awal (FEFO)");
        sec3_title.set_label_font(Font::HelveticaBold);
        sec3_title.set_label_size(13);
        sec3_title.set_label_color(AppTheme::TEXT_TITLE);
        sec3_title.set_align(Align::Left | Align::Inside);

        let mut _lbl_bno = Frame::new(32, 478, 140, 16, "Nomor Batch *");
        _lbl_bno.set_label_size(11);
        _lbl_bno.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_bno.set_align(Align::Left | Align::Inside);

        let mut input_no_batch = Input::new(32, 496, 180, 30, "");
        input_no_batch.set_value("BATCH-01");
        input_no_batch.set_frame(FrameType::RFlatBox);
        input_no_batch.set_color(Color::from_hex(0xF8FAFC));

        let mut _lbl_exp = Frame::new(230, 478, 180, 16, "Tgl Expired (YYYY-MM-DD)");
        _lbl_exp.set_label_size(11);
        _lbl_exp.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_exp.set_align(Align::Left | Align::Inside);

        let mut input_expired = Input::new(230, 496, 180, 30, "");
        input_expired.set_value("2026-12-31");
        input_expired.set_frame(FrameType::RFlatBox);
        input_expired.set_color(Color::from_hex(0xF8FAFC));

        let mut _lbl_stk = Frame::new(430, 478, 180, 16, "Stok Fisik Awal (Satuan Dasar)");
        _lbl_stk.set_label_size(11);
        _lbl_stk.set_label_color(AppTheme::TEXT_MUTED);
        _lbl_stk.set_align(Align::Left | Align::Inside);

        let mut input_stok_awal = Input::new(430, 496, 218, 30, "");
        input_stok_awal.set_value("100");
        input_stok_awal.set_frame(FrameType::RFlatBox);
        input_stok_awal.set_color(Color::from_hex(0xF8FAFC));

        // ========================================================
        // TOMBOL AKSI MODAL (Batal & Simpan)
        // ========================================================
        let mut btn_batal = Button::new(400, 585, 110, 44, "Batal");
        btn_batal.set_frame(FrameType::RFlatBox);
        btn_batal.set_color(Color::from_hex(0xF1F5F9));
        btn_batal.set_label_color(AppTheme::TEXT_BODY);
        btn_batal.set_label_font(Font::HelveticaBold);

        let mut btn_simpan = Button::new(520, 585, 144, 44, "Simpan Obat");
        btn_simpan.set_frame(FrameType::RFlatBox);
        btn_simpan.set_color(AppTheme::PRIMARY);
        btn_simpan.set_label_color(Color::White);
        btn_simpan.set_label_font(Font::HelveticaBold);

        let mut win_close = win.clone();
        btn_batal.set_callback(move |_| {
            win_close.hide();
        });

        // Prepopulate jika sedang Edit
        if let Some(p) = existing {
            input_kode.set_value(&p.code);
            input_nama.set_value(&p.name);
            input_satuan_dasar.set_value(&p.base_unit);
            input_min_stok.set_value(&p.min_stock.to_string());
            input_harga_beli.set_value(&format!("{:.0}", p.buy_price));

            if let Some(u1) = p.units.first() {
                input_harga_jual_1.set_value(&format!("{:.0}", u1.sell_price));
            }
            if let Some(u2) = p.units.get(1) {
                input_nama_satuan_2.set_value(&u2.unit_name);
                input_konversi_2.set_value(&u2.conversion_qty.to_string());
                input_harga_jual_2.set_value(&format!("{:.0}", u2.sell_price));
            }
            if let Some(u3) = p.units.get(2) {
                input_nama_satuan_3.set_value(&u3.unit_name);
                input_konversi_3.set_value(&u3.conversion_qty.to_string());
                input_harga_jual_3.set_value(&format!("{:.0}", u3.sell_price));
            }
            if let Some(b) = p.batches.first() {
                input_no_batch.set_value(&b.batch_no);
                if let Some(exp) = &b.expired_date {
                    input_expired.set_value(exp);
                }
                input_stok_awal.set_value(&b.stock.to_string());
            }
        }

        win.end();
        win.make_modal(true);

        Self {
            win,
            input_kode,
            input_nama,
            choice_kategori,
            choice_suplier,
            input_satuan_dasar,
            input_harga_beli,
            input_min_stok,
            input_harga_jual_1,
            input_nama_satuan_2,
            input_konversi_2,
            input_harga_jual_2,
            input_nama_satuan_3,
            input_konversi_3,
            input_harga_jual_3,
            input_no_batch,
            input_expired,
            input_stok_awal,
            btn_simpan,
            btn_batal,
        }
    }
}
