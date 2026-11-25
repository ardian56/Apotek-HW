use super::components::*;
use crate::{
    db,
    gui::theme::AppTheme as T,
    models::Product,
    services::barcode::{self, LabelItem},
};
use fltk::{
    browser::MultiBrowser,
    enums::{Color, FrameType},
    menu::Choice,
    prelude::*,
    window::Window,
};
use mysql::Pool;
use std::{cell::RefCell, rc::Rc};

const FORMATS: [Option<(u32, u32, u32)>; 6] = [
    Some((30, 20, 3)),
    Some((40, 25, 3)),
    Some((50, 30, 3)),
    Some((58, 30, 1)),
    Some((80, 40, 1)),
    None,
];

fn add_products(queue: &mut Vec<(Product, i32)>, products: Vec<Product>) -> Result<usize, String> {
    let mut ids = queue
        .iter()
        .map(|(p, _)| p.id)
        .collect::<std::collections::HashSet<_>>();
    let additions = products
        .into_iter()
        .filter(|p| ids.insert(p.id))
        .collect::<Vec<_>>();
    if queue.len() + additions.len() > barcode::MAX_PRODUCTS {
        return Err("Maksimum 200 obat dalam antrean label".into());
    }
    if queue.iter().map(|(_, n)| *n as i64).sum::<i64>() + additions.len() as i64
        > barcode::MAX_LABELS as i64
    {
        return Err("Maksimum 1.000 label per dokumen cetak".into());
    }
    let added = additions.len();
    queue.extend(additions.into_iter().map(|p| (p, 1)));
    Ok(added)
}

fn set_quantity(
    queue: &mut [(Product, i32)],
    selected: &[usize],
    quantity: i32,
) -> Result<(), String> {
    if !(1..=barcode::MAX_LABELS).contains(&quantity) {
        return Err("Jumlah harus 1–1000".into());
    }
    if selected.is_empty() || selected.iter().any(|row| *row >= queue.len()) {
        return Err("Pilih obat pada antrean".into());
    }
    let total = queue
        .iter()
        .enumerate()
        .map(|(index, (_, n))| {
            if selected.contains(&index) {
                quantity as i64
            } else {
                *n as i64
            }
        })
        .sum::<i64>();
    if total > barcode::MAX_LABELS as i64 {
        return Err("Total melebihi 1.000 label. Kurangi jumlah per obat.".into());
    }
    for row in selected {
        queue[*row].1 = quantity;
    }
    Ok(())
}

fn choose_products(
    pool: Option<Pool>,
    mut selected: impl FnMut(Vec<Product>) -> Result<(), String> + 'static,
) {
    let mut win = Window::default()
        .with_size(760, 500)
        .with_label("Pilih banyak obat");
    win.set_color(T::BG_APP);
    text(24, 20, 712, 30, "Pilih obat untuk label", 22, true);
    let mut search = input(24, 64, 712, "Cari nama / kode", "");
    let mut list = MultiBrowser::new(24, 140, 712, 270, None);
    list.set_frame(FrameType::BorderBox);
    list.set_color(Color::White);
    list.set_text_size(13);
    list.set_selection_color(T::PRIMARY_SOFT);
    // FLTK rejects NUL. Printable medicine codes cannot start with this control character.
    list.set_format_char('\u{1f}');
    let status = text(
        24,
        412,
        712,
        20,
        "Ctrl untuk pilihan terpisah, Shift untuk rentang.",
        12,
        false,
    );
    let mut all = button(24, 438, 144, "Pilih semua", false);
    let mut cancel = button(420, 438, 140, "Batal", false);
    let mut add = button(576, 438, 160, "Tambah pilihan", true);
    let choices = Rc::new(RefCell::new(Vec::<Product>::new()));
    let load = {
        let search = search.clone();
        let choices = choices.clone();
        let mut list = list.clone();
        let mut status = status.clone();
        move || {
            list.clear();
            choices.borrow_mut().clear();
            match db::view_products(pool.as_ref(), Some(&search.value())) {
                Ok(products) => {
                    for p in &products {
                        list.add(&format!("{} · {}", p.code, p.name));
                    }
                    status.set_label(&format!("{} hasil · Ctrl/Shift untuk beberapa pilihan · maksimum 200 hasil pencarian", products.len()));
                    *choices.borrow_mut() = products;
                }
                Err(error) => status.set_label(&error),
            }
        }
    };
    let mut browser = list.clone();
    all.set_callback(move |_| {
        for row in 1..=browser.size() {
            browser.select(row);
        }
    });
    let mut w = win.clone();
    cancel.set_callback(move |_| close_modal(&mut w));
    let mut w = win.clone();
    let mut status = status.clone();
    add.set_callback(move |_| {
        let products = choices
            .borrow()
            .iter()
            .enumerate()
            .filter(|(i, _)| list.selected(*i as i32 + 1))
            .map(|(_, p)| p.clone())
            .collect::<Vec<_>>();
        if products.is_empty() {
            status.set_label("Pilih minimal satu obat");
            return;
        }
        match selected(products) {
            Ok(()) => close_modal(&mut w),
            Err(error) => status.set_label(&error),
        }
    });
    search.set_trigger(fltk::enums::CallbackTrigger::EnterKey);
    let mut l = load.clone();
    search.set_callback(move |_| l());
    let mut load = load;
    load();
    win.end();
    win.make_modal(true);
    win.set_callback(close_modal);
    win.show();
}

fn queue_items(queue: &[(Product, i32)]) -> Vec<LabelItem<'_>> {
    queue
        .iter()
        .map(|(p, n)| LabelItem {
            code: &p.code,
            name: &p.name,
            quantity: *n,
            price: p
                .units
                .iter()
                .find(|u| u.conversion_qty == 1)
                .map(|u| u.sell_price)
                .unwrap_or(0.0),
        })
        .collect()
}

fn preview_sheet(
    queue: Vec<(Product, i32)>,
    format: Option<(u32, u32, u32)>,
    dpi: u32,
) -> Result<(), String> {
    super::print_preview::show_barcodes(&queue_items(&queue), format, dpi)
}

pub fn show(products: Vec<Product>, pool: Option<Pool>) -> Result<(), String> {
    let mut initial = Vec::new();
    add_products(&mut initial, products)?;
    let mut win = Window::default()
        .with_size(980, 520)
        .with_label("Cetak barcode");
    win.set_color(T::BG_APP);
    text(24, 18, 680, 30, "Cetak barcode", 22, true);
    text(
        24,
        62,
        932,
        20,
        "Tambah obat, atur jumlah label, lalu periksa hasil sebelum mencetak.",
        12,
        false,
    );
    let mut pick = button(740, 18, 216, "Tambah obat", false);
    let mut grid = Grid::new(24, 106, 652, 292, &["Kode", "Obat", "Harga (Rp)", "Jumlah"]);
    for (i, w) in [164, 280, 124, 64].iter().enumerate() {
        grid.table.set_col_width(i as i32, *w);
    }
    text(704, 106, 252, 24, "Pengaturan label", 15, true);
    text(704, 146, 244, 18, "Ukuran label", 12, false);
    let mut size = Choice::new(704, 178, 252, 34, None);
    style_choice(&mut size);
    size.add_choice("30 × 20 mm · 3 kolom|40 × 25 mm · 3 kolom|50 × 30 mm · 3 kolom|58 × 30 mm · 1 kolom|80 × 40 mm · 1 kolom|Otomatis");
    size.set_value(2);
    let quantity = input(704, 228, 132, "Jumlah label", "1");
    text(852, 228, 104, 18, "DPI printer", 12, false);
    let mut dpi = Choice::new(852, 252, 104, 34, None);
    style_choice(&mut dpi);
    dpi.add_choice("203 dpi|300 dpi");
    dpi.set_value(0);
    let mut apply = button(704, 298, 252, "Ubah jumlah pilihan", false);
    apply.set_tooltip("Mengubah jumlah label hanya pada pilihan. Shift memilih rentang; Ctrl memilih baris terpisah.");
    let mut remove = button(704, 346, 252, "Hapus dari daftar", false);
    remove.set_tooltip("Menghapus pilihan dari daftar label. Data obat tetap tersimpan.");
    let mut reset = button(24, 444, 164, "Kosongkan daftar", false);
    let mut preview = button(792, 444, 164, "Lihat label", true);
    let mut close = button(656, 444, 120, "Tutup", false);
    let status = text(
        24,
        490,
        932,
        20,
        "Pilih beberapa baris dengan Shift. Nama, harga, dan kode tercetak pada label.",
        12,
        false,
    );
    let queue = Rc::new(RefCell::new(initial));
    let refresh = {
        let q = queue.clone();
        let mut g = grid.clone();
        let mut actions = [
            preview.clone(),
            apply.clone(),
            remove.clone(),
            reset.clone(),
        ];
        let mut status = status.clone();
        move || {
            let q = q.borrow();
            g.set(
                q.iter()
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
            for button in &mut actions {
                if q.is_empty() {
                    button.deactivate();
                } else {
                    button.activate();
                }
            }
            let (first, _, last, _) = g.table.get_selection();
            if q.is_empty() {
                g.clear_selection();
            } else if first < 0 || last as usize >= q.len() {
                g.select_first();
            }
            status.set_label(&format!(
                "{} obat · {} label · Shift: rentang · Ctrl: baris terpisah.",
                q.len(),
                q.iter().map(|(_, n)| *n).sum::<i32>()
            ));
        }
    };
    let q = queue.clone();
    let p = pool.clone();
    let r = refresh.clone();
    pick.set_callback(move |_| {
        let q = q.clone();
        let mut r = r.clone();
        choose_products(p.clone(), move |products| {
            add_products(&mut q.borrow_mut(), products)?;
            r();
            Ok(())
        });
    });
    let q = queue.clone();
    let g = grid.clone();
    let mut r = refresh.clone();
    let mut st = status.clone();
    apply.set_callback(move |_| {
        let selected = g.selected_rows();
        let result = quantity
            .value()
            .parse::<i32>()
            .map_err(|_| "Jumlah harus bilangan bulat".to_string())
            .and_then(|n| set_quantity(&mut q.borrow_mut(), &selected, n));
        match result {
            Ok(()) => r(),
            Err(error) => st.set_label(&error),
        }
    });
    let q = queue.clone();
    let g = grid.clone();
    let mut r = refresh.clone();
    remove.set_callback(move |_| {
        let selected = g.selected_rows();
        {
            let mut queue = q.borrow_mut();
            for row in selected.into_iter().rev() {
                if row < queue.len() {
                    queue.remove(row);
                }
            }
        }
        r();
    });
    let q = queue.clone();
    let mut r = refresh.clone();
    reset.set_callback(move |_| {
        q.borrow_mut().clear();
        r();
    });
    let q = queue.clone();
    let s = size.clone();
    let d = dpi.clone();
    let mut st = status.clone();
    preview.set_callback(move |_| {
        if let Err(error) = preview_sheet(
            q.borrow().clone(),
            FORMATS[s.value().max(0) as usize],
            if d.value() == 1 { 300 } else { 203 },
        ) {
            st.set_label(&error);
            st.set_tooltip(&error);
        }
    });
    let mut w = win.clone();
    close.set_callback(move |_| close_modal(&mut w));
    let mut r = refresh;
    r();
    win.end();
    win.make_modal(true);
    win.set_callback(close_modal);
    win.show();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mass_label_queue_deduplicates_and_rejects_changes_atomically() {
        let seed = db::view_products(None, None).unwrap().remove(0);
        let products = (1..=201)
            .map(|id| Product { id, ..seed.clone() })
            .collect::<Vec<_>>();
        let mut queue = Vec::new();
        assert_eq!(add_products(&mut queue, products[..3].to_vec()).unwrap(), 3);
        assert_eq!(add_products(&mut queue, products[..2].to_vec()).unwrap(), 0);
        set_quantity(&mut queue, &[0, 1], 2).unwrap();
        assert_eq!(queue.iter().map(|(_, n)| *n).collect::<Vec<_>>(), [2, 2, 1]);
        for (selected, quantity) in [
            (Vec::new(), 2),
            (vec![0, 3], 2),
            (vec![0], 0),
            (vec![0], 1001),
            (vec![0, 1, 2], 334),
        ] {
            assert!(set_quantity(&mut queue, &selected, quantity).is_err());
            assert_eq!(queue.iter().map(|(_, n)| *n).collect::<Vec<_>>(), [2, 2, 1]);
        }
        assert!(add_products(&mut queue, products.clone()).is_err());
        assert_eq!(queue.len(), 3);
        set_quantity(&mut queue, &[0, 2], 3).unwrap();
        assert_eq!(queue.iter().map(|(_, n)| *n).collect::<Vec<_>>(), [3, 2, 3]);
        set_quantity(&mut queue, &[0, 1, 2], 1).unwrap();
        set_quantity(&mut queue, &[0], 998).unwrap();
        assert!(add_products(&mut queue, vec![products[3].clone()]).is_err());
        assert_eq!(queue.len(), 3);
        set_quantity(&mut queue, &[0], 1).unwrap();
        assert_eq!(
            add_products(&mut queue, products[..200].to_vec()).unwrap(),
            197
        );
        assert_eq!(queue.len(), barcode::MAX_PRODUCTS);
    }
}
