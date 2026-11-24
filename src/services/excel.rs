use crate::models::Product;
use calamine::{open_workbook_auto, Data, Reader};
use rust_xlsxwriter::{Format, Workbook};
use std::path::Path;

pub fn export_rows(headers: &[&str], rows: &[Vec<String>], path: &Path) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    let format = Format::new().set_bold().set_background_color("#E7F3F1");
    for (col, h) in headers.iter().enumerate() {
        sheet
            .write_string_with_format(0, col as u16, *h, &format)
            .map_err(|e| e.to_string())?;
        sheet
            .set_column_width(col as u16, 24)
            .map_err(|e| e.to_string())?;
    }
    for (row, values) in rows.iter().enumerate() {
        for (col, value) in values.iter().enumerate() {
            sheet
                .write_string(row as u32 + 1, col as u16, value)
                .map_err(|e| e.to_string())?;
        }
    }
    workbook.save(path).map_err(|e| e.to_string())
}

pub fn supplier_rows(path: &Path) -> Result<Vec<Vec<String>>, String> {
    let mut workbook = open_workbook_auto(path).map_err(|e| e.to_string())?;
    let sheet = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or("Berkas tidak memiliki sheet")?;
    let range = workbook
        .worksheet_range(&sheet)
        .map_err(|e| e.to_string())?;
    let mut iter = range.rows();
    let header = iter
        .next()
        .ok_or("Berkas kosong")?
        .iter()
        .map(|v| v.to_string().trim().to_lowercase())
        .collect::<Vec<_>>();
    let names = [
        "kode",
        "nama",
        "telepon",
        "alamat",
        "email",
        "rekening",
        "keterangan",
    ];
    let columns = names
        .iter()
        .map(|n| header.iter().position(|v| v == n))
        .collect::<Vec<_>>();
    if columns[1].is_none() {
        return Err("Kolom Nama wajib ada".into());
    }
    let mut result = Vec::new();
    for row in iter {
        if row.iter().all(|v| matches!(v, Data::Empty)) {
            continue;
        }
        let values = columns
            .iter()
            .map(|c| {
                c.and_then(|i| row.get(i))
                    .map(ToString::to_string)
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            })
            .collect::<Vec<_>>();
        if values[1].is_empty() {
            return Err("Nama suplier wajib diisi pada setiap baris".into());
        }
        result.push(values);
    }
    if result.is_empty() {
        return Err("Tidak ada suplier dalam berkas".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exported_master_can_be_imported() {
        let path = std::env::temp_dir().join(format!(
            "apotek-excel-{}.xlsx",
            chrono::Utc::now().timestamp_micros()
        ));
        let mut product = Product {
            id: 7,
            code: "000899001".into(),
            name: "Vitamin C 100 mg".into(),
            category: "Vitamin".into(),
            supplier_id: None,
            base_unit: "Tablet".into(),
            purchase_unit: "Tablet".into(),
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
        product.purchase_unit = "tablet".into();
        export_products_to_excel(std::slice::from_ref(&product), &path).unwrap();
        let rows = import_products_from_excel(&path).unwrap();
        assert_eq!(rows[0].purchase_unit, "Tablet");
        assert_eq!(rows[0].purchase_conversion, 1);
        product.units[0].conversion_qty = 10;
        export_products_to_excel(std::slice::from_ref(&product), &path).unwrap();
        assert!(import_products_from_excel(&path)
            .err()
            .unwrap()
            .contains("harus 1"));
        let supplier = vec![
            "001234".into(),
            "PT IB".into(),
            "08123".into(),
            "Alamat".into(),
            "ib@example.com".into(),
            "000123456".into(),
            "Catatan".into(),
        ];
        export_rows(
            &[
                "Kode",
                "Nama",
                "Telepon",
                "Alamat",
                "Email",
                "Rekening",
                "Keterangan",
            ],
            &[supplier.clone()],
            &path,
        )
        .unwrap();
        assert_eq!(supplier_rows(&path).unwrap(), vec![supplier]);
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

    worksheet
        .set_column_format(1, &Format::new().set_num_format("@"))
        .map_err(|e| e.to_string())?;
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
        "Harga Beli Dasar",
        "Harga Jual Dasar",
        "Status",
        "Satuan Beli",
        "Isi Satuan Beli",
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
        worksheet
            .write_string(r, 10, &p.purchase_unit)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_number(
                r,
                11,
                p.units
                    .iter()
                    .find(|u| u.unit_name.eq_ignore_ascii_case(&p.purchase_unit))
                    .map(|u| u.conversion_qty)
                    .unwrap_or(1),
            )
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
    pub purchase_unit: String,
    pub purchase_conversion: i32,
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
        find(&["harga beli dasar", "harga beli"]),
        find(&["harga jual dasar", "harga jual"]),
        find(&["satuan beli"]),
        find(&["isi satuan beli"]),
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
        let mut purchase_unit = if cell(7).is_empty() {
            base_unit.clone()
        } else {
            cell(7)
        };
        let purchase_conversion = if cell(8).is_empty() {
            1
        } else {
            crate::db::operations::quantity(&cell(8))?
        };
        if purchase_unit.eq_ignore_ascii_case(&base_unit) {
            if purchase_conversion != 1 {
                return Err(format!(
                    "Baris {}: isi satuan beli yang sama dengan satuan dasar harus 1",
                    index + 2
                ));
            }
            purchase_unit = base_unit.clone();
        }
        rows.push(ImportedProductRow {
            purchase_unit,
            purchase_conversion,
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
