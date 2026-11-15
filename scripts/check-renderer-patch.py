#!/usr/bin/env python3
"""Verify the pinned vendored backend against its recorded original crate files.
Reverse the checked-in patch in a temporary copy; do not modify the working tree.
"""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
origin = json.loads((root / "patches/wgpu-hal-27.0.4-origin.json").read_text())
source = root / "vendor/wgpu-hal"
expected = origin["files"]
actual = {str(path.relative_to(source)) for path in source.rglob("*") if path.is_file()}
assert actual == set(expected), f"vendor file set changed: {actual ^ set(expected)}"
with tempfile.TemporaryDirectory(prefix="rust-flutter-renderer-patch-") as temporary:
    destination = Path(temporary) / "wgpu-hal"
    shutil.copytree(source, destination)
    subprocess.run(["git", "apply", "--reverse", str(root / "patches/wgpu-hal-27.0.4-android-rgba8.patch")],
                   cwd=destination, check=True, stdin=subprocess.DEVNULL, timeout=30)
    for relative, digest in expected.items():
        assert hashlib.sha256((destination / relative).read_bytes()).hexdigest() == digest, f"unrecorded vendor change: {relative}"
print(f"PASS: {origin['crate']} {origin['version']} matches the recorded upstream archive plus the reviewed patch")
