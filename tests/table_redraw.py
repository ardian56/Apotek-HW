"""Check table redraw: xvfb-run -a -s '-screen 0 1366x768x24' python3 tests/table_redraw.py."""
import ctypes
import ctypes.util
import os
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
DISPLAY = os.environ.get("DISPLAY")
if not DISPLAY:
    raise SystemExit("Run under X11: xvfb-run -a -s '-screen 0 1366x768x24' python3 tests/table_redraw.py")

x11 = ctypes.CDLL(ctypes.util.find_library("X11"))
xtest = ctypes.CDLL(ctypes.util.find_library("Xtst"))
x11.XOpenDisplay.argtypes = [ctypes.c_char_p]
x11.XOpenDisplay.restype = ctypes.c_void_p
x11.XFlush.argtypes = [ctypes.c_void_p]
xtest.XTestFakeMotionEvent.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_ulong]
xtest.XTestFakeButtonEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
display = x11.XOpenDisplay(DISPLAY.encode())
if not display:
    raise SystemExit("Cannot open virtual display")


def click(x, y, button=1):
    xtest.XTestFakeMotionEvent(display, -1, x, y, 0)
    xtest.XTestFakeButtonEvent(display, button, 1, 0)
    x11.XFlush(display)
    time.sleep(0.08)
    xtest.XTestFakeButtonEvent(display, button, 0, 0)
    x11.XFlush(display)
    time.sleep(0.25)


def capture():
    return subprocess.check_output(["import", "-window", "root", "png:-"])


def pixels(image, region):
    result = subprocess.check_output(["magick", "png:-", "-crop", region, "+repage", "-depth", "8", "rgb:-"], input=image)
    width, height = map(int, region.split("+")[0].split("x"))
    assert len(result) == width * height * 3, f"Region {region} must fit inside virtual screen"
    return result


x11.XDefaultScreen.argtypes = [ctypes.c_void_p]
x11.XDefaultScreen.restype = ctypes.c_int
x11.XDisplayWidth.argtypes = [ctypes.c_void_p, ctypes.c_int]
x11.XDisplayWidth.restype = ctypes.c_int
x11.XDisplayHeight.argtypes = [ctypes.c_void_p, ctypes.c_int]
x11.XDisplayHeight.restype = ctypes.c_int
screen = x11.XDefaultScreen(display)
SHELL_WIDTH = min(1320, x11.XDisplayWidth(display, screen) - 32)
SHELL_HEIGHT = min(740, x11.XDisplayHeight(display, screen) - 28)
PAGE_WIDTH, PAGE_HEIGHT = SHELL_WIDTH - 188, SHELL_HEIGHT - 56
PAGE_LEFT, PAGE_TOP = 1 + 188, 20 + 56
NAV_GROUPS = [("Ringkasan", []), ("Transaksi", ["Penjualan", "Pembelian", "Hutang", "Piutang", "Pembayaran", "Retur"]),
              ("Master data", ["Obat", "Suplier", "Pelanggan", "Sales", "Operator", "Stok opname", "Kartu stok"]),
              ("Laporan", []), ("Pengaturan", [])]
expanded = None


def page_position(x, y):
    return PAGE_LEFT + x * PAGE_WIDTH // 1200, PAGE_TOP + y * PAGE_HEIGHT // 612


def page_click(x, y, button=1):
    click(*page_position(x, y), button=button)


def row_click(index, x=350, table_y=166, button=1):
    px, py = page_position(x, table_y)
    click(px, py + 36 + index * 32 + 16, button=button)


def nav_rows():
    y = 116
    result = {}
    header_height, row_height = (36, 30) if SHELL_HEIGHT < 728 else (40, 32)
    for i, (header, children) in enumerate(NAV_GROUPS):
        result[header] = y + header_height // 2 + 20
        y += header_height
        if expanded == i:
            y += 4
            for child in children:
                result[child] = y + row_height // 2 + 20
                y += row_height + 2
        y += 6
    return result


def navigate(name):
    global expanded
    for i, (header, children) in enumerate(NAV_GROUPS):
        if name == header and not children:
            click(90, nav_rows()[header])
            expanded = None
            return
        if name in children:
            if expanded != i:
                click(90, nav_rows()[header])
                expanded = i
            click(90, nav_rows()[name])
            return
    raise AssertionError(f"Unknown navigation: {name}")


def main():
    app = subprocess.Popen([str(ROOT / "target/debug/kasir_apotek"), "--preview"], cwd=ROOT,
                           stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    try:
        time.sleep(0.7)
        navigate("Stok opname")
        before = capture()
        # Exclude the antialiased outer border; only table content must stay identical.
        gx, gy = page_position(28, 166)
        gw = 1144 * PAGE_WIDTH // 1200
        sx, _ = page_position(850, 166)
        regions = (f"1000x24+{gx+12}+{gy+6}", f"240x25+{sx}+{gy+40}", f"10x6+{gx+gw-16}+{gy+4}")
        expected = [pixels(before, region) for region in regions]
        assert len(set(expected[0])) > 1, "Table header must be visible before selection"
        assert len(set(expected[1])) > 1, "Unit and stock values must be visible before selection"
        for x, row in ((350, 0), (85, 1), (1040, 2), (350, 0)):
            row_click(row, x=x)
            after = capture()
            for region, original in zip(regions, expected):
                assert pixels(after, region) == original, f"Header or unselected columns erased after selecting ({x}, row {row})"
            if (x, row) == (350, 0):
                selected = f"460x24+{page_position(205,166)[0]}+{gy+40}"
                assert pixels(after, selected) != pixels(before, selected), "Selection must change the medicine cell"
            assert app.poll() is None, "App crashed during table selection"
        for _ in range(3):
            page_click(1000, 322, button=5)  # Scroll down.
        scrolled = capture()
        assert pixels(scrolled, regions[0]) == expected[0], "Header erased after scrolling"
        assert pixels(scrolled, regions[2]) == expected[2], "Scrollbar arrow erased after scrolling"
        assert pixels(scrolled, regions[1]) != expected[1], "Wheel must scroll medicine rows"
        for _ in range(3):
            page_click(1000, 322, button=4)  # Scroll up.
        restored = capture()
        for region, original in zip(regions, expected):
            assert pixels(restored, region) == original, "Table content did not return after scrolling up"
        assert app.poll() is None, "App crashed during table scrolling"
        print("PASS: table header, other columns, and scrollbar persist after selection and scrolling")
    finally:
        app.terminate()
        app.wait(timeout=5)


if __name__ == "__main__":
    main()
