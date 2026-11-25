use crate::gui::theme::AppTheme as T;
use fltk::{
    button::Button,
    draw,
    enums::{Align, Color, Event, Font, FrameType, Key, Shortcut},
    frame::Frame,
    input::Input,
    prelude::*,
    table::{Table, TableContext},
};
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

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

struct ColumnSizing {
    width: i32,
    ratios: Vec<f64>,
    expected: Vec<i32>,
}

impl ColumnSizing {
    fn new(width: i32, columns: Vec<i32>) -> Self {
        Self {
            width,
            ratios: columns
                .iter()
                .map(|size| *size as f64 / width.max(1) as f64)
                .collect(),
            expected: columns,
        }
    }

    fn resize(&mut self, width: i32, current: &[i32]) -> Option<&[i32]> {
        if width <= 0 || width == self.width {
            return None;
        }
        if current != self.expected {
            self.ratios = current
                .iter()
                .map(|size| *size as f64 / self.width.max(1) as f64)
                .collect();
        }
        self.expected = self
            .ratios
            .iter()
            .map(|ratio| (ratio * width as f64).round() as i32)
            .collect();
        self.width = width;
        Some(&self.expected)
    }
}

pub fn fit_page(page: &mut fltk::window::Window, parent: &fltk::window::Window) {
    fn children(
        group: &fltk::group::Group,
        layout: &mut Vec<(fltk::widget::Widget, i32, i32, i32, i32)>,
        grids: &mut Vec<(Table, i32, Vec<i32>)>,
        groups: &mut Vec<fltk::group::Group>,
    ) {
        for i in 0..group.children() {
            if let Some(widget) = group.child(i) {
                layout.push((
                    widget.clone(),
                    widget.x(),
                    widget.y(),
                    widget.w(),
                    widget.h(),
                ));
                if let Some(table) = Table::from_dyn_widget(&widget) {
                    let widths = (0..table.cols()).map(|c| table.col_width(c)).collect();
                    grids.push((table.clone(), table.w(), widths));
                } else if let Some(child) = widget.as_group() {
                    if fltk::group::Scroll::from_dyn_widget(&widget).is_none() {
                        groups.push(child.clone());
                    }
                    children(&child, layout, grids, groups);
                }
            }
        }
    }
    let mut grids = Vec::new();
    let mut layout = Vec::new();
    let mut groups = Vec::new();
    children(
        &page.as_group().unwrap(),
        &mut layout,
        &mut grids,
        &mut groups,
    );
    page.resize(
        super::app_shell::SIDEBAR_WIDTH,
        super::app_shell::HEADER_HEIGHT,
        parent.w() - super::app_shell::SIDEBAR_WIDTH,
        parent.h() - super::app_shell::HEADER_HEIGHT,
    );
    // Scroll resizes its viewport only; apply the same geometry to its content as other pages.
    for (mut widget, x, y, width, height) in layout {
        resize_in_page(&mut widget, page, x, y, width, height);
    }
    for (mut table, width, columns) in grids {
        for (i, column) in columns.iter().enumerate() {
            table.set_col_width(i as i32, column * table.w() / width);
        }
        let current = (0..table.cols())
            .map(|column| table.col_width(column))
            .collect::<Vec<_>>();
        let mut sizing = ColumnSizing::new(table.w(), current);
        table.resize_callback(move |table, _, _, width, _| {
            let current = (0..table.cols())
                .map(|column| table.col_width(column))
                .collect::<Vec<_>>();
            if let Some(sizes) = sizing.resize(width, &current) {
                for (column, size) in sizes.iter().enumerate() {
                    table.set_col_width(column as i32, *size);
                }
            }
        });
    }
    for mut group in groups {
        group.make_resizable(true);
        group.init_sizes();
    }
    page.make_resizable(true);
    page.init_sizes();
}

pub fn resize_in_page<W: WidgetExt>(
    widget: &mut W,
    page: &fltk::window::Window,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    widget.resize(
        x * page.w() / 1200,
        y * page.h() / 612,
        width * page.w() / 1200,
        height * page.h() / 612,
    );
    // Refresh native layout anchors when a page changes its filter/detail arrangement.
    page.clone().init_sizes();
}

fn fullscreen_action(key: Key, active: bool) -> Option<bool> {
    if key == Key::F11 {
        Some(!active)
    } else if key == Key::Escape && active {
        Some(false)
    } else {
        None
    }
}

pub fn enable_fullscreen(window: &mut fltk::window::Window) {
    window.handle(|window, event| {
        if !window.visible_r() || !matches!(event, Event::KeyDown | Event::Shortcut) {
            return false;
        }
        if let Some(modal) = fltk::app::modal() {
            if modal.as_widget_ptr() != window.as_widget_ptr() {
                return false;
            }
        }
        if let Some(active) = fullscreen_action(fltk::app::event_key(), window.fullscreen_active())
        {
            window.fullscreen(active);
            return true;
        }
        false
    });
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
    pub(crate) selection: Rc<RefCell<RowSelection>>,
}

#[derive(Default)]
pub(crate) struct RowSelection {
    anchor: Option<i32>,
    cursor: Option<i32>,
    rows: BTreeSet<i32>,
    dragging: bool,
}

impl RowSelection {
    fn bounds(&self) -> Option<(i32, i32)> {
        self.rows
            .first()
            .zip(self.rows.last())
            .map(|(first, last)| (*first, *last))
    }

    fn sync(&mut self, first: i32, last: i32) {
        let bounds = (first >= 0 && last >= first).then_some((first, last));
        if self.bounds() != bounds {
            self.rows = bounds
                .map(|(first, last)| (first..=last).collect())
                .unwrap_or_default();
            self.anchor = bounds.map(|(first, _)| first);
            self.cursor = bounds.map(|(_, last)| last);
        }
    }

    pub(crate) fn click(&mut self, row: i32, shift: bool, control: bool) {
        self.cursor = Some(row);
        if shift {
            let first = self.anchor.unwrap_or(row);
            self.anchor = Some(first);
            if !control {
                self.rows.clear();
            }
            self.rows.extend(first.min(row)..=first.max(row));
        } else {
            self.anchor = Some(row);
            if control {
                if !self.rows.remove(&row) {
                    self.rows.insert(row);
                }
            } else {
                self.rows.clear();
                self.rows.insert(row);
            }
        }
    }

    fn drag(&mut self, row: i32, control: bool) {
        if self.cursor != Some(row) {
            self.click(row, true, control);
        }
    }

    fn indices(&self, first: i32, last: i32, count: usize) -> Vec<usize> {
        if count == 0 || first < 0 || last < first {
            return Vec::new();
        }
        if self.bounds() == Some((first, last)) {
            self.rows
                .iter()
                .filter_map(|row| ((*row as usize) < count).then_some(*row as usize))
                .collect()
        } else {
            (first as usize..=(last as usize).min(count - 1)).collect()
        }
    }
}

pub(crate) fn select_rows(
    table: &mut Table,
    event: Event,
    selection: &RefCell<RowSelection>,
) -> bool {
    let mouse_down = selection.borrow().dragging;
    if event == Event::Released && mouse_down {
        selection.borrow_mut().dragging = false;
        if table
            .trigger()
            .contains(fltk::enums::CallbackTrigger::Release)
        {
            table.do_callback();
        }
        return true;
    }
    let dragging = event == Event::Drag && mouse_down;
    if !dragging && (event != Event::Push || fltk::app::event_button() != 1) {
        return false;
    }
    let Some((context, row, _, _)) = table.cursor2rowcol() else {
        return dragging;
    };
    if context != TableContext::Cell || row < 0 {
        return dragging;
    }
    if !dragging {
        let _ = table.take_focus();
    }
    let modifiers = fltk::app::event_state();
    let bounds = {
        let mut selected = selection.borrow_mut();
        if !dragging {
            let (first, _, last, _) = table.get_selection();
            selected.sync(first, last);
        }
        if dragging {
            selected.drag(row, modifiers.contains(Shortcut::Ctrl));
        } else {
            selected.click(
                row,
                modifiers.contains(Shortcut::Shift),
                modifiers.contains(Shortcut::Ctrl),
            );
        }
        selected.dragging = true;
        selected.bounds()
    };
    if let Some((first, last)) = bounds {
        table.set_selection(first, 0, last, table.cols() - 1);
    } else {
        table.set_selection(-1, -1, -1, -1);
    }
    table.redraw();
    true
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
        let selection = Rc::new(RefCell::new(RowSelection::default()));
        let select = selection.clone();
        table.handle(move |table, event| select_rows(table, event, &select));
        let rows = Rc::new(RefCell::new(Vec::<Vec<String>>::new()));
        let header = Rc::new(RefCell::new(
            headers.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        ));
        let r = rows.clone();
        let hd = header.clone();
        let selected_rows = selection.clone();
        table.draw_cell(move |t, ctx, row, col, x, y, w, h| {
            if !matches!(ctx, TableContext::ColHeader | TableContext::Cell) {
                return;
            }
            draw::push_clip(x, y, w, h);
            let (first, _, last, _) = t.get_selection();
            let selected = ctx == TableContext::Cell && {
                let selection = selected_rows.borrow();
                if selection.bounds() == Some((first, last)) {
                    selection.rows.contains(&row)
                } else {
                    t.is_selected(row, col)
                }
            };
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
            selection,
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
        let changed = self
            .rows
            .borrow()
            .iter()
            .map(|row| row.first())
            .ne(rows.iter().map(|row| row.first()));
        let appended = {
            let current = self.rows.borrow();
            rows.len() >= current.len()
                && current
                    .iter()
                    .zip(&rows)
                    .all(|(old, new)| old.first() == new.first())
        };
        if changed && !appended {
            self.clear_selection();
        }
        self.table.set_rows(rows.len() as i32);
        self.table.set_row_height_all(32);
        *self.rows.borrow_mut() = rows;
        self.table.redraw();
    }
    pub fn selected(&self) -> Option<Vec<String>> {
        self.selected_rows()
            .first()
            .and_then(|row| self.rows.borrow().get(*row).cloned())
    }
    pub fn selected_rows(&self) -> Vec<usize> {
        let (first, _, last, _) = self.table.get_selection();
        self.selection
            .borrow()
            .indices(first, last, self.rows.borrow().len())
    }
    pub fn clear_selection(&mut self) {
        *self.selection.borrow_mut() = RowSelection::default();
        self.table.set_selection(-1, -1, -1, -1);
        self.table.redraw();
    }
    pub fn select_first(&mut self) {
        if !self.rows.borrow().is_empty() {
            self.selection.borrow_mut().click(0, false, false);
            self.table.set_selection(0, 0, 0, self.table.cols() - 1);
            self.table.redraw();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn columns_restore_without_drift_and_preserve_user_changes() {
        let original = vec![0, 164, 280, 124, 64];
        let mut sizing = ColumnSizing::new(652, original.clone());
        let mut current = original.clone();
        for _ in 0..20 {
            for width in [970, 500, 1100, 652] {
                current = sizing.resize(width, &current).unwrap().to_vec();
                assert_eq!(current[0], 0);
            }
            assert_eq!(current, original);
        }
        current[1] = 200;
        let doubled = sizing.resize(1304, &current).unwrap().to_vec();
        assert_eq!(doubled[1], 400);
        assert_eq!(sizing.resize(652, &doubled).unwrap(), current);
        let changed_headers = [80, 200, 400];
        assert_eq!(
            sizing.resize(326, &changed_headers).unwrap(),
            [40, 100, 200]
        );
    }
    #[test]
    fn fullscreen_keys_toggle_restore_and_leave_pos_shortcuts_available() {
        assert_eq!(fullscreen_action(Key::F11, false), Some(true));
        assert_eq!(fullscreen_action(Key::F11, true), Some(false));
        assert_eq!(fullscreen_action(Key::Escape, true), Some(false));
        assert_eq!(fullscreen_action(Key::Escape, false), None);
        for key in [Key::F1, Key::F2, Key::F3, Key::F4, Key::Enter] {
            assert_eq!(fullscreen_action(key, false), None);
            assert_eq!(fullscreen_action(key, true), None);
        }
    }
    #[test]
    fn row_selection_tracks_shift_anchor_ctrl_toggles_and_resets() {
        let mut selection = RowSelection::default();
        selection.click(2, false, false);
        selection.click(3, true, false);
        assert_eq!(selection.indices(2, 3, 5), [2, 3]);
        selection.click(4, true, false);
        assert_eq!(selection.indices(2, 4, 5), [2, 3, 4]);
        selection.click(3, true, false);
        assert_eq!(selection.indices(2, 3, 5), [2, 3]);
        selection.click(2, false, false);
        selection.click(2, true, false);
        assert_eq!(selection.indices(2, 2, 5), [2]);
        selection.click(0, false, false);
        selection.click(2, false, true);
        assert_eq!(selection.indices(0, 2, 5), [0, 2]);
        selection.click(1, false, true);
        selection.click(1, false, true);
        assert_eq!(selection.indices(0, 2, 5), [0, 2]);
        selection.click(3, true, false);
        assert_eq!(selection.indices(1, 3, 5), [1, 2, 3]);
        selection.sync(0, 0);
        selection.click(1, true, false);
        assert_eq!(selection.indices(0, 1, 5), [0, 1]);
        selection.sync(-1, -1);
        selection.click(3, true, false);
        assert_eq!(selection.indices(3, 3, 5), [3]);
        selection.click(3, false, true);
        selection.drag(3, true);
        assert!(selection.indices(-1, -1, 5).is_empty());
        assert_eq!(RowSelection::default().indices(1, 3, 3), [1, 2]);
    }

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
