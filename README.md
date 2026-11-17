# Rust Flutter (experimental)

Build desktop and Android applications from a shared Rust application model.
The library provides composable widgets, typed navigation, reactive shared state,
background tasks, themes, responsive breakpoints, animation primitives, lifecycle
hooks, versioned JSON snapshots, Fluent localization, and embedded asset bundles
with density-aware images and a bounded texture cache. An owned constraint-based
box layout tree adds flex/expanded children, padding, alignment and positioned stacks.
`ElementTree` adds declarative stateful/stateless components, sibling keys, queued
state updates and lifecycle hooks. Typed inherited scopes notify dependent
components while preserving their state. Inherited models add typed aspect
subscriptions and nested supported-aspect lookup. Observable values, scoped
listeners and notifier-driven builders support shared UI-local state.
Desktop windows use [eframe 0.33.3](https://docs.rs/eframe/0.33.3/eframe/);
Android uses a framework-owned NativeActivity host. Both render egui through wgpu.

**Status: early development. Full Flutter parity is not implemented.**
Android rendering, input, back navigation, background/resume and process-state
restoration pass an emulator smoke test. Three activity recreations in one process
and final activity destruction also pass a dedicated test; see its packaging
qualification in [verification results](docs/verification.md). macOS runtime
behavior is not yet verified.
See [the feature audit](docs/flutter-parity.md) for the gaps and verification status.
There is no dependency on Dart, the Flutter SDK, or Flutter's engine. Applications can use immediate-mode helpers or retained components. Painting and
input still use egui; the complete Flutter Widget/Element/RenderObject pipeline
is not implemented.

## Run the gallery

Rust 1.89+ and your platform's native build tools are required.

```sh
cargo run -p rust-flutter-gallery
```

On macOS install Xcode Command Line Tools (`xcode-select --install`). The renderer
uses wgpu with Metal on macOS. On Linux, Wayland/X11 development libraries and a
Vulkan or GLES driver are needed. Linux is a development host, in addition to the
requested macOS and Android targets.

## Create an app

```sh
cargo install --git https://github.com/rishwith2306/rust-flutter --locked rust-flutter-cli
rust-flutter create my-app
cd my-app
rust-flutter run
```

`rust-flutter doctor --android` checks Android build prerequisites.
`rust-flutter devices` lists connected devices. `build` and `run` accept
`--android`, `--release`, and `--target`; Android `run` also accepts `--device`. Generated projects pin both framework crates to the CLI source commit on GitHub.
Commit the generated `Cargo.lock` when sharing your app. Contributors can use
`rust-flutter create my-app --framework-path /path/to/rust-flutter` to test local changes.

## Use the library

The CLI configures Git dependencies automatically. For local framework development:

```toml
[dependencies]
rust-flutter = { path = "/path/to/rust-flutter/crates/rust-flutter" }

[patch.crates-io]
wgpu-hal = { path = "/path/to/rust-flutter/vendor/wgpu-hal" }
```

```rust
use rust_flutter::prelude::*;

#[derive(Default)]
struct Counter { count: usize }
impl Application for Counter {
    fn build(&mut self, ui: &mut Ui) {
        column(ui, |ui| {
            heading(ui, format!("Count: {}", self.count));
            if button(ui, "Increment").clicked() {
                self.count += 1;
            }
        });
    }
}
fn main() -> Result<(), rust_flutter::Error> {
    rust_flutter::run("Counter", Counter::default())
}
```

Generated apps include the required renderer override. Existing/manual consumers
should follow [renderer patch setup](docs/renderer-patches.md); Cargo only honors
patches at the application workspace root.

`egui` is re-exported for sliders, checkboxes, grids, images, custom painting,
keyboard shortcuts and advanced layout. Functions in `widgets` return responses
so applications can handle clicks, focus, hover and validation directly.
Give repeated text fields and scroll areas unique, stable IDs. Keep file/network
I/O outside `build`; `state::spawn_task` runs work on a background thread and
requests a frame when results are available. `Signal` read/update closures must
not recursively access the same signal. Tasks are not automatically cancelled.

## Android

See [platform setup](docs/platforms.md). The gallery contains both a desktop binary
and an Android `cdylib`; the application model is shared between them.

## Checks

```sh
cargo fmt --all -- --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
cargo doc --workspace --no-deps
```

The tests cover localization/plural rules, snapshot integrity and restoration,
lifecycle dispatch, pointer activation and paint generation without a window,
navigation invariants, cross-thread state notifications, worker failure, animation
boundaries, service replacement and responsive thresholds. These tests do not
prove device lifecycle, macOS integration or full accessibility behavior.

See [observable state](docs/notifiers.md), [inherited models](docs/inherited-models.md), [inherited values](docs/inherited.md), [stateful components](docs/components.md), [constraint layout](docs/layout.md), [asset bundles](docs/assets.md), [application services](docs/application-services.md),
[view geometry and keyboard metrics](docs/display.md), [custom render objects](docs/render-objects.md), [dry and intrinsic sizing](docs/size-queries.md), [baseline layout](docs/baselines.md),
[retained paragraphs and styled spans](docs/paragraphs.md), [inherited typography](docs/text-styles.md), [verification results](docs/verification.md), [architecture](docs/architecture.md), [feature audit](docs/flutter-parity.md),
and [contributing](CONTRIBUTING.md) before extending the framework.

## Contributing and releases

See [CONTRIBUTING.md](CONTRIBUTING.md) for development and review,
[release instructions](docs/releases.md) for maintaining versions, and
[CHANGELOG.md](CHANGELOG.md) for changes. Original framework source is MIT licensed; the vendored renderer retains its upstream licenses.
The project is independent of Google and the Flutter project.
