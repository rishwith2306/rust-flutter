# Verification log

Local environment: Linux x86_64, Rust 1.98.1 / Cargo 1.98.1. Dependencies are
pinned in Cargo.lock. Results below describe this workspace, not Flutter parity.
No remote CI run or macOS runtime result has been obtained.

| Check | Result | Evidence / scope |
| --- | --- | --- |
| Workspace compilation | Passed | Library, CLI and shared gallery source |
| Workspace tests | Passed, 84 tests | Navigation, signals/repaint, workers, animation, services, display insets, Fluent negotiation/plurals/errors, snapshots and corruption/concurrent writes, lifecycle/back/close, native destroy terminality and repaint deadlines, asset compiler/resolution/decoding/cache bounds, constraint layout/flex/stack/clip/hit testing, stable leaf IDs, element reconciliation/lifecycle/queued-update races, inherited scopes/predicates/lifetime/worker updates, scaffold arguments and gallery restoration |
| Desktop Clippy | Passed with warnings denied | All workspace targets |
| Android Clippy | Passed with warnings denied | Framework compiled for x86_64-linux-android, including JNI code |
| Formatting | Passed | `cargo fmt --all -- --check` |
| Rustdoc | Passed | `cargo doc --workspace --no-deps --locked` |
| Generated desktop app | Passed | CLI scaffold compiled against local library |
| Desktop rendered interaction | Passed | Isolated Xvfb display: real counter click, keyboard text, fresh-process restoration; `scripts/desktop-smoke.py` |
| Asset Cargo rebuild smoke | Passed | Temporary GPU-free executable loads generated bundle; changing text and adding/removing a density variant rebuilds the embedded data automatically |
| Bundled image rendering | Passed within tested scope | Desktop screenshot confirms 1× PNG pixels when launched outside source directory; Android screenshot shows the 3× variant at 48 logical points, API 37 / x86_64 / 16 KB pages |
| Constraint layout geometry and UI bridge | Passed, 14 tests | Constraints, flex weights/spacing/directions, shrink wrapping, positioned stacks, numeric overflow, clip bounds, hit order, measured-size validation, stable widget IDs after reorder and overlapping-control pointer activation |
| Constraint layout desktop rendering | Passed | Fixed child plus 1:2 flex, positioned overlay and clicks before/after resize; image row measures 297 / 595 pixels in the resized window |
| Constraint layout Android rendering | Passed on test emulator | Gallery screenshot shows fixed child, 1:2 flexible children and overlay; injected touch increments the constrained button counter to 5, then system Back returns home |
| Retained component engine | Passed, 17 tests | State/init/update/disposal, sibling keys, type replacement, update races, dirty-child isolation, failures, stale handles, measured leaves and actual pointer activation |
| Retained component desktop rendering | Passed | Foreground glyph-mask comparisons verify clicks, counter state after keyed reorder, removal, and fresh state on reinsertion; no direct component-state test hook |
| Retained component Android rendering | Passed on test emulator | Injected touches and exact trimmed foreground glyph masks verify state through swaps, removal and reinsertion at 2.625 density; screenshot shows A=1, fresh B=0, init=3, dispose=1 |
| Inherited values | Passed, 16 tests | Exact-type and nearest lookup, shadowing, read/listen, predicates, snapshots, coalesced dependency hooks, mounted-lifetime registrations, non-Send values with worker updates, cleanup, measurement/paint/event registrations and stable render identity |
| Inherited Step desktop rendering | Earlier probe passed; final-source run pending | Step update preserves A/B counts, then a single B click matches the original two-click glyphs |
| Inherited Step Android rendering | Pending current run | Same shared image-mask probe using real injected touch on isolated emulator |
| ARM64 Android APK | Built and debug-signed | NDK 28.2.13676358, min SDK 24 / target SDK 35; APK signature and alignment checked |
| x86_64 Android APK | Built, installed and launched | Isolated Pixel 6 emulator, Android 37 system image with 16 KB pages |
| Android basic interaction | Passed within smoke scope | GLES rendering, counter touch, injected text `Ada`, private JSON snapshot writes and route back |
| Android system-bar/cutout insets | Visually verified | Java WindowInsets query on Java UI thread; screenshot clears the notch/status bar and navigation bar |
| Android extended runtime test | Passed after manifest correction | Touch, injected text, route back, background/resume, force-stop and fresh-process restoration on Android 17 / API 37, x86_64, 16 KB pages. The separate recreation test below covers activity destruction. |
| Android activity recreation | Passed, three cycles in one process | `Activity.recreate()` produces fresh Rust hosts, each previous loop returns; persisted state and injected touch verified after every recreation; final `Activity.finish()` destroys the fourth host. Test-only instrumentable variant retains identical native bytes. |
| macOS compilation / runtime | Not verified locally | Workflow is configured; it has not been run. No Metal/window/VoiceOver claim is established. |
| Full Flutter feature parity | Not achieved | See feature audit and API inventory; most declarations remain unreviewed |

## Reproduce local checks

```sh
cargo fmt --all -- --check
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo clippy -p rust-flutter --target x86_64-linux-android --locked -- -D warnings
cargo doc --workspace --no-deps --locked
cargo build -p rust-flutter-gallery --locked
python3 scripts/desktop-smoke.py
python3 scripts/asset-build-smoke.py
```

Android packaging commands are in `platforms.md`. The extended emulator test is
profile-specific, requires a debug APK and ImageMagick (`convert`), and resets the gallery's app data. Use only an isolated test
emulator, not an existing personal device/profile:

```sh
ANDROID_HOME=/path/to/Android/Sdk python3 scripts/android-smoke.py \
  --serial emulator-5554 --scale 2.625 --top-inset-px 128 --reset-gallery-data
```

The ordinary smoke test uses the unchanged pure-native cargo-apk package. Its
startup gate waits for a successful presented frame with the expected density and
safe top inset, using debug host diagnostics. A prior cold-start run injected the
first touch before presentation; the snapshot file alone was insufficient
readiness evidence. The corrected gate and extended smoke sequence pass. The
separate recreation test requires the AVD to be named `RustFlutterSmoke`:

```sh
ANDROID_HOME=/path/to/Android/Sdk python3 scripts/android-recreation.py \
  --serial emulator-5554 --cycles 3
```

It compiles a separate Android instrumentation package with Java and SDK build
tools (default 36.0.0, override `ANDROID_BUILD_TOOLS_VERSION`). The raw gallery
manifest must sit alongside the original debug APK. Density and top insets are
read from the activity unless explicitly supplied. Android refuses instrumentation
of `hasCode=false` packages, so the script installs a **test-only gallery variant**
with `hasCode=true` and an unused marker DEX. Every native library is byte-for-byte
identical to the original APK, checked by SHA-256. There is no Activity subclass,
recreation backdoor or model mutation hook. This variant is limited to the current
resource-free pure-native gallery. It remains installed after the test; reinstall
the ordinary cargo-apk package to restore production packaging.

The latest local result records PID 4747, four native host generations, four
completed destruction handshakes, and a final rendered count of 9 with name `Ada`
preserved.
Each of the three recreations increased the counter through injected touch on its
new surface. This establishes the lifecycle repair on one Android 17 / API 37
x86_64 emulator with 16 KB pages; it does not establish all lifecycle behavior on
all Android versions or devices. A stable API 35 emulator CI job is configured,
but no remote CI result has been obtained.

The component visual probe thresholds white text at 75% intensity and trims ink
bounds before exact mask comparison. This removes positional antialiasing
differences seen at fractional density without allowing different digit shapes.
It also requires visible changes after injected clicks and fresh B text matching
the initial B after reinsertion. These are targeted component-glyph checks, not
full-screen goldens or an accessibility validation.

Generated evidence is under `target/smoke/`: desktop JSON results/screenshots,
Android input/settings/inset screenshots, and diagnostics when available. APKs
are under `target/dist/`. Intermediate captures can include an empty frame or an
ANR dialog; inspect the specific image before interpreting it as a rendering pass.
The Android ANR screenshot is `android-settings-manual.png` and the verified inset
capture is `android-edge-to-edge.png`.

## Remaining runtime gates

1. Validate real soft-keyboard composition and candidate selection. Injected text
   is weaker evidence; the NativeActivity glue does not supply composition events.
2. Validate rotation/configuration changes, IME occlusion, permissions, clipboard,
   links, system back variations and TalkBack with actual platform services.
3. Repeat on stable Android SDKs and physical devices, then expand the OS matrix.
4. Run macOS window/input/Metal/VoiceOver checks on a Mac.

Insets currently poll at 1 Hz for edge-to-edge apps on Android/target SDK 35+.
An event-driven inset listener, older OS/device validation and IME view-inset
handling remain work items. Lifecycle notifications do not yet serialize Android
native saved-instance bytes; the gallery persists its model in private JSON.

## ANR stack finding

The captured `android-anr-threads.txt` shows the Java main thread blocked in
`WaitableNativeActivityState::notify_destroyed` / `NativeActivity.onDestroy`, while
`android_main` remains in the winit event loop's `ALooper_pollOnce`. The Java stack
includes `ActivityThread.handleRelaunchActivity`: an activity configuration
relaunch occurred. Inspection of winit 0.30.13 confirms its `MainEvent::Destroy`
branch logs a TODO and does not exit the loop. This is stronger evidence than the
initial keyboard-timing hypothesis.

The gallery and generated project manifests handle standard native UI
configuration changes in-process. That avoided incidental relaunches but did not
resolve this defect. The framework now owns the Android activity loop directly
through android-activity 0.6.1, drops surfaces on termination, and exits on Destroy.
It no longer constructs a winit event loop on Android. The separate recreation
and finish test verifies the old loop returns and a new host initializes in the
same process, without a process-wide loop-creation guard. Desktop still uses
winit through eframe. The captured ANR diagnostics are retained as historical
failure evidence, not a current passing result.
