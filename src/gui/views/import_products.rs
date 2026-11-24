use super::components::*;
use crate::{
    db::{self, NewProductInput},
    gui::theme::AppTheme as T,
    services,
};
use fltk::{misc::Progress, prelude::*, window::Window};
use mysql::Pool;
use std::{cell::RefCell, rc::Rc};

enum ImportMessage {
    Progress(usize, usize),
    Finished(Result<usize, String>),
}

pub fn show(pool: Pool, updated: impl FnMut() + 'static) {
    let mut win = Window::default()
        .with_size(640, 340)
        .with_label("Impor obat");
    win.set_color(T::BG_APP);
    text(24, 20, 592, 30, "Impor master obat", 22, true);
    text(
        24,
        60,
        592,
        38,
        "Unduh template, isi data obat, lalu pilih berkas. Stok masuk dicatat melalui pembelian.",
        12,
        false,
    );
    let mut template = button(24, 116, 228, "Unduh template Excel", false);
    let mut file = button(268, 116, 164, "Pilih file Excel", false);
    let mut start = button(448, 116, 168, "Mulai impor", true);
    let mut caption = text(24, 166, 592, 26, "Belum ada berkas dipilih", 12, false);
    let mut progress = Progress::new(24, 210, 592, 28, None);
    progress.set_minimum(0.0);
    progress.set_maximum(100.0);
    progress.set_value(0.0);
    progress.set_selection_color(T::PRIMARY);
    let mut status = text(24, 258, 592, 48, "", 12, false);
    let path = Rc::new(RefCell::new(None::<std::path::PathBuf>));
    let mut st = status.clone();
    template.set_callback(move |_| {
        if let Some(path) = save_path("Template impor obat", "Excel\t*.xlsx") {
            match services::export_products_to_excel(&[], path) {
                Ok(()) => st.set_label("Template disimpan"),
                Err(e) => st.set_label(&e),
            }
        }
    });
    let p = path.clone();
    file.set_callback(move |_| {
        let mut chooser =
            fltk::dialog::NativeFileChooser::new(fltk::dialog::FileDialogType::BrowseFile);
        chooser.set_filter("Excel\t*.{xlsx,xls}");
        chooser.show();
        let file = chooser.filename();
        if !file.as_os_str().is_empty() {
            caption.set_label(&file.to_string_lossy());
            *p.borrow_mut() = Some(file);
        }
    });
    let updated = Rc::new(RefCell::new(updated));
    let w = win.clone();
    start.set_callback(move |button| {
        let Some(path) = path.borrow().clone() else {
            status.set_label("Pilih file Excel terlebih dahulu");
            return;
        };
        button.deactivate();
        file.deactivate();
        template.deactivate();
        let mut modal = w.clone();
        modal.set_callback(|_| {});
        let (tx, rx) = std::sync::mpsc::channel();
        let p = pool.clone();
        status.set_label("Membaca berkas Excel...");
        progress.set_value(0.0);
        progress.set_label("0%");
        std::thread::spawn(move || {
            let result = services::import_products_from_excel(path).and_then(|rows| {
                if rows.is_empty() {
                    return Err("Berkas tidak berisi obat".into());
                }
                Ok(rows
                    .into_iter()
                    .map(|r| {
                        let mut units = vec![(r.base_unit.clone(), 1, r.sell_price)];
                        if !r.purchase_unit.eq_ignore_ascii_case(&r.base_unit) {
                            units.push((
                                r.purchase_unit.clone(),
                                r.purchase_conversion,
                                r.sell_price * r.purchase_conversion as f64,
                            ));
                        }
                        NewProductInput {
                            code: r.code,
                            name: r.name,
                            category: r.category,
                            supplier_id: None,
                            base_unit: r.base_unit,
                            purchase_unit: r.purchase_unit,
                            min_stock: r.min_stock,
                            buy_price: r.buy_price,
                            units,
                            batch_no: String::new(),
                            expired_date: None,
                            initial_stock: 0,
                        }
                    })
                    .collect::<Vec<_>>())
            });
            let result = result.and_then(|inputs| {
                let count = inputs.len();
                let sender = tx.clone();
                db::import_product_inputs_progress(&p, &inputs, move |n| {
                    let _ = sender.send(ImportMessage::Progress(n, count));
                })
                .map(|_| count)
                .map_err(|e| e.to_string())
            });
            let _ = tx.send(ImportMessage::Finished(result));
        });
        let mut progress = progress.clone();
        let mut status = status.clone();
        let mut w = w.clone();
        let mut button = button.clone();
        let mut file = file.clone();
        let mut template = template.clone();
        // One timer keeps the native event loop responsive while the transaction imports rows.
        let updated = updated.clone();
        let done = Rc::new(RefCell::new(false));
        let completed = done.clone();
        fltk::app::add_timeout3(0.05, move |handle| {
            loop {
                let message = match rx.try_recv() {
                    Ok(message) => message,
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        if !*completed.borrow() {
                            progress.set_value(0.0);
                            progress.set_label("0%");
                            status.set_label(
                                "Proses impor terhenti. Periksa data sebelum mencoba ulang.",
                            );
                            *completed.borrow_mut() = true;
                        }
                        break;
                    }
                };
                match message {
                    ImportMessage::Progress(n, count) => {
                        progress.set_value(n as f64 / count as f64 * 100.0);
                        progress.set_label(&format!("{:.0}%", n as f64 / count as f64 * 100.0));
                        status.set_label(&format!(
                            "{n}/{count} baris diproses. Menunggu transaksi selesai."
                        ));
                    }
                    ImportMessage::Finished(Ok(count)) => {
                        progress.set_value(100.0);
                        progress.set_label("100%");
                        status.set_label(&format!("{count} obat diimpor"));
                        (updated.borrow_mut())();
                        *completed.borrow_mut() = true;
                    }
                    ImportMessage::Finished(Err(e)) => {
                        progress.set_value(0.0);
                        progress.set_label("0%");
                        status.set_label(&format!("Impor dibatalkan: {e}"));
                        *completed.borrow_mut() = true;
                    }
                }
            }
            if *completed.borrow() {
                w.set_callback(close_modal);
                button.activate();
                file.activate();
                template.activate();
            } else {
                fltk::app::repeat_timeout3(0.05, handle);
            }
        });
    });
    win.end();
    win.make_modal(true);
    win.set_callback(close_modal);
    win.show();
}
