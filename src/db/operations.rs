use crate::models::Sale;
use chrono::NaiveDate;
use mysql::{params, prelude::*, Pool, TxOpts, Value};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub fn money(text: &str) -> Result<f64, String> {
    let value: f64 = text
        .trim()
        .parse()
        .map_err(|_| "Nominal harus berupa angka".to_string())?;
    if !value.is_finite() || !(0.0..=999_999_999_999.99).contains(&value) {
        return Err("Nominal harus positif dan dalam batas yang didukung".into());
    }
    Ok((value * 100.0).round() / 100.0)
}

pub fn date(text: &str) -> Result<String, String> {
    NaiveDate::parse_from_str(text.trim(), "%Y-%m-%d")
        .map(|d| d.to_string())
        .map_err(|_| "Tanggal harus valid, format YYYY-MM-DD".into())
}

pub fn quantity(text: &str) -> Result<i32, String> {
    let n = text
        .trim()
        .parse::<i32>()
        .map_err(|_| "Jumlah harus bilangan bulat".to_string())?;
    if n <= 0 {
        return Err("Jumlah harus lebih dari nol".into());
    }
    Ok(n)
}

pub fn validate_sale(sale: &Sale) -> Result<(), String> {
    if sale.items.is_empty() || sale.operator_id <= 0 || sale.invoice_no.trim().is_empty() {
        return Err("Faktur, operator, dan barang wajib diisi".into());
    }
    if !sale.sale_date.is_empty() {
        chrono::NaiveDateTime::parse_from_str(&sale.sale_date, "%Y-%m-%d %H:%M:%S")
            .map_err(|_| "Tanggal transaksi tidak valid")?;
    }
    let mut subtotal = 0.0;
    for item in &sale.items {
        if item.product_id <= 0
            || item.qty <= 0
            || item.conversion_qty <= 0
            || item.qty.checked_mul(item.conversion_qty) != Some(item.qty_in_base)
        {
            return Err("Jumlah atau konversi satuan tidak valid".into());
        }
        for v in [item.sell_price, item.discount, item.subtotal] {
            money(&v.to_string())?;
        }
        let expected = item.sell_price * item.qty as f64 - item.discount;
        if expected < 0.0 || (expected - item.subtotal).abs() > 0.011 {
            return Err("Subtotal barang tidak sesuai".into());
        }
        subtotal += item.subtotal;
    }
    for v in [
        sale.subtotal,
        sale.discount,
        sale.shipping,
        sale.grand_total,
        sale.paid_amount,
        sale.change_amount,
    ] {
        money(&v.to_string())?;
    }
    if (subtotal - sale.subtotal).abs() > 0.011
        || sale.discount > subtotal
        || (subtotal - sale.discount + sale.shipping - sale.grand_total).abs() > 0.011
    {
        return Err("Total faktur tidak sesuai".into());
    }
    if sale.payment_type == "TUNAI" && sale.paid_amount < sale.grand_total {
        return Err("Uang bayar belum cukup".into());
    }
    match sale.payment_type.as_str() {
        "TUNAI"
            if sale.paid_amount >= sale.grand_total
                && (sale.change_amount - (sale.paid_amount - sale.grand_total)).abs() < 0.011 =>
        {
            ()
        }
        "KREDIT"
            if sale.customer_id.is_some()
                && sale.change_amount == 0.0
                && sale.paid_amount <= sale.grand_total =>
        {
            date(sale.due_date.as_deref().ok_or("Jatuh tempo wajib diisi")?)?;
        }
        _ => {
            return Err(
                "Pembayaran tidak sesuai. Kredit wajib memiliki pelanggan dan jatuh tempo".into(),
            )
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PurchaseLine {
    pub product_id: i32,
    pub code: String,
    pub name: String,
    pub unit: String,
    pub conversion: i32,
    pub qty: i32,
    pub price: f64,
    pub discount: f64,
    pub batch: String,
    pub expiry: Option<String>,
}

impl PurchaseLine {
    pub fn total(&self) -> f64 {
        (self.price * self.qty as f64 * (1.0 - self.discount / 100.0) * 100.0).round() / 100.0
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.product_id <= 0
            || self.qty <= 0
            || self.conversion <= 0
            || self.qty.checked_mul(self.conversion).is_none()
            || self.batch.trim().is_empty()
        {
            return Err("Barang, jumlah, satuan, dan batch wajib valid".into());
        }
        money(&self.price.to_string())?;
        if !self.discount.is_finite() || !(0.0..=100.0).contains(&self.discount) {
            return Err("Diskon harus 0–100 persen".into());
        }
        money(&self.total().to_string())?;
        if let Some(d) = &self.expiry {
            date(d)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Purchase {
    pub invoice: String,
    pub purchase_date: String,
    pub supplier: i32,
    pub operator: i32,
    pub credit: bool,
    pub due: Option<String>,
    #[serde(default)]
    pub discount: f64,
    #[serde(default)]
    pub tax_percent: f64,
    pub lines: Vec<PurchaseLine>,
}

pub fn save_purchase(pool: &Pool, purchase: &Purchase) -> Result<(), String> {
    if purchase.invoice.trim().is_empty()
        || purchase.supplier <= 0
        || purchase.operator <= 0
        || purchase.lines.is_empty()
    {
        return Err("Faktur, suplier, operator, dan barang wajib diisi".into());
    }
    date(&purchase.purchase_date)?;
    if purchase.credit {
        date(purchase.due.as_deref().ok_or("Jatuh tempo wajib diisi")?)?;
    }
    for line in &purchase.lines {
        line.validate()?;
    }
    let subtotal = purchase.lines.iter().map(PurchaseLine::total).sum::<f64>();
    money(&purchase.discount.to_string())?;
    if purchase.discount > subtotal
        || !purchase.tax_percent.is_finite()
        || !(0.0..=100.0).contains(&purchase.tax_percent)
    {
        return Err("Potongan atau pajak tidak valid".into());
    }
    let total = money(
        &((subtotal - purchase.discount) * (1.0 + purchase.tax_percent / 100.0)).to_string(),
    )?;
    let result = (|| -> mysql::Result<()> {
        let mut conn = pool.get_conn()?;
        let mut tx = conn.start_transaction(TxOpts::default())?;
        lock_operator(&mut tx, purchase.operator)?;
        let duplicate: Option<i32> = tx.exec_first(
            "SELECT id FROM purchases WHERE supplier_id=:sup AND invoice_no=:inv FOR UPDATE",
            params! {"sup"=>purchase.supplier,"inv"=>&purchase.invoice},
        )?;
        if duplicate.is_some() {
            return Err(super::queries::input_error(
                "Faktur suplier sudah tersimpan",
            ));
        }
        tx.exec_drop("INSERT INTO purchases (invoice_no,purchase_date,supplier_id,operator_id,total_amount,payment_type,due_date,paid_amount,discount,tax_percent) VALUES (:inv,:dt,:sup,:op,:total,:pt,:due,:paid,:discount,:tax)",
            params! {"inv"=>&purchase.invoice,"dt"=>&purchase.purchase_date,"sup"=>purchase.supplier,"op"=>purchase.operator,"total"=>total,
                "pt"=>if purchase.credit {"KREDIT"} else {"TUNAI"},"due"=>&purchase.due,"paid"=>if purchase.credit {0.0} else {total},"discount"=>purchase.discount,"tax"=>purchase.tax_percent})?;
        let id = tx.last_insert_id().unwrap_or(0) as i32;
        for line in &purchase.lines {
            let qty = line.qty * line.conversion;
            let base_price = if subtotal > 0.0 {
                line.total() * total / subtotal / qty as f64
            } else {
                0.0
            };
            tx.exec_drop("INSERT INTO product_batches (product_id,batch_no,expired_date,stock,buy_price) VALUES (:pid,:batch,:exp,:qty,:price)",
                params! {"pid"=>line.product_id,"batch"=>&line.batch,"exp"=>&line.expiry,"qty"=>qty,"price"=>base_price})?;
            let batch = tx.last_insert_id().unwrap_or(0) as i32;
            tx.exec_drop("INSERT INTO purchase_items (purchase_id,product_id,batch_no,expired_date,qty,conversion_qty,buy_price,subtotal,batch_id,unit_name) VALUES (:id,:pid,:batch,:exp,:qty,:conv,:price,:total,:bid,:unit)",
                params! {"id"=>id,"pid"=>line.product_id,"batch"=>&line.batch,"exp"=>&line.expiry,"qty"=>line.qty,"conv"=>line.conversion,"price"=>line.price,"total"=>line.total(),"bid"=>batch,"unit"=>&line.unit})?;
            tx.exec_drop(
                "UPDATE products SET buy_price=:price WHERE id=:pid",
                params! {"price"=>base_price,"pid"=>line.product_id},
            )?;
            tx.exec_drop("INSERT INTO stock_cards (product_id,batch_id,ref_type,ref_no,qty_in,qty_out,balance_after) SELECT :pid,:bid,'PEMBELIAN',:inv,:qty,0,COALESCE(SUM(stock),0) FROM product_batches WHERE product_id=:pid",
                params! {"pid"=>line.product_id,"bid"=>batch,"inv"=>&purchase.invoice,"qty"=>qty})?;
        }
        consume_pending(
            &mut tx,
            purchase.operator,
            true,
            &purchase.invoice,
            Some(purchase.supplier),
        )?;
        tx.commit()
    })();
    result.map_err(|e| e.to_string())
}

#[derive(Clone, Copy, PartialEq)]
pub enum ReportKind {
    Sales,
    Purchases,
    Receivables,
    Debts,
    Payments,
    Stock,
    StockCards,
    Returns,
    ProductSales,
    TopProducts,
    TopCustomers,
    Daily,
}

impl ReportKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::Sales => "Penjualan",
            Self::Purchases => "Pembelian",
            Self::Receivables => "Piutang pelanggan",
            Self::Debts => "Hutang suplier",
            Self::Payments => "Riwayat pembayaran",
            Self::Stock => "Persediaan obat",
            Self::StockCards => "Kartu stok",
            Self::Returns => "Retur faktur",
            Self::ProductSales => "Penjualan per barang",
            Self::TopProducts => "Obat terlaris",
            Self::TopCustomers => "Pelanggan teratas",
            Self::Daily => "Ringkasan harian",
        }
    }
    pub fn headers(self) -> Vec<&'static str> {
        match self {
            Self::Sales => vec![
                "ID",
                "Faktur",
                "Tanggal",
                "Pelanggan",
                "Total (Rp)",
                "Diterima (Rp)",
                "Status",
                "Waktu",
                "Tunai / kredit",
                "Jatuh tempo",
                "Dokter",
                "Sales",
                "Laba (Rp)",
                "Operator",
            ],
            Self::Purchases => vec![
                "ID",
                "Faktur",
                "Tanggal",
                "Suplier",
                "Total (Rp)",
                "Dibayar (Rp)",
                "Status",
            ],
            Self::Receivables | Self::Debts => vec![
                "ID",
                "Faktur",
                "Pelanggan / suplier",
                "Jatuh tempo",
                "Total (Rp)",
                "Bayar (Rp)",
                "Sisa (Rp)",
            ],
            Self::Payments => vec![
                "ID",
                "Jenis",
                "Faktur",
                "Tanggal",
                "Nominal (Rp)",
                "Operator",
            ],
            Self::Stock => vec![
                "ID",
                "Kode",
                "Obat",
                "Satuan",
                "Stok",
                "Minimum",
                "Modal (Rp)",
            ],
            Self::StockCards => vec![
                "ID", "Obat", "Tanggal", "Jenis", "Faktur", "Masuk", "Keluar", "Saldo",
            ],
            Self::Returns => vec![
                "ID",
                "Jenis",
                "Faktur",
                "Tanggal",
                "Total (Rp)",
                "Kembali (Rp)",
                "Alasan",
            ],
            Self::ProductSales => vec![
                "ID",
                "Kode",
                "Obat",
                "Faktur",
                "Tanggal",
                "Harga (Rp)",
                "Qty",
                "Satuan",
                "Total (Rp)",
                "Laba (Rp)",
                "Pelanggan",
            ],
            Self::TopProducts => vec!["ID", "Kode", "Obat", "Qty dasar", "Penjualan (Rp)"],
            Self::TopCustomers => vec!["ID", "Pelanggan", "Faktur", "Penjualan (Rp)"],
            Self::Daily => vec!["Tanggal", "Faktur", "Penjualan (Rp)", "Laba kotor (Rp)"],
        }
    }
}

pub fn report(
    pool: &Pool,
    kind: ReportKind,
    from: &str,
    to: &str,
) -> Result<Vec<Vec<String>>, String> {
    report_filtered(pool, kind, from, to, &ReportFilter::default())
}

#[derive(Default, Clone)]
pub struct ReportFilter {
    pub operator: i32,
    pub customer: i32,
    pub doctor: i32,
    pub payment: String,
}

pub fn view_report_filtered(
    pool: Option<&Pool>,
    kind: ReportKind,
    from: &str,
    to: &str,
    filter: &ReportFilter,
) -> Result<Vec<Vec<String>>, String> {
    if let Some(pool) = pool {
        return report_filtered(pool, kind, from, to, filter);
    }
    let mut rows = view_report(None, kind, from, to)?;
    if kind == ReportKind::Sales {
        let sales = super::demo::sales();
        rows.retain(|r| {
            r.first()
                .and_then(|v| v.parse::<i32>().ok())
                .and_then(|id| sales.iter().find(|s| s.id == id))
                .map(|s| {
                    (filter.operator == 0 || s.operator_id == filter.operator)
                        && (filter.customer == 0 || s.customer_id == Some(filter.customer))
                        && (filter.doctor == 0 || s.doctor_id == Some(filter.doctor))
                        && (filter.payment.is_empty() || s.payment_type == filter.payment)
                })
                .unwrap_or(false)
        });
    }
    Ok(rows)
}

pub fn report_filtered(
    pool: &Pool,
    kind: ReportKind,
    from: &str,
    to: &str,
    filter: &ReportFilter,
) -> Result<Vec<Vec<String>>, String> {
    date(from)?;
    date(to)?;
    if from > to {
        return Err("Tanggal awal melebihi tanggal akhir".into());
    }
    let sql = match kind {
        ReportKind::Sales=>"SELECT s.id,s.invoice_no,DATE_FORMAT(s.sale_date,'%Y-%m-%d'),COALESCE(c.name,'Umum'),s.grand_total,s.paid_amount,IF(r.id IS NOT NULL,'Diretur',IF(s.paid_amount>=s.grand_total,'Lunas','Kredit')),DATE_FORMAT(s.sale_date,'%H:%i'),s.payment_type,COALESCE(DATE_FORMAT(s.due_date,'%Y-%m-%d'),''),COALESCE(d.name,''),COALESCE(sp.name,''),s.total_profit,COALESCE(u.full_name,'') FROM sales s LEFT JOIN customers c ON c.id=s.customer_id LEFT JOIN doctors d ON d.id=s.doctor_id LEFT JOIN salespeople sp ON sp.id=s.salesperson_id LEFT JOIN users u ON u.id=s.operator_id LEFT JOIN returns r ON r.ref_type='PENJUALAN' AND r.ref_id=s.id WHERE DATE(s.sale_date) BETWEEN :from AND :to AND (:operator=0 OR s.operator_id=:operator) AND (:customer=0 OR s.customer_id=:customer) AND (:doctor=0 OR s.doctor_id=:doctor) AND (:payment='' OR s.payment_type=:payment) ORDER BY s.id DESC LIMIT 500",
        ReportKind::Purchases=>"SELECT p.id,p.invoice_no,DATE_FORMAT(p.purchase_date,'%Y-%m-%d'),s.name,p.total_amount,p.paid_amount,IF(r.id IS NOT NULL,'Diretur',IF(p.paid_amount>=p.total_amount,'Lunas','Kredit')) FROM purchases p JOIN suppliers s ON s.id=p.supplier_id LEFT JOIN returns r ON r.ref_type='PEMBELIAN' AND r.ref_id=p.id WHERE p.purchase_date BETWEEN :from AND :to ORDER BY p.id DESC LIMIT 500",
        ReportKind::Receivables=>"SELECT s.id,s.invoice_no,COALESCE(c.name,'Umum'),DATE_FORMAT(s.due_date,'%Y-%m-%d'),s.grand_total,s.paid_amount,s.grand_total-s.paid_amount FROM sales s LEFT JOIN customers c ON c.id=s.customer_id WHERE s.payment_type='KREDIT' AND s.paid_amount<s.grand_total AND DATE(s.sale_date) BETWEEN :from AND :to AND NOT EXISTS (SELECT 1 FROM returns r WHERE r.ref_type='PENJUALAN' AND r.ref_id=s.id) ORDER BY s.due_date LIMIT 500",
        ReportKind::Debts=>"SELECT p.id,p.invoice_no,s.name,DATE_FORMAT(p.due_date,'%Y-%m-%d'),p.total_amount,p.paid_amount,p.total_amount-p.paid_amount FROM purchases p JOIN suppliers s ON s.id=p.supplier_id WHERE p.payment_type='KREDIT' AND p.paid_amount<p.total_amount AND p.purchase_date BETWEEN :from AND :to AND NOT EXISTS (SELECT 1 FROM returns r WHERE r.ref_type='PEMBELIAN' AND r.ref_id=p.id) ORDER BY p.due_date LIMIT 500",
        ReportKind::Payments=>"SELECT p.id,p.ref_type,p.invoice_no,DATE_FORMAT(p.created_at,'%Y-%m-%d %H:%i'),p.amount,u.full_name FROM payments p JOIN users u ON u.id=p.operator_id WHERE DATE(p.created_at) BETWEEN :from AND :to ORDER BY p.id DESC LIMIT 500",
        ReportKind::Stock=>"SELECT p.id,p.code,p.name,p.base_unit,COALESCE(SUM(b.stock),0),p.min_stock,COALESCE(SUM(b.stock*b.buy_price),0) FROM products p LEFT JOIN product_batches b ON b.product_id=p.id WHERE p.is_active=1 GROUP BY p.id ORDER BY p.name LIMIT 500",
        ReportKind::StockCards=>"SELECT s.id,p.name,DATE_FORMAT(s.created_at,'%Y-%m-%d %H:%i'),s.ref_type,s.ref_no,s.qty_in,s.qty_out,s.balance_after FROM stock_cards s JOIN products p ON p.id=s.product_id WHERE DATE(s.created_at) BETWEEN :from AND :to ORDER BY s.id DESC LIMIT 500",
        ReportKind::Returns=>"SELECT id,ref_type,invoice_no,DATE_FORMAT(created_at,'%Y-%m-%d %H:%i'),total_amount,refund_amount,reason FROM returns WHERE DATE(created_at) BETWEEN :from AND :to ORDER BY id DESC LIMIT 500",
        ReportKind::ProductSales=>"SELECT p.id,p.code,p.name,s.invoice_no,DATE_FORMAT(s.sale_date,'%Y-%m-%d'),i.sell_price,i.qty,i.unit_name,ROUND(i.subtotal-IF(s.subtotal>0,s.discount*i.subtotal/s.subtotal,0),2),ROUND(i.subtotal-IF(s.subtotal>0,s.discount*i.subtotal/s.subtotal,0)-COALESCE((SELECT SUM(b.qty_deducted*b.buy_price_snapshot) FROM sale_item_batches b WHERE b.sale_item_id=i.id),0),2),COALESCE(c.name,'Umum') FROM sale_items i JOIN products p ON p.id=i.product_id JOIN sales s ON s.id=i.sale_id LEFT JOIN customers c ON c.id=s.customer_id WHERE DATE(s.sale_date) BETWEEN :from AND :to AND NOT EXISTS(SELECT 1 FROM returns r WHERE r.ref_type='PENJUALAN' AND r.ref_id=s.id) ORDER BY p.name,p.id,s.sale_date,i.id LIMIT 10000",
        ReportKind::TopProducts=>"SELECT p.id,p.code,p.name,SUM(i.qty_in_base),SUM(i.subtotal) FROM sale_items i JOIN products p ON p.id=i.product_id JOIN sales s ON s.id=i.sale_id WHERE DATE(s.sale_date) BETWEEN :from AND :to AND NOT EXISTS (SELECT 1 FROM returns r WHERE r.ref_type='PENJUALAN' AND r.ref_id=s.id) GROUP BY p.id ORDER BY SUM(i.qty_in_base) DESC LIMIT 500",
        ReportKind::TopCustomers=>"SELECT COALESCE(c.id,0),COALESCE(c.name,'Umum'),COUNT(*),SUM(s.grand_total) FROM sales s LEFT JOIN customers c ON c.id=s.customer_id WHERE DATE(s.sale_date) BETWEEN :from AND :to AND NOT EXISTS (SELECT 1 FROM returns r WHERE r.ref_type='PENJUALAN' AND r.ref_id=s.id) GROUP BY c.id ORDER BY SUM(s.grand_total) DESC LIMIT 500",
        ReportKind::Daily=>"SELECT DATE_FORMAT(s.sale_date,'%Y-%m-%d'),COUNT(*),SUM(s.grand_total),SUM(s.total_profit) FROM sales s WHERE DATE(s.sale_date) BETWEEN :from AND :to AND NOT EXISTS (SELECT 1 FROM returns r WHERE r.ref_type='PENJUALAN' AND r.ref_id=s.id) GROUP BY DATE(s.sale_date) ORDER BY DATE(s.sale_date) DESC LIMIT 500",
    };
    let mut conn = pool.get_conn().map_err(|e| e.to_string())?;
    let rows: Vec<mysql::Row> = if kind == ReportKind::Stock {
        conn.query(sql)
    } else {
        conn.exec(sql, params! {"from"=>from,"to"=>to,"operator"=>filter.operator,"customer"=>filter.customer,"doctor"=>filter.doctor,"payment"=>&filter.payment})
    }
    .map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|row| {
            row.unwrap()
                .into_iter()
                .map(|v| match v {
                    Value::NULL => "—".into(),
                    Value::Bytes(b) => String::from_utf8_lossy(&b).into_owned(),
                    Value::Int(n) => n.to_string(),
                    Value::UInt(n) => n.to_string(),
                    Value::Float(n) => n.to_string(),
                    Value::Double(n) => n.to_string(),
                    v => v.as_sql(true),
                })
                .collect()
        })
        .collect())
}

#[derive(Clone, Debug)]
pub struct InvoiceDetails {
    pub invoice: String,
    pub date: String,
    pub party: String,
    pub payment: String,
    pub total: f64,
    pub received: f64,
    pub change: f64,
    pub returned: bool,
    pub items: Vec<Vec<String>>,
}

pub fn invoice_details(pool: &Pool, sale: bool, id: i32) -> Result<InvoiceDetails, String> {
    let mut conn = pool.get_conn().map_err(|e| e.to_string())?;
    let header_sql = if sale {
        "SELECT s.invoice_no,DATE_FORMAT(s.sale_date,'%d/%m/%Y %H:%i'),COALESCE(c.name,'Pelanggan umum'),s.payment_type,s.grand_total,s.paid_amount,s.change_amount,EXISTS(SELECT 1 FROM returns r WHERE r.ref_type='PENJUALAN' AND r.ref_id=s.id) FROM sales s LEFT JOIN customers c ON c.id=s.customer_id WHERE s.id=:id"
    } else {
        "SELECT p.invoice_no,DATE_FORMAT(p.purchase_date,'%d/%m/%Y'),s.name,p.payment_type,p.total_amount,p.paid_amount,CAST(0 AS DECIMAL(15,2)),EXISTS(SELECT 1 FROM returns r WHERE r.ref_type='PEMBELIAN' AND r.ref_id=p.id) FROM purchases p JOIN suppliers s ON s.id=p.supplier_id WHERE p.id=:id"
    };
    let row: Option<mysql::Row> = conn
        .exec_first(header_sql, params! {"id"=>id})
        .map_err(|e| e.to_string())?;
    let header: Option<(String, String, String, String, f64, f64, f64, bool)> = row
        .map(mysql::from_row_opt)
        .transpose()
        .map_err(|e| e.to_string())?;
    let (invoice, date, party, payment, total, received, change, returned) =
        header.ok_or("Faktur tidak ditemukan")?;
    let items_sql = if sale {
        "SELECT COALESCE(i.product_code_snapshot,p.code),COALESCE(i.product_name_snapshot,p.name),i.qty,i.unit_name,i.sell_price,i.discount,i.subtotal,CONCAT(i.qty_in_base,' ',p.base_unit),COALESCE((SELECT GROUP_CONCAT(b.batch_no ORDER BY b.id SEPARATOR ', ') FROM sale_item_batches a JOIN product_batches b ON b.id=a.batch_id WHERE a.sale_item_id=i.id),'—') FROM sale_items i JOIN products p ON p.id=i.product_id WHERE i.sale_id=:id ORDER BY i.id"
    } else {
        "SELECT p.code,p.name,i.qty,COALESCE(i.unit_name,IF(i.conversion_qty=1,p.base_unit,CONCAT('×',i.conversion_qty,' ',p.base_unit))),i.buy_price,i.qty*i.buy_price-i.subtotal,i.subtotal,CONCAT(i.qty*i.conversion_qty,' ',p.base_unit),COALESCE(i.batch_no,'—') FROM purchase_items i JOIN products p ON p.id=i.product_id WHERE i.purchase_id=:id ORDER BY i.id"
    };
    let rows: Vec<mysql::Row> = conn
        .exec(items_sql, params! {"id"=>id})
        .map_err(|e| e.to_string())?;
    let items = rows
        .into_iter()
        .map(|r| {
            r.unwrap()
                .into_iter()
                .map(|v| match v {
                    Value::NULL => "—".into(),
                    Value::Bytes(b) => String::from_utf8_lossy(&b).into_owned(),
                    Value::Int(n) => n.to_string(),
                    Value::UInt(n) => n.to_string(),
                    Value::Float(n) => n.to_string(),
                    Value::Double(n) => n.to_string(),
                    v => v.as_sql(true),
                })
                .collect()
        })
        .collect();
    Ok(InvoiceDetails {
        invoice,
        date,
        party,
        payment,
        total,
        received,
        change,
        returned,
        items,
    })
}

pub fn view_invoice(
    pool: Option<&Pool>,
    kind: ReportKind,
    id: i32,
) -> Result<(bool, InvoiceDetails), String> {
    if let Some(pool) = pool {
        let (sale, id) = linked_invoice(pool, kind, id)?;
        return invoice_details(pool, sale, id).map(|details| (sale, details));
    }
    let (sale, id) = match kind {
        ReportKind::Sales | ReportKind::Receivables => (true, id),
        ReportKind::Purchases | ReportKind::Debts => (false, id),
        ReportKind::Payments if id == 1 => (true, 4),
        ReportKind::Payments => (false, 2),
        ReportKind::Returns => (true, 9),
        _ => return Err("Pilih faktur untuk melihat rincian".into()),
    };
    super::demo::invoice(sale, id).map(|d| (sale, d))
}

pub fn view_report(
    pool: Option<&Pool>,
    kind: ReportKind,
    from: &str,
    to: &str,
) -> Result<Vec<Vec<String>>, String> {
    date(from)?;
    date(to)?;
    if from > to {
        return Err("Tanggal awal melebihi tanggal akhir".into());
    }
    if let Some(pool) = pool {
        report(pool, kind, from, to)
    } else {
        Ok(super::demo::report(kind, from, to))
    }
}

pub fn linked_invoice(pool: &Pool, kind: ReportKind, id: i32) -> Result<(bool, i32), String> {
    match kind {
        ReportKind::Sales | ReportKind::Receivables => Ok((true, id)),
        ReportKind::Purchases | ReportKind::Debts => Ok((false, id)),
        ReportKind::Payments | ReportKind::Returns => {
            let table = if kind == ReportKind::Payments {
                "payments"
            } else {
                "returns"
            };
            let row: Option<(String, i32)> = pool
                .get_conn()
                .and_then(|mut c| {
                    c.exec_first(
                        format!("SELECT ref_type,ref_id FROM {table} WHERE id=:id"),
                        params! {"id"=>id},
                    )
                })
                .map_err(|e| e.to_string())?;
            row.map(|(kind, id)| (kind == "PENJUALAN", id))
                .ok_or("Faktur terkait tidak ditemukan".into())
        }
        _ => Err("Pilih daftar transaksi untuk melihat rincian obat".into()),
    }
}

pub fn pay(pool: &Pool, sale: bool, id: i32, amount: f64, operator: i32) -> Result<(), String> {
    money(&amount.to_string())?;
    if amount <= 0.0 {
        return Err("Pembayaran harus lebih dari nol".into());
    }
    let (table, total, kind) = if sale {
        ("sales", "grand_total", "PENJUALAN")
    } else {
        ("purchases", "total_amount", "PEMBELIAN")
    };
    (|| -> mysql::Result<()> {
        let mut conn = pool.get_conn()?;
        let mut tx = conn.start_transaction(TxOpts::default())?;
        let row: Option<(String,f64,f64)> = tx.exec_first(format!("SELECT invoice_no,{total},paid_amount FROM {table} WHERE id=:id FOR UPDATE"),params! {"id"=>id})?;
        let (invoice,total,paid) = row.ok_or_else(|| super::queries::input_error("Faktur tidak ditemukan"))?;
        let returned: Option<i32> = tx.exec_first("SELECT id FROM returns WHERE ref_type=:kind AND ref_id=:id",params! {"kind"=>kind,"id"=>id})?;
        if returned.is_some() || amount > (total-paid).max(0.0)+0.001 { return Err(super::queries::input_error("Nominal melebihi sisa atau faktur sudah diretur")); }
        tx.exec_drop(format!("UPDATE {table} SET paid_amount=paid_amount+:amount WHERE id=:id"),params! {"id"=>id,"amount"=>amount})?;
        tx.exec_drop("INSERT INTO payments (ref_type,ref_id,invoice_no,amount,operator_id) VALUES (:kind,:id,:inv,:amount,:op)",params! {"kind"=>kind,"id"=>id,"inv"=>invoice,"amount"=>amount,"op"=>operator})?;
        tx.commit()
    })().map_err(|e| e.to_string())
}

// ponytail: retur satu faktur utuh; tambah alokasi per item jika retur parsial diperlukan.
pub fn return_invoice(
    pool: &Pool,
    sale: bool,
    id: i32,
    reason: &str,
    operator: i32,
) -> Result<(), String> {
    if reason.trim().is_empty() {
        return Err("Alasan retur wajib diisi".into());
    }
    let (table, total, kind) = if sale {
        ("sales", "grand_total", "PENJUALAN")
    } else {
        ("purchases", "total_amount", "PEMBELIAN")
    };
    (|| -> mysql::Result<()> {
        let mut conn = pool.get_conn()?;
        let mut tx = conn.start_transaction(TxOpts::default())?;
        let row: Option<(String,f64,f64)> = tx.exec_first(format!("SELECT invoice_no,{total},paid_amount FROM {table} WHERE id=:id FOR UPDATE"),params! {"id"=>id})?;
        let (invoice,total,paid) = row.ok_or_else(|| super::queries::input_error("Faktur tidak ditemukan"))?;
        let previous: Option<i32> = tx.exec_first("SELECT id FROM returns WHERE ref_type=:kind AND ref_id=:id",params! {"kind"=>kind,"id"=>id})?;
        if previous.is_some() { return Err(super::queries::input_error("Faktur sudah diretur")); }
        let sql = if sale {"SELECT b.batch_id,i.product_id,b.qty_deducted FROM sale_item_batches b JOIN sale_items i ON i.id=b.sale_item_id WHERE i.sale_id=:id ORDER BY b.batch_id"}
            else {"SELECT batch_id,product_id,qty*conversion_qty FROM purchase_items WHERE purchase_id=:id ORDER BY batch_id"};
        let allocations: Vec<(Option<i32>,i32,i32)> = tx.exec(sql,params! {"id"=>id})?;
        if allocations.is_empty() { return Err(super::queries::input_error("Alokasi batch faktur tidak tersedia")); }
        for (batch,product,qty) in allocations {
            let batch = batch.ok_or_else(|| super::queries::input_error("Faktur lama belum memiliki alokasi batch untuk retur"))?;
            let stock: Option<i32> = tx.exec_first("SELECT stock FROM product_batches WHERE id=:bid FOR UPDATE",params! {"bid"=>batch})?;
            let stock = stock.ok_or_else(|| super::queries::input_error("Batch tidak ditemukan"))?;
            if !sale && stock < qty { return Err(super::queries::input_error("Stok batch telah terpakai. Retur pembelian utuh tidak dapat dilakukan")); }
            let new_stock=if sale {stock.checked_add(qty)}else{stock.checked_sub(qty)}.ok_or_else(||super::queries::input_error("Stok melewati batas yang didukung"))?;
            tx.exec_drop("UPDATE product_batches SET stock=:stock WHERE id=:bid",params! {"stock"=>new_stock,"bid"=>batch})?;
            tx.exec_drop("INSERT INTO stock_cards (product_id,batch_id,ref_type,ref_no,qty_in,qty_out,balance_after) SELECT :pid,:bid,'RETUR',:inv,:qin,:qout,COALESCE(SUM(stock),0) FROM product_batches WHERE product_id=:pid",
                params! {"pid"=>product,"bid"=>batch,"inv"=>&invoice,"qin"=>if sale {qty} else {0},"qout"=>if sale {0} else {qty}})?;
        }
        tx.exec_drop("INSERT INTO returns (ref_type,ref_id,invoice_no,total_amount,refund_amount,reason,operator_id) VALUES (:kind,:id,:inv,:total,:refund,:reason,:op)",
            params! {"kind"=>kind,"id"=>id,"inv"=>invoice,"total"=>total,"refund"=>paid.min(total),"reason"=>reason,"op"=>operator})?;
        tx.commit()
    })().map_err(|e| e.to_string())
}

pub fn settings(pool: &Pool) -> Result<BTreeMap<String, String>, String> {
    let mut conn = pool.get_conn().map_err(|e| e.to_string())?;
    let rows: Vec<(String, String)> = conn
        .query("SELECT `key`,`value` FROM settings")
        .map_err(|e| e.to_string())?;
    Ok(rows.into_iter().collect())
}

pub fn pharmacy_name_from_values(values: &BTreeMap<String, String>) -> String {
    values
        .get("store_name")
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .unwrap_or("Apotek")
        .to_string()
}

pub fn pharmacy_name(pool: Option<&Pool>) -> Result<String, String> {
    let values = if let Some(pool) = pool {
        settings(pool)?
    } else {
        super::demo::settings()
    };
    Ok(pharmacy_name_from_values(&values))
}

pub fn save_settings(pool: &Pool, values: &[(String, String)]) -> Result<(), String> {
    (|| -> mysql::Result<()> {
        let mut conn=pool.get_conn()?;
        let mut tx=conn.start_transaction(TxOpts::default())?;
        for (key,value) in values {
            tx.exec_drop("INSERT INTO settings (`key`,`value`) VALUES (:k,:v) ON DUPLICATE KEY UPDATE `value`=VALUES(`value`)",params! {"k"=>key,"v"=>value})?;
        }
        tx.commit()
    })().map_err(|e|e.to_string())
}

#[derive(Eq, PartialEq, Hash)]
struct PendingIdentity {
    invoice: String,
    supplier: Option<i32>,
}

fn pending_identity(purchase: bool, payload: &str) -> Result<PendingIdentity, String> {
    let value: serde_json::Value = serde_json::from_str(payload).map_err(|e| e.to_string())?;
    let invoice = value[if purchase { "invoice" } else { "invoice_no" }]
        .as_str()
        .ok_or("Nomor faktur pending tidak valid")?
        .to_string();
    if invoice.trim().is_empty() || invoice.chars().count() > 50 {
        return Err("Nomor faktur wajib diisi dan maksimal 50 karakter".into());
    }
    let supplier = if purchase {
        Some(
            value["supplier"]
                .as_i64()
                .and_then(|id| i32::try_from(id).ok())
                .filter(|id| *id > 0)
                .ok_or("Suplier pending tidak valid")?,
        )
    } else {
        None
    };
    Ok(PendingIdentity { invoice, supplier })
}

pub(crate) fn lock_operator(tx: &mut mysql::Transaction<'_>, operator: i32) -> mysql::Result<()> {
    let user: Option<i32> = tx.exec_first(
        "SELECT id FROM users WHERE id=:op FOR UPDATE",
        params! {"op"=>operator},
    )?;
    if user.is_none() {
        return Err(super::queries::input_error("Operator tidak ditemukan"));
    }
    Ok(())
}

fn pending_table(purchase: bool) -> &'static str {
    if purchase {
        "pending_purchases"
    } else {
        "pending_sales"
    }
}

pub fn save_pending(
    pool: &Pool,
    operator: i32,
    purchase: bool,
    payload: &str,
) -> Result<i32, String> {
    let identity = pending_identity(purchase, payload)?;
    if purchase {
        let draft: Purchase = serde_json::from_str(payload).map_err(|e| e.to_string())?;
        if draft.operator != operator || operator <= 0 || draft.lines.is_empty() {
            return Err("Operator atau keranjang pending tidak valid".into());
        }
        date(&draft.purchase_date)?;
        for line in &draft.lines {
            line.validate()?;
        }
        let subtotal = draft.lines.iter().map(PurchaseLine::total).sum::<f64>();
        money(&draft.discount.to_string())?;
        if draft.discount > subtotal
            || !draft.tax_percent.is_finite()
            || !(0.0..=100.0).contains(&draft.tax_percent)
        {
            return Err("Diskon atau pajak pending tidak valid".into());
        }
        money(&((subtotal - draft.discount) * (1.0 + draft.tax_percent / 100.0)).to_string())?;
    } else {
        let mut draft: Sale = serde_json::from_str(payload).map_err(|e| e.to_string())?;
        if draft.operator_id != operator
            || !matches!(draft.payment_type.as_str(), "TUNAI" | "KREDIT")
        {
            return Err("Operator atau pembayaran pending tidak valid".into());
        }
        money(&draft.paid_amount.to_string())?;
        // A draft may wait for payment; validate its items/totals without requiring settlement.
        draft.payment_type = "TUNAI".into();
        draft.paid_amount = draft.grand_total;
        draft.change_amount = 0.0;
        validate_sale(&draft)?;
    }
    let table = pending_table(purchase);
    let result = (|| -> mysql::Result<i32> {
        let mut conn = pool.get_conn()?;
        let mut tx = conn.start_transaction(TxOpts::default())?;
        // Serialize pending and finalization per operator, including legacy drafts without keys.
        lock_operator(&mut tx, operator)?;
        let completed: Option<i32> = if purchase {
            tx.exec_first("SELECT id FROM purchases WHERE supplier_id=:supplier AND invoice_no=:invoice FOR UPDATE", params! {"supplier"=>identity.supplier,"invoice"=>&identity.invoice})?
        } else {
            tx.exec_first(
                "SELECT id FROM sales WHERE invoice_no=:invoice FOR UPDATE",
                params! {"invoice"=>&identity.invoice},
            )?
        };
        if completed.is_some() {
            return Err(super::queries::input_error(
                "Faktur sudah tersimpan; mulai transaksi baru",
            ));
        }
        // ponytail: scan drafts per operator; add an indexed invoice key if large queues become common.
        let rows: Vec<(i32, String)> = tx.exec(format!("SELECT id,cart_json FROM {table} WHERE operator_id=:op ORDER BY id DESC FOR UPDATE"), params! {"op"=>operator})?;
        let existing = rows
            .iter()
            .find(|(_, data)| pending_identity(purchase, data).ok().as_ref() == Some(&identity))
            .map(|r| r.0);
        let id = if let Some(id) = existing {
            tx.exec_drop(
                format!("UPDATE {table} SET cart_json=:cart WHERE id=:id AND operator_id=:op"),
                params! {"id"=>id,"op"=>operator,"cart"=>payload},
            )?;
            id
        } else {
            tx.exec_drop(
                format!("INSERT INTO {table} (operator_id,cart_json) VALUES (:op,:cart)"),
                params! {"op"=>operator,"cart"=>payload},
            )?;
            tx.last_insert_id()
                .and_then(|id| i32::try_from(id).ok())
                .ok_or_else(|| super::queries::input_error("Nomor pending melampaui batas"))?
        };
        tx.commit()?;
        Ok(id)
    })();
    result.map_err(|e| e.to_string())
}

pub fn pending(pool: &Pool, operator: i32, purchase: bool) -> Result<Vec<(i32, String)>, String> {
    let table = pending_table(purchase);
    let rows: Vec<(i32, String)> = pool
        .get_conn()
        .and_then(|mut c| {
            c.exec(
                format!("SELECT id,cart_json FROM {table} WHERE operator_id=:op ORDER BY id DESC"),
                params! {"op"=>operator},
            )
        })
        .map_err(|e| e.to_string())?;
    let mut seen = std::collections::HashSet::new();
    Ok(rows
        .into_iter()
        .filter(|(_, payload)| {
            pending_identity(purchase, payload)
                .map(|identity| seen.insert(identity))
                .unwrap_or(true)
        })
        .take(100)
        .collect())
}

pub(crate) fn consume_pending(
    tx: &mut mysql::Transaction<'_>,
    operator: i32,
    purchase: bool,
    invoice: &str,
    supplier: Option<i32>,
) -> mysql::Result<()> {
    let table = pending_table(purchase);
    let identity = PendingIdentity {
        invoice: invoice.into(),
        supplier,
    };
    let rows: Vec<(i32, String)> = tx.exec(
        format!("SELECT id,cart_json FROM {table} WHERE operator_id=:op FOR UPDATE"),
        params! {"op"=>operator},
    )?;
    for (id, payload) in rows {
        if pending_identity(purchase, &payload).ok().as_ref() == Some(&identity) {
            tx.exec_drop(
                format!("DELETE FROM {table} WHERE id=:id AND operator_id=:op"),
                params! {"id"=>id,"op"=>operator},
            )?;
        }
    }
    Ok(())
}

pub fn backup_database(pool: &Pool, path: &std::path::Path) -> Result<(), String> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    let mut conn = pool.get_conn().map_err(|e| e.to_string())?;
    let mut tx = conn
        .start_transaction(mysql::TxOpts::default().set_with_consistent_snapshot(true))
        .map_err(|e| e.to_string())?;
    writeln!(file,"-- Cadangan Apotek POS. Pulihkan ke database kosong.\nSET NAMES utf8mb4;\nSET SQL_MODE='NO_AUTO_VALUE_ON_ZERO';\nSET FOREIGN_KEY_CHECKS=0;").map_err(|e|e.to_string())?;
    let tables: Vec<String> = tx.query("SHOW TABLES").map_err(|e| e.to_string())?;
    for table in tables {
        let name = table.replace('`', "``");
        let schema: Option<(String, String)> = tx
            .query_first(format!("SHOW CREATE TABLE `{name}`"))
            .map_err(|e| e.to_string())?;
        if let Some((_, create)) = schema {
            writeln!(file, "{create};").map_err(|e| e.to_string())?;
        }
        for row in tx
            .query_iter(format!("SELECT * FROM `{name}`"))
            .map_err(|e| e.to_string())?
        {
            let row = row.map_err(|e| e.to_string())?;
            let data = row
                .unwrap()
                .iter()
                .map(|v| v.as_sql(false))
                .collect::<Vec<_>>()
                .join(",");
            writeln!(file, "INSERT INTO `{name}` VALUES ({data});").map_err(|e| e.to_string())?;
        }
    }
    writeln!(file, "SET FOREIGN_KEY_CHECKS=1;").map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pharmacy_identity_uses_settings_and_matches_receipts() {
        let mut values = BTreeMap::new();
        assert_eq!(pharmacy_name_from_values(&values), "Apotek");
        values.insert("store_name".into(), "  Apotek Tirta Sehat  ".into());
        assert_eq!(pharmacy_name_from_values(&values), "Apotek Tirta Sehat");
        assert_eq!(
            crate::hardware::ReceiptSettings::from_values(&values).store_name,
            "Apotek Tirta Sehat"
        );
        values.insert("store_name".into(), "   ".into());
        assert_eq!(pharmacy_name_from_values(&values), "Apotek");
    }
    #[test]
    fn validate_numeric_and_purchase_boundaries() {
        assert!(money("NaN").is_err());
        assert!(money("inf").is_err());
        assert!(money("-1").is_err());
        assert_eq!(money("10.125").unwrap(), 10.13);
        assert!(quantity("0").is_err());
        assert!(quantity("1.5").is_err());
        assert!(date("2026-02-30").is_err());
        assert_eq!(date("2026-11-22").unwrap(), "2026-11-22");
        let mut line = PurchaseLine {
            product_id: 1,
            code: "A".into(),
            name: "Obat".into(),
            unit: "Strip".into(),
            conversion: 10,
            qty: 2,
            price: 100.0,
            discount: 10.0,
            batch: "B1".into(),
            expiry: None,
        };
        assert_eq!(line.total(), 180.0);
        assert!(line.validate().is_ok());
        line.discount = 101.0;
        assert!(line.validate().is_err());
        line.discount = 0.0;
        line.qty = i32::MAX;
        assert!(line.validate().is_err());
    }

    #[test]
    #[ignore = "Jalankan dengan APOTEK_TEST_SOCKET yang menunjuk MariaDB terisolasi di /tmp"]
    fn database_transaction_workflow() {
        let socket = std::env::var("APOTEK_TEST_SOCKET").expect("APOTEK_TEST_SOCKET wajib diisi");
        assert!(
            socket.starts_with("/tmp/apotek-test-db-"),
            "Tes hanya boleh memakai database terisolasi"
        );
        let name = format!("apotek_test_{}", std::process::id());
        let root_opts = mysql::OptsBuilder::new()
            .user(Some("root"))
            .socket(Some(socket));
        let root = Pool::new(root_opts.clone()).unwrap();
        root.get_conn()
            .unwrap()
            .query_drop(format!("CREATE DATABASE `{name}` CHARACTER SET utf8mb4"))
            .unwrap();
        struct Cleanup {
            pool: Pool,
            name: String,
        }
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = self
                    .pool
                    .get_conn()
                    .and_then(|mut c| c.query_drop(format!("DROP DATABASE `{}`", self.name)));
            }
        }
        let _cleanup = Cleanup {
            pool: root.clone(),
            name: name.clone(),
        };
        let pool = Pool::new(root_opts.db_name(Some(name))).unwrap();
        super::super::queries::auto_migrate(&pool).unwrap();
        let mut conn = pool.get_conn().unwrap();
        let counts = |conn: &mut mysql::PooledConn| -> (u64, u64, u64) {
            (
                conn.query_first("SELECT COUNT(*) FROM suppliers")
                    .unwrap()
                    .unwrap(),
                conn.query_first("SELECT COUNT(*) FROM product_units")
                    .unwrap()
                    .unwrap(),
                conn.query_first("SELECT COUNT(*) FROM product_batches")
                    .unwrap()
                    .unwrap(),
            )
        };
        let legacy_counts = counts(&mut conn);
        conn.query_drop("ALTER TABLE sale_items DROP COLUMN product_code_snapshot,DROP COLUMN product_name_snapshot").unwrap();
        conn.query_drop("ALTER TABLE sales DROP FOREIGN KEY fk_sales_salesperson")
            .unwrap();
        conn.query_drop("ALTER TABLE sales DROP COLUMN salesperson_id,DROP COLUMN shipping")
            .unwrap();
        conn.query_drop("ALTER TABLE products DROP COLUMN purchase_unit")
            .unwrap();
        conn.query_drop("ALTER TABLE suppliers DROP INDEX idx_supplier_code,DROP COLUMN code,DROP COLUMN email,DROP COLUMN bank_account,DROP COLUMN notes").unwrap();
        super::super::queries::auto_migrate(&pool).unwrap();
        assert_eq!(counts(&mut conn), legacy_counts);
        let before = counts(&mut conn);
        super::super::queries::auto_migrate(&pool).unwrap();
        assert_eq!(before, counts(&mut conn));
        assert!(
            super::super::queries::authenticate_user(&pool, "admin", "wrong")
                .unwrap()
                .is_none()
        );
        assert!(
            super::super::queries::authenticate_user(&pool, "admin", "admin123")
                .unwrap()
                .is_some()
        );
        let hash: String = conn
            .query_first("SELECT password_hash FROM users WHERE username='admin'")
            .unwrap()
            .unwrap();
        assert!(hash.starts_with("$2"));
        assert!(
            super::super::queries::authenticate_user(&pool, "admin", "admin123")
                .unwrap()
                .is_some()
        );
        let products = super::super::queries::get_products(&pool, Some("8999909001")).unwrap();
        assert_eq!(products.len(), 1);
        assert!(
            super::super::queries::get_products(&pool, Some("' OR 1=1 --"))
                .unwrap()
                .is_empty()
        );
        let make_sale = |invoice: &str, qty: i32, credit: bool| -> Sale {
            let total = qty as f64 * 9500.0;
            Sale {
                id: 0,
                invoice_no: invoice.into(),
                sale_date: String::new(),
                payment_type: if credit { "KREDIT" } else { "TUNAI" }.into(),
                due_date: if credit {
                    Some(
                        (chrono::Local::now().date_naive() + chrono::Duration::days(30))
                            .to_string(),
                    )
                } else {
                    None
                },
                customer_id: if credit { Some(2) } else { None },
                doctor_id: None,
                salesperson_id: None,
                shipping: 0.0,
                operator_id: 1,
                subtotal: total,
                discount: 0.0,
                grand_total: total,
                paid_amount: if credit { 0.0 } else { total },
                change_amount: 0.0,
                total_profit: 0.0,
                items: vec![crate::models::SaleItem {
                    id: 0,
                    sale_id: 0,
                    product_id: 1,
                    product_code: "8999909001".into(),
                    product_name: "Paracetamol".into(),
                    unit_name: "Strip".into(),
                    qty,
                    conversion_qty: 10,
                    qty_in_base: qty * 10,
                    sell_price: 9500.0,
                    discount: 0.0,
                    subtotal: total,
                    batch_allocations: vec![],
                }],
            }
        };
        super::super::queries::execute_sale_transaction_fefo(
            &pool,
            make_sale("TEST-SALE", 10, false),
        )
        .unwrap();
        let stocks: Vec<i32> = conn
            .query("SELECT stock FROM product_batches WHERE product_id=1 ORDER BY expired_date,id")
            .unwrap();
        assert_eq!(stocks, vec![50, 500]);
        assert!(super::super::queries::execute_sale_transaction_fefo(
            &pool,
            make_sale("TEST-INSUFFICIENT", 1000, false)
        )
        .is_err());
        assert_eq!(
            conn.query_first::<u64, _>(
                "SELECT COUNT(*) FROM sales WHERE invoice_no='TEST-INSUFFICIENT'"
            )
            .unwrap(),
            Some(0)
        );
        assert_eq!(
            conn.query::<i32, _>(
                "SELECT stock FROM product_batches WHERE product_id=1 ORDER BY expired_date,id"
            )
            .unwrap(),
            stocks
        );
        conn.query_drop("UPDATE product_batches SET expired_date=DATE_SUB(CURRENT_DATE,INTERVAL 1 DAY) WHERE product_id=1").unwrap();
        assert!(super::super::queries::execute_sale_transaction_fefo(
            &pool,
            make_sale("TEST-EXPIRED", 1, false)
        )
        .is_err());
        conn.query_drop("UPDATE product_batches SET expired_date=DATE_ADD(CURRENT_DATE,INTERVAL id MONTH) WHERE product_id=1").unwrap();
        super::super::queries::execute_sale_transaction_fefo(
            &pool,
            make_sale("TEST-CREDIT", 2, true),
        )
        .unwrap();
        let sale_id: i32 = conn
            .query_first("SELECT id FROM sales WHERE invoice_no='TEST-CREDIT'")
            .unwrap()
            .unwrap();
        assert!(pay(&pool, true, sale_id, 19001.0, 1).is_err());
        pay(&pool, true, sale_id, 5000.0, 1).unwrap();
        let mut draft = make_sale("TEST-PENDING", 1, false);
        draft.paid_amount = 0.0;
        draft.change_amount = 0.0;
        let pending_sale = serde_json::to_string(&draft).unwrap();
        let before_pending: i64 = conn
            .query_first("SELECT SUM(stock) FROM product_batches WHERE product_id=1")
            .unwrap()
            .unwrap();
        let pending_id = save_pending(&pool, 1, false, &pending_sale).unwrap();
        let workers = (0..6)
            .map(|_| {
                let pool = pool.clone();
                let payload = pending_sale.clone();
                std::thread::spawn(move || save_pending(&pool, 1, false, &payload).unwrap())
            })
            .collect::<Vec<_>>();
        for worker in workers {
            assert_eq!(worker.join().unwrap(), pending_id);
        }
        draft.shipping = 100.0;
        draft.grand_total += 100.0;
        let edited = serde_json::to_string(&draft).unwrap();
        assert_eq!(save_pending(&pool, 1, false, &edited).unwrap(), pending_id);
        assert_eq!(
            pending(&pool, 1, false).unwrap(),
            vec![(pending_id, edited.clone())]
        );
        assert!(save_pending(&pool, 2, false, &edited).is_err());
        let mut empty = draft.clone();
        empty.items.clear();
        assert!(save_pending(&pool, 1, false, &serde_json::to_string(&empty).unwrap()).is_err());
        assert_eq!(
            conn.query_first::<i64, _>("SELECT SUM(stock) FROM product_batches WHERE product_id=1")
                .unwrap(),
            Some(before_pending)
        );
        // Old duplicate records remain recoverable, but the list shows one current draft.
        for _ in 0..2 {
            conn.exec_drop(
                "INSERT INTO pending_sales(operator_id,cart_json) VALUES(1,?)",
                (&edited,),
            )
            .unwrap();
        }
        assert_eq!(pending(&pool, 1, false).unwrap().len(), 1);
        return_invoice(&pool, true, sale_id, "Tes retur", 1).unwrap();
        assert!(return_invoice(&pool, true, sale_id, "Duplikat", 1).is_err());
        assert!(pay(&pool, true, sale_id, 1.0, 1).is_err());
        let purchase = Purchase {
            invoice: "TEST-PURCHASE".into(),
            purchase_date: chrono::Local::now().date_naive().to_string(),
            supplier: 1,
            operator: 1,
            credit: true,
            due: Some((chrono::Local::now().date_naive() + chrono::Duration::days(30)).to_string()),
            discount: 0.0,
            tax_percent: 0.0,
            lines: vec![PurchaseLine {
                product_id: 1,
                code: "8999909001".into(),
                name: "Paracetamol".into(),
                unit: "Tablet".into(),
                conversion: 1,
                qty: 5,
                price: 500.0,
                discount: 0.0,
                batch: "TEST-PURCHASE".into(),
                expiry: None,
            }],
        };
        let purchase_payload = serde_json::to_string(&purchase).unwrap();
        let purchase_pending_id = save_pending(&pool, 1, true, &purchase_payload).unwrap();
        for _ in 0..4 {
            assert_eq!(
                save_pending(&pool, 1, true, &purchase_payload).unwrap(),
                purchase_pending_id
            );
        }
        assert_eq!(pending(&pool, 1, true).unwrap().len(), 1);
        save_purchase(&pool, &purchase).unwrap();
        assert!(save_purchase(&pool, &purchase).is_err());
        assert!(pending(&pool, 1, true).unwrap().is_empty());
        assert!(save_pending(&pool, 1, true, &purchase_payload).is_err());
        let purchase_id: i32 = conn
            .query_first("SELECT id FROM purchases WHERE invoice_no='TEST-PURCHASE'")
            .unwrap()
            .unwrap();
        pay(&pool, false, purchase_id, 500.0, 1).unwrap();
        return_invoice(&pool, false, purchase_id, "Tes retur suplier", 1).unwrap();
        let today = chrono::Local::now().date_naive().to_string();
        for kind in [
            ReportKind::Sales,
            ReportKind::Purchases,
            ReportKind::Receivables,
            ReportKind::Debts,
            ReportKind::Payments,
            ReportKind::Stock,
            ReportKind::StockCards,
            ReportKind::Returns,
            ReportKind::TopProducts,
            ReportKind::TopCustomers,
            ReportKind::Daily,
        ] {
            report(&pool, kind, "2000-01-01", &today).unwrap();
        }
        super::super::queries::save_stock_opname_adjustment(&pool, 1, 1, "Tes opname lintas batch")
            .unwrap();
        assert_eq!(
            conn.query_first::<i64, _>("SELECT SUM(stock) FROM product_batches WHERE product_id=1")
                .unwrap(),
            Some(1)
        );
        assert!(
            super::super::queries::save_stock_opname_adjustment(&pool, 1, -1, "Tes invalid")
                .is_err()
        );
        save_settings(&pool, &[("store_name".into(), "Apotek Uji".into())]).unwrap();
        super::super::queries::auto_migrate(&pool).unwrap();
        assert_eq!(settings(&pool).unwrap()["store_name"], "Apotek Uji");
        assert_eq!(pharmacy_name(Some(&pool)).unwrap(), "Apotek Uji");

        let duplicate = super::super::queries::NewProductInput {
            code: "ATOMIC-IMPORT".into(),
            name: "Obat uji".into(),
            category: "Umum".into(),
            supplier_id: None,
            base_unit: "Tablet".into(),
            purchase_unit: "Tablet".into(),
            min_stock: 0,
            buy_price: 100.0,
            units: vec![("Tablet".into(), 1, 200.0)],
            batch_no: String::new(),
            expired_date: None,
            initial_stock: 0,
        };
        let first_code = super::super::queries::generate_product_code(&pool).unwrap();
        let second_code = super::super::queries::generate_product_code(&pool).unwrap();
        assert_ne!(first_code, second_code);
        assert_eq!(first_code.len(), 20);
        assert!(first_code.bytes().all(|b| b.is_ascii_digit()));
        let mut generated = super::super::queries::NewProductInput {
            code: first_code.clone(),
            name: "A barcode reference 8999909001".into(),
            ..duplicate.clone()
        };
        let generated_id = super::super::queries::insert_product(&pool, &generated).unwrap();
        let found = super::super::queries::get_products(&pool, Some(&first_code)).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].code, first_code);
        assert!(super::super::queries::insert_product(&pool, &generated).is_err());
        // An exact scan must win over a different medicine mentioning that code in its name.
        let matches = super::super::queries::get_products(&pool, Some("8999909001")).unwrap();
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].code, "8999909001");
        generated.code = "8999909001".into();
        assert!(super::super::queries::update_product(&pool, generated_id, &generated).is_err());
        generated.code = second_code.clone();
        super::super::queries::update_product(&pool, generated_id, &generated).unwrap();
        assert!(
            super::super::queries::get_products(&pool, Some(&first_code))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            super::super::queries::get_products(&pool, Some(&second_code)).unwrap()[0].id,
            generated_id
        );
        let unchanged = counts(&mut conn);
        assert!(
            super::super::queries::update_product_barcode(&pool, generated_id, "8999909001")
                .is_err()
        );
        assert!(super::super::queries::update_product_barcode(&pool, generated_id, "").is_err());
        super::super::queries::update_product_barcode(&pool, generated_id, "000123456789").unwrap();
        assert_eq!(counts(&mut conn), unchanged);
        assert_eq!(
            super::super::queries::get_products(&pool, Some("000123456789")).unwrap()[0].id,
            generated_id
        );
        assert!(super::super::queries::import_product_inputs(
            &pool,
            &[duplicate.clone(), duplicate]
        )
        .is_err());
        assert!(
            super::super::queries::get_products(&pool, Some("ATOMIC-IMPORT"))
                .unwrap()
                .is_empty()
        );
        let backup =
            std::env::temp_dir().join(format!("apotek-workflow-backup-{}.sql", std::process::id()));
        backup_database(&pool, &backup).unwrap();
        assert!(backup_database(&pool, &backup).is_err());
        let restored_name = format!("apotek_restore_test_{}", std::process::id());
        root.get_conn()
            .unwrap()
            .query_drop(format!(
                "CREATE DATABASE `{restored_name}` CHARACTER SET utf8mb4"
            ))
            .unwrap();
        let _restore_cleanup = Cleanup {
            pool: root.clone(),
            name: restored_name.clone(),
        };
        let output = std::process::Command::new("mariadb")
            .arg("--no-defaults")
            .arg(format!(
                "--socket={}",
                std::env::var("APOTEK_TEST_SOCKET").unwrap()
            ))
            .arg("--user=root")
            .arg(&restored_name)
            .stdin(std::fs::File::open(&backup).unwrap())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let restored = Pool::new(
            mysql::OptsBuilder::new()
                .user(Some("root"))
                .socket(Some(std::env::var("APOTEK_TEST_SOCKET").unwrap()))
                .db_name(Some(restored_name)),
        )
        .unwrap();
        assert_eq!(settings(&restored).unwrap()["store_name"], "Apotek Uji");
        assert_eq!(pharmacy_name(Some(&restored)).unwrap(), "Apotek Uji");
        let restored_sales: u64 = restored
            .get_conn()
            .unwrap()
            .query_first("SELECT COUNT(*) FROM sales")
            .unwrap()
            .unwrap();
        assert_eq!(restored_sales, 2);
        std::fs::remove_file(backup).unwrap();
        let original =
            super::super::queries::get_products(&pool, Some("8999909001")).unwrap()[0].total_stock;
        let header: Option<i32> = conn
            .query_first("SELECT id FROM sales WHERE invoice_no='TEST-SALE'")
            .unwrap();
        let detail = invoice_details(&pool, true, header.unwrap()).unwrap();
        assert_eq!(detail.items[0][2], "10");
        assert_eq!(detail.items[0][3], "Strip");
        assert_eq!(detail.items[0][7], "100 Tablet");
        assert!(detail.items[0][8].contains("BATCH-PCT-01"));
        let purchase_detail = invoice_details(&pool, false, purchase_id).unwrap();
        assert_eq!(purchase_detail.items[0][3], "Tablet");
        super::super::demo::seed(&pool).unwrap();
        assert_eq!(
            super::super::queries::get_products(&pool, Some("8999909001")).unwrap()[0].total_stock,
            original
        );
        for expected in super::super::demo::products() {
            let actual = super::super::queries::get_products(&pool, Some(&expected.code)).unwrap();
            assert_eq!(actual.len(), 1);
            assert_eq!(
                actual[0].total_stock, expected.total_stock,
                "Stok {}",
                expected.code
            );
        }
        assert_eq!(
            conn.query_first::<u64, _>("SELECT COUNT(*) FROM sales WHERE invoice_no LIKE 'CTH-%'")
                .unwrap(),
            Some(9)
        );
        assert_eq!(
            conn.query_first::<u64, _>(
                "SELECT COUNT(*) FROM purchases WHERE invoice_no LIKE 'CTH-%'"
            )
            .unwrap(),
            Some(3)
        );
        super::super::demo::seed(&pool).unwrap();
        assert_eq!(
            conn.query_first::<u64, _>("SELECT COUNT(*) FROM sales WHERE invoice_no LIKE 'CTH-%'")
                .unwrap(),
            Some(9)
        );
        assert_eq!(settings(&pool).unwrap()["store_name"], "Apotek Uji");
        // Reference workflow: purchase units, discounts/fees, actors, archived receipts and filters.
        let item = super::super::queries::NewProductInput {
            code: "IB-PROD".into(),
            name: "Obat referensi IB".into(),
            category: "IB".into(),
            supplier_id: None,
            base_unit: "Tablet".into(),
            purchase_unit: "Strip".into(),
            min_stock: 10,
            buy_price: 100.0,
            units: vec![("Tablet".into(), 1, 500.0), ("Strip".into(), 10, 4500.0)],
            batch_no: "IB-BATCH".into(),
            expired_date: Some(
                (chrono::Local::now().date_naive() + chrono::Duration::days(365)).to_string(),
            ),
            initial_stock: 50,
        };
        let id = super::super::queries::insert_product(&pool, &item).unwrap();
        conn.query_drop("INSERT INTO salespeople(name) VALUES('Sales IB')")
            .unwrap();
        let actor = conn.last_insert_id() as i32;
        conn.query_drop("INSERT INTO doctors(name,sip_no) VALUES('Dokter IB','IB-SIP')")
            .unwrap();
        let doctor = conn.last_insert_id() as i32;
        conn.query_drop("INSERT INTO customers(name) VALUES('Pasien IB')")
            .unwrap();
        let customer = conn.last_insert_id() as i32;
        let mut sale = make_sale("IB-SALE", 1, false);
        sale.sale_date = format!(
            "{} 09:15:00",
            chrono::Local::now().date_naive() - chrono::Duration::days(1)
        );
        sale.customer_id = Some(customer);
        sale.doctor_id = Some(doctor);
        sale.salesperson_id = Some(actor);
        sale.subtotal = 1500.0;
        sale.discount = 500.0;
        sale.shipping = 1000.0;
        sale.grand_total = 2000.0;
        sale.paid_amount = 2000.0;
        sale.items[0].product_id = id;
        sale.items[0].product_code = "IB-PROD".into();
        sale.items[0].product_name = item.name.clone();
        sale.items[0].unit_name = "Tablet".into();
        sale.items[0].qty = 3;
        sale.items[0].conversion_qty = 1;
        sale.items[0].qty_in_base = 3;
        sale.items[0].sell_price = 500.0;
        sale.items[0].subtotal = 1500.0;
        super::super::queries::execute_sale_transaction_fefo(&pool, sale.clone()).unwrap();
        let sale_id: i32 = conn
            .query_first("SELECT id FROM sales WHERE invoice_no='IB-SALE'")
            .unwrap()
            .unwrap();
        let archived = super::super::queries::get_sale(&pool, sale_id).unwrap();
        assert_eq!(archived.sale_date, sale.sale_date);
        assert_eq!(archived.shipping, 1000.0);
        assert_eq!(archived.salesperson_id, Some(actor));
        assert_eq!(archived.total_profit, 1700.0);
        let original = archived.items[0].clone();
        conn.exec_drop(
            "UPDATE products SET name='Nama baru uji',code='IB-RENAMED' WHERE id=?",
            (id,),
        )
        .unwrap();
        let reprint = super::super::queries::get_sale(&pool, sale_id).unwrap();
        assert_eq!(reprint.items[0].product_name, original.product_name);
        assert_eq!(reprint.items[0].product_code, original.product_code);
        let detail = invoice_details(&pool, true, sale_id).unwrap();
        assert_eq!(detail.items[0][0], original.product_code);
        assert_eq!(detail.items[0][1], original.product_name);
        conn.exec_drop(
            "UPDATE products SET name=?,code=? WHERE id=?",
            (&original.product_name, &original.product_code, id),
        )
        .unwrap();

        let actual = super::super::queries::get_products(&pool, Some("IB-PROD")).unwrap();
        assert_eq!(actual[0].total_stock, 47);
        assert_eq!(actual[0].purchase_unit, "Strip");
        let rows = report_filtered(
            &pool,
            ReportKind::Sales,
            "2000-01-01",
            &today,
            &ReportFilter {
                operator: 1,
                customer,
                doctor,
                payment: "TUNAI".into(),
            },
        )
        .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][1], "IB-SALE");
        assert_eq!(rows[0][10], "Dokter IB");
        assert_eq!(rows[0][11], "Sales IB");
        let goods = report(&pool, ReportKind::ProductSales, "2000-01-01", &today).unwrap();
        let goods = goods.iter().find(|r| r[1] == "IB-PROD").unwrap();
        assert_eq!(goods[8].parse::<f64>().unwrap(), 1000.0);
        assert_eq!(goods[9].parse::<f64>().unwrap(), 700.0);
        let filtered = super::super::queries::product_page(
            Some(&pool),
            &super::super::queries::ProductFilter {
                category: "IB".into(),
                purchase_unit: "Strip".into(),
                stock: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(filtered.1, 1);
        assert_eq!(filtered.0[0].id, id);
        let mut inputs = Vec::new();
        for i in 0..205 {
            let mut input = item.clone();
            input.code = format!("PAGE-IB-{i}");
            input.name = format!("Page IB {i:03}");
            input.category = "Page IB".into();
            input.initial_stock = 0;
            inputs.push(input);
        }
        let mut progress = Vec::new();
        super::super::queries::import_product_inputs_progress(&pool, &inputs, |n| progress.push(n))
            .unwrap();
        assert_eq!(progress.first(), Some(&1));
        assert_eq!(progress.last(), Some(&205));
        let mut ids = std::collections::BTreeSet::new();
        for (page, expected) in [(0, 100), (1, 100), (2, 5)] {
            let (rows, count) = super::super::queries::product_page(
                Some(&pool),
                &super::super::queries::ProductFilter {
                    category: "Page IB".into(),
                    page,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(count, 205);
            assert_eq!(rows.len(), expected);
            ids.extend(rows.iter().map(|p| p.id));
        }
        assert_eq!(ids.len(), 205);
        let selected = ids.iter().take(2).copied().collect::<Vec<_>>();
        assert!(super::super::queries::delete_products(&pool, &[0, selected[0]]).is_err());
        assert_eq!(
            super::super::queries::product_page(
                Some(&pool),
                &super::super::queries::ProductFilter {
                    category: "Page IB".into(),
                    ..Default::default()
                }
            )
            .unwrap()
            .1,
            205
        );
        super::super::queries::delete_products(&pool, &selected).unwrap();
        assert_eq!(
            super::super::queries::product_page(
                Some(&pool),
                &super::super::queries::ProductFilter {
                    category: "Page IB".into(),
                    ..Default::default()
                }
            )
            .unwrap()
            .1,
            203
        );

        let supplier = vec![
            "001234".into(),
            "PT IB".into(),
            "081234".into(),
            "Alamat".into(),
            "ib@example.com".into(),
            "000123456".into(),
            "Catatan IB".into(),
        ];
        super::super::queries::import_suppliers(&pool, &[supplier.clone()]).unwrap();
        let suppliers: u64 = conn
            .query_first("SELECT COUNT(*) FROM suppliers")
            .unwrap()
            .unwrap();
        assert!(
            super::super::queries::import_suppliers(&pool, &[supplier.clone(), supplier]).is_err()
        );
        assert_eq!(
            conn.query_first::<u64, _>("SELECT COUNT(*) FROM suppliers")
                .unwrap(),
            Some(suppliers)
        );
        let mut legacy = serde_json::to_value(super::super::demo::sales()[0].clone()).unwrap();
        legacy.as_object_mut().unwrap().remove("shipping");
        legacy.as_object_mut().unwrap().remove("salesperson_id");
        let legacy: Sale = serde_json::from_value(legacy).unwrap();
        assert_eq!(legacy.shipping, 0.0);
        assert_eq!(legacy.salesperson_id, None);

        sale.invoice_no = "FLOW-F4".into();
        let payload = serde_json::to_string(&sale).unwrap();
        save_pending(&pool, 1, false, &payload).unwrap();
        for _ in 0..2 {
            conn.exec_drop(
                "INSERT INTO pending_sales(operator_id,cart_json) VALUES(1,?)",
                (&payload,),
            )
            .unwrap();
        }
        let before: i64 = conn
            .exec_first(
                "SELECT SUM(stock) FROM product_batches WHERE product_id=?",
                (id,),
            )
            .unwrap()
            .unwrap();
        let events = std::cell::RefCell::new(Vec::new());
        let result = crate::services::cart::finish_sale(
            Some(sale),
            None,
            true,
            |sale| {
                events.borrow_mut().push("save");
                super::super::queries::save_sale_fefo(&pool, sale.clone())
                    .map_err(|e| e.to_string())
            },
            |_| {
                events.borrow_mut().push("print");
                Err("Printer offline".into())
            },
        )
        .unwrap();
        assert_eq!(*events.borrow(), vec!["save", "print"]);
        assert!(result.saved_now && result.print_error.is_some());
        assert_eq!(
            conn.query_first::<u64, _>("SELECT COUNT(*) FROM sales WHERE invoice_no='FLOW-F4'")
                .unwrap(),
            Some(1)
        );
        assert_eq!(
            conn.exec_first::<i64, _, _>(
                "SELECT SUM(stock) FROM product_batches WHERE product_id=?",
                (id,)
            )
            .unwrap(),
            Some(before - 3)
        );
        assert_eq!(
            conn.query_first::<u64, _>(
                "SELECT COUNT(*) FROM pending_sales WHERE cart_json LIKE '%FLOW-F4%'"
            )
            .unwrap(),
            Some(0)
        );
        assert!(save_pending(&pool, 1, false, &payload).is_err());
        let retry = crate::services::cart::finish_sale(
            None,
            Some(result.sale),
            true,
            |_| panic!("A print retry must not save another invoice"),
            |_| Ok(()),
        )
        .unwrap();
        assert!(!retry.saved_now && retry.print_error.is_none());
        assert_eq!(
            conn.query_first::<u64, _>("SELECT COUNT(*) FROM sales WHERE invoice_no='FLOW-F4'")
                .unwrap(),
            Some(1)
        );
        assert_eq!(
            conn.exec_first::<i64, _, _>(
                "SELECT SUM(stock) FROM product_batches WHERE product_id=?",
                (id,)
            )
            .unwrap(),
            Some(before - 3)
        );

        // Two cashiers may see the same stock; the database must prevent overselling.
        let stock_id = super::super::queries::insert_product(
            &pool,
            &super::super::queries::NewProductInput {
                code: "POS-RACE".into(),
                name: "Obat uji bersamaan".into(),
                category: "Uji POS".into(),
                supplier_id: None,
                base_unit: "Tablet".into(),
                purchase_unit: "Tablet".into(),
                min_stock: 1,
                buy_price: 100.0,
                units: vec![("Tablet".into(), 1, 200.0)],
                batch_no: "POS-RACE".into(),
                expired_date: None,
                initial_stock: 5,
            },
        )
        .unwrap();
        conn.exec_drop(
            "INSERT INTO users(username,password_hash,full_name,role) VALUES(?,?,?,?)",
            ("pos-race", "unused", "Kasir uji", "KASIR"),
        )
        .unwrap();
        let other_operator = conn.last_insert_id() as i32;
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let workers = [1, other_operator]
            .into_iter()
            .enumerate()
            .map(|(n, operator)| {
                let pool = pool.clone();
                let barrier = barrier.clone();
                let mut sale = super::super::demo::sales()[0].clone();
                sale.invoice_no = format!("POS-RACE-{n}");
                sale.operator_id = operator;
                sale.customer_id = None;
                sale.doctor_id = None;
                sale.salesperson_id = None;
                sale.subtotal = 800.0;
                sale.discount = 0.0;
                sale.shipping = 0.0;
                sale.grand_total = 800.0;
                sale.paid_amount = 800.0;
                sale.change_amount = 0.0;
                sale.items.truncate(1);
                let line = &mut sale.items[0];
                line.product_id = stock_id;
                line.qty = 4;
                line.unit_name = "Tablet".into();
                line.conversion_qty = 1;
                line.qty_in_base = 4;
                line.sell_price = 200.0;
                line.discount = 0.0;
                line.subtotal = 800.0;
                std::thread::spawn(move || {
                    barrier.wait();
                    super::super::queries::execute_sale_transaction_fefo(&pool, sale)
                })
            })
            .collect::<Vec<_>>();
        let results = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            conn.exec_first::<i64, _, _>(
                "SELECT SUM(stock) FROM product_batches WHERE product_id=?",
                (stock_id,)
            )
            .unwrap(),
            Some(1)
        );
        assert_eq!(
            conn.query_first::<u64, _>(
                "SELECT COUNT(*) FROM sales WHERE invoice_no LIKE 'POS-RACE-%'"
            )
            .unwrap(),
            Some(1)
        );
    }
}
