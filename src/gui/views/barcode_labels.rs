use super::components::*;
use crate::{
    db,
    gui::theme::AppTheme as T,
    models::Product,
    services::{
        barcode::{self, LabelItem},
        documents,
    },
};
use fltk::{menu::Choice, prelude::*, window::Window};
use mysql::Pool;
use std::{cell::RefCell, rc::Rc};

pub fn show(products: Vec<Product>, pool: Option<Pool>) -> Result<(), String> {
    if products.is_empty() {
        return Err("Pilih obat untuk label barcode".into());
    }
    let mut win = Window::default()
        .with_size(980, 520)
        .with_label("Label barcode");
    win.set_color(T::BG_APP);
    text(24, 18, 920, 30, "Label barcode", 22, true);
    let search = input(24, 64, 300, "Cari nama / kode obat", "");
    let mut found = Choice::new(340, 88, 384, 34, None);
    style_choice(&mut found);
    let mut add = button(740, 88, 216, "Tambah ke daftar", false);
    let mut grid = Grid::new(24, 154, 652, 244, &["Kode", "Obat", "Harga (Rp)", "Label"]);
    for (i, w) in [164, 280, 124, 64].iter().enumerate() {
        grid.table.set_col_width(i as i32, *w);
    }
    text(704, 154, 244, 18, "Ukuran label", 12, false);
    let mut size = Choice::new(704, 178, 252, 34, None);
    style_choice(&mut size);
    size.add_choice("30 × 20 mm · 3 kolom|40 × 25 mm · 3 kolom|50 × 30 mm · 3 kolom|58 × 30 mm · 1 kolom|80 × 40 mm · 1 kolom|Otomatis");
    size.set_value(2);
    let quantity = input(704, 228, 132, "Label per obat", "1");
    text(852, 228, 104, 18, "DPI printer", 12, false);
    let mut dpi = Choice::new(852, 252, 104, 34, None);
    style_choice(&mut dpi);
    dpi.add_choice("203 dpi|300 dpi");
    dpi.set_value(0);
    let mut apply = button(704, 298, 252, "Terapkan jumlah", false);
    let mut remove = button(704, 346, 120, "Hapus", false);
    let mut reset = button(836, 346, 120, "Reset", false);
    let mut preview = button(24, 444, 164, "Pratinjau", true);
    let mut save = button(204, 444, 164, "Simpan label", false);
    let mut print = button(384, 444, 164, "Cetak label", false);
    let mut close = button(836, 444, 120, "Tutup", false);
    let status = text(
        24,
        490,
        932,
        20,
        "Pilih beberapa baris dengan Shift. Nama, harga, dan kode tercetak pada label.",
        12,
        false,
    );
    let queue = Rc::new(RefCell::new(
        products.into_iter().map(|p| (p, 1)).collect::<Vec<_>>(),
    ));
    let refresh = {
        let q = queue.clone();
        let mut g = grid.clone();
        move || {
            g.set(
                q.borrow()
                    .iter()
                    .map(|(p, n)| {
                        vec![
                            p.code.clone(),
                            p.name.clone(),
                            p.units
                                .iter()
                                .find(|u| u.conversion_qty == 1)
                                .map(|u| u.sell_price)
                                .unwrap_or(0.0)
                                .to_string(),
                            n.to_string(),
                        ]
                    })
                    .collect(),
            );
        }
    };
    let choices = Rc::new(RefCell::new(Vec::<Product>::new()));
    let load = {
        let s = search.clone();
        let p = pool.clone();
        let c = choices.clone();
        let mut f = found.clone();
        let mut st = status.clone();
        move || match db::view_products(p.as_ref(), Some(&s.value())) {
            Ok(data) => {
                f.clear();
                for p in &data {
                    f.add_choice(
                        &format!("{} · {}", p.code, p.name)
                            .replace('|', " ")
                            .replace('/', " "),
                    );
                }
                f.set_value(0);
                *c.borrow_mut() = data;
            }
            Err(e) => st.set_label(&e),
        }
    };
    let mut search = search;
    search.set_trigger(fltk::enums::CallbackTrigger::EnterKey);
    let mut l = load.clone();
    search.set_callback(move |_| l());
    let q = queue.clone();
    let c = choices.clone();
    let f = found.clone();
    let mut r = refresh.clone();
    add.set_callback(move |_| {
        if let Some(p) = c.borrow().get(f.value().max(0) as usize) {
            if !q.borrow().iter().any(|(v, _)| v.id == p.id) {
                q.borrow_mut().push((p.clone(), 1));
                r();
            }
        }
    });
    let q = queue.clone();
    let g = grid.clone();
    let mut r = refresh.clone();
    let mut st = status.clone();
    apply.set_callback(move |_| {
        let n = quantity.value().parse::<i32>();
        match n {
            Ok(n) if (1..=1000).contains(&n) => {
                let (a, _, b, _) = g.table.get_selection();
                if a < 0 {
                    st.set_label("Pilih obat pada daftar");
                    return;
                }
                for (_, v) in q
                    .borrow_mut()
                    .iter_mut()
                    .skip(a as usize)
                    .take((b - a + 1) as usize)
                {
                    *v = n;
                }
                r();
            }
            _ => st.set_label("Jumlah harus 1–1000"),
        }
    });
    let q = queue.clone();
    let g = grid.clone();
    let mut r = refresh.clone();
    remove.set_callback(move |_| {
        let (a, _, b, _) = g.table.get_selection();
        if a >= 0 {
            let end = ((b + 1) as usize).min(q.borrow().len());
            q.borrow_mut().drain(a as usize..end);
            r();
        }
    });
    let q = queue.clone();
    let mut r = refresh.clone();
    reset.set_callback(move |_| {
        q.borrow_mut().clear();
        r();
    });
    let render = {
        let q = queue.clone();
        let s = size.clone();
        let dpi = dpi.clone();
        move || {
            let q = q.borrow();
            let items = q
                .iter()
                .map(|(p, n)| LabelItem {
                    code: &p.code,
                    name: &p.name,
                    price: p
                        .units
                        .iter()
                        .find(|u| u.conversion_qty == 1)
                        .map(|u| u.sell_price)
                        .unwrap_or(0.0),
                    quantity: *n,
                })
                .collect::<Vec<_>>();
            let format = [
                Some((30, 20, 3)),
                Some((40, 25, 3)),
                Some((50, 30, 3)),
                Some((58, 30, 1)),
                Some((80, 40, 1)),
                None,
            ];
            barcode::labels_html_at_dpi(
                &items,
                format[s.value().max(0) as usize],
                if dpi.value() == 1 { 300 } else { 203 },
            )
        }
    };
    for b in [&mut preview, &mut print] {
        let render = render.clone();
        let mut st = status.clone();
        b.set_callback(
            move |_| match render().and_then(|h| documents::open_html(&h)) {
                Ok(()) => st.set_label("Pratinjau dibuka. Cetak dengan skala 100%."),
                Err(e) => st.set_label(&e),
            },
        );
    }
    let mut st = status.clone();
    save.set_callback(move |_| match render() {
        Ok(h) => {
            if let Some(path) = save_path("Simpan label barcode", "HTML\t*.html") {
                match std::fs::write(path, h) {
                    Ok(()) => st.set_label("Label disimpan"),
                    Err(e) => st.set_label(&e.to_string()),
                }
            }
        }
        Err(e) => st.set_label(&e),
    });
    let mut w = win.clone();
    close.set_callback(move |_| close_modal(&mut w));
    let mut r = refresh;
    r();
    grid.select_first();
    let mut l = load;
    l();
    win.end();
    win.make_modal(true);
    win.set_callback(close_modal);
    win.show();
    Ok(())
}
