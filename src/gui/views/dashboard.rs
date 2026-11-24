use super::components::*;
use crate::{
    db::{
        demo,
        operations::{self, ReportKind},
    },
    gui::theme::AppTheme as T,
};
use fltk::{
    enums::{Color, FrameType},
    frame::Frame,
    group::Scroll,
    misc::{Chart, ChartType},
    prelude::*,
    window::Window,
};
use mysql::{prelude::*, Pool};

pub struct DashboardView {
    pub win: Window,
}
struct Data {
    numbers: [f64; 4],
    descriptions: [String; 4],
    warnings: [String; 3],
    daily: Vec<Vec<String>>,
    products: Vec<Vec<String>>,
    customers: Vec<Vec<String>>,
}

fn load(pool: Option<&Pool>) -> Result<Data, String> {
    let today = chrono::Local::now().date_naive();
    let to = today.to_string();
    let month = today.format("%Y-%m-01").to_string();
    let from = (today - chrono::Duration::days(29)).to_string();
    let (sales, profit, invoices, monthly, count, asset, stocks, expired) = if let Some(pool) = pool
    {
        let mut conn = pool.get_conn().map_err(|e| e.to_string())?;
        let daily:Option<(f64,f64,u64)>=conn.query_first("SELECT COALESCE(SUM(grand_total),0),COALESCE(SUM(total_profit),0),COUNT(*) FROM sales s WHERE DATE(sale_date)=CURRENT_DATE AND NOT EXISTS(SELECT 1 FROM returns r WHERE r.ref_type='PENJUALAN' AND r.ref_id=s.id)").map_err(|e|e.to_string())?;
        let monthly:Option<(f64,u64)>=conn.query_first("SELECT COALESCE(SUM(grand_total),0),COUNT(*) FROM sales s WHERE sale_date>=DATE_FORMAT(CURRENT_DATE,'%Y-%m-01') AND sale_date<DATE_ADD(CURRENT_DATE,INTERVAL 1 DAY) AND NOT EXISTS(SELECT 1 FROM returns r WHERE r.ref_type='PENJUALAN' AND r.ref_id=s.id)").map_err(|e|e.to_string())?;
        let asset:Option<f64>=conn.query_first("SELECT COALESCE(SUM(b.stock*b.buy_price),0) FROM product_batches b JOIN products p ON p.id=b.product_id WHERE p.is_active=1").map_err(|e|e.to_string())?;
        let stocks:Vec<(i32,i64)>=conn.query("SELECT p.min_stock,COALESCE(SUM(b.stock),0) FROM products p LEFT JOIN product_batches b ON b.product_id=p.id WHERE p.is_active=1 GROUP BY p.id").map_err(|e|e.to_string())?;
        let expired: Option<u64> = conn
            .query_first(
                "SELECT COUNT(*) FROM product_batches WHERE stock>0 AND expired_date<=CURRENT_DATE",
            )
            .map_err(|e| e.to_string())?;
        let (sales, profit, invoices) = daily.unwrap_or_default();
        let (month, count) = monthly.unwrap_or_default();
        (
            sales,
            profit,
            invoices,
            month,
            count,
            asset.unwrap_or(0.0),
            stocks,
            expired.unwrap_or(0),
        )
    } else {
        let all = demo::sales();
        let sales = all.into_iter().take(8).collect::<Vec<_>>();
        let daily = sales
            .iter()
            .filter(|s| s.sale_date[..10] == to)
            .collect::<Vec<_>>();
        let monthly = sales
            .iter()
            .filter(|s| &s.sale_date[..10] >= month.as_str())
            .collect::<Vec<_>>();
        let products = demo::products();
        let stocks = products
            .iter()
            .map(|p| (p.min_stock, p.total_stock as i64))
            .collect();
        let expired = products
            .iter()
            .flat_map(|p| &p.batches)
            .filter(|b| {
                b.stock > 0
                    && b.expired_date
                        .as_deref()
                        .map(|d| d <= to.as_str())
                        .unwrap_or(false)
            })
            .count() as u64;
        (
            daily.iter().map(|s| s.grand_total).sum(),
            daily.iter().map(|s| s.total_profit).sum(),
            daily.len() as u64,
            monthly.iter().map(|s| s.grand_total).sum(),
            monthly.len() as u64,
            products
                .iter()
                .map(|p| p.total_stock as f64 * p.buy_price)
                .sum(),
            stocks,
            expired,
        )
    };
    Ok(Data {
        numbers: [sales, profit, monthly, asset],
        descriptions: [
            format!("{invoices} faktur hari ini"),
            "Penjualan dikurangi modal obat".into(),
            format!("{count} faktur bulan berjalan"),
            format!("{} obat aktif", stocks.len()),
        ],
        warnings: [
            format!(
                "{} obat · Stok hampir habis",
                stocks
                    .iter()
                    .filter(|(min, s)| *s > 0 && *s <= *min as i64)
                    .count()
            ),
            format!(
                "{} obat · Stok kosong",
                stocks.iter().filter(|(_, s)| *s <= 0).count()
            ),
            format!("{expired} batch · Kedaluwarsa"),
        ],
        daily: operations::view_report(pool, ReportKind::Daily, &from, &to)?,
        products: operations::view_report(pool, ReportKind::TopProducts, &month, &to)?
            .into_iter()
            .take(10)
            .map(|r| vec![r[2].clone(), r[3].clone(), r[4].clone()])
            .collect(),
        customers: operations::view_report(pool, ReportKind::TopCustomers, &month, &to)?
            .into_iter()
            .take(10)
            .map(|r| vec![r[1].clone(), r[2].clone(), r[3].clone()])
            .collect(),
    })
}

impl DashboardView {
    pub fn new(pool: Option<Pool>) -> Self {
        let mut win = Window::new(0, 108, 1200, 612, None);
        win.set_color(T::BG_APP);
        let mut scroll = Scroll::new(0, 0, 1200, 612, None);
        scroll.set_type(fltk::group::ScrollType::Vertical);
        page_header("Ringkasan apotek", "");
        let status = text(28, 58, 1144, 20, "", 12, false);
        let mut numbers = Vec::<Frame>::new();
        let mut descriptions = Vec::<Frame>::new();
        for (i, label) in [
            "Penjualan hari ini",
            "Laba kotor hari ini",
            "Penjualan bulan ini",
            "Nilai modal stok",
        ]
        .iter()
        .enumerate()
        {
            let x = 28 + i as i32 * 286;
            panel(x, 96, 270, 112);
            text(x + 16, 110, 238, 20, label, 13, false);
            numbers.push(text(x + 16, 140, 238, 30, "—", 24, true));
            let mut d = text(x + 16, 180, 238, 18, "", 12, false);
            d.set_label_color(T::TEXT_MUTED);
            descriptions.push(d);
        }
        let mut warnings = Vec::new();
        for i in 0..3 {
            let x = 28 + i * 382;
            if i == 0 {
                panel(28, 226, 1144, 48);
            }
            warnings.push(text(x + 16, 238, 332, 24, "", 13, true));
        }
        text(28, 296, 800, 26, "Penjualan harian", 16, true);
        text(948, 296, 220, 26, "30 hari terakhir", 12, false);
        let mut chart = Chart::new(28, 336, 1130, 188, None);
        chart.set_type(ChartType::Line);
        chart.set_frame(FrameType::BorderBox);
        chart.set_color(Color::White);
        chart.set_text_size(10);
        text(28, 550, 550, 26, "Obat terlaris bulan ini", 16, true);
        text(606, 550, 552, 26, "Pelanggan teratas bulan ini", 16, true);
        let mut products = Grid::new(28, 588, 550, 240, &["Obat", "Qty dasar", "Penjualan (Rp)"]);
        let mut customers = Grid::new(
            606,
            588,
            552,
            240,
            &["Pelanggan", "Faktur", "Penjualan (Rp)"],
        );
        for g in [&mut products, &mut customers] {
            g.table.set_col_width(0, 282);
            g.table.set_col_width(1, 80);
            g.table.set_col_width(2, 170);
        }
        let _space = Frame::new(28, 840, 1130, 24, None);
        scroll.end();
        win.end();
        let refresh = {
            let mut n = numbers.clone();
            let mut d = descriptions.clone();
            let mut w = warnings.clone();
            let mut c = chart.clone();
            let mut p = products.clone();
            let mut cs = customers.clone();
            let mut st = status.clone();
            move || match load(pool.as_ref()) {
                Ok(data) => {
                    for i in 0..4 {
                        n[i].set_label(&rupiah(data.numbers[i]));
                        d[i].set_label(&data.descriptions[i]);
                    }
                    for i in 0..3 {
                        w[i].set_label(&data.warnings[i]);
                    }
                    c.clear();
                    let today = chrono::Local::now().date_naive();
                    for i in 0..30 {
                        let day = today - chrono::Duration::days(29 - i);
                        let value = data
                            .daily
                            .iter()
                            .find(|r| r[0] == day.to_string())
                            .and_then(|r| r[2].parse::<f64>().ok())
                            .unwrap_or(0.0);
                        c.add(
                            value,
                            &if i % 5 == 0 {
                                day.format("%d/%m").to_string()
                            } else {
                                String::new()
                            },
                            T::PRIMARY,
                        );
                    }
                    p.set(data.products);
                    cs.set(data.customers);
                    st.set_label(&format!(
                        "{}{} · Laba kotor belum dikurangi biaya operasional",
                        if pool.is_none() { "Contoh · " } else { "" },
                        today.format("%d/%m/%Y")
                    ));
                }
                Err(e) => st.set_label(&format!("Ringkasan gagal dimuat: {e}")),
            }
        };
        let mut r = refresh.clone();
        win.handle(move |_, event| {
            if event == fltk::enums::Event::Show {
                r();
            }
            false
        });
        let mut r = refresh;
        r();
        Self { win }
    }
}
