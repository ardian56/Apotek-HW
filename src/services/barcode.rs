use std::fmt::Write;

pub const MAX_PRODUCTS: usize = 200;
pub const MAX_LABELS: i32 = 1000;

// Code 128 bar/space widths, including the seven-part stop symbol.
const PATTERNS: [&str; 107] = [
    "212222", "222122", "222221", "121223", "121322", "131222", "122213", "122312", "132212",
    "221213", "221312", "231212", "112232", "122132", "122231", "113222", "123122", "123221",
    "223211", "221132", "221231", "213212", "223112", "312131", "311222", "321122", "321221",
    "312212", "322112", "322211", "212123", "212321", "232121", "111323", "131123", "131321",
    "112313", "132113", "132311", "211313", "231113", "231311", "112133", "112331", "132131",
    "113123", "113321", "133121", "313121", "211331", "231131", "213113", "213311", "213131",
    "311123", "311321", "331121", "312113", "312311", "332111", "314111", "221411", "431111",
    "111224", "111422", "121124", "121421", "141122", "141221", "112214", "112412", "122114",
    "122411", "142112", "142211", "241211", "221114", "413111", "241112", "134111", "111242",
    "121142", "121241", "114212", "124112", "124211", "411212", "421112", "421211", "212141",
    "214121", "412121", "111143", "111341", "131141", "114113", "114311", "411113", "411311",
    "113141", "114131", "311141", "411131", "211412", "211214", "211232", "2331112",
];

#[derive(Clone)]
pub struct Barcode {
    pub bars: Vec<(i32, i32)>,
    pub width: i32,
}

impl Barcode {
    pub fn new(code: &str) -> Result<Self, String> {
        if code.is_empty() || code.len() > 50 || !code.bytes().all(|b| (32..=126).contains(&b)) {
            return Err(
                "Barcode harus berisi 1–50 karakter huruf, angka, atau tanda baca biasa".into(),
            );
        }
        // Pack digit runs in set C; scanners still receive the exact original text.
        let bytes = code.as_bytes();
        let mut start = 104;
        let mut data = Vec::<usize>::new();
        let mut pos = 0;
        while pos < bytes.len() {
            let run = bytes[pos..]
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .count();
            if run >= 4 {
                let mut count = run;
                if count % 2 != 0 {
                    data.push((bytes[pos] - 32) as usize);
                    pos += 1;
                    count -= 1;
                }
                if pos == 0 && data.is_empty() {
                    start = 105;
                } else {
                    data.push(99);
                }
                for _ in 0..count / 2 {
                    data.push(((bytes[pos] - b'0') * 10 + bytes[pos + 1] - b'0') as usize);
                    pos += 2;
                }
                if pos < bytes.len() {
                    data.push(100);
                }
            } else {
                data.push((bytes[pos] - 32) as usize);
                pos += 1;
            }
        }
        let checksum = (start
            + data
                .iter()
                .enumerate()
                .map(|(i, v)| (i + 1) * v)
                .sum::<usize>())
            % 103;
        let mut bars = Vec::new();
        let mut x = 10; // Quiet zone: at least ten modules on either side.
        for symbol in std::iter::once(start).chain(data).chain([checksum, 106]) {
            for (i, width) in PATTERNS[symbol].bytes().enumerate() {
                let width = (width - b'0') as i32;
                if i % 2 == 0 {
                    bars.push((x, width));
                }
                x += width;
            }
        }
        Ok(Self {
            bars,
            width: x + 10,
        })
    }

    #[cfg(test)]
    pub fn svg(&self) -> String {
        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {} 64" width="{}mm" height="12mm" preserveAspectRatio="none" shape-rendering="crispEdges"><rect width="100%" height="100%" fill="white"/><g fill="black">"#,
            self.width,
            self.width as f64 * 0.254
        );
        for (x, w) in &self.bars {
            write!(svg, r#"<rect x="{x}" y="0" width="{w}" height="64"/>"#).unwrap();
        }
        svg.push_str("</g></svg>");
        svg
    }
}

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub struct LabelItem<'a> {
    pub code: &'a str,
    pub name: &'a str,
    pub price: f64,
    pub quantity: i32,
}

pub struct LabelLayout {
    pub width: f64,
    pub height: f64,
    pub columns: u32,
    pub barcodes: Vec<Barcode>,
}

pub fn prepare_layout(
    items: &[LabelItem<'_>],
    size: Option<(u32, u32, u32)>,
    dpi: u32,
) -> Result<LabelLayout, String> {
    if !matches!(dpi, 203 | 300) {
        return Err("DPI printer harus 203 atau 300".into());
    }
    if items.is_empty()
        || items.len() > MAX_PRODUCTS
        || items
            .iter()
            .any(|i| !(1..=MAX_LABELS).contains(&i.quantity))
    {
        return Err("Pilih obat dan jumlah label. Maksimum 1.000 label per cetak.".into());
    }
    let count: i32 = items.iter().map(|i| i.quantity).sum();
    if count > MAX_LABELS {
        return Err("Maksimum 1.000 label per cetak".into());
    }
    let barcodes = items
        .iter()
        .map(|i| Barcode::new(i.code))
        .collect::<Result<Vec<_>, _>>()?;
    let automatic = barcodes
        .iter()
        .map(|b| b.width as f64 * 0.254 + 4.0)
        .fold(50.0, f64::max);
    let (width, height, columns) = size
        .map(|(w, h, c)| (w as f64, h as f64, c))
        .unwrap_or((automatic, 30.0, 1));
    if width < 20.0 || height < 20.0 || !(1..=4).contains(&columns) {
        return Err("Ukuran label tidak valid".into());
    }
    for (item, barcode) in items.iter().zip(&barcodes) {
        if !item.price.is_finite() || item.price < 0.0 || item.price >= 1e13 {
            return Err("Harga label tidak valid".into());
        }
        if (width - 4.0) / (barcode.width as f64) < (50.8 / dpi as f64).max(0.19) {
            return Err(format!(
                "Kode {} terlalu panjang untuk label ini. Pilih ukuran lebih lebar, Otomatis, atau DPI yang sesuai printer.",
                item.code
            ));
        }
    }
    Ok(LabelLayout {
        width,
        height,
        columns,
        barcodes,
    })
}

pub fn price_text(price: f64) -> String {
    let cents = (price * 100.0).round() as u64;
    let digits = (cents / 100).to_string();
    let mut output = String::from("Rp ");
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            output.push('.');
        }
        output.push(digit);
    }
    if cents % 100 != 0 {
        write!(output, ",{:02}", cents % 100).unwrap();
    }
    output
}

pub fn file_uri(path: &std::path::Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    let mut encoded = String::new();
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~:".contains(&byte) {
            encoded.push(byte as char);
        } else {
            write!(encoded, "%{byte:02X}").unwrap();
        }
    }
    if encoded.starts_with("//") {
        format!("file:{encoded}")
    } else if encoded.starts_with('/') {
        format!("file://{encoded}")
    } else {
        format!("file:///{encoded}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mass_sheet_preserves_each_medicine_and_quantity() {
        let mut items = [
            LabelItem {
                code: "0012",
                name: "Obat <A>",
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
                quantity: 1,
            },
        ];
        let size = Some((50, 30, 3));
        let layout = prepare_layout(&items, size, 203).unwrap();
        assert_eq!(
            (layout.width, layout.height, layout.columns),
            (50.0, 30.0, 3)
        );
        assert_eq!(layout.barcodes.len(), 3);
        assert_eq!(items.iter().map(|item| item.quantity).sum::<i32>(), 6);
        assert_eq!(price_text(items[0].price), "Rp 12.500,50");
        assert!(prepare_layout(&items, size, 96).is_err());
        assert!(prepare_layout(&items, Some((20, 30, 3)), 203).is_err());
        assert!(prepare_layout(&items, Some((50, 30, 0)), 203).is_err());
        items[0].quantity = 997;
        assert!(prepare_layout(&items, size, 203).is_err());
        items[0].quantity = 1;
        for price in [f64::NAN, f64::INFINITY, -0.01, 1e13] {
            items[0].price = price;
            assert!(prepare_layout(&items, size, 203).is_err());
        }
        assert_eq!(price_text(0.0), "Rp 0");
        assert_eq!(price_text(999.999), "Rp 1.000");
    }

    #[test]
    fn barcode_labels_encode_and_preserve_codes() {
        // Published Code 128-B vector: start 104, A/B/C (33/34/35), checksum 1, stop 106.
        let abc = Barcode::new("ABC").unwrap();
        assert_eq!(abc.width, 88);
        assert_eq!(&abc.bars[..3], &[(10, 2), (13, 1), (16, 1)]);
        assert_eq!(&abc.bars[12..15], &[(54, 2), (58, 2), (61, 2)]);
        // Numeric vector: start 105, 00/12 (0/12), checksum 26.
        let numeric = Barcode::new("0012").unwrap();
        assert_eq!(numeric.width, 77);
        assert_eq!(&numeric.bars[9..12], &[(43, 3), (48, 1), (51, 2)]);
        for code in ["", "é", "123\n", &"x".repeat(51)] {
            assert!(Barcode::new(code).is_err());
        }
        assert_eq!(escape("Obat <A> & B"), "Obat &lt;A&gt; &amp; B");
        assert!(abc.svg().contains("<svg"));
        assert!(prepare_layout(
            &[LabelItem {
                code: "0012",
                name: "Obat",
                price: 0.0,
                quantity: 0
            }],
            None,
            203
        )
        .is_err());
        assert!(prepare_layout(
            &[LabelItem {
                code: "0012",
                name: "Obat",
                price: 0.0,
                quantity: 1001
            }],
            None,
            203
        )
        .is_err());
        assert_eq!(
            file_uri(std::path::Path::new("/tmp/label obat#1.html")),
            "file:///tmp/label%20obat%231.html"
        );
        assert_eq!(
            file_uri(std::path::Path::new("C:\\Label Obat\\contoh.html")),
            "file:///C:/Label%20Obat/contoh.html"
        );
    }
}
