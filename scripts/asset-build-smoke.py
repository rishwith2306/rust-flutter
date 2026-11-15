#!/usr/bin/env python3
"""Verify Cargo rebundles changed files and newly discovered density variants.
Uses a temporary, GPU-free Rust application and the workspace's Cargo target cache.
"""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

root = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="rust-flutter-assets-") as temporary:
    project = Path(temporary)
    (project / "src").mkdir()
    (project / "assets/2x").mkdir(parents=True)
    dependency = json.dumps(str(root / "crates/rust-flutter-assets"))
    (project / "Cargo.toml").write_text(f'''[package]
name = "rust-flutter-asset-build-probe"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
rust-flutter-assets = {{ path = {dependency} }}
[build-dependencies]
rust-flutter-assets = {{ path = {dependency}, features = ["build"] }}
''')
    (project / "build.rs").write_text('''fn main() {
    rust_flutter_assets::build::compile("assets.json",
        std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("assets.rs")).unwrap();
}
''')
    (project / "src/main.rs").write_text('''use rust_flutter_assets::{AssetBundle, EmbeddedBundle};
static BUNDLE: EmbeddedBundle = EmbeddedBundle::new(include!(concat!(env!("OUT_DIR"), "/assets.rs")));
fn main() {
    let selected = BUNDLE.resolve("assets/icon.bin", 2.75).unwrap();
    println!("{}|{}|{}", BUNDLE.load_string("assets/message.txt").unwrap(), selected.scale, std::str::from_utf8(selected.bytes).unwrap());
}
''')
    (project / "assets.json").write_text('{"version":1,"assets":["assets/"]}')
    (project / "assets/message.txt").write_text("first")
    (project / "assets/icon.bin").write_bytes(b"base")
    (project / "assets/2x/icon.bin").write_bytes(b"double")
    env = {**os.environ, "CARGO_TARGET_DIR": str(root / "target")}

    def run(expected):
        result = subprocess.run(["cargo", "run", "--offline", "--quiet", "--manifest-path", str(project / "Cargo.toml")], env=env, text=True, capture_output=True, timeout=180)
        assert result.returncode == 0, result.stderr
        assert result.stdout.strip() == expected, result.stdout

    run("first|2|double")
    # Distinct mtimes matter on build hosts with coarse filesystem timestamps.
    time.sleep(1.1)
    (project / "assets/message.txt").write_text("updated")
    run("updated|2|double")
    time.sleep(1.1)
    (project / "assets/3x").mkdir()
    (project / "assets/3x/icon.bin").write_bytes(b"triple")
    run("updated|3|triple")
    time.sleep(1.1)
    (project / "assets/3x/icon.bin").unlink()
    run("updated|2|double")
print("PASS: generated bundle executes, changed text and added/removed density variants trigger Cargo rebuilds")
