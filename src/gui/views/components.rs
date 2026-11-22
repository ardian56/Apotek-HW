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
    b.set_frame(FrameType::RFlatBox);
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
    i.set_text_color(T::TEXT_BODY);
    i
}

pub fn disable(button: &mut Button) {
    button.deactivate();
    button.set_color(T::TABLE_HEADER);
    button.set_label_color(T::TEXT_MUTED);
}

pub fn close_modal(window: &mut fltk::window::Window) {
    window.hide();
    fltk::app::delete_widget(window.clone());
}

pub fn style_choice(choice: &mut fltk::menu::Choice) {
    choice.set_frame(FrameType::BorderBox);
    choice.set_down_frame(FrameType::BorderBox);
    choice.set_color(Color::White);
    choice.set_text_color(T::TEXT_BODY);
    choice.set_text_size(14);
}

pub fn page_header(title: &str, description: &str) {
    let mut heading = text(28, 20, 900, 32, title, 22, true);
    heading.set_label_color(T::TEXT_TITLE);
    let mut sub = text(28, 58, 1140, 20, description, 12, false);
    sub.set_label_color(T::TEXT_MUTED);
}

pub fn panel(x: i32, y: i32, w: i32, h: i32) -> Frame {
    let mut f = Frame::new(x, y, w, h, None);
    f.set_frame(FrameType::BorderBox);
    f.set_color(Color::White);
    f
}

pub fn number(value: f64) -> String {
    if !value.is_finite() {
        return "—".into();
    }
    let cents = (value.abs() * 100.0).round() as u64;
    let digits = (cents / 100).to_string();
    let mut result = String::new();
    if value < 0.0 && cents > 0 {
        result.push('-');
    }
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            result.push('.');
        }
        result.push(c);
    }
    if cents % 100 != 0 {
        result.push_str(&format!(",{:02}", cents % 100));
    }
    result
}

pub fn rupiah(value: f64) -> String {
    format!("Rp {}", number(value))
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
        // Fl_Table menggambar bingkai juga saat redraw seleksi; bingkai tidak boleh mengisi latar.
        table.set_frame(FrameType::BorderFrame);
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
                T::PRIMARY_SOFT
            } else {
                Color::White
            };
            draw::draw_box(FrameType::FlatBox, x, y, w, h, bg);
            draw::set_draw_color(T::BORDER);
            draw::draw_line(x, y + h - 1, x + w, y + h - 1);
            draw::set_draw_color(if ctx == TableContext::ColHeader {
                T::TEXT_MUTED
            } else {
                T::TEXT_BODY
            });
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
            let displayed = if ctx == TableContext::Cell && numeric {
                value
                    .as_ref()
                    .and_then(|v| v.parse::<f64>().ok())
                    .map(number)
                    .or(value)
            } else {
                value
            };
            draw::draw_text2(
                displayed.as_deref().unwrap_or(""),
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
        let width =
            (self.table.w() - fltk::app::scrollbar_size() - 2 - if has_id { 52 } else { 0 })
                / (headers.len() as i32 - if has_id { 1 } else { 0 }).max(1);
        for (i, h) in headers.iter().enumerate() {
            self.table
                .set_col_width(i as i32, if *h == "ID" { 52 } else { width.max(100) });
        }
    }
    pub fn set(&mut self, rows: Vec<Vec<String>>) {
        self.table.set_rows(rows.len() as i32);
        self.table.set_row_height_all(32);
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
    pub fn select_first(&mut self) {
        if !self.rows.borrow().is_empty() {
            self.table.set_selection(0, 0, 0, self.table.cols() - 1);
            self.table.redraw();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn currency_keeps_amount_and_decimal_precision() {
        assert_eq!(rupiah(100000.0), "Rp 100.000");
        assert_eq!(rupiah(2700.50), "Rp 2.700,50");
        assert_eq!(rupiah(-1250.75), "Rp -1.250,75");
        assert_eq!(rupiah(-0.0), "Rp 0");
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
