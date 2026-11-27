use super::{
    components::*, dashboard::DashboardView, kasir::KasirView, master_crud::MasterSimpleView,
    master_obat::MasterObatView, pembelian::PembelianView, reports::ReportsView,
    settings::SettingsView, stok_opname::StokOpnameView, transactions,
};
use crate::{db::operations::ReportKind as R, gui::theme::AppTheme as T, models::User};
use fltk::{
    button::Button,
    draw,
    enums::{Align, Color, Event, Font, FrameType, Shortcut},
    frame::Frame,
    group::Group,
    menu::{MenuButton, MenuFlag},
    prelude::*,
    window::Window,
};
use mysql::Pool;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

pub struct AppShell {
    pub win: Window,
}
type Views = Rc<RefCell<BTreeMap<&'static str, Window>>>;

pub const SIDEBAR_WIDTH: i32 = 188;
pub const HEADER_HEIGHT: i32 = 56;
const GROUPS: [(&str, &[&str]); 5] = [
    ("Ringkasan", &[]),
    (
        "Transaksi",
        &[
            "Penjualan",
            "Pembelian",
            "Hutang",
            "Piutang",
            "Pembayaran",
            "Retur",
        ],
    ),
    (
        "Master data",
        &[
            "Obat",
            "Suplier",
            "Pelanggan",
            "Sales",
            "Operator",
            "Stok opname",
            "Kartu stok",
        ],
    ),
    ("Laporan", &[]),
    ("Pengaturan", &[]),
];

fn allowed(user: &User, name: &str) -> bool {
    (user.is_admin() || !matches!(name, "Operator" | "Pengaturan"))
        && (user.can_access_reports()
            || !matches!(
                name,
                "Pembelian"
                    | "Pembelian baru"
                    | "Hutang"
                    | "Stok opname"
                    | "Kartu stok"
                    | "Laporan"
            ))
}

fn nav_button(label: &str, child: bool, expandable: bool) -> Button {
    let mut b = Button::new(
        if child { 24 } else { 12 },
        0,
        if child { 152 } else { 164 },
        if child { 32 } else { 40 },
        None,
    );
    b.set_label(label);
    b.set_frame(FrameType::NoBox);
    b.set_color(Color::White);
    b.set_label_color(T::TEXT_BODY);
    let hovered = Rc::new(Cell::new(false));
    let hover = hovered.clone();
    b.handle(move |b, event| match event {
        Event::Enter | Event::Leave => {
            hover.set(event == Event::Enter);
            b.redraw();
            true
        }
        _ => false,
    });
    b.draw(move |b| {
        let bg = if hovered.get() && b.active_r() && b.color() == Color::White {
            T::TABLE_HEADER
        } else {
            b.color()
        };
        draw::draw_rbox(b.x(), b.y(), b.w(), b.h(), 6, true, bg);
        draw::set_draw_color(if b.active_r() {
            b.label_color()
        } else {
            T::TEXT_MUTED
        });
        draw::set_font(
            if child {
                Font::Helvetica
            } else {
                Font::HelveticaBold
            },
            13,
        );
        draw::draw_text2(
            &b.label(),
            b.x() + 14,
            b.y(),
            b.w() - 36,
            b.h(),
            Align::Left | Align::Inside,
        );
        if expandable {
            let x = b.x() + b.w() - 18;
            let y = b.y() + b.h() / 2;
            if b.value() {
                draw::draw_line(x - 3, y - 2, x, y + 1);
                draw::draw_line(x, y + 1, x + 3, y - 2);
            } else {
                draw::draw_line(x - 2, y - 3, x + 1, y);
                draw::draw_line(x + 1, y, x - 2, y + 3);
            }
        }
        if b.has_focus() && b.has_visible_focus() {
            draw::draw_rbox(
                b.x() + 1,
                b.y() + 1,
                b.w() - 2,
                b.h() - 2,
                6,
                false,
                T::PRIMARY,
            );
        }
    });
    b
}

struct Sidebar {
    headers: Vec<Button>,
    rows: Vec<Vec<(&'static str, Button)>>,
    active: &'static str,
    expanded: Option<usize>,
    breadcrumb: Frame,
    group: Group,
}

impl Sidebar {
    fn activate(&mut self, name: &'static str) {
        self.active = match name {
            "Penjualan baru" => "Penjualan",
            "Pembelian baru" => "Pembelian",
            _ => name,
        };
        let index = GROUPS
            .iter()
            .position(|(label, rows)| *label == self.active || rows.contains(&self.active))
            .unwrap_or(0);
        self.expanded = if GROUPS[index].1.is_empty() {
            None
        } else {
            Some(index)
        };
        self.breadcrumb.set_label(&if GROUPS[index].1.is_empty() {
            name.into()
        } else {
            format!("{} / {name}", GROUPS[index].0)
        });
        self.layout();
    }

    fn layout(&mut self) {
        let compact = self.group.h() < 728;
        let header_height = if compact { 36 } else { 40 };
        let row_height = if compact { 30 } else { 32 };
        let mut y = 116;
        for (i, header) in self.headers.iter_mut().enumerate() {
            header.resize(12, y, 164, header_height);
            header.set_value(self.expanded == Some(i));
            let selected = GROUPS[i].0 == self.active;
            let in_group = GROUPS[i].1.contains(&self.active);
            header.set_color(if selected {
                T::PRIMARY_SOFT
            } else {
                Color::White
            });
            header.set_label_color(if selected || in_group {
                T::PRIMARY
            } else {
                T::TEXT_BODY
            });
            y += header_height;
            if self.expanded == Some(i) {
                y += 4;
            }
            for (name, row) in &mut self.rows[i] {
                if self.expanded == Some(i) {
                    row.resize(24, y, 152, row_height);
                    row.set_color(if *name == self.active {
                        T::PRIMARY_SOFT
                    } else {
                        Color::White
                    });
                    row.set_label_color(if *name == self.active {
                        T::PRIMARY
                    } else {
                        T::TEXT_BODY
                    });
                    row.show();
                    y += row_height + 2;
                } else {
                    row.hide();
                }
            }
            y += 6;
        }
        self.group.redraw();
    }
}

#[derive(Clone)]
struct Pages {
    parent: Window,
    cache: Views,
    pool: Option<Pool>,
    user: User,
    sidebar: Rc<RefCell<Sidebar>>,
    brand: Frame,
}

impl Pages {
    fn open(&self, name: &'static str) {
        if !allowed(&self.user, name) {
            return;
        }
        let parent = self.parent.clone();
        let cache = self.cache.clone();
        let pool = self.pool.clone();
        let user = self.user.clone();
        self.sidebar.borrow_mut().activate(name);
        for w in cache.borrow_mut().values_mut() {
            w.hide();
        }
        if !cache.borrow().contains_key(name) {
            parent.begin();
            let mut win = match name {
                "Ringkasan" => DashboardView::new(pool.clone()).win,
                "Penjualan baru" => {
                    let mut view = KasirView::new();
                    transactions::bind_cashier(&mut view, pool.clone(), user.clone());
                    let pages = self.clone();
                    view.btn_back.set_callback(move |_| pages.open("Penjualan"));
                    view.win
                }
                "Pembelian baru" => {
                    let mut view = PembelianView::new(pool.clone());
                    transactions::bind_purchase(&mut view, pool.clone(), user.clone());
                    let pages = self.clone();
                    view.btn_back.set_callback(move |_| pages.open("Pembelian"));
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
                    let pages = self.clone();
                    view.add.set_callback(move |_| pages.open(target));
                    view.win
                }
                "Hutang" => ReportsView::new(pool.clone(), user.clone(), vec![R::Debts]).win,
                "Piutang" => ReportsView::new(pool.clone(), user.clone(), vec![R::Receivables]).win,
                "Pembayaran" => ReportsView::new(pool.clone(), user.clone(), vec![R::Payments]).win,
                "Retur" => ReportsView::new(pool.clone(), user.clone(), vec![R::Returns]).win,
                "Obat" => MasterObatView::new(pool.clone()).win,
                "Suplier" => MasterSimpleView::suplier(pool.clone()).win,
                "Pelanggan" => MasterSimpleView::pelanggan(pool.clone()).win,
                "Sales" => MasterSimpleView::sales(pool.clone()).win,
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
                            R::ProductSales,
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
                _ => {
                    let mut brand = self.brand.clone();
                    let mut window = parent.clone();
                    let pages = self.clone();
                    SettingsView::new(
                        pool.clone(),
                        user.clone(),
                        move |name| {
                            brand.set_label(&name);
                            brand.set_tooltip(&name);
                            brand.redraw();
                            window.set_label(&format!("{name} · Kasir"));
                        },
                        move || pages.open("Operator"),
                    )
                    .win
                }
            };
            fit_page(&mut win, &parent);
            parent.end();
            cache.borrow_mut().insert(name, win);
        }
        let page = cache.borrow().get(name).cloned();
        if let Some(mut w) = page {
            w.show();
        }
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
        let (_, _, screen_width, screen_height) = fltk::app::screen_work_area(0);
        let width = 1320.min(screen_width - 32);
        let height = 740.min(screen_height - 28);
        let name = crate::db::operations::pharmacy_name(pool.as_ref()).unwrap_or_else(|error| {
            eprintln!("Nama apotek gagal dimuat: {error}");
            "Apotek".into()
        });
        let mut win = Window::default()
            .with_size(width, height)
            .with_label(&format!("{name} · Kasir"));
        win.set_color(T::BG_APP);
        let mut sidebar_group = Group::new(0, 0, SIDEBAR_WIDTH, height, None);
        sidebar_group.set_frame(FrameType::FlatBox);
        sidebar_group.set_color(Color::White);
        let mut mark = Frame::new(22, 24, 28, 28, None);
        mark.draw(|f| {
            draw::draw_rbox(f.x(), f.y(), f.w(), f.h(), 7, true, T::PRIMARY);
            draw::set_draw_color(Color::White);
            draw::draw_rectf(f.x() + 12, f.y() + 6, 4, 16);
            draw::draw_rectf(f.x() + 6, f.y() + 12, 16, 4);
        });
        let mut brand = text(60, 22, 112, 36, &name, 14, true);
        brand.set_align(Align::Left | Align::Inside | Align::Wrap);
        brand.set_tooltip(&name);
        brand.set_label_color(T::TEXT_TITLE);
        let mut subtitle = text(24, 62, 152, 18, "Kasir & persediaan", 11, false);
        subtitle.set_label_color(T::TEXT_MUTED);
        let mut caption = text(26, 92, 144, 18, "Menu utama", 11, false);
        caption.set_label_color(T::TEXT_MUTED);
        let mut headers = Vec::new();
        let mut rows = Vec::new();
        for (label, children) in GROUPS {
            let mut header = nav_button(label, false, !children.is_empty());
            if children.is_empty() && !allowed(&user, label) {
                header.deactivate();
            }
            headers.push(header);
            rows.push(
                children
                    .iter()
                    .map(|name| {
                        let mut b = nav_button(name, true, false);
                        if !allowed(&user, name) {
                            b.deactivate();
                        }
                        (*name, b)
                    })
                    .collect::<Vec<_>>(),
            );
        }
        let mut account_line = Frame::new(20, height - 90, 148, 1, None);
        account_line.set_frame(FrameType::FlatBox);
        account_line.set_color(T::BORDER);
        let mut profile = MenuButton::new(20, height - 76, 148, 34, "Akun");
        profile.set_frame(FrameType::RFlatBox);
        profile.set_color(T::TABLE_HEADER);
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
            &format!("{} · {}", user.username, user.role)
                .replace('/', " ")
                .replace('|', " "),
            Shortcut::None,
            MenuFlag::Inactive | MenuFlag::MenuDivider,
            |_| {},
        );
        let mut role = text(
            26,
            height - 32,
            140,
            18,
            match user.role.to_uppercase().as_str() {
                "ADMIN" => "Administrator",
                "GUDANG" => "Gudang",
                "KASIR" => "Kasir",
                _ => "Operator",
            },
            11,
            false,
        );
        role.set_label_color(T::TEXT_MUTED);
        sidebar_group.end();
        let mut border = Frame::new(SIDEBAR_WIDTH - 1, 0, 1, height, None);
        border.set_frame(FrameType::FlatBox);
        border.set_color(T::BORDER);
        let mut header = Group::new(SIDEBAR_WIDTH, 0, width - SIDEBAR_WIDTH, HEADER_HEIGHT, None);
        header.set_frame(FrameType::FlatBox);
        header.set_color(Color::White);
        let breadcrumb = text(SIDEBAR_WIDTH + 28, 16, 680, 24, "Ringkasan", 12, false);
        let date = chrono::Local::now().format("%d/%m/%Y").to_string();
        let mut info = text(
            width - 280,
            16,
            252,
            24,
            &if pool.is_none() {
                format!("Mode contoh · {date}")
            } else {
                date
            },
            12,
            false,
        );
        info.set_align(Align::Right | Align::Inside);
        info.set_label_color(T::TEXT_MUTED);
        header.end();
        let mut line = Frame::new(
            SIDEBAR_WIDTH,
            HEADER_HEIGHT - 1,
            width - SIDEBAR_WIDTH,
            1,
            None,
        );
        line.set_frame(FrameType::FlatBox);
        line.set_color(T::BORDER);
        let mut content_area = Frame::new(
            SIDEBAR_WIDTH,
            HEADER_HEIGHT,
            width - SIDEBAR_WIDTH,
            height - HEADER_HEIGHT,
            None,
        );
        content_area.hide();
        win.resizable(&content_area);
        win.size_range(1024.min(width), 640.min(height), 0, 0);
        win.end();
        win.set_callback(close_application_window);
        let sidebar = Rc::new(RefCell::new(Sidebar {
            headers,
            rows,
            active: "Ringkasan",
            expanded: None,
            breadcrumb,
            group: sidebar_group,
        }));
        let pages = Pages {
            parent: win.clone(),
            cache: Rc::new(RefCell::new(BTreeMap::new())),
            pool,
            user,
            sidebar: sidebar.clone(),
            brand,
        };
        let layout = sidebar.clone();
        let mut resize_profile = profile.clone();
        let mut resize_header = header.clone();
        let mut breadcrumb = sidebar.borrow().breadcrumb.clone();
        win.resize_callback(move |window, _, _, width, height| {
            {
                let mut sidebar = layout.borrow_mut();
                sidebar.group.resize(0, 0, SIDEBAR_WIDTH, height);
                sidebar.layout();
            }
            account_line.resize(20, height - 90, 148, 1);
            resize_profile.resize(20, height - 76, 148, 34);
            role.resize(26, height - 32, 140, 18);
            border.resize(SIDEBAR_WIDTH - 1, 0, 1, height);
            resize_header.resize(SIDEBAR_WIDTH, 0, width - SIDEBAR_WIDTH, HEADER_HEIGHT);
            breadcrumb.resize(
                SIDEBAR_WIDTH + 28,
                16,
                (width - SIDEBAR_WIDTH - 332).max(120),
                24,
            );
            info.resize(width - 280, 16, 252, 24);
            line.resize(SIDEBAR_WIDTH, HEADER_HEIGHT - 1, width - SIDEBAR_WIDTH, 1);
            window.redraw();
        });
        enable_fullscreen(&mut win);
        {
            let mut nav = sidebar.borrow_mut();
            for (i, b) in nav.headers.iter_mut().enumerate() {
                let pages = pages.clone();
                b.set_callback(move |_| {
                    if GROUPS[i].1.is_empty() {
                        pages.open(GROUPS[i].0);
                    } else {
                        let mut nav = pages.sidebar.borrow_mut();
                        nav.expanded = if nav.expanded == Some(i) {
                            None
                        } else {
                            Some(i)
                        };
                        nav.layout();
                    }
                });
            }
            for group in &mut nav.rows {
                for (name, b) in group {
                    let name = *name;
                    let pages = pages.clone();
                    b.set_callback(move |_| pages.open(name));
                }
            }
        }
        let mut fullscreen_window = win.clone();
        profile.add(
            "Layar penuh (F11)",
            Shortcut::None,
            MenuFlag::Normal | MenuFlag::MenuDivider,
            move |_| {
                fullscreen_window.fullscreen(!fullscreen_window.fullscreen_active());
            },
        );
        let mut parent = win.clone();
        let cache = pages.cache.clone();
        profile.add("Keluar akun", Shortcut::None, MenuFlag::Normal, move |_| {
            parent.hide();
            cache.borrow_mut().clear();
            on_logout();
            fltk::app::delete_widget(parent.clone());
        });
        pages.open("Ringkasan");
        Self { win }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_keeps_cashier_and_warehouse_permissions() {
        let mut user = User {
            id: 1,
            username: "operator".into(),
            full_name: "Operator".into(),
            role: "KASIR".into(),
        };
        assert!(allowed(&user, "Penjualan baru"));
        assert!(allowed(&user, "Obat"));
        assert!(!allowed(&user, "Pembelian baru"));
        assert!(!allowed(&user, "Pengaturan"));
        user.role = "GUDANG".into();
        assert!(allowed(&user, "Pembelian baru"));
        assert!(allowed(&user, "Stok opname"));
        assert!(!allowed(&user, "Operator"));
        user.role = "ADMIN".into();
        assert!(allowed(&user, "Pengaturan"));
        assert!(allowed(&user, "Operator"));
    }
}
