use super::{
    components::*, dashboard::DashboardView, kasir::KasirView, master_crud::MasterSimpleView,
    master_obat::MasterObatView, pembelian::PembelianView, reports::ReportsView,
    settings::SettingsView, stok_opname::StokOpnameView, transactions,
};
use crate::{db::operations::ReportKind as R, gui::theme::AppTheme as T, models::User};
use fltk::{
    button::Button,
    enums::{Color, FrameType, Shortcut},
    group::Group,
    menu::{MenuButton, MenuFlag},
    prelude::*,
    window::Window,
};
use mysql::Pool;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

pub struct AppShell {
    pub win: Window,
}
type Views = Rc<RefCell<BTreeMap<&'static str, Window>>>;

fn open_page(name: &'static str, parent: Window, cache: Views, pool: Option<Pool>, user: User) {
    for w in cache.borrow_mut().values_mut() {
        w.hide();
    }
    if !cache.borrow().contains_key(name) {
        parent.begin();
        let win = match name {
            "Ringkasan" => DashboardView::new(pool.clone()).win,
            "Penjualan baru" => {
                let mut view = KasirView::new();
                transactions::bind_cashier(&mut view, pool.clone(), user.clone());
                let p = parent.clone();
                let c = cache.clone();
                let db = pool.clone();
                let u = user.clone();
                view.btn_back.set_callback(move |_| {
                    open_page("Penjualan", p.clone(), c.clone(), db.clone(), u.clone())
                });
                view.win
            }
            "Pembelian baru" => {
                let mut view = PembelianView::new(pool.clone());
                transactions::bind_purchase(&mut view, pool.clone(), user.clone());
                let p = parent.clone();
                let c = cache.clone();
                let db = pool.clone();
                let u = user.clone();
                view.btn_back.set_callback(move |_| {
                    open_page("Pembelian", p.clone(), c.clone(), db.clone(), u.clone())
                });
                view.win
            }
            "Penjualan" | "Pembelian" => {
                let kind = if name == "Penjualan" {
                    R::Sales
                } else {
                    R::Purchases
                };
                let mut view = ReportsView::new(pool.clone(), user.clone(), vec![kind]);
                let target = if name == "Penjualan" {
                    "Penjualan baru"
                } else {
                    "Pembelian baru"
                };
                let p = parent.clone();
                let c = cache.clone();
                let db = pool.clone();
                let u = user.clone();
                view.add.set_callback(move |_| {
                    open_page(target, p.clone(), c.clone(), db.clone(), u.clone())
                });
                view.win
            }
            "Hutang" => ReportsView::new(pool.clone(), user.clone(), vec![R::Debts]).win,
            "Piutang" => ReportsView::new(pool.clone(), user.clone(), vec![R::Receivables]).win,
            "Pembayaran" => ReportsView::new(pool.clone(), user.clone(), vec![R::Payments]).win,
            "Retur" => ReportsView::new(pool.clone(), user.clone(), vec![R::Returns]).win,
            "Obat" => MasterObatView::new(pool.clone()).win,
            "Suplier" => MasterSimpleView::suplier(pool.clone()).win,
            "Pelanggan" => MasterSimpleView::pelanggan(pool.clone()).win,
            "Dokter" => MasterSimpleView::dokter(pool.clone()).win,
            "Operator" => MasterSimpleView::karyawan(pool.clone()).win,
            "Stok opname" => StokOpnameView::new(pool.clone()).win,
            "Kartu stok" => {
                ReportsView::new(pool.clone(), user.clone(), vec![R::StockCards, R::Stock]).win
            }
            "Laporan" => {
                ReportsView::new(
                    pool.clone(),
                    user.clone(),
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
            _ => SettingsView::new(pool.clone(), user.clone()).win,
        };
        parent.end();
        cache.borrow_mut().insert(name, win);
    }
    if let Some(w) = cache.borrow_mut().get_mut(name) {
        w.show();
    }
}

impl AppShell {
    pub fn new(pool: Option<Pool>) -> Self {
        Self::for_user(
            pool,
            User {
                id: 0,
                username: "contoh".into(),
                full_name: "Mode contoh".into(),
                role: "ADMIN".into(),
            },
            fltk::app::quit,
        )
    }
    pub fn for_user(pool: Option<Pool>, user: User, mut on_logout: impl FnMut() + 'static) -> Self {
        let mut win = Window::default()
            .with_size(1200, 720)
            .with_label("Apotek POS");
        win.set_color(T::BG_APP);
        let mut header = Group::new(0, 0, 1200, 50, None);
        header.set_frame(FrameType::FlatBox);
        header.set_color(Color::White);
        let mut brand = text(28, 10, 168, 30, "+  Apotek POS", 17, true);
        brand.set_label_color(T::PRIMARY);
        if pool.is_none() {
            let mut mode = text(180, 10, 48, 30, "Contoh", 11, false);
            mode.set_label_color(T::TEXT_MUTED);
        }
        let mut nav = Vec::<Button>::new();
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
            let mut b = button(232 + i as i32 * 130, 8, 122, label, false);
            b.set_color(Color::White);
            nav.push(b);
        }
        let mut profile = MenuButton::new(1072, 8, 100, 34, "Akun");
        profile.set_frame(FrameType::BorderBox);
        profile.set_color(Color::White);
        profile.set_label_color(T::TEXT_BODY);
        profile.set_label_size(13);
        profile.set_selection_color(T::PRIMARY_SOFT);
        profile.add(
            &user.full_name.replace('/', " ").replace('|', " "),
            Shortcut::None,
            MenuFlag::Inactive,
            |_| {},
        );
        profile.add(
            &format!("{} · {}", user.username, user.role).replace('/', " "),
            Shortcut::None,
            MenuFlag::Inactive | MenuFlag::MenuDivider,
            |_| {},
        );
        header.end();
        let mut ribbon = Group::new(0, 50, 1200, 58, None);
        ribbon.set_frame(FrameType::FlatBox);
        ribbon.set_color(Color::White);
        let mut tabs = Vec::<Button>::new();
        for i in 0..8 {
            let mut b = button(28 + i * 142, 62, 134, "", false);
            b.set_color(Color::White);
            tabs.push(b);
        }
        ribbon.end();
        let mut line = fltk::frame::Frame::new(0, 107, 1200, 1, None);
        line.set_frame(FrameType::FlatBox);
        line.set_color(T::BORDER);
        win.end();
        let groups = vec![
            vec!["Ringkasan"],
            vec![
                "Penjualan",
                "Pembelian",
                "Hutang",
                "Piutang",
                "Pembayaran",
                "Retur",
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
        let cache = Rc::new(RefCell::new(BTreeMap::<&'static str, Window>::new()));
        let change = {
            let groups = groups.clone();
            let mut tabs = tabs.clone();
            let mut nav = nav.clone();
            let parent = win.clone();
            let cache = cache.clone();
            let pool = pool.clone();
            let user = user.clone();
            move |index: usize| {
                for (i, b) in nav.iter_mut().enumerate() {
                    b.set_color(if i == index {
                        T::PRIMARY_SOFT
                    } else {
                        Color::White
                    });
                    b.set_label_color(if i == index { T::PRIMARY } else { T::TEXT_BODY });
                    b.redraw();
                }
                let all = tabs.clone();
                for (i, b) in tabs.iter_mut().enumerate() {
                    if let Some(name) = groups[index].get(i) {
                        let name = *name;
                        b.set_label(name);
                        b.set_color(if i == 0 {
                            T::PRIMARY_SOFT
                        } else {
                            Color::White
                        });
                        b.set_label_color(if i == 0 { T::PRIMARY } else { T::TEXT_BODY });
                        b.show();
                        let forbidden = (!user.is_admin()
                            && matches!(name, "Operator" | "Pengaturan"))
                            || (!user.can_access_reports()
                                && matches!(
                                    name,
                                    "Pembelian"
                                        | "Hutang"
                                        | "Stok opname"
                                        | "Kartu stok"
                                        | "Laporan"
                                ));
                        if forbidden {
                            b.deactivate();
                        } else {
                            b.activate();
                        }
                        let p = parent.clone();
                        let c = cache.clone();
                        let db = pool.clone();
                        let u = user.clone();
                        let mut siblings = all.clone();
                        b.set_callback(move |_| {
                            for (j, b) in siblings.iter_mut().enumerate() {
                                b.set_color(if i == j {
                                    T::PRIMARY_SOFT
                                } else {
                                    Color::White
                                });
                                b.set_label_color(if i == j { T::PRIMARY } else { T::TEXT_BODY });
                                b.redraw();
                            }
                            open_page(name, p.clone(), c.clone(), db.clone(), u.clone());
                        });
                    } else {
                        b.hide();
                    }
                }
                if let Some(name) = groups[index].first() {
                    open_page(
                        name,
                        parent.clone(),
                        cache.clone(),
                        pool.clone(),
                        user.clone(),
                    );
                }
            }
        };
        for (i, b) in nav.iter_mut().enumerate() {
            let mut change = change.clone();
            b.set_callback(move |_| change(i));
        }
        if !user.can_access_reports() {
            nav[3].deactivate();
        }
        if !user.is_admin() {
            nav[4].deactivate();
        }
        let mut parent = win.clone();
        let c = cache.clone();
        profile.add("Keluar akun", Shortcut::None, MenuFlag::Normal, move |_| {
            parent.hide();
            c.borrow_mut().clear();
            on_logout();
            fltk::app::delete_widget(parent.clone());
        });
        let mut change = change;
        change(0);
        Self { win }
    }
}
