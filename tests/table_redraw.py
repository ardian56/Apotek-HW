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



def main():
    app = subprocess.Popen([str(ROOT / "target/debug/kasir_apotek"), "--preview"], cwd=ROOT,
                           stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    try:
        time.sleep(0.7)
        click(550, 45)   # Master data.
        click(805, 98)   # Stok opname.
        before = capture()
        # Exclude the antialiased outer border; only table content must stay identical.
        regions = ("1080x24+40+302", "280x25+850+334", "10x6+1155+300")
        expected = [pixels(before, region) for region in regions]
        assert len(set(expected[0])) > 1, "Table header must be visible before selection"
        assert len(set(expected[1])) > 1, "Unit and stock values must be visible before selection"
        for x, y in ((350, 346), (85, 378), (1040, 410), (350, 346)):
            click(x, y)
            after = capture()
            for region, original in zip(regions, expected):
                assert pixels(after, region) == original, f"Header or unselected columns erased after selecting ({x}, {y})"
            if (x, y) == (350, 346):
                assert pixels(after, "500x24+205+334") != pixels(before, "500x24+205+334"), "Selection must change the medicine cell"
            assert app.poll() is None, "App crashed during table selection"
        for _ in range(3):
            click(1000, 450, button=5)  # Scroll down.
        scrolled = capture()
        assert pixels(scrolled, regions[0]) == expected[0], "Header erased after scrolling"
        assert pixels(scrolled, regions[2]) == expected[2], "Scrollbar arrow erased after scrolling"
        assert pixels(scrolled, regions[1]) != expected[1], "Wheel must scroll medicine rows"
        for _ in range(3):
            click(1000, 450, button=4)  # Scroll up.
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
