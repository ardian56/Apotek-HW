"""xvfb-run -a -s '-screen 0 1366x768x24' python3 tests/pos_workflow.py.

Runs preview only. Requires the table redraw tools and zbarimg.
"""
import ctypes
import subprocess
import time
from pathlib import Path

from table_redraw import ROOT, x11, xtest, display, click, capture, pixels, page_position, page_click, row_click, navigate, PAGE_WIDTH

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


def cart_region(x, width, row=0, height=24):
    px, py = page_position(x, 268)
    return f"{width * PAGE_WIDTH // 1200}x{height}+{px}+{py + 36 + row * 32 + 4}"


def master_ed_region():
    px, py = page_position(1035,166)
    return f"{100 * PAGE_WIDTH // 1200}x24+{px}+{py + 40}"


import os
import tempfile
from urllib.parse import unquote, urlparse

with tempfile.TemporaryDirectory(prefix="apotek-pos-test-") as directory:
    folder=Path(directory);uri_file=folder/"opened-uri";opener=folder/"xdg-open"
    opener.write_text('#!/usr/bin/python3\nimport os,sys\nfrom pathlib import Path\nPath(os.environ["APOTEK_OPEN_URI_FILE"]).write_text(sys.argv[1])\n')
    opener.chmod(0o755);env=os.environ.copy();env["PATH"]=directory+os.pathsep+env["PATH"];env["APOTEK_OPEN_URI_FILE"]=str(uri_file)
    app=subprocess.Popen([str(ROOT/"target/debug/kasir_apotek"),"--preview"],cwd=ROOT,env=env,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
    def document():
        time.sleep(0.3);assert uri_file.exists(),"Preview did not open a document";return Path(unquote(urlparse(uri_file.read_text()).path)).read_text()
    try:
        time.sleep(0.8)
        navigate("Pembelian");page_click(1080,41);key("F1");time.sleep(0.25)
        assert "Tambah barang pembelian" in windows();screenshot("purchase-form");key("Escape");time.sleep(0.2)
        navigate("Penjualan");page_click(1080,41);key("F5");key("Return");key("F1")
        assert "Tambah barang pembelian" not in windows(),"Hidden purchase intercepted F1"
        scan("cth-001");first=screenshot("first-scan");assert "Tambah obat penjualan" not in windows()
        code=pixels(first,cart_region(40,96));assert len(set(code))>1,"Scan did not add medicine"
        scan("cth-001");second=screenshot("second-scan");assert pixels(second,cart_region(40,96))==code
        assert pixels(first,cart_region(580,28))!=pixels(second,cart_region(580,28)),"Scan did not increase quantity"
        assert len(set(pixels(second,cart_region(40,740,row=1))))==1,"Repeated scan did not merge"
        scan("0000000000000000");unknown=screenshot("unknown-scan");assert pixels(unknown,cart_region(40,780,height=160))==pixels(second,cart_region(40,780,height=160))
        scan("paracetamol");assert "Tambah obat penjualan" not in windows(),"Manual search must stay inline"
        page_click(532,236);key("BackSpace");key("3");page_click(1120,236);manual=screenshot("inline-added")
        assert pixels(manual,cart_region(580,28))!=pixels(second,cart_region(580,28)),"Inline quantity did not apply"
        page_click(75,509);key("BackSpace");key("1");key("0");page_click(384,509);key("BackSpace");key("5");key("0");key("0")
        key("F2");key("BackSpace");scan("5000");screenshot("invoice-discount")
        page_click(838,569);html=document();assert "Paracetamol 500 mg" in html;assert "Rp 4.550" in html,"Invoice discount or shipping is wrong"
        screenshot("receipt-preview")
        navigate("Obat");master=screenshot("master");assert len(set(pixels(master,master_ed_region())))>1,"ED missing"
        row_click(0)
        shift=x11.XKeysymToKeycode(display,x11.XStringToKeysym(b"Shift_L"));xtest.XTestFakeKeyEvent(display,shift,1,0);x11.XFlush(display)
        row_click(4);xtest.XTestFakeKeyEvent(display,shift,0,0);x11.XFlush(display)
        row_click(4,button=3);screenshot("context");key("Down");key("Down");key("Down");key("Return");time.sleep(0.25)
        assert "Label barcode" in windows(),"Barcode menu did not open";screenshot("barcode-queue");click(100,481);labels=document()
        assert labels.count("<article class=label>")==5,"Selected medicines did not enter label queue"
        for code in ["CTH-001","CTH-002","CTH-003","CTH-004","CTH-005"]:assert code in labels
        assert "Rp 900" in labels and "3,50.0mm" not in labels
        click(300,228);xtest.XTestFakeKeyEvent(display,shift,1,0);x11.XFlush(display);click(300,260);xtest.XTestFakeKeyEvent(display,shift,0,0);x11.XFlush(display)
        click(742,290);key("BackSpace");key("2");click(830,335);click(100,481);labels=document()
        assert labels.count("<article class=label>")==7,"Label count did not apply to selected medicine"
        key("Escape");time.sleep(0.2);row_click(4,button=3);key("Down");key("Down");key("Return");time.sleep(0.25)
        assert "Kartu stok obat" in windows();screenshot("stock-card");key("Escape");time.sleep(0.2)
        row_click(4,button=3)
        for _ in range(4):key("Down")
        key("Return");time.sleep(0.25);assert "Batch dan ED obat" in windows();screenshot("batch-ed");key("Escape")
        navigate("Penjualan");screenshot("history");page_click(872,41);key("Down");key("Return");receipt=document();assert "Faktur:" in receipt,"Archived receipt missing"
        page_click(924,128);screenshot("history-filters")
        page_click(960,195);key("End");key("Return")
        page_click(872,41);key("Down");key("Down");key("Return");listed=document()
        assert "<th>Dokter</th>" not in listed, "Doctor is still shown in printed sales"
        assert "KREDIT" in listed and "TUNAI" not in listed, "Additional payment filter did not apply"
        navigate("Laporan");page_click(872,41);key("Down");key("Return");key("Return");time.sleep(0.3);chart=document();assert "Omzet" in chart and "Laba" in chart and "<svg" in chart
        assert app.poll() is None,"App crashed";print("PASS: inline sales, scans, invoice totals, batch barcode labels, row actions, receipts, and monthly chart")
    finally:
        app.terminate();app.wait(timeout=5)
