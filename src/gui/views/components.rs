use crate::gui::theme::AppTheme as T;
use fltk::{
    button::Button,
    draw,
    enums::{Align, Color, Font, FrameType},
    frame::Frame,
    input::Input,
    prelude::*,
    table::{Table, TableContext},
};
use std::{cell::RefCell, rc::Rc};

pub fn text(x: i32, y: i32, w: i32, h: i32, label: &str, size: i32, bold: bool) -> Frame {
    let mut f = Frame::new(x, y, w, h, None);
    f.set_label(label);
    f.set_label_size(size);
    f.set_label_color(T::TEXT_BODY);
    f.set_label_font(if bold {
        Font::HelveticaBold
    } else {
        Font::Helvetica
    });
    f.set_align(Align::Left | Align::Inside);
    f
}

pub fn button(x: i32, y: i32, w: i32, label: &str, primary: bool) -> Button {
    let mut b = Button::new(x, y, w, 34, None);
    b.set_label(label);
    b.set_frame(FrameType::FlatBox);
    b.set_label_size(13);
    b.set_color(if primary { T::PRIMARY } else { T::TABLE_HEADER });
    b.set_label_color(if primary { Color::White } else { T::TEXT_BODY });
    b.set_selection_color(T::PRIMARY_HOVER);
    b
}

pub fn input(x: i32, y: i32, w: i32, label: &str, value: &str) -> Input {
    text(x, y, w, 18, label, 12, false);
    let mut i = Input::new(x, y + 24, w, 34, None);
    i.set_value(value);
    i.set_text_size(14);
    i.set_frame(FrameType::BorderBox);
    i.set_color(Color::White);
    i
}

#[derive(Clone)]
pub struct Grid {
    pub table: Table,
    pub rows: Rc<RefCell<Vec<Vec<String>>>>,
    pub headers: Rc<RefCell<Vec<String>>>,
}

impl Grid {
    pub fn new(x: i32, y: i32, w: i32, h: i32, headers: &[&str]) -> Self {
        let mut table = Table::new(x, y, w, h, None);
        table.set_frame(FrameType::BorderBox);
        table.set_color(Color::White);
        table.set_col_header(true);
        table.set_col_header_height(36);
        table.set_row_height_all(34);
        table.set_col_resize(true);
        table.end();
        let rows = Rc::new(RefCell::new(Vec::<Vec<String>>::new()));
        let header = Rc::new(RefCell::new(
            headers.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        ));
        let r = rows.clone();
        let hd = header.clone();
        table.draw_cell(move |t, ctx, row, col, x, y, w, h| {
            if !matches!(ctx, TableContext::ColHeader | TableContext::Cell) {
                return;
            }
            draw::push_clip(x, y, w, h);
            let selected = ctx == TableContext::Cell && t.is_selected(row, col);
            let bg = if ctx == TableContext::ColHeader {
                T::TABLE_HEADER
            } else if selected {
                T::BADGE_GREEN_BG
            } else {
                Color::White
            };
            draw::draw_box(FrameType::FlatBox, x, y, w, h, bg);
            draw::set_draw_color(T::BORDER);
            draw::draw_line(x, y + h - 1, x + w, y + h - 1);
            draw::set_draw_color(T::TEXT_BODY);
            draw::set_font(
                if ctx == TableContext::ColHeader {
                    Font::HelveticaBold
                } else {
                    Font::Helvetica
                },
                12,
            );
            let value = if ctx == TableContext::ColHeader {
                hd.borrow().get(col as usize).cloned()
            } else {
                r.borrow()
                    .get(row as usize)
                    .and_then(|r| r.get(col as usize))
                    .cloned()
            };
            let numeric = hd
                .borrow()
                .get(col as usize)
                .map(|s| s.contains("(Rp)") || s == "Qty" || s == "Stok" || s == "Saldo")
                .unwrap_or(false);
            draw::draw_text2(
                value.as_deref().unwrap_or(""),
                x + 12,
                y,
                w - 24,
                h,
                if numeric { Align::Right } else { Align::Left },
            );
            draw::pop_clip();
        });
        let mut grid = Self {
            table,
            rows,
            headers: header,
        };
        grid.columns(headers);
        grid
    }
    pub fn columns(&mut self, headers: &[&str]) {
        *self.headers.borrow_mut() = headers.iter().map(|s| s.to_string()).collect();
        self.table.set_cols(headers.len() as i32);
        let has_id = headers.first() == Some(&"ID");
        let width = (self.table.w() - 16 - if has_id { 52 } else { 0 })
            / (headers.len() as i32 - if has_id { 1 } else { 0 }).max(1);
        for (i, h) in headers.iter().enumerate() {
            self.table
                .set_col_width(i as i32, if *h == "ID" { 52 } else { width.max(100) });
        }
    }
    pub fn set(&mut self, rows: Vec<Vec<String>>) {
        self.table.set_rows(rows.len() as i32);
        *self.rows.borrow_mut() = rows;
        self.table.redraw();
    }
    pub fn selected(&self) -> Option<Vec<String>> {
        let (row, _, _, _) = self.table.get_selection();
        if row < 0 {
            None
        } else {
            self.rows.borrow().get(row as usize).cloned()
        }
    }
}

pub fn save_path(title: &str, filter: &str) -> Option<std::path::PathBuf> {
    let mut chooser =
        fltk::dialog::NativeFileChooser::new(fltk::dialog::FileDialogType::BrowseSaveFile);
    chooser.set_title(title);
    chooser.set_filter(filter);
    chooser.set_option(fltk::dialog::FileDialogOptions::SaveAsConfirm);
    chooser.show();
    let p = chooser.filename();
    if p.as_os_str().is_empty() {
        None
    } else {
        Some(p)
    }
}
