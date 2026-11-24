"""Check sidebar on a smaller screen: xvfb-run -a -s '-screen 0 1366x720x24' python3 tests/sidebar_layout.py."""
import ctypes
import subprocess
import time
from pathlib import Path
import table_redraw as ui

app = subprocess.Popen([str(ui.ROOT / 'target/debug/kasir_apotek'), '--preview'], cwd=ui.ROOT,
                       stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
try:
    time.sleep(0.8)
    ui.navigate('Obat')
    before = ui.capture()
    Path('/tmp/apotek-sidebar-small.png').write_bytes(before)
    active = f"2x2+32+{ui.nav_rows()['Obat']}"
    assert ui.pixels(before, active) == bytes.fromhex('e7f3f1') * 4, 'Active medicine menu is not marked'
    x, y = ui.page_position(40, 20)
    content = f"700x90+{x}+{y}"
    ui.click(90, ui.nav_rows()['Master data'])
    ui.expanded = None
    folded = ui.capture()
    assert ui.pixels(folded, content) == ui.pixels(before, content), 'Folding sidebar changed the active page'
    ui.navigate('Penjualan')
    ui.page_click(1080, 41)
    cashier = ui.capture()
    active = f"2x2+32+{ui.nav_rows()['Penjualan']}"
    assert ui.pixels(cashier, active) == bytes.fromhex('e7f3f1') * 4, 'New sale lost the active menu marker'
    ui.page_click(1080,41)
    assert ui.pixels(ui.capture(), active) == bytes.fromhex('e7f3f1') * 4, 'Back to sales lost the active menu marker'
    for name in ['Pembelian', 'Hutang', 'Piutang', 'Pembayaran', 'Retur', 'Suplier', 'Pelanggan',
                 'Sales', 'Operator', 'Stok opname', 'Kartu stok', 'Laporan', 'Pengaturan', 'Ringkasan']:
        ui.navigate(name)
        assert app.poll() is None, f'App stopped on {name}'
    ui.click(90, ui.SHELL_HEIGHT - 59 + 20)
    time.sleep(0.2)
    x11 = ui.x11
    x11.XStringToKeysym.argtypes = [ctypes.c_char_p]
    x11.XStringToKeysym.restype = ctypes.c_ulong
    x11.XKeysymToKeycode.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
    x11.XKeysymToKeycode.restype = ctypes.c_ubyte
    ui.xtest.XTestFakeKeyEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
    key = x11.XKeysymToKeycode(ui.display, x11.XStringToKeysym(b'Escape'))
    for pressed in [1, 0]:
        ui.xtest.XTestFakeKeyEvent(ui.display, key, pressed, 0)
    x11.XFlush(ui.display)
    assert app.poll() is None, 'App stopped in account menu'
    print('PASS: compact sidebar, folding, active markers, page navigation, and account menu')
finally:
    app.terminate()
    app.wait(timeout=5)
