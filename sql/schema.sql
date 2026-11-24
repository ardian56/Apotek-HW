-- ==============================================================
-- DATABASE SCHEMA: APOTEK_DB
-- Optimal untuk Windows 7 SP1 (32-bit), Core 2 Duo, RAM 1 GB
-- ==============================================================

CREATE DATABASE IF NOT EXISTS apotek_db CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
USE apotek_db;

-- 1. PENGGUNA & HAK AKSES
CREATE TABLE IF NOT EXISTS users (
    id INT AUTO_INCREMENT PRIMARY KEY,
    username VARCHAR(50) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    full_name VARCHAR(100) NOT NULL,
    role ENUM('ADMIN', 'KASIR', 'GUDANG') NOT NULL DEFAULT 'KASIR',
    is_active TINYINT(1) NOT NULL DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB;

-- 2. MASTER DATA PENDUKUNG
CREATE TABLE IF NOT EXISTS suppliers (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    code VARCHAR(50) NOT NULL DEFAULT '',
    email VARCHAR(150) NOT NULL DEFAULT '',
    bank_account VARCHAR(150) NOT NULL DEFAULT '',
    notes TEXT,
    phone VARCHAR(25),
    address TEXT,
    is_active TINYINT(1) NOT NULL DEFAULT 1,
    INDEX idx_supplier_name (name)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS customers (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    phone VARCHAR(25),
    address TEXT,
    INDEX idx_customer_name (name)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS doctors (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    sip_no VARCHAR(50),
    INDEX idx_doctor_name (name)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS salespeople (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    phone VARCHAR(25),
    address TEXT,
    is_active TINYINT(1) NOT NULL DEFAULT 1
) ENGINE=InnoDB;

-- 3. MASTER OBAT & MULTI-SATUAN
CREATE TABLE IF NOT EXISTS products (
    id INT AUTO_INCREMENT PRIMARY KEY,
    code VARCHAR(50) NOT NULL UNIQUE,
    name VARCHAR(150) NOT NULL,
    category VARCHAR(50) DEFAULT 'Umum',
    supplier_id INT NULL,
    base_unit VARCHAR(30) NOT NULL,
    purchase_unit VARCHAR(30) NOT NULL DEFAULT '',
    min_stock INT NOT NULL DEFAULT 0,
    buy_price DECIMAL(15, 2) NOT NULL DEFAULT 0,
    is_active TINYINT(1) NOT NULL DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id) ON DELETE SET NULL,
    INDEX idx_prod_name (name),
    INDEX idx_prod_code (code),
    INDEX idx_prod_category (category)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS product_units (
    id INT AUTO_INCREMENT PRIMARY KEY,
    product_id INT NOT NULL,
    unit_name VARCHAR(30) NOT NULL,
    conversion_qty INT NOT NULL DEFAULT 1,
    sell_price DECIMAL(15, 2) NOT NULL DEFAULT 0,
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE,
    INDEX idx_unit_prod (product_id)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS product_batches (
    id INT AUTO_INCREMENT PRIMARY KEY,
    product_id INT NOT NULL,
    batch_no VARCHAR(50) NOT NULL,
    expired_date DATE NULL,
    stock INT NOT NULL DEFAULT 0,
    buy_price DECIMAL(15, 2) NOT NULL DEFAULT 0,
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE,
    INDEX idx_batch_exp (expired_date),
    INDEX idx_batch_prod_stock (product_id, stock)
) ENGINE=InnoDB;

-- 4. TRANSAKSI PENJUALAN (POS)
CREATE TABLE IF NOT EXISTS sales (
    id INT AUTO_INCREMENT PRIMARY KEY,
    invoice_no VARCHAR(50) NOT NULL UNIQUE,
    sale_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    payment_type ENUM('TUNAI', 'KREDIT') NOT NULL DEFAULT 'TUNAI',
    due_date DATE NULL,
    customer_id INT NULL,
    doctor_id INT NULL,
    salesperson_id INT NULL,
    shipping DECIMAL(15,2) NOT NULL DEFAULT 0,
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
    FOREIGN KEY (operator_id) REFERENCES users(id),
    CONSTRAINT fk_sales_salesperson FOREIGN KEY(salesperson_id) REFERENCES salespeople(id) ON DELETE SET NULL
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS sale_items (
    id INT AUTO_INCREMENT PRIMARY KEY,
    sale_id INT NOT NULL,
    product_id INT NOT NULL,
    product_code_snapshot VARCHAR(50) NULL,
    product_name_snapshot VARCHAR(150) NULL,
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
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS sale_item_batches (
    id INT AUTO_INCREMENT PRIMARY KEY,
    sale_item_id INT NOT NULL,
    batch_id INT NOT NULL,
    qty_deducted INT NOT NULL,
    buy_price_snapshot DECIMAL(15, 2) NOT NULL,
    FOREIGN KEY (sale_item_id) REFERENCES sale_items(id) ON DELETE CASCADE,
    FOREIGN KEY (batch_id) REFERENCES product_batches(id)
) ENGINE=InnoDB;

-- 5. TRANSAKSI PEMBELIAN
CREATE TABLE IF NOT EXISTS purchases (
    id INT AUTO_INCREMENT PRIMARY KEY,
    invoice_no VARCHAR(50) NOT NULL,
    purchase_date DATE NOT NULL,
    supplier_id INT NOT NULL,
    operator_id INT NOT NULL,
    total_amount DECIMAL(15, 2) NOT NULL DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id),
    FOREIGN KEY (operator_id) REFERENCES users(id),
    INDEX idx_purchase_date (purchase_date)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS purchase_items (
    id INT AUTO_INCREMENT PRIMARY KEY,
    purchase_id INT NOT NULL,
    product_id INT NOT NULL,
    batch_no VARCHAR(50),
    expired_date DATE NULL,
    qty INT NOT NULL,
    conversion_qty INT NOT NULL DEFAULT 1,
    buy_price DECIMAL(15, 2) NOT NULL,
    subtotal DECIMAL(15, 2) NOT NULL,
    FOREIGN KEY (purchase_id) REFERENCES purchases(id) ON DELETE CASCADE,
    FOREIGN KEY (product_id) REFERENCES products(id)
) ENGINE=InnoDB;

-- 6. PENDING TRANSACTION & KARTU STOK
CREATE TABLE IF NOT EXISTS pending_sales (
    id INT AUTO_INCREMENT PRIMARY KEY,
    operator_id INT NOT NULL,
    customer_name VARCHAR(100),
    cart_json TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS stock_cards (
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
) ENGINE=InnoDB;

-- 7. PENGATURAN TOKO & STRUK
CREATE TABLE IF NOT EXISTS settings (
    `key` VARCHAR(50) PRIMARY KEY,
    `value` TEXT NOT NULL
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS payments (
    id INT AUTO_INCREMENT PRIMARY KEY,
    ref_type ENUM('PENJUALAN','PEMBELIAN') NOT NULL,
    ref_id INT NOT NULL,
    invoice_no VARCHAR(50) NOT NULL,
    amount DECIMAL(15,2) NOT NULL,
    operator_id INT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (operator_id) REFERENCES users(id),
    INDEX idx_payment_ref (ref_type,ref_id)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS returns (
    id INT AUTO_INCREMENT PRIMARY KEY,
    ref_type ENUM('PENJUALAN','PEMBELIAN') NOT NULL,
    ref_id INT NOT NULL,
    invoice_no VARCHAR(50) NOT NULL,
    total_amount DECIMAL(15,2) NOT NULL,
    refund_amount DECIMAL(15,2) NOT NULL,
    reason TEXT NOT NULL,
    operator_id INT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE KEY idx_return_ref (ref_type,ref_id),
    FOREIGN KEY (operator_id) REFERENCES users(id)
) ENGINE=InnoDB;

CREATE TABLE IF NOT EXISTS pending_purchases (
    id INT AUTO_INCREMENT PRIMARY KEY,
    operator_id INT NOT NULL,
    cart_json LONGTEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (operator_id) REFERENCES users(id)
) ENGINE=InnoDB;
