# Platform development

## Desktop / macOS

Install Rust 1.89+ and Xcode Command Line Tools. Run:

```sh
cargo run -p rust-flutter-gallery
cargo build -p rust-flutter-gallery --release
```

The executable appears in `target/release/rust-flutter-gallery`. Building an Apple
Silicon or Intel executable requires the corresponding Rust target on a Mac with
an appropriate SDK. Cross-checking Rust code on Linux is not a macOS runtime test.
App bundle creation, icons, entitlements, signing, notarization and store delivery
are not automated yet. The macOS CI job checks source compilation and core tests;
it does not replace interactive window/input/VoiceOver tests.

## Android NativeActivity

Install Android SDK command-line tools, platform-tools, an Android platform SDK,
an NDK, Java, and the Rust target. For the checked-in gallery use:

```sh
rustup target add aarch64-linux-android
cargo install cargo-apk --locked
export ANDROID_HOME=/path/to/Android/Sdk
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/28.2.13676358"
cargo apk build -p rust-flutter-gallery --lib
cargo apk run -p rust-flutter-gallery --lib
```

Use the installed NDK's version directory if different. Connect an authorized ARM64
Android device or configure a compatible emulator before `run`. For an x86_64
emulator, install `x86_64-linux-android` and pass `--target x86_64-linux-android`.
The package metadata selects API 24 minimum and API 35 target as an initial test
configuration, not a claim about current app-store submission requirements.

`android_main` is exported by `examples/gallery/src/lib.rs`. The framework pins `android-activity` 0.6.1 and re-exports its `AndroidApp`
type. The Android runner owns its activity loop and GPU surface lifetime; it does
not use winit's event loop or its process-wide loop-creation guard. The eframe
dependency still resolves the same glue crate for type compatibility. Do not switch to
GameActivity without changing both packaging and the glue configuration.

For a compile-only library check, configure the linker to the NDK wrapper:

```sh
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android24-clang"
cargo check --workspace --target aarch64-linux-android
```

A successful `check` does not link an APK or verify touch, keyboard, rotation,
background/resume, permissions or TalkBack. Those remain explicit device gates.

NativeActivity currently builds without eframe's AccessKit integration: that
feature requires GameActivity in this backend version. TalkBack support needs a
GameActivity host and packaging, or a separately implemented semantics bridge.

## Release gates

Before claiming support, run the gallery on each platform and check input,
resizing/rotation, text entry and IME, suspend/resume, rendering surface recreation,
route behavior, theme changes, accessibility, and error logs. Record exact OS,
device, architecture and toolchain. Release APK signing and macOS notarization
must be verified separately from debug builds.

Upstream references: [eframe options](https://docs.rs/eframe/0.33.3/eframe/struct.NativeOptions.html),
[winit Android integration](https://docs.rs/winit/0.30.13/winit/platform/android/index.html),
[cargo-apk](https://github.com/rust-mobile/cargo-apk).

## Rendering selection

The default Android runner selects GLES alone. Enabling Vulkan and GLES together
can cause competing Android native-surface connections during adapter selection.
Both backends are compiled in; applications can choose explicitly with
`native_options(GraphicsBackend::Vulkan)` and pass an activity through
`NativeOptions.android_app` to `run_with_options`. Desktop defaults retain backend
auto-selection, with Metal compiled for macOS. Unsupported explicit choices return
backend initialization errors; there is no cross-backend retry after taking a surface.

The runtime reads the legacy Android content rectangle and queries Java
`WindowInsets` on the Java UI thread for edge-to-edge apps targeting SDK 35+ on
Android 35+. System bars and display cutouts are converted to logical safe-area
insets. The current NativeActivity integration polls at 1 Hz; a listener-based
host remains future work. IME occlusion and a wider OS/device matrix still need
validation. Insets have been visually checked on the Android 37 test emulator.

## Android lifecycle regression test

After building the x86_64 debug gallery, run `scripts/android-recreation.py
--serial emulator-5554 --cycles 3` with `ANDROID_HOME` set against an isolated AVD
named `RustFlutterSmoke`. It tests three `Activity.recreate()` cycles in one
process and final activity destruction, requiring every Rust host to return and
every new surface to accept touch. See [verification](verification.md) for the
instrumentable test-only packaging qualification and ordinary APK smoke test.

The native host emits `AppEvent::Lifecycle` for activity, surface, memory,
configuration and save-state notifications. Applications should persist durable
state on pause/save-state; native saved-instance byte serialization is pending.
Mobile IME composition, clipboard and URL output commands, native child viewports
and accessibility semantics are not implemented by this host yet. Winit-specific
window/event-loop hooks and unsupported depth/stencil/multisampling options return
an explicit error. These limits remain part of the feature backlog.

Linux X11 runtime requires `libxkbcommon-x11.so` as well as the development
libraries. On Debian/Ubuntu install `libxkbcommon-x11-0`; CI also installs
Xvfb, xdotool, ImageMagick and Mesa Vulkan drivers for rendered smoke checks.
