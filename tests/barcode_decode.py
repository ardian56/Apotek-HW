"""Verify barcode vectors with an independent reader: python3 tests/barcode_decode.py.

Requires rustc, rsvg-convert, and zbarimg. No database or GUI required.
"""
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
CODES = [
    "ABC", "0012", "8999909001001", "CTH-001", "00100349230000001234",
    "".join(chr(n) for n in range(32, 80)),
    "".join(chr(n) for n in range(80, 127)),
    "9" * 50, "AK0209", "AK0209-AB123456-CD", "123456AB",
]

with tempfile.TemporaryDirectory(prefix="apotek-barcode-test-") as directory:
    output = Path(directory)
    module = json.dumps(str(ROOT / "src/services/barcode.rs"), ensure_ascii=False)
    codes = ", ".join(json.dumps(code) for code in CODES)
    source = output / "encode.rs"
    source.write_text(f"""#[path = {module}] mod barcode;
fn sized_svg(barcode: &barcode::Barcode, w: f64, h: f64) -> String {{
    barcode.svg().replace(&format!("{{}}mm", barcode.width as f64 * 0.254), &format!("{{}}mm", w - 4.0))
        .replace("height=\\\"12mm\\\"", &format!("height=\\\"{{}}mm\\\"", h - 12.0))
}}
fn main() {{
    let output = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    for (i, code) in [{codes}].iter().enumerate() {{
        let svg = barcode::Barcode::new(code).unwrap().svg();
        std::fs::write(output.join(format!("{{i}}.svg")), svg).unwrap();
    }}
    for (i, (code, w, h, dpi)) in [("CTH-001",30,20,300),("00100349230000001234",40,25,300),("8999909001001",50,30,203)].iter().enumerate() {{
        let items = [barcode::LabelItem{{code,name:"Obat A",price:12500.0,quantity:2}}];
        let layout = barcode::prepare_layout(&items,Some((*w,*h,3)),*dpi).unwrap();
        std::fs::write(output.join(format!("bulk-{{i}}.svg")), sized_svg(&layout.barcodes[0],layout.width,layout.height)).unwrap();
    }}
    let items = [
        barcode::LabelItem{{code:"0012",name:"Obat A",price:12500.50,quantity:2}},
        barcode::LabelItem{{code:"CTH-002",name:"Obat B",price:900.0,quantity:3}},
        barcode::LabelItem{{code:"8999909001001",name:"Obat C",price:0.0,quantity:1}},
    ];
    let layout = barcode::prepare_layout(&items,Some((50,30,3)),203).unwrap();
    assert_eq!(layout.columns,3);
    assert_eq!(barcode::price_text(items[0].price),"Rp 12.500,50");
    let mut i = 0;
    for (item, barcode) in items.iter().zip(&layout.barcodes) {{
        for _ in 0..item.quantity {{
            std::fs::write(output.join(format!("mixed-{{i}}.svg")),sized_svg(barcode,layout.width,layout.height)).unwrap();
            i += 1;
        }}
    }}
    assert_eq!(i,6);
}}
""")
    binary = output / "encode"
    subprocess.run(["rustc", "--edition=2021", "--cfg=test", "--cap-lints=allow", str(source), "-o", str(binary)], check=True)
    subprocess.run([str(binary), str(output)], check=True)

    def decode(svg, code, dpi):
        image = svg.with_suffix(".png")
        subprocess.run(["rsvg-convert", f"--dpi-x={dpi}", f"--dpi-y={dpi}", str(svg), "-o", str(image)], check=True)
        decoded = subprocess.check_output(["zbarimg", "--quiet", "--raw", "--nodbus", "-Sdisable", "-Scode128.enable", str(image)], text=True)
        assert decoded.removesuffix("\n") == code, f"Printed barcode changed code {code!r}"

    for i, code in enumerate(CODES):
        decode(output / f"{i}.svg", code, 203)
    for i, code in enumerate(["CTH-001", "00100349230000001234", "8999909001001"]):
        decode(output / f"bulk-{i}.svg", code, 300 if i < 2 else 203)
    expected = ["0012"] * 2 + ["CTH-002"] * 3 + ["8999909001001"]
    assert len(list(output.glob("mixed-*.svg"))) == len(expected)
    for i, code in enumerate(expected):
        decode(output / f"mixed-{i}.svg", code, 203)
    print(f"PASS: {len(CODES)} codes, 3 label presets, and all 6 mixed medicine vectors decode exactly")
