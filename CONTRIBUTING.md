# Contributing

This is an experimental Rust UI framework targeting macOS and Android first.
Start with README.md, docs/architecture.md and docs/flutter-parity.md. Public APIs
are evolving; describe compatibility changes explicitly in pull requests.

## Develop

Install stable Rust and platform tools described in docs/platforms.md, then clone:

```sh
git clone https://github.com/rishwith2306/rust-flutter.git
cd rust-flutter
cargo run -p rust-flutter-gallery
cargo run -p rust-flutter-cli -- create ../sample-app --framework-path .
```

The framework is in crates/rust-flutter; asset compilation is in
crates/rust-flutter-assets; the CLI and starter templates are in
crates/rust-flutter-cli. examples/gallery demonstrates public APIs.

Before submitting:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo doc --workspace --all-features --no-deps --locked
```

Add tests for observable behavior and regressions, plus a gallery example when
an API benefits from one. For rendering or lifecycle changes, run the applicable
scripts/desktop-smoke.py, scripts/android-smoke.py or scripts/android-recreation.py
as described in docs/verification.md. Distinguish headless tests from device
runtime evidence. Never mark a Flutter API supported solely because it compiles.
Update the parity audit and documentation with actual behavior and limitations.

Keep changes focused. Describe the problem, resulting behavior, validation and
platform gaps in your pull request. Do not commit build products, signing keys,
credentials, emulator data or personal application data.
