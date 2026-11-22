use crate::models::Product;
use calamine::{open_workbook_auto, DataType, Reader};
use rust_xlsxwriter::{Format, Workbook};
use std::path::Path;

/// Export daftar produk ke format Excel (.xlsx) dengan rust_xlsxwriter
pub fn export_products_to_excel<P: AsRef<Path>>(products: &[Product], path: P) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let header_format = Format::new()
        .set_bold()
        .set_background_color("#1E88E5")
        .set_font_color("#FFFFFF");

    let headers = [
        "ID", "Kode Barcode", "Nama Obat", "Kategori", "Satuan Dasar",
        "Stok Minimal", "Total Stok Fisik", "Harga Beli", "Status",
    ];

    for (col, text) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(0, col as u16, *text, &header_format)
            .map_err(|e| e.to_string())?;
    }

    for (row_idx, p) in products.iter().enumerate() {
        let r = (row_idx + 1) as u32;
        worksheet.write_number(r, 0, p.id).map_err(|e| e.to_string())?;
        worksheet.write_string(r, 1, &p.code).map_err(|e| e.to_string())?;
        worksheet.write_string(r, 2, &p.name).map_err(|e| e.to_string())?;
        worksheet.write_string(r, 3, &p.category).map_err(|e| e.to_string())?;
        worksheet.write_string(r, 4, &p.base_unit).map_err(|e| e.to_string())?;
        worksheet.write_number(r, 5, p.min_stock).map_err(|e| e.to_string())?;
        worksheet.write_number(r, 6, p.total_stock).map_err(|e| e.to_string())?;
        worksheet.write_number(r, 7, p.buy_price).map_err(|e| e.to_string())?;
        worksheet.write_string(r, 8, if p.is_active { "Aktif" } else { "Nonaktif" }).map_err(|e| e.to_string())?;
    }

    workbook.save(path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Struktur data hasil parsing file Excel untuk di-import
pub struct ImportedProductRow {
    pub code: String,
    pub name: String,
    pub category: String,
    pub base_unit: String,
    pub min_stock: i32,
    pub buy_price: f64,
}

/// Import file Excel (.xls atau .xlsx) menggunakan calamine
pub fn import_products_from_excel<P: AsRef<Path>>(path: P) -> Result<Vec<ImportedProductRow>, String> {
    let mut workbook = open_workbook_auto(path).map_err(|e| format!("Gagal buka file Excel: {}", e))?;
    let sheet_names = workbook.sheet_names();
    if sheet_names.is_empty() {
        return Err("File Excel tidak memiliki lembar kerja (sheet)".to_string());
    }

    let range = workbook
        .worksheet_range(&sheet_names[0])
        .map_err(|e| format!("Gagal membaca sheet pertama: {}", e))?;

    let mut rows = Vec::new();
    let mut iter = range.rows();

    // Lewati baris header (baris 0)
    let _ = iter.next();

    for row in iter {
        if row.len() < 6 {
            continue;
        }

        let code = match &row[0] {
            DataType::String(s) => s.trim().to_string(),
            DataType::Int(n) => n.to_string(),
            DataType::Float(f) => format!("{:.0}", f),
            _ => continue,
        };

        if code.is_empty() {
            continue;
        }

        let name = match &row[1] {
            DataType::String(s) => s.trim().to_string(),
            _ => continue,
        };

        let category = match &row[2] {
            DataType::String(s) => s.trim().to_string(),
            _ => "Umum".to_string(),
        };

        let base_unit = match &row[3] {
            DataType::String(s) => s.trim().to_string(),
            _ => "Pcs".to_string(),
        };

        let min_stock = match &row[4] {
            DataType::Int(n) => *n as i32,
            DataType::Float(f) => *f as i32,
            _ => 0,
        };

        let buy_price = match &row[5] {
            DataType::Float(f) => *f,
            DataType::Int(n) => *n as f64,
            _ => 0.0,
        };

        rows.push(ImportedProductRow {
            code,
            name,
            category,
            base_unit,
            min_stock,
            buy_price,
        });
    }

    Ok(rows)
}
