use crate::models::Sale;

pub struct ReceiptSettings {
    pub customer_name: String,
    pub operator_name: String,
    pub returned: bool,
    pub store_name: String,
    pub address: String,
    pub phone: String,
    pub header_note: String,
    pub footer_note: String,
    pub paper_width: u8, // 58 atau 80 (kolom karakter: 32 untuk 58mm, 48 untuk 80mm)
    pub printer_name: String,
}

impl Default for ReceiptSettings {
    fn default() -> Self {
        Self {
            customer_name: "Pelanggan umum".into(),
            operator_name: String::new(),
            returned: false,
            store_name: "APOTEK SEHAT SEJAHTERA".to_string(),
            address: "Jl. Merdeka No. 45, Kota Sehat".to_string(),
            phone: "0812-3456-7890".to_string(),
            header_note: "Struk Pembelian Obat".to_string(),
            footer_note: "Semoga Lekas Sembuh\nBarang dibeli tdk dpt ditukar".to_string(),
            paper_width: 58,
            printer_name: "POS-58".to_string(),
        }
    }
}

impl ReceiptSettings {
    pub fn from_values(values: &std::collections::BTreeMap<String, String>) -> Self {
        Self {
            customer_name: "Pelanggan umum".into(),
            operator_name: String::new(),
            returned: false,
            store_name: crate::db::operations::pharmacy_name_from_values(values),
            address: values.get("store_address").cloned().unwrap_or_default(),
            phone: values.get("store_phone").cloned().unwrap_or_default(),
            header_note: values.get("receipt_header").cloned().unwrap_or_default(),
            footer_note: values.get("receipt_footer").cloned().unwrap_or_default(),
            paper_width: values
                .get("receipt_paper_width")
                .and_then(|v| v.parse().ok())
                .unwrap_or(58),
            printer_name: values.get("printer_name").cloned().unwrap_or_default(),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum ReceiptStyle {
    Header,
    Center,
    Body,
    Bold,
    Total,
    Rule,
}

pub struct ReceiptRow {
    pub text: String,
    pub right: Option<String>,
    pub style: ReceiptStyle,
}

pub fn receipt_rows(sale: &Sale, settings: &ReceiptSettings) -> Vec<ReceiptRow> {
    use crate::gui::views::components::{number, rupiah};
    let mut rows = Vec::new();
    let mut add =
        |text: String, right: Option<String>, style| rows.push(ReceiptRow { text, right, style });
    if settings.returned {
        add("FAKTUR DIRETUR (salinan)".into(), None, ReceiptStyle::Bold);
    }
    add(settings.store_name.clone(), None, ReceiptStyle::Header);
    for value in [&settings.address, &settings.phone, &settings.header_note] {
        if !value.trim().is_empty() {
            add(value.clone(), None, ReceiptStyle::Center);
        }
    }
    add(String::new(), None, ReceiptStyle::Rule);
    let date = chrono::NaiveDateTime::parse_from_str(&sale.sale_date, "%Y-%m-%d %H:%M:%S")
        .map(|date| date.format("%d/%m/%Y %H:%M:%S").to_string())
        .unwrap_or_else(|_| sale.sale_date.clone());
    for value in [
        format!("No. : {}", sale.invoice_no),
        format!("Tgl : {date}"),
        format!("Kasir : {}", settings.operator_name),
        format!("Pelanggan : {}", settings.customer_name),
        format!("Pemb. : {}", sale.payment_type),
    ] {
        add(value, None, ReceiptStyle::Body);
    }
    add(String::new(), None, ReceiptStyle::Rule);
    add("BARANG".into(), Some("JUMLAH".into()), ReceiptStyle::Bold);
    for item in &sale.items {
        add(item.product_name.clone(), None, ReceiptStyle::Body);
        add(
            format!(
                "{} {} x {}",
                item.qty,
                item.unit_name,
                number(item.sell_price)
            ),
            Some(number(item.subtotal)),
            ReceiptStyle::Body,
        );
        if item.discount > 0.0 {
            add(
                "Diskon barang".into(),
                Some(rupiah(item.discount)),
                ReceiptStyle::Body,
            );
        }
    }
    add(String::new(), None, ReceiptStyle::Rule);
    add(
        "Subtotal".into(),
        Some(rupiah(sale.subtotal)),
        ReceiptStyle::Body,
    );
    if sale.discount > 0.0 {
        add(
            "Diskon".into(),
            Some(rupiah(sale.discount)),
            ReceiptStyle::Body,
        );
    }
    if sale.shipping > 0.0 {
        add(
            "Ongkos".into(),
            Some(rupiah(sale.shipping)),
            ReceiptStyle::Body,
        );
    }
    add(
        "TOTAL".into(),
        Some(rupiah(sale.grand_total)),
        ReceiptStyle::Total,
    );
    add(
        if sale.payment_type == "KREDIT" {
            "Bayar"
        } else {
            "Tunai"
        }
        .into(),
        Some(rupiah(sale.paid_amount)),
        ReceiptStyle::Body,
    );
    if sale.payment_type == "KREDIT" {
        add(
            "Sisa:".into(),
            Some(rupiah(if settings.returned {
                0.0
            } else {
                (sale.grand_total - sale.paid_amount).max(0.0)
            })),
            ReceiptStyle::Body,
        );
        add(
            format!("Jatuh tempo: {}", sale.due_date.as_deref().unwrap_or("—")),
            None,
            ReceiptStyle::Body,
        );
    } else {
        add(
            "Kembali".into(),
            Some(rupiah(sale.change_amount)),
            ReceiptStyle::Body,
        );
    }
    add(String::new(), None, ReceiptStyle::Rule);
    if !settings.footer_note.trim().is_empty() {
        add(settings.footer_note.clone(), None, ReceiptStyle::Center);
    }
    rows
}

pub fn wrap_receipt_text(value: &str, columns: usize) -> Vec<String> {
    let columns = columns.max(1);
    let mut lines = Vec::new();
    for paragraph in value.lines() {
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let chars = word.chars().collect::<Vec<_>>();
            if !line.is_empty() && line.chars().count() + chars.len() + 1 > columns {
                lines.push(std::mem::take(&mut line));
            }
            for chunk in chars.chunks(columns) {
                if !line.is_empty() {
                    line.push(' ');
                }
                line.extend(chunk);
                if line.chars().count() >= columns {
                    lines.push(std::mem::take(&mut line));
                }
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    lines
}

pub fn generate_escpos_bytes(sale: &Sale, settings: &ReceiptSettings) -> Vec<u8> {
    let mut buf = Vec::from(&b"\x1B@"[..]);
    let columns = if settings.paper_width == 80 { 48 } else { 32 };
    for row in receipt_rows(sale, settings) {
        let center = matches!(row.style, ReceiptStyle::Header | ReceiptStyle::Center);
        buf.extend_from_slice(&[0x1b, b'a', if center { 1 } else { 0 }]);
        buf.extend_from_slice(&[
            0x1b,
            b'E',
            if matches!(
                row.style,
                ReceiptStyle::Header | ReceiptStyle::Bold | ReceiptStyle::Total
            ) {
                1
            } else {
                0
            },
        ]);
        buf.extend_from_slice(&[
            0x1d,
            b'!',
            if row.style == ReceiptStyle::Header {
                0x11
            } else if row.style == ReceiptStyle::Total {
                0x01
            } else {
                0
            },
        ]);
        if row.style == ReceiptStyle::Rule {
            buf.extend_from_slice("-".repeat(columns).as_bytes());
            buf.push(b'\n');
            continue;
        }
        let limit = if row.style == ReceiptStyle::Header {
            columns / 2
        } else {
            columns
        };
        let mut lines = wrap_receipt_text(&row.text, limit);
        if let Some(right) = row.right {
            let last = lines.pop().unwrap_or_default();
            if last.chars().count() + right.chars().count() + 1 > columns {
                lines.push(last);
                for line in lines {
                    buf.extend_from_slice(line.as_bytes());
                    buf.push(b'\n');
                }
                for line in wrap_receipt_text(&right, columns) {
                    buf.extend_from_slice(
                        " ".repeat(columns.saturating_sub(line.chars().count()))
                            .as_bytes(),
                    );
                    buf.extend_from_slice(line.as_bytes());
                    buf.push(b'\n');
                }
            } else {
                for line in lines {
                    buf.extend_from_slice(line.as_bytes());
                    buf.push(b'\n');
                }
                buf.extend_from_slice(last.as_bytes());
                buf.extend_from_slice(
                    " ".repeat(columns - last.chars().count() - right.chars().count())
                        .as_bytes(),
                );
                buf.extend_from_slice(right.as_bytes());
                buf.push(b'\n');
            }
        } else {
            for line in lines {
                buf.extend_from_slice(line.as_bytes());
                buf.push(b'\n');
            }
        }
    }
    buf.extend_from_slice(b"\x1bE\x00\x1d!\x00\n\n\n\x1dV\x42\x00");
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thermal_receipts_wrap_long_rows_align_amounts_and_keep_shared_content() {
        let mut sale = crate::db::demo::sales()[0].clone();
        sale.sale_date = "2026-11-25 16:10:41".into();
        sale.items[0].unit_name = "Satuan panjang àààààààààààààààààààààààààààà".into();
        sale.items[0].discount = 100.25;
        sale.discount = 0.0;
        sale.shipping = 0.0;
        let mut settings = ReceiptSettings::from_values(&crate::db::demo::settings());
        settings.store_name = "Apotek nama panjang untuk cetak".into();
        settings.footer_note = "Terima kasih\n\nSemoga sehat <3".into();
        for (paper, columns) in [(58, 32), (80, 48)] {
            settings.paper_width = paper;
            let raw = generate_escpos_bytes(&sale, &settings);
            assert!(raw.windows(3).any(|bytes| bytes == b"\x1d!\x11"));
            assert!(raw.windows(3).any(|bytes| bytes == b"\x1d!\x01"));
            let mut plain = Vec::new();
            let mut pos = 0;
            while pos < raw.len() {
                if raw[pos..].starts_with(b"\x1b@") {
                    pos += 2;
                } else if raw[pos..].starts_with(b"\x1ba")
                    || raw[pos..].starts_with(b"\x1bE")
                    || raw[pos..].starts_with(b"\x1d!")
                {
                    pos += 3;
                } else if raw[pos..].starts_with(b"\x1dV") {
                    break;
                } else {
                    plain.push(raw[pos]);
                    pos += 1;
                }
            }
            let plain = String::from_utf8(plain).unwrap();
            assert!(plain.contains("Terima kasih\n\nSemoga sehat <3\n"));
            assert!(plain.lines().all(|line| line.chars().count() <= columns));
            for value in [
                "BARANG",
                "JUMLAH",
                "25/11/2026 16:10:41",
                "Diskon barang",
                "Rp 100,25",
                "TOTAL",
                "Tunai",
                "Kembali",
            ] {
                assert!(plain.contains(value), "Missing {value}");
            }
            assert!(!plain.lines().any(|line| line.starts_with("Ongkos")));
            let rows = receipt_rows(&sale, &settings);
            assert!(rows
                .iter()
                .any(|row| row.style == ReceiptStyle::Total && row.text == "TOTAL"));
            assert_eq!(
                rows.iter()
                    .filter(|row| row.text == "Diskon barang")
                    .count(),
                1
            );
            assert!(!rows.iter().any(|row| row.text == "Diskon"));
        }
    }
}

#[cfg(windows)]
pub fn print_raw_to_spooler(printer_name: &str, raw_data: &[u8]) -> Result<(), String> {
    use std::ffi::CString;
    use std::ptr;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Graphics::Printing::*;

    let c_printer = CString::new(printer_name).map_err(|e| e.to_string())?;
    let mut h_printer: HANDLE = 0;

    unsafe {
        if OpenPrinterA(
            c_printer.as_ptr() as *mut u8,
            &mut h_printer,
            ptr::null_mut(),
        ) == 0
        {
            return Err(format!("Gagal membuka printer: {}", printer_name));
        }

        let doc_name = CString::new("Struk Apotek POS").unwrap();
        let data_type = CString::new("RAW").unwrap();
        let doc_info = DOC_INFO_1A {
            pDocName: doc_name.as_ptr() as *mut u8,
            pOutputFile: ptr::null_mut(),
            pDatatype: data_type.as_ptr() as *mut u8,
        };

        if StartDocPrinterA(h_printer, 1, &doc_info as *const _ as *mut _) == 0 {
            ClosePrinter(h_printer);
            return Err("Gagal StartDocPrinterA".to_string());
        }

        StartPagePrinter(h_printer);

        let mut bytes_written: u32 = 0;
        let success = WritePrinter(
            h_printer,
            raw_data.as_ptr() as *const _ as *mut _,
            raw_data.len() as u32,
            &mut bytes_written,
        );

        EndPagePrinter(h_printer);
        EndDocPrinterA(h_printer);
        ClosePrinter(h_printer);

        if success == 0 {
            return Err("Gagal menulis ke WritePrinter".to_string());
        }
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn print_raw_to_spooler(printer_name: &str, raw_data: &[u8]) -> Result<(), String> {
    println!(
        "[SIMULASI CETAK STRUK - Printer: {}] ({} bytes)",
        printer_name,
        raw_data.len()
    );
    // Simpan ke struk_preview.bin atau tampilkan teks yang bisa dibaca
    let clean_text = String::from_utf8_lossy(raw_data)
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ' || *c == '\n' || *c == '\r')
        .collect::<String>();
    println!("--- ISI STRUK ---\n{}\n-----------------", clean_text);
    Ok(())
}
