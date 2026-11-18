"""Rendered component test shared by desktop and isolated Android smoke tests.
Uses injected pointer events and image patches, without component state backdoors.
Image operations require ImageMagick's convert. Coordinates are physical pixels.
"""
import re
import json
import math
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
    assert "gallery renderer dithering=false" in trace, "missing explicit gallery pixel profile"
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
    arena_taps = trace.count("gesture-arena tap accepted")
    arena_drags = trace.count("gesture-arena drag accepted")
    assert arena_taps >= 4 and arena_drags >= 1, (arena_taps, arena_drags)
    assert "gesture-arena tap rejected" in trace, "drag did not reject the tap candidate"
    binding_taps = trace.count("gesture-binding tap targets=6 pointers=1")
    shared_frames = trace.count("gesture-frame roots=3 targets=6")
    assert shared_frames >= 4, shared_frames
    binding_pans = trace.count("gesture-binding pan targets=6 pointers=1")
    assert binding_taps >= 4 and binding_pans >= 1, (binding_taps, binding_pans)
    ends = re.findall(r"pan-end vx=([\d.eE+-]+) vy=([\d.eE+-]+) raw_x=([\d.eE+-]+) raw_y=([\d.eE+-]+) confidence=([\d.eE+-]+) samples=(\d+) age_ms=([\d.eE+-]+) status=(Fitted|Unavailable|Stopped) fling=(true|false)", trace)
    assert ends, "missing rich pan-end details"
    for vx, vy, raw_x, raw_y, confidence, samples, age_ms, status, fling in ends:
        values = list(map(float, (vx, vy, raw_x, raw_y, confidence)))
        assert all(math.isfinite(v) for v in values), values
        assert 0.0 <= values[4] <= 1.0 and int(samples) <= 20, (confidence, samples)
        assert math.hypot(values[0], values[1]) <= 8000.01, values[:2]
        age = float(age_ms)
        assert math.isfinite(age) and age >= 0.0, age
        assert (age > 40.0) if status == "Stopped" else (age <= 40.0), (age, status)
        if status == "Fitted":
            assert int(samples) >= 3, samples
        if fling == "false":
            assert values[:2] == [0.0, 0.0], values[:2]
        else:
            assert status == "Fitted" and math.hypot(values[2], values[3]) > 50.0, (status, values)
    spring_samples = re.findall(r"spring width=([\d.eE+-]+) active=(true|false)", trace)
    assert spring_samples, "missing controller-driven spring samples"
    assert all(math.isfinite(float(width)) and 12.0 <= float(width) <= 24.0 for width, _ in spring_samples), spring_samples
    assert any(12.0 < float(width) < 24.0 and active == "true" for width, active in spring_samples), "missing intermediate spring motion"
    assert any(float(width) == 24.0 and active == "false" for width, active in spring_samples), "spring did not settle at expanded endpoint"
    implicit_completions = trace.count("implicit-animation end")
    assert implicit_completions >= 4, f"missing implicit completion callbacks: {implicit_completions}"
    foreground = bounds(path, "#834eb3")
    assert foreground, "missing foreground decoration"
    assert abs(foreground[2] - 24 * scale) <= 2 and abs(foreground[3] - 24 * scale) <= 2, foreground
    exact_fill(path, foreground, "#834eb3", scale)
    assert not bounds(path, "#e3bd42"), "opaque child painted above its foreground decoration"
    builder_widths = list(dict.fromkeys(float(width) for width in re.findall(r"layout-builder scroll width=([\d.eE+-]+) max_height=inf", trace)))
    assert builder_widths and all(math.isfinite(width) and width > 0 for width in builder_widths), "missing layout-time scroll child construction"
    return {"renderer_profile": {"dithering": False}, "layout_builder": {"passed": True, "scroll_widths": builder_widths}, "spring_controller": {"passed": True, "samples": len(spring_samples)}, "pan_end": {"passed": True, "details": ends}, "gesture_binding": {"passed": True, "taps": binding_taps, "pans": binding_pans, "shared_frames": shared_frames}, "arena_tap_winners": arena_taps, "arena_drag_winners": arena_drags, "implicit_completions": implicit_completions, "animation_intermediate_frames": sum(0.0 < value < 1.0 for value in animation_values), "foreground_decoration": {"passed": True, "bounds": foreground, "covered_child": True},
            "passed": True, "small_baseline": small_metric, "large_baseline": large_metric,
            "small_pixel_y": small_y, "large_pixel_y": large_y, "line_pixel_y": line_y,
            "tolerance_physical_pixels": 2.0}


def probe_components(capture, tap, scale=1.0, drag=None, wheel=None, press_key=None):
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
    def spring_bounds(path, expected=None):
        rect = bounds(path, "#c88537")
        assert rect, "missing spring-driven marker"
        if expected is not None:
            assert abs(rect[2] - expected * scale) <= 1, (rect, expected)
        x, y = center(rect)
        pixel = subprocess.check_output(["convert", str(path), "-alpha", "off", "-crop", f"1x1+{int(x)}+{int(y)}", "-depth", "8", "rgb:-"], timeout=15)
        assert pixel == bytes.fromhex("c88537"), pixel.hex()
        return rect
    initial_spring = spring_bounds(burst, 12)
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
    expanded_spring = spring_bounds(expanded, 24)
    # Wait for the endpoint before reversing so a nearly-complete screenshot
    # cannot shorten the reverse trip through a partially finished first run.
    time.sleep(0.15)
    tap(*center(larger))
    contracted, smaller = await_width("components-animation-contracted.png", 24)
    time.sleep(0.15)
    contracted_spring = spring_bounds(contracted, 12)
    assert not bounds(contracted, "#e3bd42"), "animation broke foreground painting"
    ticker = bounds(contracted, "#3e6c72")
    assert ticker, "ticker mode control is missing"
    tap(*center(smaller))
    time.sleep(0.35)
    tap(*center(ticker))
    time.sleep(0.3)
    paused = capture("components-animation-muted.png")
    paused_spring = spring_bounds(paused)
    paused_marker = bounds(paused, "#834eb3")
    assert paused_marker and 24 * scale + 2 < paused_marker[2] < 48 * scale - 2, paused_marker
    time.sleep(2.2)
    frozen = capture("components-animation-still-muted.png")
    frozen_marker = bounds(frozen, "#834eb3")
    assert frozen_marker == paused_marker, (paused_marker, frozen_marker)
    assert spring_bounds(frozen) == paused_spring, "muted spring geometry changed"
    tap(*center(bounds(frozen, "#3e6c72")))
    resumed, resumed_marker = await_width("components-animation-unmuted.png", 48)
    spring_bounds(resumed, 24)
    tap(*center(resumed_marker))
    contracted, final_marker = await_width("components-animation-restored.png", 24)
    time.sleep(0.15)
    assert drag is not None, "native arena probe requires drag injection"
    start_x, start_y = center(final_marker)
    drag(start_x, start_y, start_x + 80 * scale, start_y)
    dragged = settle("components-arena-drag.png")
    drag_marker = bounds(dragged, "#834eb3")
    assert drag_marker and abs(drag_marker[2] - 24 * scale) <= 1, "drag incorrectly triggered a tap animation"
    feedback = bounds(dragged, "#d2618d")
    assert feedback, "accepted drag did not render feedback"
    feedback_x, feedback_y = center(feedback)
    pixel = subprocess.check_output(["convert", str(dragged), "-alpha", "off", "-crop", f"1x1+{int(feedback_x)}+{int(feedback_y)}", "-depth", "8", "rgb:-"], timeout=15)
    assert pixel == bytes.fromhex("d2618d"), pixel.hex()
    ended = bounds(dragged, "#48c7bd")
    assert ended, "pan end did not render release feedback"
    end_x, end_y = center(ended)
    pixel = subprocess.check_output(["convert", str(dragged), "-alpha", "off", "-crop", f"1x1+{int(end_x)}+{int(end_y)}", "-depth", "8", "rgb:-"], timeout=15)
    assert pixel == bytes.fromhex("48c7bd"), pixel.hex()
    tap(*center(drag_marker))
    after_drag_expanded, after_drag_large = await_width("components-arena-tap-after-drag.png", 48)
    tap(*center(after_drag_large))
    after_drag_restored, after_drag_small = await_width("components-arena-restored.png", 24)



    spring_bounds(after_drag_restored, 12)
    scroll_initial = capture("components-scroll-initial.png")
    scroll_view = bounds(scroll_initial, "#284b72")
    assert scroll_view and abs(scroll_view[3] - 60 * scale) <= 1, scroll_view
    assert not bounds(scroll_initial, "#6f8e42") and not bounds(scroll_initial, "#b87657"), "scroll content escaped initial clip"
    exact_fill(scroll_initial, scroll_view, "#284b72", scale)
    x, y = center(scroll_view)
    drag(x, y, x + 80 * scale, y)
    cross_axis = settle("components-scroll-cross-axis.png")
    assert bounds(cross_axis, "#284b72") == scroll_view, "cross-axis drag moved the vertical viewport"
    assert not bounds(cross_axis, "#6f8e42") and not bounds(cross_axis, "#b87657")
    drag(x, y, x, y - 80 * scale)
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        scrolled = capture("components-scroll-dragged.png")
        green = bounds(scrolled, "#6f8e42")
        coral = bounds(scrolled, "#b87657")
        if not bounds(scrolled, "#284b72") and (green or coral):
            break
        time.sleep(0.1)
    else:
        raise AssertionError("captured pan did not scroll the viewport")
    for rect in (green, coral):
        if rect:
            assert rect[0] == scroll_view[0] and rect[2] == scroll_view[2], (rect, scroll_view)
            assert rect[1] >= scroll_view[1] - 1 and rect[1] + rect[3] <= scroll_view[1] + scroll_view[3] + 1, "scrolled content escaped hard clip"
    solid_rect, solid_color = max([(r, c) for r, c in ((green, "#6f8e42"), (coral, "#b87657")) if r], key=lambda item: item[0][3])
    exact_fill(scrolled, solid_rect, solid_color, scale)
    reset_scroll = bounds(scrolled, "#49614c")
    assert reset_scroll, "missing controller reset button"
    tap(*center(reset_scroll))
    deadline = time.monotonic() + 8
    while time.monotonic() < deadline:
        reset_scroll_image = capture("components-scroll-reset.png")
        if bounds(reset_scroll_image, "#284b72") == scroll_view:
            break
        time.sleep(0.1)
    else:
        raise AssertionError("controller jump did not reset scroll pixels")
    assert not bounds(reset_scroll_image, "#6f8e42") and not bounds(reset_scroll_image, "#b87657")
    exact_fill(reset_scroll_image, scroll_view, "#284b72", scale)
    wheel_verified = False
    if wheel is not None:
        wheel(*center(scroll_view), 4)
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            wheel_image = capture("components-scroll-wheel.png")
            if not bounds(wheel_image, "#284b72") and (bounds(wheel_image, "#6f8e42") or bounds(wheel_image, "#b87657")):
                break
            time.sleep(0.1)
        else:
            raise AssertionError("native wheel did not scroll the retained viewport")
        tap(*center(bounds(wheel_image, "#49614c")))
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            wheel_reset = capture("components-scroll-wheel-reset.png")
            if bounds(wheel_reset, "#284b72") == scroll_view:
                break
            time.sleep(0.1)
        else:
            raise AssertionError("controller reset failed after native wheel input")
        exact_fill(wheel_reset, scroll_view, "#284b72", scale)
        wheel_verified = True
    keyboard_verified = False
    if press_key is not None:
        def await_keyboard(name, expected_blue=False, expected_end=False):
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline:
                path = capture(name)
                blue = bounds(path, "#284b72")
                coral = bounds(path, "#b87657")
                valid = blue == scroll_view if expected_blue else (not blue and bool(coral))
                if expected_end:
                    valid = valid and coral == scroll_view
                if valid:
                    return path
                time.sleep(0.1)
            raise AssertionError(f"keyboard scroll did not reach expected pixels: {name}")
        tap(*center(scroll_view))
        press_key("End")
        await_keyboard("components-scroll-key-end.png", expected_end=True)
        press_key("Home")
        await_keyboard("components-scroll-key-home.png", expected_blue=True)
        press_key("PageDown")
        press_key("PageDown")
        await_keyboard("components-scroll-key-page.png")
        press_key("Home")
        await_keyboard("components-scroll-key-page-reset.png", expected_blue=True)
        press_key("ArrowDown")
        press_key("ArrowDown")
        await_keyboard("components-scroll-key-arrows.png")
        press_key("Home")
        final_keys = await_keyboard("components-scroll-key-reset.png", expected_blue=True)
        exact_fill(final_keys, scroll_view, "#284b72", scale)
        keyboard_verified = True
    def await_bar(name, predicate):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            path = capture(name)
            if predicate(path):
                return path
            time.sleep(0.1)
        raise AssertionError(f"scrollbar did not reach expected rendered state: {name}")
    bar_initial = capture("components-scrollbar-initial.png")
    thumb = bounds(bar_initial, "#b9c7d8")
    track = bounds(bar_initial, "#4e5866")
    assert thumb and track, "missing scrollbar thumb/track"
    assert abs(thumb[2] - 8 * scale) <= 2 and abs(thumb[3] - (56 / 3) * scale) <= 2, thumb
    for rect in (thumb, track):
        assert rect[0] >= scroll_view[0] and rect[1] >= scroll_view[1]
        assert rect[0] + rect[2] <= scroll_view[0] + scroll_view[2] + 1
        assert rect[1] + rect[3] <= scroll_view[1] + scroll_view[3] + 1
    tx, ty = center(thumb)
    drag(tx, ty, tx, ty + 40 * scale)
    bar_dragged = await_bar("components-scrollbar-dragged.png", lambda path: bounds(path, "#b87657") == scroll_view)
    end_thumb = bounds(bar_dragged, "#b9c7d8")
    assert end_thumb and abs((end_thumb[1] - thumb[1]) - (56 * 2 / 3) * scale) <= 2, (thumb, end_thumb)
    tap(*center(reset_scroll))
    await_bar("components-scrollbar-drag-reset.png", lambda path: bounds(path, "#284b72") == scroll_view and bounds(path, "#b9c7d8") == thumb)
    tap(tx, scroll_view[1] + 50 * scale)
    def page_matches(path):
        green = bounds(path, "#6f8e42")
        blue = bounds(path, "#284b72")
        return green and blue and abs(green[3] - 48 * scale) <= 2 and abs(blue[3] - 12 * scale) <= 2
    await_bar("components-scrollbar-page.png", page_matches)
    tap(*center(reset_scroll))
    await_bar("components-scrollbar-page-reset.png", lambda path: bounds(path, "#284b72") == scroll_view and bounds(path, "#b9c7d8") == thumb)
    bar_wheel_verified = False
    if wheel is not None:
        wheel(*center(thumb), 4)
        await_bar("components-scrollbar-wheel.png", lambda path: not bounds(path, "#284b72") and (bounds(path, "#6f8e42") or bounds(path, "#b87657")))
        tap(*center(reset_scroll))
        bar_wheel_verified = True
    bar_reset = await_bar("components-scrollbar-reset.png", lambda path: bounds(path, "#284b72") == scroll_view and bounds(path, "#b9c7d8") == thumb)
    exact_fill(bar_reset, scroll_view, "#284b72", scale)
    return {"scrollbar": {"passed": True, "thumb_drag": True, "track_page": True, "wheel_over_thumb": bar_wheel_verified, "initial_thumb": thumb}, "scroll_viewport": {"passed": True, "height": scroll_view[3], "drag_scroll": True, "cross_axis_no_scroll": True, "controller_reset": True, "hard_clip": True, "native_wheel": wheel_verified, "native_keyboard": keyboard_verified}, "spring_controller": {"passed": True, "initial_width": initial_spring[2], "expanded_width": expanded_spring[2], "contracted_width": contracted_spring[2], "muted_width": paused_spring[2]}, "gesture_arena": {"passed": True, "drag_feedback": True, "release_feedback": True, "tap_rejected_on_drag": True, "tap_after_drag": True}, "animation": {"passed": True, "initial_width": marker[2], "expanded_width": larger[2], "contracted_width": smaller[2], "muted_width": paused_marker[2], "frozen_width": frozen_marker[2], "catch_up_width": resumed_marker[2], "restored_width": final_marker[2], "ticker_mode": True}, "passed": True, "inherited_step": True, "inherited_aspect": True, "notifier_burst": True, "exact_rgb888_fills": True, "scope": "injected clicks, keyed swap, removal/reinsertion, inherited step change preserving local state, then one click matching the original two-click result; A-only aspect update preserves both counts, then A remains interactive while B count stays unchanged; observable reset and three-change burst match the three-click counter; clicked frame-driven expansion and reverse reach rendered endpoints; exact trimmed foreground glyph masks at a 75% intensity threshold"}


def probe_lazy_list(capture, tap, drag, press_key, scale=1.0, wheel=None):
    """Drive the standalone 10,002-row gallery list through OS input/pixels."""
    initial = capture("lazy-list-initial.png")
    first = bounds(initial, "#17695b")
    reset = bounds(initial, "#4d6552")
    assert first and reset and abs(first[3] - 60 * scale) <= 2, (first, reset)
    assert not bounds(initial, "#c99843"), "last item painted at initial offset"
    exact_fill(initial, first, "#17695b", scale)

    def await_state(name, predicate):
        deadline = time.monotonic() + 8
        while time.monotonic() < deadline:
            path = capture(name)
            if predicate(path):
                return path
            time.sleep(0.1)
        raise AssertionError(f"lazy list did not reach expected pixels: {name}")

    x, y = center(first)
    drag(x, y, x, y - 80 * scale)
    dragged = await_state("lazy-list-dragged.png", lambda path: not bounds(path, "#17695b") and any(bounds(path, color) for color in ["#2e515c", "#644d6f", "#9e5f39", "#436c51"]))
    for color in ["#2e515c", "#644d6f", "#9e5f39", "#436c51"]:
        rect = bounds(dragged, color)
        if rect:
            assert rect[0] == first[0] and rect[2] == first[2], (rect, first)
            assert rect[1] >= first[1] - 1 and rect[1] + rect[3] <= first[1] + 240 * scale + 1, "lazy row escaped viewport"
    tap(*center(reset))
    await_state("lazy-list-drag-reset.png", lambda path: bounds(path, "#17695b") == first)
    tap(first[0] + first[2] / 2, first[1] + 90 * scale)
    press_key("End")
    end = await_state("lazy-list-end.png", lambda path: not bounds(path, "#17695b") and bool(bounds(path, "#c99843")))
    last = bounds(end, "#c99843")
    assert last and abs(last[1] - first[1] - 180 * scale) <= 2 and abs(last[3] - 60 * scale) <= 2, (first, last)
    exact_fill(end, last, "#c99843", scale)
    press_key("Home")
    await_state("lazy-list-home.png", lambda path: bounds(path, "#17695b") == first and not bounds(path, "#c99843"))
    wheel_verified = False
    if wheel is not None:
        wheel(*center(first), 4)
        await_state("lazy-list-wheel.png", lambda path: not bounds(path, "#17695b") and any(bounds(path, color) for color in ["#2e515c", "#644d6f", "#9e5f39", "#436c51"]))
        tap(*center(reset))
        wheel_verified = True
    final = await_state("lazy-list-reset.png", lambda path: bounds(path, "#17695b") == first and not bounds(path, "#c99843"))
    exact_fill(final, first, "#17695b", scale)
    # F8 preserves viewport focus, unlike clicking the toggle button.
    palette = ["#17695b", "#c99843", "#2e515c", "#644d6f", "#9e5f39", "#436c51", "#b9c7d8", "#4e5866"]
    def hidden(path):
        return not any(bounds(path, color) for color in palette)
    tap(first[0] + first[2] / 2, first[1] + 90 * scale)
    press_key("F8")
    await_state("lazy-list-offstage.png", hidden)
    if wheel is not None:
        wheel(first[0] + first[2] / 2, first[1] + 120 * scale, 4)
    press_key("F8")
    await_state("lazy-list-offstage-restored.png", lambda path: bounds(path, "#17695b") == first)
    press_key("F8")
    await_state("lazy-list-offstage-keyboard.png", hidden)
    press_key("End")
    await_state("lazy-list-offstage-end-hidden.png", hidden)
    press_key("F8")
    restored = await_state("lazy-list-offstage-end-restored.png", lambda path: bounds(path, "#c99843") == last)
    exact_fill(restored, last, "#c99843", scale)
    press_key("Home")
    restored = await_state("lazy-list-offstage-home.png", lambda path: bounds(path, "#17695b") == first)
    exact_fill(restored, first, "#17695b", scale)
    before = patch(restored, first, scale)
    tap(*center(first))
    clicked = await_state("lazy-list-kept-count.png", lambda path: patch(path, first, scale) != before)
    counted = patch(clicked, first, scale)
    # Focus the viewport through an ordinary row, preserving the row-owned count.
    tap(first[0] + first[2] / 2, first[1] + 90 * scale)
    press_key("End")
    await_state("lazy-list-kept-away.png", lambda path: bounds(path, "#c99843") == last and not bounds(path, "#17695b"))
    press_key("Home")
    returned = await_state("lazy-list-kept-returned.png", lambda path: bounds(path, "#17695b") == first and patch(path, first, scale) == counted)
    exact_fill(returned, first, "#17695b", scale)
    return {"passed": True, "items": 10002, "viewport_points": 240, "drag": True, "end_home": True, "native_wheel": wheel_verified, "exact_reset": True, "offstage": {"passed": True, "hidden_wheel_ignored": wheel_verified, "hidden_keyboard": True, "exact_restore": True}, "keep_alive": {"passed": True, "local_counter_retained": True}}


def probe_lazy_builds(trace):
    batches = [(int(count), int(first), int(last), int(total), json.loads(indices)) for count, first, last, total, indices in re.findall(r"lazy-list built=(\d+) first=(\d+) last=(\d+) total=(\d+) indices=(\[[^\]\n]*\])", trace)]
    assert batches, "missing lazy list build diagnostics"
    kept_batches = 0
    for count, first, last, total, indices in batches:
        assert total == 10002 and count == len(indices) and len(set(indices)) == count and all(0 <= i < total for i in indices), batches
        ordered = sorted(indices)
        assert first == ordered[0] and last == ordered[-1], batches
        if ordered == list(range(first, last + 1)):
            assert count <= 9, batches
        else:
            # Only row 0 is kept alive in the gallery. A configuration update
            # may rebuild it plus the ordinary contiguous viewport/cache range.
            assert ordered[0] == 0 and ordered[1:] == list(range(ordered[1], last + 1)) and count <= 10, batches
            kept_batches += 1
    assert any(first == 0 for _, first, _, _, _ in batches) and any(last == 10001 for _, _, last, _, _ in batches), "lazy list never built both data endpoints"
    return {"passed": True, "batches": len(batches), "max_built_per_pass": max(row[0] for row in batches), "last_index": max(row[2] for row in batches), "total_items": 10002, "kept_item_build_batches": kept_batches}
