use crate::models::{Product, ProductBatch, ProductUnit, Sale, User};
use mysql::prelude::*;
use mysql::*;

pub fn authenticate_user(pool: &Pool, username: &str, password: &str) -> Result<Option<User>> {
    let mut conn = pool.get_conn()?;
    let query = "SELECT id, username, full_name, role FROM users WHERE username = :u AND password_hash = :p AND is_active = 1 LIMIT 1";
    let res: Option<(i32, String, String, String)> = conn.exec_first(
        query,
        params! {
            "u" => username,
            "p" => password,
        },
    )?;

    if let Some((id, uname, fname, role)) = res {
        Ok(Some(User {
            id,
            username: uname,
            full_name: fname,
            role,
        }))
    } else {
        Ok(None)
    }
}

pub fn get_products(pool: &Pool, search: Option<&str>) -> Result<Vec<Product>> {
    let mut conn = pool.get_conn()?;
    
    let sql = match search {
        Some(s) if !s.trim().is_empty() => format!(
            "SELECT id, code, name, category, supplier_id, base_unit, min_stock, buy_price, is_active \
             FROM products WHERE is_active = 1 AND (code LIKE '%{}%' OR name LIKE '%{}%') ORDER BY name ASC LIMIT 200",
            s.replace('\'', "''"), s.replace('\'', "''")
        ),
        _ => "SELECT id, code, name, category, supplier_id, base_unit, min_stock, buy_price, is_active \
              FROM products WHERE is_active = 1 ORDER BY name ASC LIMIT 200".to_string(),
    };

    let rows: Vec<(i32, String, String, Option<String>, Option<i32>, String, i32, f64, i8)> = conn.query(sql)?;
    let mut products = Vec::with_capacity(rows.len());

    for (id, code, name, cat, sup_id, base_unit, min_stock, buy_price, is_active) in rows {
        // Ambil units
        let units_sql = format!(
            "SELECT id, unit_name, conversion_qty, sell_price FROM product_units WHERE product_id = {}",
            id
        );
        let u_rows: Vec<(i32, String, i32, f64)> = conn.query(units_sql)?;
        let units: Vec<ProductUnit> = u_rows
            .into_iter()
            .map(|(uid, uname, conv, sp)| ProductUnit {
                id: uid,
                product_id: id,
                unit_name: uname,
                conversion_qty: conv,
                sell_price: sp,
            })
            .collect();

        // Ambil batches & hitung total stock
        let batches_sql = format!(
            "SELECT id, batch_no, DATE_FORMAT(expired_date, '%Y-%m-%d'), stock, buy_price \
             FROM product_batches WHERE product_id = {} AND stock > 0 ORDER BY expired_date ASC",
            id
        );
        let b_rows: Vec<(i32, String, Option<String>, i32, f64)> = conn.query(batches_sql)?;
        let mut total_stock = 0;
        let batches: Vec<ProductBatch> = b_rows
            .into_iter()
            .map(|(bid, bno, exp, st, bp)| {
                total_stock += st;
                ProductBatch {
                    id: bid,
                    product_id: id,
                    batch_no: bno,
                    expired_date: exp,
                    stock: st,
                    buy_price: bp,
                }
            })
            .collect();

        products.push(Product {
            id,
            code,
            name,
            category: cat.unwrap_or_else(|| "Umum".to_string()),
            supplier_id: sup_id,
            base_unit,
            min_stock,
            buy_price,
            is_active: is_active == 1,
            total_stock,
            units,
            batches,
        });
    }

    Ok(products)
}

/// Eksekusi transaksi penjualan dengan FEFO (First Expired First Out)
pub fn execute_sale_transaction_fefo(pool: &Pool, mut sale: Sale) -> Result<String> {
    let mut conn = pool.get_conn()?;
    let mut tx = conn.start_transaction(TxOpts::default())?;

    // 1. Simpan Header Sales
    tx.exec_drop(
        "INSERT INTO sales (invoice_no, payment_type, due_date, customer_id, doctor_id, operator_id, \
         subtotal, discount, grand_total, paid_amount, change_amount, total_profit) \
         VALUES (:inv, :pt, :dd, :cid, :did, :op, :sub, :disc, :gt, :paid, :change, :prof)",
        params! {
            "inv" => &sale.invoice_no,
            "pt" => &sale.payment_type,
            "dd" => &sale.due_date,
            "cid" => sale.customer_id,
            "did" => sale.doctor_id,
            "op" => sale.operator_id,
            "sub" => sale.subtotal,
            "disc" => sale.discount,
            "gt" => sale.grand_total,
            "paid" => sale.paid_amount,
            "change" => sale.change_amount,
            "prof" => sale.total_profit,
        },
    )?;

    let sale_id: i32 = tx.last_insert_id().unwrap_or(0) as i32;
    let mut total_profit = 0.0;

    // 2. Proses tiap item belanja
    for item in &mut sale.items {
        tx.exec_drop(
            "INSERT INTO sale_items (sale_id, product_id, unit_name, qty, conversion_qty, qty_in_base, sell_price, discount, subtotal) \
             VALUES (:sid, :pid, :uname, :q, :conv, :qbase, :sp, :disc, :sub)",
            params! {
                "sid" => sale_id,
                "pid" => item.product_id,
                "uname" => &item.unit_name,
                "q" => item.qty,
                "conv" => item.conversion_qty,
                "qbase" => item.qty_in_base,
                "sp" => item.sell_price,
                "disc" => item.discount,
                "sub" => item.subtotal,
            },
        )?;

        let sale_item_id: i32 = tx.last_insert_id().unwrap_or(0) as i32;
        let mut needed_qty = item.qty_in_base;

        // Ambil batch dengan kriteria FEFO (expired_date ASC) dengan SELECT FOR UPDATE
        let batch_rows: Vec<(i32, String, i32, f64)> = tx.exec(
            "SELECT id, batch_no, stock, buy_price FROM product_batches \
             WHERE product_id = :pid AND stock > 0 ORDER BY expired_date ASC, id ASC FOR UPDATE",
            params! { "pid" => item.product_id },
        )?;

        for (batch_id, _batch_no, stock, buy_price) in batch_rows {
            if needed_qty <= 0 {
                break;
            }

            let deduct = std::cmp::min(needed_qty, stock);
            needed_qty -= deduct;

            // Kurangi stok batch
            tx.exec_drop(
                "UPDATE product_batches SET stock = stock - :deduct WHERE id = :bid",
                params! { "deduct" => deduct, "bid" => batch_id },
            )?;

            // Simpan rincian pemotongan batch
            tx.exec_drop(
                "INSERT INTO sale_item_batches (sale_item_id, batch_id, qty_deducted, buy_price_snapshot) \
                 VALUES (:siid, :bid, :deduct, :bp)",
                params! {
                    "siid" => sale_item_id,
                    "bid" => batch_id,
                    "deduct" => deduct,
                    "bp" => buy_price,
                },
            )?;

            // Hitung profit
            let modal = deduct as f64 * buy_price;
            let jual_proporsi = (deduct as f64 / item.qty_in_base as f64) * item.subtotal;
            total_profit += jual_proporsi - modal;
        }

        // Catat Kartu Stok
        tx.exec_drop(
            "INSERT INTO stock_cards (product_id, ref_type, ref_no, qty_in, qty_out, balance_after) \
             SELECT :pid, 'PENJUALAN', :inv, 0, :qout, COALESCE(SUM(stock), 0) \
             FROM product_batches WHERE product_id = :pid",
            params! {
                "pid" => item.product_id,
                "inv" => &sale.invoice_no,
                "qout" => item.qty_in_base,
            },
        )?;
    }

    // Update total profit di sales
    tx.exec_drop(
        "UPDATE sales SET total_profit = :prof WHERE id = :sid",
        params! { "prof" => total_profit, "sid" => sale_id },
    )?;

    tx.commit()?;
    Ok(sale.invoice_no)
}

/// Inisialisasi otomatis tabel database langsung dari schema.sql (DRY: Single Source of Truth)
pub fn auto_migrate(pool: &Pool) -> Result<()> {
    let mut conn = pool.get_conn()?;
    const SCHEMA_SQL: &str = include_str!("../../sql/schema.sql");
    const SEED_SQL: &str = include_str!("../../sql/seed.sql");

    for sql in [SCHEMA_SQL, SEED_SQL] {
        for stmt in sql.split(';') {
            let clean = stmt
                .lines()
                .filter(|line| !line.trim_start().starts_with("--") && !line.trim_start().starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n");
            let trimmed = clean.trim();
            if !trimmed.is_empty() {
                let _ = conn.query_drop(trimmed);
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct NewProductInput {
    pub code: String,
    pub name: String,
    pub category: String,
    pub supplier_id: Option<i32>,
    pub base_unit: String,
    pub min_stock: i32,
    pub buy_price: f64,
    pub units: Vec<(String, i32, f64)>, // (unit_name, conversion_qty, sell_price)
    pub batch_no: String,
    pub expired_date: Option<String>,
    pub initial_stock: i32,
}

pub fn insert_product(pool: &Pool, p: &NewProductInput) -> Result<i32> {
    let mut conn = pool.get_conn()?;
    let mut tx = conn.start_transaction(TxOpts::default())?;

    tx.exec_drop(
        "INSERT INTO products (code, name, category, supplier_id, base_unit, min_stock, buy_price, is_active) \
         VALUES (:code, :name, :cat, :sup, :unit, :min, :buy, 1)",
        params! {
            "code" => &p.code,
            "name" => &p.name,
            "cat" => &p.category,
            "sup" => p.supplier_id,
            "unit" => &p.base_unit,
            "min" => p.min_stock,
            "buy" => p.buy_price,
        },
    )?;

    let prod_id: i32 = tx.last_insert_id().unwrap_or(0) as i32;

    for (u_name, conv, sell_price) in &p.units {
        if !u_name.trim().is_empty() {
            tx.exec_drop(
                "INSERT INTO product_units (product_id, unit_name, conversion_qty, sell_price) \
                 VALUES (:pid, :uname, :conv, :sell)",
                params! {
                    "pid" => prod_id,
                    "uname" => u_name,
                    "conv" => conv,
                    "sell" => sell_price,
                },
            )?;
        }
    }

    if p.initial_stock > 0 {
        let bno = if p.batch_no.trim().is_empty() { "B001".to_string() } else { p.batch_no.clone() };
        tx.exec_drop(
            "INSERT INTO product_batches (product_id, batch_no, expired_date, stock, buy_price) \
             VALUES (:pid, :bno, :exp, :stock, :buy)",
            params! {
                "pid" => prod_id,
                "bno" => &bno,
                "exp" => &p.expired_date,
                "stock" => p.initial_stock,
                "buy" => p.buy_price,
            },
        )?;

        let batch_id: i32 = tx.last_insert_id().unwrap_or(0) as i32;

        tx.exec_drop(
            "INSERT INTO stock_cards (product_id, batch_id, ref_type, ref_no, qty_in, qty_out, balance_after) \
             VALUES (:pid, :bid, 'OPNAME', 'STOK-AWAL', :qin, 0, :bal)",
            params! {
                "pid" => prod_id,
                "bid" => batch_id,
                "qin" => p.initial_stock,
                "bal" => p.initial_stock,
            },
        )?;
    }

    tx.commit()?;
    Ok(prod_id)
}

pub fn delete_product(pool: &Pool, id: i32) -> Result<()> {
    let mut conn = pool.get_conn()?;
    conn.exec_drop("UPDATE products SET is_active = 0 WHERE id = :id", params! { "id" => id })?;
    Ok(())
}

// Supliers
pub fn get_suppliers(pool: &Pool) -> Result<Vec<(i32, String, String, String)>> {
    let mut conn = pool.get_conn()?;
    let rows: Vec<(i32, String, Option<String>, Option<String>)> = conn.query(
        "SELECT id, name, phone, address FROM suppliers WHERE is_active = 1 ORDER BY name ASC"
    )?;
    Ok(rows.into_iter().map(|(id, name, p, a)| {
        (id, name, p.unwrap_or_else(|| "-".to_string()), a.unwrap_or_else(|| "-".to_string()))
    }).collect())
}

pub fn insert_supplier(pool: &Pool, name: &str, phone: &str, address: &str) -> Result<i32> {
    let mut conn = pool.get_conn()?;
    conn.exec_drop(
        "INSERT INTO suppliers (name, phone, address, is_active) VALUES (:name, :phone, :addr, 1)",
        params! {
            "name" => name,
            "phone" => phone,
            "addr" => address,
        },
    )?;
    Ok(conn.last_insert_id() as i32)
}

// Customers
pub fn get_customers(pool: &Pool) -> Result<Vec<(i32, String, String, String)>> {
    let mut conn = pool.get_conn()?;
    let rows: Vec<(i32, String, Option<String>, Option<String>)> = conn.query(
        "SELECT id, name, phone, address FROM customers ORDER BY name ASC"
    )?;
    Ok(rows.into_iter().map(|(id, name, p, a)| {
        (id, name, p.unwrap_or_else(|| "-".to_string()), a.unwrap_or_else(|| "-".to_string()))
    }).collect())
}

pub fn insert_customer(pool: &Pool, name: &str, phone: &str, address: &str) -> Result<i32> {
    let mut conn = pool.get_conn()?;
    conn.exec_drop(
        "INSERT INTO customers (name, phone, address) VALUES (:name, :phone, :addr)",
        params! {
            "name" => name,
            "phone" => phone,
            "addr" => address,
        },
    )?;
    Ok(conn.last_insert_id() as i32)
}

// Doctors
pub fn get_doctors(pool: &Pool) -> Result<Vec<(i32, String, String)>> {
    let mut conn = pool.get_conn()?;
    let rows: Vec<(i32, String, Option<String>)> = conn.query(
        "SELECT id, name, sip_no FROM doctors ORDER BY name ASC"
    )?;
    Ok(rows.into_iter().map(|(id, name, s)| {
        (id, name, s.unwrap_or_else(|| "-".to_string()))
    }).collect())
}

pub fn insert_doctor(pool: &Pool, name: &str, sip_no: &str) -> Result<i32> {
    let mut conn = pool.get_conn()?;
    conn.exec_drop(
        "INSERT INTO doctors (name, sip_no) VALUES (:name, :sip)",
        params! {
            "name" => name,
            "sip" => sip_no,
        },
    )?;
    Ok(conn.last_insert_id() as i32)
}

// Users
pub fn get_users_list(pool: &Pool) -> Result<Vec<(i32, String, String, String, i8)>> {
    let mut conn = pool.get_conn()?;
    let rows: Vec<(i32, String, String, String, i8)> = conn.query(
        "SELECT id, username, full_name, role, is_active FROM users ORDER BY full_name ASC"
    )?;
    Ok(rows)
}

pub fn insert_user(pool: &Pool, username: &str, password_hash: &str, full_name: &str, role: &str) -> Result<i32> {
    let mut conn = pool.get_conn()?;
    conn.exec_drop(
        "INSERT INTO users (username, password_hash, full_name, role, is_active) \
         VALUES (:u, :p, :fn, :role, 1)",
        params! {
            "u" => username,
            "p" => password_hash,
            "fn" => full_name,
            "role" => role,
        },
    )?;
    Ok(conn.last_insert_id() as i32)
}

// Stock Opname
pub fn get_stock_opname_items(pool: &Pool) -> Result<Vec<(i32, String, String, String, i32)>> {
    let mut conn = pool.get_conn()?;
    let rows: Vec<(i32, String, String, String, i32)> = conn.query(
        "SELECT p.id, p.code, p.name, p.base_unit, COALESCE(SUM(b.stock), 0) AS total_stock \
         FROM products p \
         LEFT JOIN product_batches b ON p.id = b.product_id \
         WHERE p.is_active = 1 \
         GROUP BY p.id, p.code, p.name, p.base_unit \
         ORDER BY p.name ASC"
    )?;
    Ok(rows)
}

pub fn save_stock_opname_adjustment(pool: &Pool, product_id: i32, real_stock: i32, reason: &str) -> Result<()> {
    let mut conn = pool.get_conn()?;
    let mut tx = conn.start_transaction(TxOpts::default())?;

    let cur_stock: i32 = tx.exec_first(
        "SELECT COALESCE(SUM(stock), 0) FROM product_batches WHERE product_id = :pid",
        params! { "pid" => product_id },
    )?.unwrap_or(0);

    let selisih = real_stock - cur_stock;
    if selisih != 0 {
        let latest_batch: Option<(i32, i32)> = tx.exec_first(
            "SELECT id, stock FROM product_batches WHERE product_id = :pid ORDER BY id DESC LIMIT 1",
            params! { "pid" => product_id },
        )?;

        let batch_id = match latest_batch {
            Some((bid, st)) => {
                let new_st = std::cmp::max(0, st + selisih);
                tx.exec_drop("UPDATE product_batches SET stock = :st WHERE id = :bid", params! { "st" => new_st, "bid" => bid })?;
                bid
            }
            None => {
                tx.exec_drop(
                    "INSERT INTO product_batches (product_id, batch_no, expired_date, stock, buy_price) \
                     VALUES (:pid, 'OPNAME', DATE_ADD(CURRENT_DATE, INTERVAL 1 YEAR), :st, 0)",
                    params! { "pid" => product_id, "st" => real_stock },
                )?;
                tx.last_insert_id().unwrap_or(0) as i32
            }
        };

        let (qin, qout) = if selisih > 0 { (selisih, 0) } else { (0, -selisih) };
        tx.exec_drop(
            "INSERT INTO stock_cards (product_id, batch_id, ref_type, ref_no, qty_in, qty_out, balance_after) \
             VALUES (:pid, :bid, 'OPNAME', :note, :qin, :qout, :bal)",
            params! {
                "pid" => product_id,
                "bid" => batch_id,
                "note" => reason,
                "qin" => qin,
                "qout" => qout,
                "bal" => real_stock,
            },
        )?;
    }

    tx.commit()?;
    Ok(())
}

