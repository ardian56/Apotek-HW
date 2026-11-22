use super::{
    components::*,
    kasir::{KasirView, MockCartRow},
    pembelian::{PembelianItemRow, PembelianView},
};
use crate::{
    db::{
        self,
        operations::{self, Purchase, PurchaseLine},
    },
    gui::theme::AppTheme as T,
    models::{Product, Sale, SaleItem, User},
};
use fltk::{
    enums::{CallbackTrigger, Event, FrameType, Key},
    frame::Frame,
    menu::Choice,
    prelude::*,
    window::Window,
};
use mysql::Pool;
use std::{cell::RefCell, rc::Rc};

fn item_dialog(
    pool: Pool,
    purchase: bool,
    query: &str,
    mut add: impl FnMut(PurchaseLine) -> Result<(), String> + 'static,
) {
    let mut win = Window::default()
        .with_size(700, 492)
        .with_label(if purchase {
            "Tambah barang pembelian"
        } else {
            "Tambah obat penjualan"
        });
    win.set_color(T::BG_APP);
    let mut search = input(24, 20, 480, "Cari nama / barcode", query);
    search.set_trigger(CallbackTrigger::EnterKey);
    let mut find = button(520, 44, 156, "Cari", true);
    text(24, 100, 640, 20, "Obat", 12, false);
    let mut product = Choice::new(24, 126, 652, 34, None);
    product.set_frame(FrameType::BorderBox);
    text(24, 184, 180, 18, "Satuan", 12, false);
    let mut unit = Choice::new(24, 208, 180, 34, None);
    unit.set_frame(FrameType::BorderBox);
    let qty = input(224, 184, 100, "Jumlah", "1");
    let price = input(
        344,
        184,
        170,
        if purchase {
            "Harga beli (Rp)"
        } else {
            "Harga jual (Rp)"
        },
        "0",
    );
    let discount = input(534, 184, 142, "Diskon (%)", "0");
    let batch = input(24, 268, 300, "Nomor batch", "");
    let expiry = input(348, 268, 328, "Kedaluwarsa (YYYY-MM-DD)", "");
    if !purchase {
        let mut b = batch.clone();
        b.hide();
        let mut e = expiry.clone();
        e.hide();
    }
    let mut status = text(24, 350, 652, 40, "", 12, false);
    let mut save = button(24, 432, 200, "Tambah ke keranjang", true);
    let mut cancel = button(240, 432, 120, "Batal", false);
    let products = Rc::new(RefCell::new(Vec::<Product>::new()));
    let select = {
        let ps = products.clone();
        let p = product.clone();
        let mut u = unit.clone();
        let mut price = price.clone();
        move || {
            u.clear();
            if let Some(product) = ps.borrow().get(p.value().max(0) as usize) {
                for unit in &product.units {
                    u.add_choice(&unit.unit_name.replace('|', " "));
                }
                u.set_value(0);
                if let Some(unit) = product.units.first() {
                    price.set_value(&format!(
                        "{:.2}",
                        if purchase {
                            product.buy_price * unit.conversion_qty as f64
                        } else {
                            unit.sell_price
                        }
                    ));
                }
            }
        }
    };
    let load = {
        let pool = pool.clone();
        let ps = products.clone();
        let mut p = product.clone();
        let s = search.clone();
        let mut st = status.clone();
        let mut select = select.clone();
        move || match db::get_products(&pool, Some(&s.value())) {
            Ok(data) => {
                p.clear();
                for product in &data {
                    p.add_choice(
                        &format!("{} — {}", product.code, product.name)
                            .replace('|', " ")
                            .replace('/', " "),
                    );
                }
                *ps.borrow_mut() = data;
                p.set_value(0);
                select();
                st.set_label("Pilih obat dan satuan. Pencarian menampilkan maksimum 200 obat.");
            }
            Err(e) => st.set_label(&format!("Pencarian gagal: {e}")),
        }
    };
    let mut l = load.clone();
    find.set_callback(move |_| l());
    let mut l = load.clone();
    search.set_callback(move |_| l());
    let mut s = select;
    product.set_callback(move |_| s());
    let ps = products.clone();
    let p = product.clone();
    let mut pr = price.clone();
    unit.set_callback(move |u| {
        if let Some(p) = ps.borrow().get(p.value().max(0) as usize) {
            if let Some(unit) = p.units.get(u.value().max(0) as usize) {
                pr.set_value(&format!(
                    "{:.2}",
                    if purchase {
                        p.buy_price * unit.conversion_qty as f64
                    } else {
                        unit.sell_price
                    }
                ));
            }
        }
    });
    let mut w = win.clone();
    cancel.set_callback(move |_| w.hide());
    let mut w = win.clone();
    save.set_callback(move |_| {
        let result = (|| -> Result<(), String> {
            let ps = products.borrow();
            let p = ps
                .get(product.value().max(0) as usize)
                .ok_or("Pilih obat")?;
            let u = p
                .units
                .get(unit.value().max(0) as usize)
                .ok_or("Obat belum memiliki satuan jual")?;
            let line = PurchaseLine {
                product_id: p.id,
                code: p.code.clone(),
                name: p.name.clone(),
                unit: u.unit_name.clone(),
                conversion: u.conversion_qty,
                qty: operations::quantity(&qty.value())?,
                price: operations::money(&price.value())?,
                discount: operations::money(&discount.value())?,
                batch: if purchase {
                    batch.value()
                } else {
                    "PENJUALAN".into()
                },
                expiry: if purchase && !expiry.value().trim().is_empty() {
                    Some(operations::date(&expiry.value())?)
                } else {
                    None
                },
            };
            line.validate()?;
            if !purchase {
                let today = chrono::Local::now().date_naive().to_string();
                let available = p
                    .batches
                    .iter()
                    .filter(|b| {
                        b.expired_date
                            .as_deref()
                            .map(|d| d > today.as_str())
                            .unwrap_or(true)
                    })
                    .map(|b| b.stock as i64)
                    .sum::<i64>();
                if (line.qty as i64 * line.conversion as i64) > available {
                    return Err("Stok layak jual tidak cukup".into());
                }
                if line.price * (1.0 - line.discount / 100.0) / (line.conversion as f64)
                    < p.buy_price
                {
                    return Err("Harga setelah diskon di bawah modal. Periksa harga jual.".into());
                }
            }
            add(line)
        })();
        match result {
            Ok(()) => w.hide(),
            Err(e) => status.set_label(&e),
        }
    });
    let mut l = load;
    l();
    win.end();
    win.make_modal(true);
    win.show();
}

fn choose_pending(
    pool: &Pool,
    operator: i32,
    purchase: bool,
    mut open: impl FnMut(i32, String) -> Result<(), String> + 'static,
) -> Result<(), String> {
    let data = operations::pending(pool, operator, purchase)?;
    let mut win = Window::default()
        .with_size(640, 300)
        .with_label("Transaksi pending");
    win.set_color(T::BG_APP);
    text(24, 20, 592, 28, "Pilih transaksi pending", 20, true);
    let mut choice = Choice::new(24, 80, 592, 34, None);
    for (id, payload) in &data {
        let description = if purchase {
            serde_json::from_str::<Purchase>(payload)
                .map(|p| format!("{} • {} barang", p.invoice, p.lines.len()))
                .unwrap_or_else(|_| "Data tidak terbaca".into())
        } else {
            serde_json::from_str::<Sale>(payload)
                .map(|p| format!("{} • Rp {:.2}", p.invoice_no, p.subtotal))
                .unwrap_or_else(|_| "Data tidak terbaca".into())
        };
        choice.add_choice(&format!("#{id} • {description}"));
    }
    choice.set_value(0);
    let mut status = text(
        24,
        132,
        592,
        54,
        if data.is_empty() {
            "Belum ada transaksi pending"
        } else {
            "Pending tetap tersimpan sampai transaksi berhasil disimpan."
        },
        12,
        false,
    );
    let mut restore = button(24, 218, 200, "Buka transaksi", true);
    if data.is_empty() {
        restore.deactivate();
    }
    let mut w = win.clone();
    restore.set_callback(move |_| {
        if let Some((id, payload)) = data.get(choice.value().max(0) as usize) {
            match open(*id, payload.clone()) {
                Ok(()) => w.hide(),
                Err(e) => status.set_label(&e),
            }
        }
    });
    win.end();
    win.make_modal(true);
    win.show();
    Ok(())
}

pub fn bind_cashier(view: &mut KasirView, pool: Option<Pool>, user: User) {
    let items = Rc::new(RefCell::new(Vec::<SaleItem>::new()));
    let invoice = Rc::new(RefCell::new(String::new()));
    let last = Rc::new(RefCell::new(None::<Sale>));
    let pending_id = Rc::new(RefCell::new(None::<i32>));
    view.rows_data.borrow_mut().clear();
    view.cart_table.set_rows(0);
    view.input_barcode.set_value("");
    view.input_qty.set_value("1");
    view.lbl_grand_total.set_label("Rp 0");
    view.input_bayar.set_value("0");
    view.lbl_kembali.set_label("Rp 0");
    view.lbl_network_status.set_label(if pool.is_some() {
        "Database terhubung"
    } else {
        "Pratinjau • Tanpa database"
    });
    view.lbl_operator
        .set_label(&format!("Operator: {}", user.full_name));
    view.lbl_warning_rugi.resize(24, 658, 970, 20);
    view.lbl_warning_rugi.set_label("");
    view.choice_unit.deactivate();
    view.input_qty.deactivate();
    view.win.begin();
    let mut payment = Choice::new(350, 500, 210, 32, None);
    payment.add_choice("Tunai|Kredit");
    payment.set_value(0);
    let mut customer = Choice::new(350, 544, 230, 30, None);
    customer.add_choice("Pelanggan umum");
    customer.set_value(0);
    let customers = pool
        .as_ref()
        .map(db::get_customers)
        .transpose()
        .ok()
        .flatten()
        .unwrap_or_default();
    for (_, name, _, _) in &customers {
        customer.add_choice(&name.replace('|', " ").replace('/', " "));
    }
    let mut due = fltk::input::Input::new(350, 578, 230, 28, None);
    due.set_value(&(chrono::Local::now().date_naive() + chrono::Duration::days(30)).to_string());
    due.set_tooltip("Jatuh tempo kredit (YYYY-MM-DD)");
    due.hide();
    let mut credit_date = due.clone();
    payment.set_callback(move |p| {
        if p.value() == 1 {
            credit_date.show();
        } else {
            credit_date.hide();
        }
    });
    let mut remove = button(790, 616, 196, "Hapus barang terpilih", false);
    view.win.end();
    let refresh = {
        let data = items.clone();
        let rows = view.rows_data.clone();
        let mut table = view.cart_table.clone();
        let mut total = view.lbl_grand_total.clone();
        let paid = view.input_bayar.clone();
        let mut change = view.lbl_kembali.clone();
        move || {
            *rows.borrow_mut() = data
                .borrow()
                .iter()
                .enumerate()
                .map(|(i, line)| MockCartRow {
                    no: i as i32 + 1,
                    barcode: line.product_code.clone(),
                    name: line.product_name.clone(),
                    unit: line.unit_name.clone(),
                    qty: line.qty,
                    price: line.sell_price,
                    subtotal: line.subtotal,
                })
                .collect();
            table.set_rows(data.borrow().len() as i32);
            table.redraw();
            let sum = data
                .borrow()
                .iter()
                .map(|i| i.subtotal)
                .sum::<f64>()
                .max(0.0);
            total.set_label(&format!("Rp {sum:.2}"));
            change.set_label(&format!(
                "Rp {:.2}",
                (operations::money(&paid.value()).unwrap_or(0.0) - sum).max(0.0)
            ));
        }
    };
    view.input_bayar.set_trigger(CallbackTrigger::Changed);
    let mut r = refresh.clone();
    view.input_bayar.set_callback(move |_| r());
    let data = items.clone();
    let p = pool.clone();
    let query = view.input_barcode.clone();
    let mut st = view.lbl_warning_rugi.clone();
    let r = refresh.clone();
    view.btn_add_item.set_callback(move |_| {
        let Some(p) = &p else {
            st.set_label("Hubungkan database terlebih dahulu");
            return;
        };
        let data = data.clone();
        let mut r = r.clone();
        item_dialog(p.clone(), false, &query.value(), move |line| {
            let subtotal = line.total();
            data.borrow_mut().push(SaleItem {
                id: 0,
                sale_id: 0,
                product_id: line.product_id,
                product_code: line.code,
                product_name: line.name,
                unit_name: line.unit,
                qty: line.qty,
                conversion_qty: line.conversion,
                qty_in_base: line.qty * line.conversion,
                sell_price: line.price,
                discount: line.price * line.qty as f64 * line.discount / 100.0,
                subtotal,
                batch_allocations: vec![],
            });
            r();
            Ok(())
        });
    });
    let mut add = view.btn_add_item.clone();
    view.input_barcode.set_trigger(CallbackTrigger::EnterKey);
    view.input_barcode.set_callback(move |_| add.do_callback());
    let data = items.clone();
    let t = view.cart_table.clone();
    let mut r = refresh.clone();
    remove.set_callback(move |_| {
        let (row, _, _, _) = t.get_selection();
        if row >= 0 && (row as usize) < data.borrow().len() {
            data.borrow_mut().remove(row as usize);
            r();
        }
    });
    let clear = {
        let invoice = invoice.clone();
        let data = items.clone();
        let mut paid = view.input_bayar.clone();
        let mut r = refresh.clone();
        let mut st = view.lbl_warning_rugi.clone();
        let mut c = customer.clone();
        let mut pt = payment.clone();
        let pending = pending_id.clone();
        let last = last.clone();
        move || {
            data.borrow_mut().clear();
            paid.set_value("0");
            c.set_value(0);
            pt.set_value(0);
            pt.do_callback();
            *pending.borrow_mut() = None;
            *last.borrow_mut() = None;
            st.set_label("");
            r();
            invoice.borrow_mut().clear();
        }
    };
    let data = items.clone();
    let mut c = clear.clone();
    view.btn_baru.set_callback(move |_| {
        if data.borrow().is_empty()
            || fltk::dialog::choice2_default(
                "Bersihkan keranjang yang belum disimpan?",
                "Batal",
                "Bersihkan",
                "",
            ) == Some(1)
        {
            c();
        }
    });
    let customer_ids = customers.iter().map(|r| r.0).collect::<Vec<_>>();
    let build = {
        let invoice = invoice.clone();
        let customers = customers.clone();
        let data = items.clone();
        let paid = view.input_bayar.clone();
        let pt = payment.clone();
        let c = customer.clone();
        let due = due.clone();
        move || -> Result<Sale, String> {
            let subtotal = data
                .borrow()
                .iter()
                .map(|i| i.subtotal)
                .sum::<f64>()
                .max(0.0);
            let paid = operations::money(&paid.value())?;
            let credit = pt.value() == 1;
            let customer = if c.value() > 0 {
                customers.get((c.value() - 1) as usize).map(|r| r.0)
            } else {
                None
            };
            if invoice.borrow().is_empty() {
                *invoice.borrow_mut() =
                    format!("PJ-{}-{}", chrono::Utc::now().timestamp_micros(), user.id);
            }
            Ok(Sale {
                id: 0,
                invoice_no: invoice.borrow().clone(),
                sale_date: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                payment_type: if credit { "KREDIT" } else { "TUNAI" }.into(),
                due_date: if credit {
                    Some(operations::date(&due.value())?)
                } else {
                    None
                },
                customer_id: customer,
                doctor_id: None,
                operator_id: user.id,
                subtotal,
                discount: 0.0,
                grand_total: subtotal,
                paid_amount: paid,
                change_amount: if credit {
                    0.0
                } else {
                    (paid - subtotal).max(0.0)
                },
                total_profit: 0.0,
                items: data.borrow().clone(),
            })
        }
    };
    let mut paid_after_save = view.input_bayar.clone();
    let current_invoice = invoice.clone();
    let p = pool.clone();
    let build_save = build.clone();
    let data = items.clone();
    let saved = last.clone();
    let pending = pending_id.clone();
    let mut r = refresh.clone();
    let mut st = view.lbl_warning_rugi.clone();
    view.btn_simpan.set_callback(move |_| {
        let Some(p) = &p else {
            return;
        };
        match build_save().and_then(|sale| {
            db::execute_sale_transaction_fefo(p, sale.clone())
                .map(|_| sale)
                .map_err(|e| e.to_string())
        }) {
            Ok(sale) => {
                let invoice = sale.invoice_no.clone();
                *saved.borrow_mut() = Some(sale);
                data.borrow_mut().clear();
                paid_after_save.set_value("0");
                r();
                st.set_label(&format!("Tersimpan: {invoice}"));
                current_invoice.borrow_mut().clear();
                if let Some(id) = pending.borrow_mut().take() {
                    if let Err(e) = operations::delete_pending(p, user.id, false, id) {
                        st.set_label(&format!("Faktur tersimpan. Pending gagal dihapus: {e}"));
                    }
                }
            }
            Err(e) => st.set_label(&format!("Simpan gagal: {e}")),
        }
    });
    let p = pool.clone();
    let mut st = view.lbl_warning_rugi.clone();
    view.btn_pending.set_callback(move |_| {
        let Some(p) = &p else {
            return;
        };
        let result = build()
            .and_then(|sale| {
                if sale.items.is_empty() {
                    return Err("Keranjang kosong".into());
                }
                serde_json::to_string(&sale).map_err(|e| e.to_string())
            })
            .and_then(|payload| operations::save_pending(p, user.id, false, &payload));
        st.set_label(&match result {
            Ok(()) => "Transaksi disimpan sebagai pending".into(),
            Err(e) => format!("Pending gagal: {e}"),
        });
    });
    let p = pool.clone();
    let data = items.clone();
    let pending = pending_id.clone();
    let paid = view.input_bayar.clone();
    let mut st = view.lbl_warning_rugi.clone();
    let r = refresh.clone();
    view.btn_buka_pending.set_callback(move |_| {
        let Some(p) = &p else {
            return;
        };
        if !data.borrow().is_empty() {
            st.set_label("Simpan atau bersihkan keranjang sebelum membuka pending");
            return;
        }
        let invoice = invoice.clone();
        let customer_ids = customer_ids.clone();
        let data = data.clone();
        let pending = pending.clone();
        let mut paid = paid.clone();
        let mut r = r.clone();
        let mut c = customer.clone();
        let mut pt = payment.clone();
        let mut d = due.clone();
        if let Err(e) = choose_pending(p, user.id, false, move |id, payload| {
            let sale: Sale = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
            *invoice.borrow_mut() = sale.invoice_no.clone();
            *data.borrow_mut() = sale.items;
            *pending.borrow_mut() = Some(id);
            paid.set_value(&sale.paid_amount.to_string());
            pt.set_value(if sale.payment_type == "KREDIT" { 1 } else { 0 });
            pt.do_callback();
            c.set_value(
                sale.customer_id
                    .and_then(|id| customer_ids.iter().position(|v| *v == id))
                    .map(|i| i as i32 + 1)
                    .unwrap_or(0),
            );
            if let Some(due) = sale.due_date {
                d.set_value(&due);
            }
            r();
            Ok(())
        }) {
            st.set_label(&e);
        }
    });
    let p = pool.clone();
    let saved = last.clone();
    let mut st = view.lbl_warning_rugi.clone();
    view.btn_cetak.set_callback(move |_| {
        let Some(sale) = saved.borrow().clone() else {
            st.set_label("Simpan transaksi sebelum mencetak struk");
            return;
        };
        let values = p
            .as_ref()
            .map(operations::settings)
            .transpose()
            .ok()
            .flatten()
            .unwrap_or_default();
        let settings = crate::hardware::ReceiptSettings {
            store_name: values
                .get("store_name")
                .cloned()
                .unwrap_or_else(|| "Apotek".into()),
            address: values.get("store_address").cloned().unwrap_or_default(),
            phone: values.get("store_phone").cloned().unwrap_or_default(),
            header_note: values.get("receipt_header").cloned().unwrap_or_default(),
            footer_note: values.get("receipt_footer").cloned().unwrap_or_default(),
            paper_width: values
                .get("receipt_paper_width")
                .and_then(|s| s.parse().ok())
                .unwrap_or(58),
            printer_name: values.get("printer_name").cloned().unwrap_or_default(),
        };
        let bytes = crate::hardware::generate_escpos_bytes(&sale, &settings);
        if let Err(e) = crate::hardware::print_raw_to_spooler(&settings.printer_name, &bytes) {
            st.set_label(&format!("Cetak gagal: {e}"));
        }
    });
    if pool.is_none() {
        for b in [
            &mut view.btn_simpan,
            &mut view.btn_pending,
            &mut view.btn_buka_pending,
            &mut view.btn_cetak,
        ] {
            b.deactivate();
        }
    }
    let mut barcode = view.input_barcode.clone();
    let mut paid = view.input_bayar.clone();
    let mut buttons = [
        view.btn_simpan.clone(),
        view.btn_cetak.clone(),
        view.btn_baru.clone(),
        view.btn_pending.clone(),
        view.btn_buka_pending.clone(),
    ];
    view.win.handle(move |_, event| {
        if matches!(event, Event::KeyDown | Event::Shortcut) {
            let key = fltk::app::event_key();
            if key == Key::F1 {
                let _ = barcode.take_focus();
                return true;
            }
            if key == Key::F2 {
                let _ = paid.take_focus();
                return true;
            }
            for (i, k) in [Key::F3, Key::F4, Key::F5, Key::F6, Key::F7]
                .iter()
                .enumerate()
            {
                if key == *k {
                    if buttons[i].active() {
                        buttons[i].do_callback();
                    }
                    return true;
                }
            }
        }
        false
    });
}

pub fn bind_purchase(view: &mut PembelianView, pool: Option<Pool>, user: User) {
    let lines = Rc::new(RefCell::new(Vec::<PurchaseLine>::new()));
    let pending_id = Rc::new(RefCell::new(None::<i32>));
    view.items.borrow_mut().clear();
    view.table.set_rows(0);
    view.input_faktur.set_value("");
    view.input_tempo
        .set_value(&(chrono::Local::now().date_naive() + chrono::Duration::days(30)).to_string());
    view.input_tempo.resize(510, 38, 110, 30);
    view.input_tempo.set_tooltip("Jatuh tempo: YYYY-MM-DD");
    view.choice_suplier.clear();
    view.choice_suplier.add_choice("Pilih suplier");
    view.choice_suplier.set_value(0);
    let suppliers = pool
        .as_ref()
        .map(db::get_suppliers)
        .transpose()
        .ok()
        .flatten()
        .unwrap_or_default();
    for (_, name, _, _) in &suppliers {
        view.choice_suplier
            .add_choice(&name.replace('|', " ").replace('/', " "));
    }
    view.win.begin();
    let mut status = text(
        36,
        80,
        1110,
        26,
        if pool.is_none() {
            "Pratinjau • Hubungkan database untuk menyimpan pembelian"
        } else {
            "Isi faktur suplier dan tambahkan barang"
        },
        12,
        false,
    );
    let mut remove = button(896, 486, 268, "Hapus barang terpilih", false);
    let mut pending = button(896, 548, 130, "Pending", false);
    let mut restore = button(1034, 548, 130, "Buka pending", false);
    view.win.end();
    let refresh = {
        let disc = view.input_discount.clone();
        let tax = view.input_tax.clone();
        let data = lines.clone();
        let rows = view.items.clone();
        let mut table = view.table.clone();
        let mut sub = view.lbl_subtotal.clone();
        let mut total = view.lbl_grand_total.clone();
        move || {
            *rows.borrow_mut() = data
                .borrow()
                .iter()
                .enumerate()
                .map(|(i, line)| PembelianItemRow {
                    no: i as i32 + 1,
                    barcode: line.code.clone(),
                    nama: line.name.clone(),
                    satuan: line.unit.clone(),
                    harga_beli: line.price,
                    qty: line.qty,
                    diskon_persen: line.discount,
                    total: line.total(),
                })
                .collect();
            table.set_rows(data.borrow().len() as i32);
            table.redraw();
            let sum = data
                .borrow()
                .iter()
                .map(PurchaseLine::total)
                .sum::<f64>()
                .max(0.0);
            sub.set_label(&format!("Rp {sum:.2}"));
            let result = (sum - operations::money(&disc.value()).unwrap_or(0.0))
                * (1.0 + operations::money(&tax.value()).unwrap_or(0.0) / 100.0);
            total.set_label(&format!("Rp {result:.2}"));
        }
    };
    let mut r = refresh.clone();
    r();
    let mut r = refresh.clone();
    view.input_discount.set_trigger(CallbackTrigger::Changed);
    view.input_discount.set_callback(move |_| r());
    let mut r = refresh.clone();
    view.input_tax.set_trigger(CallbackTrigger::Changed);
    view.input_tax.set_callback(move |_| r());
    let p = pool.clone();
    let data = lines.clone();
    let r = refresh.clone();
    view.btn_tambah_item.set_callback(move |_| {
        if let Some(p) = &p {
            let data = data.clone();
            let mut r = r.clone();
            item_dialog(p.clone(), true, "", move |line| {
                data.borrow_mut().push(line);
                r();
                Ok(())
            });
        }
    });
    let data = lines.clone();
    let t = view.table.clone();
    let mut r = refresh.clone();
    remove.set_callback(move |_| {
        let (row, _, _, _) = t.get_selection();
        if row >= 0 && (row as usize) < data.borrow().len() {
            data.borrow_mut().remove(row as usize);
            r();
        }
    });
    let supplier_ids = suppliers.iter().map(|r| r.0).collect::<Vec<_>>();
    let build = {
        let disc = view.input_discount.clone();
        let tax = view.input_tax.clone();
        let data = lines.clone();
        let invoice = view.input_faktur.clone();
        let supplier = view.choice_suplier.clone();
        let payment = view.choice_bayar.clone();
        let due = view.input_tempo.clone();
        let dt = view.input_tgl.clone();
        move || -> Result<Purchase, String> {
            let supplier = suppliers
                .get(supplier.value().checked_sub(1).unwrap_or(-1) as usize)
                .ok_or("Pilih suplier")?
                .0;
            let credit = payment.value() == 1;
            Ok(Purchase {
                invoice: invoice.value().trim().to_string(),
                purchase_date: operations::date(&dt.value())?,
                supplier,
                operator: user.id,
                credit,
                due: if credit {
                    Some(operations::date(&due.value())?)
                } else {
                    None
                },
                discount: operations::money(&disc.value())?,
                tax_percent: operations::money(&tax.value())?,
                lines: data.borrow().clone(),
            })
        }
    };
    let clear = {
        let mut disc = view.input_discount.clone();
        let mut tax = view.input_tax.clone();
        let data = lines.clone();
        let mut invoice = view.input_faktur.clone();
        let mut r = refresh.clone();
        let pid = pending_id.clone();
        move || {
            data.borrow_mut().clear();
            invoice.set_value("");
            disc.set_value("0");
            tax.set_value("0");
            *pid.borrow_mut() = None;
            r();
        }
    };
    let data = lines.clone();
    let mut c = clear.clone();
    view.btn_baru.set_callback(move |_| {
        if data.borrow().is_empty()
            || fltk::dialog::choice2_default(
                "Bersihkan pembelian yang belum disimpan?",
                "Batal",
                "Bersihkan",
                "",
            ) == Some(1)
        {
            c();
        }
    });
    let p = pool.clone();
    let b = build.clone();
    let mut c = clear.clone();
    let pid = pending_id.clone();
    let mut st = status.clone();
    view.btn_simpan.set_callback(move |_| {
        let Some(p) = &p else {
            return;
        };
        match b()
            .and_then(|purchase| operations::save_purchase(p, &purchase).map(|_| purchase.invoice))
        {
            Ok(invoice) => {
                let id = pid.borrow_mut().take();
                c();
                st.set_label(&format!("Faktur {invoice} tersimpan"));
                if let Some(id) = id {
                    if let Err(e) = operations::delete_pending(p, user.id, true, id) {
                        st.set_label(&format!("Faktur tersimpan. Pending gagal dihapus: {e}"));
                    }
                }
            }
            Err(e) => st.set_label(&format!("Simpan gagal: {e}")),
        }
    });
    let p = pool.clone();
    let mut st = status.clone();
    pending.set_callback(move |_| {
        let Some(p) = &p else {
            return;
        };
        let result = build()
            .and_then(|purchase| {
                if purchase.lines.is_empty() {
                    return Err("Keranjang kosong".into());
                }
                serde_json::to_string(&purchase).map_err(|e| e.to_string())
            })
            .and_then(|payload| operations::save_pending(p, user.id, true, &payload));
        st.set_label(&match result {
            Ok(()) => "Pembelian disimpan sebagai pending".into(),
            Err(e) => format!("Pending gagal: {e}"),
        });
    });
    let p = pool.clone();
    let data = lines.clone();
    let pid = pending_id.clone();
    let mut st = status.clone();
    let r = refresh.clone();
    let invoice = view.input_faktur.clone();
    let pt = view.choice_bayar.clone();
    let dt = view.input_tgl.clone();
    let due = view.input_tempo.clone();
    let supplier = view.choice_suplier.clone();
    let disc = view.input_discount.clone();
    let tax = view.input_tax.clone();
    restore.set_callback(move |_| {
        let Some(p) = &p else {
            return;
        };
        if !data.borrow().is_empty() {
            st.set_label("Simpan atau bersihkan keranjang sebelum membuka pending");
            return;
        }
        let mut disc = disc.clone();
        let mut tax = tax.clone();
        let supplier_ids = supplier_ids.clone();
        let mut supplier = supplier.clone();
        let data = data.clone();
        let pid = pid.clone();
        let mut r = r.clone();
        let mut invoice = invoice.clone();
        let mut pt = pt.clone();
        let mut dt = dt.clone();
        let mut due = due.clone();
        if let Err(e) = choose_pending(p, user.id, true, move |id, payload| {
            let purchase: Purchase = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
            disc.set_value(&purchase.discount.to_string());
            tax.set_value(&purchase.tax_percent.to_string());
            supplier.set_value(
                supplier_ids
                    .iter()
                    .position(|id| *id == purchase.supplier)
                    .map(|i| i as i32 + 1)
                    .unwrap_or(0),
            );
            *data.borrow_mut() = purchase.lines;
            *pid.borrow_mut() = Some(id);
            invoice.set_value(&purchase.invoice);
            pt.set_value(if purchase.credit { 1 } else { 0 });
            dt.set_value(&purchase.purchase_date);
            if let Some(value) = purchase.due {
                due.set_value(&value);
            }
            r();
            Ok(())
        }) {
            st.set_label(&e);
        }
    });
    view.btn_cetak.hide();
    if pool.is_none() {
        view.btn_simpan.deactivate();
        view.btn_tambah_item.deactivate();
        pending.deactivate();
        restore.deactivate();
    }
    let mut buttons = [
        view.btn_tambah_item.clone(),
        view.btn_simpan.clone(),
        view.btn_baru.clone(),
        pending.clone(),
        restore.clone(),
    ];
    view.win.handle(move |_, event| {
        if matches!(event, Event::KeyDown | Event::Shortcut) {
            let key = fltk::app::event_key();
            for (i, k) in [Key::F1, Key::F3, Key::F5, Key::F6, Key::F7]
                .iter()
                .enumerate()
            {
                if key == *k {
                    if buttons[i].active() {
                        buttons[i].do_callback();
                    }
                    return true;
                }
            }
        }
        false
    });
}
