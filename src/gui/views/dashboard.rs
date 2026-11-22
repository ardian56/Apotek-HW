use super::components::*;
use crate::{
    db::operations::{self, ReportKind},
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
use mysql::{params, prelude::*, Pool};

pub struct DashboardView {
    pub win: Window,
}

impl DashboardView {
    pub fn new(pool: Option<Pool>) -> Self {
        let mut win = Window::new(0, 108, 1200, 612, None);
        win.set_color(T::BG_APP);
        let mut scroll = Scroll::new(0, 0, 1200, 612, None);
        scroll.set_type(fltk::group::ScrollType::Vertical);
        text(28, 18, 700, 30, "Ringkasan apotek", 24, true);
        let mut status = text(28, 56, 1140, 20, "", 12, false);
        let mut values = Vec::<Frame>::new();
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
            let mut bg = Frame::new(x, 100, 270, 112, None);
            bg.set_frame(FrameType::FlatBox);
            bg.set_color(Color::White);
            text(x + 16, 112, 238, 20, label, 13, false);
            values.push(text(x + 16, 140, 238, 32, "—", 23, true));
            descriptions.push(text(x + 16, 181, 238, 18, "", 12, false));
        }
        let mut warnings = Vec::new();
        for (i, label) in ["Stok hampir habis", "Stok kosong", "Batch kedaluwarsa"]
            .iter()
            .enumerate()
        {
            warnings.push(text(28 + i as i32 * 382, 232, 364, 30, label, 14, true));
        }
        text(
            28,
            284,
            800,
            24,
            "Penjualan harian • 30 hari terakhir",
            17,
            true,
        );
        let mut chart = Chart::new(28, 320, 1130, 188, None);
        chart.set_type(ChartType::Line);
        chart.set_frame(FrameType::FlatBox);
        chart.set_color(Color::White);
        chart.set_text_size(10);
        text(28, 534, 534, 26, "Obat terlaris bulan ini", 17, true);
        text(606, 534, 534, 26, "Pelanggan teratas bulan ini", 17, true);
        let mut products = Grid::new(28, 574, 550, 244, &["Obat", "Qty dasar", "Penjualan (Rp)"]);
        let mut customers = Grid::new(
            606,
            574,
            552,
            244,
            &["Pelanggan", "Faktur", "Penjualan (Rp)"],
        );
        let _space = Frame::new(28, 836, 1130, 24, None);
        scroll.end();
        win.end();
        let refresh = {
            let mut values = values.clone();
            let mut descriptions = descriptions.clone();
            let mut warnings = warnings.clone();
            let mut chart = chart.clone();
            let mut products = products.clone();
            let mut customers = customers.clone();
            let mut status = status.clone();
            move || {
                let Some(pool) = &pool else {
                    status
                        .set_label("Pratinjau • Hubungkan database untuk melihat ringkasan apotek");
                    return;
                };
                let result = (|| -> Result<(), String> {
                    let mut conn = pool.get_conn().map_err(|e| e.to_string())?;
                    let daily:Option<(f64,f64,u64)>=conn.query_first("SELECT COALESCE(SUM(grand_total),0),COALESCE(SUM(total_profit),0),COUNT(*) FROM sales s WHERE DATE(sale_date)=CURRENT_DATE AND NOT EXISTS(SELECT 1 FROM returns r WHERE r.ref_type='PENJUALAN' AND r.ref_id=s.id)").map_err(|e|e.to_string())?;
                    let monthly:Option<(f64,u64)>=conn.query_first("SELECT COALESCE(SUM(grand_total),0),COUNT(*) FROM sales s WHERE sale_date>=DATE_FORMAT(CURRENT_DATE,'%Y-%m-01') AND sale_date<DATE_ADD(CURRENT_DATE,INTERVAL 1 DAY) AND NOT EXISTS(SELECT 1 FROM returns r WHERE r.ref_type='PENJUALAN' AND r.ref_id=s.id)").map_err(|e|e.to_string())?;
                    let asset:Option<f64>=conn.query_first("SELECT COALESCE(SUM(b.stock*b.buy_price),0) FROM product_batches b JOIN products p ON p.id=b.product_id WHERE p.is_active=1").map_err(|e|e.to_string())?;
                    let stocks:Vec<(i32,i64)>=conn.query("SELECT p.min_stock,COALESCE(SUM(b.stock),0) FROM products p LEFT JOIN product_batches b ON b.product_id=p.id WHERE p.is_active=1 GROUP BY p.id").map_err(|e|e.to_string())?;
                    let expired:Option<u64>=conn.query_first("SELECT COUNT(*) FROM product_batches WHERE stock>0 AND expired_date<=CURRENT_DATE").map_err(|e|e.to_string())?;
                    let (sales, profit, invoices) = daily.unwrap_or_default();
                    let (month, count) = monthly.unwrap_or_default();
                    for (i, amount) in [sales, profit, month, asset.unwrap_or(0.0)]
                        .iter()
                        .enumerate()
                    {
                        values[i].set_label(&format!("Rp {amount:.2}"));
                    }
                    descriptions[0].set_label(&format!("{invoices} faktur selesai"));
                    descriptions[1].set_label("Penjualan dikurangi modal barang");
                    descriptions[2].set_label(&format!("{count} faktur bulan berjalan"));
                    descriptions[3].set_label(&format!("{} obat aktif", stocks.len()));
                    warnings[0].set_label(&format!(
                        "{} obat • Stok hampir habis",
                        stocks
                            .iter()
                            .filter(|(min, stock)| *stock > 0 && *stock <= *min as i64)
                            .count()
                    ));
                    warnings[1].set_label(&format!(
                        "{} obat • Stok kosong",
                        stocks.iter().filter(|(_, stock)| *stock <= 0).count()
                    ));
                    warnings[2].set_label(&format!("{} batch • Kedaluwarsa", expired.unwrap_or(0)));
                    let today = chrono::Local::now().date_naive();
                    let from = (today - chrono::Duration::days(29)).to_string();
                    let to = today.to_string();
                    let rows = operations::report(pool, ReportKind::Daily, &from, &to)?;
                    chart.clear();
                    for row in rows.iter().rev() {
                        chart.add(row[2].parse().unwrap_or(0.0), &row[0][8..10], T::PRIMARY);
                    }
                    let month = today.format("%Y-%m-01").to_string();
                    products.set(
                        operations::report(pool, ReportKind::TopProducts, &month, &to)?
                            .into_iter()
                            .take(10)
                            .map(|r| vec![r[2].clone(), r[3].clone(), r[4].clone()])
                            .collect(),
                    );
                    customers.set(
                        operations::report(pool, ReportKind::TopCustomers, &month, &to)?
                            .into_iter()
                            .take(10)
                            .map(|r| vec![r[1].clone(), r[2].clone(), r[3].clone()])
                            .collect(),
                    );
                    status.set_label(&format!(
                        "Data terakhir dimuat {} • Laba kotor belum dikurangi biaya operasional",
                        chrono::Local::now().format("%H:%M")
                    ));
                    Ok(())
                })();
                if let Err(e) = result {
                    status.set_label(&format!("Ringkasan gagal dimuat: {e}"));
                }
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
