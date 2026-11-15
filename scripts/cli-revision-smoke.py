#!/usr/bin/env python3
"""Check CLI revision pinning across cached builds before/after Git initialization."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib

source = Path(__file__).resolve().parents[1]
env = dict(os.environ)
env.pop("RUST_FLUTTER_FRAMEWORK_REV", None)
with tempfile.TemporaryDirectory(prefix="rust-flutter-cli-revision-") as temporary:
    root = Path(temporary)
    shutil.copytree(source / "crates/rust-flutter-cli", root / "crates/rust-flutter-cli")
    metadata = tomllib.loads((source / "Cargo.toml").read_text())["workspace"]["package"]
    text = '[workspace]\nmembers = ["crates/rust-flutter-cli"]\nresolver = "2"\n\n[workspace.package]\n'
    text += ''.join(f'{key} = {json.dumps(value)}\n' for key, value in metadata.items())
    (root / "Cargo.toml").write_text(text)
    (root / ".gitignore").write_text("/target/\n/generated*/\n/hooks/\n")
    (root / "hooks").mkdir()

    def run(*args, **kwargs):
        return subprocess.check_output(args, cwd=root, env=env, text=True, timeout=120, **kwargs).strip()

    def generated_revision(index):
        run("cargo", "build", "--quiet", "--manifest-path", str(root / "crates/rust-flutter-cli/Cargo.toml"), "--target-dir", str(root / "target"))
        app = root / f"generated{index}"
        run(str(root / "target/debug/rust-flutter"), "create", str(app))
        manifest = tomllib.loads((app / "Cargo.toml").read_text())
        revisions = [manifest["dependencies"]["rust-flutter"]["rev"], manifest["build-dependencies"]["rust-flutter-assets"]["rev"], manifest["patch"]["crates-io"]["wgpu-hal"]["rev"]]
        assert len(set(revisions)) == 1, revisions
        return revisions[0]

    assert generated_revision(0) == f"v{metadata['version']}"
    run("git", "init", "-q", "-b", "main")
    run("git", "config", "user.name", "Rust Flutter fixture")
    run("git", "config", "user.email", "fixture@example.invalid")
    run("git", "config", "commit.gpgsign", "false")
    run("git", "config", "core.hooksPath", str(root / "hooks"))
    run("git", "add", "Cargo.toml", ".gitignore", "crates")
    run("git", "commit", "-q", "-m", "Initialize fixture")
    first = run("git", "rev-parse", "HEAD")
    assert generated_revision(1) == first, "cached CLI did not notice Git initialization"
    (root / "marker.txt").write_text("second source revision\n")
    run("git", "add", "marker.txt")
    run("git", "commit", "-q", "-m", "Advance fixture HEAD")
    second = run("git", "rev-parse", "HEAD")
    assert first != second and generated_revision(2) == second, "cached CLI did not notice a changed HEAD"
print("PASS: cached CLI pins framework, assets and renderer after Git initialization and HEAD changes")
