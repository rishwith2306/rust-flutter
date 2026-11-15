"""Rendered component test shared by desktop and isolated Android smoke tests.
Uses injected pointer events and image patches, without component state backdoors.
Requires ImageMagick's convert. Coordinates are physical pixels.
"""
import re
import subprocess
import time


def bounds(path, color):
    result = subprocess.run(["convert", str(path), "-alpha", "off", "-fuzz", "1%", "-fill", "white", "+opaque", color, "-fill", "black", "-opaque", color, "-format", "%@", "info:"], capture_output=True, text=True, timeout=15, check=True)
    match = re.fullmatch(r"(\d+)x(\d+)\+(-?\d+)\+(-?\d+)", result.stdout.strip())
    if not match:
        raise AssertionError(f"could not measure rendered component: {result.stdout} {result.stderr}")
    w, h, x, y = map(int, match.groups())
    return (x, y, w, h) if w > 0 and h > 0 else None


def center(rect):
    x, y, w, h = rect
    return x + w / 2, y + h / 2


def patch(path, rect, scale):
    x, y = center(rect)
    w, h = round(96 * scale), round(40 * scale)
    # The demo uses white text on dark fills. Compare exact foreground glyph
    # masks, trimmed to their ink bounds: fractional density can alter edge RGB
    # values or translate rasterization by a pixel when equal-width boxes swap.
    return subprocess.check_output(["convert", str(path), "-crop", f"{w}x{h}+{round(x-w/2)}+{round(y-h/2)}", "-colorspace", "gray", "-threshold", "75%", "-trim", "+repage", "-depth", "8", "pgm:-"], timeout=15)


def probe_components(capture, tap, scale=1.0):
    """capture(name) returns a PNG path; tap(x,y) injects a physical click/touch."""
    initial = capture("components-initial.png")
    blue = bounds(initial, "#1769aa")
    green = bounds(initial, "#197d55")
    assert blue and green and center(blue)[0] < center(green)[0]
    a0, b0 = patch(initial, blue, scale), patch(initial, green, scale)
    left = min(blue[0], green[0])
    controls_y = blue[1] - 12 * scale

    def settle(name):
        time.sleep(0.5)
        return capture(name)

    tap(*center(blue))
    time.sleep(0.3)
    tap(*center(green))
    time.sleep(0.3)
    tap(*center(green))
    changed = settle("components-counted.png")
    a1 = patch(changed, bounds(changed, "#1769aa"), scale)
    b2 = patch(changed, bounds(changed, "#197d55"), scale)
    assert a1 != a0 and b2 != b0, "component clicks did not change rendered counters"
    tap(left + 72 * scale, controls_y)
    swapped = settle("components-swapped.png")
    swapped_a, swapped_b = bounds(swapped, "#1769aa"), bounds(swapped, "#197d55")
    assert center(swapped_a)[0] > center(swapped_b)[0], "keyed children did not reorder"
    assert patch(swapped, swapped_a, scale) == a1, "A lost its rendered state on reorder"
    assert patch(swapped, swapped_b, scale) == b2, "B lost its rendered state on reorder"
    tap(left + 160 * scale, controls_y)
    removed = settle("components-removed.png")
    assert bounds(removed, "#197d55") is None, "removed B still paints"
    assert patch(removed, bounds(removed, "#1769aa"), scale) == a1, "remaining A lost its state"
    tap(left + 160 * scale, controls_y)
    reinserted = settle("components-reinserted.png")
    assert patch(reinserted, bounds(reinserted, "#1769aa"), scale) == a1
    assert patch(reinserted, bounds(reinserted, "#197d55"), scale) == b0, "reinserted B did not reset to its initial rendered state"
    tap(left + 240 * scale, controls_y)
    inherited = settle("components-inherited-step.png")
    assert patch(inherited, bounds(inherited, "#1769aa"), scale) == a1, "inherited update reset A"
    assert patch(inherited, bounds(inherited, "#197d55"), scale) == b0, "inherited update reset B"
    tap(*center(bounds(inherited, "#197d55")))
    stepped = settle("components-inherited-count.png")
    assert patch(stepped, bounds(stepped, "#197d55"), scale) == b2, "one click after inherited step=2 did not match the original two-click counter"
    assert patch(stepped, bounds(stepped, "#1769aa"), scale) == a1
    return {"passed": True, "inherited_step": True, "scope": "injected clicks, keyed swap, removal/reinsertion, inherited step change preserving local state, then one click matching the original two-click result; exact trimmed foreground glyph masks at a 75% intensity threshold"}
