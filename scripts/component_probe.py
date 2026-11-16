"""Rendered component test shared by desktop and isolated Android smoke tests.
Uses injected pointer events and image patches, without component state backdoors.
Image operations require ImageMagick's convert. Coordinates are physical pixels.
"""
import re
import subprocess
import time


def native_formats(trace, minimum=1):
    formats = re.findall(r"surface native_format=(\w+) target_format=(\w+)", trace)
    assert len(formats) >= minimum, f"missing native surface diagnostics: {formats}"
    assert all(native in {"R8G8B8A8_UNORM", "R8G8B8X8_UNORM"} for native, _ in formats), f"native presentation is not RGB888: {formats}"
    return [{"native": native, "target": target} for native, target in formats]


def bounds(path, color):
    result = subprocess.run(["convert", str(path), "-alpha", "off", "-fuzz", "1%", "-fill", "white", "+opaque", color, "-fill", "black", "-opaque", color, "-format", "%@", "info:"], capture_output=True, text=True, timeout=15, check=True)
    match = re.fullmatch(r"(\d+)x(\d+)\+(-?\d+)\+(-?\d+)", result.stdout.strip())
    if not match:
        raise AssertionError(f"could not measure rendered component: {result.stdout} {result.stderr}")
    w, h, x, y = map(int, match.groups())
    return (x, y, w, h) if w > 0 and h > 0 else None


def exact_fill(path, rect, color, scale):
    """Sample a solid interior patch away from rounded borders and centered text."""
    assert rect, f"missing {color} swatch"
    x, y, _, _ = rect
    offset, side = round(8 * scale), max(1, round(2 * scale))
    rgb = subprocess.check_output(["convert", str(path), "-alpha", "off", "-crop", f"{side}x{side}+{x+offset}+{y+offset}", "-depth", "8", "rgb:-"], timeout=15)
    expected = bytes.fromhex(color[1:]) * (side * side)
    assert rgb == expected, f"native presentation altered {color}: {rgb[:12].hex()}"


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


def probe_baselines(path, trace, scale=1.0):
    """Compare rendered box offsets to the exact galley metrics used to paint."""
    metrics = re.findall(r"baseline-demo small=([\d.-]+) large=([\d.-]+)", trace)
    assert metrics, "missing actual font baseline diagnostics"
    small_metric, large_metric = map(float, metrics[-1])
    small, large, line = bounds(path, "#31537b"), bounds(path, "#67512f"), bounds(path, "#4ea4a8")
    assert small and large and line, (small, large, line)
    small_y = small[1] + small_metric * scale
    large_y = large[1] + large_metric * scale
    line_y = line[1] + line[3] / 2
    assert small[1] > large[1], "smaller font was not shifted down"
    assert abs(small_y - large_y) <= 2.0, (small_y, large_y)
    assert abs(line_y - large_y) <= 2.0, (line_y, large_y)
    return {"passed": True, "small_baseline": small_metric, "large_baseline": large_metric,
            "small_pixel_y": small_y, "large_pixel_y": large_y, "line_pixel_y": line_y,
            "tolerance_physical_pixels": 2.0}


def probe_components(capture, tap, scale=1.0):
    """capture(name) returns a PNG path; tap(x,y) injects a physical click/touch."""
    initial = capture("components-initial.png")
    blue = bounds(initial, "#1769aa")
    green = bounds(initial, "#197d55")
    assert blue and green and center(blue)[0] < center(green)[0]
    exact_fill(initial, blue, "#1769aa", scale)
    exact_fill(initial, green, "#197d55", scale)
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
    exact_fill(after_aspect, observable, "#a14b32", scale)
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
    return {"passed": True, "inherited_step": True, "inherited_aspect": True, "notifier_burst": True, "exact_rgb888_fills": True, "scope": "injected clicks, keyed swap, removal/reinsertion, inherited step change preserving local state, then one click matching the original two-click result; A-only aspect update preserves both counts, then A remains interactive while B count stays unchanged; observable reset and three-change burst match the three-click counter; exact trimmed foreground glyph masks at a 75% intensity threshold"}
