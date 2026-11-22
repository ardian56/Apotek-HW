"""xvfb-run -a -s '-screen 0 1366x768x24' python3 tests/pos_workflow.py.

Runs preview only. Requires the table redraw tools and zbarimg.
"""
import ctypes
import subprocess
import time
from pathlib import Path

from table_redraw import ROOT, x11, xtest, display, click, capture, pixels

x11.XStringToKeysym.argtypes = [ctypes.c_char_p]
x11.XStringToKeysym.restype = ctypes.c_ulong
x11.XKeysymToKeycode.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
x11.XKeysymToKeycode.restype = ctypes.c_ubyte
xtest.XTestFakeKeyEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
x11.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
x11.XDefaultRootWindow.restype = ctypes.c_ulong
x11.XQueryTree.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_ulong),
                          ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.POINTER(ctypes.c_ulong)), ctypes.POINTER(ctypes.c_uint)]
x11.XFetchName.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_char_p)]
x11.XFree.argtypes = [ctypes.c_void_p]
x11.XGetGeometry.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_ulong),
    ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_uint),
    ctypes.POINTER(ctypes.c_uint), ctypes.POINTER(ctypes.c_uint), ctypes.POINTER(ctypes.c_uint)]


def key(name):
    name = {"-": "minus"}.get(name, name)
    code = x11.XKeysymToKeycode(display, x11.XStringToKeysym(name.encode()))
    assert code, name
    xtest.XTestFakeKeyEvent(display, code, 1, 0)
    x11.XFlush(display)
    time.sleep(0.04)
    xtest.XTestFakeKeyEvent(display, code, 0, 0)
    x11.XFlush(display)
    time.sleep(0.04)


def scan(code):
    for char in code:
        key(char)
    key("Return")
    time.sleep(0.25)


def windows():
    root, parent, count = ctypes.c_ulong(), ctypes.c_ulong(), ctypes.c_uint()
    children = ctypes.POINTER(ctypes.c_ulong)()
    x11.XQueryTree(display, x11.XDefaultRootWindow(display), ctypes.byref(root),
                   ctypes.byref(parent), ctypes.byref(children), ctypes.byref(count))
    result = {}
    for i in range(count.value):
        name = ctypes.c_char_p()
        if x11.XFetchName(display, children[i], ctypes.byref(name)) and name.value:
            result[name.value.decode()] = children[i]
        if name:
            x11.XFree(name)
    if children:
        x11.XFree(children)
    return result


def window_height(title):
    root = ctypes.c_ulong()
    x, y = ctypes.c_int(), ctypes.c_int()
    width, height, border, depth = [ctypes.c_uint() for _ in range(4)]
    x11.XGetGeometry(display, windows()[title], ctypes.byref(root), ctypes.byref(x), ctypes.byref(y),
        ctypes.byref(width), ctypes.byref(height), ctypes.byref(border), ctypes.byref(depth))
    return height.value


def screenshot(name):
    path = Path(f"/tmp/apotek-pos-{name}.png")
    image = capture()
    path.write_bytes(image)
    return image


app = subprocess.Popen([str(ROOT / "target/debug/kasir_apotek"), "--preview"], cwd=ROOT,
                       stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
try:
    time.sleep(0.8)
    click(420, 45)
    click(237, 98)  # Create the purchase page before the sale page.
    click(1080, 169)
    key("F1")
    time.sleep(0.25)
    assert "Tambah barang pembelian" in windows(), "Purchase F1 must open its own form"
    screenshot("purchase-form")
    key("Escape")
    time.sleep(0.2)
    click(95, 98)
    click(1080, 169)
    click(650, 697)  # Clear the preview sale.
    click(560, 697)  # Confirm clear.
    key("F1")
    time.sleep(0.2)
    assert "Tambah barang pembelian" not in windows(), "Hidden purchase page intercepted sale F1"
    scan("cth-001")
    first = screenshot("first-scan")
    assert "Tambah obat penjualan" not in windows(), "Exact barcode must add without a modal"
    code = pixels(first, "96x24+40+334")
    assert len(set(code)) > 1, "Scan did not add medicine"
    assert len(set(pixels(first, "740x24+40+366"))) == 1, "First scan should use one row"
    scan("cth-001")
    second = screenshot("second-scan")
    assert pixels(second, "96x24+40+334") == code, "Second scan changed medicine"
    assert pixels(first, "28x24+400+334") != pixels(second, "28x24+400+334"), "Second scan must increase quantity"
    assert len(set(pixels(second, "740x24+40+366"))) == 1, "Repeated scans should merge"
    scan("0000000000000000")
    unknown = screenshot("unknown-scan")
    assert pixels(unknown, "780x310+40+334") == pixels(second, "780x310+40+334"), "Unknown barcode changed cart"
    assert "Tambah barang pembelian" not in windows(), "Unknown scan opened a purchase form"
    click(1080, 169)
    time.sleep(0.2)
    assert window_height("Tambah obat penjualan") == 430, "Sale form still includes the purchase section"
    screenshot("manual-sale")
    click(120, 410)
    manual = screenshot("manual-added")
    assert pixels(manual, "28x24+400+334") != pixels(second, "28x24+400+334"), "Manual sale did not add medicine"
    click(550, 45)
    master = screenshot("master")
    assert len(set(pixels(master, "100x24+936+334"))) > 1, "ED must be visible on the medicine table"
    click(350, 346)  # Previously select the first row.
    click(350, 474, button=3)  # Context menu must use the fifth row instead.
    screenshot("context")
    key("Down")
    key("Return")
    time.sleep(0.25)
    label = screenshot("label")
    decoded = subprocess.check_output(["zbarimg", "--quiet", "--raw", "--nodbus", "/tmp/apotek-pos-label.png"], text=True)
    assert decoded.removesuffix("\n") == "CTH-005", "Context menu acted on a previously selected row"
    key("Escape")
    time.sleep(0.2)
    click(350, 474, button=3)
    key("Down")
    key("Down")
    key("Return")
    time.sleep(0.25)
    assert "Batch dan ED obat" in windows(), "Batch and ED details did not open"
    screenshot("batch-ed")
    assert app.poll() is None, "App crashed during POS workflow"
    print("PASS: page-specific F1, direct scans, quantity merge, manual sale, visible ED, and right-click barcode")
finally:
    app.terminate()
    app.wait(timeout=5)
