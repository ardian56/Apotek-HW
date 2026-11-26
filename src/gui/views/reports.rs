use super::components::*;
use crate::{
    db::operations::{self, ReportKind},
    gui::theme::AppTheme as T,
    models::User,
};
use fltk::{
    button::Button,
    enums::{CallbackTrigger, Event, FrameType, Key, Shortcut},
    frame::Frame,
    menu::{Choice, MenuButton, MenuFlag},
    prelude::*,
    window::Window,
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
                "Faktur diretur".to_string()
            } else if sale {
                format!("Kembalian {}", rupiah(data.change))
            } else {
                "Stok masuk".to_string()
            };
            caption.set_label(&format!(
                "{} · {} · {} · {}",
                data.date,
                data.party,
                if data.payment == "KREDIT" {
                    "Kredit"
                } else {
                    "Tunai"
                },
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
    filters: &operations::ReportFilter,
    grid: &mut Grid,
    status: &mut Frame,
) {
    let all_headers = kind.headers();
    let doctor_column = all_headers.iter().position(|h| *h == "Dokter");
    let headers = all_headers
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != doctor_column)
        .map(|(_, h)| *h)
        .collect::<Vec<_>>();
    grid.columns(&headers);
    if kind.headers().first() == Some(&"ID") {
        grid.table.set_col_width(0, 0);
        if matches!(
            kind,
            ReportKind::Sales | ReportKind::Purchases | ReportKind::Receivables | ReportKind::Debts
        ) {
            for (i, w) in [0, 218, 148, 244, 166, 166, 172].iter().enumerate() {
                grid.table
                    .set_col_width(i as i32, *w * grid.table.w() / 1144);
            }
        }
    }
    match operations::view_report_filtered(pool.as_ref(), kind, from, to, filters) {
        Ok(rows) => {
            let query = search.to_lowercase();
            let filtered = rows
                .into_iter()
                .map(|mut r| {
                    if let Some(column) = doctor_column {
                        r.remove(column);
                    }
                    r
                })
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
                "Pilih faktur untuk melihat rincian obat."
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
            } else if single && kinds[0] == ReportKind::Sales {
                "Tambah penjualan [F1]"
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
            144,
            "Dari tanggal",
            &chrono::Local::now().format("%Y-%m-01").to_string(),
        );
        let to = input(
            188 + offset,
            88,
            144,
            "Sampai tanggal",
            &chrono::Local::now().date_naive().to_string(),
        );
        let mut search = input(
            348 + offset,
            88,
            if single { 480 } else { 240 },
            "Cari faktur atau nama",
            "",
        );
        search.set_trigger(CallbackTrigger::EnterKey);
        let mut refresh = button(1020, 112, 152, "Tampilkan", false);
        let mut filter_button = button(
            if single { 844 } else { 828 },
            112,
            if single { 160 } else { 176 },
            "Filter tambahan",
            false,
        );
        let filters_open = std::rc::Rc::new(std::cell::Cell::new(false));
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
                "Qty dasar",
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
        fn filter_choice(
            x: i32,
            w: i32,
            label: &str,
            options: &[(i32, String)],
        ) -> (Frame, Choice) {
            let label = text(x, 154, w, 18, label, 12, false);
            let mut choice = Choice::new(x, 178, w, 34, None);
            style_choice(&mut choice);
            choice.add_choice("Semua");
            for (_, name) in options {
                choice.add_choice(&name.replace('|', " ").replace('/', " "));
            }
            choice.set_value(0);
            (label, choice)
        }
        let users = crate::db::named_people(pool.as_ref(), "users").unwrap_or_default();
        let customers = if let Some(pool) = &pool {
            crate::db::get_customers(pool)
                .unwrap_or_default()
                .into_iter()
                .map(|r| (r.0, r.1))
                .collect::<Vec<_>>()
        } else {
            crate::db::demo::master_rows("customers")
                .into_iter()
                .map(|r| (r[0].parse().unwrap_or(0), r[1].clone()))
                .collect()
        };
        let (operator_label, operator) = filter_choice(28, 260, "Operator", &users);
        let (customer_label, customer) = filter_choice(304, 432, "Pelanggan", &customers);
        let (payment_label, payment) = filter_choice(
            752,
            420,
            "Pembayaran",
            &[(1, "Tunai".into()), (2, "Kredit".into())],
        );
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
            let page = win.clone();
            let open = filters_open.clone();
            let mut filter_button = filter_button.clone();
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
            let mut operator = operator.clone();
            let mut customer = customer.clone();
            let mut payment = payment.clone();
            let mut labels = [
                operator_label.clone(),
                customer_label.clone(),
                payment_label.clone(),
            ];
            let users = users.clone();
            let customers = customers.clone();
            move || {
                let kind = ks[k.value().max(0) as usize];
                load(
                    &pool,
                    kind,
                    &f.value(),
                    &t.value(),
                    &s.value(),
                    &operations::ReportFilter {
                        operator: users
                            .get((operator.value() - 1) as usize)
                            .map(|r| r.0)
                            .unwrap_or(0),
                        customer: customers
                            .get((customer.value() - 1) as usize)
                            .map(|r| r.0)
                            .unwrap_or(0),
                        doctor: 0,
                        payment: match payment.value() {
                            1 => "TUNAI".into(),
                            2 => "KREDIT".into(),
                            _ => String::new(),
                        },
                    },
                    &mut g,
                    &mut st,
                );
                let sale_filters = kind == ReportKind::Sales && open.get();
                if kind == ReportKind::Sales {
                    filter_button.show();
                    filter_button.set_label(if open.get() {
                        "Tutup filter"
                    } else if operator.value() > 0 || customer.value() > 0 || payment.value() > 0 {
                        "Filter aktif"
                    } else {
                        "Filter tambahan"
                    });
                } else {
                    filter_button.hide();
                }
                for c in [&mut operator, &mut customer, &mut payment] {
                    if sale_filters {
                        c.show();
                    } else {
                        c.hide();
                    }
                }
                for label in &mut labels {
                    if sale_filters {
                        label.show();
                    } else {
                        label.hide();
                    }
                }
                if financial(kind) {
                    resize_in_page(
                        &mut g.table,
                        &page,
                        28,
                        if sale_filters { 234 } else { 166 },
                        1144,
                        if sale_filters { 126 } else { 194 },
                    );
                    resize_in_page(&mut dt, &page, 28, 374, 1144, 26);
                    resize_in_page(&mut dc, &page, 28, 402, 1144, 22);
                    resize_in_page(&mut i.table, &page, 28, 430, 1144, 112);
                    dt.show();
                    dc.show();
                    i.table.show();
                    detail();
                } else {
                    resize_in_page(&mut g.table, &page, 28, 166, 1144, 358);
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
        let open = filters_open.clone();
        let mut u = update.clone();
        filter_button.set_callback(move |_| {
            open.set(!open.get());
            u();
        });
        for mut choice in [operator, customer, payment] {
            let mut u = update.clone();
            choice.set_callback(move |_| u());
        }
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
        let sales_list = single && kinds[0] == ReportKind::Sales;
        let mut add_sale = add.clone();
        win.handle(move |window, event| {
            if event == Event::Show {
                u();
            }
            if sales_list
                && window.visible_r()
                && fltk::app::modal().is_none()
                && add_sale.visible_r()
                && add_sale.active_r()
                && matches!(event, Event::KeyDown | Event::Shortcut)
                && fltk::app::event_key() == Key::F1
            {
                add_sale.do_callback();
                return true;
            }
            false
        });
        let mut print = button(812, 24, 156, "Cetak laporan", false);
        let mut receipt = button(
            624,
            24,
            172,
            if single {
                "Pratinjau struk"
            } else {
                "Grafik bulanan"
            },
            false,
        );
        if single && kinds[0] != ReportKind::Sales {
            receipt.hide();
        }
        let mut tools = MenuButton::new(788, 24, 180, 34, "Cetak dan ekspor");
        tools.set_frame(FrameType::RFlatBox);
        tools.set_color(T::TABLE_HEADER);
        tools.set_label_color(T::TEXT_BODY);
        tools.set_label_size(13);
        if !single || kinds[0] == ReportKind::Sales {
            let mut action = receipt.clone();
            tools.add(
                if single {
                    "Lihat struk"
                } else {
                    "Grafik bulanan"
                },
                Shortcut::None,
                MenuFlag::Normal,
                move |_| action.do_callback(),
            );
        }
        let mut action = print.clone();
        tools.add(
            "Cetak daftar",
            Shortcut::None,
            MenuFlag::Normal,
            move |_| action.do_callback(),
        );
        let mut action = export.clone();
        tools.add(
            "Ekspor Excel",
            Shortcut::None,
            if pool.is_some() {
                MenuFlag::Normal
            } else {
                MenuFlag::Inactive
            },
            move |_| action.do_callback(),
        );
        receipt.hide();
        print.hide();
        export.hide();
        let g = grid.clone();
        let p = pool.clone();
        let ks = kinds.clone();
        let k = kind.clone();
        let from_print = from.clone();
        let to_print = to.clone();
        let mut st = status.clone();
        print.set_callback(move |_| {
            let current = ks[k.value().max(0) as usize];
            let values = p
                .as_ref()
                .map(operations::settings)
                .transpose()
                .ok()
                .flatten()
                .unwrap_or_else(crate::db::demo::settings);
            let title = format!(
                "{} · {}",
                operations::pharmacy_name_from_values(&values),
                current.title()
            );
            let period = format!("{} – {}", from_print.value(), to_print.value());
            let data = g.rows.borrow();
            let html = if current == ReportKind::ProductSales {
                crate::services::documents::grouped_sales_html(&title, &period, &data)
            } else {
                let headers = g.headers.borrow();
                let skip = usize::from(headers.first().map(String::as_str) == Some("ID"));
                let headers = headers
                    .iter()
                    .skip(skip)
                    .map(String::as_str)
                    .collect::<Vec<_>>();
                let rows = data.iter().map(|r| r[skip..].to_vec()).collect::<Vec<_>>();
                crate::services::documents::report_html(&title, &period, &headers, &rows)
            };
            if let Err(e) = crate::services::documents::open_html(&html) {
                st.set_label(&e);
            }
        });
        let p = pool.clone();
        let g = grid.clone();
        let mut st = status.clone();
        receipt.set_callback(move |_| {
            if !single {
                if let Err(e) = monthly_chart(p.as_ref()) {
                    st.set_label(&e);
                }
                return;
            }
            let Some(row) = g.selected() else {
                st.set_label("Pilih faktur");
                return;
            };
            let id = row[0].parse::<i32>().unwrap_or(0);
            let result = (|| -> Result<(), String> {
                let sale = if let Some(pool) = &p {
                    crate::db::get_sale(pool, id).map_err(|e| e.to_string())?
                } else {
                    crate::db::demo::sales()
                        .into_iter()
                        .find(|s| s.id == id)
                        .ok_or("Faktur tidak ditemukan")?
                };
                let settings =
                    crate::services::documents::receipt_settings(p.as_ref(), &sale, None)?;
                super::print_preview::show_receipt(sale, settings, false);
                Ok(())
            })();
            if let Err(e) = result {
                st.set_label(&e);
            }
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

fn monthly_chart(pool: Option<&Pool>) -> Result<(), String> {
    let Some(month) = fltk::dialog::input_default(
        "Bulan grafik (YYYY-MM):",
        &chrono::Local::now().format("%Y-%m").to_string(),
    ) else {
        return Ok(());
    };
    let from = chrono::NaiveDate::parse_from_str(&format!("{}-01", month.trim()), "%Y-%m-%d")
        .map_err(|_| "Bulan harus YYYY-MM")?;
    let to = from
        .checked_add_months(chrono::Months::new(1))
        .ok_or("Bulan di luar batas")?
        - chrono::Duration::days(1);
    let rows =
        operations::view_report(pool, ReportKind::Daily, &from.to_string(), &to.to_string())?;
    let values = pool
        .map(operations::settings)
        .transpose()?
        .unwrap_or_else(crate::db::demo::settings);
    let title = format!(
        "{} · Omzet dan laba harian",
        operations::pharmacy_name_from_values(&values)
    );
    let html = crate::services::documents::monthly_chart_html(&title, month.trim(), &rows)?;
    crate::services::documents::open_html(&html)
}
