"""Verify printable barcodes with an independent reader: python3 tests/barcode_decode.py.

Requires rustc, rsvg-convert, and zbarimg. No database or GUI required.
"""
import json
import re
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
fn main() {{
    let output = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    for (i, code) in [{codes}].iter().enumerate() {{
        let svg = barcode::Barcode::new(code).unwrap().svg();
        std::fs::write(output.join(format!("{{i}}.svg")), svg).unwrap();
    }}
    for (i,(code,w,h,dpi)) in [("CTH-001",30,20,300),("00100349230000001234",40,25,300),("8999909001001",50,30,203)].iter().enumerate() {{
        let html=barcode::labels_html_at_dpi(&[barcode::LabelItem{{code,name:"Obat <A>",price:12500.0,quantity:2}}],Some((*w,*h,3)),*dpi).unwrap();
        std::fs::write(output.join(format!("bulk-{{i}}.html")),html).unwrap();
    }}
}}
""")
    binary = output / "encode"
    subprocess.run(["rustc", "--edition=2021", "--cap-lints=allow", str(source), "-o", str(binary)], check=True)
    subprocess.run([str(binary), str(output)], check=True)
    for i, code in enumerate(CODES):
        image = output / f"{i}.png"
        subprocess.run(["rsvg-convert", "--dpi-x=203", "--dpi-y=203", str(output / f"{i}.svg"), "-o", str(image)], check=True)
        decoded = subprocess.check_output(["zbarimg", "--quiet", "--raw", "--nodbus", "-Sdisable", "-Scode128.enable", str(image)], text=True)
        assert decoded.removesuffix("\n") == code, f"Printed barcode changed code {code!r}"
    for i, code in enumerate(["CTH-001", "00100349230000001234", "8999909001001"]):
        html=(output/f"bulk-{i}.html").read_text()
        assert html.count("<article class=label>")==2 and "Rp 12.500" in html
        svg=re.search(r"<svg.*?</svg>",html,re.S).group()
        path=output/f"bulk-{i}.svg";path.write_text(svg)
        image=output/f"bulk-{i}.png"
        subprocess.run(["rsvg-convert",f"--dpi-x={300 if i<2 else 203}",f"--dpi-y={300 if i<2 else 203}",str(path),"-o",str(image)],check=True)
        decoded=subprocess.check_output(["zbarimg","--quiet","--raw","--nodbus","-Sdisable","-Scode128.enable",str(image)],text=True)
        assert decoded.removesuffix("\n")==code, "Selected label size no longer decodes"
    print(f"PASS: {len(CODES)} printable barcodes decode exactly, including leading zeroes")
