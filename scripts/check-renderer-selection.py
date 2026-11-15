#!/usr/bin/env python3
"""Verify that an app resolves exactly the reviewed renderer source, not registry HAL."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--manifest-path", type=Path, default=Path(__file__).resolve().parents[1] / "Cargo.toml")
args = parser.parse_args()
root = Path(__file__).resolve().parents[1]
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1", "--manifest-path", str(args.manifest_path)], timeout=180))
packages = [package for package in metadata["packages"] if package["name"] == "wgpu-hal"]
assert len(packages) == 1 and packages[0]["version"] == "27.0.4", "app does not resolve the pinned single renderer"
selected = Path(packages[0]["manifest_path"]).parent
for relative in ["Cargo.toml", "src/gles/egl.rs", "LICENSE.MIT", "LICENSE.APACHE"]:
    assert hashlib.sha256((selected / relative).read_bytes()).digest() == hashlib.sha256((root / "vendor/wgpu-hal" / relative).read_bytes()).digest(), f"app did not select the reviewed renderer: {relative}"
print("PASS: app resolves the pinned, patched wgpu-hal source")
