use super::{components::*, kasir::KasirView, pembelian::PembelianView};
use crate::{
    db::{
        self,
        operations::{self, Purchase, PurchaseLine},
    },
    gui::theme::AppTheme as T,
    models::{Product, ProductUnit, Sale, SaleItem, User},
};
use fltk::{
    browser::HoldBrowser,
    enums::{CallbackTrigger, Color, Event, FrameType, Key},
    input::Input,
    menu::Choice,
    prelude::*,
    window::Window,
};
use mysql::Pool;
use std::{cell::RefCell, rc::Rc};

fn transaction_unit(product: &Product, purchase: bool) -> Option<&ProductUnit> {
    let name = if purchase && !product.purchase_unit.trim().is_empty() {
        &product.purchase_unit
    } else {
        &product.base_unit
    };
    let is_base_unit = name.trim().eq_ignore_ascii_case(product.base_unit.trim());
    product.units.iter().find(|unit| {
        unit.unit_name.trim().eq_ignore_ascii_case(name.trim())
            && unit.conversion_qty > 0
            && (!is_base_unit || unit.conversion_qty == 1)
    })
}

fn medicine_browser() -> HoldBrowser {
    let mut product = HoldBrowser::new(24, 136, 712, 160, None);
    product.set_frame(FrameType::BorderBox);
    product.set_color(Color::White);
    product.set_text_size(14);
    product.set_selection_color(T::PRIMARY);
    product.set_format_char('\u{1f}');
    product.set_column_char('\t');
    // FLTK appends the zero terminator; the last column uses the remaining width.
    product.set_column_widths(&[144, 272, 80, 96]);
    product.set_trigger(CallbackTrigger::Changed);
    product
}

fn medicine_search(query: &str) -> Input {
    let mut search = input(24, 20, 536, "Cari nama / barcode", query);
    search.set_trigger(CallbackTrigger::EnterKeyAlways);
    search
}

fn item_dialog(
    pool: Option<Pool>,
    purchase: bool,
    query: &str,
    return_focus: Option<Input>,
    mut add: impl FnMut(Product, PurchaseLine) -> Result<(), String> + 'static,
) {
    let mut win = Window::default()
        .with_size(760, if purchase { 608 } else { 550 })
        .with_label(if purchase {
            "Tambah barang pembelian"
        } else {
            "Tambah obat penjualan"
        });
    win.set_color(T::BG_APP);
    let mut search = medicine_search(query);
    let mut find = button(576, 44, 160, "Cari [Enter]", false);
    text(
        24,
        82,
        712,
        20,
        "Cari + Enter · Panah pilih obat · Enter ke jumlah · Satuan otomatis dari master",
        12,
        false,
    );
    for (x, width, label) in [
        (24, 144, "Kode"),
        (168, 272, "Nama obat"),
        (440, 80, "Stok"),
        (520, 96, "Satuan"),
        (616, 120, "ED terdekat"),
    ] {
        text(x, 110, width, 20, label, 12, true);
    }
    let mut product = medicine_browser();
    let qty = input(24, 312, 100, "Jumlah", "1");
    let unit = readonly_value(144, 312, 180, "Satuan", "");
    let price = money_input(
        344,
        312,
        190,
        if purchase {
            "Harga beli (Rp)"
        } else {
            "Harga jual (Rp)"
        },
        "0",
    );
    let discount = input(554, 312, 182, "Diskon (%)", "0");
    let batch = purchase.then(|| input(24, 388, 324, "Nomor batch", ""));
    let expiry = purchase.then(|| input(368, 388, 368, "Kedaluwarsa (YYYY-MM-DD)", ""));
    let details = (!purchase).then(|| text(24, 388, 712, 34, "", 12, false));
    let mut status = text(24, if purchase { 466 } else { 432 }, 712, 34, "", 12, false);
    let button_y = if purchase { 550 } else { 490 };
    let mut save = button(24, button_y, 216, "Tambah [Enter]", true);
    let mut cancel = button(256, button_y, 144, "Batal [Esc]", false);
    let products = Rc::new(RefCell::new(Vec::<Product>::new()));
    let select = {
        let ps = products.clone();
        let p = product.clone();
        let mut u = unit.clone();
        let mut price = price.clone();
        let mut details = details.clone();
        move || {
            u.set_label("");
            price.set_value("0");
            if let Some(details) = &mut details {
                details.set_label("");
            }
            if let Some(product) = ps.borrow().get((p.value() - 1) as usize) {
                if let Some(details) = &mut details {
                    details.set_label(&format!(
                        "Stok layak jual: {} {}  •  ED terdekat: {}",
                        product.sellable_stock_on(&chrono::Local::now().date_naive().to_string()),
                        product.base_unit,
                        product.earliest_expiry().unwrap_or("—")
                    ));
                }
                if let Some(unit) = transaction_unit(product, purchase) {
                    u.set_label(&unit.unit_name);
                    price.set_value(&number(if purchase {
                        product.buy_price * unit.conversion_qty as f64
                    } else {
                        unit.sell_price
                    }));
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
                    let clean = |value: &str| value.replace(['\t', '\n', '\r', '\u{1f}'], " ");
                    p.add(&format!(
                        "{}\t{}\t{}\t{}\t{}",
                        clean(&product.code),
                        clean(&product.name),
                        product.sellable_stock_on(&chrono::Local::now().date_naive().to_string()),
                        clean(&product.base_unit),
                        product.earliest_expiry().unwrap_or("—")
                    ));
                }
                *ps.borrow_mut() = data;
                if !ps.borrow().is_empty() {
                    p.select(1);
                }
                select();
                st.set_label(if ps.borrow().is_empty() {
                    "Obat tidak ditemukan. Ubah pencarian."
                } else {
                    "Maksimum 200 obat. Enter pada jumlah menambah. Satuan mengikuti master."
                });
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
    let mut results = product.clone();
    let mut search_again = search.clone();
    find.set_callback(move |_| {
        l();
        search_again.clear_changed();
        if results.size() > 0 {
            let _ = results.take_focus();
        } else {
            focus_input(&mut search_again);
        }
    });
    let mut l = load.clone();
    let mut results = product.clone();
    let mut quantity = qty.clone();
    search.set_callback(move |query| {
        if query.changed() || results.size() == 0 {
            l();
            if results.size() > 0 {
                let _ = results.take_focus();
            } else {
                focus_input(query);
            }
        } else if results.value() > 0 {
            focus_input(&mut quantity);
        } else {
            focus_input(query);
        }
    });
    let mut find_results = find.clone();
    let mut results = product.clone();
    search.handle(move |query, event| {
        if event == Event::KeyDown && matches!(fltk::app::event_key(), Key::Up | Key::Down) {
            if query.changed() || results.size() == 0 {
                find_results.do_callback();
            } else {
                let step = if fltk::app::event_key() == Key::Down {
                    1
                } else {
                    -1
                };
                let row = (results.value() + step).clamp(1, results.size());
                let _ = results.take_focus();
                results.select(row);
                results.make_visible(row);
                results.do_callback();
            }
            return true;
        }
        false
    });
    let mut s = select;
    product.set_callback(move |_| s());
    let close = move |window: &mut Window| {
        close_modal(window);
        if let Some(mut field) = return_focus.clone() {
            focus_input(&mut field);
        }
    };
    let mut w = win.clone();
    let close_cancel = close.clone();
    cancel.set_callback(move |_| close_cancel(&mut w));
    let mut w = win.clone();
    let close_save = close.clone();
    let qty_for_keys = qty.clone();
    let price_for_keys = price.clone();
    let discount_for_keys = discount.clone();
    let batch_for_keys = batch.clone();
    let expiry_for_keys = expiry.clone();
    let mut quantity = qty.clone();
    product.handle(move |list, event| {
        if event == Event::KeyDown && matches!(fltk::app::event_key(), Key::Enter | Key::KPEnter) {
            if list.value() > 0 {
                focus_input(&mut quantity);
            }
            return true;
        }
        false
    });
    save.set_callback(move |_| {
        let result = (|| -> Result<(), String> {
            let ps = products.borrow();
            let p = ps.get((product.value() - 1) as usize).ok_or("Pilih obat")?;
            let u = transaction_unit(p, purchase)
                .ok_or("Satuan master belum valid. Perbaiki di Master data obat.")?;
            let line = PurchaseLine {
                product_id: p.id,
                code: p.code.clone(),
                name: p.name.clone(),
                unit: u.unit_name.clone(),
                conversion: u.conversion_qty,
                qty: operations::quantity(&qty.value())?,
                price: money_value(&price.value())?,
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
            Ok(()) => close_save(&mut w),
            Err(e) => status.set_label(&e),
        }
    });
    let mut submit = save.clone();
    let mut cancel_key = cancel.clone();
    let mut find_key = find.clone();
    let close_window = close.clone();
    win.handle(move |window, event| {
        if event == Event::KeyDown {
            let key = fltk::app::event_key();
            if key == Key::Escape {
                close(window);
                return true;
            }
            if key == Key::Enter || key == Key::KPEnter {
                let focus = fltk::app::focus().map(|widget| widget.as_widget_ptr());
                let editing = [
                    qty_for_keys.as_widget_ptr(),
                    price_for_keys.as_widget_ptr(),
                    discount_for_keys.as_widget_ptr(),
                    submit.as_widget_ptr(),
                ]
                .into_iter()
                .chain(batch_for_keys.as_ref().map(|input| input.as_widget_ptr()))
                .chain(expiry_for_keys.as_ref().map(|input| input.as_widget_ptr()))
                .any(|widget| Some(widget) == focus);
                if editing {
                    submit.do_callback();
                    return true;
                }
                for button in [&mut cancel_key, &mut find_key] {
                    if Some(button.as_widget_ptr()) == focus {
                        button.do_callback();
                        return true;
                    }
                }
            }
        }
        false
    });
    let mut l = load;
    l();
    win.end();
    win.make_modal(true);
    let mut search_focus = search.clone();
    win.set_callback(close_window);
    win.show();
    focus_input(&mut search_focus);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medicine_picker_config_is_accepted_by_fltk_without_showing_a_window() {
        let product = medicine_browser();
        assert_eq!(product.column_widths().len(), 4);
        assert!(product.column_widths().iter().all(|width| *width > 0));
        assert_eq!(product.column_char(), '\t');
        fltk::app::delete_widget(product);
        // Keep native widgets in one test: FLTK binds them to the first UI thread.
        for query in ["", "paracetamol"] {
            let search = medicine_search(query);
            assert!(!search.changed());
            assert!(search
                .trigger()
                .contains(CallbackTrigger::EnterKey | CallbackTrigger::NotChanged));
            fltk::app::delete_widget(search);
        }
        let mut unit = readonly_value(144, 312, 180, "Satuan", "Tablet");
        assert_eq!(unit.label(), "Tablet");
        assert!(unit.take_focus().is_err());
        fltk::app::delete_widget(unit);
        let paid = money_input(876, 468, 296, "Bayar (Rp) [F2]", "110000");
        assert_eq!(paid.value(), "110.000");
        assert_eq!(money_value(&paid.value()).unwrap(), 110000.0);
        fltk::app::delete_widget(paid);
    }

    #[test]
    fn transaction_units_follow_master_defaults_and_preserve_conversion() {
        let mut product = db::demo::products().remove(0);
        let base = product
            .units
            .iter()
            .find(|unit| unit.conversion_qty == 1)
            .unwrap()
            .clone();
        let mut purchase = base.clone();
        purchase.unit_name = "Strip".into();
        purchase.conversion_qty = 10;
        purchase.sell_price = base.sell_price * 10.0;
        product.base_unit = base.unit_name.to_uppercase();
        product.purchase_unit = purchase.unit_name.clone();
        product.units = vec![purchase, base.clone()];

        let sale = transaction_unit(&product, false).unwrap();
        assert_eq!(sale.unit_name, base.unit_name);
        assert_eq!(sale.conversion_qty, 1);
        assert_eq!(sale.sell_price, base.sell_price);
        assert_eq!(transaction_unit(&product, true).unwrap().conversion_qty, 10);

        product.purchase_unit.clear();
        assert_eq!(
            transaction_unit(&product, true).unwrap().unit_name,
            base.unit_name
        );
        product.purchase_unit = "Tidak terdaftar".into();
        assert!(transaction_unit(&product, true).is_none());
        product.units[1].conversion_qty = 2;
        assert!(transaction_unit(&product, false).is_none());
        product.purchase_unit.clear();
        assert!(transaction_unit(&product, true).is_none());
    }
}

fn choose_pending(
    pool: &Pool,
    operator: i32,
    purchase: bool,
    return_focus: Option<Input>,
    mut open: impl FnMut(i32, String) -> Result<(), String> + 'static,
) -> Result<(), String> {
    let data = operations::pending(pool, operator, purchase)?;
    let mut win = Window::default()
        .with_size(640, 300)
        .with_label("Transaksi pending");
    win.set_color(T::BG_APP);
    text(24, 20, 592, 28, "Pilih transaksi pending", 20, true);
    let mut choice = Choice::new(24, 80, 592, 34, None);
    style_choice(&mut choice);
    for (_, payload) in &data {
        let description = if purchase {
            serde_json::from_str::<Purchase>(payload)
                .map(|p| format!("{} • {} barang", p.invoice, p.lines.len()))
                .unwrap_or_else(|_| "Data tidak terbaca".into())
        } else {
            serde_json::from_str::<Sale>(payload)
                .map(|p| {
                    format!(
                        "{} · {} obat · {}",
                        p.invoice_no,
                        p.items.len(),
                        rupiah(p.grand_total)
                    )
                })
                .unwrap_or_else(|_| "Data tidak terbaca".into())
        };
        choice.add_choice(&description.replace('|', " ").replace('/', " "));
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
    let close = move |window: &mut Window| {
        close_modal(window);
        if let Some(mut field) = return_focus.clone() {
            focus_input(&mut field);
        }
    };
    let close_restore = close.clone();
    let mut w = win.clone();
    let mut choice_focus = choice.clone();
    restore.set_callback(move |_| {
        if let Some((id, payload)) = data.get(choice.value().max(0) as usize) {
            match open(*id, payload.clone()) {
                Ok(()) => close_restore(&mut w),
                Err(e) => status.set_label(&e),
            }
        }
    });
    let mut restore_key = restore.clone();
    win.handle(move |_, event| {
        if event == Event::KeyDown && matches!(fltk::app::event_key(), Key::Enter | Key::KPEnter) {
            if restore_key.active() {
                restore_key.do_callback();
            }
            return true;
        }
        false
    });
    win.end();
    win.make_modal(true);
    win.set_callback(close);
    win.show();
    let _ = choice_focus.take_focus();
    Ok(())
}

pub fn bind_cashier(view: &mut KasirView, pool: Option<Pool>, user: User) {
    let items = Rc::new(RefCell::new(Vec::<SaleItem>::new()));
    let invoice = Rc::new(RefCell::new(String::new()));
    let doctor_id = Rc::new(std::cell::Cell::new(None::<i32>));
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
    let salespeople = db::named_people(pool.as_ref(), "salespeople").unwrap_or_default();
    view.salesperson.add_choice("Tidak dipilih");
    for (_, name) in &salespeople {
        view.salesperson
            .add_choice(&name.replace('|', " ").replace('/', " "));
    }
    view.salesperson.set_value(0);
    let mut invoice_display = view.invoice.clone();
    *invoice.borrow_mut() = format!("PJ-{}-{}", chrono::Utc::now().timestamp_micros(), user.id);
    invoice_display.set_value(&invoice.borrow());
    let mut due = view.due.clone();
    due.set_value(&(chrono::Local::now().date_naive() + chrono::Duration::days(30)).to_string());
    let mut credit_date = due.clone();
    let mut credit_label = view.due_label.clone();
    let mut normal_sale = view.sale_type_label.clone();
    payment.set_callback(move |p| {
        if p.value() == 1 {
            credit_date.show();
            credit_label.show();
            normal_sale.hide();
        } else {
            credit_date.hide();
            credit_label.hide();
            normal_sale.show();
        }
    });
    let mut remove = view.btn_remove.clone();
    let mut back = view.btn_back.clone();
    view.btn_add_item.deactivate();
    let percentage_mode = Rc::new(std::cell::Cell::new(false));
    let refresh = {
        let data = items.clone();
        let mut grid = view.grid.clone();
        let mut total = view.lbl_grand_total.clone();
        let mut subtotal_label = view.lbl_subtotal.clone();
        let mut save = view.btn_simpan.clone();
        let mut pending_button = view.btn_pending.clone();
        let connected = pool.is_some();
        let paid = view.input_bayar.clone();
        let mut change = view.lbl_kembali.clone();
        let mut discount = view.discount.clone();
        let shipping = view.shipping.clone();
        let percent = view.discount_percent.clone();
        let mode = percentage_mode.clone();
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
            if grid.selected_rows().is_empty() {
                grid.select_first();
            }
            let sum = data
                .borrow()
                .iter()
                .map(|i| i.subtotal)
                .sum::<f64>()
                .max(0.0);
            subtotal_label.set_label(&format!("Subtotal {}", rupiah(sum)));
            if mode.get() {
                let value = operations::money(&percent.value()).unwrap_or(101.0);
                discount.set_value(&number(sum * value / 100.0));
            }
            let sum = match money_value(&discount.value()).and_then(|d| {
                money_value(&shipping.value())
                    .and_then(|s| crate::services::cart::invoice_total(sum, d, s))
            }) {
                Ok(v) => v,
                Err(_) => {
                    total.set_label("—");
                    change.set_label("—");
                    subtotal_label.set_label("Periksa diskon dan ongkos");
                    save.deactivate();
                    pending_button.deactivate();
                    return;
                }
            };
            total.set_label(&rupiah(sum));
            if connected && !data.borrow().is_empty() {
                save.activate();
                pending_button.activate();
            } else {
                save.deactivate();
                pending_button.deactivate();
            }
            change.set_label(&rupiah(
                (money_value(&paid.value()).unwrap_or(0.0) - sum).max(0.0),
            ));
        }
    };
    view.input_bayar.set_trigger(CallbackTrigger::Changed);
    let mut r = refresh.clone();
    view.input_bayar.set_callback(move |_| r());
    let mode = percentage_mode.clone();
    let mut r = refresh.clone();
    view.discount_percent.set_trigger(CallbackTrigger::Changed);
    view.discount_percent.set_callback(move |_| {
        mode.set(true);
        r();
    });
    let mode = percentage_mode.clone();
    let nominal = view.discount.clone();
    let cart = items.clone();
    let mut percent = view.discount_percent.clone();
    let mut r = refresh.clone();
    view.discount.set_trigger(CallbackTrigger::Changed);
    view.discount.set_callback(move |_| {
        mode.set(false);
        let subtotal = cart.borrow().iter().map(|i| i.subtotal).sum::<f64>();
        let discount = money_value(&nominal.value()).unwrap_or(0.0);
        percent.set_value(&format!(
            "{:.2}",
            if subtotal > 0.0 {
                discount / subtotal * 100.0
            } else {
                0.0
            }
        ));
        r();
    });
    let mut r = refresh.clone();
    view.shipping.set_trigger(CallbackTrigger::Changed);
    view.shipping.set_callback(move |_| r());
    let selected = Rc::new(RefCell::new(Vec::<Product>::new()));
    let select = {
        let selected = selected.clone();
        let product = view.product.clone();
        let mut unit = view.unit.clone();
        let mut price = view.price.clone();
        let mut add = view.btn_add_item.clone();
        move || {
            unit.set_label("");
            price.set_value("0");
            add.deactivate();
            if let Some(p) = selected.borrow().get(product.value().max(0) as usize) {
                if let Some(u) = transaction_unit(p, false) {
                    add.activate();
                    unit.set_label(&u.unit_name);
                    price.set_value(&number(u.sell_price));
                    price.do_callback();
                }
            }
        }
    };
    let mut on_select = select.clone();
    view.product.set_callback(move |_| on_select());
    let line_preview = {
        let qty = view.qty.clone();
        let price = view.price.clone();
        let discount = view.item_discount.clone();
        let mut total = view.line_total.clone();
        move || {
            let n = operations::quantity(&qty.value()).unwrap_or(0);
            let price = money_value(&price.value()).unwrap_or(0.0);
            let d = operations::money(&discount.value()).unwrap_or(0.0);
            total.set_label(&rupiah((n as f64 * price * (1.0 - d / 100.0)).max(0.0)));
        }
    };
    for field in [&mut view.qty, &mut view.price, &mut view.item_discount] {
        field.set_trigger(CallbackTrigger::Changed);
        let mut update = line_preview.clone();
        field.set_callback(move |_| update());
    }
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
                "{} ditambahkan. Scan lagi atau F1 untuk memilih obat.",
                product.name
            ));
            barcode.set_value("");
            let mut focus = barcode.clone();
            fltk::app::add_timeout3(0.0, move |_| {
                if focus.visible_r() && fltk::app::modal().is_none() {
                    focus_input(&mut focus);
                }
            });
            Ok(())
        }
    };
    let mut append_manual = append.clone();
    let ps = selected.clone();
    let name = view.product.clone();
    let qty = view.qty.clone();
    let price = view.price.clone();
    let discount = view.item_discount.clone();
    let mut st = view.status.clone();
    view.btn_add_item.set_callback(move |_| {
        let result = (|| -> Result<(), String> {
            let ps = ps.borrow();
            let p = ps
                .get(name.value().max(0) as usize)
                .ok_or("Cari dan pilih obat terlebih dahulu")?;
            let u = transaction_unit(p, false)
                .ok_or("Satuan master belum valid. Perbaiki di Master data obat.")?;
            let line = PurchaseLine {
                product_id: p.id,
                code: p.code.clone(),
                name: p.name.clone(),
                unit: u.unit_name.clone(),
                conversion: u.conversion_qty,
                qty: operations::quantity(&qty.value())?,
                price: money_value(&price.value())?,
                discount: operations::money(&discount.value())?,
                batch: "PENJUALAN".into(),
                expiry: None,
            };
            append_manual(p.clone(), line)
        })();
        if let Err(e) = result {
            st.set_label(&e);
        }
    });
    let p = pool.clone();
    let append_picker = append.clone();
    let picker_pool = pool.clone();
    let picker_input = view.input_barcode.clone();
    let open_picker = move |query: &str| {
        item_dialog(
            picker_pool.clone(),
            false,
            query,
            Some(picker_input.clone()),
            append_picker.clone(),
        );
    };
    let mut append_scan = append;
    let search_picker = open_picker.clone();
    let ps = selected.clone();
    let mut names = view.product.clone();
    let mut select = select;
    let mut st = view.status.clone();
    let mut qty = view.qty.clone();
    let mut disc = view.item_discount.clone();
    let mut preview = line_preview;
    view.input_barcode.set_trigger(CallbackTrigger::EnterKey);
    view.input_barcode.set_callback(move |query| {
        let value = query.value().trim().to_string();
        if value.is_empty() {
            return;
        }
        let result = (|| -> Result<(), String> {
            let data = db::view_products(p.as_ref(), Some(&value))?;
            if data.is_empty() {
                return Err("Obat atau barcode tidak ditemukan".into());
            }
            let exact = data
                .iter()
                .find(|p| p.code.eq_ignore_ascii_case(&value))
                .cloned();
            names.clear();
            for p in &data {
                names.add_choice(&p.name.replace('|', " ").replace('/', " "));
            }
            *ps.borrow_mut() = data;
            names.set_value(0);
            qty.set_value("1");
            disc.set_value("0");
            select();
            preview();
            if let Some(product) = exact {
                let u = transaction_unit(&product, false)
                    .ok_or("Satuan master belum valid. Perbaiki di Master data obat.")?;
                let line = PurchaseLine {
                    product_id: product.id,
                    code: product.code.clone(),
                    name: product.name.clone(),
                    unit: u.unit_name.clone(),
                    conversion: u.conversion_qty,
                    qty: 1,
                    price: u.sell_price,
                    discount: 0.0,
                    batch: "PENJUALAN".into(),
                    expiry: None,
                };
                append_scan(product, line)?;
            } else {
                search_picker(&value);
            }
            Ok(())
        })();
        if let Err(e) = result {
            st.set_label(&e);
            ps.borrow_mut().clear();
            names.clear();
            select();
            preview();
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
        let mut discount = view.discount.clone();
        let mut percent = view.discount_percent.clone();
        let mut shipping = view.shipping.clone();
        let mut draft = view.invoice.clone();
        let mode = percentage_mode.clone();
        let doctor_id = doctor_id.clone();
        let mut sales = view.salesperson.clone();
        let mut date = view.date.clone();
        let mut names = view.product.clone();
        let mut units = view.unit.clone();
        let selected = selected.clone();
        let mut code = view.input_barcode.clone();
        let mut qty = view.qty.clone();
        let mut price = view.price.clone();
        let mut item_discount = view.item_discount.clone();
        let mut line_total = view.line_total.clone();
        let mut add = view.btn_add_item.clone();
        move || {
            selected.borrow_mut().clear();
            names.clear();
            units.set_label("");
            code.set_value("");
            qty.set_value("1");
            price.set_value("0");
            item_discount.set_value("0");
            line_total.set_label("Rp 0");
            add.deactivate();
            date.set_value(&chrono::Local::now().date_naive().to_string());
            mode.set(false);
            doctor_id.set(None);
            sales.set_value(0);
            discount.set_value("0");
            percent.set_value("0");
            shipping.set_value("0");
            data.borrow_mut().clear();
            paid.set_value("0");
            c.set_value(0);
            pt.set_value(0);
            pt.do_callback();
            st.set_label("");
            r();
            *invoice.borrow_mut() = format!("PJ-{}", chrono::Utc::now().timestamp_micros());
            draft.set_value(&invoice.borrow());
            focus_input(&mut code);
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
        let date = view.date.clone();
        let doctor_id = doctor_id.clone();
        let salesperson = view.salesperson.clone();
        let salespeople = salespeople.clone();
        let discount = view.discount.clone();
        let shipping = view.shipping.clone();
        move || -> Result<Sale, String> {
            let subtotal = data
                .borrow()
                .iter()
                .map(|i| i.subtotal)
                .sum::<f64>()
                .max(0.0);
            let paid = money_value(&paid.value())?;
            let discount = money_value(&discount.value())?;
            let shipping = money_value(&shipping.value())?;
            let total = crate::services::cart::invoice_total(subtotal, discount, shipping)?;
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
                sale_date: format!(
                    "{} {}",
                    operations::date(&date.value())?,
                    chrono::Local::now().format("%H:%M:%S")
                ),
                payment_type: if credit { "KREDIT" } else { "TUNAI" }.into(),
                due_date: if credit {
                    Some(operations::date(&due.value())?)
                } else {
                    None
                },
                customer_id: customer,
                doctor_id: doctor_id.get(),
                salesperson_id: salespeople
                    .get((salesperson.value() - 1) as usize)
                    .map(|r| r.0),
                shipping,
                operator_id: user.id,
                subtotal,
                discount,
                grand_total: total,
                paid_amount: paid,
                change_amount: if credit { 0.0 } else { (paid - total).max(0.0) },
                total_profit: 0.0,
                items: data.borrow().clone(),
            })
        }
    };
    let complete = {
        let pool = pool.clone();
        let build = build.clone();
        let mut clear = clear.clone();
        let mut status = view.status.clone();
        let paper = view.paper.clone();
        let mut history = view.btn_back.clone();
        let mut paid = view.input_bayar.clone();
        move || {
            let Some(pool) = &pool else {
                return;
            };
            let result = (|| -> Result<crate::services::cart::CheckoutResult, String> {
                crate::services::cart::finish_sale(
                    Some(build()?),
                    None,
                    |sale| {
                        clear();
                        status.set_label(&format!("Faktur {} tersimpan.", sale.invoice_no));
                        fltk::dialog::choice2_default(
                            &format!("Faktur {} tersimpan.\nIngin cetak struk?\nEnter: cetak · Esc: tidak", sale.invoice_no),
                            "&Tidak",
                            "&Ya, cetak",
                            "",
                        ) == Some(1)
                    },
                    |sale| db::save_sale_fefo(pool, sale.clone()).map_err(|e| e.to_string()),
                    |sale| {
                        crate::services::documents::print_receipt(
                            pool,
                            sale,
                            if paper.value() == 1 { 80 } else { 58 },
                        )
                    },
                )
            })();
            match result {
                Ok(result) => {
                    debug_assert!(result.saved_now, "Cashier checkout must save a new invoice");
                    let invoice = &result.sale.invoice_no;
                    if let Some(error) = result.print_error {
                        status.set_label(&format!(
                            "Faktur {invoice} tersimpan; cetak gagal. Cetak ulang dari riwayat penjualan."
                        ));
                        status.set_tooltip(&error);
                        fltk::dialog::alert_default(&format!(
                            "Faktur {invoice} tersimpan.\n{error}\nCetak ulang tersedia di riwayat penjualan."
                        ));
                    } else if result.print_requested {
                        status.set_label(&format!(
                            "Faktur {invoice} tersimpan. Struk dikirim ke proses cetak."
                        ));
                        status.set_tooltip("");
                    } else {
                        status.set_label(&format!(
                            "Faktur {invoice} tersimpan. Siap untuk penjualan berikutnya."
                        ));
                        status.set_tooltip("");
                    }
                    history.do_callback();
                }
                Err(error) => {
                    status.set_label(&format!("Transaksi belum selesai: {error}"));
                    focus_input(&mut paid);
                }
            }
        }
    };
    let mut save = complete;
    view.btn_simpan.set_callback(move |_| save());
    let p = pool.clone();
    let mut after_pending = clear.clone();
    let mut st = view.status.clone();
    view.btn_pending.set_callback(move |_| {
        let Some(p) = &p else {
            return;
        };
        let result = build().and_then(|sale| {
            let payload = serde_json::to_string(&sale).map_err(|e| e.to_string())?;
            operations::save_pending(p, user.id, false, &payload)?;
            Ok(sale.invoice_no)
        });
        match result {
            Ok(invoice) => {
                after_pending();
                st.set_label(&format!(
                    "Pending {invoice} disimpan. F7 untuk melanjutkan."
                ));
            }
            Err(error) => st.set_label(&format!("Pending gagal: {error}")),
        }
    });
    let p = pool.clone();
    let data = items.clone();
    let paid = view.input_bayar.clone();
    let mut st = view.status.clone();
    let r = refresh.clone();
    let saved_percent = view.discount_percent.clone();
    let mode_pending = percentage_mode.clone();
    let saved_discount = view.discount.clone();
    let saved_shipping = view.shipping.clone();
    let saved_date = view.date.clone();
    let saved_invoice = view.invoice.clone();
    let saved_doctor_id = doctor_id.clone();
    let saved_sales = view.salesperson.clone();
    let pending_focus = view.input_barcode.clone();
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
        let mut paid = paid.clone();
        let mut r = r.clone();
        let mut c = customer.clone();
        let mut pt = payment.clone();
        let mut d = due.clone();
        let mut percent = saved_percent.clone();
        let mode = mode_pending.clone();
        let mut discount = saved_discount.clone();
        let mut shipping = saved_shipping.clone();
        let mut date = saved_date.clone();
        let mut invoice_display = saved_invoice.clone();
        let doctor_id = saved_doctor_id.clone();
        let mut sales = saved_sales.clone();
        let sales_ids = salespeople.iter().map(|v| v.0).collect::<Vec<_>>();
        if let Err(e) = choose_pending(
            p,
            user.id,
            false,
            Some(pending_focus.clone()),
            move |_id, payload| {
                let sale: Sale = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
                *invoice.borrow_mut() = sale.invoice_no.clone();
                invoice_display.set_value(&sale.invoice_no);
                mode.set(false);
                percent.set_value(&format!(
                    "{:.2}",
                    if sale.subtotal > 0.0 {
                        sale.discount / sale.subtotal * 100.0
                    } else {
                        0.0
                    }
                ));
                discount.set_value(&number(sale.discount));
                shipping.set_value(&number(sale.shipping));
                if let Some(day) = sale.sale_date.get(..10) {
                    date.set_value(day);
                }
                doctor_id.set(sale.doctor_id);
                sales.set_value(
                    sale.salesperson_id
                        .and_then(|id| sales_ids.iter().position(|v| *v == id))
                        .map(|i| i as i32 + 1)
                        .unwrap_or(0),
                );
                *data.borrow_mut() = sale.items;
                paid.set_value(&number(sale.paid_amount));
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
            },
        ) {
            st.set_label(&e);
        }
    });
    if pool.is_none() {
        for b in [
            &mut view.btn_simpan,
            &mut view.btn_pending,
            &mut view.btn_buka_pending,
        ] {
            disable(b);
        }
    }
    let barcode = view.input_barcode.clone();
    let mut paid = view.input_bayar.clone();
    let row_fields = [
        view.qty.as_widget_ptr(),
        view.price.as_widget_ptr(),
        view.item_discount.as_widget_ptr(),
    ];
    let table = view.grid.table.clone();
    let mut add = view.btn_add_item.clone();
    let mut remove = view.btn_remove.clone();
    let mut buttons = [
        view.btn_simpan.clone(),
        view.btn_baru.clone(),
        view.btn_pending.clone(),
        view.btn_buka_pending.clone(),
    ];
    view.win.handle(move |window, event| {
        if event == Event::Show {
            let mut scanner = barcode.clone();
            fltk::app::add_timeout3(0.0, move |_| {
                if scanner.visible_r() && fltk::app::modal().is_none() {
                    focus_input(&mut scanner);
                }
            });
        }
        if window.visible_r()
            && fltk::app::modal().is_none()
            && matches!(event, Event::KeyDown | Event::Shortcut)
        {
            let key = fltk::app::event_key();
            if key == Key::Escape {
                if let Some(mut main) = window.top_window() {
                    if main.fullscreen_active() {
                        main.fullscreen(false);
                        return true;
                    }
                }
                back.do_callback();
                return true;
            }
            if key == Key::F1 {
                open_picker(&barcode.value());
                return true;
            }
            if key == Key::F2 {
                focus_input(&mut paid);
                return true;
            }
            let focus = fltk::app::focus().map(|widget| widget.as_widget_ptr());
            if key == Key::Delete && focus == Some(table.as_widget_ptr()) {
                remove.do_callback();
                return true;
            }
            if event == Event::KeyDown && (key == Key::Enter || key == Key::KPEnter) {
                if focus == Some(paid.as_widget_ptr()) {
                    if buttons[0].active() {
                        buttons[0].do_callback();
                    }
                    return true;
                }
                if row_fields.iter().any(|field| Some(*field) == focus)
                    || focus == Some(add.as_widget_ptr())
                {
                    if add.active() {
                        add.do_callback();
                    }
                    return true;
                }
                for button in buttons.iter_mut().chain([&mut remove, &mut back]) {
                    if Some(button.as_widget_ptr()) == focus {
                        if button.active() {
                            button.do_callback();
                        }
                        return true;
                    }
                }
            }
            for (i, k) in [Key::F3, Key::F5, Key::F6, Key::F7].iter().enumerate() {
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
        view.input_bayar.set_value("50.000");
        view.customer.set_value(1);
        let mut r = refresh;
        r();
        view.status
            .set_label("Contoh penjualan. Penyimpanan dinonaktifkan pada mode contoh.");
    }
}

pub fn bind_purchase(view: &mut PembelianView, pool: Option<Pool>, user: User) {
    let lines = Rc::new(RefCell::new(Vec::<PurchaseLine>::new()));
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
        let mut save_button = view.btn_simpan.clone();
        let mut pending_button = pending.clone();
        let connected = pool.is_some();
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
            let result = (sum - money_value(&disc.value()).unwrap_or(0.0))
                * (1.0 + operations::money(&tax.value()).unwrap_or(0.0) / 100.0);
            total.set_label(&rupiah(result));
            if connected && !data.borrow().is_empty() {
                save_button.activate();
                pending_button.activate();
            } else {
                save_button.deactivate();
                pending_button.deactivate();
            }
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
        item_dialog(p.clone(), true, "", None, move |_, line| {
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
                discount: money_value(&disc.value())?,
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
        move || {
            data.borrow_mut().clear();
            invoice.set_value("");
            disc.set_value("0");
            tax.set_value("0");
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
    let mut st = status.clone();
    view.btn_simpan.set_callback(move |_| {
        let Some(p) = &p else {
            return;
        };
        match b()
            .and_then(|purchase| operations::save_purchase(p, &purchase).map(|_| purchase.invoice))
        {
            Ok(invoice) => {
                c();
                st.set_label(&format!("Faktur {invoice} tersimpan"));
            }
            Err(e) => st.set_label(&format!("Simpan gagal: {e}")),
        }
    });
    let p = pool.clone();
    let mut after_pending = clear.clone();
    let mut st = status.clone();
    pending.set_callback(move |_| {
        let Some(p) = &p else {
            return;
        };
        let result = build().and_then(|purchase| {
            let payload = serde_json::to_string(&purchase).map_err(|e| e.to_string())?;
            operations::save_pending(p, user.id, true, &payload)?;
            Ok(purchase.invoice)
        });
        match result {
            Ok(invoice) => {
                after_pending();
                st.set_label(&format!(
                    "Pending pembelian {invoice} disimpan. F7 untuk melanjutkan."
                ));
            }
            Err(error) => st.set_label(&format!("Pending gagal: {error}")),
        }
    });
    let p = pool.clone();
    let data = lines.clone();
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
        let mut r = r.clone();
        let mut invoice = invoice.clone();
        let mut pt = pt.clone();
        let mut dt = dt.clone();
        let mut due = due.clone();
        if let Err(e) = choose_pending(p, user.id, true, None, move |_id, payload| {
            let purchase: Purchase = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
            disc.set_value(&number(purchase.discount));
            tax.set_value(&purchase.tax_percent.to_string());
            supplier.set_value(
                supplier_ids
                    .iter()
                    .position(|id| *id == purchase.supplier)
                    .map(|i| i as i32 + 1)
                    .unwrap_or(0),
            );
            *data.borrow_mut() = purchase.lines;
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
