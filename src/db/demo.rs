use super::{
    operations::{self, InvoiceDetails, Purchase, PurchaseLine, ReportKind},
    queries::{self, NewProductInput},
};
use crate::models::{Product, ProductBatch, ProductUnit, Sale, SaleItem};
use chrono::{Duration, Local};
use mysql::{params, prelude::*, Pool};
use std::collections::BTreeMap;

const MEDICINES: [(&str, &str, &str, f64, f64, i32, i32, i32); 17] = [
    (
        "Paracetamol 500 mg",
        "Obat bebas",
        "Tablet",
        450.0,
        900.0,
        180,
        20,
        10,
    ),
    (
        "Cetirizine 10 mg",
        "Obat keras",
        "Tablet",
        650.0,
        1500.0,
        120,
        15,
        10,
    ),
    (
        "Vitamin C 100 mg",
        "Vitamin",
        "Tablet",
        350.0,
        1000.0,
        210,
        20,
        10,
    ),
    (
        "Oralit 200 ml",
        "Obat bebas",
        "Sachet",
        1400.0,
        2500.0,
        64,
        10,
        1,
    ),
    (
        "Antasida DOEN",
        "Obat bebas",
        "Tablet",
        450.0,
        1200.0,
        180,
        20,
        10,
    ),
    (
        "OBH Combi 100 ml",
        "Obat bebas",
        "Botol",
        11800.0,
        16500.0,
        30,
        5,
        1,
    ),
    (
        "Betadine 15 ml",
        "Obat bebas",
        "Botol",
        12800.0,
        18000.0,
        22,
        5,
        1,
    ),
    (
        "Minyak kayu putih 60 ml",
        "Obat bebas",
        "Botol",
        18000.0,
        23500.0,
        24,
        5,
        1,
    ),
    (
        "Masker medis",
        "Alat kesehatan",
        "Pcs",
        450.0,
        1000.0,
        200,
        30,
        50,
    ),
    (
        "Amoxicillin 500 mg",
        "Obat keras",
        "Kapsul",
        700.0,
        1400.0,
        120,
        15,
        10,
    ),
    (
        "Amlodipine 5 mg",
        "Obat keras",
        "Tablet",
        550.0,
        1200.0,
        150,
        20,
        10,
    ),
    (
        "Omeprazole 20 mg",
        "Obat keras",
        "Kapsul",
        650.0,
        1500.0,
        85,
        15,
        10,
    ),
    (
        "Termometer digital",
        "Alat kesehatan",
        "Unit",
        18500.0,
        27500.0,
        12,
        3,
        1,
    ),
    (
        "Kasa steril",
        "Alat kesehatan",
        "Pack",
        3200.0,
        5500.0,
        17,
        4,
        1,
    ),
    (
        "Hand sanitizer 100 ml",
        "Alat kesehatan",
        "Botol",
        7500.0,
        12000.0,
        8,
        10,
        1,
    ),
    (
        "Rivanol 100 ml",
        "Obat bebas",
        "Botol",
        3500.0,
        6000.0,
        0,
        5,
        1,
    ),
    (
        "Salep kulit 5 g",
        "Obat bebas",
        "Tube",
        6000.0,
        9500.0,
        4,
        5,
        1,
    ),
];

fn day(ago: i64) -> String {
    (Local::now().date_naive() - Duration::days(ago)).to_string()
}
fn timestamp(ago: i64, hour: i32) -> String {
    format!("{} {hour:02}:15:00", day(ago))
}

pub fn master_rows(table: &str) -> Vec<Vec<String>> {
    let rows: Vec<Vec<&str>> = match table {
        "suppliers" => vec![
            vec!["1", "PBF Mitra Farmasi", "021-555-0101", "Jl. Melati 12"],
            vec!["2", "PBF Sehat Bersama", "021-555-0102", "Jl. Kenanga 8"],
        ],
        "customers" => vec![
            vec!["1", "Rina Pramesti", "0812-0000-0101", "Jl. Anggrek 4"],
            vec!["2", "Ahmad Fauzi", "0812-0000-0102", "Jl. Mawar 9"],
            vec!["3", "Dewi Lestari", "0812-0000-0103", "Jl. Melati 7"],
            vec!["4", "Klinik Melati", "021-555-0103", "Jl. Melati 2"],
        ],
        "doctors" => vec![vec!["1", "dr. Nadya Putri", "SIP-CONTOH-001"]],
        "users" => vec![
            vec!["1", "admin", "Admin apotek", "ADMIN", "1"],
            vec!["2", "kasir", "Petugas kasir", "KASIR", "1"],
        ],
        _ => vec![],
    };
    rows.into_iter()
        .map(|r| r.into_iter().map(str::to_owned).collect())
        .collect()
}

fn catalog() -> Vec<Product> {
    MEDICINES
        .iter()
        .enumerate()
        .map(|(i, (name, category, base, buy, sell, stock, min, pack))| {
            let id = i as i32 + 1;
            let mut units = vec![ProductUnit {
                id: id * 10,
                product_id: id,
                unit_name: (*base).into(),
                conversion_qty: 1,
                sell_price: *sell,
            }];
            if *pack > 1 {
                units.push(ProductUnit {
                    id: id * 10 + 1,
                    product_id: id,
                    unit_name: if *pack == 50 { "Box" } else { "Strip" }.into(),
                    conversion_qty: *pack,
                    sell_price: if *pack == 50 {
                        45000.0
                    } else {
                        sell * 10.0 - 1000.0
                    },
                });
            }
            let expiry = if i == 16 {
                -7
            } else if i == 14 {
                22
            } else {
                180 + i as i64 * 7
            };
            Product {
                id,
                code: format!("CTH-{:03}", i + 1),
                name: (*name).into(),
                category: (*category).into(),
                supplier_id: Some(i as i32 % 2 + 1),
                base_unit: (*base).into(),
                min_stock: *min,
                buy_price: *buy,
                is_active: true,
                total_stock: *stock,
                units,
                batches: vec![ProductBatch {
                    id,
                    product_id: id,
                    batch_no: format!("CTH-AWAL-{:03}", i + 1),
                    expired_date: Some(
                        (Local::now().date_naive() + Duration::days(expiry)).to_string(),
                    ),
                    stock: *stock,
                    buy_price: *buy,
                }],
            }
        })
        .collect()
}

pub fn sales() -> Vec<Sale> {
    let products = catalog();
    let samples: [(i64, i32, bool, f64, &[(usize, i32, usize)]); 9] = [
        (0, 1, false, 50000.0, &[(0, 2, 1), (1, 4, 0)]),
        (0, 2, false, 20000.0, &[(3, 2, 0), (2, 1, 1)]),
        (1, 3, false, 30000.0, &[(5, 1, 0), (8, 5, 0)]),
        (2, 1, true, 10000.0, &[(4, 1, 1), (7, 1, 0)]),
        (3, 2, false, 50000.0, &[(9, 1, 1), (11, 1, 1)]),
        (5, 3, false, 50000.0, &[(12, 1, 0), (8, 10, 0)]),
        (7, 4, true, 0.0, &[(6, 1, 0), (13, 2, 0)]),
        (10, 1, false, 35000.0, &[(10, 2, 1), (2, 1, 1)]),
        (13, 2, false, 20000.0, &[(14, 1, 0), (8, 2, 0)]),
    ];
    samples
        .iter()
        .enumerate()
        .map(|(index, (ago, customer, credit, paid, lines))| {
            let items = lines
                .iter()
                .map(|(pid, qty, unit)| {
                    let p = &products[*pid];
                    let u = &p.units[*unit];
                    SaleItem {
                        id: 0,
                        sale_id: 0,
                        product_id: p.id,
                        product_code: p.code.clone(),
                        product_name: p.name.clone(),
                        unit_name: u.unit_name.clone(),
                        qty: *qty,
                        conversion_qty: u.conversion_qty,
                        qty_in_base: qty * u.conversion_qty,
                        sell_price: u.sell_price,
                        discount: 0.0,
                        subtotal: qty.to_owned() as f64 * u.sell_price,
                        batch_allocations: vec![],
                    }
                })
                .collect::<Vec<_>>();
            let total = items.iter().map(|i| i.subtotal).sum::<f64>();
            Sale {
                id: index as i32 + 1,
                invoice_no: format!("CTH-JL-{:03}", index + 1),
                sale_date: timestamp(*ago, 11 + index as i32 % 5),
                payment_type: if *credit { "KREDIT" } else { "TUNAI" }.into(),
                due_date: if *credit {
                    Some((Local::now().date_naive() + Duration::days(14)).to_string())
                } else {
                    None
                },
                customer_id: Some(*customer),
                doctor_id: None,
                operator_id: 1,
                subtotal: total,
                discount: 0.0,
                grand_total: total,
                paid_amount: *paid,
                change_amount: if *credit { 0.0 } else { paid - total },
                total_profit: items
                    .iter()
                    .map(|i| {
                        i.subtotal
                            - i.qty_in_base as f64 * products[i.product_id as usize - 1].buy_price
                    })
                    .sum(),
                items,
            }
        })
        .collect()
}

pub fn purchases() -> Vec<Purchase> {
    let products = catalog();
    let samples: [(i64, i32, bool, &[(usize, i32, usize)]); 3] = [
        (0, 1, false, &[(0, 10, 1), (2, 10, 1)]),
        (3, 2, true, &[(8, 100, 0), (6, 5, 0)]),
        (7, 1, false, &[(5, 10, 0), (3, 20, 0)]),
    ];
    samples
        .iter()
        .enumerate()
        .map(|(index, (ago, supplier, credit, lines))| Purchase {
            invoice: format!("CTH-PB-{:03}", index + 1),
            purchase_date: day(*ago),
            supplier: *supplier,
            operator: 1,
            credit: *credit,
            due: if *credit {
                Some((Local::now().date_naive() + Duration::days(21)).to_string())
            } else {
                None
            },
            discount: 0.0,
            tax_percent: 0.0,
            lines: lines
                .iter()
                .map(|(pid, qty, unit)| {
                    let p = &products[*pid];
                    let u = &p.units[*unit];
                    PurchaseLine {
                        product_id: p.id,
                        code: p.code.clone(),
                        name: p.name.clone(),
                        unit: u.unit_name.clone(),
                        conversion: u.conversion_qty,
                        qty: *qty,
                        price: p.buy_price * u.conversion_qty as f64,
                        discount: 0.0,
                        batch: format!("CTH-PB-{:03}-{:03}", index + 1, p.id),
                        expiry: Some((Local::now().date_naive() + Duration::days(365)).to_string()),
                    }
                })
                .collect(),
        })
        .collect()
}

pub fn products() -> Vec<Product> {
    let mut products = catalog();
    for purchase in purchases() {
        for line in purchase.lines {
            products[line.product_id as usize - 1].total_stock += line.qty * line.conversion;
        }
    }
    for sale in sales().into_iter().take(8) {
        for item in sale.items {
            products[item.product_id as usize - 1].total_stock -= item.qty_in_base;
        }
    }
    for p in &mut products {
        p.batches[0].stock = p.total_stock;
    }
    products
}

pub fn invoice(sale: bool, id: i32) -> Result<InvoiceDetails, String> {
    let catalog = catalog();
    if sale {
        let mut sale = sales()
            .into_iter()
            .find(|s| s.id == id)
            .ok_or("Faktur contoh tidak ditemukan")?;
        if id == 4 {
            sale.paid_amount += 5000.0;
        }
        let party = master_rows("customers")
            .into_iter()
            .find(|r| r[0] == sale.customer_id.unwrap_or(0).to_string())
            .map(|r| r[1].clone())
            .unwrap_or_else(|| "Pelanggan umum".into());
        let items = sale
            .items
            .iter()
            .map(|i| {
                let p = &catalog[i.product_id as usize - 1];
                vec![
                    i.product_code.clone(),
                    i.product_name.clone(),
                    i.qty.to_string(),
                    i.unit_name.clone(),
                    i.sell_price.to_string(),
                    i.discount.to_string(),
                    i.subtotal.to_string(),
                    format!("{} {}", i.qty_in_base, p.base_unit),
                    p.batches[0].batch_no.clone(),
                ]
            })
            .collect();
        Ok(InvoiceDetails {
            invoice: sale.invoice_no,
            date: chrono::NaiveDateTime::parse_from_str(&sale.sale_date, "%Y-%m-%d %H:%M:%S")
                .unwrap()
                .format("%d/%m/%Y %H:%M")
                .to_string(),
            party,
            payment: sale.payment_type,
            total: sale.grand_total,
            received: sale.paid_amount,
            change: sale.change_amount,
            returned: id == 9,
            items,
        })
    } else {
        let purchase = purchases()
            .into_iter()
            .nth(id.checked_sub(1).ok_or("Faktur contoh tidak ditemukan")? as usize)
            .ok_or("Faktur contoh tidak ditemukan")?;
        let total = purchase.lines.iter().map(PurchaseLine::total).sum();
        let items = purchase
            .lines
            .iter()
            .map(|i| {
                vec![
                    i.code.clone(),
                    i.name.clone(),
                    i.qty.to_string(),
                    i.unit.clone(),
                    i.price.to_string(),
                    "0".into(),
                    i.total().to_string(),
                    format!(
                        "{} {}",
                        i.qty * i.conversion,
                        catalog[i.product_id as usize - 1].base_unit
                    ),
                    i.batch.clone(),
                ]
            })
            .collect();
        Ok(InvoiceDetails {
            invoice: purchase.invoice,
            date: chrono::NaiveDate::parse_from_str(&purchase.purchase_date, "%Y-%m-%d")
                .unwrap()
                .format("%d/%m/%Y")
                .to_string(),
            party: master_rows("suppliers")[purchase.supplier as usize - 1][1].clone(),
            payment: if purchase.credit { "KREDIT" } else { "TUNAI" }.into(),
            total,
            received: if purchase.credit { 20000.0 } else { total },
            change: 0.0,
            returned: false,
            items,
        })
    }
}

pub fn report(kind: ReportKind, from: &str, to: &str) -> Vec<Vec<String>> {
    let sales = sales();
    let purchases = purchases();
    let visible = |date: &str| {
        date.get(..10)
            .map(|d| d >= from && d <= to)
            .unwrap_or(false)
    };
    let mut rows = match kind {
        ReportKind::Sales => sales
            .iter()
            .filter(|s| visible(&s.sale_date))
            .map(|s| {
                let d = invoice(true, s.id).unwrap();
                vec![
                    s.id.to_string(),
                    d.invoice,
                    s.sale_date[..10].into(),
                    d.party,
                    d.total.to_string(),
                    d.received.to_string(),
                    if d.returned {
                        "Diretur"
                    } else if d.payment == "KREDIT" {
                        "Kredit"
                    } else {
                        "Lunas"
                    }
                    .into(),
                ]
            })
            .collect(),
        ReportKind::Purchases => purchases
            .iter()
            .enumerate()
            .filter(|(_, p)| visible(&p.purchase_date))
            .map(|(i, p)| {
                let d = invoice(false, i as i32 + 1).unwrap();
                vec![
                    (i + 1).to_string(),
                    d.invoice,
                    p.purchase_date.clone(),
                    d.party,
                    d.total.to_string(),
                    d.received.to_string(),
                    if p.credit { "Kredit" } else { "Lunas" }.into(),
                ]
            })
            .collect(),
        ReportKind::Receivables => sales
            .iter()
            .filter(|s| s.payment_type == "KREDIT" && visible(&s.sale_date))
            .map(|s| {
                let d = invoice(true, s.id).unwrap();
                vec![
                    s.id.to_string(),
                    d.invoice,
                    d.party,
                    s.due_date.clone().unwrap(),
                    d.total.to_string(),
                    d.received.to_string(),
                    (d.total - d.received).to_string(),
                ]
            })
            .collect(),
        ReportKind::Debts => purchases
            .iter()
            .enumerate()
            .filter(|(_, p)| p.credit && visible(&p.purchase_date))
            .map(|(i, p)| {
                let d = invoice(false, i as i32 + 1).unwrap();
                vec![
                    (i + 1).to_string(),
                    d.invoice,
                    d.party,
                    p.due.clone().unwrap(),
                    d.total.to_string(),
                    d.received.to_string(),
                    (d.total - d.received).to_string(),
                ]
            })
            .collect(),
        ReportKind::Payments => vec![
            vec![
                "1".into(),
                "PENJUALAN".into(),
                "CTH-JL-004".into(),
                day(0),
                "5000".into(),
                "Admin apotek".into(),
            ],
            vec![
                "2".into(),
                "PEMBELIAN".into(),
                "CTH-PB-002".into(),
                day(0),
                "20000".into(),
                "Admin apotek".into(),
            ],
        ],
        ReportKind::Returns => vec![vec![
            "1".into(),
            "PENJUALAN".into(),
            "CTH-JL-009".into(),
            day(0),
            sales[8].grand_total.to_string(),
            sales[8].grand_total.to_string(),
            "Kemasan rusak, retur contoh".into(),
        ]],
        ReportKind::Stock => products()
            .iter()
            .map(|p| {
                vec![
                    p.id.to_string(),
                    p.code.clone(),
                    p.name.clone(),
                    p.base_unit.clone(),
                    p.total_stock.to_string(),
                    p.min_stock.to_string(),
                    (p.total_stock as f64 * p.buy_price).to_string(),
                ]
            })
            .collect(),
        ReportKind::Daily => {
            let mut sums = BTreeMap::<String, (usize, f64, f64)>::new();
            for s in sales.iter().take(8).filter(|s| visible(&s.sale_date)) {
                let v = sums.entry(s.sale_date[..10].into()).or_default();
                v.0 += 1;
                v.1 += s.grand_total;
                v.2 += s.total_profit;
            }
            sums.into_iter()
                .rev()
                .map(|(d, (n, total, profit))| {
                    vec![d, n.to_string(), total.to_string(), profit.to_string()]
                })
                .collect()
        }
        ReportKind::TopProducts => {
            let mut sums = BTreeMap::<i32, (i32, f64)>::new();
            for s in sales.iter().take(8).filter(|s| visible(&s.sale_date)) {
                for i in &s.items {
                    let v = sums.entry(i.product_id).or_default();
                    v.0 += i.qty_in_base;
                    v.1 += i.subtotal;
                }
            }
            let p = products();
            let mut rows = sums
                .into_iter()
                .map(|(id, (qty, total))| {
                    vec![
                        id.to_string(),
                        p[id as usize - 1].code.clone(),
                        p[id as usize - 1].name.clone(),
                        qty.to_string(),
                        total.to_string(),
                    ]
                })
                .collect::<Vec<_>>();
            rows.sort_by(|a, b| {
                b[3].parse::<i32>()
                    .unwrap()
                    .cmp(&a[3].parse::<i32>().unwrap())
            });
            rows
        }
        ReportKind::TopCustomers => {
            let mut sums = BTreeMap::<i32, (usize, f64)>::new();
            for s in sales.iter().take(8).filter(|s| visible(&s.sale_date)) {
                let v = sums.entry(s.customer_id.unwrap()).or_default();
                v.0 += 1;
                v.1 += s.grand_total;
            }
            let c = master_rows("customers");
            let mut rows = sums
                .into_iter()
                .map(|(id, (qty, total))| {
                    vec![
                        id.to_string(),
                        c[id as usize - 1][1].clone(),
                        qty.to_string(),
                        total.to_string(),
                    ]
                })
                .collect::<Vec<_>>();
            rows.sort_by(|a, b| {
                b[3].parse::<f64>()
                    .unwrap()
                    .partial_cmp(&a[3].parse::<f64>().unwrap())
                    .unwrap()
            });
            rows
        }
        ReportKind::StockCards => stock_cards()
            .into_iter()
            .filter(|r| visible(&r[2]))
            .collect(),
    };
    if matches!(kind, ReportKind::Payments | ReportKind::Returns) {
        rows.retain(|r| visible(&r[3]));
    }
    rows
}

fn stock_cards() -> Vec<Vec<String>> {
    let catalog = catalog();
    let mut balances = catalog
        .iter()
        .map(|p| (p.id, p.total_stock))
        .collect::<BTreeMap<_, _>>();
    let mut rows = Vec::new();
    for p in &catalog {
        if p.total_stock > 0 {
            rows.push(vec![
                (rows.len() + 1).to_string(),
                p.name.clone(),
                timestamp(30, 8),
                "OPNAME".into(),
                "STOK-AWAL".into(),
                p.total_stock.to_string(),
                "0".into(),
                p.total_stock.to_string(),
            ]);
        }
    }
    let sales = sales();
    let purchases = purchases();
    let mut events = Vec::new();
    for (i, s) in sales.iter().enumerate() {
        events.push((s.sale_date.clone(), true, i));
    }
    for (i, p) in purchases.iter().enumerate() {
        events.push((format!("{} 09:00:00", p.purchase_date), false, i));
    }
    events.sort_by(|a, b| a.0.cmp(&b.0));
    for (date, is_sale, index) in events {
        if is_sale {
            let s = &sales[index];
            for i in &s.items {
                let balance = balances.get_mut(&i.product_id).unwrap();
                *balance -= i.qty_in_base;
                rows.push(vec![
                    (rows.len() + 1).to_string(),
                    i.product_name.clone(),
                    date.clone(),
                    "PENJUALAN".into(),
                    s.invoice_no.clone(),
                    "0".into(),
                    i.qty_in_base.to_string(),
                    balance.to_string(),
                ]);
            }
        } else {
            let p = &purchases[index];
            for i in &p.lines {
                let balance = balances.get_mut(&i.product_id).unwrap();
                *balance += i.qty * i.conversion;
                rows.push(vec![
                    (rows.len() + 1).to_string(),
                    i.name.clone(),
                    date.clone(),
                    "PEMBELIAN".into(),
                    p.invoice.clone(),
                    (i.qty * i.conversion).to_string(),
                    "0".into(),
                    balance.to_string(),
                ]);
            }
        }
    }
    for i in &sales[8].items {
        let balance = balances.get_mut(&i.product_id).unwrap();
        *balance += i.qty_in_base;
        rows.push(vec![
            (rows.len() + 1).to_string(),
            i.product_name.clone(),
            timestamp(0, 18),
            "RETUR".into(),
            sales[8].invoice_no.clone(),
            i.qty_in_base.to_string(),
            "0".into(),
            balance.to_string(),
        ]);
    }
    rows.reverse();
    rows
}

pub fn settings() -> BTreeMap<String, String> {
    [
        ("store_name", "Apotek Sehat"),
        ("store_address", "Jl. Melati 12"),
        ("store_phone", "021-555-0100"),
        ("receipt_header", "Terima kasih atas kunjungan Anda"),
        ("receipt_footer", "Semoga lekas sembuh"),
        ("receipt_paper_width", "58"),
        ("printer_name", "POS-58"),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into()))
    .collect()
}

pub fn seed(pool: &Pool) -> Result<(), String> {
    let mut conn = pool.get_conn().map_err(|e| e.to_string())?;
    let completed: Option<String> = conn
        .exec_first(
            "SELECT `value` FROM settings WHERE `key`='demo_dataset_v1'",
            (),
        )
        .map_err(|e| e.to_string())?;
    if completed.is_some() {
        return Ok(());
    }
    let operator: Option<i32> = conn
        .query_first("SELECT id FROM users WHERE role='ADMIN' AND is_active=1 ORDER BY id LIMIT 1")
        .map_err(|e| e.to_string())?;
    let operator = operator.ok_or("Akun admin aktif diperlukan untuk data contoh")?;
    let mut suppliers = Vec::new();
    for r in master_rows("suppliers") {
        let id: Option<i32> = conn
            .exec_first(
                "SELECT id FROM suppliers WHERE name=:name LIMIT 1",
                params! {"name"=>&r[1]},
            )
            .map_err(|e| e.to_string())?;
        suppliers.push(match id {
            Some(id) => id,
            None => {
                queries::insert_supplier(pool, &r[1], &r[2], &r[3]).map_err(|e| e.to_string())?
            }
        });
    }
    let mut customers = Vec::new();
    for r in master_rows("customers") {
        let id: Option<i32> = conn
            .exec_first(
                "SELECT id FROM customers WHERE name=:name LIMIT 1",
                params! {"name"=>&r[1]},
            )
            .map_err(|e| e.to_string())?;
        customers.push(match id {
            Some(id) => id,
            None => {
                queries::insert_customer(pool, &r[1], &r[2], &r[3]).map_err(|e| e.to_string())?
            }
        });
    }
    for r in master_rows("doctors") {
        let exists: Option<i32> = conn
            .exec_first(
                "SELECT id FROM doctors WHERE sip_no=:sip",
                params! {"sip"=>&r[2]},
            )
            .map_err(|e| e.to_string())?;
        if exists.is_none() {
            queries::insert_doctor(pool, &r[1], &r[2]).map_err(|e| e.to_string())?;
        }
    }
    let mut product_ids = BTreeMap::<i32, i32>::new();
    for p in catalog() {
        let existing: Option<i32> = conn
            .exec_first(
                "SELECT id FROM products WHERE code=:code",
                params! {"code"=>&p.code},
            )
            .map_err(|e| e.to_string())?;
        let id = if let Some(id) = existing {
            id
        } else {
            let b = &p.batches[0];
            queries::insert_product(
                pool,
                &NewProductInput {
                    code: p.code.clone(),
                    name: p.name,
                    category: p.category,
                    supplier_id: Some(suppliers[p.supplier_id.unwrap() as usize - 1]),
                    base_unit: p.base_unit,
                    min_stock: p.min_stock,
                    buy_price: p.buy_price,
                    units: p
                        .units
                        .iter()
                        .map(|u| (u.unit_name.clone(), u.conversion_qty, u.sell_price))
                        .collect(),
                    batch_no: b.batch_no.clone(),
                    expired_date: b.expired_date.clone(),
                    initial_stock: p.total_stock,
                },
            )
            .map_err(|e| e.to_string())?
        };
        product_ids.insert(p.id, id);
        if existing.is_none() {
            conn.exec_drop("UPDATE stock_cards SET created_at=:date WHERE product_id=:id AND ref_no='STOK-AWAL'",params!{"date"=>timestamp(30,8),"id"=>id}).map_err(|e|e.to_string())?;
        }
    }
    let sales = sales();
    let purchases = purchases();
    let mut events = Vec::new();
    for (i, s) in sales.iter().enumerate() {
        events.push((s.sale_date.clone(), true, i));
    }
    for (i, p) in purchases.iter().enumerate() {
        events.push((format!("{} 09:00:00", p.purchase_date), false, i));
    }
    events.sort_by(|a, b| a.0.cmp(&b.0));
    for (date, is_sale, index) in events {
        let invoice = if is_sale {
            &sales[index].invoice_no
        } else {
            &purchases[index].invoice
        };
        let table = if is_sale { "sales" } else { "purchases" };
        let exists: Option<i32> = conn
            .exec_first(
                format!("SELECT id FROM {table} WHERE invoice_no=:invoice LIMIT 1"),
                params! {"invoice"=>invoice},
            )
            .map_err(|e| e.to_string())?;
        if exists.is_some() {
            continue;
        }
        if is_sale {
            let mut s = sales[index].clone();
            s.operator_id = operator;
            s.customer_id = s.customer_id.map(|id| customers[id as usize - 1]);
            for i in &mut s.items {
                i.product_id = product_ids[&i.product_id];
            }
            queries::execute_sale_transaction_fefo(pool, s).map_err(|e| e.to_string())?;
            conn.exec_drop(
                "UPDATE sales SET sale_date=:date WHERE invoice_no=:inv",
                params! {"date"=>&date,"inv"=>invoice},
            )
            .map_err(|e| e.to_string())?;
        } else {
            let mut p = purchases[index].clone();
            p.operator = operator;
            p.supplier = suppliers[p.supplier as usize - 1];
            for line in &mut p.lines {
                line.product_id = product_ids[&line.product_id];
            }
            operations::save_purchase(pool, &p)?;
        }
        conn.exec_drop(
            "UPDATE stock_cards SET created_at=:date WHERE ref_no=:inv AND ref_type=:kind",
            params! {"date"=>date,"inv"=>invoice,"kind"=>if is_sale{"PENJUALAN"}else{"PEMBELIAN"}},
        )
        .map_err(|e| e.to_string())?;
    }
    for (sale, invoice, target) in [
        (true, "CTH-JL-004", 15000.0),
        (false, "CTH-PB-002", 20000.0),
    ] {
        let table = if sale { "sales" } else { "purchases" };
        let row: Option<(i32, f64)> = conn
            .exec_first(
                format!("SELECT id,paid_amount FROM {table} WHERE invoice_no=:inv"),
                params! {"inv"=>invoice},
            )
            .map_err(|e| e.to_string())?;
        if let Some((id, paid)) = row {
            if paid < target {
                operations::pay(pool, sale, id, target - paid, operator)?;
            }
        }
    }
    let returned: Option<i32> = conn
        .query_first("SELECT id FROM returns WHERE invoice_no='CTH-JL-009'")
        .map_err(|e| e.to_string())?;
    if returned.is_none() {
        let id: i32 = conn
            .query_first("SELECT id FROM sales WHERE invoice_no='CTH-JL-009'")
            .map_err(|e| e.to_string())?
            .ok_or("Faktur retur contoh tidak ditemukan")?;
        operations::return_invoice(pool, true, id, "Kemasan rusak, retur contoh", operator)?;
    }
    let pending_exists: Option<i32> = conn
        .exec_first(
            "SELECT id FROM pending_sales WHERE cart_json LIKE '%CTH-PENDING%' AND operator_id=:op",
            params! {"op"=>operator},
        )
        .map_err(|e| e.to_string())?;
    if pending_exists.is_none() {
        let mut pending = sales[0].clone();
        pending.invoice_no = "CTH-PENDING-001".into();
        pending.operator_id = operator;
        pending.customer_id = Some(customers[0]);
        for i in &mut pending.items {
            i.product_id = product_ids[&i.product_id];
        }
        operations::save_pending(
            pool,
            operator,
            false,
            &serde_json::to_string(&pending).map_err(|e| e.to_string())?,
        )?;
    }
    operations::save_settings(
        pool,
        &[(
            "demo_dataset_v1".into(),
            "17 obat, 9 penjualan, 3 pembelian".into(),
        )],
    )
}
