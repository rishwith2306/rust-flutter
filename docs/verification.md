# Verification log

Local environment: Linux x86_64, Rust 1.98.1 / Cargo 1.98.1. Dependencies are
pinned in Cargo.lock. Results below describe this workspace, not Flutter parity.
The public repository runs [GitHub Actions](https://github.com/rishwith2306/rust-flutter/actions).
macOS build checks passed in [run 43745194590](https://github.com/rishwith2306/rust-flutter/actions/runs/43745194590),
including tests, Clippy, documentation, asset rebuilding and generated-app
compilation. The ARM64 Android link/Clippy job also passed in that run.
Runtime claims below remain scoped to their stated environment.

| Check | Result | Evidence / scope |
| --- | --- | --- |
| Workspace compilation | Passed | Library, CLI and shared gallery source |
| Workspace tests | Passed, 200 tests | Navigation, signals/repaint, workers, animation, services, display insets, Fluent negotiation/plurals/errors, snapshots and corruption/concurrent writes, lifecycle/back/close, native destroy terminality and repaint deadlines, asset compiler/resolution/decoding/cache bounds, constraint layout/flex/stack/clip/hit testing, stable leaf IDs, element reconciliation/lifecycle/queued-update races, inherited scopes/predicates/lifetime/worker updates, model aspect unions/gates/nested fallback, notifier reentrancy/dispatch/disposal/weak subscriptions, equality/merged-source rollback, inherited notifier rebinding/coalescing/failures/builders, scaffold arguments and gallery restoration |
| Desktop Clippy | Passed with warnings denied | All workspace targets |
| Android Clippy | Passed with warnings denied | Framework compiled for x86_64-linux-android, including JNI code |
| Formatting | Passed | `cargo fmt --all -- --check` |
| Rustdoc | Passed | `cargo doc --workspace --no-deps --locked` |
| Generated desktop app | Passed | GitHub-installed CLI creates an external app with pinned public Git dependencies; cargo check passes |
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
| Inherited Step desktop rendering | Passed on final source | Step update preserves A/B counts, then a single B click matches the original two-click glyphs |
| Inherited Step Android rendering | Passed on test emulator | Shared image-mask probe using injected touch; changing Step preserves counts and a single click with Step 2 matches the original two-click result |
| Inherited models | Passed, 8 tests | Independent aspect builds/hooks, set union, unconditional precedence, global notification gate, forced notify, nested supported-aspect traversal, missing models, worker-queued transitions and cleanup |
| Inherited model desktop rendering | Passed | Shared glyph-mask probe preserves A/B counts across an A-only aspect change, then A responds while B count remains unchanged; headless tests separately prove B does not rebuild |
| Inherited model Android rendering | Passed on test emulator | Shared injected-touch and glyph-mask probe on API 37 / x86_64 / 16 KB pages, including A-only aspect update and subsequent input |
| Notifier foundations | Passed, 11 tests | Ordered add/remove/recursive dispatch, exact duplicate registrations, panic isolation, disposal, capture-destructor reentrancy, scoped/weak ownership, value equality/interior mutation and merged-source rollback |
| Inherited notifier and builder adapters | Passed, 11 tests | Coalesced hooks, UI repaint wake, local-state preservation, source swaps/removal, custom gate, latest pending event, nested scopes, failed subscription cleanup, builder adapters and type-erased sources |
| Notifier desktop rendering | Passed | Three individual clicks and a three-update burst produce identical counter glyph masks; reset restores the original text |
| Notifier Android rendering | Passed on API 37 / x86_64 / 16 KB pages | Same shared touch/image probe; exact RGB888 fills and foreground masks verify reset and three-update burst against three individual touches |
| Pinned renderer source | Passed | Original 27.0.4 archive checksum verified; reverse-patch/file-hash verifier checks all 80 upstream files and licenses |
| Generated local consumer renderer | Passed | Fresh scaffold compiles outside the checkout and selects the patched HAL; verifier rejects a separate consumer with the root patch omitted |
| CLI cached revision pinning | Passed | Fresh temporary checkout built before Git initialization then rebuilt after initialization and a new commit; framework, assets and renderer all pin the actual current SHA |
| RGB888 presentation | Passed on Linux and API 37 Android | Exact solid blue, green and orange samples; Android native surface R8G8B8A8_UNORM after initial creation and all four activity hosts |
| ARM64 Android APK | Built and debug-signed | NDK 28.2.13676358, min SDK 24 / target SDK 35; APK signature and alignment checked |
| x86_64 Android APK | Built, installed and launched | Isolated Pixel 6 emulator, Android 37 system image with 16 KB pages |
| Android basic interaction | Passed within smoke scope | GLES rendering, counter touch, injected text `Ada`, private JSON snapshot writes and route back |
| Android system-bar/cutout insets | Visually verified | Java WindowInsets query on Java UI thread; screenshot clears the notch/status bar and navigation bar |
| Android extended runtime test | Passed after manifest correction | Touch, injected text, route back, background/resume, force-stop and fresh-process restoration on Android 17 / API 37, x86_64, 16 KB pages. The separate recreation test below covers activity destruction. |
| Android activity recreation | Passed, three cycles in one process | `Activity.recreate()` produces fresh Rust hosts, each previous loop returns; persisted state and injected touch verified after every recreation; final `Activity.finish()` destroys the fourth host. Test-only instrumentable variant retains identical native bytes. |
| macOS compilation | Passed on GitHub macos-latest / Apple Silicon | Stable Rust 1.100.0: workspace tests, Clippy, docs, asset rebuild and generated-app compilation |
| macOS runtime | Not verified | No window/input/Metal/VoiceOver runtime claim is established. |
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
text-entry gate waits for both the keyboard request and Window Manager IME
provider visibility before injecting keys or Back, avoiding a request/window-opening
race. The old private mImeWindowVis dump field is absent on API 35. Request/visibility
transitions, IME dumps and screenshots are retained as test evidence. Its
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
read from the activity unless explicitly supplied. CI passes `--stop-pixel-launcher`
to stop the Google API image launcher immediately before instrumentation: cold boot
occasionally leaves a Pixel Launcher ANR dialog blocking gallery input. This
option is restricted to the validated isolated AVD; it does not dismiss gallery
ANRs or relax the required window focus/touch assertions. Failure artifacts
include a screenshot, window focus state and logcat.
Android refuses instrumentation
of `hasCode=false` packages, so the script installs a **test-only gallery variant**
with `hasCode=true` and an unused marker DEX. Every native library is byte-for-byte
identical to the original APK, checked by SHA-256. There is no Activity subclass,
recreation backdoor or model mutation hook. This variant is limited to the current
resource-free pure-native gallery. It remains installed after the test; reinstall
the ordinary cargo-apk package to restore production packaging.

The latest local result records PID 4159, four native host generations, four
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

Insets currently poll at approximately 1 Hz. Keyboard visibility is queried on
Android API 30+; edge occlusion requires Android/target SDK 35+.
An event-driven inset listener, older OS/device validation and animation-frame
tracking remain work items. The app-facing display API now exposes sampled
keyboard visibility and optional IME edge occlusion; see [display metrics](display.md). Lifecycle notifications do not yet serialize Android
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

## Android native color precision

The unpatched pinned wgpu-hal 27.0.4 backend selected R5G6B5_UNORM on the
API 37 / x86_64 emulator even with an Rgba8Unorm render target. The requested
RGB (161, 75, 50) appeared as (165, 77, 49). The framework now carries a reviewed
Android EGL configuration patch requesting at least eight bits per RGBA channel.
Its source archive, original file hashes, exact diff and licenses are recorded;
see [renderer patch maintenance and consumer setup](renderer-patches.md).

Current debug diagnostics report `native_format=R8G8B8A8_UNORM` and
`target_format=Rgba8Unorm`. The shared interaction probe now requires exact solid
blue, green and orange interior samples. The former RGB565 locator alternative
has been removed; this test fails if quantization returns. Existing text-glyph
comparisons remain exact at the documented threshold. Desktop and API 37 Android
runs pass the stronger probe, including normal input and restoration.

Three same-process activity recreations and final destruction pass with four
responsive native hosts; all four recorded native surfaces are RGBA8. The test's
instrumentable-packaging qualification still applies. The API 35 CI recreation
job also checks native format diagnostics, restores the ordinary APK, and runs
the complete interaction/exact-color probe with measured density and insets. This is scoped evidence against the
specific RGB565 loss, not complete color-management, HDR, gradient or alpha tests.
Physical-device, GPU/OS matrix and macOS runtime coverage remain outstanding.

Cargo only applies patches from a consuming workspace root. Generated apps now
include the renderer override at the same source revision as the framework;
manual/existing apps must add it explicitly. The selection verifier checks the
resolved HAL source, and Android rejects an RGB565 native surface with an
explanation of the required patch. The renderer continues using wgpu.


## App-facing keyboard geometry

The public `display::view_metrics` API exposes effective scale, native density,
viewport bounds, safe areas, sampled keyboard visibility and optional occlusion.
Headless tests cover zoom/density separation, unknown measurements, scoped
viewports, rejection of stale resize geometry and overlapping-inset arithmetic.
The ordinary Android interaction test now checks public API diagnostics against
the Window Manager IME source frame, along with stable bottom padding before and
after dismissal. The local API 37 / x86_64 run passed: 972 physical pixels of
IME occlusion matched the system frame, with stable 63-pixel bottom view padding;
dismissal returned occlusion to zero and padding to 63. This does not establish
IME composition/candidates, automatic
focused-editor scrolling or animation-frame synchronization.


## Ambient retained view dependencies

Every retained tree now provides host-owned `ViewMetrics` through the inherited
model mechanism. Components can watch all metrics or selected `ViewAspect`s.
Tests cover initial/non-listening reads, notification coalescing, stable IDs and
state, subtree overrides and their independent handles, unmount/remount, keyboard
visibility versus edge occlusion, repeated identical samples, and zoom versus
native density. Host values expose no mutable inherited handle.

The Linux X11/Xvfb rendered probe leaves the view component at one build across
keyed counter and notifier interactions, resizes the native window from
1000×600 to 900×650, and observes exactly one additional view build. Its screenshot
shows the new dimensions and build count. Existing exact-color/glyph interaction
checks continue to pass. This verifies a native resize on Linux; it does not
establish macOS window events or every Android configuration-change path.


## Retained SafeArea

Headless tests cover nested padding consumption, selective edges and minimums,
keyboard-covered versus stable bottom padding, child identity/state across inset
changes, invalid-layout recovery and actual egui pointer hit regions in a raw
viewport area. These tests use injected geometry; they are not new native-device
notch or rotation evidence. The default egui CentralPanel already consumes safe
areas, so adding a raw-view SafeArea there requires an explicitly consumed model.
The display guide records this current integration boundary.


## Persistent custom render boxes

Typed render-object tests cover one-time creation/attachment/disposal, identity
across keyed updates, type replacement with the same object type, ordered actual
render-child slots through components, inherited reconfiguration, paint-only
cache reuse, worker/layout-phase invalidations, initial/dynamic build failure
cleanup and layout-error recovery. The child protocol rejects malformed geometry,
invalid indices, missing/duplicate placement and nested reuse of an active layout
object. egui pointer tests check background-before-child painting and child input.

The native Linux gallery now uses a persistent custom container for its view
report. The rendered probe records one creation and one layout across unrelated
counter/notifier interactions, then one update and one additional layout after
resizing: widths 984 to 884 logical points, with a stable 28-point height. Existing
exact-color and glyph probes pass. The local API 37 / x86_64 Android probe passes with one creation,
configuration and layout for that container during ordinary component interaction
(395.4375 × 28 logical points).
These are scoped render-object checks, not complete RenderBox/PipelineOwner parity.
Full limitations and API usage are in [render objects](render-objects.md).


## Dry layout and intrinsic sizing

The pure query suite compares wet/dry flex results over axes, main-size policies,
alignment, fits, directions and finite/unbounded constraints. It checks intrinsic
flex contributions, padding deflation, aligned/constrained boxes, positioned-stack
exclusion, stepped intrinsic wrappers, unbounded stretched flex, tight-axis
shortcuts, invalid arguments/results, explicit unsupported callbacks, and retained
state/error recovery. A custom parent queries its child without satisfying the
wet layout/placement protocol. Retained queries flush inherited changes without
performing actual layout or invalidating a warm layout cache.

The Linux native resize probe records matching dry and allocated rectangles at
984×28 and 884×28 logical points, while retaining exactly one/two actual layouts.
Existing interaction, restoration and exact-color/glyph probes still pass. The
Android probe also checks a matching dry/allocated rectangle and one wet layout.
This is not text-shaping, baseline or complete intrinsic-compatibility evidence.


## Baseline geometry and mixed font sizes

The baseline suite covers both coordinate systems, flex ascent/descent grouping,
missing-baseline fallbacks, direction/fit/tight-constraint sweeps, shifted and
ignored baselines, first/highest propagation, positioned stacks, negative overflow
and gated hits, finite-data validation, custom child access, retained state/error
recovery and dry-query cache preservation. These are declared-metric tests.

The native gallery additionally uses the backend's actual galley glyph positions
for two ASCII font sizes. A pixel probe locates their distinct backgrounds and
reference line; it checks common physical baseline coordinates within two pixels.
The Linux X11 run passes with both text baselines and the line at y=429 after
resize. The local API 37 / x86_64 run places both text baselines at y≈1405 and
the reference line at y=1404.5, within the two-pixel tolerance. Existing exact-color/glyph interactions and dry/actual size checks remain
passing. This does not establish complete shaping, font fallback, ideographic
metrics or accessibility behavior; see [baseline scope](baselines.md).


## Retained paragraph verification

The workspace now passes 173 Rust tests, including ten paragraph tests. New
coverage includes UTF-8 formatted runs, wrapping/line limits and ellipsis,
constraints and overflow clips, intrinsic word/nonbreaking-segment sizing,
dry/actual baseline agreement, empty first lines, pixel-rounded alignment,
query/paint separation, recoverable invalid formats, density changes and font
replacement beneath a cached ancestor without widget remounts. The paragraph
Markdown example compiles and runs separately with rustdoc. Formatting, all-target
Clippy with warnings denied, documentation and renderer provenance/source checks
pass locally.

The Linux X11 smoke test passes after replacing the gallery's hand-measured text
leaves with retained Paragraph widgets. Both font baselines and the reference
line remain at physical y=429 after resize (two-pixel tolerance). Existing input,
restoration, asset, exact RGB, flex, keyed component, inherited-view and custom
render-object cache assertions also pass. Android and macOS results for this
change are recorded by the commit's CI run; no local Android paragraph runtime
claim is made here. Full shaping, bidi, fallback, selection and accessibility
remain outside this evidence; see [paragraph scope](paragraphs.md).


## Inherited typography verification

The workspace now passes 182 Rust tests, including nine new text-style tests.
They verify partial property overlay/replacement, nested span inheritance and
sibling isolation, clearing inherited effects, line-limit removal/overrides,
positive linear scaling, stable Text/Paragraph element IDs after live inherited
font/color updates, nested provider shadowing and equality gates, fallback and
empty content, recoverable invalid inherited geometry and scaling overflow.
Actual galley checks cover UTF-8 section ranges, colors, sizes and baselines.
Formatting, Clippy with warnings denied, generated documentation, both paragraph
and text-style Markdown examples, and renderer provenance/selection checks pass.

The Linux X11 smoke test passes using high-level Text with scoped defaults and
nested spans in the existing mixed-size fixture. Both rendered font baselines
and the reference line remain at physical y=429 after resize. Existing native
input/restoration/assets, exact-color/glyph, flex, component, inherited-view and
custom-render cache checks also pass. Cross-platform checks for this change are
recorded by its CI run. These tests do not demonstrate system accessibility
scaling or full Flutter typography; see [inherited text scope](text-styles.md).


## Decoration and foreground paint verification

The workspace passes 188 Rust tests, including six new decoration tests. New
coverage includes background/foreground ordering around opaque child painting,
nested RenderBox phase ordering and shared Ui identity/geometry/clip, legacy
single-callback behavior, sizing/intrinsic/baseline forwarding, invalid border
and child-count recovery, retained property updates, rounded shape metadata and
inside stroke policy. Formatting, all-target Clippy with warnings denied,
generated documentation and the decoration Markdown example pass locally.

The Linux X11 smoke test passes the added native foreground probe. A 24×24
logical-point foreground region renders exact RGB #834eb3 at (203,403) after
resize, while its opaque #e3bd42 child is absent from the screenshot. Existing
font boxes are now decorated backgrounds; both text baselines and the reference
line remain at physical y=429. Existing native interactions, restoration,
assets, exact RGB/glyph, layout, inherited-view and render-cache checks pass.
Android/macOS results for this revision are recorded by CI. This verifies solid
foreground order, not full decoration, clipping or compositing compatibility;
see [decoration scope](decorations.md).


## Frame-driven animation controller verification

The workspace passes 200 Rust tests. Eleven new integration tests and one
private registration-lifetime test cover automatic frame advancement,
forward/reverse duration and distance, directional status retention, point-to-point
curves, repeat phase/alternation/count/endpoints, indefinite repeat, muting and
elapsed-time catch-up, clamping/instant runs, invalid-command rollback, finite
interpolation across extreme bounds, completion futures/waker deduplication,
replacement/cancellation/disposal/drop, reentrant listeners, disposal guards,
context/viewport isolation, weak registration cleanup/reuse and retained builder
geometry updates. Formatting, warnings-denied all-target Clippy, generated
documentation and the controller Markdown example pass locally.

The Linux X11 native smoke test passes injected animation clicks: the purple
foreground region expands from 24 to 48 physical pixels and contracts to 24.
Intermediate controller values are recorded by actual retained widget builds,
and exact color/covered-child/baseline assertions still pass. Other input,
restoration, assets, layout, component, inherited-view and render-cache gates
also pass. Android/macOS results for this revision are recorded by CI. These
results establish native frame-driven progress and endpoints, not frame-rate,
jank, system accessibility, inherited ticker-mode or complete Flutter animation
compatibility; see [controller scope](animation-controllers.md).
