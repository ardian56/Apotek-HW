mod db;
mod gui;
mod hardware;
mod models;
mod network;
mod services;

use db::{create_pool, DbConfig};
use gui::theme::AppTheme;
use gui::views::{KasirView, LoginView, MasterObatView};
use fltk::dialog;
use fltk::enums::{Event, Key};
use fltk::prelude::*;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

fn main() {
    let app = fltk::app::App::default();
    AppTheme::apply();

    let args: Vec<String> = env::args().collect();
    let is_server_mode = args.iter().any(|arg| arg == "--server" || arg == "-s");

    let is_running = Arc::new(AtomicBool::new(true));

    let mut db_config = DbConfig::default();

    if is_server_mode {
        println!("[MODE SERVER / PC GUDANG AKTIF]");
        // Jalankan UDP Responder untuk melayani PC Kasir
        network::start_server_discovery_responder(is_running.clone());
        db_config.host = "127.0.0.1".to_string();
    } else {
        println!("[MODE CLIENT / PC KASIR AKTIF]");
        print!("Mencari PC Gudang di LAN (UDP Broadcast port {})... ", network::DISCOVERY_PORT);
        if let Some(server_ip) = network::scan_server_ip(2) {
            println!("DITEMUKAN di {}", server_ip);
            db_config.host = server_ip;
        } else {
            println!("Tidak terdeteksi via broadcast, menggunakan fallback host: {}", db_config.host);
        }
    }

    // Buat Pool Database
    let pool = match create_pool(&db_config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Gagal inisialisasi pool database: {}", e);
            dialog::alert_default(&format!(
                "Gagal terhubung ke Database MariaDB ({}:{}).\nPastikan MariaDB berjalan di PC Gudang!\n\nError: {}",
                db_config.host, db_config.port, e
            ));
            return;
        }
    };

    // Tampilkan Layar Login
    let mut login_view = LoginView::new();
    login_view.win.show();

    let pool_clone = pool.clone();
    let mut login_win_clone = login_view.win.clone();
    let mut input_user_clone = login_view.input_user.clone();
    let mut input_pass_clone = login_view.input_pass.clone();
    let mut lbl_status_clone = login_view.lbl_status.clone();

    login_view.btn_login.set_callback(move |_| {
        let username = input_user_clone.value();
        let password = input_pass_clone.value();

        if username.trim().is_empty() || password.trim().is_empty() {
            lbl_status_clone.set_label("Username & Password wajib diisi!");
            return;
        }

        match db::authenticate_user(&pool_clone, &username, &password) {
            Ok(Some(user)) => {
                println!("Login Berhasil! User: {} ({})", user.full_name, user.role);
                login_win_clone.hide();

                // Buka Layar Kasir Utama
                let mut kasir_view = KasirView::new();
                kasir_view.win.show();

                // Shortcut Global Keyboard Handler (F1 - F7)
                let mut inp_bc = kasir_view.input_barcode.clone();
                let mut inp_byr = kasir_view.input_bayar.clone();
                let btn_save = kasir_view.btn_simpan.clone();
                let btn_print = kasir_view.btn_cetak.clone();
                let btn_new = kasir_view.btn_baru.clone();

                kasir_view.win.handle(move |_w, ev| match ev {
                    Event::KeyDown => {
                        let key = fltk::app::event_key();
                        if key == Key::from_char('1') && fltk::app::event_key_down(Key::F1) || key == Key::F1 {
                            inp_bc.take_focus().unwrap_or(());
                            return true;
                        } else if key == Key::F2 {
                            inp_byr.take_focus().unwrap_or(());
                            return true;
                        } else if key == Key::F3 {
                            btn_save.do_callback();
                            return true;
                        } else if key == Key::F4 {
                            btn_print.do_callback();
                            return true;
                        } else if key == Key::F5 {
                            btn_new.do_callback();
                            return true;
                        }
                        false
                    }
                    _ => false,
                });
            }
            Ok(None) => {
                lbl_status_clone.set_label("Username atau Password salah!");
            }
            Err(e) => {
                lbl_status_clone.set_label("Gagal koneksi ke database!");
                eprintln!("Auth error: {}", e);
            }
        }
    });

    // Jalankan FLTK Event Loop
    app.run().unwrap();

    // Cleanup saat aplikasi ditutup
    is_running.store(false, Ordering::Relaxed);
}
