use std::fmt::Write;

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
        // Numeric codes use set C to keep labels compact and preserve leading zeroes.
        let numeric = code.len() % 2 == 0 && code.bytes().all(|b| b.is_ascii_digit());
        let start = if numeric { 105 } else { 104 };
        let data = if numeric {
            code.as_bytes()
                .chunks_exact(2)
                .map(|p| ((p[0] - b'0') * 10 + p[1] - b'0') as usize)
                .collect::<Vec<_>>()
        } else {
            code.bytes().map(|b| (b - 32) as usize).collect()
        };
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

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn label_html(code: &str, name: &str, quantity: i32) -> Result<String, String> {
    if !(1..=1000).contains(&quantity) {
        return Err("Jumlah label harus 1–1000".into());
    }
    let barcode = Barcode::new(code)?;
    // Keep the narrow bar at 0.254 mm (two dots on a 203 dpi printer).
    let label_width = (barcode.width as f64 * 0.254 + 8.0).max(50.0);
    let label = format!(
        "<article class=\"label\"><div class=\"name\">{}</div>{}<div class=\"code\">{}</div></article>",
        escape(name), barcode.svg(), escape(code)
    );
    Ok(format!(
        r#"<!doctype html>
<html lang="id"><meta charset="utf-8"><title>Label barcode obat</title>
<style>
* {{ box-sizing: border-box; }}
body {{ margin: 24px; color: #334155; font: 14px Arial, sans-serif; background: #f8fafc; }}
.controls {{ margin-bottom: 24px; }}
button {{ border: 0; border-radius: 5px; padding: 10px 18px; color: white; background: #0d9488; cursor: pointer; }}
.sheet {{ display: flex; flex-wrap: wrap; gap: 2mm; }}
.label {{ width: {label_width:.3}mm; height: 30mm; padding: 2mm; text-align: center; background: white; color: black; break-inside: avoid; border: 1px solid #e2e8f0; }}
.name {{ height: 8mm; overflow: hidden; font-size: 10px; line-height: 3.5mm; }}
svg {{ display: block; margin: 0 auto 1mm; }}
.code {{ font: 10px monospace; white-space: nowrap; }}
@page {{ margin: 10mm; }}
@media print {{ body {{ margin: 0; background: white; }} .controls {{ display: none; }} .label {{ border-color: transparent; }} }}
</style>
<body><div class="controls"><button onclick="window.print()">Cetak label</button>
<p>Cetak dengan skala 100% / ukuran sebenarnya. Matikan header dan footer cetak.</p></div>
<div class="sheet">{}</div></body></html>"#,
        label.repeat(quantity as usize)
    ))
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
        let html = label_html("0012", "Obat <A> & B", 3).unwrap();
        assert_eq!(html.matches("<article ").count(), 3);
        assert!(html.contains("Obat &lt;A&gt; &amp; B"));
        assert!(html.contains("0012</div>"));
        assert!(label_html("0012", "Obat", 0).is_err());
        assert!(label_html("0012", "Obat", 1001).is_err());
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
