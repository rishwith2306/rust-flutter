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
from component_probe import probe_components

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


def wait_keyboard_hidden():
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        dump = device("shell", "dumpsys", "input_method").stdout.decode()
        if re.search(r"mInputShown=false", dump) and re.search(r"mImeWindowVis=0", dump):
            return
        time.sleep(0.3)
    raise TimeoutError("keyboard did not hide")


def wait_keyboard_shown():
    # ADB text events can arrive before the native host processes the focus tap.
    # Require the app's asynchronous IME request before injecting keyboard events.
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        dump = device("shell", "dumpsys", "input_method").stdout.decode()
        if re.search(r"mInputShown=true", dump):
            return
        time.sleep(0.3)
    screenshot("android-keyboard-focus-failed.png")
    raise TimeoutError("text field did not request the keyboard")


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
result = {"passed": True, "serial": args.serial,
          "android_release": device("shell", "getprop", "ro.build.version.release").stdout.decode().strip(),
          "sdk": device("shell", "getprop", "ro.build.version.sdk").stdout.decode().strip(),
          "abi": device("shell", "getprop", "ro.product.cpu.abi").stdout.decode().strip(),
          "page_size": device("shell", "getconf", "PAGE_SIZE").stdout.decode().strip(),
          "scale": args.scale, "top_inset_px": args.top_inset_px,
          "state_after_restart": value,
          "asset_page_opened": True,
          "layout_state_after_touch": layout_value,
          "components": components,
          "scope": "rendered touch controls, injected keyboard text, route back, background/resume, process-restart restoration, bundled assets and constraint layout interaction"}
(output / "android-result.json").write_text(json.dumps(result, indent=2) + "\n")
print("PASS: Android touch/text, routes, resume/restoration, layout and retained component state")
