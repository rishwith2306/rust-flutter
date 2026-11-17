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
    animation_values = [float(value) for value in re.findall(r"animation value=([\d.eE+-]+)", trace)]
    assert any(0.0 < value < 1.0 for value in animation_values), "missing intermediate frame-driven animation"
    foreground = bounds(path, "#834eb3")
    assert foreground, "missing foreground decoration"
    assert abs(foreground[2] - 24 * scale) <= 2 and abs(foreground[3] - 24 * scale) <= 2, foreground
    exact_fill(path, foreground, "#834eb3", scale)
    assert not bounds(path, "#e3bd42"), "opaque child painted above its foreground decoration"
    return {"animation_intermediate_frames": sum(0.0 < value < 1.0 for value in animation_values), "foreground_decoration": {"passed": True, "bounds": foreground, "covered_child": True},
            "passed": True, "small_baseline": small_metric, "large_baseline": large_metric,
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
    marker = bounds(burst, "#834eb3")
    assert marker and abs(marker[2] - 24 * scale) <= 2, marker
    def await_width(name, expected):
        deadline = time.monotonic() + 12
        while time.monotonic() < deadline:
            path = capture(name)
            measured = bounds(path, "#834eb3")
            if measured and abs(measured[2] - expected * scale) <= 1:
                time.sleep(0.25)
                settled_path = capture(name)
                settled = bounds(settled_path, "#834eb3")
                if settled == measured:
                    return settled_path, settled
            time.sleep(0.15)
        raise AssertionError(f"animation did not reach {expected} logical points: {measured}")

    tap(*center(marker))
    expanded, larger = await_width("components-animation-expanded.png", 48)
    exact_fill(expanded, larger, "#834eb3", scale)
    # Wait for the endpoint before reversing so a nearly-complete screenshot
    # cannot shorten the reverse trip through a partially finished first run.
    time.sleep(0.15)
    tap(*center(larger))
    contracted, smaller = await_width("components-animation-contracted.png", 24)
    time.sleep(0.15)
    assert not bounds(contracted, "#e3bd42"), "animation broke foreground painting"
    ticker = bounds(contracted, "#3e6c72")
    assert ticker, "ticker mode control is missing"
    tap(*center(smaller))
    time.sleep(0.35)
    tap(*center(ticker))
    time.sleep(0.3)
    paused = capture("components-animation-muted.png")
    paused_marker = bounds(paused, "#834eb3")
    assert paused_marker and 24 * scale + 2 < paused_marker[2] < 48 * scale - 2, paused_marker
    time.sleep(2.2)
    frozen = capture("components-animation-still-muted.png")
    frozen_marker = bounds(frozen, "#834eb3")
    assert frozen_marker == paused_marker, (paused_marker, frozen_marker)
    tap(*center(bounds(frozen, "#3e6c72")))
    resumed, resumed_marker = await_width("components-animation-unmuted.png", 48)
    tap(*center(resumed_marker))
    contracted, final_marker = await_width("components-animation-restored.png", 24)
    time.sleep(0.15)

    return {"animation": {"passed": True, "initial_width": marker[2], "expanded_width": larger[2], "contracted_width": smaller[2], "muted_width": paused_marker[2], "frozen_width": frozen_marker[2], "catch_up_width": resumed_marker[2], "restored_width": final_marker[2], "ticker_mode": True}, "passed": True, "inherited_step": True, "inherited_aspect": True, "notifier_burst": True, "exact_rgb888_fills": True, "scope": "injected clicks, keyed swap, removal/reinsertion, inherited step change preserving local state, then one click matching the original two-click result; A-only aspect update preserves both counts, then A remains interactive while B count stays unchanged; observable reset and three-change burst match the three-click counter; clicked frame-driven expansion and reverse reach rendered endpoints; exact trimmed foreground glyph masks at a 75% intensity threshold"}
