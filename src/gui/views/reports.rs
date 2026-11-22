use super::components::*;
use crate::{
    db::operations::{self, ReportKind},
    gui::theme::AppTheme as T,
    models::User,
};
use fltk::{
    enums::{CallbackTrigger, FrameType},
    frame::Frame,
    menu::Choice,
    prelude::*,
    window::Window,
};
use mysql::Pool;

pub struct ReportsView {
    pub win: Window,
}

fn load(
    pool: &Option<Pool>,
    kind: ReportKind,
    from: &str,
    to: &str,
    search: &str,
    grid: &mut Grid,
    status: &mut Frame,
) {
    grid.columns(&kind.headers());
    let Some(pool) = pool else {
        grid.set(vec![]);
        status.set_label("Pratinjau • Hubungkan database untuk menampilkan transaksi");
        return;
    };
    match operations::report(pool, kind, from, to) {
        Ok(rows) => {
            let query = search.to_lowercase();
            let filtered = rows
                .into_iter()
                .filter(|r| r.iter().any(|s| s.to_lowercase().contains(&query)))
                .collect::<Vec<_>>();
            status.set_label(&format!(
                "{} baris • Maksimum 500 baris per filter",
                filtered.len()
            ));
            grid.set(filtered);
        }
        Err(e) => {
            grid.set(vec![]);
            status.set_label(&format!("Data gagal dimuat: {e}"));
        }
    }
}

impl ReportsView {
    pub fn new(pool: Option<Pool>, user: User, kinds: Vec<ReportKind>) -> Self {
        let mut win = Window::new(0, 108, 1200, 612, None);
        win.set_color(T::BG_APP);
        text(28, 16, 520, 30, "Transaksi dan laporan", 22, true);
        let mut kind = Choice::new(28, 64, 220, 34, None);
        kind.set_frame(FrameType::BorderBox);
        for k in &kinds {
            kind.add_choice(k.title());
        }
        kind.set_value(0);
        let from = input(268, 40, 140, "Dari tanggal", "2000-01-01");
        let to = input(
            424,
            40,
            140,
            "Sampai tanggal",
            &chrono::Local::now().date_naive().to_string(),
        );
        let mut search = input(584, 40, 240, "Cari faktur / nama", "");
        search.set_trigger(CallbackTrigger::EnterKey);
        let mut refresh = button(844, 64, 104, "Tampilkan", true);
        let mut export = button(964, 64, 204, "Ekspor Excel", false);
        let mut status = text(28, 110, 1140, 24, "", 12, false);
        let mut grid = Grid::new(28, 144, 1140, 388, &kinds[0].headers());
        let mut pay = button(28, 552, 184, "Catat pembayaran", true);
        let mut returns = button(228, 552, 184, "Retur faktur utuh", false);
        if pool.is_none() {
            pay.deactivate();
            returns.deactivate();
            export.deactivate();
        }
        let is_admin = user.is_admin();
        let update = {
            let pool = pool.clone();
            let kinds = kinds.clone();
            let k = kind.clone();
            let f = from.clone();
            let t = to.clone();
            let s = search.clone();
            let mut g = grid.clone();
            let mut st = status.clone();
            let mut p = pay.clone();
            let mut r = returns.clone();
            move || {
                let kind = kinds[k.value().max(0) as usize];
                load(
                    &pool,
                    kind,
                    &f.value(),
                    &t.value(),
                    &s.value(),
                    &mut g,
                    &mut st,
                );
                if pool.is_some() && matches!(kind, ReportKind::Receivables | ReportKind::Debts) {
                    p.show();
                } else {
                    p.hide();
                }
                if pool.is_some()
                    && is_admin
                    && matches!(kind, ReportKind::Sales | ReportKind::Purchases)
                {
                    r.show();
                } else {
                    r.hide();
                }
            }
        };
        for mut b in [refresh.clone()] {
            let mut update = update.clone();
            b.set_callback(move |_| update());
        }
        let mut u = update.clone();
        kind.set_callback(move |_| u());
        let mut u = update.clone();
        search.set_callback(move |_| u());
        let mut u = update.clone();
        win.handle(move |_, event| {
            if event == fltk::enums::Event::Show {
                u();
            }
            false
        });
        let g = grid.clone();
        let mut st = status.clone();
        export.set_callback(move |_| {
            if let Some(path) = save_path("Ekspor laporan", "Excel\t*.xlsx") {
                let mut workbook = rust_xlsxwriter::Workbook::new();
                let result = (|| -> Result<(), String> {
                    let sheet = workbook.add_worksheet();
                    for (c, h) in g.headers.borrow().iter().enumerate() {
                        sheet
                            .write_string(0, c as u16, h)
                            .map_err(|e| e.to_string())?;
                    }
                    for (r, row) in g.rows.borrow().iter().enumerate() {
                        for (c, value) in row.iter().enumerate() {
                            sheet
                                .write_string(r as u32 + 1, c as u16, value)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                    workbook.save(path).map_err(|e| e.to_string())
                })();
                st.set_label(&match result {
                    Ok(()) => "Laporan diekspor".into(),
                    Err(e) => format!("Ekspor gagal: {e}"),
                });
            }
        });
        let g = grid.clone();
        let p = pool.clone();
        let k = kind.clone();
        let ks = kinds.clone();
        let mut st = status.clone();
        let mut u = update.clone();
        let operator = user.id;
        pay.set_callback(move |_| {
            let Some(row) = g.selected() else {
                st.set_label("Pilih satu faktur pada tabel");
                return;
            };
            let Some(p) = &p else {
                return;
            };
            let Some(value) = fltk::dialog::input_default(
                &format!("Pembayaran faktur {}. Sisa Rp {}", row[1], row[6]),
                &row[6],
            ) else {
                return;
            };
            let result = operations::money(&value).and_then(|amount| {
                operations::pay(
                    p,
                    ks[k.value() as usize] == ReportKind::Receivables,
                    row[0].parse().unwrap_or(0),
                    amount,
                    operator,
                )
            });
            match result {
                Ok(()) => u(),
                Err(e) => st.set_label(&format!("Pembayaran gagal: {e}")),
            }
        });
        let g = grid.clone();
        let p = pool.clone();
        let k = kind.clone();
        let ks = kinds.clone();
        let mut st = status.clone();
        let mut u = update.clone();
        returns.set_callback(move |_| {
            let Some(row) = g.selected() else {
                st.set_label("Pilih satu faktur pada tabel");
                return;
            };
            let Some(p) = &p else {
                return;
            };
            let Some(reason) = fltk::dialog::input_default("Alasan retur seluruh faktur", "")
            else {
                return;
            };
            if fltk::dialog::choice2_default(
                &format!(
                    "Retur seluruh faktur {}? Stok dan catatan retur akan berubah.",
                    row[1]
                ),
                "Batal",
                "Retur",
                "",
            ) != Some(1)
            {
                return;
            }
            match operations::return_invoice(
                p,
                ks[k.value() as usize] == ReportKind::Sales,
                row[0].parse().unwrap_or(0),
                &reason,
                operator,
            ) {
                Ok(()) => u(),
                Err(e) => st.set_label(&format!("Retur gagal: {e}")),
            }
        });
        let mut u = update;
        u();
        win.end();
        Self { win }
    }
}
