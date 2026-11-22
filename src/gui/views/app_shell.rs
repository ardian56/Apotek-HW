use super::{
    components::*, dashboard::DashboardView, kasir::KasirView, master_crud::MasterSimpleView,
    master_obat::MasterObatView, pembelian::PembelianView, reports::ReportsView,
    settings::SettingsView, stok_opname::StokOpnameView, transactions,
};
use crate::{db::operations::ReportKind as R, gui::theme::AppTheme as T, models::User};
use fltk::{
    button::Button,
    enums::{Color, Event, FrameType, Key},
    group::Group,
    prelude::*,
    window::Window,
};
use mysql::Pool;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

pub struct AppShell {
    pub win: Window,
}

fn page(name: &str, pool: Option<Pool>, user: User) -> Window {
    match name {
        "Ringkasan" => DashboardView::new(pool).win,
        "Penjualan" => {
            ReportsView::new(
                pool,
                user,
                vec![R::Sales, R::Receivables, R::Payments, R::Returns],
            )
            .win
        }
        "Pembelian" => {
            let mut view = PembelianView::new(pool.clone());
            transactions::bind_purchase(&mut view, pool, user);
            view.win
        }
        "Daftar pembelian" => {
            ReportsView::new(
                pool,
                user,
                vec![R::Purchases, R::Debts, R::Payments, R::Returns],
            )
            .win
        }
        "Hutang / piutang" => {
            ReportsView::new(pool, user, vec![R::Debts, R::Receivables, R::Payments]).win
        }
        "Obat" => MasterObatView::new(pool).win,
        "Suplier" => MasterSimpleView::suplier(pool).win,
        "Pelanggan" => MasterSimpleView::pelanggan(pool).win,
        "Dokter" => MasterSimpleView::dokter(pool).win,
        "Operator" => MasterSimpleView::karyawan(pool).win,
        "Stok opname" => StokOpnameView::new(pool).win,
        "Kartu stok" => ReportsView::new(pool, user, vec![R::StockCards, R::Stock]).win,
        "Laporan" => {
            ReportsView::new(
                pool,
                user,
                vec![
                    R::Daily,
                    R::Sales,
                    R::Purchases,
                    R::TopProducts,
                    R::TopCustomers,
                    R::Stock,
                    R::StockCards,
                    R::Payments,
                    R::Returns,
                ],
            )
            .win
        }
        _ => SettingsView::new(pool, user).win,
    }
}

impl AppShell {
    pub fn new(pool: Option<Pool>) -> Self {
        Self::for_user(
            pool,
            User {
                id: 0,
                username: "preview".into(),
                full_name: "Pratinjau".into(),
                role: "ADMIN".into(),
            },
        )
    }
    pub fn for_user(pool: Option<Pool>, user: User) -> Self {
        let mut win = Window::default()
            .with_size(1200, 720)
            .with_label("Apotek POS");
        win.set_color(T::BG_APP);
        let mut header = Group::new(0, 0, 1200, 50, None);
        header.set_frame(FrameType::FlatBox);
        header.set_color(Color::White);
        let mut brand = text(24, 10, 140, 30, "+ APOTEK POS", 16, true);
        brand.set_label_color(T::PRIMARY);
        let mut nav = Vec::new();
        for (i, label) in [
            "Ringkasan",
            "Transaksi",
            "Master data",
            "Laporan",
            "Pengaturan",
        ]
        .iter()
        .enumerate()
        {
            nav.push(button(182 + i as i32 * 124, 8, 116, label, false));
        }
        text(806, 10, 176, 30, &user.full_name, 12, false);
        let mut cashier = button(998, 8, 178, "Buka kasir [F1]", true);
        header.end();
        let mut ribbon = Group::new(0, 50, 1200, 58, None);
        ribbon.set_frame(FrameType::FlatBox);
        ribbon.set_color(T::BG_APP);
        let mut pills = Vec::<Button>::new();
        for i in 0..8 {
            pills.push(button(24 + i * 144, 62, 136, "", false));
        }
        ribbon.end();
        let mut line = fltk::frame::Frame::new(0, 107, 1200, 1, None);
        line.set_frame(FrameType::FlatBox);
        line.set_color(T::BORDER);
        win.end();
        let categories: Vec<Vec<&'static str>> = vec![
            vec!["Ringkasan"],
            vec![
                "Penjualan",
                "Pembelian",
                "Daftar pembelian",
                "Hutang / piutang",
            ],
            vec![
                "Obat",
                "Suplier",
                "Pelanggan",
                "Dokter",
                "Operator",
                "Stok opname",
                "Kartu stok",
            ],
            vec!["Laporan"],
            vec!["Pengaturan"],
        ];
        let cache = Rc::new(RefCell::new(BTreeMap::<String, Window>::new()));
        let select = {
            let cache = cache.clone();
            let pool = pool.clone();
            let user = user.clone();
            let mut parent = win.clone();
            move |name: &str| {
                for w in cache.borrow_mut().values_mut() {
                    w.hide();
                }
                if !cache.borrow().contains_key(name) {
                    parent.begin();
                    let w = page(name, pool.clone(), user.clone());
                    parent.end();
                    cache.borrow_mut().insert(name.into(), w);
                }
                if let Some(w) = cache.borrow_mut().get_mut(name) {
                    w.show();
                }
            }
        };
        let change_category = {
            let categories = categories.clone();
            let mut pills = pills.clone();
            let mut nav = nav.clone();
            let select = select.clone();
            let user = user.clone();
            move |index: usize| {
                for (i, b) in nav.iter_mut().enumerate() {
                    b.set_color(if i == index { T::PRIMARY } else { T::BG_APP });
                    b.set_label_color(if i == index {
                        Color::White
                    } else {
                        T::TEXT_BODY
                    });
                    b.redraw();
                }
                let all_pills = pills.clone();
                for (i, b) in pills.iter_mut().enumerate() {
                    if let Some(name) = categories[index].get(i) {
                        b.set_label(name);
                        b.set_color(if i == 0 { T::PRIMARY } else { T::TABLE_HEADER });
                        b.set_label_color(if i == 0 { Color::White } else { T::TEXT_BODY });
                        b.show();
                        let forbidden = (!user.is_admin()
                            && matches!(*name, "Operator" | "Pengaturan"))
                            || (!user.can_access_reports()
                                && matches!(
                                    *name,
                                    "Pembelian"
                                        | "Daftar pembelian"
                                        | "Hutang / piutang"
                                        | "Stok opname"
                                        | "Kartu stok"
                                        | "Laporan"
                                ));
                        if forbidden {
                            b.deactivate();
                        } else {
                            b.activate();
                        }
                        let select = select.clone();
                        let mut siblings = all_pills.clone();
                        let name = *name;
                        b.set_callback(move |_| {
                            for (j, b) in siblings.iter_mut().enumerate() {
                                b.set_color(if i == j { T::PRIMARY } else { T::TABLE_HEADER });
                                b.set_label_color(if i == j { Color::White } else { T::TEXT_BODY });
                                b.redraw();
                            }
                            select(name);
                        });
                    } else {
                        b.hide();
                    }
                }
                let first = categories[index].iter().find(|name| {
                    !(!user.is_admin() && matches!(**name, "Operator" | "Pengaturan"))
                        && !(!user.can_access_reports()
                            && matches!(
                                **name,
                                "Pembelian"
                                    | "Daftar pembelian"
                                    | "Hutang / piutang"
                                    | "Stok opname"
                                    | "Kartu stok"
                                    | "Laporan"
                            ))
                });
                if let Some(name) = first {
                    select(name);
                }
            }
        };
        for (i, b) in nav.iter_mut().enumerate() {
            let mut change = change_category.clone();
            b.set_callback(move |_| change(i));
        }
        if !user.can_access_reports() {
            nav[3].deactivate();
        }
        if !user.is_admin() {
            nav[4].deactivate();
        }
        let cashier_window = Rc::new(RefCell::new(None::<Window>));
        let active_cashier = cashier_window.clone();
        let p = pool.clone();
        let u = user.clone();
        cashier.set_callback(move |_| {
            if let Some(w) = active_cashier.borrow_mut().as_mut() {
                w.show();
                return;
            }
            let mut view = KasirView::new();
            transactions::bind_cashier(&mut view, p.clone(), u.clone());
            view.win.show();
            *active_cashier.borrow_mut() = Some(view.win);
        });
        let mut open = cashier.clone();
        win.handle(move |_, event| {
            if matches!(event, Event::KeyDown | Event::Shortcut)
                && fltk::app::event_key() == Key::F1
            {
                open.do_callback();
                return true;
            }
            false
        });
        let mut change = change_category;
        change(0);
        Self { win }
    }
}
