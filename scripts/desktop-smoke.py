#!/usr/bin/env python3
"""Exercise a real gallery window on an isolated Xvfb display.
Requires Xvfb, xdotool and ImageMagick. App data stays inside a temporary directory.
"""
import json
import os
import re
from pathlib import Path
import select
import subprocess
import tempfile
import time
from component_probe import probe_baselines, probe_components, probe_lazy_list, probe_lazy_builds, bounds

root = Path(__file__).resolve().parents[1]
output = root / "target" / "smoke"
output.mkdir(parents=True, exist_ok=True)
(output / "desktop-result.json").unlink(missing_ok=True)
xvfb_log = (output / "xvfb.log").open("w")
read_fd, write_fd = os.pipe()
server = subprocess.Popen(
    ["Xvfb", "-noreset", "-displayfd", str(write_fd), "-screen", "0", "1024x768x24"],
    pass_fds=(write_fd,), stdout=xvfb_log, stderr=xvfb_log,
)
os.close(write_fd)
app = None


def wait_for(predicate, description, timeout=20):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if server.poll() is not None:
            raise RuntimeError(f"Xvfb exited ({server.returncode}); see {output / 'xvfb.log'}")
        if app is not None and app.poll() is not None:
            raise RuntimeError(f"gallery exited ({app.returncode}); see {output / 'gallery.log'}")
        value = predicate()
        if value:
            return value
        time.sleep(0.1)
    raise TimeoutError(description)


try:
    if not select.select([read_fd], [], [], 10)[0]:
        raise RuntimeError("Xvfb failed to allocate a display")
    with os.fdopen(read_fd) as pipe:
        display = pipe.readline().strip()
    if not display.isdecimal() or server.poll() is not None:
        raise RuntimeError(f"Xvfb did not provide a live display number; see {output / 'xvfb.log'}")
    with tempfile.TemporaryDirectory(prefix="rust-flutter-smoke-") as runtime:
        env = {**os.environ, "DISPLAY": f":{display}", "XDG_RUNTIME_DIR": runtime,
               "XDG_DATA_HOME": str(Path(runtime) / "data")}
        env.pop("WAYLAND_DISPLAY", None)
        state_path = Path(env["XDG_DATA_HOME"]) / "rust-flutter-gallery/gallery.json"

        def xdo(*args):
            return subprocess.run(["xdotool", *args], env=env, capture_output=True, text=True, timeout=5)

        def find_window():
            result = xdo("search", "--onlyvisible", "--name", "Rust Flutter Gallery")
            return result.stdout.splitlines()[0] if result.returncode == 0 and result.stdout else None

        def state_matches(**values):
            try:
                data = json.loads(state_path.read_text())["data"]
                return data if all(data.get(k) == v for k, v in values.items()) else None
            except (FileNotFoundError, json.JSONDecodeError):
                return None

        def click(window, x, y):
            assert xdo("mousemove", "--window", window, str(x), str(y)).returncode == 0
            assert xdo("click", "1").returncode == 0

        with (output / "gallery.log").open("w") as log:
            def launch():
                global app
                app = subprocess.Popen([str(root / "target/debug/rust-flutter-gallery")], cwd=runtime, env=env, stdout=log, stderr=log)
                window = wait_for(find_window, "gallery window did not appear")
                assert xdo("windowfocus", "--sync", window).returncode == 0
                return window

            window = launch()
            wait_for(lambda: state_matches(count=0, name=""), "initial snapshot not written")
            click(window, 270, 187)
            wait_for(lambda: state_matches(count=1), "counter button did not update state")
            click(window, 80, 140)
            assert xdo("type", "--clearmodifiers", "Ada").returncode == 0
            wait_for(lambda: state_matches(count=1, name="Ada"), "text input did not update state")
            time.sleep(0.2)
            subprocess.run(["import", "-window", window, str(output / "gallery.png")], env=env, timeout=10, check=True)
            app.terminate()
            app.wait(timeout=10)
            app = None
            # A fresh process must load state, not just leave an old file in place.
            window = launch()
            time.sleep(0.5)
            # The greeting adds one text row after restoring the name.
            click(window, 270, 205)
            data = wait_for(lambda: state_matches(count=2, name="Ada"), "restored process did not use saved counter and name")
            subprocess.run(["import", "-window", window, str(output / "gallery-restored.png")], env=env, timeout=10, check=True)
            click(window, 80, 299)
            wait_for(lambda: state_matches(route="assets"), "bundled assets page did not open")
            time.sleep(0.5)
            asset_shot = output / "gallery-assets.png"
            subprocess.run(["import", "-window", window, str(asset_shot)], env=env, timeout=10, check=True)
            histogram = subprocess.check_output(["convert", str(asset_shot), "-depth", "8", "-format", "%c", "histogram:info:-"], text=True, timeout=10)
            import re
            badge_pixels = sum(int(count) for count, r, g, b in re.findall(r"^\s*(\d+):\s*\((\d+),(\d+),(\d+)", histogram, re.MULTILINE) if abs(int(r)-23) <= 3 and abs(int(g)-105) <= 3 and abs(int(b)-170) <= 3)
            assert badge_pixels > 100, f"bundled image was not visibly rendered: {badge_pixels} matching pixels"
            click(window, 80, 207)
            wait_for(lambda: state_matches(route="home"), "assets back button did not return home")
            click(window, 80, 346)
            wait_for(lambda: state_matches(route="layout"), "constraint layout page did not open")
            click(window, 50, 180)
            wait_for(lambda: state_matches(count=3), "button in constrained leaf did not respond")
            time.sleep(0.3)
            subprocess.run(["import", "-window", window, str(output / "gallery-layout.png")], env=env, timeout=10, check=True)
            assert xdo("windowsize", window, "1000", "600").returncode == 0
            time.sleep(0.5)
            click(window, 50, 180)
            layout_state = wait_for(lambda: state_matches(count=4), "constrained leaf lost interaction after resize")
            time.sleep(0.3)
            subprocess.run(["import", "-window", window, str(output / "gallery-layout-wide.png")], env=env, timeout=10, check=True)
            # Sample a row above the overlay/text to verify the rendered flex ratio.
            histogram = subprocess.check_output(["convert", str(output / "gallery-layout-wide.png"), "-crop", "1000x1+0+150", "-depth", "8", "-format", "%c", "histogram:info:-"], text=True, timeout=10)
            colors = re.findall(r"^\s*(\d+):\s*\((\d+),(\d+),(\d+)", histogram, re.MULTILINE)
            def colored_width(target):
                return sum(int(n) for n, r, g, b in colors if all(abs(int(actual)-expected) <= 3 for actual, expected in zip((r,g,b), target)))
            widths = [colored_width((23,105,170)), colored_width((25,125,85))]
            assert widths[0] > 100 and abs(widths[1] / widths[0] - 2) < 0.03, widths
            click(window, 80, 325)
            wait_for(lambda: state_matches(route="home"), "layout back did not return home")
            click(window, 80, 393)
            wait_for(lambda: state_matches(route="components"), "stateful component page did not open")
            time.sleep(0.5)
            def component_capture(name):
                path = output / name
                subprocess.run(["import", "-window", window, str(path)], env=env, timeout=10, check=True)
                return path
            def component_drag(x, y, end_x, end_y):
                assert xdo("mousemove", "--window", window, str(round(x)), str(round(y))).returncode == 0
                assert xdo("mousedown", "1").returncode == 0
                time.sleep(0.1)
                for step in range(1, 5):
                    assert xdo("mousemove", "--sync", "--window", window, str(round(x+(end_x-x)*step/4)), str(round(y+(end_y-y)*step/4))).returncode == 0
                    time.sleep(0.05)
                assert xdo("mouseup", "1").returncode == 0
            def component_wheel(x, y, steps):
                assert xdo("mousemove", "--window", window, str(round(x)), str(round(y))).returncode == 0
                assert xdo("click", "--repeat", str(abs(steps)), "--delay", "120", "5" if steps > 0 else "4").returncode == 0
            def component_key(key):
                name = {"End": "End", "Home": "Home", "PageDown": "Next", "ArrowDown": "Down", "F8": "F8"}[key]
                assert xdo("key", "--clearmodifiers", name).returncode == 0
            components = probe_components(component_capture, lambda x,y: click(window, round(x), round(y)), drag=component_drag, wheel=component_wheel, press_key=component_key)
            def view_report():
                matches = re.findall(r"inherited-view build=(\d+) size=([\d.]+)x([\d.]+) keyboard=(\w+)", (output / "gallery.log").read_text())
                return matches[-1] if matches else None
            before = wait_for(view_report, "retained view report did not build")
            assert before == ("1", "1000", "600", "Unknown"), before
            assert xdo("windowsize", window, "900", "650").returncode == 0
            after = wait_for(lambda: (v if (v := view_report()) and v[1:3] == ("900", "650") else None), "ambient view size did not update after native resize")
            assert int(after[0]) == 2, after
            baseline_image = component_capture("desktop-inherited-view-resized.png")
            view_metrics = {"before": before, "after": after}
            trace=(output / "gallery.log").read_text()
            baselines = probe_baselines(baseline_image, trace)
            render_layouts=re.findall(r"render-view layout=(\d+) size=([\d.]+)x([\d.]+)",trace)
            assert baselines["layout_builder"]["scroll_widths"] == [984.0, 884.0], baselines["layout_builder"]
            assert trace.count("render-view create") == 1, trace
            assert trace.count("render-view update") == 2, trace
            assert len(render_layouts) == 2 and [v[0] for v in render_layouts] == ["1", "2"], render_layouts
            render_queries=re.findall(r"render-view query dry=([\d.]+)x([\d.]+) actual=([\d.]+)x([\d.]+) matched=(true|false)",trace)
            assert len(render_queries)==2 and all(q[-1]=="true" for q in render_queries), render_queries
            render_objects={"created":1,"updated":2,"layouts":render_layouts,"dry_queries":render_queries}
            assert xdo("windowsize", window, "430", "740").returncode == 0
            wait_for(lambda: (v if (v := view_report()) and v[1:3] == ("430", "740") else None), "compact view did not resize")
            compact = component_capture("desktop-components-compact.png")
            ticker_rect = bounds(compact, "#3e6c72")
            assert ticker_rect and abs(ticker_rect[2] - 80) <= 2 and abs(ticker_rect[3] - 24) <= 2, ticker_rect
            probe_baselines(compact, (output / "gallery.log").read_text())
            components["compact_layout"] = {"passed": True, "window_width": 430, "ticker_size": list(ticker_rect[2:])}
            assert xdo("windowsize", window, "900", "650").returncode == 0
            wait_for(lambda: (v if (v := view_report()) and v[1:3] == ("900", "650") else None), "wide view did not restore")
            assert xdo("key", "XF86Back").returncode == 0
            wait_for(lambda: state_matches(route="home"), "components back request did not return home")
            home = component_capture("lazy-list-home-route.png")
            from component_probe import bounds, center
            lazy_button = bounds(home, "#42658b")
            assert lazy_button, "missing lazy list route button"
            click(window, *map(round, center(lazy_button)))
            wait_for(lambda: state_matches(route="lazy_list"), "lazy list page did not open")
            time.sleep(0.5)
            lazy_list = probe_lazy_list(component_capture, lambda x,y: click(window, round(x), round(y)), component_drag, component_key, wheel=component_wheel)
            trace = (output / "gallery.log").read_text()
            lazy_list["builds"] = probe_lazy_builds(trace)
            (output / "desktop-result.json").write_text(json.dumps({"passed": True, "state_after_restart": data, "bundled_asset_pixels": badge_pixels, "layout_after_resize": layout_state, "layout_flex_pixel_widths": widths, "components": components, "lazy_list": lazy_list, "inherited_view_metrics": view_metrics, "render_objects": render_objects, "baseline_alignment": baselines, "platform": "Linux X11/Xvfb", "scope": "GPU rendering, pointer, keyboard text, private state persistence, restart, assets, constraint layout and retained components"}, indent=2))
            print("PASS: gallery input/restoration/assets, resized layout, keyed component state and selective inherited view updates")
finally:
    if app is not None and app.poll() is None:
        app.terminate()
        app.wait(timeout=10)
    if server.poll() is None:
        server.terminate()
    server.wait(timeout=10)
    xvfb_log.close()
