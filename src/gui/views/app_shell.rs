use crate::gui::theme::AppTheme;
use crate::gui::views::kasir::KasirView;
use crate::gui::views::master_crud::MasterSimpleView;
use crate::gui::views::master_obat::MasterObatView;
use crate::gui::views::stok_opname::StokOpnameView;
use fltk::button::Button;
use fltk::enums::{Align, Color, Font, FrameType};
use fltk::frame::Frame;
use fltk::group::Group;
use fltk::prelude::*;
use fltk::window::Window;

pub struct AppShell {
    pub win: Window,
    pub btn_nav_transaksi: Button,
    pub btn_nav_master: Button,
    pub btn_nav_laporan: Button,
    pub btn_nav_pengaturan: Button,
}

impl AppShell {
    pub fn new(pool: Option<mysql::Pool>) -> Self {
        let mut win = Window::default()
            .with_size(1200, 720)
            .with_label("Apotek Solution - Sistem POS & Inventori Modern");
        win.set_color(AppTheme::BG_APP);

        // ========================================================
        // 1. TOP GLOBAL NAVIGATION BAR (Dark Slate 900, Height 50)
        // ========================================================
        let mut top_nav = Group::new(0, 0, 1200, 50, "");
        top_nav.set_color(AppTheme::HEADER_BG);
        top_nav.set_frame(FrameType::FlatBox);

        // Logo Apotek Modern
        let mut logo_icon = Frame::new(16, 10, 30, 30, "✚");
        logo_icon.set_label_size(20);
        logo_icon.set_label_color(AppTheme::SUCCESS);
        logo_icon.set_label_font(Font::HelveticaBold);

        let mut logo_text = Frame::new(48, 10, 140, 30, "APOTEK POS");
        logo_text.set_label_size(16);
        logo_text.set_label_font(Font::HelveticaBold);
        logo_text.set_label_color(Color::White);
        logo_text.set_align(Align::Left | Align::Inside);

        let create_nav_btn = |x: i32, label: &'static str, is_active: bool| -> Button {
            let mut btn = Button::new(x, 9, 130, 32, label);
            btn.set_label_font(Font::HelveticaBold);
            btn.set_label_size(12);
            btn.set_frame(FrameType::RFlatBox);
            if is_active {
                btn.set_color(AppTheme::HEADER_ACTIVE);
                btn.set_label_color(Color::White);
            } else {
                btn.set_color(AppTheme::HEADER_BG);
                btn.set_label_color(Color::from_hex(0x94A3B8));
            }
            btn
        };

        let btn_transaksi = create_nav_btn(200, "TRANSAKSI", false);
        let btn_master = create_nav_btn(335, "MASTER DATA", true);
        let btn_laporan = create_nav_btn(470, "LAPORAN", false);
        let btn_pengaturan = create_nav_btn(605, "PENGATURAN", false);

        // Indikator Status & Operator di kanan
        let status_text = if pool.is_some() { "● Terhubung DB (Server)" } else { "● Mode Offline (Preview)" };
        let status_color = if pool.is_some() { AppTheme::SUCCESS } else { AppTheme::WARNING };
        let mut lbl_status = Frame::new(820, 10, 180, 30, "");
        lbl_status.set_label(status_text);
        lbl_status.set_label_color(status_color);
        lbl_status.set_label_font(Font::HelveticaBold);
        lbl_status.set_label_size(12);
        lbl_status.set_align(Align::Right | Align::Inside);

        let mut lbl_user = Frame::new(1010, 10, 170, 30, "Admin (Gudang)");
        lbl_user.set_label_color(Color::from_hex(0xCBD5E1));
        lbl_user.set_label_size(12);
        lbl_user.set_align(Align::Right | Align::Inside);

        top_nav.end();

        // ========================================================
        // 2. SUB-RIBBON ACTION BAR (White Card, Height 58)
        // ========================================================
        let mut ribbon = Group::new(0, 50, 1200, 58, "");
        ribbon.set_color(AppTheme::CARD_BG);
        ribbon.set_frame(FrameType::FlatBox);

        let mut div_line = Frame::new(0, 107, 1200, 1, "");
        div_line.set_color(AppTheme::BORDER);
        div_line.set_frame(FrameType::FlatBox);

        let create_pill = |x: i32, w: i32, label: &'static str, is_active: bool| -> Button {
            let mut b = Button::new(x, 62, w, 34, label);
            b.set_label_font(Font::HelveticaBold);
            b.set_label_size(12);
            b.set_frame(FrameType::RFlatBox);
            if is_active {
                b.set_color(AppTheme::PRIMARY);
                b.set_label_color(Color::White);
            } else {
                b.set_color(Color::from_hex(0xF1F5F9));
                b.set_label_color(AppTheme::TEXT_BODY);
            }
            b
        };

        let mut pill_obat = create_pill(20, 95, "💊 Obat", true);
        let mut pill_suplier = create_pill(122, 100, "🏢 Suplier", false);
        let mut pill_pelanggan = create_pill(228, 95, "👥 Pasien", false);
        let mut pill_dokter = create_pill(329, 95, "🩺 Dokter", false);
        let mut pill_karyawan = create_pill(430, 110, "👤 Karyawan", false);
        let mut pill_opname = create_pill(546, 130, "⚖ Stok Opname", false);

        // Tombol Quick Action Kasir POS di Kanan
        let mut btn_buka_kasir = Button::new(980, 60, 200, 38, "+ Buka Kasir POS (F1)");
        btn_buka_kasir.set_color(AppTheme::PRIMARY);
        btn_buka_kasir.set_label_color(Color::White);
        btn_buka_kasir.set_label_font(Font::HelveticaBold);
        btn_buka_kasir.set_label_size(13);
        btn_buka_kasir.set_frame(FrameType::RFlatBox);

        btn_buka_kasir.set_callback(|_| {
            let mut kasir = KasirView::new();
            kasir.win.show();
        });

        ribbon.end();

        // ========================================================
        // 3. MAIN CONTENT: SUITE VIEW MASTER DATA
        // ========================================================
        let view_obat = MasterObatView::new(pool.clone());
        let view_suplier = MasterSimpleView::suplier(pool.clone());
        let view_pelanggan = MasterSimpleView::pelanggan(pool.clone());
        let view_dokter = MasterSimpleView::dokter(pool.clone());
        let view_karyawan = MasterSimpleView::karyawan(pool.clone());
        let view_opname = StokOpnameView::new(pool.clone());

        let mut w_obat = view_obat.win.clone();
        let mut w_suplier = view_suplier.win.clone();
        let mut w_pelanggan = view_pelanggan.win.clone();
        let mut w_dokter = view_dokter.win.clone();
        let mut w_karyawan = view_karyawan.win.clone();
        let mut w_opname = view_opname.win.clone();

        // Default tampilkan Master Obat
        w_obat.show();

        win.end();

        // Handler Pergantian Tab Ribbon (DRY Switching)
        let reset_pills = {
            let mut p1 = pill_obat.clone();
            let mut p2 = pill_suplier.clone();
            let mut p3 = pill_pelanggan.clone();
            let mut p4 = pill_dokter.clone();
            let mut p5 = pill_karyawan.clone();
            let mut p6 = pill_opname.clone();
            move |active_idx: usize| {
                let pills = [&mut p1, &mut p2, &mut p3, &mut p4, &mut p5, &mut p6];
                for (i, p) in pills.into_iter().enumerate() {
                    if i == active_idx {
                        p.set_color(AppTheme::PRIMARY);
                        p.set_label_color(Color::White);
                    } else {
                        p.set_color(Color::from_hex(0xF1F5F9));
                        p.set_label_color(AppTheme::TEXT_BODY);
                    }
                    p.redraw();
                }
            }
        };

        // Helper untuk hide all windows
        let hide_all = {
            let mut wo = w_obat.clone();
            let mut ws = w_suplier.clone();
            let mut wp = w_pelanggan.clone();
            let mut wd = w_dokter.clone();
            let mut wk = w_karyawan.clone();
            let mut wn = w_opname.clone();
            move || {
                wo.hide();
                ws.hide();
                wp.hide();
                wd.hide();
                wk.hide();
                wn.hide();
            }
        };

        let mut r_pills_1 = reset_pills.clone();
        let mut h1 = hide_all.clone();
        let mut show_obat = w_obat.clone();
        pill_obat.set_callback(move |_| {
            h1();
            show_obat.show();
            r_pills_1(0);
        });

        let mut r_pills_2 = reset_pills.clone();
        let mut h2 = hide_all.clone();
        let mut show_suplier = w_suplier.clone();
        pill_suplier.set_callback(move |_| {
            h2();
            show_suplier.show();
            r_pills_2(1);
        });

        let mut r_pills_3 = reset_pills.clone();
        let mut h3 = hide_all.clone();
        let mut show_pelanggan = w_pelanggan.clone();
        pill_pelanggan.set_callback(move |_| {
            h3();
            show_pelanggan.show();
            r_pills_3(2);
        });

        let mut r_pills_4 = reset_pills.clone();
        let mut h4 = hide_all.clone();
        let mut show_dokter = w_dokter.clone();
        pill_dokter.set_callback(move |_| {
            h4();
            show_dokter.show();
            r_pills_4(3);
        });

        let mut r_pills_5 = reset_pills.clone();
        let mut h5 = hide_all.clone();
        let mut show_karyawan = w_karyawan.clone();
        pill_karyawan.set_callback(move |_| {
            h5();
            show_karyawan.show();
            r_pills_5(4);
        });

        let mut r_pills_6 = reset_pills;
        let mut h6 = hide_all;
        let mut show_opname = w_opname.clone();
        pill_opname.set_callback(move |_| {
            h6();
            show_opname.show();
            r_pills_6(5);
        });

        btn_buka_kasir.set_callback(|_| {
            let mut kasir = KasirView::new();
            kasir.win.make_modal(true);
            kasir.win.show();
        });

        Self {
            win,
            btn_nav_transaksi: btn_transaksi,
            btn_nav_master: btn_master,
            btn_nav_laporan: btn_laporan,
            btn_nav_pengaturan: btn_pengaturan,
        }
    }
}
