use super::{components::*, form_obat::FormObatModal};
use crate::{
    db::{self, operations, NewProductInput},
    gui::theme::AppTheme as T,
    models::Product,
};
use fltk::{
    enums::{CallbackTrigger, Event},
    menu::{Choice, MenuFlag, MenuItem},
    prelude::*,
    table::TableContext,
    window::Window,
};
use mysql::Pool;
use std::{cell::RefCell, rc::Rc};

pub struct MasterObatView {
    pub win: Window,
}

fn edit_barcode(pool: Pool, product: Product, mut updated: impl FnMut() + 'static) {
    let mut win = Window::default()
        .with_size(560, 300)
        .with_label("Isi barcode kemasan");
    win.set_color(T::BG_APP);
    text(24, 20, 512, 30, "Isi barcode kemasan", 22, true);
    text(24, 64, 512, 24, &product.name, 14, false);
    let mut code = input(24, 106, 512, "Scan atau ketik barcode kemasan", "");
    text(
        24,
        176,
        512,
        24,
        &format!(
            "Kode saat ini: {}. Label lama perlu dicetak ulang.",
            product.code
        ),
        12,
        false,
    );
    let mut save = button(360, 218, 176, "Simpan barcode", true);
    let mut cancel = button(24, 218, 112, "Batal", false);
    let mut status = text(24, 262, 512, 24, "", 12, false);
    let mut w = win.clone();
    cancel.set_callback(move |_| close_modal(&mut w));
    let value = code.clone();
    let mut w = win.clone();
    save.set_callback(move |_| {
        match db::update_product_barcode(&pool, product.id, &value.value()) {
            Ok(()) => {
                close_modal(&mut w);
                updated();
            }
            Err(e) => status.set_label(&format!("Barcode gagal disimpan: {e}")),
        }
    });
    win.end();
    win.make_modal(true);
    win.set_callback(close_modal);
    win.show();
    let _ = code.take_focus();
}

fn batch_details(product: &Product) {
    let mut win = Window::default()
        .with_size(700, 380)
        .with_label("Batch dan ED obat");
    win.set_color(T::BG_APP);
    text(24, 20, 652, 30, "Batch dan ED obat", 22, true);
    text(24, 62, 652, 24, &product.name, 14, false);
    let mut grid = Grid::new(
        24,
        110,
        652,
        206,
        &["Batch", "ED", "Stok", "Harga beli (Rp)"],
    );
    grid.set(
        product
            .batches
            .iter()
            .filter(|b| b.stock > 0)
            .map(|b| {
                vec![
                    b.batch_no.clone(),
                    b.expired_date.clone().unwrap_or_else(|| "—".into()),
                    b.stock.to_string(),
                    b.buy_price.to_string(),
                ]
            })
            .collect(),
    );
    let mut close = button(24, 334, 112, "Tutup", false);
    let mut w = win.clone();
    close.set_callback(move |_| close_modal(&mut w));
    win.end();
    win.make_modal(true);
    win.set_callback(close_modal);
    win.show();
}

fn product_form(pool: Pool, existing: Option<Product>, mut updated: impl FnMut() + 'static) {
    let mut form = FormObatModal::new(existing.as_ref());
    let suppliers = match db::get_suppliers(&pool) {
        Ok(rows) => rows,
        Err(e) => {
            fltk::dialog::alert_default(&format!("Suplier gagal dimuat: {e}"));
            return;
        }
    };
    form.choice_suplier.clear();
    form.choice_suplier.add_choice("Tanpa suplier");
    for (_, name, _, _) in &suppliers {
        form.choice_suplier
            .add_choice(&name.replace('|', " ").replace('/', " "));
    }
    form.choice_suplier.set_value(
        existing
            .as_ref()
            .and_then(|p| p.supplier_id)
            .and_then(|id| suppliers.iter().position(|r| r.0 == id))
            .map(|i| i as i32 + 1)
            .unwrap_or(0),
    );
    let mut category_values = db::product_filter_options(Some(&pool))
        .map(|v| v.0)
        .unwrap_or_default();
    category_values.extend(
        [
            "Obat bebas",
            "Obat keras",
            "Alat kesehatan",
            "Vitamin",
            "Lainnya",
        ]
        .iter()
        .map(|v| v.to_string()),
    );
    if let Some(p) = &existing {
        category_values.push(p.category.clone());
    }
    let category_values = category_values
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    form.choice_kategori.clear();
    for value in &category_values {
        form.choice_kategori
            .add_choice(&value.replace('|', " ").replace('/', " "));
    }
    form.choice_kategori.set_value(
        category_values
            .iter()
            .position(|v| {
                v == existing
                    .as_ref()
                    .map(|p| p.category.as_str())
                    .unwrap_or("Obat bebas")
            })
            .unwrap_or(0) as i32,
    );
    if let Some(_p) = &existing {
        form.input_satuan_dasar.deactivate();
        form.input_stok_awal.set_value("0");
        form.input_stok_awal.deactivate();
        form.input_no_batch.deactivate();
        form.input_expired.deactivate();
    }
    form.win.begin();
    let mut status = text(16, 632, 648, 22, "", 12, false);
    form.win.end();
    let p = pool.clone();
    let mut code = form.input_kode.clone();
    let mut st = status.clone();
    form.btn_barcode.set_callback(move |_| {
        if !code.value().trim().is_empty() {
            st.set_label("Kode sudah terisi. Kosongkan untuk membuat kode baru.");
            return;
        }
        match db::generate_product_code(&p) {
            Ok(value) => {
                code.set_value(&value);
                st.set_label(
                    "Barcode dibuat. Simpan obat, lalu buka Label barcode untuk mencetak.",
                );
            }
            Err(e) => st.set_label(&format!("Barcode gagal dibuat: {e}")),
        }
    });
    let f = form.clone();
    let mut w = form.win.clone();
    form.btn_simpan.set_callback(move |_| {
        let result = (|| -> Result<(), String> {
            let min = f
                .input_min_stok
                .value()
                .parse::<i32>()
                .map_err(|_| "Stok minimum harus bilangan bulat")?;
            let stock = f
                .input_stok_awal
                .value()
                .parse::<i32>()
                .map_err(|_| "Stok awal harus bilangan bulat")?;
            let base = f.input_satuan_dasar.value().trim().to_string();
            let mut units = vec![(
                base.clone(),
                1,
                operations::money(&f.input_harga_jual_1.value())?,
            )];
            for (name, conversion, price) in [
                (
                    &f.input_nama_satuan_2,
                    &f.input_konversi_2,
                    &f.input_harga_jual_2,
                ),
                (
                    &f.input_nama_satuan_3,
                    &f.input_konversi_3,
                    &f.input_harga_jual_3,
                ),
            ] {
                if !name.value().trim().is_empty() {
                    units.push((
                        name.value().trim().into(),
                        operations::quantity(&conversion.value())?,
                        operations::money(&price.value())?,
                    ));
                }
            }
            let data = NewProductInput {
                code: f.input_kode.value().trim().into(),
                name: f.input_nama.value().trim().into(),
                category: category_values
                    .get(f.choice_kategori.value().max(0) as usize)
                    .cloned()
                    .unwrap_or_else(|| "Umum".into()),
                supplier_id: if f.choice_suplier.value() > 0 {
                    suppliers
                        .get((f.choice_suplier.value() - 1) as usize)
                        .map(|r| r.0)
                } else {
                    None
                },
                base_unit: base.clone(),
                purchase_unit: if f.input_satuan_beli.value().trim().is_empty() {
                    base.clone()
                } else {
                    f.input_satuan_beli.value().trim().into()
                },
                min_stock: min,
                buy_price: operations::money(&f.input_harga_beli.value())?,
                units,
                batch_no: f.input_no_batch.value().trim().into(),
                expired_date: if !f.input_expired.value().trim().is_empty() {
                    Some(operations::date(&f.input_expired.value())?)
                } else {
                    None
                },
                initial_stock: stock,
            };
            if let Some(p) = &existing {
                db::update_product(&pool, p.id, &data).map_err(|e| e.to_string())
            } else {
                db::insert_product(&pool, &data)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
            }
        })();
        match result {
            Ok(()) => {
                close_modal(&mut w);
                updated();
            }
            Err(e) => status.set_label(&format!("Simpan gagal: {e}")),
        }
    });
    form.win.show();
}

fn stock_card(pool: Option<&Pool>, product: &Product) -> Result<(), String> {
    let rows = if let Some(pool) = pool {
        use mysql::prelude::*;
        let data:Vec<(String,String,String,i32,i32,i32)>=pool.get_conn().and_then(|mut c|c.exec("SELECT DATE_FORMAT(created_at,'%Y-%m-%d %H:%i'),ref_type,ref_no,qty_in,qty_out,balance_after FROM stock_cards WHERE product_id=? ORDER BY id DESC LIMIT 500",(product.id,))).map_err(|e|e.to_string())?;
        data.into_iter()
            .map(|(d, t, r, i, o, b)| vec![d, t, r, i.to_string(), o.to_string(), b.to_string()])
            .collect()
    } else {
        vec![vec![
            chrono::Local::now().date_naive().to_string(),
            "STOK AWAL".into(),
            product.code.clone(),
            product.total_stock.to_string(),
            "0".into(),
            product.total_stock.to_string(),
        ]]
    };
    let mut win = Window::default()
        .with_size(880, 420)
        .with_label("Kartu stok obat");
    win.set_color(T::BG_APP);
    text(
        24,
        20,
        832,
        30,
        &format!("Kartu stok · {}", product.name),
        20,
        true,
    );
    let mut grid = Grid::new(
        24,
        88,
        832,
        248,
        &["Tanggal", "Jenis", "Faktur", "Masuk", "Keluar", "Saldo"],
    );
    grid.set(rows);
    text(
        24,
        352,
        832,
        24,
        "Maksimum 500 pergerakan terbaru untuk obat ini.",
        12,
        false,
    );
    let mut close = button(24, 380, 112, "Tutup", false);
    let mut w = win.clone();
    close.set_callback(move |_| close_modal(&mut w));
    win.end();
    win.make_modal(true);
    win.set_callback(close_modal);
    win.show();
    Ok(())
}

impl MasterObatView {
    pub fn new(pool: Option<Pool>) -> Self {
        let mut win = Window::new(0, 108, 1200, 612, None);
        win.set_color(T::BG_APP);
        page_header(
            "Obat dan persediaan",
            "Filter obat, pilih beberapa baris dengan Shift, atau klik kanan untuk tindakan obat.",
        );
        let mut search = input(28, 88, 248, "Nama / barcode", "");
        search.set_trigger(CallbackTrigger::EnterKey);
        fn filter(x: i32, w: i32, label: &str) -> Choice {
            text(x, 88, w, 18, label, 12, false);
            let mut c = Choice::new(x, 112, w, 34, None);
            style_choice(&mut c);
            c
        }
        let mut category = filter(292, 172, "Kategori");
        let mut supplier = filter(480, 224, "Suplier");
        let mut unit = filter(720, 156, "Satuan beli");
        let mut stock = filter(892, 116, "Stok");
        let mut expiry = filter(1024, 148, "ED");
        let (categories, units) = db::product_filter_options(pool.as_ref()).unwrap_or_default();
        category.add_choice("Semua kategori");
        for c in &categories {
            category.add_choice(&c.replace('|', " ").replace('/', " "));
        }
        category.set_value(0);
        let suppliers = if let Some(pool) = &pool {
            db::get_suppliers(pool).unwrap_or_default()
        } else {
            db::demo::master_rows("suppliers")
                .into_iter()
                .map(|r| {
                    (
                        r[0].parse().unwrap_or(0),
                        r[1].clone(),
                        r[2].clone(),
                        r[3].clone(),
                    )
                })
                .collect()
        };
        supplier.add_choice("Semua suplier");
        for (_, name, _, _) in &suppliers {
            supplier.add_choice(&name.replace('|', " ").replace('/', " "));
        }
        supplier.set_value(0);
        unit.add_choice("Semua satuan");
        for u in &units {
            unit.add_choice(&u.replace('|', " ").replace('/', " "));
        }
        unit.set_value(0);
        stock.add_choice("Semua|Tersedia|Hampir habis|Kosong");
        stock.set_value(0);
        expiry.add_choice("Semua|Kedaluwarsa|Dalam 30 hari");
        expiry.set_value(0);
        let mut grid = Grid::new(
            28,
            166,
            1144,
            358,
            &[
                "ID",
                "No.",
                "Kode",
                "Obat",
                "Kategori",
                "Stok",
                "Beli (Rp)",
                "Sat. beli",
                "Jual (Rp)",
                "Satuan",
                "Suplier",
                "ED terdekat",
                "Batch",
            ],
        );
        for (i, w) in [0, 40, 156, 204, 84, 52, 100, 76, 100, 76, 104, 116, 100]
            .iter()
            .enumerate()
        {
            grid.table.set_col_width(i as i32, *w);
        }
        let products = Rc::new(RefCell::new(Vec::<Product>::new()));
        let page = Rc::new(std::cell::Cell::new(0usize));
        let pages = Rc::new(std::cell::Cell::new(1usize));
        let mut add = button(28, 552, 136, "Tambah obat", true);
        let mut edit = button(180, 552, 88, "Edit", false);
        let mut deactivate = button(284, 552, 128, "Nonaktifkan", false);
        let mut label = button(428, 552, 128, "Barcode", false);
        let mut refresh = button(572, 552, 96, "Refresh", false);
        let mut prev = button(684, 552, 44, "<", false);
        let mut next = button(740, 552, 44, ">", false);
        let mut paging = text(796, 552, 70, 34, "1/1", 12, false);
        let mut import = button(872, 552, 136, "Impor Excel", false);
        let mut export = button(1024, 552, 144, "Ekspor Excel", false);
        let status = text(28, 592, 1144, 18, "", 12, false);
        if pool.is_none() {
            for b in [&mut add, &mut edit, &mut deactivate, &mut import] {
                disable(b);
            }
        }
        let categories = Rc::new(RefCell::new(categories));
        let units = Rc::new(RefCell::new(units));
        let suppliers = Rc::new(RefCell::new(suppliers));
        let filter_value = {
            let search = search.clone();
            let category = category.clone();
            let supplier = supplier.clone();
            let unit = unit.clone();
            let stock = stock.clone();
            let expiry = expiry.clone();
            let page = page.clone();
            let categories = categories.clone();
            let units = units.clone();
            let suppliers = suppliers.clone();
            move || db::ProductFilter {
                search: search.value(),
                category: categories
                    .borrow()
                    .get((category.value() - 1) as usize)
                    .cloned()
                    .unwrap_or_default(),
                supplier_id: suppliers
                    .borrow()
                    .get((supplier.value() - 1) as usize)
                    .map(|r| r.0)
                    .unwrap_or(0),
                purchase_unit: units
                    .borrow()
                    .get((unit.value() - 1) as usize)
                    .cloned()
                    .unwrap_or_default(),
                stock: stock.value(),
                expiry: expiry.value(),
                page: page.get(),
            }
        };
        let update = {
            let pool = pool.clone();
            let data = products.clone();
            let page = page.clone();
            let pages = pages.clone();
            let mut g = grid.clone();
            let mut st = status.clone();
            let f = filter_value.clone();
            let suppliers = suppliers.clone();
            let categories = categories.clone();
            let units = units.clone();
            let mut category_choice = category.clone();
            let mut unit_choice = unit.clone();
            let mut supplier_choice = supplier.clone();
            move || {
                let selected = f();
                if let Ok((new_categories, new_units)) = db::product_filter_options(pool.as_ref()) {
                    category_choice.clear();
                    category_choice.add_choice("Semua kategori");
                    for value in &new_categories {
                        category_choice.add_choice(&value.replace('|', " ").replace('/', " "));
                    }
                    category_choice.set_value(
                        new_categories
                            .iter()
                            .position(|v| v == &selected.category)
                            .map(|i| i as i32 + 1)
                            .unwrap_or(0),
                    );
                    *categories.borrow_mut() = new_categories;
                    unit_choice.clear();
                    unit_choice.add_choice("Semua satuan");
                    for value in &new_units {
                        unit_choice.add_choice(&value.replace('|', " ").replace('/', " "));
                    }
                    unit_choice.set_value(
                        new_units
                            .iter()
                            .position(|v| v == &selected.purchase_unit)
                            .map(|i| i as i32 + 1)
                            .unwrap_or(0),
                    );
                    *units.borrow_mut() = new_units;
                }
                if let Some(pool) = &pool {
                    if let Ok(values) = db::get_suppliers(pool) {
                        supplier_choice.clear();
                        supplier_choice.add_choice("Semua suplier");
                        for (_, name, _, _) in &values {
                            supplier_choice.add_choice(&name.replace('|', " ").replace('/', " "));
                        }
                        supplier_choice.set_value(
                            values
                                .iter()
                                .position(|v| v.0 == selected.supplier_id)
                                .map(|i| i as i32 + 1)
                                .unwrap_or(0),
                        );
                        *suppliers.borrow_mut() = values;
                    }
                }
                match db::product_page(pool.as_ref(), &f()) {
                    Ok((rows, count)) => {
                        let total = (count + 99) / 100;
                        pages.set(total.max(1));
                        page.set(page.get().min(total.saturating_sub(1)));
                        paging.set_label(&format!("{}/{}", page.get() + 1, total.max(1)));
                        let display = rows
                            .iter()
                            .enumerate()
                            .map(|(i, p)| {
                                vec![
                                    p.id.to_string(),
                                    (page.get() * 100 + i + 1).to_string(),
                                    p.code.clone(),
                                    p.name.clone(),
                                    p.category.clone(),
                                    p.total_stock.to_string(),
                                    (p.buy_price
                                        * p.units
                                            .iter()
                                            .find(|u| {
                                                u.unit_name.eq_ignore_ascii_case(&p.purchase_unit)
                                            })
                                            .map(|u| u.conversion_qty)
                                            .unwrap_or(1)
                                            as f64)
                                        .to_string(),
                                    p.purchase_unit.clone(),
                                    p.units
                                        .iter()
                                        .find(|u| u.conversion_qty == 1)
                                        .map(|u| u.sell_price)
                                        .unwrap_or(0.0)
                                        .to_string(),
                                    p.base_unit.clone(),
                                    p.supplier_id
                                        .and_then(|id| {
                                            suppliers
                                                .borrow()
                                                .iter()
                                                .find(|s| s.0 == id)
                                                .map(|s| s.1.clone())
                                        })
                                        .unwrap_or_else(|| "—".into()),
                                    p.earliest_expiry().unwrap_or("—").into(),
                                    p.batches
                                        .first()
                                        .map(|b| b.batch_no.clone())
                                        .unwrap_or_else(|| "—".into()),
                                ]
                            })
                            .collect();
                        *data.borrow_mut() = rows;
                        g.set(display);
                        st.set_label(&format!(
                            "{count} obat · 100 per halaman · Geser horizontal untuk kolom batch."
                        ));
                    }
                    Err(e) => {
                        g.set(vec![]);
                        st.set_label(&e);
                    }
                }
            }
        };
        for c in [
            &mut category,
            &mut supplier,
            &mut unit,
            &mut stock,
            &mut expiry,
        ] {
            let page = page.clone();
            let mut u = update.clone();
            c.set_callback(move |_| {
                page.set(0);
                u();
            });
        }
        let page_search = page.clone();
        let mut u = update.clone();
        search.set_callback(move |_| {
            page_search.set(0);
            u();
        });
        let mut u = update.clone();
        refresh.set_callback(move |_| u());
        let p = page.clone();
        let mut u = update.clone();
        prev.set_callback(move |_| {
            p.set(p.get().saturating_sub(1));
            u();
        });
        let p = page.clone();
        let ps = pages.clone();
        let mut u = update.clone();
        next.set_callback(move |_| {
            p.set((p.get() + 1).min(ps.get().saturating_sub(1)));
            u();
        });
        let selected = {
            let data = products.clone();
            let table = grid.table.clone();
            move || {
                let (a, _, b, _) = table.get_selection();
                if a < 0 {
                    Vec::new()
                } else {
                    data.borrow()
                        .iter()
                        .skip(a as usize)
                        .take((b - a + 1).max(1) as usize)
                        .cloned()
                        .collect::<Vec<_>>()
                }
            }
        };
        let p = pool.clone();
        let u = update.clone();
        add.set_callback(move |_| {
            if let Some(p) = &p {
                product_form(p.clone(), None, u.clone());
            }
        });
        let p = pool.clone();
        let choose = selected.clone();
        let u = update.clone();
        let mut st = status.clone();
        edit.set_callback(move |_| {
            if let Some(product) = choose().first().cloned() {
                if let Some(p) = &p {
                    product_form(p.clone(), Some(product), u.clone());
                }
            } else {
                st.set_label("Pilih obat");
            }
        });
        let p = pool.clone();
        let choose = selected.clone();
        let mut st = status.clone();
        label.set_callback(move |_| {
            if let Err(e) = super::barcode_labels::show(choose(), p.clone()) {
                st.set_label(&e);
            }
        });
        let deactivate_action = {
            let p = pool.clone();
            let mut u = update.clone();
            let mut st = status.clone();
            move |selected: Vec<Product>| {
                if selected.is_empty() {
                    st.set_label("Pilih obat");
                    return;
                }
                let Some(p) = &p else {
                    return;
                };
                if fltk::dialog::choice2_default(
                    &format!(
                        "Nonaktifkan {} obat? Riwayat transaksi dipertahankan.",
                        selected.len()
                    ),
                    "Batal",
                    "Nonaktifkan",
                    "",
                ) == Some(1)
                {
                    if let Err(e) =
                        db::delete_products(p, &selected.iter().map(|p| p.id).collect::<Vec<_>>())
                    {
                        st.set_label(&e.to_string());
                        return;
                    }
                    u();
                }
            }
        };
        let choose = selected.clone();
        let mut act = deactivate_action.clone();
        deactivate.set_callback(move |_| act(choose()));
        let p = pool.clone();
        let u = update.clone();
        let choose = selected.clone();
        let data = products.clone();
        let mut act = deactivate_action;
        let mut st = status.clone();
        let selection_anchor = Rc::new(std::cell::Cell::new(-1));
        grid.table.handle(move |table, event| {
            if select_rows(table, event, &selection_anchor) {
                return true;
            }
            if event != Event::Push {
                return false;
            }
            let Some((context, row, _, _)) = table.cursor2rowcol() else {
                return false;
            };
            if context != TableContext::Cell || row < 0 {
                return false;
            }
            if fltk::app::event_button() != 3 {
                return false;
            }
            let Some(product) = data.borrow().get(row as usize).cloned() else {
                return false;
            };
            let (a, _, b, _) = table.get_selection();
            if row < a || row > b {
                table.set_selection(row, 0, row, table.cols() - 1);
            }
            table.redraw();
            let menu = MenuItem::new(&[
                "Edit obat",
                "Nonaktifkan",
                "Refresh",
                "Kartu stok",
                "Barcode",
                "Stok opname",
                "Isi barcode kemasan",
                "Batch dan ED",
            ]);
            if p.is_none() {
                for index in [0, 1, 5, 6] {
                    if let Some(mut item) = menu.at(index) {
                        item.set_flag(MenuFlag::Inactive);
                    }
                }
            }
            if let Some(choice) = menu.popup(fltk::app::event_x(), fltk::app::event_y()) {
                match choice.label().as_deref() {
                    Some("Edit obat") => {
                        if let Some(p) = &p {
                            product_form(p.clone(), Some(product), u.clone());
                        }
                    }
                    Some("Nonaktifkan") => act(choose()),
                    Some("Refresh") => {
                        let mut u = u.clone();
                        u();
                    }
                    Some("Kartu stok") => {
                        if let Err(e) = stock_card(p.as_ref(), &product) {
                            st.set_label(&e);
                        }
                    }
                    Some("Barcode") => {
                        if let Err(e) = super::barcode_labels::show(choose(), p.clone()) {
                            st.set_label(&e);
                        }
                    }
                    Some("Stok opname") => {
                        if let Some(p) = &p {
                            match super::stok_opname::adjust_stock(
                                p,
                                product.id,
                                &product.name,
                                &product.base_unit,
                                product.total_stock,
                            ) {
                                Ok(true) => {
                                    let mut u = u.clone();
                                    u();
                                }
                                Ok(false) => (),
                                Err(e) => st.set_label(&e),
                            }
                        }
                    }
                    Some("Isi barcode kemasan") => {
                        if let Some(p) = &p {
                            edit_barcode(p.clone(), product, u.clone());
                        }
                    }
                    Some("Batch dan ED") => batch_details(&product),
                    _ => (),
                }
            }
            true
        });
        let p = pool.clone();
        let u = update.clone();
        import.set_callback(move |_| {
            if let Some(p) = &p {
                super::import_products::show(p.clone(), u.clone());
            }
        });
        let p = pool.clone();
        let f = filter_value;
        let mut st = status.clone();
        export.set_callback(move |_| {
            if let Some(path) = save_path("Ekspor hasil filter obat", "Excel\t*.xlsx") {
                let mut f = f();
                f.page = 0;
                let result = (|| -> Result<(), String> {
                    let (mut rows, count) = db::product_page(p.as_ref(), &f)?;
                    for page in 1..(count + 99) / 100 {
                        f.page = page;
                        rows.extend(db::product_page(p.as_ref(), &f)?.0);
                    }
                    crate::services::export_products_to_excel(&rows, path)
                })();
                st.set_label(&match result {
                    Ok(()) => "Master obat diekspor".into(),
                    Err(e) => e,
                });
            }
        });
        let mut u = update.clone();
        win.handle(move |_, event| {
            if event == Event::Show {
                u();
            }
            false
        });
        let mut u = update;
        u();
        win.end();
        Self { win }
    }
}
