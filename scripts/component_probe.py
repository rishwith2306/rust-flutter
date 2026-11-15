"""Rendered component test shared by desktop and isolated Android smoke tests.
Uses injected pointer events and image patches, without component state backdoors.
Requires ImageMagick's convert. Coordinates are physical pixels.
"""
import re
import subprocess
import time


def bounds(path, color):
    # wgpu's EGL config on some emulator images uses RGB565. Match either the
    # requested RGB888 fill or its explicitly quantized RGB565 representation;
    # keep the same 1% locator tolerance and exact foreground-mask comparisons.
    rgb = [int(color[index:index+2], 16) for index in (1, 3, 5)]
    quantized = "#" + "".join(f"{round(round(value * levels / 255) * 255 / levels):02x}"
        for value, levels in zip(rgb, (31, 63, 31)))
    for candidate in dict.fromkeys((color, quantized)):
        result = subprocess.run(["convert", str(path), "-alpha", "off", "-fuzz", "1%", "-fill", "white", "+opaque", candidate, "-fill", "black", "-opaque", candidate, "-format", "%@", "info:"], capture_output=True, text=True, timeout=15, check=True)
        match = re.fullmatch(r"(\d+)x(\d+)\+(-?\d+)\+(-?\d+)", result.stdout.strip())
        if not match:
            raise AssertionError(f"could not measure rendered component: {result.stdout} {result.stderr}")
        w, h, x, y = map(int, match.groups())
        if w > 0 and h > 0:
            return x, y, w, h
    return None


def center(rect):
    x, y, w, h = rect
    return x + w / 2, y + h / 2


def patch(path, rect, scale, width=96):
    x, y = center(rect)
    w, h = round(width * scale), round(40 * scale)
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
    aspect_control = bounds(stepped, "#6e4187")
    assert aspect_control, "aspect-specific control is missing"
    tap(*center(aspect_control))
    selective = settle("components-aspect-step.png")
    assert patch(selective, bounds(selective, "#1769aa"), scale) == a1, "aspect update reset A"
    assert patch(selective, bounds(selective, "#197d55"), scale) == b2, "aspect update changed B's count"
    tap(*center(bounds(selective, "#1769aa")))
    after_aspect = settle("components-aspect-count.png")
    assert patch(after_aspect, bounds(after_aspect, "#1769aa"), scale) != a1
    assert patch(after_aspect, bounds(after_aspect, "#197d55"), scale) == b2
    observable = bounds(after_aspect, "#a14b32")
    assert observable, "observable counter is missing"
    observable_zero = patch(after_aspect, observable, scale, 220)
    for _ in range(3):
        tap(*center(observable))
        time.sleep(0.3)
    individually = settle("components-observable-individual.png")
    observable_three = patch(individually, bounds(individually, "#a14b32"), scale, 220)
    assert observable_three != observable_zero, "observable clicks did not change the rendered value"
    tap(*center(bounds(individually, "#4b627d")))
    reset = settle("components-observable-reset.png")
    assert patch(reset, bounds(reset, "#a14b32"), scale, 220) == observable_zero
    tap(*center(bounds(reset, "#8f5f22")))
    burst = settle("components-observable-burst.png")
    assert patch(burst, bounds(burst, "#a14b32"), scale, 220) == observable_three, "three synchronous notifier changes did not match three individual clicks"
    return {"passed": True, "inherited_step": True, "inherited_aspect": True, "notifier_burst": True, "scope": "injected clicks, keyed swap, removal/reinsertion, inherited step change preserving local state, then one click matching the original two-click result; A-only aspect update preserves both counts, then A remains interactive while B count stays unchanged; observable reset and three-change burst match the three-click counter; exact trimmed foreground glyph masks at a 75% intensity threshold"}
