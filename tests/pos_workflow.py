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


def window_geometry(title):
    root = ctypes.c_ulong()
    x, y = ctypes.c_int(), ctypes.c_int()
    width, height, border, depth = [ctypes.c_uint() for _ in range(4)]
    x11.XGetGeometry(display, windows()[title], ctypes.byref(root), ctypes.byref(x), ctypes.byref(y),
        ctypes.byref(width), ctypes.byref(height), ctypes.byref(border), ctypes.byref(depth))
    return x.value, y.value, width.value, height.value


def window_click(title, px, py, button=1):
    x, y, width, _ = window_geometry(title)
    click(x + (width + px if px < 0 else px), y + py, button=button)


def window_pixels(image, title, px, py, width, height):
    x, y, _, _ = window_geometry(title)
    return pixels(image, f"{width}x{height}+{x + px}+{y + py}")


def decode_preview(name):
    screenshot(name)
    decoded = subprocess.check_output(["zbarimg", "--quiet", "--raw", "--nodbus", "-Sdisable", "-Scode128.enable", f"/tmp/apotek-pos-{name}.png"], text=True)
    return set(decoded.splitlines())


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
        page_click(838,569);time.sleep(0.25)
        assert "Pratinjau struk" in windows(),"Receipt did not stay inside the app"
        assert not uri_file.exists(),"Receipt unexpectedly opened the browser"
        screenshot("receipt-preview");key("Escape");time.sleep(0.2)
        navigate("Obat");master=screenshot("master");assert len(set(pixels(master,master_ed_region())))>1,"ED missing"
        row_click(0)
        shift=x11.XKeysymToKeycode(display,x11.XStringToKeysym(b"Shift_L"));xtest.XTestFakeKeyEvent(display,shift,1,0);x11.XFlush(display)
        row_click(4);xtest.XTestFakeKeyEvent(display,shift,0,0);x11.XFlush(display)
        row_click(4,button=3);screenshot("context");key("Down");key("Down");key("Down");key("Return");time.sleep(0.25)
        assert "Cetak barcode" in windows(),"Barcode menu did not open";screenshot("barcode-queue")
        window_click("Cetak barcode",300,190)
        before_append = screenshot("barcode-before-append")
        before_add = window_pixels(screenshot("barcode-before-add"),"Cetak barcode",24,490,500,20)
        window_click("Cetak barcode",840,35);time.sleep(0.25)
        assert "Pilih banyak obat" in windows(),"Tambah obat failed to open the picker"
        window_click("Pilih banyak obat",150,105);scan("cth-016")
        window_click("Pilih banyak obat",150,154)
        window_click("Pilih banyak obat",650,455);time.sleep(0.2)
        assert "Pilih banyak obat" not in windows(),"Chosen medicine did not enter the queue"
        assert window_pixels(screenshot("barcode-after-add"),"Cetak barcode",24,490,500,20) != before_add,"Tambah obat did not update label queue"
        xtest.XTestFakeKeyEvent(display,shift,1,0);x11.XFlush(display);window_click("Cetak barcode",300,222);xtest.XTestFakeKeyEvent(display,shift,0,0);x11.XFlush(display)
        assert window_pixels(screenshot("barcode-append-selection"),"Cetak barcode",600,147,40,24) == window_pixels(before_append,"Cetak barcode",600,147,40,24),"Adding medicine moved Shift anchor to first row"
        window_click("Cetak barcode",874,461);time.sleep(0.25)
        assert "Pratinjau lembar barcode" in windows(),"Mass barcode sheet preview did not open"
        expected = {f"CTH-{i:03}" for i in range(1,6)} | {"CTH-016"}
        assert expected <= decode_preview("barcode-sheet"),"Medicine barcodes missing from native sheet"
        window_click("Pratinjau lembar barcode",-90,35);key("Home");key("Down");key("Return");time.sleep(0.2)
        assert expected <= decode_preview("barcode-sheet-zoom"),"Zoom lost barcodes from native sheet"
        assert not uri_file.exists(),"Barcode preview unexpectedly opened the browser"
        before_dialog = set(windows())
        window_click("Pratinjau lembar barcode",-114,window_geometry("Pratinjau lembar barcode")[3]-31);time.sleep(0.3)
        assert set(windows()) - before_dialog,"Native print dialog did not open"
        assert not uri_file.exists(),"Barcode print unexpectedly opened the browser"
        key("Escape");time.sleep(0.2);key("Escape");time.sleep(0.2)
        before_qty = screenshot("barcode-before-quantity")
        window_click("Cetak barcode",300,158);xtest.XTestFakeKeyEvent(display,shift,1,0);x11.XFlush(display);window_click("Cetak barcode",300,190);xtest.XTestFakeKeyEvent(display,shift,0,0);x11.XFlush(display)
        window_click("Cetak barcode",742,269);key("BackSpace");key("2");window_click("Cetak barcode",830,315)
        after_qty = screenshot("barcode-after-quantity")
        assert window_pixels(before_qty,"Cetak barcode",600,147,40,24) != window_pixels(after_qty,"Cetak barcode",600,147,40,24),"Quantity did not change in label queue"
        window_click("Cetak barcode",300,158)
        control=x11.XKeysymToKeycode(display,x11.XStringToKeysym(b"Control_L"))
        xtest.XTestFakeKeyEvent(display,control,1,0);x11.XFlush(display);window_click("Cetak barcode",300,222);xtest.XTestFakeKeyEvent(display,control,0,0);x11.XFlush(display)
        before_ctrl=screenshot("barcode-before-ctrl-quantity")
        window_click("Cetak barcode",742,269);key("BackSpace");key("3");window_click("Cetak barcode",830,315)
        after_ctrl=screenshot("barcode-after-ctrl-quantity")
        for y in [147,211]:
            assert window_pixels(before_ctrl,"Cetak barcode",600,y,40,24) != window_pixels(after_ctrl,"Cetak barcode",600,y,40,24),"Ctrl-selected quantity did not change"
        assert window_pixels(before_ctrl,"Cetak barcode",600,179,40,24) == window_pixels(after_ctrl,"Cetak barcode",600,179,40,24),"Ctrl changed an unselected row"
        window_click("Cetak barcode",300,190)
        xtest.XTestFakeKeyEvent(display,shift,1,0);x11.XFlush(display);window_click("Cetak barcode",300,190);xtest.XTestFakeKeyEvent(display,shift,0,0);x11.XFlush(display)
        before_single=screenshot("barcode-before-single-shift")
        window_click("Cetak barcode",742,269);key("BackSpace");key("4");window_click("Cetak barcode",830,315)
        after_single=screenshot("barcode-after-single-shift")
        for y in [147,211]:
            assert window_pixels(before_single,"Cetak barcode",600,y,40,24) == window_pixels(after_single,"Cetak barcode",600,y,40,24),"Shift-click on same row selected extra medicines"
        assert window_pixels(before_single,"Cetak barcode",600,179,40,24) != window_pixels(after_single,"Cetak barcode",600,179,40,24),"Single selected quantity did not change"
        window_click("Cetak barcode",874,461);time.sleep(0.2)
        decoded = decode_preview("barcode-quantity-sheet")
        for _ in range(24):window_click("Pratinjau lembar barcode",400,300,button=5)
        decoded |= decode_preview("barcode-quantity-sheet-bottom")
        assert expected <= decoded,"Quantity change lost medicines from sheet"
        key("Escape");time.sleep(0.2)
        window_click("Cetak barcode",106,461);time.sleep(0.2)
        window_click("Cetak barcode",840,35);time.sleep(0.25)
        assert "Pilih banyak obat" in windows(),"Multiple-medicine picker did not open"
        window_click("Pilih banyak obat",90,455)
        window_click("Pilih banyak obat",650,455);time.sleep(0.2)
        all_status = window_pixels(screenshot("barcode-all"),"Cetak barcode",24,490,500,20)
        window_click("Cetak barcode",840,35);time.sleep(0.2)
        window_click("Pilih banyak obat",90,455);window_click("Pilih banyak obat",650,455);time.sleep(0.2)
        assert window_pixels(screenshot("barcode-all-repeat"),"Cetak barcode",24,490,500,20) == all_status,"Picking the same medicines duplicated labels"
        assert not uri_file.exists(),"Barcode queue unexpectedly opened browser"
        key("Escape");time.sleep(0.2);row_click(4,button=3);key("Down");key("Down");key("Return");time.sleep(0.25)
        assert "Kartu stok obat" in windows();screenshot("stock-card");key("Escape");time.sleep(0.2)
        row_click(4,button=3)
        for _ in range(4):key("Down")
        key("Return");time.sleep(0.25);assert "Batch dan ED obat" in windows();screenshot("batch-ed");key("Escape")
        navigate("Penjualan");screenshot("history");page_click(872,41);key("Down");key("Return");time.sleep(0.25)
        assert "Pratinjau struk" in windows(),"Archived receipt missing from app"
        assert not uri_file.exists(),"Archived receipt unexpectedly opened browser"
        screenshot("archived-receipt");key("Escape");time.sleep(0.2)
        navigate("Pengaturan");page_click(1100,41);time.sleep(0.25)
        import table_redraw as ui
        ui.expanded=2
        x,y=page_position(40,20)
        active=f"2x2+32+{ui.nav_rows()['Operator']}"
        assert pixels(screenshot("settings-accounts"),active)==bytes.fromhex('e7f3f1')*4,"Settings did not open employee accounts"
        navigate("Pengaturan");page_click(620,350)
        control=x11.XKeysymToKeycode(display,x11.XStringToKeysym(b"Control_L"))
        xtest.XTestFakeKeyEvent(display,control,1,0);x11.XFlush(display);key("a");xtest.XTestFakeKeyEvent(display,control,0,0);x11.XFlush(display)
        scan("baris pertama");scan("baris kedua")
        notes=screenshot("settings-multiline-footer")
        fx,fy=page_position(584,332)
        for offset in [6,23]:
            assert len(set(pixels(notes,f"140x12+{fx+10}+{fy+offset}")))>1,"Footer did not show ordinary multiline text"
        navigate("Penjualan")
        page_click(924,128);screenshot("history-filters")
        page_click(960,195);key("End");key("Return")
        page_click(872,41);key("Down");key("Down");key("Return");listed=document()
        assert "<th>Dokter</th>" not in listed, "Doctor is still shown in printed sales"
        assert "KREDIT" in listed and "TUNAI" not in listed, "Additional payment filter did not apply"
        navigate("Laporan");page_click(872,41);key("Down");key("Return");key("Return");time.sleep(0.3);chart=document();assert "Omzet" in chart and "Laba" in chart and "<svg" in chart
        assert app.poll() is None,"App crashed";print("PASS: inline sales, scans, invoice totals, batch barcode labels, row actions, receipts, and monthly chart")
    finally:
        app.terminate();app.wait(timeout=5)
