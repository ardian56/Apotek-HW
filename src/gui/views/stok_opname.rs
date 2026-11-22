use super::components::*;
use crate::{db, gui::theme::AppTheme as T};
use fltk::{prelude::*, window::Window};
use mysql::Pool;

pub struct StokOpnameView {
    pub win: Window,
}
impl StokOpnameView {
    pub fn new(pool: Option<Pool>) -> Self {
        let mut win = Window::new(0, 108, 1200, 612, None);
        win.set_color(T::BG_APP);
        text(28, 20, 900, 32, "Stok opname", 22, true);
        text(
            28,
            62,
            1140,
            24,
            "Pilih obat, masukkan hasil hitung fisik, lalu catat alasan penyesuaian.",
            13,
            false,
        );
        let mut refresh = button(28, 100, 160, "Muat ulang", false);
        let mut status = text(212, 100, 950, 30, "", 12, false);
        let mut grid = Grid::new(
            28,
            154,
            1140,
            370,
            &["ID", "Kode", "Obat", "Satuan", "Stok sistem"],
        );
        grid.table.set_col_width(2, 440);
        let mut save = button(28, 552, 240, "Catat stok fisik terpilih", true);
        if pool.is_none() {
            save.deactivate();
        }
        let update = {
            let pool = pool.clone();
            let mut g = grid.clone();
            let mut st = status.clone();
            move || {
                let Some(p) = &pool else {
                    st.set_label("Pratinjau • Hubungkan database untuk menyesuaikan stok");
                    return;
                };
                match db::get_stock_opname_items(p) {
                    Ok(rows) => {
                        st.set_label(&format!(
                            "{} obat. Penyesuaian dicatat pada kartu stok.",
                            rows.len()
                        ));
                        g.set(
                            rows.into_iter()
                                .map(|(id, code, name, unit, stock)| {
                                    vec![id.to_string(), code, name, unit, stock.to_string()]
                                })
                                .collect(),
                        );
                    }
                    Err(e) => {
                        g.set(vec![]);
                        st.set_label(&format!("Stok gagal dimuat: {e}"));
                    }
                }
            }
        };
        let mut u = update.clone();
        refresh.set_callback(move |_| u());
        let g = grid.clone();
        let p = pool.clone();
        let mut st = status.clone();
        let mut u = update.clone();
        save.set_callback(move |_| {
            let Some(p) = &p else {
                return;
            };
            let Some(row) = g.selected() else {
                st.set_label("Pilih satu obat pada tabel");
                return;
            };
            let Some(quantity) = fltk::dialog::input_default(
                &format!("Stok fisik {} ({}):", row[2], row[3]),
                &row[4],
            ) else {
                return;
            };
            let real = match quantity.trim().parse::<i32>() {
                Ok(n) if n >= 0 => n,
                _ => {
                    st.set_label("Stok fisik harus bilangan bulat, minimal nol");
                    return;
                }
            };
            let Some(reason) = fltk::dialog::input_default("Alasan penyesuaian stok", "") else {
                return;
            };
            match db::save_stock_opname_adjustment(p, row[0].parse().unwrap_or(0), real, &reason) {
                Ok(()) => {
                    u();
                    st.set_label("Stok fisik dan kartu stok tersimpan");
                }
                Err(e) => st.set_label(&format!("Penyesuaian gagal: {e}")),
            }
        });
        let mut u = update.clone();
        win.handle(move |_, event| {
            if event == fltk::enums::Event::Show {
                u();
            }
            false
        });
        let mut u = update;
        u();
        win.end();
        Self { win }
    }
}
