use crate::models::{Product, ProductBatch, ProductUnit, Sale, User};
use mysql::prelude::*;
use mysql::*;

pub fn authenticate_user(pool: &Pool, username: &str, password: &str) -> Result<Option<User>> {
    let mut conn = pool.get_conn()?;
    let query = "SELECT id, username, full_name, role FROM users WHERE username = :u AND password_hash = :p AND is_active = 1 LIMIT 1";
    let res = conn.exec_first::<(i32, String, String, String), _>(
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

/// Inisialisasi otomatis tabel database tanpa perlu impor file .sql manual
pub fn auto_migrate(pool: &Pool) -> Result<()> {
    let mut conn = pool.get_conn()?;

    let statements = [
        "CREATE TABLE IF NOT EXISTS users (
            id INT AUTO_INCREMENT PRIMARY KEY,
            username VARCHAR(50) NOT NULL UNIQUE,
            password_hash VARCHAR(255) NOT NULL,
            full_name VARCHAR(100) NOT NULL,
            role ENUM('ADMIN', 'KASIR', 'GUDANG') NOT NULL DEFAULT 'KASIR',
            is_active TINYINT(1) NOT NULL DEFAULT 1,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        ) ENGINE=InnoDB;",

        "CREATE TABLE IF NOT EXISTS suppliers (
            id INT AUTO_INCREMENT PRIMARY KEY,
            name VARCHAR(100) NOT NULL,
            phone VARCHAR(25),
            address TEXT,
            is_active TINYINT(1) NOT NULL DEFAULT 1,
            INDEX idx_supplier_name (name)
        ) ENGINE=InnoDB;",

        "CREATE TABLE IF NOT EXISTS customers (
            id INT AUTO_INCREMENT PRIMARY KEY,
            name VARCHAR(100) NOT NULL,
            phone VARCHAR(25),
            address TEXT,
            INDEX idx_customer_name (name)
        ) ENGINE=InnoDB;",

        "CREATE TABLE IF NOT EXISTS doctors (
            id INT AUTO_INCREMENT PRIMARY KEY,
            name VARCHAR(100) NOT NULL,
            sip_no VARCHAR(50),
            INDEX idx_doctor_name (name)
        ) ENGINE=InnoDB;",

        "CREATE TABLE IF NOT EXISTS products (
            id INT AUTO_INCREMENT PRIMARY KEY,
            code VARCHAR(50) NOT NULL UNIQUE,
            name VARCHAR(150) NOT NULL,
            category VARCHAR(50) DEFAULT 'Umum',
            supplier_id INT NULL,
            base_unit VARCHAR(30) NOT NULL,
            min_stock INT NOT NULL DEFAULT 0,
            buy_price DECIMAL(15, 2) NOT NULL DEFAULT 0,
            is_active TINYINT(1) NOT NULL DEFAULT 1,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (supplier_id) REFERENCES suppliers(id) ON DELETE SET NULL,
            INDEX idx_prod_name (name),
            INDEX idx_prod_code (code)
        ) ENGINE=InnoDB;",

        "CREATE TABLE IF NOT EXISTS product_units (
            id INT AUTO_INCREMENT PRIMARY KEY,
            product_id INT NOT NULL,
            unit_name VARCHAR(30) NOT NULL,
            conversion_qty INT NOT NULL DEFAULT 1,
            sell_price DECIMAL(15, 2) NOT NULL DEFAULT 0,
            FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE,
            INDEX idx_unit_prod (product_id)
        ) ENGINE=InnoDB;",

        "CREATE TABLE IF NOT EXISTS product_batches (
            id INT AUTO_INCREMENT PRIMARY KEY,
            product_id INT NOT NULL,
            batch_no VARCHAR(50) NOT NULL,
            expired_date DATE NULL,
            stock INT NOT NULL DEFAULT 0,
            buy_price DECIMAL(15, 2) NOT NULL DEFAULT 0,
            FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE,
            INDEX idx_batch_exp (expired_date),
            INDEX idx_batch_prod_stock (product_id, stock)
        ) ENGINE=InnoDB;",

        "CREATE TABLE IF NOT EXISTS sales (
            id INT AUTO_INCREMENT PRIMARY KEY,
            invoice_no VARCHAR(50) NOT NULL UNIQUE,
            sale_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            payment_type ENUM('TUNAI', 'KREDIT') NOT NULL DEFAULT 'TUNAI',
            due_date DATE NULL,
            customer_id INT NULL,
            doctor_id INT NULL,
            operator_id INT NOT NULL,
            subtotal DECIMAL(15, 2) NOT NULL DEFAULT 0,
            discount DECIMAL(15, 2) NOT NULL DEFAULT 0,
            grand_total DECIMAL(15, 2) NOT NULL DEFAULT 0,
            paid_amount DECIMAL(15, 2) NOT NULL DEFAULT 0,
            change_amount DECIMAL(15, 2) NOT NULL DEFAULT 0,
            total_profit DECIMAL(15, 2) NOT NULL DEFAULT 0,
            INDEX idx_sale_date (sale_date),
            FOREIGN KEY (customer_id) REFERENCES customers(id) ON DELETE SET NULL,
            FOREIGN KEY (doctor_id) REFERENCES doctors(id) ON DELETE SET NULL,
            FOREIGN KEY (operator_id) REFERENCES users(id)
        ) ENGINE=InnoDB;",

        "CREATE TABLE IF NOT EXISTS sale_items (
            id INT AUTO_INCREMENT PRIMARY KEY,
            sale_id INT NOT NULL,
            product_id INT NOT NULL,
            unit_name VARCHAR(30) NOT NULL,
            qty INT NOT NULL,
            conversion_qty INT NOT NULL DEFAULT 1,
            qty_in_base INT NOT NULL,
            sell_price DECIMAL(15, 2) NOT NULL,
            discount DECIMAL(15, 2) NOT NULL DEFAULT 0,
            subtotal DECIMAL(15, 2) NOT NULL,
            FOREIGN KEY (sale_id) REFERENCES sales(id) ON DELETE CASCADE,
            FOREIGN KEY (product_id) REFERENCES products(id),
            INDEX idx_item_sale (sale_id)
        ) ENGINE=InnoDB;",

        "CREATE TABLE IF NOT EXISTS sale_item_batches (
            id INT AUTO_INCREMENT PRIMARY KEY,
            sale_item_id INT NOT NULL,
            batch_id INT NOT NULL,
            qty_deducted INT NOT NULL,
            buy_price_snapshot DECIMAL(15, 2) NOT NULL,
            FOREIGN KEY (sale_item_id) REFERENCES sale_items(id) ON DELETE CASCADE,
            FOREIGN KEY (batch_id) REFERENCES product_batches(id)
        ) ENGINE=InnoDB;",

        "CREATE TABLE IF NOT EXISTS pending_sales (
            id INT AUTO_INCREMENT PRIMARY KEY,
            operator_id INT NOT NULL,
            customer_name VARCHAR(100),
            cart_json TEXT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        ) ENGINE=InnoDB;",

        "CREATE TABLE IF NOT EXISTS stock_cards (
            id BIGINT AUTO_INCREMENT PRIMARY KEY,
            product_id INT NOT NULL,
            batch_id INT NULL,
            ref_type ENUM('PENJUALAN', 'PEMBELIAN', 'RETUR', 'OPNAME') NOT NULL,
            ref_no VARCHAR(50),
            qty_in INT NOT NULL DEFAULT 0,
            qty_out INT NOT NULL DEFAULT 0,
            balance_after INT NOT NULL,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (product_id) REFERENCES products(id),
            INDEX idx_card_lookup (product_id, created_at)
        ) ENGINE=InnoDB;",

        "CREATE TABLE IF NOT EXISTS settings (
            `key` VARCHAR(50) PRIMARY KEY,
            `value` TEXT NOT NULL
        ) ENGINE=InnoDB;",

        // Default Users
        "INSERT INTO users (username, password_hash, full_name, role) VALUES 
         ('admin', 'admin123', 'Administrator Utama', 'ADMIN'),
         ('kasir', 'kasir123', 'Kasir Apotek', 'KASIR')
         ON DUPLICATE KEY UPDATE full_name=VALUES(full_name);",
    ];

    for stmt in statements {
        conn.query_drop(stmt)?;
    }

    Ok(())
}

