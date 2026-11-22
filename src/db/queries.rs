use crate::models::{Product, ProductBatch, ProductUnit, Sale, User};
use mysql::prelude::*;
use mysql::*;

pub fn input_error(message: &str) -> Error {
    Error::IoError(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        message,
    ))
}

pub fn authenticate_user(pool: &Pool, username: &str, password: &str) -> Result<Option<User>> {
    let mut conn = pool.get_conn()?;
    let query = "SELECT id, username, full_name, role, password_hash FROM users WHERE username = :u AND is_active = 1 LIMIT 1";
    let res: Option<(i32, String, String, String, String)> = conn.exec_first(
        query,
        params! {
            "u" => username,
        },
    )?;

    if let Some((id, uname, fname, role, hash)) = res {
        let legacy = !hash.starts_with("$2");
        let valid = if legacy {
            hash == password
        } else {
            password.len() <= 72 && bcrypt::verify(password, &hash).unwrap_or(false)
        };
        if !valid {
            return Ok(None);
        }
        if legacy && password.len() <= 72 {
            let hash = bcrypt::hash(password, 12).map_err(|e| input_error(&e.to_string()))?;
            conn.exec_drop(
                "UPDATE users SET password_hash=:hash WHERE id=:id",
                params! {"hash"=>hash,"id"=>id},
            )?;
        }
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

    let pattern = format!("%{}%", search.unwrap_or("").trim());
    let rows: Vec<(i32, String, String, Option<String>, Option<i32>, String, i32, f64, i8)> = conn.exec(
        "SELECT id,code,name,category,supplier_id,base_unit,min_stock,buy_price,is_active FROM products WHERE is_active=1 AND (code LIKE :search OR name LIKE :search) ORDER BY (code=:exact) DESC,name LIMIT 200",
        params!{"search"=>pattern,"exact"=>search.unwrap_or("").trim()})?;
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

pub fn view_products(
    pool: Option<&Pool>,
    search: Option<&str>,
) -> std::result::Result<Vec<Product>, String> {
    if let Some(pool) = pool {
        return get_products(pool, search).map_err(|e| e.to_string());
    }
    let query = search.unwrap_or("").trim().to_lowercase();
    let mut products = super::demo::products()
        .into_iter()
        .filter(|p| {
            p.name.to_lowercase().contains(&query) || p.code.to_lowercase().contains(&query)
        })
        .collect::<Vec<_>>();
    products.sort_by_key(|p| p.code.to_lowercase() != query);
    Ok(products)
}

/// Generate on the shared database server so LAN clients use the same source.
pub fn generate_product_code(pool: &Pool) -> Result<String> {
    let mut conn = pool.get_conn()?;
    for _ in 0..3 {
        let value: u64 = conn
            .query_first("SELECT UUID_SHORT()")?
            .ok_or_else(|| input_error("Tidak dapat membuat barcode"))?;
        let code = format!("{value:020}");
        let existing: Option<i32> = conn.exec_first(
            "SELECT id FROM products WHERE code=:code",
            params! {"code"=>&code},
        )?;
        if existing.is_none() {
            return Ok(code);
        }
    }
    Err(input_error("Barcode sudah digunakan. Coba buat lagi."))
}

pub fn update_product_barcode(pool: &Pool, id: i32, code: &str) -> Result<()> {
    let code = code.trim();
    crate::services::barcode::Barcode::new(code).map_err(|e| input_error(&e))?;
    let mut conn = pool.get_conn()?;
    let mut tx = conn.start_transaction(TxOpts::default())?;
    let existing: Option<i32> = tx.exec_first(
        "SELECT id FROM products WHERE id=:id AND is_active=1 FOR UPDATE",
        params! {"id"=>id},
    )?;
    if existing.is_none() {
        return Err(input_error("Obat tidak ditemukan atau sudah nonaktif"));
    }
    tx.exec_drop(
        "UPDATE products SET code=:code WHERE id=:id",
        params! {"code"=>code,"id"=>id},
    )
    .map_err(|e| match e {
        Error::MySqlError(ref error) if error.code == 1062 => {
            input_error("Barcode sudah digunakan obat lain")
        }
        _ => e,
    })?;
    tx.commit()
}

/// Eksekusi transaksi penjualan dengan FEFO (First Expired First Out)
pub fn execute_sale_transaction_fefo(pool: &Pool, mut sale: Sale) -> Result<String> {
    super::operations::validate_sale(&sale).map_err(|e| input_error(&e))?;
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
        let product: Option<i32> = tx.exec_first(
            "SELECT id FROM products WHERE id=:pid AND is_active=1 FOR UPDATE",
            params! {"pid"=>item.product_id},
        )?;
        if product.is_none() {
            return Err(input_error("Obat tidak aktif atau tidak ditemukan"));
        }
        let unit:Option<i32>=tx.exec_first("SELECT conversion_qty FROM product_units WHERE product_id=:pid AND unit_name=:unit LIMIT 1",params!{"pid"=>item.product_id,"unit"=>&item.unit_name})?;
        if unit != Some(item.conversion_qty) {
            return Err(input_error(
                "Satuan atau konversi telah berubah. Tambahkan ulang obat.",
            ));
        }
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
             WHERE product_id = :pid AND stock > 0 AND (expired_date IS NULL OR expired_date > CURRENT_DATE) \
             ORDER BY expired_date IS NULL, expired_date ASC, id ASC FOR UPDATE",
            params! { "pid" => item.product_id },
        )?;

        for (batch_id, _batch_no, stock, buy_price) in batch_rows {
            if needed_qty <= 0 {
                break;
            }

            let deduct = std::cmp::min(needed_qty, stock);
            let effective_price = item.subtotal / item.qty_in_base as f64;
            if effective_price + 0.001 < buy_price {
                return Err(input_error("Harga setelah diskon di bawah modal batch"));
            }
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

        if needed_qty > 0 {
            return Err(input_error(&format!(
                "Stok layak jual '{}' tidak cukup. Transaksi dibatalkan",
                item.product_name
            )));
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
        params! { "prof" => total_profit - sale.discount, "sid" => sale_id },
    )?;

    tx.commit()?;
    Ok(sale.invoice_no)
}

/// Inisialisasi otomatis tabel database langsung dari schema.sql (DRY: Single Source of Truth)
pub fn auto_migrate(pool: &Pool) -> Result<()> {
    let mut conn = pool.get_conn()?;
    const SCHEMA_SQL: &str = include_str!("../../sql/schema.sql");
    const SEED_SQL: &str = include_str!("../../sql/seed.sql");

    for sql in [SCHEMA_SQL] {
        for stmt in sql.split(';') {
            let clean = stmt
                .lines()
                .filter(|line| {
                    !line.trim_start().starts_with("--") && !line.trim_start().starts_with('#')
                })
                .collect::<Vec<_>>()
                .join("\n");
            let trimmed = clean.trim();
            if !trimmed.is_empty()
                && !trimmed.starts_with("CREATE DATABASE")
                && !trimmed.starts_with("USE ")
            {
                conn.query_drop(trimmed)?;
            }
        }
    }

    for (table, column, definition) in [
        (
            "purchases",
            "payment_type",
            "ENUM('TUNAI','KREDIT') NOT NULL DEFAULT 'TUNAI'",
        ),
        ("purchases", "due_date", "DATE NULL"),
        (
            "purchases",
            "paid_amount",
            "DECIMAL(15,2) NOT NULL DEFAULT 0",
        ),
        ("purchases", "discount", "DECIMAL(15,2) NOT NULL DEFAULT 0"),
        (
            "purchases",
            "tax_percent",
            "DECIMAL(5,2) NOT NULL DEFAULT 0",
        ),
        ("purchase_items", "batch_id", "INT NULL"),
        ("purchase_items", "unit_name", "VARCHAR(30) NULL"),
        ("customers", "is_active", "TINYINT(1) NOT NULL DEFAULT 1"),
        ("doctors", "is_active", "TINYINT(1) NOT NULL DEFAULT 1"),
    ] {
        let exists: Option<u64> = conn.exec_first(
            "SELECT COUNT(*) FROM information_schema.columns WHERE table_schema=DATABASE() AND table_name=:t AND column_name=:c",
            params! {"t"=>table,"c"=>column})?;
        if exists == Some(0) {
            conn.query_drop(format!(
                "ALTER TABLE {table} ADD COLUMN {column} {definition}"
            ))?;
            if table == "purchases" && column == "paid_amount" {
                conn.query_drop(
                    "UPDATE purchases SET paid_amount=total_amount WHERE payment_type='TUNAI'",
                )?;
            }
        }
    }
    let users: u64 = conn.query_first("SELECT COUNT(*) FROM users")?.unwrap_or(0);
    if users == 0 {
        for stmt in SEED_SQL.split(';') {
            let clean = stmt
                .lines()
                .filter(|l| !l.trim_start().starts_with("--"))
                .collect::<Vec<_>>()
                .join("\n");
            let clean = clean.trim();
            if !clean.is_empty() && !clean.starts_with("USE ") {
                conn.query_drop(clean)?;
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

impl NewProductInput {
    pub fn validate(&self) -> Result<()> {
        if self.code.trim().is_empty()
            || self.name.trim().is_empty()
            || self.base_unit.trim().is_empty()
            || self.min_stock < 0
            || self.initial_stock < 0
            || self.units.is_empty()
        {
            return Err(input_error("Kode, nama, satuan, dan stok wajib valid"));
        }
        super::operations::money(&self.buy_price.to_string()).map_err(|e| input_error(&e))?;
        let mut names = std::collections::BTreeSet::new();
        for (name, conversion, price) in &self.units {
            if name.trim().is_empty() || *conversion <= 0 || !names.insert(name.to_lowercase()) {
                return Err(input_error("Nama satuan harus unik dan konversi positif"));
            }
            super::operations::money(&price.to_string()).map_err(|e| input_error(&e))?;
        }
        if let Some(expiry) = &self.expired_date {
            super::operations::date(expiry).map_err(|e| input_error(&e))?;
        }
        Ok(())
    }
}

pub fn insert_product(pool: &Pool, p: &NewProductInput) -> Result<i32> {
    p.validate()?;
    let mut conn = pool.get_conn()?;
    let mut tx = conn.start_transaction(TxOpts::default())?;
    let id = insert_product_in_tx(&mut tx, p)?;
    tx.commit()?;
    Ok(id)
}

fn insert_product_in_tx(tx: &mut Transaction<'_>, p: &NewProductInput) -> Result<i32> {
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
        let bno = if p.batch_no.trim().is_empty() {
            "B001".to_string()
        } else {
            p.batch_no.clone()
        };
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

    Ok(prod_id)
}

pub fn import_product_inputs(pool: &Pool, products: &[NewProductInput]) -> Result<()> {
    for product in products {
        product.validate()?;
    }
    let mut conn = pool.get_conn()?;
    let mut tx = conn.start_transaction(TxOpts::default())?;
    for product in products {
        insert_product_in_tx(&mut tx, product)?;
    }
    tx.commit()
}

pub fn update_product(pool: &Pool, id: i32, p: &NewProductInput) -> Result<()> {
    p.validate()?;
    let mut conn = pool.get_conn()?;
    let mut tx = conn.start_transaction(TxOpts::default())?;
    let unit: Option<String> = tx.exec_first(
        "SELECT base_unit FROM products WHERE id=:id FOR UPDATE",
        params! {"id"=>id},
    )?;
    if unit.as_deref() != Some(p.base_unit.as_str()) {
        return Err(input_error(
            "Satuan dasar obat yang sudah dibuat tidak dapat diubah",
        ));
    }
    tx.exec_drop("UPDATE products SET code=:code,name=:name,category=:category,supplier_id=:sup,min_stock=:min,buy_price=:buy WHERE id=:id",params!{"id"=>id,"code"=>&p.code,"name"=>&p.name,"category"=>&p.category,"sup"=>p.supplier_id,"min"=>p.min_stock,"buy"=>p.buy_price})?;
    tx.exec_drop(
        "DELETE FROM product_units WHERE product_id=:id",
        params! {"id"=>id},
    )?;
    for (name, conversion, price) in &p.units {
        tx.exec_drop("INSERT INTO product_units(product_id,unit_name,conversion_qty,sell_price) VALUES(:id,:name,:conv,:price)",params!{"id"=>id,"name"=>name,"conv"=>conversion,"price"=>price})?;
    }
    tx.commit()
}

pub fn delete_product(pool: &Pool, id: i32) -> Result<()> {
    let mut conn = pool.get_conn()?;
    conn.exec_drop(
        "UPDATE products SET is_active = 0 WHERE id = :id",
        params! { "id" => id },
    )?;
    Ok(())
}

// Supliers
pub fn get_suppliers(pool: &Pool) -> Result<Vec<(i32, String, String, String)>> {
    let mut conn = pool.get_conn()?;
    let rows: Vec<(i32, String, Option<String>, Option<String>)> = conn.query(
        "SELECT id, name, phone, address FROM suppliers WHERE is_active = 1 ORDER BY name ASC",
    )?;
    Ok(rows
        .into_iter()
        .map(|(id, name, p, a)| {
            (
                id,
                name,
                p.unwrap_or_else(|| "-".to_string()),
                a.unwrap_or_else(|| "-".to_string()),
            )
        })
        .collect())
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
        "SELECT id, name, phone, address FROM customers WHERE is_active=1 ORDER BY name ASC",
    )?;
    Ok(rows
        .into_iter()
        .map(|(id, name, p, a)| {
            (
                id,
                name,
                p.unwrap_or_else(|| "-".to_string()),
                a.unwrap_or_else(|| "-".to_string()),
            )
        })
        .collect())
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
// Stock Opname
pub fn get_stock_opname_items(pool: &Pool) -> Result<Vec<(i32, String, String, String, i32)>> {
    let mut conn = pool.get_conn()?;
    let rows: Vec<(i32, String, String, String, i32)> = conn.query(
        "SELECT p.id, p.code, p.name, p.base_unit, COALESCE(SUM(b.stock), 0) AS total_stock \
         FROM products p \
         LEFT JOIN product_batches b ON p.id = b.product_id \
         WHERE p.is_active = 1 \
         GROUP BY p.id, p.code, p.name, p.base_unit \
         ORDER BY p.name ASC",
    )?;
    Ok(rows)
}

pub fn save_stock_opname_adjustment(
    pool: &Pool,
    product_id: i32,
    real_stock: i32,
    reason: &str,
) -> Result<()> {
    if real_stock < 0 || reason.trim().is_empty() {
        return Err(input_error(
            "Stok tidak boleh negatif dan alasan wajib diisi",
        ));
    }
    let mut conn = pool.get_conn()?;
    let mut tx = conn.start_transaction(TxOpts::default())?;
    let price: Option<f64> = tx.exec_first(
        "SELECT buy_price FROM products WHERE id=:pid AND is_active=1 FOR UPDATE",
        params! {"pid"=>product_id},
    )?;
    let price = price.ok_or_else(|| input_error("Obat tidak ditemukan"))?;
    let batches: Vec<(i32, i32)> = tx.exec(
        "SELECT id,stock FROM product_batches WHERE product_id=:pid ORDER BY id DESC FOR UPDATE",
        params! {"pid"=>product_id},
    )?;
    let current = batches.iter().map(|(_, n)| *n as i64).sum::<i64>();
    let difference = real_stock as i64 - current;
    if difference > 0 {
        if let Some((id, stock)) = batches.first() {
            tx.exec_drop(
                "UPDATE product_batches SET stock=:stock WHERE id=:id",
                params! {"stock"=>*stock as i64+difference,"id"=>id},
            )?;
        } else {
            tx.exec_drop("INSERT INTO product_batches(product_id,batch_no,stock,buy_price) VALUES(:pid,'OPNAME',:stock,:price)",params!{"pid"=>product_id,"stock"=>real_stock,"price"=>price})?;
        }
    } else if difference < 0 {
        let mut remaining = -difference;
        for (id, stock) in batches {
            let take = remaining.min(stock as i64);
            remaining -= take;
            tx.exec_drop(
                "UPDATE product_batches SET stock = stock - :take WHERE id=:id",
                params! {"take"=>take,"id"=>id},
            )?;
            if remaining == 0 {
                break;
            }
        }
    }
    if difference != 0 {
        tx.exec_drop("INSERT INTO stock_cards(product_id,ref_type,ref_no,qty_in,qty_out,balance_after) VALUES(:pid,'OPNAME',:reason,:qin,:qout,:balance)",params!{"pid"=>product_id,"reason"=>reason,"qin"=>difference.max(0),"qout"=>(-difference).max(0),"balance"=>real_stock})?;
    }
    tx.commit()
}
