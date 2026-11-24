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

pub fn generate_escpos_bytes(sale: &Sale, settings: &ReceiptSettings) -> Vec<u8> {
    let mut buf = Vec::new();

    // 1. Inisialisasi printer
    buf.extend_from_slice(b"\x1B@"); // ESC @
    if settings.returned {
        buf.extend_from_slice(b"FAKTUR DIRETUR (salinan)\n");
    }

    // 2. Center Align & Double Size Toko
    buf.extend_from_slice(b"\x1Ba\x01"); // Center
    buf.extend_from_slice(b"\x1D!\x11"); // Double width & height
    buf.extend_from_slice(settings.store_name.as_bytes());
    buf.extend_from_slice(b"\n");

    // Normal Font
    buf.extend_from_slice(b"\x1D!\x00"); // Normal
    buf.extend_from_slice(settings.address.as_bytes());
    buf.extend_from_slice(b"\n");
    buf.extend_from_slice(settings.phone.as_bytes());
    buf.extend_from_slice(b"\n");
    buf.extend_from_slice(settings.header_note.as_bytes());
    buf.extend_from_slice(b"\n");

    let cols = if settings.paper_width == 80 { 48 } else { 32 };
    let separator = "-".repeat(cols);

    buf.extend_from_slice(separator.as_bytes());
    buf.extend_from_slice(b"\n");

    // 3. Info Nota & Kasir (Left Align)
    buf.extend_from_slice(b"\x1Ba\x00"); // Left align
    let meta = format!("No: {}\nTgl: {}\n", sale.invoice_no, sale.sale_date);
    buf.extend_from_slice(meta.as_bytes());
    buf.extend_from_slice(
        format!(
            "Pelanggan: {}\nOperator: {}\n",
            settings.customer_name, settings.operator_name
        )
        .as_bytes(),
    );
    buf.extend_from_slice(separator.as_bytes());
    buf.extend_from_slice(b"\n");

    // 4. Daftar Item Belanja
    for item in &sale.items {
        let name_line = format!("{}\n", item.product_name);
        buf.extend_from_slice(name_line.as_bytes());

        let detail_left = format!(
            " {}x {} @{}",
            item.qty,
            item.unit_name,
            crate::gui::views::components::number(item.sell_price)
        );
        let detail_right = crate::gui::views::components::number(item.subtotal);
        let spaces_count = cols.saturating_sub(detail_left.len() + detail_right.len());
        let item_line = format!(
            "{}{}{}\n",
            detail_left,
            " ".repeat(spaces_count),
            detail_right
        );
        buf.extend_from_slice(item_line.as_bytes());
    }

    buf.extend_from_slice(separator.as_bytes());
    buf.extend_from_slice(b"\n");

    // 5. Total, Bayar, Kembali
    let print_row = |buf: &mut Vec<u8>, label: &str, amount: f64| {
        let val_str = crate::gui::views::components::rupiah(amount);
        let sp_count = cols.saturating_sub(label.len() + val_str.len());
        let row_line = format!("{}{}{}\n", label, " ".repeat(sp_count), val_str);
        buf.extend_from_slice(row_line.as_bytes());
    };

    print_row(&mut buf, "Subtotal:", sale.subtotal);
    if sale.discount > 0.0 {
        print_row(&mut buf, "Diskon:", sale.discount);
    }
    if sale.shipping > 0.0 {
        print_row(&mut buf, "Ongkos:", sale.shipping);
    }
    print_row(&mut buf, "Total:", sale.grand_total);
    print_row(&mut buf, "Bayar:", sale.paid_amount);
    if sale.payment_type == "KREDIT" {
        print_row(
            &mut buf,
            "Sisa:",
            if settings.returned {
                0.0
            } else {
                (sale.grand_total - sale.paid_amount).max(0.0)
            },
        );
        if let Some(due) = &sale.due_date {
            buf.extend_from_slice(format!("Jatuh tempo: {due}\n").as_bytes());
        }
    } else {
        print_row(&mut buf, "Kembali:", sale.change_amount);
    }

    buf.extend_from_slice(separator.as_bytes());
    buf.extend_from_slice(b"\n");

    // 6. Footer (Center)
    buf.extend_from_slice(b"\x1Ba\x01"); // Center
    for line in settings.footer_note.lines() {
        buf.extend_from_slice(line.as_bytes());
        buf.extend_from_slice(b"\n");
    }

    // Feed baris & Potong kertas
    buf.extend_from_slice(b"\n\n\n");
    buf.extend_from_slice(b"\x1DV\x42\x00"); // GS V 66 0 (Cut paper)

    buf
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
