use super::components::*;
use crate::{
    db::operations::{self, ReportKind},
    gui::theme::AppTheme as T,
    models::User,
};
use fltk::{
    button::Button, enums::CallbackTrigger, frame::Frame, menu::Choice, prelude::*, window::Window,
};
use mysql::Pool;

pub struct ReportsView {
    pub win: Window,
    pub add: Button,
}

fn financial(kind: ReportKind) -> bool {
    matches!(
        kind,
        ReportKind::Sales
            | ReportKind::Purchases
            | ReportKind::Receivables
            | ReportKind::Debts
            | ReportKind::Payments
            | ReportKind::Returns
    )
}

fn details(
    pool: &Option<Pool>,
    kind: ReportKind,
    list: &Grid,
    items: &mut Grid,
    title: &mut Frame,
    caption: &mut Frame,
) {
    let Some(row) = list.selected() else {
        items.set(vec![]);
        title.set_label("Rincian obat");
        caption.set_label("Pilih faktur pada daftar di atas.");
        return;
    };
    let result = operations::view_invoice(pool.as_ref(), kind, row[0].parse().unwrap_or(0));
    match result {
        Ok((sale, data)) => {
            title.set_label(&format!("Obat dalam faktur {}", data.invoice));
            let movement = if data.returned {
                "Faktur diretur; lihat kartu stok."
            } else if sale {
                "Stok berkurang sesuai satuan dasar."
            } else {
                "Stok bertambah sesuai satuan dasar."
            };
            caption.set_label(&format!(
                "{} · {} · {} · Total {} · Diterima {} · Kembali {} · {}",
                data.date,
                data.party,
                if data.payment == "KREDIT" {
                    "Kredit"
                } else {
                    "Tunai"
                },
                rupiah(data.total),
                rupiah(data.received),
                rupiah(data.change),
                movement
            ));
            items.set(data.items);
        }
        Err(e) => {
            items.set(vec![]);
            caption.set_label(&format!("Rincian gagal dimuat: {e}"));
        }
    }
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
    if kind.headers().first() == Some(&"ID") {
        grid.table.set_col_width(0, 0);
        if matches!(
            kind,
            ReportKind::Sales | ReportKind::Purchases | ReportKind::Receivables | ReportKind::Debts
        ) {
            for (i, w) in [0, 218, 148, 244, 166, 166, 172].iter().enumerate() {
                grid.table.set_col_width(i as i32, *w);
            }
        }
    }
    match operations::view_report(pool.as_ref(), kind, from, to) {
        Ok(rows) => {
            let query = search.to_lowercase();
            let filtered = rows
                .into_iter()
                .filter(|r| r.iter().any(|s| s.to_lowercase().contains(&query)))
                .collect::<Vec<_>>();
            status.set_label(&format!(
                "{}{} hasil. Pilih baris untuk melihat rincian.",
                if pool.is_none() { "Contoh · " } else { "" },
                filtered.len()
            ));
            grid.set(filtered);
            grid.select_first();
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
        let single = kinds.len() == 1;
        let title = if single {
            kinds[0].title()
        } else {
            "Laporan apotek"
        };
        page_header(
            title,
            if single {
                "Pilih faktur untuk melihat obat, jumlah, pembayaran, dan perubahan stok."
            } else {
                "Pilih laporan dan periode yang ingin ditampilkan."
            },
        );
        let add = button(
            984,
            24,
            188,
            if kinds[0] == ReportKind::Purchases {
                "Tambah pembelian"
            } else {
                "Tambah penjualan"
            },
            true,
        );
        let mut add = add;
        if !single || !matches!(kinds[0], ReportKind::Sales | ReportKind::Purchases) {
            add.hide();
        }
        let mut kind = Choice::new(28, 112, 208, 34, None);
        style_choice(&mut kind);
        kind.set_text_size(14);
        let mut kind_label = text(28, 88, 208, 18, "Jenis laporan", 12, false);
        for k in &kinds {
            kind.add_choice(k.title());
        }
        kind.set_value(0);
        if single {
            kind.hide();
            kind_label.hide();
        }
        let offset = if single { 0 } else { 224 };
        let from = input(
            28 + offset,
            88,
            160,
            "Dari tanggal",
            &chrono::Local::now().format("%Y-%m-01").to_string(),
        );
        let to = input(
            204 + offset,
            88,
            160,
            "Sampai tanggal",
            &chrono::Local::now().date_naive().to_string(),
        );
        let mut search = input(
            380 + offset,
            88,
            if single { 432 } else { 208 },
            "Cari faktur atau nama",
            "",
        );
        search.set_trigger(CallbackTrigger::EnterKey);
        let mut refresh = button(828, 112, 140, "Tampilkan", false);
        let mut export = button(984, 112, 188, "Ekspor Excel", false);
        let mut grid = Grid::new(28, 166, 1144, 148, &kinds[0].headers());
        let detail_title = text(28, 326, 1144, 26, "Rincian obat", 15, true);
        let mut detail_caption = text(28, 358, 1144, 22, "", 12, false);
        detail_caption.set_label_color(T::TEXT_MUTED);
        let mut items = Grid::new(
            28,
            386,
            1144,
            138,
            &[
                "Kode",
                "Obat",
                "Qty",
                "Satuan",
                "Harga (Rp)",
                "Diskon (Rp)",
                "Subtotal (Rp)",
                "Stok dasar",
                "Batch",
            ],
        );
        for (i, w) in [96, 228, 44, 76, 108, 100, 112, 122, 142]
            .iter()
            .enumerate()
        {
            items.table.set_col_width(i as i32, *w);
        }
        let mut pay = button(984, 552, 188, "Catat pembayaran", true);
        let mut returns = button(28, 552, 184, "Retur faktur", false);
        returns.set_label_color(T::DANGER_TEXT);
        let status = text(28, 592, 1144, 18, "", 12, false);
        if pool.is_none() {
            disable(&mut pay);
            disable(&mut returns);
            disable(&mut export);
        }
        let is_admin = user.is_admin();
        let detail = {
            let pool = pool.clone();
            let ks = kinds.clone();
            let k = kind.clone();
            let g = grid.clone();
            let mut i = items.clone();
            let mut t = detail_title.clone();
            let mut c = detail_caption.clone();
            move || {
                details(
                    &pool,
                    ks[k.value().max(0) as usize],
                    &g,
                    &mut i,
                    &mut t,
                    &mut c,
                );
            }
        };
        let update = {
            let pool = pool.clone();
            let ks = kinds.clone();
            let k = kind.clone();
            let f = from.clone();
            let t = to.clone();
            let s = search.clone();
            let mut g = grid.clone();
            let mut st = status.clone();
            let mut p = pay.clone();
            let mut r = returns.clone();
            let mut dt = detail_title.clone();
            let mut dc = detail_caption.clone();
            let mut i = items.clone();
            let mut detail = detail.clone();
            move || {
                let kind = ks[k.value().max(0) as usize];
                load(
                    &pool,
                    kind,
                    &f.value(),
                    &t.value(),
                    &s.value(),
                    &mut g,
                    &mut st,
                );
                if financial(kind) {
                    g.table.resize(28, 166, 1144, 148);
                    dt.show();
                    dc.show();
                    i.table.show();
                    detail();
                } else {
                    g.table.resize(28, 166, 1144, 358);
                    dt.hide();
                    dc.hide();
                    i.table.hide();
                }
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
        let mut u = update.clone();
        refresh.set_callback(move |_| u());
        let mut u = update.clone();
        kind.set_callback(move |_| u());
        let mut u = update.clone();
        search.set_callback(move |_| u());
        let mut d = detail;
        grid.table.set_trigger(CallbackTrigger::Release);
        grid.table.set_callback(move |t| {
            let (row, _, _, _) = t.get_selection();
            if row >= 0 {
                t.set_selection(row, 0, row, t.cols() - 1);
                t.redraw();
            }
            d();
        });
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
            let Some(path) = save_path("Ekspor laporan", "Excel\t*.xlsx") else {
                return;
            };
            let result = (|| -> Result<(), String> {
                let mut workbook = rust_xlsxwriter::Workbook::new();
                let sheet = workbook.add_worksheet();
                let headers = g.headers.borrow();
                let skip = usize::from(headers.first().map(String::as_str) == Some("ID"));
                for (c, h) in headers.iter().skip(skip).enumerate() {
                    sheet
                        .write_string(0, c as u16, h)
                        .map_err(|e| e.to_string())?;
                }
                for (r, row) in g.rows.borrow().iter().enumerate() {
                    for (c, value) in row.iter().skip(skip).enumerate() {
                        if headers[c + skip].contains("(Rp)") || headers[c + skip] == "Qty dasar" {
                            if let Ok(n) = value.parse::<f64>() {
                                sheet
                                    .write_number(r as u32 + 1, c as u16, n)
                                    .map_err(|e| e.to_string())?;
                                continue;
                            }
                        }
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
                st.set_label("Pilih faktur pada tabel");
                return;
            };
            let Some(p) = &p else {
                return;
            };
            let Some(value) = fltk::dialog::input_default(
                &format!(
                    "Pembayaran {}. Sisa {}",
                    row[1],
                    rupiah(row[6].parse().unwrap_or(0.0))
                ),
                &row[6],
            ) else {
                return;
            };
            match operations::money(&value).and_then(|amount| {
                operations::pay(
                    p,
                    ks[k.value() as usize] == ReportKind::Receivables,
                    row[0].parse().unwrap_or(0),
                    amount,
                    operator,
                )
            }) {
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
                st.set_label("Pilih faktur pada tabel");
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
        Self { win, add }
    }
}
