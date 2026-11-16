#!/usr/bin/env python3
"""Test the gallery on an explicitly selected isolated emulator.
Resets only dev.rustflutter.gallery's test data. Requires a preinstalled gallery APK.
Coordinates use the gallery's logical layout and the supplied emulator density/inset.
Requires ImageMagick convert for rendered component text comparisons.
This is a profile-specific smoke test, not a general widget/device test harness.
"""
import argparse
import json
import os
import re
from pathlib import Path
import shutil
import subprocess
import time
from component_probe import probe_components, native_formats
from android_ime import ime_state

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--serial", required=True)
parser.add_argument("--scale", type=float, required=True)
parser.add_argument("--top-inset-px", type=int, required=True)
parser.add_argument("--reset-gallery-data", action="store_true", required=True)
args = parser.parse_args()
if not args.serial.startswith("emulator-"):
    parser.error("use an isolated emulator; this test resets gallery data")
if args.scale <= 0 or args.top_inset_px < 0:
    parser.error("density must be positive and the inset nonnegative")
sdk = os.environ.get("ANDROID_HOME")
adb = str(Path(sdk) / "platform-tools/adb") if sdk else shutil.which("adb")
if not adb:
    parser.error("set ANDROID_HOME or put adb on PATH")
root = Path(__file__).resolve().parents[1]
output = root / "target/smoke"
output.mkdir(parents=True, exist_ok=True)
package = "dev.rustflutter.gallery"
surface_formats = []
keyboard_states = []
keyboard_metrics = {}


def device(*command, check=True, timeout=15):
    result = subprocess.run([adb, "-s", args.serial, *command], capture_output=True, timeout=timeout)
    if check and result.returncode:
        raise RuntimeError(f"adb {command} failed: {result.stdout.decode(errors='replace')} {result.stderr.decode(errors='replace')}")
    return result


def state():
    result = device("shell", "run-as", package, "cat", "files/gallery.json", check=False)
    if result.returncode:
        return None
    return json.loads(result.stdout)["data"]


def wait_state(**expected):
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        value = state()
        if value and all(value.get(k) == v for k, v in expected.items()):
            return value
        time.sleep(0.2)
    raise AssertionError(f"expected {expected}; last state: {state()}")


def tap(x, y):
    device("shell", "input", "tap", str(round(x * args.scale)), str(round(y * args.scale + args.top_inset_px)))


def wait_keyboard(shown):
    phase = "shown" if shown else "hidden"
    deadline = time.monotonic() + 30
    dump = ""
    while time.monotonic() < deadline:
        dump = device("shell", "dumpsys", "input_method").stdout.decode()
        windows = device("shell", "dumpsys", "window").stdout.decode()
        observed = ime_state(dump, windows)
        if observed is not None:
            status = {"phase": phase, **observed}
            if not keyboard_states or keyboard_states[-1] != status:
                keyboard_states.append(status)
            if status["requested"] == shown and status["visible"] == shown:
                (output / f"android-keyboard-{phase}.txt").write_text(dump)
                (output / f"android-keyboard-{phase}-window.txt").write_text(windows)
                screenshot(f"android-keyboard-{phase}.png")
                wait_keyboard_metrics(shown, windows)
                return
        time.sleep(0.3)
    (output / f"android-keyboard-{phase}-failed.txt").write_text(dump)
    (output / "android-keyboard-window.txt").write_bytes(device("shell", "dumpsys", "window").stdout)
    (output / "android-keyboard-logcat.txt").write_bytes(device("logcat", "-d", "-t", "1500").stdout)
    screenshot(f"android-keyboard-{phase}-failed.png")
    raise TimeoutError(f"keyboard did not become {phase}; observations: {keyboard_states}")


def wait_keyboard_metrics(shown, windows):
    """Check the app-facing API, separately from the OS readiness observations."""
    deadline = time.monotonic() + 15
    expected = "Visible" if shown else "Hidden"
    trace = ""
    frames = re.findall(r"\bmSource=InsetsSource\b[^\n]*\btype=ime\b[^\n]*\bframe=\[(-?\d+),(-?\d+)\]\[(-?\d+),(-?\d+)\]", windows)
    assert len(frames) == 1, frames
    left, top, right, bottom = map(int, frames[0])
    expected_occlusion = bottom - top if shown else 0
    while time.monotonic() < deadline:
        pid = device("shell", "pidof", package).stdout.decode().strip()
        trace = device("logcat", "-d", f"--pid={pid}", "-s", "RustStdoutStderr:I").stdout.decode(errors="replace")
        matches = re.findall(r"keyboard=(Unknown|Visible|Hidden) view_padding_bottom_px=([\d.-]+) view_insets_bottom_px=([\d.-]+) padding_bottom_px=([\d.-]+)", trace)
        if matches:
            visibility, stable, obscured, padding = matches[-1]
            stable, obscured, padding = map(float, (stable, obscured, padding))
            if visibility == expected and stable >= 0 and abs(obscured - expected_occlusion) < 1:
                if shown:
                    assert obscured > stable, (stable, obscured)
                else:
                    assert obscured == 0, obscured
                assert abs(padding - max(stable - obscured, 0)) < 0.01
                keyboard_metrics[expected] = {"view_padding_bottom_px": stable, "view_insets_bottom_px": obscured, "padding_bottom_px": padding, "os_ime_frame_height_px": expected_occlusion}
                (output / f"android-keyboard-metrics-{expected}.txt").write_text(trace)
                if len(keyboard_metrics) == 2:
                    assert keyboard_metrics["Visible"]["view_padding_bottom_px"] == keyboard_metrics["Hidden"]["view_padding_bottom_px"]
                return
        time.sleep(0.3)
    (output / "android-keyboard-metrics-failed.txt").write_text(trace)
    raise TimeoutError(f"app-facing keyboard metrics did not become {expected}")


def wait_keyboard_hidden():
    wait_keyboard(False)


def wait_keyboard_shown():
    # A request can precede the IME window. Wait for visibility before typing/Back.
    wait_keyboard(True)


def start():
    result = device("shell", "am", "start", "-W", "-n", f"{package}/android.app.NativeActivity", timeout=30)
    if b"Status: ok" not in result.stdout:
        raise AssertionError(result.stdout.decode(errors="replace"))
    # App snapshots are written during UI construction, before GPU presentation.
    # Wait for the debug host's successful present at the expected inset/density.
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        pid = device("shell", "pidof", package, check=False).stdout.decode().strip()
        if pid:
            trace = device("logcat", "-d", f"--pid={pid}", "-s", "RustStdoutStderr:I").stdout.decode(errors="replace")
            geometry = re.findall(r"presented \d+x\d+ scale=([\d.]+) safe_top_px=([\d.]+)", trace)
            if geometry and abs(float(geometry[-1][0]) - args.scale) < 0.01 and abs(float(geometry[-1][1]) - args.top_inset_px) < 1:
                surface_formats.extend(native_formats(trace))
                return
        time.sleep(0.2)
    raise TimeoutError("debug gallery did not present the expected density/insets")


def screenshot(name):
    (output / name).write_bytes(device("exec-out", "screencap", "-p").stdout)
    return output / name


device("shell", "am", "force-stop", package)
result = device("shell", "pm", "clear", package)
assert b"Success" in result.stdout
device("shell", "svc", "power", "stayon", "true")
start()
wait_state(count=0, name="", route="home")
# Insets are queried asynchronously on the Java UI thread.
time.sleep(2)
tap(270, 187)
wait_state(count=1)
tap(80, 140)
wait_keyboard_shown()
device("shell", "input", "text", "Ada")
wait_state(count=1, name="Ada")
# Dismiss the asynchronous IME, then remove the text editor's focus.
device("shell", "input", "keyevent", "4")
wait_keyboard_hidden()
tap(20, 20)
screenshot("android-input.png")
tap(80, 252)  # A greeting adds one logical text row to the home card.
wait_state(route="settings")
screenshot("android-settings.png")
device("shell", "input", "keyevent", "4")
wait_state(route="home")
tap(270, 205)
wait_state(count=2, name="Ada")
# Verify surface recreation and app state after background/resume.
device("shell", "input", "keyevent", "3")
start()
time.sleep(1)
tap(270, 205)
wait_state(count=3, name="Ada")
# A new process must load saved state, then increment from the restored value.
device("shell", "am", "force-stop", package)
start()
time.sleep(2)
tap(270, 205)
value = wait_state(count=4, name="Ada")
screenshot("android-restored.png")
tap(80, 299)
wait_state(route="assets")
time.sleep(1)
screenshot("android-assets.png")
# Restore home so the separate recreation instrumentation can run afterwards.
device("shell", "input", "keyevent", "4")
wait_state(route="home")
tap(80, 346)
wait_state(route="layout")
# The navigation model is saved by the old page before the next frame builds
# the new page's hit regions. Give the emulator a presentation interval.
time.sleep(1)
tap(50, 200)
layout_value = wait_state(count=5, route="layout")
time.sleep(1)
screenshot("android-layout.png")
device("shell", "input", "keyevent", "4")
wait_state(route="home")
tap(80, 393)
wait_state(route="components")
time.sleep(1)
components = probe_components(lambda name: screenshot("android-" + name),
    lambda x,y: device("shell", "input", "tap", str(round(x)), str(round(y))), args.scale)
device("shell", "input", "keyevent", "4")
wait_state(route="home")
pid = device("shell", "pidof", package).stdout.decode().strip()
render_trace = device("logcat", "-d", f"--pid={pid}", "-s", "RustStdoutStderr:I").stdout.decode(errors="replace")
render_layouts = re.findall(r"render-view layout=(\d+) size=([\d.]+)x([\d.]+)", render_trace)
assert render_trace.count("render-view create") == 1, render_trace
assert render_trace.count("render-view update") == 1, render_trace
assert len(render_layouts) == 1 and render_layouts[0][0] == "1", render_layouts
(output / "android-render-object.txt").write_text(render_trace)
render_objects = {"created": 1, "updated": 1, "layouts": render_layouts}
result = {"passed": True, "serial": args.serial,
          "android_release": device("shell", "getprop", "ro.build.version.release").stdout.decode().strip(),
          "sdk": device("shell", "getprop", "ro.build.version.sdk").stdout.decode().strip(),
          "abi": device("shell", "getprop", "ro.product.cpu.abi").stdout.decode().strip(),
          "native_surface_formats": surface_formats,
          "keyboard_states": keyboard_states,
          "keyboard_metrics": keyboard_metrics,
          "page_size": device("shell", "getconf", "PAGE_SIZE").stdout.decode().strip(),
          "scale": args.scale, "top_inset_px": args.top_inset_px,
          "state_after_restart": value,
          "asset_page_opened": True,
          "layout_state_after_touch": layout_value,
          "components": components,
          "render_objects": render_objects,
          "scope": "rendered touch controls, injected keyboard text, route back, background/resume, process-restart restoration, bundled assets and constraint layout interaction"}
(output / "android-result.json").write_text(json.dumps(result, indent=2) + "\n")
print("PASS: Android touch/text, keyboard metrics, routes, resume/restoration, layout and retained component state")
