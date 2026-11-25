use super::components::{button, close_modal, enable_fullscreen, text};
use crate::{
    gui::theme::AppTheme as T,
    hardware::{receipt_rows, wrap_receipt_text as wrap_lines, ReceiptSettings, ReceiptStyle},
    models::Sale,
    services::barcode::{self, Barcode, LabelItem},
};
use fltk::{
    draw,
    enums::{Color, Font, FrameType},
    frame::Frame,
    group::{Scroll, ScrollType},
    menu::Choice,
    prelude::*,
    printer::Printer,
    window::Window,
};
use std::{cell::Cell, ops::Range, rc::Rc};

// FLTK printer coordinates are points (1/72 inch); preview zoom never changes print size.
const MM: f64 = 72.0 / 25.4;
const SCREEN_SCALE: f64 = 96.0 / 72.0;
const ZOOMS: [i32; 5] = [75, 100, 125, 150, 200];

#[derive(Clone)]
struct Label {
    name: String,
    code: String,
    price: String,
    barcode: Barcode,
}

#[derive(Clone)]
enum Content {
    Text {
        value: String,
        right: Option<String>,
        size: i32,
        bold: bool,
        center: bool,
    },
    Rule,
    Labels {
        width: f64,
        height: f64,
        labels: Vec<Label>,
    },
}

#[derive(Clone)]
struct Band {
    height: f64,
    content: Content,
}

#[derive(Clone)]
struct Document {
    width: f64,
    gap: f64,
    bands: Vec<Band>,
}

impl Document {
    fn height(&self) -> f64 {
        self.bands.iter().map(|band| band.height).sum::<f64>()
            + self.gap * self.bands.len().saturating_sub(1) as f64
    }

    fn text(&mut self, value: &str, size: i32, bold: bool, center: bool) {
        let columns = (self.width / (size as f64 * 0.6)).floor().max(1.0) as usize;
        for line in wrap_lines(value, columns) {
            self.bands.push(Band {
                height: size as f64 + 2.0,
                content: Content::Text {
                    value: line,
                    right: None,
                    size,
                    bold,
                    center,
                },
            });
        }
    }

    fn pair(&mut self, left: &str, right: &str, size: i32, bold: bool) {
        let columns = (self.width / (size as f64 * 0.6)).floor().max(1.0) as usize;
        let mut lines = wrap_lines(left, columns);
        let last = lines.pop().unwrap_or_default();
        for line in lines {
            self.text(&line, size, bold, false);
        }
        if last.chars().count() + right.chars().count() + 1 > columns {
            self.text(&last, size, bold, false);
            for value in wrap_lines(right, columns) {
                self.bands.push(Band {
                    height: size as f64 + 2.0,
                    content: Content::Text {
                        value: String::new(),
                        right: Some(value),
                        size,
                        bold,
                        center: false,
                    },
                });
            }
        } else {
            self.bands.push(Band {
                height: size as f64 + 2.0,
                content: Content::Text {
                    value: last,
                    right: Some(right.into()),
                    size,
                    bold,
                    center: false,
                },
            });
        }
    }

    fn rule(&mut self) {
        self.bands.push(Band {
            height: 6.0,
            content: Content::Rule,
        });
    }

    fn pages(&self, width: f64, height: f64) -> Result<Vec<Range<usize>>, String> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err("Area cetak printer tidak tersedia".into());
        }
        if self.width > width + 0.1 {
            return Err(format!("Dokumen membutuhkan area cetak {:.1} mm; printer menyediakan {:.1} mm. Pilih ukuran kertas atau jumlah kolom yang sesuai.", self.width / MM, width / MM));
        }
        let mut pages = Vec::new();
        let mut start = 0;
        let mut used = 0.0;
        for (index, band) in self.bands.iter().enumerate() {
            if band.height > height {
                return Err("Tinggi label melebihi area cetak. Pilih kertas lebih tinggi.".into());
            }
            let gap = if index > start { self.gap } else { 0.0 };
            if used + gap + band.height > height {
                pages.push(start..index);
                start = index;
                used = 0.0;
            } else {
                used += gap;
            }
            used += band.height;
        }
        if start < self.bands.len() {
            pages.push(start..self.bands.len());
        }
        Ok(pages)
    }

    fn draw(&self, x: f64, mut y: f64, scale: f64, range: Range<usize>) {
        let width = self.width * scale;
        for (index, band) in self.bands[range].iter().enumerate() {
            if index > 0 {
                y += self.gap * scale;
            }
            let height = band.height * scale;
            if !draw::not_clipped(
                x.floor() as i32,
                y.floor() as i32,
                width.ceil() as i32,
                height.ceil() as i32,
            ) {
                y += height;
                continue;
            }
            match &band.content {
                Content::Text {
                    value,
                    right,
                    size,
                    bold,
                    center,
                } => {
                    draw::set_draw_color(Color::White);
                    draw::draw_rectf(
                        x.round() as i32,
                        y.round() as i32,
                        width.ceil() as i32,
                        height.ceil() as i32,
                    );
                    draw::set_draw_color(Color::Black);
                    draw::set_font(
                        if *bold {
                            Font::CourierBold
                        } else {
                            Font::Courier
                        },
                        (*size as f64 * scale).round() as i32,
                    );
                    let tx = if *center {
                        x + (width - draw::width(value)) / 2.0
                    } else {
                        x
                    };
                    draw::draw_text_n(
                        value,
                        tx.round() as i32,
                        (y + *size as f64 * scale).round() as i32,
                    );
                    if let Some(right) = right {
                        draw::draw_text_n(
                            right,
                            (x + width - draw::width(right)).round() as i32,
                            (y + *size as f64 * scale).round() as i32,
                        );
                    }
                }
                Content::Rule => {
                    draw::set_draw_color(Color::White);
                    draw::draw_rectf(
                        x.round() as i32,
                        y.round() as i32,
                        width.ceil() as i32,
                        height.ceil() as i32,
                    );
                    draw::set_draw_color(Color::Black);
                    draw::set_line_style(draw::LineStyle::Dash, 0);
                    draw::draw_line(
                        x.round() as i32,
                        (y + height / 2.0).round() as i32,
                        (x + width).round() as i32,
                        (y + height / 2.0).round() as i32,
                    );
                    draw::set_line_style(draw::LineStyle::Solid, 0);
                }
                Content::Labels {
                    width,
                    height,
                    labels,
                } => {
                    for (column, label) in labels.iter().enumerate() {
                        draw_label(
                            label,
                            x + column as f64 * (width + 2.0 * MM) * scale,
                            y,
                            *width * scale,
                            *height * scale,
                            scale,
                        );
                    }
                }
            }
            y += height;
        }
    }
}

fn draw_label(label: &Label, x: f64, y: f64, width: f64, height: f64, scale: f64) {
    draw::push_clip(
        x.floor() as i32,
        y.floor() as i32,
        width.ceil() as i32,
        height.ceil() as i32,
    );
    draw::set_draw_color(Color::White);
    draw::draw_rectf(
        x.floor() as i32,
        y.floor() as i32,
        width.ceil() as i32,
        height.ceil() as i32,
    );
    draw::set_draw_color(Color::Black);
    draw::set_font(Font::Helvetica, (6.0 * scale).round() as i32);
    let pad = MM * scale;
    let columns = ((width - 2.0 * pad) / (3.6 * scale)).floor().max(1.0) as usize;
    let center = |value: &str, baseline: f64| {
        draw::draw_text_n(
            value,
            (x + (width - draw::width(value)) / 2.0).round() as i32,
            baseline.round() as i32,
        );
    };
    draw::push_clip(
        (x + pad).round() as i32,
        (y + pad).round() as i32,
        (width - 2.0 * pad).round() as i32,
        (4.0 * pad).round() as i32,
    );
    for (line, value) in wrap_lines(&label.name, columns).iter().take(2).enumerate() {
        draw::draw_text_n(
            value,
            (x + pad).round() as i32,
            (y + pad + 5.5 * scale + line as f64 * 5.5 * scale).round() as i32,
        );
    }
    draw::pop_clip();
    draw::draw_text_n(
        &label.price,
        (x + pad).round() as i32,
        (y + 5.0 * pad + 6.0 * scale).round() as i32,
    );
    let bar_x = x + 2.0 * pad;
    let bar_y = y + 8.0 * pad;
    let bar_width = width - 4.0 * pad;
    let bar_height = height - 12.0 * pad;
    // Vector paths retain fractional-point bar widths on the printer; no bitmap scaling.
    for (start, bar) in &label.barcode.bars {
        let left = bar_x + *start as f64 * bar_width / label.barcode.width as f64;
        let right = bar_x + (*start + *bar) as f64 * bar_width / label.barcode.width as f64;
        draw::begin_polygon();
        draw::vertex(left, bar_y);
        draw::vertex(right, bar_y);
        draw::vertex(right, bar_y + bar_height);
        draw::vertex(left, bar_y + bar_height);
        draw::end_polygon();
    }
    draw::set_font(Font::Courier, (6.0 * scale).round() as i32);
    center(&label.code, bar_y + bar_height + 6.0 * scale);
    draw::pop_clip();
}

fn label_document(
    items: &[LabelItem<'_>],
    format: Option<(u32, u32, u32)>,
    dpi: u32,
) -> Result<Document, String> {
    let layout = barcode::prepare_layout(items, format, dpi)?;
    let mut labels = Vec::new();
    for (item, barcode) in items.iter().zip(layout.barcodes) {
        let label = Label {
            name: item.name.into(),
            code: item.code.into(),
            price: barcode::price_text(item.price),
            barcode,
        };
        labels.extend(std::iter::repeat(label).take(item.quantity as usize));
    }
    let width = layout.width * MM;
    let height = layout.height * MM;
    Ok(Document {
        width: width * layout.columns as f64 + 2.0 * MM * (layout.columns - 1) as f64,
        gap: 2.0 * MM,
        bands: labels
            .chunks(layout.columns as usize)
            .map(|labels| Band {
                height,
                content: Content::Labels {
                    width,
                    height,
                    labels: labels.to_vec(),
                },
            })
            .collect(),
    })
}

fn receipt_document(sale: &Sale, settings: &ReceiptSettings, draft: bool) -> Document {
    let mut doc = Document {
        width: (if settings.paper_width == 80 {
            74.0
        } else {
            52.0
        }) * MM,
        gap: 0.0,
        bands: Vec::new(),
    };
    if draft {
        doc.text("DRAF · belum disimpan", 8, true, true);
    }
    for row in receipt_rows(sale, settings) {
        if row.style == ReceiptStyle::Rule {
            doc.rule();
            continue;
        }
        let size = match row.style {
            ReceiptStyle::Header => 12,
            ReceiptStyle::Total => 10,
            _ => 8,
        };
        let bold = matches!(
            row.style,
            ReceiptStyle::Header | ReceiptStyle::Total | ReceiptStyle::Bold
        );
        if let Some(right) = row.right {
            doc.pair(&row.text, &right, size, bold);
        } else {
            doc.text(
                &row.text,
                size,
                bold,
                matches!(row.style, ReceiptStyle::Header | ReceiptStyle::Center),
            );
        }
    }
    doc
}

fn print_document(doc: &Document) -> Result<(), String> {
    let mut printer = Printer::default();
    let (from, to) = printer
        .begin_job(0)
        .map_err(|_| "Cetak dibatalkan atau dialog printer tidak dapat dibuka".to_string())?;
    let result = (|| {
        let (width, height) = printer.printable_rect();
        let pages = doc.pages(width as f64, height as f64)?;
        let first = from.unwrap_or(1).max(1) as usize;
        let last = to
            .map(|page| page.max(1) as usize)
            .unwrap_or(pages.len())
            .min(pages.len());
        if first > last {
            return Err("Rentang halaman tidak ada dalam dokumen".into());
        }
        for range in pages.into_iter().skip(first - 1).take(last - first + 1) {
            printer
                .begin_page()
                .map_err(|e| format!("Gagal memulai halaman cetak: {e}"))?;
            printer.scale(1.0, 1.0);
            doc.draw(0.0, 0.0, 1.0, range);
            printer
                .end_page()
                .map_err(|e| format!("Gagal mencetak halaman: {e}"))?;
        }
        Ok(())
    })();
    printer.end_job();
    result
}

fn show_document(
    doc: Document,
    title: &str,
    subtitle: &str,
    enabled: bool,
    margin_mm: f64,
    mut print_action: impl FnMut() -> Result<(), String> + 'static,
) {
    let (_, _, screen_w, screen_h) = fltk::app::screen_work_area(0);
    let scale = Rc::new(Cell::new(SCREEN_SCALE * 1.5));
    let margin = margin_mm * MM;
    let paper_width = doc.width + 2.0 * margin;
    let paper_height = doc.height() + 2.0 * margin;
    let width = (paper_width * scale.get() + 96.0).ceil() as i32;
    let width = width.clamp(660, 1040).min(screen_w - 32);
    let height = 700.min(screen_h - 40);
    let mut win = Window::default().with_size(width, height).with_label(title);
    win.set_color(T::BG_APP);
    let heading = text(24, 18, width - 284, 30, title, 22, true);
    let zoom_label = text(width - 220, 24, 64, 20, "Zoom", 12, false);
    let mut zoom = Choice::new(width - 156, 18, 132, 34, None);
    super::components::style_choice(&mut zoom);
    zoom.add_choice("75%|100%|125%|150%|200%");
    zoom.set_value(3);
    let description = text(24, 56, width - 48, 20, subtitle, 12, false);
    let mut print = button(width - 204, height - 48, 180, "Cetak", true);
    if !enabled {
        print.deactivate();
    }
    let mut status = text(
        24,
        height - 78,
        width - 48,
        22,
        "Pilih printer dan ukuran kertas pada dialog cetak.",
        12,
        false,
    );
    let mut close = button(width - 356, height - 48, 136, "Tutup", false);
    let mut scroll = Scroll::new(24, 90, width - 48, height - 182, None);
    scroll.set_type(ScrollType::Both);
    scroll.set_color(Color::from_rgb(224, 228, 233));
    scroll.set_frame(FrameType::FlatBox);
    let canvas_width = (paper_width * scale.get()).ceil() as i32;
    let canvas_x = 32 + ((width - 80 - canvas_width).max(0) / 2);
    let mut canvas = Frame::new(
        canvas_x,
        98,
        canvas_width,
        (paper_height * scale.get()).ceil() as i32,
        None,
    );
    let draw_scale = scale.clone();
    canvas.draw(move |frame| {
        draw::push_clip(frame.x(), frame.y(), frame.w(), frame.h());
        draw::set_draw_color(Color::White);
        draw::draw_rectf(frame.x(), frame.y(), frame.w(), frame.h());
        let scale = draw_scale.get();
        doc.draw(
            frame.x() as f64 + margin * scale,
            frame.y() as f64 + margin * scale,
            scale,
            0..doc.bands.len(),
        );
        draw::set_draw_color(Color::from_rgb(160, 168, 176));
        draw::draw_rect(frame.x(), frame.y(), frame.w(), frame.h());
        draw::pop_clip();
    });
    scroll.end();
    win.resizable(&scroll);
    win.size_range(660.min(width), 480.min(height), 0, 0);
    let mut resize_scroll = scroll.clone();
    let mut resize_canvas = canvas.clone();
    let resize_scale = scale.clone();
    let mut resize_heading = heading;
    let mut resize_zoom_label = zoom_label;
    let mut resize_zoom = zoom.clone();
    let mut resize_description = description;
    let mut resize_print = print.clone();
    let mut resize_close = close.clone();
    let mut resize_status = status.clone();
    win.resize_callback(move |window, _, _, width, height| {
        resize_heading.resize(24, 18, width - 284, 30);
        resize_zoom_label.resize(width - 220, 24, 64, 20);
        resize_zoom.resize(width - 156, 18, 132, 34);
        resize_description.resize(24, 56, width - 48, 20);
        resize_scroll.resize(24, 90, width - 48, height - 182);
        let canvas_width = (paper_width * resize_scale.get()).ceil() as i32;
        resize_canvas.resize(
            32 + (width - 80 - canvas_width).max(0) / 2 - resize_scroll.xposition(),
            98 - resize_scroll.yposition(),
            canvas_width,
            (paper_height * resize_scale.get()).ceil() as i32,
        );
        resize_print.resize(width - 204, height - 48, 180, 34);
        resize_close.resize(width - 356, height - 48, 136, 34);
        resize_status.resize(24, height - 78, width - 48, 22);
        window.redraw();
    });
    enable_fullscreen(&mut win);
    zoom.set_callback(move |choice| {
        let value = SCREEN_SCALE * ZOOMS[choice.value().max(0) as usize] as f64 / 100.0;
        scale.set(value);
        scroll.scroll_to(0, 0);
        let canvas_width = (paper_width * value).ceil() as i32;
        canvas.resize(
            scroll.x() + 8 + (scroll.w() - 32 - canvas_width).max(0) / 2,
            98,
            canvas_width,
            (paper_height * value).ceil() as i32,
        );
        scroll.redraw();
    });
    print.set_callback(move |_| match print_action() {
        Ok(()) => status.set_label("Dokumen dikirim ke printer"),
        Err(error) => {
            status.set_label(&error);
            status.set_tooltip(&error);
        }
    });
    let mut w = win.clone();
    close.set_callback(move |_| close_modal(&mut w));
    win.end();
    win.make_modal(true);
    win.set_callback(close_modal);
    win.show();
}

pub fn show_barcodes(
    items: &[LabelItem<'_>],
    format: Option<(u32, u32, u32)>,
    dpi: u32,
) -> Result<(), String> {
    let doc = label_document(items, format, dpi)?;
    let count = items.iter().map(|item| item.quantity).sum::<i32>();
    let subtitle = format!(
        "{count} label · area cetak {:.1} mm · ukuran cetak 100%",
        doc.width / MM
    );
    let print = doc.clone();
    show_document(
        doc,
        "Pratinjau lembar barcode",
        &subtitle,
        true,
        4.0,
        move || print_document(&print),
    );
    Ok(())
}

pub fn show_receipt(sale: Sale, settings: ReceiptSettings, draft: bool) {
    let doc = receipt_document(&sale, &settings, draft);
    let subtitle = if draft {
        "Transaksi belum disimpan. Simpan transaksi untuk mencetak struk."
    } else {
        "Struk tersimpan · cetak ulang memakai faktur yang sama."
    };
    show_document(doc, "Pratinjau struk", subtitle, !draft, 3.0, move || {
        crate::services::documents::print_receipt_with_settings(&sale, &settings)
    });
}

pub fn print_receipt_native(sale: &Sale, settings: &ReceiptSettings) -> Result<(), String> {
    print_document(&receipt_document(sale, settings, false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_labels_paginate_without_scaling_losing_or_splitting_labels() {
        let mut items = [
            LabelItem {
                code: "0012",
                name: "Obat A",
                price: 12500.50,
                quantity: 2,
            },
            LabelItem {
                code: "CTH-002",
                name: "Obat B",
                price: 900.0,
                quantity: 3,
            },
            LabelItem {
                code: "8999909001001",
                name: "Obat C",
                price: 0.0,
                quantity: 2,
            },
        ];
        let doc = label_document(&items, Some((50, 30, 3)), 203).unwrap();
        assert!((doc.width / MM - 154.0).abs() < 0.001);
        assert_eq!(doc.pages(doc.width, 62.001 * MM).unwrap(), [0..2, 2..3]);
        assert_eq!(doc.pages(doc.width, 30.0 * MM).unwrap(), [0..1, 1..2, 2..3]);
        assert!(doc.pages(80.0 * MM, 300.0 * MM).is_err());
        assert!(doc.pages(doc.width, 29.0 * MM).is_err());
        assert!(doc.pages(0.0, 100.0).is_err());
        let labels = doc
            .bands
            .iter()
            .flat_map(|band| match &band.content {
                Content::Labels { labels, .. } => labels.iter(),
                _ => unreachable!(),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            labels
                .iter()
                .map(|label| label.code.as_str())
                .collect::<Vec<_>>(),
            [
                "0012",
                "0012",
                "CTH-002",
                "CTH-002",
                "CTH-002",
                "8999909001001",
                "8999909001001"
            ]
        );
        assert_eq!(labels[0].price, "Rp 12.500,50");
        let encoded = Barcode::new("0012").unwrap();
        assert_eq!(labels[0].barcode.bars, encoded.bars);
        if let Ok(path) = std::env::var("APOTEK_RECEIPT_SAMPLE") {
            let bands = doc.bands.iter().map(|band| match &band.content {
                Content::Labels { width, height, labels } => serde_json::json!({"height":band.height,"label_width":width,"label_height":height,"labels":labels.iter().map(|label| serde_json::json!({"name":label.name,"code":label.code,"price":label.price,"bars":label.barcode.bars,"barcode_width":label.barcode.width})).collect::<Vec<_>>()}),
                _ => unreachable!(),
            }).collect::<Vec<_>>();
            std::fs::write(
                format!("{path}.labels"),
                serde_json::json!({"width":doc.width,"gap":doc.gap,"bands":bands}).to_string(),
            )
            .unwrap();
        }
        items[0].quantity = 334;
        items[1].quantity = 333;
        items[2].quantity = 333;
        let doc = label_document(&items, Some((50, 30, 3)), 203).unwrap();
        let pages = doc.pages(doc.width, 126.001 * MM).unwrap();
        assert_eq!(pages.len(), 84);
        let count = pages
            .iter()
            .flat_map(|range| doc.bands[range.clone()].iter())
            .map(|band| match &band.content {
                Content::Labels { labels, .. } => labels.len(),
                _ => 0,
            })
            .sum::<usize>();
        assert_eq!(count, 1000);
        assert_eq!(pages.last().unwrap().end, doc.bands.len());
    }

    #[test]
    fn native_receipts_keep_identity_snapshots_cents_credit_and_long_text() {
        assert_eq!(wrap_lines("Obat ààààà", 4), ["Obat", "àààà", "à"]);
        assert_eq!(
            wrap_lines("Terima kasih\n\nSemoga sehat <3", 30),
            ["Terima kasih", "", "Semoga sehat <3"]
        );
        let mut sale = crate::db::demo::sales()[0].clone();
        sale.items.truncate(1);
        sale.items[0].product_name = "Obat @ Lama".into();
        sale.items[0].discount = 100.25;
        sale.items[0].sell_price = 9500.55;
        sale.items[0].subtotal = 9500.55;
        sale.subtotal = 9500.55;
        sale.grand_total = 9500.55;
        sale.paid_amount = 10000.10;
        sale.change_amount = 499.55;
        let mut values = crate::db::demo::settings();
        values.insert(
            "receipt_footer".into(),
            "Terima kasih\n\nSemoga sehat <3".into(),
        );
        let mut settings = ReceiptSettings::from_values(&values);
        assert_eq!(settings.footer_note, "Terima kasih\n\nSemoga sehat <3");
        settings.store_name = "Apotek @ Utama".into();
        settings.customer_name =
            "Pelanggan panjang àààààààààààààààààààààààààààààààààààààààà".into();
        let doc = receipt_document(&sale, &settings, true);
        let values = |doc: &Document| {
            doc.bands
                .iter()
                .filter_map(|band| match &band.content {
                    Content::Text { value, right, .. } => {
                        Some(format!("{value} {}", right.as_deref().unwrap_or("")))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        let rendered = values(&doc);
        for text in [
            "DRAF",
            "Apotek @ Utama",
            "Obat @ Lama",
            "Rp 9.500,55",
            "Rp 499,55",
            "Diskon barang Rp 100,25",
        ] {
            assert!(rendered.contains(text), "Missing {text}");
        }
        assert!(doc.bands.iter().all(|band| match &band.content {
            Content::Text { value, size, .. } =>
                value.chars().count() <= (doc.width / (*size as f64 * 0.6)).floor() as usize,
            _ => true,
        }));
        sale.payment_type = "KREDIT".into();
        sale.paid_amount = 2000.10;
        sale.due_date = Some("2026-11-25".into());
        let doc = receipt_document(&sale, &settings, false);
        assert!(values(&doc).contains("Sisa: Rp 7.500,45"));
        assert!(values(&doc).contains("Jatuh tempo: 2026-11-25"));
        assert!(!values(&doc).contains("DRAF"));
        let pages = doc.pages(doc.width, 80.0).unwrap();
        assert!(pages.len() > 1);
        assert_eq!(
            pages.iter().map(|range| range.len()).sum::<usize>(),
            doc.bands.len()
        );
        settings.returned = true;
        let rendered = values(&receipt_document(&sale, &settings, false));
        assert!(rendered.contains("FAKTUR DIRETUR (salinan)"));
        assert!(rendered.contains("Sisa: Rp 0"));
        let mut narrow = Document {
            width: 52.0 * MM,
            gap: 0.0,
            bands: Vec::new(),
        };
        narrow.pair(&"SATUAN".repeat(5), "Rp 9.500,55", 8, false);
        assert!(
            matches!(&narrow.bands.last().unwrap().content, Content::Text { value, right: Some(right), .. } if value.is_empty() && right == "Rp 9.500,55")
        );

        // Optional headless layout artifact uses the same bands as the native preview.
        if let Ok(path) = std::env::var("APOTEK_RECEIPT_SAMPLE") {
            let sale = crate::db::demo::sales()[0].clone();
            let settings =
                crate::services::documents::receipt_settings(None, &sale, Some(58)).unwrap();
            let sample = receipt_document(&sale, &settings, false);
            let bands = sample.bands.iter().map(|band| match &band.content {
                Content::Text { value, right, size, bold, center } => serde_json::json!({"height":band.height,"text":value,"right":right,"size":size,"bold":bold,"center":center}),
                Content::Rule => serde_json::json!({"height":band.height,"rule":true}),
                _ => unreachable!(),
            }).collect::<Vec<_>>();
            std::fs::write(
                path,
                serde_json::json!({"width":sample.width,"bands":bands}).to_string(),
            )
            .unwrap();
        }
    }
}
