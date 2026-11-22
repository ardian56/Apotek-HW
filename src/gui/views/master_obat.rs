use super::{components::*, form_obat::FormObatModal};
use crate::{
    db::{self, operations, NewProductInput},
    gui::theme::AppTheme as T,
    models::Product,
};
use fltk::{
    enums::{CallbackTrigger, FrameType},
    menu::Choice,
    prelude::*,
    window::Window,
};
use mysql::Pool;
use std::{cell::RefCell, rc::Rc};

pub struct MasterObatView {
    pub win: Window,
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
    if let Some(p) = &existing {
        form.choice_kategori
            .add_choice(&p.category.replace('|', " "));
        form.choice_kategori
            .set_value(form.choice_kategori.size() - 2);
        form.input_satuan_dasar.deactivate();
        form.input_stok_awal.set_value("0");
        form.input_stok_awal.deactivate();
        form.input_no_batch.deactivate();
        form.input_expired.deactivate();
    }
    form.win.begin();
    let mut status = text(16, 632, 648, 22, "", 12, false);
    form.win.end();
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
                category: f.choice_kategori.choice().unwrap_or_else(|| "Umum".into()),
                supplier_id: if f.choice_suplier.value() > 0 {
                    suppliers
                        .get((f.choice_suplier.value() - 1) as usize)
                        .map(|r| r.0)
                } else {
                    None
                },
                base_unit: base,
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
                w.hide();
                updated();
            }
            Err(e) => status.set_label(&format!("Simpan gagal: {e}")),
        }
    });
    form.win.show();
}

impl MasterObatView {
    pub fn new(pool: Option<Pool>) -> Self {
        let mut win = Window::new(0, 108, 1200, 612, None);
        win.set_color(T::BG_APP);
        text(28, 20, 900, 32, "Obat dan persediaan", 22, true);
        let mut search = input(28, 64, 392, "Nama / barcode", "");
        search.set_trigger(CallbackTrigger::EnterKey);
        text(444, 64, 240, 20, "Stok", 12, false);
        let mut stock = Choice::new(444, 88, 220, 34, None);
        stock.add_choice("Semua stok|Tersedia|Hampir habis|Kosong");
        stock.set_value(0);
        stock.set_frame(FrameType::BorderBox);
        text(688, 64, 240, 20, "Kedaluwarsa", 12, false);
        let mut expiry = Choice::new(688, 88, 260, 34, None);
        expiry.add_choice("Semua batch|Sudah kedaluwarsa|Dalam 30 hari");
        expiry.set_value(0);
        expiry.set_frame(FrameType::BorderBox);
        let mut refresh = button(968, 88, 200, "Tampilkan", true);
        let mut status = text(28, 134, 1140, 22, "", 12, false);
        let mut grid = Grid::new(
            28,
            170,
            1140,
            352,
            &[
                "ID",
                "Kode",
                "Obat",
                "Kategori",
                "Satuan",
                "Stok",
                "Harga beli (Rp)",
                "Harga jual (Rp)",
                "Status",
            ],
        );
        for (i, width) in [52, 115, 250, 130, 80, 60, 145, 145, 120]
            .iter()
            .enumerate()
        {
            grid.table.set_col_width(i as i32, *width);
        }
        let products = Rc::new(RefCell::new(Vec::<Product>::new()));
        let mut add = button(28, 552, 160, "Tambah obat", true);
        let mut edit = button(204, 552, 128, "Edit obat", false);
        let mut deactivate = button(348, 552, 160, "Nonaktifkan", false);
        let mut import = button(836, 552, 154, "Impor Excel", false);
        let mut export = button(1006, 552, 162, "Ekspor Excel", false);
        if pool.is_none() {
            for b in [
                &mut add,
                &mut edit,
                &mut deactivate,
                &mut import,
                &mut export,
            ] {
                b.deactivate();
            }
        }
        let update = {
            let pool = pool.clone();
            let s = search.clone();
            let st = stock.clone();
            let e = expiry.clone();
            let data = products.clone();
            let mut g = grid.clone();
            let mut status = status.clone();
            move || {
                let Some(p) = &pool else {
                    g.set(vec![]);
                    status.set_label("Pratinjau • Hubungkan database untuk menampilkan obat");
                    return;
                };
                match db::get_products(p, Some(&s.value())) {
                    Ok(rows) => {
                        let today = chrono::Local::now().date_naive();
                        let soon = today + chrono::Duration::days(30);
                        let today = today.to_string();
                        let soon = soon.to_string();
                        let rows = rows
                            .into_iter()
                            .filter(|p| match st.value() {
                                1 => p.total_stock > 0,
                                2 => p.total_stock > 0 && p.total_stock <= p.min_stock,
                                3 => p.total_stock == 0,
                                _ => true,
                            })
                            .filter(|p| match e.value() {
                                1 => p.batches.iter().any(|b| {
                                    b.expired_date
                                        .as_deref()
                                        .map(|d| d <= today.as_str())
                                        .unwrap_or(false)
                                }),
                                2 => p.batches.iter().any(|b| {
                                    b.expired_date
                                        .as_deref()
                                        .map(|d| d > today.as_str() && d <= soon.as_str())
                                        .unwrap_or(false)
                                }),
                                _ => true,
                            })
                            .collect::<Vec<_>>();
                        let display = rows
                            .iter()
                            .map(|p| {
                                let status = if p.total_stock == 0 {
                                    "Kosong"
                                } else if p.batches.iter().any(|b| {
                                    b.expired_date
                                        .as_deref()
                                        .map(|d| d <= today.as_str())
                                        .unwrap_or(false)
                                }) {
                                    "Ada expired"
                                } else if p.total_stock <= p.min_stock {
                                    "Stok kritis"
                                } else {
                                    "Tersedia"
                                };
                                vec![
                                    p.id.to_string(),
                                    p.code.clone(),
                                    p.name.clone(),
                                    p.category.clone(),
                                    p.base_unit.clone(),
                                    p.total_stock.to_string(),
                                    format!("{:.2}", p.buy_price),
                                    format!(
                                        "{:.2}",
                                        p.units
                                            .iter()
                                            .find(|u| u.conversion_qty == 1)
                                            .map(|u| u.sell_price)
                                            .unwrap_or(0.0)
                                    ),
                                    status.into(),
                                ]
                            })
                            .collect();
                        status.set_label(&format!(
                            "{} obat • Maksimum 200 hasil. Persempit pencarian untuk obat lainnya.",
                            rows.len()
                        ));
                        *data.borrow_mut() = rows;
                        g.set(display);
                    }
                    Err(e) => {
                        g.set(vec![]);
                        status.set_label(&format!("Obat gagal dimuat: {e}"));
                    }
                }
            }
        };
        let mut u = update.clone();
        refresh.set_callback(move |_| u());
        let mut u = update.clone();
        search.set_callback(move |_| u());
        let mut u = update.clone();
        stock.set_callback(move |_| u());
        let mut u = update.clone();
        expiry.set_callback(move |_| u());
        let p = pool.clone();
        let u = update.clone();
        add.set_callback(move |_| {
            if let Some(p) = &p {
                product_form(p.clone(), None, u.clone());
            }
        });
        let p = pool.clone();
        let data = products.clone();
        let g = grid.clone();
        let u = update.clone();
        let mut status_edit = status.clone();
        edit.set_callback(move |_| {
            if let Some(p) = &p {
                if let Some(row) = g.selected() {
                    if let Some(product) = data
                        .borrow()
                        .iter()
                        .find(|p| p.id.to_string() == row[0])
                        .cloned()
                    {
                        product_form(p.clone(), Some(product), u.clone());
                    }
                } else {
                    status_edit.set_label("Pilih obat pada tabel");
                }
            }
        });
        let p = pool.clone();
        let g = grid.clone();
        let mut u = update.clone();
        let mut st = status.clone();
        deactivate.set_callback(move |_| {
            let Some(p) = &p else {
                return;
            };
            let Some(row) = g.selected() else {
                st.set_label("Pilih obat pada tabel");
                return;
            };
            if fltk::dialog::choice2_default(
                &format!("Nonaktifkan {}? Riwayat transaksi tetap tersimpan.", row[2]),
                "Batal",
                "Nonaktifkan",
                "",
            ) != Some(1)
            {
                return;
            }
            match db::delete_product(p, row[0].parse().unwrap_or(0)) {
                Ok(()) => u(),
                Err(e) => st.set_label(&format!("Gagal menonaktifkan: {e}")),
            }
        });
        let data = products.clone();
        let mut st = status.clone();
        export.set_callback(move |_| {
            if let Some(path) = save_path("Ekspor master obat", "Excel\t*.xlsx") {
                match crate::services::export_products_to_excel(&data.borrow(), path) {
                    Ok(()) => st.set_label("Master obat diekspor"),
                    Err(e) => st.set_label(&format!("Ekspor gagal: {e}")),
                }
            }
        });
        let p = pool.clone();
        let mut st = status.clone();
        let mut u = update.clone();
        import.set_callback(move |_| {
            let Some(p) = &p else {
                return;
            };
            let mut chooser =
                fltk::dialog::NativeFileChooser::new(fltk::dialog::FileDialogType::BrowseFile);
            chooser.set_filter("Excel\t*.{xlsx,xls}");
            chooser.show();
            let path = chooser.filename();
            if path.as_os_str().is_empty() {
                return;
            }
            let result = crate::services::import_products_from_excel(path).and_then(|rows| {
                let inputs = rows
                    .into_iter()
                    .map(|r| NewProductInput {
                        code: r.code,
                        name: r.name,
                        category: r.category,
                        supplier_id: None,
                        base_unit: r.base_unit.clone(),
                        min_stock: r.min_stock,
                        buy_price: r.buy_price,
                        units: vec![(r.base_unit, 1, r.sell_price)],
                        batch_no: String::new(),
                        expired_date: None,
                        initial_stock: 0,
                    })
                    .collect::<Vec<_>>();
                if inputs.is_empty() {
                    return Err("File tidak berisi master obat".into());
                }
                db::import_product_inputs(p, &inputs)
                    .map_err(|e| e.to_string())
                    .map(|_| inputs.len())
            });
            match result {
                Ok(count) => {
                    u();
                    st.set_label(&format!(
                        "{count} obat diimpor. Tambahkan stok melalui pembelian atau stok opname."
                    ));
                }
                Err(e) => st.set_label(&format!("Impor dibatalkan: {e}")),
            }
        });
        let mut u = update.clone();
        win.handle(move |_, event| {
            if event == fltk::enums::Event::Show {
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
