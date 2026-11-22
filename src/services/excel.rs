use crate::models::Product;
use calamine::{open_workbook_auto, Data, Reader};
use rust_xlsxwriter::{Format, Workbook};
use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exported_master_can_be_imported() {
        let path = std::env::temp_dir().join(format!(
            "apotek-excel-{}.xlsx",
            chrono::Utc::now().timestamp_micros()
        ));
        let product = Product {
            id: 7,
            code: "000899001".into(),
            name: "Vitamin C 100 mg".into(),
            category: "Vitamin".into(),
            supplier_id: None,
            base_unit: "Tablet".into(),
            min_stock: 5,
            buy_price: 1250.50,
            is_active: true,
            total_stock: 0,
            units: vec![crate::models::ProductUnit {
                id: 1,
                product_id: 7,
                unit_name: "Tablet".into(),
                conversion_qty: 1,
                sell_price: 1500.75,
            }],
            batches: vec![],
        };
        export_products_to_excel(&[product.clone()], &path).unwrap();
        let rows = import_products_from_excel(&path).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].code, product.code);
        assert_eq!(rows[0].name, product.name);
        assert_eq!(rows[0].min_stock, 5);
        assert_eq!(rows[0].buy_price, 1250.50);
        assert_eq!(rows[0].sell_price, 1500.75);
        std::fs::remove_file(path).unwrap();
    }
}

/// Export daftar produk ke format Excel (.xlsx) dengan rust_xlsxwriter
pub fn export_products_to_excel<P: AsRef<Path>>(
    products: &[Product],
    path: P,
) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    let header_format = Format::new()
        .set_bold()
        .set_background_color("#1E88E5")
        .set_font_color("#FFFFFF");

    let headers = [
        "ID",
        "Kode Barcode",
        "Nama Obat",
        "Kategori",
        "Satuan Dasar",
        "Stok Minimal",
        "Total Stok Fisik",
        "Harga Beli",
        "Harga Jual",
        "Status",
    ];

    for (col, text) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(0, col as u16, *text, &header_format)
            .map_err(|e| e.to_string())?;
    }

    for (row_idx, p) in products.iter().enumerate() {
        let r = (row_idx + 1) as u32;
        worksheet
            .write_number(r, 0, p.id)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(r, 1, &p.code)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(r, 2, &p.name)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(r, 3, &p.category)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(r, 4, &p.base_unit)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_number(r, 5, p.min_stock)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_number(r, 6, p.total_stock)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_number(r, 7, p.buy_price)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_number(
                r,
                8,
                p.units
                    .iter()
                    .find(|u| u.conversion_qty == 1)
                    .map(|u| u.sell_price)
                    .unwrap_or(p.buy_price),
            )
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(r, 9, if p.is_active { "Aktif" } else { "Nonaktif" })
            .map_err(|e| e.to_string())?;
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
    pub sell_price: f64,
}

/// Import file Excel (.xls atau .xlsx) menggunakan calamine
pub fn import_products_from_excel<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<ImportedProductRow>, String> {
    let mut workbook =
        open_workbook_auto(path).map_err(|e| format!("Gagal buka file Excel: {}", e))?;
    let sheet_names = workbook.sheet_names();
    if sheet_names.is_empty() {
        return Err("File Excel tidak memiliki lembar kerja (sheet)".to_string());
    }

    let range = workbook
        .worksheet_range(&sheet_names[0])
        .map_err(|e| format!("Gagal membaca sheet pertama: {}", e))?;

    let mut iter = range.rows();
    let headers = iter
        .next()
        .ok_or("File Excel kosong")?
        .iter()
        .map(|v| v.to_string().trim().to_lowercase())
        .collect::<Vec<_>>();
    let find = |names: &[&str]| headers.iter().position(|h| names.contains(&h.as_str()));
    let columns = [
        find(&["kode barcode", "kode", "barcode"]),
        find(&["nama obat", "nama"]),
        find(&["kategori"]),
        find(&["satuan dasar", "satuan"]),
        find(&["stok minimal", "min stock"]),
        find(&["harga beli"]),
        find(&["harga jual"]),
    ];
    if columns[..6].iter().any(Option::is_none) {
        return Err("Header wajib: Kode Barcode, Nama Obat, Kategori, Satuan Dasar, Stok Minimal, Harga Beli".into());
    }
    let mut rows = Vec::new();
    for (index, row) in iter.enumerate() {
        if row.iter().all(|v| matches!(v, Data::Empty)) {
            continue;
        }
        let cell = |c: usize| {
            columns[c]
                .and_then(|i| row.get(i))
                .map(ToString::to_string)
                .unwrap_or_default()
                .trim()
                .to_string()
        };
        let code = cell(0);
        let name = cell(1);
        let base_unit = cell(3);
        if code.is_empty() || name.is_empty() || base_unit.is_empty() {
            return Err(format!(
                "Baris {}: kode, nama, dan satuan wajib diisi",
                index + 2
            ));
        }
        let min_stock = cell(4)
            .parse::<i32>()
            .map_err(|_| format!("Baris {}: stok minimum harus bilangan bulat", index + 2))?;
        if min_stock < 0 {
            return Err(format!("Baris {}: stok minimum negatif", index + 2));
        }
        let buy_price = crate::db::operations::money(&cell(5))?;
        let sell_price = if columns[6].is_some() {
            crate::db::operations::money(&cell(6))?
        } else {
            buy_price
        };
        rows.push(ImportedProductRow {
            code,
            name,
            category: cell(2),
            base_unit,
            min_stock,
            buy_price,
            sell_price,
        });
    }
    Ok(rows)
}
