#!/usr/bin/env python3
"""Build and run a separate Android instrumentation APK against the debug gallery.
Calls Activity.recreate() repeatedly, requires fresh Rust hosts in the same PID,
and injects real touch events after each recreation. The test-only gallery variant
keeps the exact native bytes but adds hasCode=true and an unused marker DEX because
Android refuses to instrument pure-native APKs. Only the isolated
RustFlutterSmoke AVD is accepted; gallery data is preserved except counter clicks.
No emulator-wide developer settings or application test hooks are required.
"""
import argparse
import hashlib
import shutil
import xml.etree.ElementTree as ET
import json
import os
from pathlib import Path
import re
import subprocess
import zipfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--serial", required=True)
parser.add_argument("--scale", type=float, default=0.0, help="logical-pixel scale; default reads Android display metrics")
parser.add_argument("--top-inset-px", type=int, default=-1, help="top safe inset; default reads WindowInsets")
parser.add_argument("--cycles", type=int, default=3)
parser.add_argument("--apk", type=Path, help="cargo-apk gallery APK (default: target/debug/apk/rust-flutter-gallery.apk)")
args = parser.parse_args()
if not args.serial.startswith("emulator-") or args.scale < 0 or args.top_inset_px < -1 or args.cycles < 1:
    parser.error("use an isolated emulator, nonnegative scale, and at least one cycle")
sdk = Path(os.environ["ANDROID_HOME"])
root = Path(__file__).resolve().parents[1]
output = root / "target/smoke"
build = root / "target/android-instrumentation"
classes = build / "classes"
dex = build / "dex"
for directory in [output, classes, dex]: directory.mkdir(parents=True, exist_ok=True)
tools = sdk / "build-tools" / os.environ.get("ANDROID_BUILD_TOOLS_VERSION", "36.0.0")
android_jar = sdk / "platforms/android-35/android.jar"
adb = sdk / "platform-tools/adb"

def run(command, **kwargs):
    result = subprocess.run([str(c) for c in command], capture_output=True, timeout=60, **kwargs)
    if result.returncode:
        raise RuntimeError(f"command failed: {command}\n{result.stdout.decode(errors='replace')}\n{result.stderr.decode(errors='replace')}")
    return result

def device(*command, **kwargs):
    return run([adb, "-s", args.serial, *command], **kwargs).stdout

if device("emu", "avd", "name").decode().splitlines()[0] != "RustFlutterSmoke":
    parser.error("select the isolated RustFlutterSmoke AVD")
source = root / "scripts/android_test"
run(["javac", "--release", "8", "-classpath", android_jar, "-d", classes, *source.rglob("*.java")])
run([tools / "d8", "--lib", android_jar, "--min-api", "24", "--output", dex, *classes.rglob("*.class")])
unsigned = build / "unsigned.apk"
aligned = build / "aligned.apk"
apk = build / "rust-flutter-lifecycle-tests.apk"
run([tools / "aapt", "package", "-f", "-M", source / "AndroidManifest.xml", "-I", android_jar, "-F", unsigned])
with zipfile.ZipFile(unsigned, "a") as archive:
    archive.write(dex / "classes.dex", "classes.dex")
run([tools / "zipalign", "-f", "4", unsigned, aligned])
keystore = Path(os.environ.get("ANDROID_DEBUG_KEYSTORE", str(Path.home() / ".android/debug.keystore")))
run([tools / "apksigner", "sign", "--ks", keystore, "--ks-key-alias", "androiddebugkey", "--ks-pass", "pass:android", "--key-pass", "pass:android", "--out", apk, aligned])
# Android refuses instrumentation of hasCode=false packages. Build a test-only
# variant with an unused marker DEX and hasCode=true; retain every native byte.
# There is no Activity subclass, recreation hook, or model-manipulation code in it.
original_apk = args.apk or root / "target/debug/apk/rust-flutter-gallery.apk"
manifest = ET.parse(original_apk.parent / "AndroidManifest.xml")
android = "{http://schemas.android.com/apk/res/android}"
assert manifest.getroot().get("package") == "dev.rustflutter.gallery"
manifest.getroot().find("application").set(android + "hasCode", "true")
ET.register_namespace("android", "http://schemas.android.com/apk/res/android")
app_manifest = build / "gallery/AndroidManifest.xml"
app_manifest.parent.mkdir(exist_ok=True)
manifest.write(app_manifest, encoding="utf-8", xml_declaration=True)
app_classes = build / "app-classes"
app_dex = build / "app-dex"
for directory in [app_classes, app_dex]:
    if directory.exists(): shutil.rmtree(directory)
    directory.mkdir()
marker = build / "CodeMarker.java"
marker.write_text("package dev.rustflutter.testmarker; public final class CodeMarker { private CodeMarker() {} }\n")
run(["javac", "--release", "8", "-d", app_classes, marker])
run([tools / "d8", "--lib", android_jar, "--min-api", "24", "--output", app_dex, *app_classes.rglob("*.class")])
app_unsigned = build / "gallery-unsigned.apk"
app_aligned = build / "gallery-aligned.apk"
app_apk = build / "gallery-instrumentable.apk"
run([tools / "aapt", "package", "-f", "-M", app_manifest, "-I", android_jar, "-F", app_unsigned])
native_hashes = {}
with zipfile.ZipFile(original_apk) as source_apk, zipfile.ZipFile(app_unsigned, "a") as variant:
    assert "resources.arsc" not in source_apk.namelist(), "resource-bearing APKs require a resource-aware test packager"
    assert "classes.dex" not in source_apk.namelist(), "this variant adapter expects a pure-native cargo-apk gallery"
    for entry in source_apk.infolist():
        if entry.filename == "AndroidManifest.xml" or entry.filename.startswith("META-INF/"):
            continue
        data = source_apk.read(entry)
        variant.writestr(entry, data)
        if entry.filename.endswith(".so"):
            native_hashes[entry.filename] = hashlib.sha256(data).hexdigest()
    variant.write(app_dex / "classes.dex", "classes.dex")
assert native_hashes, "gallery APK contains no native code"
run([tools / "zipalign", "-f", "4", app_unsigned, app_aligned])
run([tools / "apksigner", "sign", "--ks", keystore, "--ks-key-alias", "androiddebugkey", "--ks-pass", "pass:android", "--key-pass", "pass:android", "--out", app_apk, app_aligned])
with zipfile.ZipFile(app_apk) as variant:
    for path, digest in native_hashes.items():
        assert hashlib.sha256(variant.read(path)).hexdigest() == digest
# Reinstalling does not clear the gallery's persisted model.
device("install", "--no-incremental", "-r", app_apk)
device("install", "--no-incremental", "-r", apk)
command = [adb, "-s", args.serial, "shell", "am", "instrument", "-w", "-r", "-e", "scale", str(args.scale), "-e", "top", str(args.top_inset_px), "-e", "cycles", str(args.cycles), "dev.rustflutter.tests/dev.rustflutter.tests.RecreateInstrumentation"]
# Run asynchronously at the tool level; the instrumentation itself has per-stage deadlines.
instrument = subprocess.run([str(c) for c in command], capture_output=True, timeout=180)
text = instrument.stdout.decode(errors="replace") + instrument.stderr.decode(errors="replace")
(output / "android-instrumentation.txt").write_text(text)
match = re.search(r"^INSTRUMENTATION_RESULT: summary=(.+)$", text, re.MULTILINE)
if instrument.returncode or not match or "INSTRUMENTATION_CODE: -1" not in text:
    raise AssertionError(f"instrumentation did not pass; see {output / 'android-instrumentation.txt'}\n{text}")
result = json.loads(match[1])
assert result["passed"] and len(result["cycles"]) == args.cycles
pid = result["pid"]
trace = device("logcat", "-d", f"--pid={pid}", "-s", "RustStdoutStderr:I").decode(errors="replace")
(output / "android-recreation-trace.txt").write_text(trace)
events = [(int(host), int(process), event) for host, process, event in re.findall(r"rust-flutter android host=(\d+) pid=(\d+) ([a-z-]+)", trace)]
created = sorted({host for host, process, event in events if process == pid and event == "created"})
assert len(created) >= args.cycles + 1, f"expected a new Rust host per Activity: {events}"
for host in created:
    assert (host, pid, "destroyed") in events and (host, pid, "returned") in events, f"host {host} did not release the native destruction handshake"
result["rust_host_generations"] = created
result["native_library_sha256"] = native_hashes
result["test_variant"] = "Original native code; hasCode=true and an unused marker DEX added solely because Android rejects instrumentation of pure-native packages"
result["scope"] = "Activity.recreate and Activity.finish, OS destroy callbacks, native loop returns, fresh GPU surfaces, one process, restored model and injected touch after every recreation"
(output / "android-recreation-result.json").write_text(json.dumps(result, indent=2) + "\n")
# The instrumentation captures the rendered frame before finishing the final Activity.
shot = subprocess.run([str(adb), "-s", args.serial, "exec-out", "run-as", "dev.rustflutter.gallery", "cat", "cache/recreation-final.png"], capture_output=True, timeout=15)
if shot.returncode == 0:
    (output / "android-recreated.png").write_bytes(shot.stdout)
print(f"PASS: {args.cycles} Activity.recreate cycles and Activity.finish, PID {pid}, Rust hosts {created}, responsive restored UI")
