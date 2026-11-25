use super::components::*;
use crate::{db::operations, gui::theme::AppTheme as T, models::User};
use fltk::{input::InputType, menu::Choice, prelude::*, window::Window};
use mysql::Pool;

pub struct SettingsView {
    pub win: Window,
}

impl SettingsView {
    pub fn new(
        pool: Option<Pool>,
        user: User,
        mut renamed: impl FnMut(String) + 'static,
        mut accounts: impl FnMut() + 'static,
    ) -> Self {
        let mut win = Window::new(0, 108, 1200, 612, None);
        win.set_color(T::BG_APP);
        text(28, 20, 700, 32, "Pengaturan apotek", 22, true);
        text(
            28,
            60,
            900,
            24,
            "Identitas apotek, akun karyawan, struk, printer, dan cadangan database.",
            13,
            false,
        );
        let loaded = pool.as_ref().map(operations::settings).transpose();
        let mut account = button(948, 24, 224, "Akun karyawan", false);
        let admin = user.is_admin();
        if !admin {
            disable(&mut account);
        }
        account.set_callback(move |_| {
            if admin {
                accounts();
            }
        });
        let mut status = text(28, 576, 1136, 24, "", 12, false);
        if let Err(e) = &loaded {
            status.set_label(&format!("Pengaturan gagal dimuat: {e}"));
        }
        let values = if pool.is_none() {
            crate::db::demo::settings()
        } else {
            loaded.ok().flatten().unwrap_or_default()
        };
        let fields = [
            ("store_name", "Nama apotek"),
            ("store_address", "Alamat"),
            ("store_phone", "Telepon"),
            ("printer_name", "Nama printer"),
            ("receipt_header", "Header struk"),
            ("receipt_footer", "Footer struk"),
        ];
        let mut inputs = Vec::new();
        for (i, (key, label)) in fields.iter().enumerate() {
            let mut field = input(
                28 + (i as i32 % 2) * 556,
                116 + (i as i32 / 2) * 96,
                520,
                label,
                values.get(*key).map(String::as_str).unwrap_or(""),
            );
            if matches!(*key, "receipt_header" | "receipt_footer") {
                field.set_type(InputType::Multiline);
                field.resize(field.x(), field.y(), field.w(), 72);
            }
            inputs.push(field);
        }
        text(
            28,
            412,
            1136,
            18,
            "Tekan Enter untuk baris baru pada header dan footer struk.",
            12,
            false,
        );
        text(28, 442, 180, 20, "Lebar kertas", 12, false);
        let mut paper = Choice::new(28, 468, 200, 34, None);
        style_choice(&mut paper);
        paper.add_choice("58 mm|80 mm");
        paper.set_value(
            if values.get("receipt_paper_width").map(String::as_str) == Some("80") {
                1
            } else {
                0
            },
        );
        let mut save = button(28, 526, 200, "Simpan pengaturan", true);
        let mut backup = button(244, 526, 216, "Cadangkan database", false);
        if pool.is_none() || !user.is_admin() {
            disable(&mut save);
            disable(&mut backup);
            status
                .set_label("Pratinjau • Pengaturan hanya dapat disimpan oleh admin yang terhubung");
        }
        let p = pool.clone();
        let mut st = status.clone();
        save.set_callback(move |_| {
            let Some(p) = &p else {
                return;
            };
            if inputs[0].value().trim().is_empty() {
                st.set_label("Nama apotek wajib diisi");
                return;
            }
            let mut data = fields
                .iter()
                .zip(&inputs)
                .map(|((key, _), i)| (key.to_string(), i.value()))
                .collect::<Vec<_>>();
            data.push((
                "receipt_paper_width".into(),
                if paper.value() == 1 { "80" } else { "58" }.into(),
            ));
            data[0].1 = data[0].1.trim().to_string();
            match operations::save_settings(p, &data) {
                Ok(()) => {
                    renamed(data[0].1.clone());
                    st.set_label("Pengaturan tersimpan");
                }
                Err(e) => st.set_label(&format!("Simpan gagal: {e}")),
            }
        });
        let mut st = status.clone();
        backup.set_callback(move |_| {
            let Some(p) = &pool else {
                return;
            };
            let Some(path) = save_path("Cadangan database (file baru)", "SQL\t*.sql") else {
                return;
            };
            let result = operations::backup_database(p, &path);
            st.set_label(&match result {
                Ok(()) => "Cadangan database selesai. Simpan file SQL di tempat aman.".into(),
                Err(e) => format!("Cadangan gagal: {e}. File tidak lengkap jangan digunakan."),
            });
        });
        win.end();
        Self { win }
    }
}
