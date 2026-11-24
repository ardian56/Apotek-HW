use super::barcode::{escape, file_uri};
use crate::{hardware::ReceiptSettings, models::Sale};

pub fn receipt_settings(
    pool: Option<&mysql::Pool>,
    sale: &Sale,
    paper: Option<u8>,
) -> Result<ReceiptSettings, String> {
    let values = pool
        .map(crate::db::operations::settings)
        .transpose()?
        .unwrap_or_else(crate::db::demo::settings);
    let mut settings = ReceiptSettings::from_values(&values);
    if let Some(width) = paper {
        settings.paper_width = width;
    }
    if let Some(pool) = pool {
        use mysql::prelude::*;
        let mut conn = pool.get_conn().map_err(|e| e.to_string())?;
        if let Some(id) = sale.customer_id {
            settings.customer_name = conn
                .exec_first("SELECT name FROM customers WHERE id=?", (id,))
                .map_err(|e| e.to_string())?
                .unwrap_or_else(|| "Pelanggan umum".into());
        }
        settings.operator_name = conn
            .exec_first(
                "SELECT full_name FROM users WHERE id=?",
                (sale.operator_id,),
            )
            .map_err(|e| e.to_string())?
            .unwrap_or_default();
        settings.returned = conn
            .exec_first(
                "SELECT EXISTS(SELECT 1 FROM returns WHERE invoice_no=? AND ref_type='PENJUALAN')",
                (&sale.invoice_no,),
            )
            .map_err(|e| e.to_string())?
            .unwrap_or(false);
    } else {
        settings.customer_name = crate::db::demo::master_rows("customers")
            .iter()
            .find(|r| r[0].parse::<i32>().ok() == sale.customer_id)
            .map(|r| r[1].clone())
            .unwrap_or_else(|| "Pelanggan umum".into());
        settings.operator_name = "Mode contoh".into();
        settings.returned = sale.id == 9;
    }
    Ok(settings)
}

pub fn open_html(html: &str) -> Result<(), String> {
    let path = std::env::temp_dir().join(format!(
        "apotek-print-{}-{}.html",
        std::process::id(),
        chrono::Utc::now().timestamp_micros()
    ));
    std::fs::write(&path, html).map_err(|e| e.to_string())?;
    fltk::utils::open_uri(&file_uri(&path))
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn print_receipt(pool: &mysql::Pool, sale: &Sale, paper: u8) -> Result<(), String> {
    let settings = receipt_settings(Some(pool), sale, Some(paper))?;
    if cfg!(target_os = "windows") && !settings.printer_name.trim().is_empty() {
        crate::hardware::print_raw_to_spooler(
            &settings.printer_name,
            &crate::hardware::generate_escpos_bytes(sale, &settings),
        )
    } else {
        open_html(&receipt_html(sale, &settings).replacen(
            "<body>",
            "<body onload=\"window.print()\">",
            1,
        ))
    }
}

pub fn report_html(
    title: &str,
    description: &str,
    headers: &[&str],
    rows: &[Vec<String>],
) -> String {
    let head = headers
        .iter()
        .map(|h| format!("<th>{}</th>", escape(h)))
        .collect::<String>();
    let body = rows
        .iter()
        .map(|r| {
            format!(
                "<tr>{}</tr>",
                r.iter()
                    .map(|v| format!("<td>{}</td>", escape(v)))
                    .collect::<String>()
            )
        })
        .collect::<String>();
    page(
        title,
        &format!(
            "<p>{}</p><table><thead><tr>{head}</tr></thead><tbody>{body}</tbody></table>",
            escape(description).replace('\n', "<br>")
        ),
        "",
    )
}

pub fn page(title: &str, body: &str, css: &str) -> String {
    format!(
        r#"<!doctype html><html lang="id"><meta charset="utf-8"><title>{}</title><style>
*{{box-sizing:border-box}}body{{margin:28px;color:#334155;font:13px Arial;background:#f8fafc}}main{{background:white;padding:24px;max-width:1100px;margin:auto}}h1{{font-size:20px}}
button{{padding:10px 18px;background:#0d9488;color:white;border:0;border-radius:5px;cursor:pointer}}table{{border-collapse:collapse;width:100%}}th,td{{padding:7px 6px;text-align:left;border-bottom:1px solid #e2e8f0}}th{{background:#f1f5f9}}thead{{display:table-header-group}}tr{{break-inside:avoid}}.controls{{margin-bottom:20px}}
@page{{margin:12mm}}@media print{{body{{margin:0;background:white;color:black}}main{{padding:0}}.controls{{display:none}}}}{css}</style><body><div class=controls><button onclick="window.print()">Cetak / simpan PDF</button></div><main><h1>{}</h1>{body}</main></body></html>"#,
        escape(title),
        escape(title)
    )
}

pub fn receipt_html(sale: &Sale, settings: &ReceiptSettings) -> String {
    let mut body = format!("<p>{}<br>{}<br>{}</p><p>Faktur: {}<br>Tanggal: {}<br>Pembayaran: {}</p><table><thead><tr><th>Obat</th><th>Qty</th><th>Total</th></tr></thead><tbody>",
        escape(&settings.address),escape(&settings.phone),escape(&settings.header_note),escape(&sale.invoice_no),escape(&sale.sale_date),escape(&sale.payment_type));
    body.push_str(&format!(
        "<tr><td colspan=3>Pelanggan: {} · Operator: {}</td></tr>",
        escape(&settings.customer_name),
        escape(&settings.operator_name)
    ));
    for item in &sale.items {
        body.push_str(&format!(
            "<tr><td>{}<br><small>{} @ {} · Disc {}</small></td><td>{}</td><td>{}</td></tr>",
            escape(&item.product_name),
            escape(&item.unit_name),
            crate::gui::views::components::number(item.sell_price),
            crate::gui::views::components::number(item.discount),
            item.qty,
            crate::gui::views::components::number(item.subtotal)
        ));
    }
    let amount = crate::gui::views::components::rupiah;
    let payment_detail = if sale.payment_type == "KREDIT" {
        format!(
            "Sisa: {}<br>Jatuh tempo: {}",
            amount(if settings.returned {
                0.0
            } else {
                (sale.grand_total - sale.paid_amount).max(0.0)
            }),
            escape(sale.due_date.as_deref().unwrap_or("—"))
        )
    } else {
        format!("Kembali: {}", amount(sale.change_amount))
    };
    body.push_str(&format!("</tbody></table><p>Subtotal: {}<br>Diskon: {}<br>Ongkos: {}<br><b>Total: {}</b><br>Bayar: {}<br>{payment_detail}</p><p>{}</p>",
        amount(sale.subtotal),amount(sale.discount),amount(sale.shipping),amount(sale.grand_total),amount(sale.paid_amount),escape(&settings.footer_note).replace('\n',"<br>")));
    if settings.returned {
        body.insert_str(0, "<p><b>FAKTUR DIRETUR (salinan)</b></p>");
    }
    page(&settings.store_name,&body,&format!("main{{width:{}mm;max-width:100%}}th,td{{padding:4px 1px;font-size:10px}}h1{{font-size:14px}}@page{{margin:3mm}}",settings.paper_width))
}

pub fn grouped_sales_html(title: &str, period: &str, rows: &[Vec<String>]) -> String {
    let mut html = format!("<p>{}</p>", escape(period));
    let mut group = String::new();
    let mut total = 0.0;
    let mut profit = 0.0;
    for row in rows {
        if row.len() < 11 {
            continue;
        }
        if row[1] != group {
            if !group.is_empty() {
                html.push_str(&format!(
                    "</tbody></table><p><b>Total {} · Laba {}</b></p>",
                    crate::gui::views::components::rupiah(total),
                    crate::gui::views::components::rupiah(profit)
                ));
            }
            group = row[1].clone();
            total = 0.0;
            profit = 0.0;
            html.push_str(&format!("<h2>{} · {}</h2><table><thead><tr><th>Faktur</th><th>Tanggal</th><th>Harga</th><th>Qty</th><th>Satuan</th><th>Total</th><th>Laba</th><th>Pelanggan</th></tr></thead><tbody>",escape(&row[1]),escape(&row[2])));
        }
        total += row[8].parse::<f64>().unwrap_or(0.0);
        profit += row[9].parse::<f64>().unwrap_or(0.0);
        html.push_str(&format!(
            "<tr>{}</tr>",
            row[3..]
                .iter()
                .map(|v| format!("<td>{}</td>", escape(v)))
                .collect::<String>()
        ));
    }
    if !group.is_empty() {
        html.push_str(&format!(
            "</tbody></table><p><b>Total {} · Laba {}</b></p>",
            crate::gui::views::components::rupiah(total),
            crate::gui::views::components::rupiah(profit)
        ));
    }
    page(
        title,
        &html,
        "h2{margin-top:28px;font-size:14px}@page{size:A4 landscape}",
    )
}

pub fn monthly_chart_html(
    title: &str,
    month: &str,
    rows: &[Vec<String>],
) -> Result<String, String> {
    let from = chrono::NaiveDate::parse_from_str(&format!("{month}-01"), "%Y-%m-%d")
        .map_err(|_| "Bulan harus YYYY-MM")?;
    let next = from
        .checked_add_months(chrono::Months::new(1))
        .ok_or("Bulan di luar batas")?;
    let days = (next - from).num_days();
    let values = (0..days)
        .map(|n| {
            let day = (from + chrono::Duration::days(n)).to_string();
            rows.iter()
                .find(|r| r.first() == Some(&day))
                .map(|r| {
                    (
                        r.get(2).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
                        r.get(3).and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0),
                    )
                })
                .unwrap_or((0.0, 0.0))
        })
        .collect::<Vec<_>>();
    let min = values.iter().map(|(_, p)| *p).fold(0.0, f64::min);
    let max = values.iter().map(|(r, p)| r.max(*p)).fold(1.0, f64::max);
    let scale = 320.0 / (max - min);
    let baseline = 380.0 + min * scale;
    let revenue = values.iter().map(|v| v.0).sum::<f64>();
    let profit = values.iter().map(|v| v.1).sum::<f64>();
    let mut svg=format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 940 440\" style=\"width:100%\"><line x1=\"70\" y1=\"{baseline}\" x2=\"910\" y2=\"{baseline}\" stroke=\"#cbd5e1\"/>");
    for tick in 0..=4 {
        let y = 380.0 - tick as f64 * 80.0;
        let value = min + (max - min) * tick as f64 / 4.0;
        svg.push_str(&format!("<line x1=\"70\" y1=\"{y}\" x2=\"910\" y2=\"{y}\" stroke=\"#e2e8f0\"/><text x=\"2\" y=\"{}\" font-size=\"9\">{}</text>",y+3.0,crate::gui::views::components::number(value)));
    }
    for (i, (r, p)) in values.iter().enumerate() {
        let x = 76.0 + i as f64 * 826.0 / days as f64;
        for (offset, value, color) in [(0.0, *r, "#0d9488"), (10.0, *p, "#64748b")] {
            let h = value.abs() * scale;
            let y = if value >= 0.0 { baseline - h } else { baseline };
            svg.push_str(&format!("<rect x=\"{}\" y=\"{y}\" width=\"8\" height=\"{h}\" fill=\"{color}\"><title>{}: {value:.2}</title></rect>",x+offset,i+1));
        }
        svg.push_str(&format!(
            "<text x=\"{x}\" y=\"410\" font-size=\"10\">{}</text>",
            i + 1
        ));
    }
    svg.push_str("</svg>");
    Ok(page(title,&format!("<p>Bulan {} · Omzet {} · Laba {}</p>{svg}<p><span style=\"color:#0d9488\">■ Omzet</span> · <span style=\"color:#64748b\">■ Laba</span></p>",escape(month),crate::gui::views::components::rupiah(revenue),crate::gui::views::components::rupiah(profit)),"@page{size:A4 landscape}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn receipts_preserve_cents_and_show_credit_balance() {
        let mut sale = crate::db::demo::sales()[0].clone();
        sale.items.truncate(1);
        sale.items[0].qty = 1;
        sale.items[0].qty_in_base = sale.items[0].conversion_qty;
        sale.items[0].sell_price = 9500.55;
        sale.items[0].subtotal = 9500.55;
        sale.subtotal = 9500.55;
        sale.grand_total = 9500.55;
        sale.paid_amount = 10000.10;
        sale.change_amount = 499.55;
        let mut settings =
            crate::hardware::ReceiptSettings::from_values(&crate::db::demo::settings());
        let html = receipt_html(&sale, &settings);
        let raw = crate::hardware::generate_escpos_bytes(&sale, &settings);
        assert!(html.contains("Rp 9.500,55") && html.contains("Rp 499,55"));
        assert!(String::from_utf8_lossy(&raw).contains("Rp 9.500,55"));
        assert!(String::from_utf8_lossy(&raw).contains("Rp 499,55"));
        sale.payment_type = "KREDIT".into();
        sale.paid_amount = 2000.10;
        sale.change_amount = 0.0;
        sale.due_date = Some("2026-11-25".into());
        let html = receipt_html(&sale, &settings);
        let raw = crate::hardware::generate_escpos_bytes(&sale, &settings);
        assert!(html.contains("Sisa: Rp 7.500,45") && html.contains("Jatuh tempo: 2026-11-25"));
        assert!(String::from_utf8_lossy(&raw).contains("Rp 7.500,45"));
        assert!(String::from_utf8_lossy(&raw).contains("Jatuh tempo: 2026-11-25"));
        settings.returned = true;
        assert!(receipt_html(&sale, &settings).contains("Sisa: Rp 0"));
    }
    #[test]
    fn printed_reports_keep_totals_and_escape_content() {
        let rows = vec![vec![
            "1".into(),
            "ABC".into(),
            "Obat <A>".into(),
            "F-1".into(),
            "2026-11-01".into(),
            "500".into(),
            "2".into(),
            "Tablet".into(),
            "900".into(),
            "300".into(),
            "Pasien & B".into(),
        ]];
        let html = grouped_sales_html("Apotek <A>", "2026-11", &rows);
        assert!(html.contains("Obat &lt;A&gt;"));
        assert!(html.contains("Pasien &amp; B"));
        assert!(html.contains("Total Rp 900 · Laba Rp 300"));
        let rows = vec![vec![
            "2026-11-01".into(),
            "1".into(),
            "1000".into(),
            "-200".into(),
        ]];
        let html = monthly_chart_html("Apotek", "2026-11", &rows).unwrap();
        assert!(html.contains("Omzet Rp 1.000 · Laba Rp -200"));
        assert!(!html.contains("height=\"-"));
        assert!(monthly_chart_html("Apotek", "2026-13", &[]).is_err());
    }
}
