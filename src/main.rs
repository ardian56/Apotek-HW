mod db;
mod gui;
mod hardware;
mod models;
mod network;
mod services;

use db::{create_pool, DbConfig};
use fltk::button::{Button, RoundButton};
use fltk::dialog;
use fltk::enums::{Align, Color, Font, FrameType};
use fltk::frame::Frame;
use fltk::prelude::*;
use fltk::window::Window;
use gui::theme::AppTheme;
use gui::views::LoginView;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const CONFIG_FILE: &str = "app_config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub role: String, // "server" atau "client"
    pub db_host: String,
    pub db_port: u16,
    pub db_user: String,
    pub db_pass: String,
    pub db_name: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            role: "server".to_string(),
            db_host: "127.0.0.1".to_string(),
            db_port: 3306,
            db_user: "apotek".to_string(),
            db_pass: "apotek123".to_string(),
            db_name: "apotek_db".to_string(),
        }
    }
}

/// Menampilkan Dialog Pemilihan Mode Komputer saat aplikasi dibuka pertama kali
fn prompt_first_run_config() -> AppConfig {
    let mut win = Window::default()
        .with_size(460, 320)
        .with_label("Pengaturan Awal POS Apotek");
    win.set_color(AppTheme::BG_APP);

    let mut title = Frame::new(20, 20, 420, 30, "PENGATURAN AWAL SISTEM APOTEK");
    title.set_label_font(Font::HelveticaBold);
    title.set_label_size(16);
    title.set_label_color(AppTheme::PRIMARY);

    let mut desc = Frame::new(
        20,
        52,
        420,
        30,
        "Pilih peran komputer ini dalam jaringan apotek:",
    );
    desc.set_label_size(12);
    desc.set_label_color(AppTheme::TEXT_MUTED);

    let mut card = Frame::new(20, 90, 420, 140, "");
    card.set_frame(FrameType::RFlatBox);
    card.set_color(Color::White);

    let mut radio_server = RoundButton::new(35, 105, 390, 30, "PC GUDANG (Server Utama)");
    radio_server.set_label_font(Font::HelveticaBold);
    radio_server.set_label_size(13);

    let mut lbl_serv_desc = Frame::new(
        55,
        130,
        370,
        20,
        "- Menyimpan database & melayani PC Kasir via LAN",
    );
    lbl_serv_desc.set_label_size(11);
    lbl_serv_desc.set_label_color(AppTheme::TEXT_MUTED);
    lbl_serv_desc.set_align(Align::Left | Align::Inside);

    let mut radio_client = RoundButton::new(35, 160, 390, 30, "PC KASIR (Client Transaksi)");
    radio_client.set_label_font(Font::HelveticaBold);
    radio_client.set_label_size(13);

    let mut lbl_cli_desc = Frame::new(
        55,
        185,
        370,
        20,
        "- Hanyalah kasir (otomatis mencari PC Gudang di LAN)",
    );
    lbl_cli_desc.set_label_size(11);
    lbl_cli_desc.set_label_color(AppTheme::TEXT_MUTED);
    lbl_cli_desc.set_align(Align::Left | Align::Inside);

    radio_client.set_value(true); // Default client kasir

    let mut btn_simpan = Button::new(20, 250, 420, 45, "SIMPAN DAN MULAI APLIKASI");
    btn_simpan.set_color(AppTheme::PRIMARY);
    btn_simpan.set_label_color(Color::White);
    btn_simpan.set_label_font(Font::HelveticaBold);
    btn_simpan.set_frame(FrameType::FlatBox);

    let mut win_clone = win.clone();
    let r_serv_clone = radio_server.clone();

    btn_simpan.set_callback(move |_| {
        win_clone.hide();
    });

    win.end();
    win.make_modal(true);
    win.show();

    while win.shown() {
        fltk::app::wait();
    }

    let is_server = r_serv_clone.value();
    let config = if is_server {
        AppConfig {
            role: "server".to_string(),
            db_host: "127.0.0.1".to_string(),
            ..AppConfig::default()
        }
    } else {
        AppConfig {
            role: "client".to_string(),
            db_host: "auto".to_string(),
            ..AppConfig::default()
        }
    };

    // Simpan ke app_config.json
    if let Ok(json) = serde_json::to_string_pretty(&config) {
        let _ = fs::write(CONFIG_FILE, json);
    }

    config
}

fn load_or_init_config() -> AppConfig {
    if Path::new(CONFIG_FILE).exists() {
        if let Ok(content) = fs::read_to_string(CONFIG_FILE) {
            if let Ok(cfg) = serde_json::from_str::<AppConfig>(&content) {
                return cfg;
            }
        }
    }
    prompt_first_run_config()
}

fn main() {
    if std::env::args().any(|a| a == "--seed-demo") {
        let result = (|| -> Result<(), String> {
            let config: AppConfig =
                serde_json::from_str(&fs::read_to_string(CONFIG_FILE).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            if config.role.to_lowercase() != "server" {
                return Err("Jalankan pengisian data contoh pada komputer server".into());
            }
            let pool = create_pool(&DbConfig {
                host: config.db_host,
                port: config.db_port,
                user: config.db_user,
                pass: config.db_pass,
                db_name: config.db_name,
            })
            .map_err(|e| e.to_string())?;
            db::auto_migrate(&pool).map_err(|e| e.to_string())?;
            db::demo::seed(&pool)
        })();
        match result{Ok(())=>println!("Data contoh siap: 17 obat, 9 penjualan, 3 pembelian, pembayaran, retur, dan pending."),Err(e)=>{eprintln!("Data contoh gagal: {e}");std::process::exit(1);}}
        return;
    }

    let app = fltk::app::App::default();
    AppTheme::apply();

    // Mode Preview Cepat (Tanpa butuh database MariaDB jalan)
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--preview") {
        println!("[MODE CONTOH] Membuka pratinjau Apotek POS...");
        let mut shell = gui::views::AppShell::new(None);
        shell.win.show();
        app.run().unwrap();
        return;
    }

    let app_config = load_or_init_config();
    let is_server_mode = app_config.role.to_lowercase() == "server";

    let is_running = Arc::new(AtomicBool::new(true));
    let mut db_config = DbConfig {
        host: app_config.db_host.clone(),
        port: app_config.db_port,
        user: app_config.db_user.clone(),
        pass: app_config.db_pass.clone(),
        db_name: app_config.db_name.clone(),
    };

    if is_server_mode {
        println!("[MODE SERVER / PC GUDANG AKTIF]");
        // Jalankan background UDP discovery responder
        network::start_server_discovery_responder(is_running.clone());
        db_config.host = "127.0.0.1".to_string();
    } else {
        println!("[MODE CLIENT / PC KASIR AKTIF]");
        print!(
            "Mencari PC Gudang di LAN (UDP Broadcast port {})... ",
            network::DISCOVERY_PORT
        );
        if let Some(server_ip) = network::scan_server_ip(2) {
            println!("DITEMUKAN di {}", server_ip);
            db_config.host = server_ip;
        } else {
            println!(
                "Broadcast timeout, menggunakan fallback host: {}",
                db_config.host
            );
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

    // Auto-Migration Database (Otomatis buat tabel dan akun jika belum ada)
    if is_server_mode {
        if let Err(e) = db::auto_migrate(&pool) {
            eprintln!("[AUTO-MIGRATION] Peringatan: {}", e);
            dialog::alert_default(&format!("Skema database gagal disiapkan. Aplikasi dihentikan agar transaksi tidak memakai skema yang belum lengkap.\n{}",e));
            is_running.store(false, Ordering::Relaxed);
            return;
        } else {
            println!("[AUTO-MIGRATION] Skema tabel database siap & terverifikasi.");
        }
    }

    // Tampilkan Layar Login
    let mut login_view = LoginView::new(pool.clone());
    login_view.win.show();

    let pool_clone = pool.clone();
    let mut login_win_clone = login_view.win.clone();
    let input_user_clone = login_view.input_user.clone();
    let input_pass_clone = login_view.input_pass.clone();
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

                // Buka AppShell Utama dengan Database Pool aktif
                let mut logout_window = login_win_clone.clone();
                let mut logout_password = input_pass_clone.clone();
                let mut logout_status = lbl_status_clone.clone();
                let mut shell =
                    gui::views::AppShell::for_user(Some(pool_clone.clone()), user, move || {
                        logout_password.set_value("");
                        logout_status.set_label("");
                        logout_window.show();
                    });
                shell.win.show();
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
