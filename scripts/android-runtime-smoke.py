#!/usr/bin/env python3
"""Run lifecycle instrumentation, restore ordinary packaging, then real UI probes.
Requires the isolated RustFlutterSmoke AVD. Resets only the gallery's app data.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--serial", required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
subprocess.run([sys.executable, str(root / "scripts/android-recreation.py"), "--serial", args.serial,
                "--cycles", "3", "--stop-pixel-launcher"], check=True, timeout=300)
metrics = json.loads((root / "target/smoke/android-recreation-result.json").read_text())
adb = Path(os.environ["ANDROID_HOME"]) / "platform-tools/adb"
subprocess.run([str(adb), "-s", args.serial, "install", "--no-incremental", "-r",
                str(root / "target/debug/apk/rust-flutter-gallery.apk")], check=True, timeout=60)
subprocess.run([sys.executable, str(root / "scripts/android-smoke.py"), "--serial", args.serial,
                "--scale", str(metrics["scale"]), "--top-inset-px", str(metrics["top_inset_px"]),
                "--reset-gallery-data", "--wheel-instrumentation"], check=True, timeout=300)
print("PASS: lifecycle and ordinary-package RGB888/input/restoration probes")
