use super::components::*;
use crate::{db, gui::theme::AppTheme as T};
use fltk::{prelude::*, window::Window};
use mysql::Pool;

pub fn adjust_stock(
    pool: &Pool,
    id: i32,
    name: &str,
    unit: &str,
    current: i32,
) -> Result<bool, String> {
    let Some(value) = fltk::dialog::input_default(
        &format!("Stok fisik {name} ({unit}):"),
        &current.to_string(),
    ) else {
        return Ok(false);
    };
    let quantity = value
        .trim()
        .parse::<i32>()
        .map_err(|_| "Stok fisik harus bilangan bulat")?;
    if quantity < 0 {
        return Err("Stok tidak boleh negatif".into());
    }
    let Some(reason) = fltk::dialog::input_default("Alasan penyesuaian stok", "") else {
        return Ok(false);
    };
    db::save_stock_opname_adjustment(pool, id, quantity, &reason).map_err(|e| e.to_string())?;
    Ok(true)
}

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
        let mut refresh = button(28, 112, 160, "Muat ulang", false);
        let status = text(28, 592, 1144, 18, "", 12, false);
        let mut grid = Grid::new(
            28,
            166,
            1140,
            358,
            &["ID", "Kode", "Obat", "Satuan", "Stok sistem"],
        );
        for (column, width) in [0, 160, 550, 134, 260].iter().enumerate() {
            grid.table.set_col_width(column as i32, *width);
        }
        let mut save = button(28, 552, 240, "Catat stok fisik terpilih", true);
        if pool.is_none() {
            disable(&mut save);
        }
        let update = {
            let pool = pool.clone();
            let mut g = grid.clone();
            let mut st = status.clone();
            move || {
                let result = if let Some(p) = &pool {
                    db::get_stock_opname_items(p).map_err(|e| e.to_string())
                } else {
                    Ok(db::demo::products()
                        .into_iter()
                        .map(|p| (p.id, p.code, p.name, p.base_unit, p.total_stock))
                        .collect())
                };
                match result {
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
            match adjust_stock(
                p,
                row[0].parse().unwrap_or(0),
                &row[2],
                &row[3],
                row[4].parse().unwrap_or(0),
            ) {
                Ok(true) => {
                    u();
                    st.set_label("Stok fisik dan kartu stok tersimpan");
                }
                Ok(false) => (),
                Err(e) => st.set_label(&e),
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
