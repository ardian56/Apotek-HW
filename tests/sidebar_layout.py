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
    x11 = ui.x11
    x11.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
    x11.XDefaultRootWindow.restype = ctypes.c_ulong
    x11.XQueryTree.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.POINTER(ctypes.c_ulong)), ctypes.POINTER(ctypes.c_uint)]
    x11.XFetchName.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_char_p)]
    x11.XFree.argtypes = [ctypes.c_void_p]
    x11.XGetGeometry.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_uint), ctypes.POINTER(ctypes.c_uint), ctypes.POINTER(ctypes.c_uint), ctypes.POINTER(ctypes.c_uint)]
    x11.XTranslateCoordinates.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_int, ctypes.c_int, ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_int), ctypes.POINTER(ctypes.c_ulong)]
    x11.XResizeWindow.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_uint, ctypes.c_uint]
    x11.XSetInputFocus.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]
    ui.xtest.XTestFakeKeyEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]

    def shell_window():
        root,parent,count=ctypes.c_ulong(),ctypes.c_ulong(),ctypes.c_uint()
        children=ctypes.POINTER(ctypes.c_ulong)();found=None
        x11.XQueryTree(ui.display,x11.XDefaultRootWindow(ui.display),ctypes.byref(root),ctypes.byref(parent),ctypes.byref(children),ctypes.byref(count))
        for index in range(count.value):
            title=ctypes.c_char_p()
            if x11.XFetchName(ui.display,children[index],ctypes.byref(title)) and title.value:
                if title.value.decode().endswith(' · Kasir'):found=children[index]
            if title:x11.XFree(title)
        if children:x11.XFree(children)
        assert found is not None,'Main window disappeared'
        return found

    def geometry():
        window=shell_window();root=ctypes.c_ulong();x,y=ctypes.c_int(),ctypes.c_int()
        width,height,border,depth=[ctypes.c_uint() for _ in range(4)]
        x11.XGetGeometry(ui.display,window,ctypes.byref(root),ctypes.byref(x),ctypes.byref(y),ctypes.byref(width),ctypes.byref(height),ctypes.byref(border),ctypes.byref(depth))
        child=ctypes.c_ulong()
        x11.XTranslateCoordinates(ui.display,window,x11.XDefaultRootWindow(ui.display),0,0,ctypes.byref(x),ctypes.byref(y),ctypes.byref(child))
        return x.value,y.value,width.value,height.value

    class SizeHints(ctypes.Structure):
        _fields_=[('flags',ctypes.c_long)]+[(name,ctypes.c_int) for name in ['x','y','width','height','min_width','min_height','max_width','max_height','width_inc','height_inc','min_num','min_den','max_num','max_den','base_width','base_height','gravity']]
    x11.XGetWMNormalHints.argtypes=[ctypes.c_void_p,ctypes.c_ulong,ctypes.POINTER(SizeHints),ctypes.POINTER(ctypes.c_long)]
    hints,supplied=SizeHints(),ctypes.c_long()
    assert x11.XGetWMNormalHints(ui.display,shell_window(),ctypes.byref(hints),ctypes.byref(supplied)),'Resize hints missing'
    assert not (hints.flags & 32) or (hints.max_width>hints.min_width and hints.max_height>hints.min_height),'Window still has a fixed maximum size'
    original=geometry()
    for width,height in [(1120,660),(1320,692),original[2:]]:
        x11.XResizeWindow(ui.display,shell_window(),width,height);x11.XFlush(ui.display);time.sleep(0.3)
        assert geometry()[2:]==(width,height),'Window did not resize'
        assert app.poll() is None,'Resize stopped the app'

    def fullscreen_key(name):
        x11.XStringToKeysym.argtypes=[ctypes.c_char_p];x11.XStringToKeysym.restype=ctypes.c_ulong
        x11.XKeysymToKeycode.argtypes=[ctypes.c_void_p,ctypes.c_ulong];x11.XKeysymToKeycode.restype=ctypes.c_ubyte
        x11.XSetInputFocus(ui.display,shell_window(),1,0)
        code=x11.XKeysymToKeycode(ui.display,x11.XStringToKeysym(name.encode()))
        for pressed in [1,0]:ui.xtest.XTestFakeKeyEvent(ui.display,code,pressed,0)
        x11.XFlush(ui.display);time.sleep(0.3)
    fullscreen_key('F11')
    screen=x11.XDefaultScreen(ui.display)
    assert geometry()[2:]==(x11.XDisplayWidth(ui.display,screen),x11.XDisplayHeight(ui.display,screen)),'F11 did not enter fullscreen'
    fullscreen_key('Escape')
    assert geometry()[2:]==original[2:],'Escape did not restore the original size'
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
    print('PASS: sidebar, page navigation, resize hints, window resize, fullscreen/restore, and account menu')
finally:
    app.terminate()
    app.wait(timeout=5)
