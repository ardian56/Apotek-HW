-- ==============================================================
-- SEED DATA AWAL APOTEK_DB
-- ==============================================================
USE apotek_db;

-- 1. Default Pengguna:
-- admin / admin123 (MD5 / SHA256 atau hash string sementara)
INSERT INTO users (username, password_hash, full_name, role) VALUES 
('admin', 'admin123', 'Administrator Utama', 'ADMIN'),
('kasir', 'kasir123', 'Kasir Apotek', 'KASIR'),
('gudang', 'gudang123', 'Petugas Gudang', 'GUDANG')
ON DUPLICATE KEY UPDATE full_name=VALUES(full_name);

-- 2. Setting Default Toko
INSERT INTO settings (`key`, `value`) VALUES
('store_name', 'APOTEK SEHAT SEJAHTERA'),
('store_address', 'Jl. Merdeka No. 45, Kota Sehat'),
('store_phone', '0812-3456-7890'),
('receipt_header', 'Terima Kasih Atas Kunjungan Anda'),
('receipt_footer', 'Semoga Lekas Sembuh\nBarang yang sudah dibeli tidak dapat ditukar/dikembalikan'),
('receipt_paper_width', '58'), -- '58' atau '80' mm
('printer_name', 'POS-58')
ON DUPLICATE KEY UPDATE `value`=VALUES(`value`);

-- 3. Contoh Master Supplier
INSERT INTO suppliers (name, phone, address) VALUES
('PT. Kimia Farma Trading', '021-1234567', 'Kawasan Industri Pulogadung'),
('PT. Anugerah Pharmindo Lestari', '021-7654321', 'Jl. Rawa Gelam IV'),
('Distributor Obat Mandiri', '0811-223344', 'Jl. Sudirman No. 12')
ON DUPLICATE KEY UPDATE name=VALUES(name);

-- 4. Contoh Master Dokter
INSERT INTO doctors (name, sip_no) VALUES
('dr. H. Rahmat Hidayat, Sp.PD', 'SIP/440/012/2023'),
('dr. Siti Aminah', 'SIP/440/098/2024')
ON DUPLICATE KEY UPDATE name=VALUES(name);

-- 5. Contoh Master Pelanggan
INSERT INTO customers (name, phone, address) VALUES
('Pelanggan Umum (Walk-in)', '-', '-'),
('Bpk. Budi Santoso', '0813-1111-2222', 'Jl. Kenanga No. 8')
ON DUPLICATE KEY UPDATE name=VALUES(name);

-- 6. Contoh Master Obat dengan Multi-Satuan & Batches
-- Obat 1: Paracetamol 500mg
INSERT INTO products (id, code, name, category, supplier_id, base_unit, min_stock, buy_price) VALUES
(1, '8999909001', 'Paracetamol 500mg Tablet', 'Analgesik', 1, 'Tablet', 50, 450.00)
ON DUPLICATE KEY UPDATE name=VALUES(name);

INSERT INTO product_units (product_id, unit_name, conversion_qty, sell_price) VALUES
(1, 'Tablet', 1, 1000.00),
(1, 'Strip', 10, 9500.00),
(1, 'Box', 100, 90000.00)
ON DUPLICATE KEY UPDATE sell_price=VALUES(sell_price);

INSERT INTO product_batches (product_id, batch_no, expired_date, stock, buy_price) VALUES
(1, 'BATCH-PCT-01', DATE_ADD(CURDATE(), INTERVAL 6 MONTH), 150, 450.00),
(1, 'BATCH-PCT-02', DATE_ADD(CURDATE(), INTERVAL 18 MONTH), 500, 450.00)
ON DUPLICATE KEY UPDATE stock=VALUES(stock);

-- Obat 2: Amoxicillin 500mg (Antibiotik)
INSERT INTO products (id, code, name, category, supplier_id, base_unit, min_stock, buy_price) VALUES
(2, '8999909002', 'Amoxicillin 500mg Kapsul', 'Antibiotik', 2, 'Kapsul', 30, 800.00)
ON DUPLICATE KEY UPDATE name=VALUES(name);

INSERT INTO product_units (product_id, unit_name, conversion_qty, sell_price) VALUES
(2, 'Kapsul', 1, 1500.00),
(2, 'Strip', 10, 14000.00),
(2, 'Box', 100, 135000.00)
ON DUPLICATE KEY UPDATE sell_price=VALUES(sell_price);

INSERT INTO product_batches (product_id, batch_no, expired_date, stock, buy_price) VALUES
(2, 'AMX-2401', DATE_ADD(CURDATE(), INTERVAL 12 MONTH), 200, 800.00)
ON DUPLICATE KEY UPDATE stock=VALUES(stock);

-- Obat 3: Bodrex Extra (Mendekati Expired untuk testing warning visual)
INSERT INTO products (id, code, name, category, supplier_id, base_unit, min_stock, buy_price) VALUES
(3, '8999909003', 'Bodrex Extra Tablet', 'Sakit Kepala', 3, 'Tablet', 20, 600.00)
ON DUPLICATE KEY UPDATE name=VALUES(name);

INSERT INTO product_units (product_id, unit_name, conversion_qty, sell_price) VALUES
(3, 'Tablet', 1, 1200.00),
(3, 'Blister', 4, 4500.00)
ON DUPLICATE KEY UPDATE sell_price=VALUES(sell_price);

INSERT INTO product_batches (product_id, batch_no, expired_date, stock, buy_price) VALUES
(3, 'BDX-2309', DATE_ADD(CURDATE(), INTERVAL 25 DAY), 15, 600.00)
ON DUPLICATE KEY UPDATE stock=VALUES(stock);
