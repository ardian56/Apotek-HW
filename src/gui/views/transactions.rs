use super::{components::*, kasir::KasirView, pembelian::PembelianView};
use crate::{
    db::{
        self,
        operations::{self, Purchase, PurchaseLine},
    },
    gui::theme::AppTheme as T,
    models::{Product, Sale, SaleItem, User},
};
use fltk::{
    enums::{CallbackTrigger, Event, Key},
    menu::Choice,
    prelude::*,
    window::Window,
};
use mysql::Pool;
use std::{cell::RefCell, rc::Rc};

fn item_dialog(
    pool: Option<Pool>,
    purchase: bool,
    query: &str,
    mut add: impl FnMut(Product, PurchaseLine) -> Result<(), String> + 'static,
) {
    let mut win = Window::default()
        .with_size(700, if purchase { 492 } else { 430 })
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
    style_choice(&mut product);
    text(24, 184, 180, 18, "Satuan", 12, false);
    let mut unit = Choice::new(24, 208, 180, 34, None);
    style_choice(&mut unit);
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
    let batch = purchase.then(|| input(24, 268, 300, "Nomor batch", ""));
    let expiry = purchase.then(|| input(348, 268, 328, "Kedaluwarsa (YYYY-MM-DD)", ""));
    let details = (!purchase).then(|| text(24, 270, 652, 44, "", 12, false));
    let mut status = text(24, if purchase { 350 } else { 330 }, 652, 34, "", 12, false);
    let button_y = if purchase { 432 } else { 374 };
    let mut save = button(24, button_y, 200, "Tambah ke keranjang", true);
    let mut cancel = button(240, button_y, 120, "Batal", false);
    let products = Rc::new(RefCell::new(Vec::<Product>::new()));
    let select = {
        let ps = products.clone();
        let p = product.clone();
        let mut u = unit.clone();
        let mut price = price.clone();
        let mut details = details.clone();
        move || {
            u.clear();
            price.set_value("0");
            if let Some(details) = &mut details {
                details.set_label("");
            }
            if let Some(product) = ps.borrow().get(p.value().max(0) as usize) {
                if let Some(details) = &mut details {
                    details.set_label(&format!(
                        "Stok layak jual: {} {}  •  ED terdekat: {}",
                        product.sellable_stock_on(&chrono::Local::now().date_naive().to_string()),
                        product.base_unit,
                        product.earliest_expiry().unwrap_or("—")
                    ));
                }
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
        move || match db::view_products(pool.as_ref(), Some(&s.value())) {
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
            Err(e) => {
                p.clear();
                ps.borrow_mut().clear();
                select();
                st.set_label(&format!("Pencarian gagal: {e}"));
            }
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
    cancel.set_callback(move |_| close_modal(&mut w));
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
                    batch.as_ref().map(|i| i.value()).unwrap_or_default()
                } else {
                    "PENJUALAN".into()
                },
                expiry: expiry
                    .as_ref()
                    .map(|i| i.value())
                    .filter(|v| !v.trim().is_empty())
                    .map(|v| operations::date(&v))
                    .transpose()?,
            };
            line.validate()?;
            add(p.clone(), line)
        })();
        match result {
            Ok(()) => close_modal(&mut w),
            Err(e) => status.set_label(&e),
        }
    });
    let mut l = load;
    l();
    win.end();
    win.make_modal(true);
    win.set_callback(close_modal);
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
        disable(&mut restore);
    }
    let mut w = win.clone();
    restore.set_callback(move |_| {
        if let Some((id, payload)) = data.get(choice.value().max(0) as usize) {
            match open(*id, payload.clone()) {
                Ok(()) => close_modal(&mut w),
                Err(e) => status.set_label(&e),
            }
        }
    });
    win.end();
    win.make_modal(true);
    win.set_callback(close_modal);
    win.show();
    Ok(())
}

pub fn bind_cashier(view: &mut KasirView, pool: Option<Pool>, user: User) {
    let items = Rc::new(RefCell::new(Vec::<SaleItem>::new()));
    let invoice = Rc::new(RefCell::new(String::new()));
    let last = Rc::new(RefCell::new(None::<Sale>));
    let pending_id = Rc::new(RefCell::new(None::<i32>));
    view.grid.set(vec![]);
    view.input_barcode.set_value("");
    view.lbl_grand_total.set_label("Rp 0");
    view.input_bayar.set_value("0");
    view.lbl_kembali.set_label("Rp 0");
    view.status.set_label(if pool.is_some() {
        ""
    } else {
        "Mode contoh. Penyimpanan memerlukan koneksi database."
    });
    let mut payment = view.payment.clone();
    let mut customer = view.customer.clone();
    customer.add_choice("Pelanggan umum");
    customer.set_value(0);
    let customers = if let Some(p) = &pool {
        db::get_customers(p).unwrap_or_default()
    } else {
        db::demo::master_rows("customers")
            .into_iter()
            .map(|r| {
                (
                    r[0].parse().unwrap(),
                    r[1].clone(),
                    r[2].clone(),
                    r[3].clone(),
                )
            })
            .collect()
    };
    for (_, name, _, _) in &customers {
        customer.add_choice(&name.replace('|', " ").replace('/', " "));
    }
    let mut due = view.due.clone();
    due.set_value(&(chrono::Local::now().date_naive() + chrono::Duration::days(30)).to_string());
    let mut credit_date = due.clone();
    let mut credit_label = view.due_label.clone();
    payment.set_callback(move |p| {
        if p.value() == 1 {
            credit_date.show();
            credit_label.show();
        } else {
            credit_date.hide();
            credit_label.hide();
        }
    });
    let mut remove = view.btn_remove.clone();
    let refresh = {
        let data = items.clone();
        let mut grid = view.grid.clone();
        let mut total = view.lbl_grand_total.clone();
        let paid = view.input_bayar.clone();
        let mut change = view.lbl_kembali.clone();
        move || {
            grid.set(
                data.borrow()
                    .iter()
                    .map(|line| {
                        vec![
                            line.product_code.clone(),
                            line.product_name.clone(),
                            line.qty.to_string(),
                            line.unit_name.clone(),
                            line.sell_price.to_string(),
                            line.discount.to_string(),
                            line.subtotal.to_string(),
                        ]
                    })
                    .collect(),
            );
            let sum = data
                .borrow()
                .iter()
                .map(|i| i.subtotal)
                .sum::<f64>()
                .max(0.0);
            total.set_label(&rupiah(sum));
            change.set_label(&rupiah(
                (operations::money(&paid.value()).unwrap_or(0.0) - sum).max(0.0),
            ));
        }
    };
    view.input_bayar.set_trigger(CallbackTrigger::Changed);
    let mut r = refresh.clone();
    view.input_bayar.set_callback(move |_| r());
    let append = {
        let data = items.clone();
        let mut r = refresh.clone();
        let mut barcode = view.input_barcode.clone();
        let mut st = view.status.clone();
        move |product: Product, line: PurchaseLine| -> Result<(), String> {
            crate::services::cart::add_sale_line(
                &mut data.borrow_mut(),
                &product,
                &line,
                &chrono::Local::now().date_naive().to_string(),
            )?;
            r();
            st.set_label(&format!(
                "{} ditambahkan. Scan berikutnya atau pilih Cari obat manual.",
                product.name
            ));
            barcode.set_value("");
            let mut focus = barcode.clone();
            fltk::app::add_timeout3(0.0, move |_| {
                let _ = focus.take_focus();
            });
            Ok(())
        }
    };
    let p = pool.clone();
    let query = view.input_barcode.clone();
    let add = append.clone();
    view.btn_add_item.set_callback(move |_| {
        item_dialog(p.clone(), false, &query.value(), add.clone());
    });
    let p = pool.clone();
    let mut add = append;
    let mut st = view.status.clone();
    view.input_barcode.set_trigger(CallbackTrigger::EnterKey);
    view.input_barcode.set_callback(move |query| {
        let value = query.value().trim().to_string();
        if value.is_empty() {
            return;
        }
        let result = (|| -> Result<(), String> {
            let products = db::view_products(p.as_ref(), Some(&value))?;
            if products.is_empty() {
                return Err("Obat atau barcode tidak ditemukan".into());
            }
            if let Some(product) = products
                .into_iter()
                .find(|product| product.code.eq_ignore_ascii_case(&value))
            {
                let unit = product
                    .units
                    .iter()
                    .find(|u| u.conversion_qty == 1)
                    .ok_or("Satuan dasar obat belum memiliki harga jual")?;
                let line = PurchaseLine {
                    product_id: product.id,
                    code: product.code.clone(),
                    name: product.name.clone(),
                    unit: unit.unit_name.clone(),
                    conversion: unit.conversion_qty,
                    qty: 1,
                    price: unit.sell_price,
                    discount: 0.0,
                    batch: "PENJUALAN".into(),
                    expiry: None,
                };
                add(product, line)?;
            } else {
                item_dialog(p.clone(), false, &value, add.clone());
            }
            Ok(())
        })();
        if let Err(e) = result {
            st.set_label(&e);
            query.set_value("");
            let _ = query.take_focus();
        }
    });
    let data = items.clone();
    let t = view.grid.table.clone();
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
        let mut st = view.status.clone();
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
    let mut st = view.status.clone();
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
    let mut st = view.status.clone();
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
    let mut st = view.status.clone();
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
    let mut st = view.status.clone();
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
            disable(b);
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
    view.win.handle(move |window, event| {
        if window.visible_r()
            && fltk::app::modal().is_none()
            && matches!(event, Event::KeyDown | Event::Shortcut)
        {
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
    if pool.is_none() {
        *items.borrow_mut() = db::demo::sales()[0].items.clone();
        view.input_bayar.set_value("50000");
        view.customer.set_value(1);
        let mut r = refresh;
        r();
        view.status
            .set_label("Contoh penjualan. Penyimpanan dinonaktifkan pada mode contoh.");
    }
}

pub fn bind_purchase(view: &mut PembelianView, pool: Option<Pool>, user: User) {
    let lines = Rc::new(RefCell::new(Vec::<PurchaseLine>::new()));
    let pending_id = Rc::new(RefCell::new(None::<i32>));
    view.grid.set(vec![]);
    view.input_faktur.set_value("");
    view.input_tempo
        .set_value(&(chrono::Local::now().date_naive() + chrono::Duration::days(30)).to_string());
    view.input_tempo.set_tooltip("Jatuh tempo: YYYY-MM-DD");
    view.choice_suplier.clear();
    view.choice_suplier.add_choice("Pilih suplier");
    view.choice_suplier.set_value(0);
    let suppliers = if let Some(p) = &pool {
        db::get_suppliers(p).unwrap_or_default()
    } else {
        db::demo::master_rows("suppliers")
            .into_iter()
            .map(|r| {
                (
                    r[0].parse().unwrap(),
                    r[1].clone(),
                    r[2].clone(),
                    r[3].clone(),
                )
            })
            .collect()
    };
    for (_, name, _, _) in &suppliers {
        view.choice_suplier
            .add_choice(&name.replace('|', " ").replace('/', " "));
    }
    let status = view.status.clone();
    let mut remove = view.btn_remove.clone();
    let mut pending = view.btn_pending.clone();
    let mut restore = view.btn_restore.clone();
    let refresh = {
        let disc = view.input_discount.clone();
        let tax = view.input_tax.clone();
        let data = lines.clone();
        let mut grid = view.grid.clone();
        let mut sub = view.lbl_subtotal.clone();
        let mut total = view.lbl_grand_total.clone();
        move || {
            grid.set(
                data.borrow()
                    .iter()
                    .map(|line| {
                        vec![
                            line.code.clone(),
                            line.name.clone(),
                            line.qty.to_string(),
                            line.unit.clone(),
                            line.price.to_string(),
                            line.discount.to_string(),
                            line.total().to_string(),
                        ]
                    })
                    .collect(),
            );
            let sum = data
                .borrow()
                .iter()
                .map(PurchaseLine::total)
                .sum::<f64>()
                .max(0.0);
            sub.set_label(&rupiah(sum));
            let result = (sum - operations::money(&disc.value()).unwrap_or(0.0))
                * (1.0 + operations::money(&tax.value()).unwrap_or(0.0) / 100.0);
            total.set_label(&rupiah(result));
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
        let data = data.clone();
        let mut r = r.clone();
        item_dialog(p.clone(), true, "", move |_, line| {
            data.borrow_mut().push(line);
            r();
            Ok(())
        });
    });
    let data = lines.clone();
    let t = view.grid.table.clone();
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
    if pool.is_none() {
        disable(&mut view.btn_simpan);
        disable(&mut pending);
        disable(&mut restore);
    }
    let mut buttons = [
        view.btn_tambah_item.clone(),
        view.btn_simpan.clone(),
        view.btn_baru.clone(),
        pending.clone(),
        restore.clone(),
    ];
    view.win.handle(move |window, event| {
        if window.visible_r()
            && fltk::app::modal().is_none()
            && matches!(event, Event::KeyDown | Event::Shortcut)
        {
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
    if pool.is_none() {
        let p = db::demo::purchases().remove(0);
        *lines.borrow_mut() = p.lines;
        view.input_faktur.set_value(&p.invoice);
        view.choice_suplier.set_value(1);
        let mut r = refresh;
        r();
        view.status
            .set_label("Contoh pembelian. Penyimpanan dinonaktifkan pada mode contoh.");
    }
}
