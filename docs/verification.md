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
| Workspace tests | Passed, 420 tests | Navigation, signals/repaint, workers, animation, services, display insets, Fluent negotiation/plurals/errors, snapshots and corruption/concurrent writes, lifecycle/back/close, native destroy terminality and repaint deadlines, asset compiler/resolution/decoding/cache bounds, constraint layout/flex/stack/clip/hit testing, stable leaf IDs, element reconciliation/lifecycle/queued-update races, inherited scopes/predicates/lifetime/worker updates, model aspect unions/gates/nested fallback, notifier reentrancy/dispatch/disposal/weak subscriptions, equality/merged-source rollback, inherited notifier rebinding/coalescing/failures/builders, scaffold arguments and gallery restoration |
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


## Scoped ticker and owned animation lifetime verification

The workspace passes 208 Rust tests, including eight new scoped-ticker tests.
They cover nested mode AND, retained identity and elapsed-time catch-up,
mode refresh without owner rebuild, explicit/scoped mute composition,
unmanaged independence, multiple owned controllers and external clones,
retained-context rejection after unmount, failed mount cleanup, value/status
listener-triggered unmount with safe deferred cleanup, read-only mode and
geometry/intrinsic/baseline forwarding. Existing controller tests still pass.
Formatting, warnings-denied Clippy, generated documentation and both controller
and managed-ticker Markdown examples pass locally.

The strengthened Linux X11 native probe passes with a managed marker controller:
settled widths are 24→48→24 physical pixels, then a second run is muted at width
30 and remains exactly 30 beyond its two-second duration. Re-enabling catches up
to 48 and reverse returns to a settled 24. The probe polls for stable endpoints,
checks actual screenshots and retains intermediate-value, exact foreground color,
covered-child and baseline assertions. Other existing native interaction,
restoration, asset, layout, component, inherited-view and render-cache gates pass.
Android/macOS checks for this revision are recorded by CI. This does not prove
force-frame policy, context resync/adoption, every native host lifecycle or full
Flutter ticker compatibility; see [scoped ticker scope](ticker-mode.md).


## Implicit value animation verification

The workspace passes 223 Rust tests: thirteen new implicit-animation integration
tests and two private completion-driver tests. They cover initial progression,
continuous retargeting/ignored replacement begin, unchanged targets and builder
replacement, active curve/duration changes, modes and retained child state,
zero-duration post-pass completion, completion snapshots, completion-driven
unmount/retarget, suppression after unmount, custom value types, rounded curve
endpoint handling, typed geometry/color/decoration interpolation, endpoint
validation, panic isolation, cross-widget next-pass delivery and weak callback
registration cleanup/reuse. Formatting, warnings-denied Clippy, generated docs
and the implicit-animation Markdown example pass locally.

The native Linux X11 gallery now uses TweenAnimationBuilder instead of an
app-managed marker controller. Injected clicks reach settled widths 24→48→24,
then mode muting holds an intermediate width at 30 beyond the duration, unmuting
catches up to 48 and a final target change returns to 24. The trace contains four
actual post-pass completion callbacks. Existing intermediate-value, exact color,
covered-child, baseline, interaction/restoration/assets, layout, component,
inherited-view and render-cache gates pass. Android/macOS results for this
revision are recorded by CI. These tests do not establish full implicit-widget,
performance or member-level Flutter compatibility; see [implicit scope](implicit-animation.md).


## Gesture arena kernel verification

The workspace passes 237 Rust tests, including thirteen new arena integration
tests and an ID-exhaustion rollback test. Coverage includes eager/explicit
claims, deferred default timing and rejection, hold/release/sweep ordering,
claim withdrawal, stale entries/defaults after pointer reuse, reentrant rejection
and arena recreation, flush snapshot boundaries, disposal during committed
resolution, drop teardown, independent pointers, protocol errors and panic
isolation. Formatting, warnings-denied Clippy, generated documentation and the
arena Markdown example pass locally.

The native Linux X11 probe injects taps and a real pointer drag through the
public kernel using the gallery's raw primary-pointer adapter. Six tap winners
and one drag winner are recorded. The drag rejects its tap competitor, paints
exact #d2618d feedback and leaves the marker unexpanded. Subsequent taps still
expand and contract it, checking pointer reuse and continued interaction. Six
implicit completion callbacks are recorded. Existing mode pause/catch-up,
exact-color, covered-child, baseline, input/restoration/assets, layout, component,
inherited-view and render-cache gates also pass. Android/macOS checks for this
revision are recorded by CI. This evidence covers arbitration and the prototype
adapter, not a general widget input binding or complete recognizer/gesture
compatibility; see [arena scope](gesture-arena.md).

## Explicit pointer routing verification

The workspace passes 246 Rust tests. Eight new integration tests and one private
ID-exhaustion test cover per-pointer/global dispatch order, nested dispatch,
registration and removal during callbacks, pointer reuse, disposal, panic
isolation, owned-token destruction and weak lifetime. Formatting, warnings-denied
Clippy, generated documentation and the pointer-routing Markdown example pass
locally. Both renderer provenance and dependency-selection checks pass.

The gallery now tracks its primary-pointer movement through the public router,
retaining an owned subscription until up or cancellation. The native Linux X11
probe passes with six tap winners, one drag winner, rejected tap on drag, exact
pink feedback and subsequent tap interaction. Existing animation, ticker mode,
foreground decoration, baseline, layout, restoration and component gates pass.
Android and macOS checks for this revision are recorded separately by CI. This
does not establish general widget hit-path dispatch, a recognizer catalog,
multitouch normalization or full Flutter pointer compatibility; see
[routing scope](pointer-routing.md).

The published router revision `ae4e8c0` passed all four jobs in
[CI run 44062494314](https://github.com/rishwith2306/rust-flutter/actions/runs/44062494314):
Linux/macOS checks, Android ARM compilation/Clippy and Android emulator
recreation plus ordinary-package rendered interaction.

## Tap/pan recognizer verification

The workspace passes 266 Rust tests, including nineteen recognizer integration
tests and one private sequence-ID exhaustion test. They cover tap/pan
competition, threshold boundaries, pre/post-accept tap slop, early arena wins,
pending claims across up/close, start-position policies and accumulated movement,
independent/ignored extra pointers, button/device changes, timestamp regression,
invalid geometry, registration rollback, arena rejection, cancellation/disposal,
final-handle drop, weak lifetime, callback panics and reentrant input/reuse.
Formatting, warnings-denied Clippy, generated documentation and the recognizer
Markdown example pass locally.

The gallery's private host now uses the public recognizers for gesture decisions.
Native Linux X11 tests pass with six tap completions, one pan start, rejected tap
on drag, exact feedback pixels and subsequent tap/return interaction. Existing
implicit animation, scoped ticker pause/catch-up, covered-child decoration,
baseline, layout, component, restoration and input gates remain green. macOS and
Android checks for this revision are recorded separately by CI. These results
do not establish a general widget input binding, complete gesture catalog,
device-dependent gesture policy or full Flutter semantics. See the documented
[recognizer policies and remaining work](gesture-recognizers.md).

The published recognizer revision `fcbe600` passed all four jobs in
[CI run 44064681593](https://github.com/rishwith2306/rust-flutter/actions/runs/44064681593),
including Android recreation and rendered interaction. Its downloaded Linux and
Android results each record six tap completions and one pan start, plus post-drag
interaction and the existing exact-pixel gates.

## Retained detector, hit paths and shared root binding

The workspace passes 287 Rust tests. Twenty new integration tests cover
deepest/frontmost path order, offsets and bounds, translucent/opaque/deferred
behavior, whole-path arena registration, captured movement, independent pointers,
target replacement/unmount, latest callbacks without gesture reset, stale queued
target identities, pointer reuse inside an up callback, disposal and weak
lifetime, invalid input/timestamp errors, idempotent reentrant cancellation,
egui multipass deduplication, parent clip (including excluded far edges), focus loss and disabled roots. A private
test checks target-ID exhaustion without partial registration. Formatting,
warnings-denied Clippy, generated documentation, both renderer checks and the
binding Markdown example pass locally.

The gallery no longer has a private primary-pointer adapter. Its marker is a
public retained GestureDetector under ElementTree::show_with_gestures. The native
Linux X11 probe passes with six taps and one pan callback that each observe one
registered binding target and one captured pointer. The drag rejects tap and
paints exact feedback; subsequent taps still work. Existing implicit animation,
scoped ticker pause/catch-up, decoration/covered-child, baseline, layout, component,
input/restoration/assets and render-cache gates pass. Android/macOS results for
this revision are recorded separately by CI.

This evidence covers one root/viewport's normalized primary pointer. Cross-root
binding, backend-control arena integration, raw multitouch/signals, local
transforms, complete gesture/member semantics and accessibility remain open;
see [binding scope](gesture-binding.md).

The published retained-detector revision `b5cf8fa` passed all four jobs in
[CI run 44069893038](https://github.com/rishwith2306/rust-flutter/actions/runs/44069893038).
Downloaded Linux/Android artifacts each confirm six taps, one pan, drag feedback
and post-drag interaction through its single-root binding. Android recreation
also passed.

## Shared viewport frames and arena-backed buttons

The workspace passes 305 Rust tests. Seventeen new integration tests and a
private lease-exhaustion test cover complete-frame root collection, paint/layer
order, translucent competition across roots, capture without retargeting,
omitted/disabled/modal-blocked roots, backend area occlusion, clip edges,
uniform layer transforms and invalid transforms, duplicate/expired collectors,
abandoned leases, input-mode changes, button/parent tap and pan arbitration,
latest callbacks, pure sizing, disabled buttons, keyboard activation across
multipass and desktop AccessKit Click actions. The accessibility-action test
also moves the retained button between parent Ui scopes, checking stable IDs.
The modal test follows egui's documented prior-frame modal gate; it is not proof
of a full framework modal-route implementation.

Formatting, warnings-denied Clippy, generated documentation, renderer provenance
and selection, and the shared-frame Markdown example pass locally. The gallery
collects its observable-counter and typography/marker roots together. Its counter
and ticker toggle use GestureButton. The native Linux X11 probe passes and records
two roots, three targets, six marker taps, one pan and seventeen processed shared
frames. Counter glyph/state changes, exact fills, ticker pause/catch-up, covered-child
painting, baseline, layout, retained components, restoration/assets and input
gates remain green. The gallery cancels capture when its host view is hidden.
Android and macOS checks for this revision are recorded separately by CI.

This is one viewport's scene plus an opt-in egui-painted button bridge. Other
backend controls/window dragging remain independent; global multi-viewport input,
raw multitouch/signals, full gesture/widget catalogs, Material/Cupertino,
semantics/TalkBack and native VoiceOver validation remain unfinished. See
[frame and button scope](gesture-frames.md).

### Android resume readiness follow-up

The first shared-frame CI run (`c7fa5f3`,
[44077536483](https://github.com/rishwith2306/rust-flutter/actions/runs/44077536483))
passed Linux/macOS checks and ARM compilation. Android activity recreation passed,
but the ordinary home counter remained at 2 when the background/resume probe
expected 3. This occurred before the component/gesture probe ran. Its missing
post-failure log limits causal diagnosis.

Review found a concrete readiness defect: `start()` could accept a presentation
record from before that resume, followed by a fixed one-second delay. The probe
now confirms a fresh pause after Home, then a new resume and the latest focused
GPU presentation at the expected density/insets. Debug-only diagnostics re-emit
presentation geometry after resume/surface/focus changes and include focus;
they do not alter application input state. Lost focus, pause or surface loss
invalidates an earlier ready record. Eight GPU-free parser regression tests
pass, alongside four IME tests, 305 Rust tests and local Android Clippy. Startup
and failed counter assertions now save trace/screenshot diagnostics. Exact
counter, restoration, pixel/glyph and gesture assertions remain required.
The follow-up revision `3c8d8ea` passed all four jobs in
[run 44080879231](https://github.com/rishwith2306/rust-flutter/actions/runs/44080879231),
including ordinary Android input/restoration and three same-process activity
recreations. This establishes that run passed; it does not prove the entire
cause of the earlier missed tap.


## Bounded velocity estimates and pan release details

The workspace passes 321 tests, including 16 new analytic velocity/release tests.
They cover linear and quadratic motion, finite numerical limits, equal/older
sample times, bounded history, pause detection, strict fling thresholds and
clamping, invalid configuration rollback, and release timestamps preserved
through delayed arena acceptance. The guide's executable Rust example passes.
Formatting, workspace Clippy with warnings denied, Rustdoc and both renderer
provenance/selection checks pass locally.

The Linux Xvfb native probe passes exact cyan release-feedback and pink
start-feedback pixels, rejects tap during drag, and accepts the next tap. Its
release records a stopped estimate with zero effective velocity and 56.134081 ms
sample age, consistent with the 40 ms stop policy. All counter, glyph, animation,
layout, asset and restoration checks remain required and pass. This verifies
release delivery and valid metadata, not precise physical fling measurements.
Revision `fabc359` passed all four jobs in
[run 44110913174](https://github.com/rishwith2306/rust-flutter/actions/runs/44110913174):
Linux rendered interactions, macOS build/tests, ARM Android compilation and
ordinary Android interactions plus three same-process activity recreations.
The Linux CI artifact independently records the release feedback and a stopped
estimate aged 52.227862 ms. This does not establish native fling accuracy or
macOS window/input/VoiceOver runtime behavior.

This adds release data, not scrolling or inertial simulations. The egui adapter
still timestamps at frame time; raw device history, platform-specific strategies,
axis/local-coordinate details and full Flutter gesture compatibility remain
unfinished. See [velocity scope and API migration](gesture-velocity.md).


## Pure friction and spring simulations

335 workspace tests pass locally, including 14 new physics tests. Analytic
friction/harmonic/critical solutions and differential-equation residuals cover
both motion directions and underdamped/critical/overdamped springs. Additional
checks cover near-critical continuity, tiny/huge elapsed times, pure query order,
object-safe dispatch, exact initial conditions, contact bounds, strict tolerance,
optional snapping and invalid/unrepresentable inputs. The guide's executable
Rust example passes. Workspace Clippy with warnings denied, Rustdoc, formatting
and both renderer provenance/selection verifiers pass locally.

These are CPU mathematical checks. No new native scrolling/physics runtime
claim is established: controller driving, scroll view integration, platform
overscroll policies and the remaining simulation family are unfinished. Existing
rendered gesture evidence belongs to the separately recorded velocity revision.
See [physics models and limits](physics.md).


### Physics foundation CI follow-up

Revision `5694911` passed all four jobs in
[run 44112454142](https://github.com/rishwith2306/rust-flutter/actions/runs/44112454142).
The downloaded Android ordinary-interaction and recreation result artifacts both
record success. Linux rendered checks, macOS build/tests and ARM compilation also
pass. This remains build/math/ordinary-gallery evidence, not a native scroll
simulation or macOS window/VoiceOver runtime claim.

## Controller simulation driving

351 workspace tests pass, including 16 new custom-simulation tests. They cover
analytic spring progress/directional completion, finite clamping without early
completion, initially done models, failed/panicking initial validation rollback,
late error/nonfinite/panic outcomes, future wake/resolution, duplicate times,
muting/catch-up, commands/disposal during sampling, superseded starts, nested
custom replacement, destructor reentrancy, final-drop cancellation and listener
replacement. The executable simulation guide passes. Workspace and local
x86_64 Android Clippy with warnings denied, Rustdoc, formatting and both renderer
provenance/selection verifiers pass.

The Linux Xvfb native probe passes the new managed spring marker: exact orange
fill, 12→24→12 physical-pixel widths at density 1, unchanged muted geometry
(width 23), and 527 build samples containing intermediate active motion and a
settled endpoint. Existing purple implicit motion, shared-root gestures, release
feedback, counter glyphs, layout, foreground/baselines, assets and restoration
remain green. These establish the tested native spring path; Android/macOS
results for this revision are recorded separately by CI.

Sampling errors now resolve AnimationOutcome::Failed, extending exhaustive
matches. Models run outside controller borrows; obsolete samples cannot overwrite
a replacement. Bounds remain finite f32, and custom simulations use f64 samples.
Unbounded/velocity APIs, scroll integration, platform motion policy and the full
Flutter animation/scroll catalog remain unfinished. See
[simulation controller semantics and limits](simulation-animations.md).


### Controller simulation CI follow-up

Revision `92596e0` passed all four jobs in
[run 44114380123](https://github.com/rishwith2306/rust-flutter/actions/runs/44114380123),
including the native spring marker probes on Linux and the Android emulator,
macOS build/tests, ARM Android compilation and three same-process activity
recreations. This is not macOS window/Metal/VoiceOver runtime evidence.

## Scroll position and controller foundation

371 workspace tests pass, including 20 new scroll position/controller tests.
They cover finite dimension/offset validation and correction, separate pixel/
metric/activity notifications, bounds/unapplied drag distance, frame-driven
programmatic and bounded-fluid release motion, interruption/completion,
attachment ownership/duplicates, multi-client validation/clamping, reentrant
removal and command supersession, disposal guards and final-drop cancellation.
A retained ValueListenable builder changes measured geometry through automatic
scroll-position animation while preserving all element IDs.

Workspace and local x86_64 Android Clippy with warnings denied, Rustdoc,
formatting, the executable scroll-position guide and both renderer source/
selection checks pass. These are CPU state/motion and headless retained-layout
checks. No new native scroll-input/rendering claim is established. Retained
viewport layout/clipping, pointer/wheel adapters, platform scroll physics,
overscroll, restoration, lazy slivers and full member compatibility remain
unfinished. See [position semantics and ownership](scroll-positions.md).


### Scroll position CI follow-up

Revision `4782d17` passed all four jobs in
[run 44116996940](https://github.com/rishwith2306/rust-flutter/actions/runs/44116996940),
covering Linux rendered checks, macOS build/tests, ARM Android compilation and
ordinary Android interactions plus three same-process activity recreations.
This remains the pre-viewport revision; it does not establish a native scroll
viewport claim.


## Retained single-child viewport

389 workspace tests pass, including 18 new viewport tests. They cover eager
vertical/horizontal/reverse layout, finite shrink wrapping, hard clip/hit bounds,
pure dry queries and explicit intrinsic/baseline policy, cached offset updates,
resize correction, deferred metric callbacks, latest-layout/unmount delivery,
exclusive viewport ownership/retry, captured pan and child-tap competition,
position/axis/reverse capture cancellation with retained child IDs, first-pass
input and scale-two scene conversion across rebuilds. The executable viewport
guide passes. Workspace/local x86_64 Android Clippy with warnings denied,
Rustdoc, formatting and both renderer provenance/selection verifiers pass.

The Linux Xvfb native probe passes on the final viewport implementation. The
60-point viewport initially shows one exact blue stripe, with the other two
fully clipped. Injected captured drag records offsets 20→40→60→80 within a
maximum extent of 120; olive/coral pixels remain inside the same viewport.
A real controller-reset click returns offset 0 and the original exact blue
pixels. The shared scene now has three roots/four targets and records 23
processed frames, six marker taps and one marker pan, alongside scroll input.
Existing purple implicit motion, spring endpoints/muting, release feedback,
counter glyphs, layout, foreground/baselines, assets and restoration remain green.
Android/macOS results for this revision are recorded separately by CI.

Wet dimensions publish outside render-object borrows at pass end, or before
pointer-down handling after layout. Viewport ownership and pending callbacks
are released on unmount; externally owned positions retain their own lifetime.
Wheel/trackpad routing, axis-specific and nested input, scrollbars, semantics,
restoration, platform overscroll, lazy slivers and full member compatibility
remain unfinished. See [viewport geometry and scope](scroll-viewports.md).


### Linux display startup follow-up

The viewport revision `203d873` passed macOS build/tests, ARM compilation and
Android native viewport/interactions/recreation in
[run 44120328306](https://github.com/rishwith2306/rust-flutter/actions/runs/44120328306).
Linux failed at initial gallery launch with XOpenDisplayFailed, before any
rendered viewport assertion. Its only artifact was gallery.log; the old script
left Xvfb stderr in an unrecorded pipe, so the full cause is not established.

The harness now starts Xvfb with `-noreset`: short-lived xdotool clients can no
longer trigger a last-client server reset while the gallery connects. Xvfb's
own help documents this option. The harness also validates a live display
number, checks server exit during waits and records xvfb.log for failure
artifacts. This removes a plausible reset race; it is not proof of the whole
cause of the earlier failure. The local full native probe passes with this
harness, retaining every existing rendered/input/restoration assertion. A new
revision's CI will check the updated startup path.


### Persistent X display CI follow-up

Revision `c5d4a37` passed all four jobs in
[run 44121916695](https://github.com/rishwith2306/rust-flutter/actions/runs/44121916695),
including Linux native viewport checks with `-noreset` and ordinary Android
interactions/recreation. The earlier `203d873` Android artifacts independently
record a passed 158-physical-pixel viewport, captured touch scroll, controller
reset and hard clip. The startup follow-up establishes this run passed; it does
not retrospectively prove the entire cause of the old display-open failure.

## Immediate scroll-signal routing

406 workspace tests pass, including 17 new signal tests. They cover first-willing
resolution and validation, panic isolation, callback updates/disposal, expired hit
target identity, FIFO ordering with reentrant pointer packets, units/modifiers,
scale-aware points versus local line/page units, interruption, raw frame replay
suppression, event-time/stationary cursor tracking, single-root input, front/
disabled-root ordering, nested edge handoff and unfinished-frame cursor cleanup.
Workspace/local x86_64 Android Clippy with warnings denied, Rustdoc, formatting,
the executable signal guide and both renderer source/selection checks pass.

The Linux Xvfb native probe passes real X11 wheel input against the retained
viewport, reaching its content limit (120) and resetting to exact original blue
pixels. Debug reports record six claimed normalized signals across three frames;
this is not a wheel-notch calibration or device fidelity claim. The existing
touch-like captured drag/reset, clip, spring, pointer arena, glyph/layout/assets
and fresh-process restoration checks remain green. The shared scene records
three roots/four targets, 27 processed frames, six marker taps and one marker pan.

Android's host now emits a scroll packet's pointer location before its wheel
signal. This mapping compiles locally; physical Android wheel input and actual
macOS trackpad hardware remain unverified. Android's native probe continues to
verify touch scrolling, not wheel input. This revision's exact CI is recorded
separately. Generic signals share the binding queue but do not join the arena;
full backend/viewport-global arbitration, trackpad phases/device settings,
residual redistribution and full scrolling/accessibility/member parity remain
unfinished. See [signal semantics and limits](scroll-signals.md).


### Scroll signal CI follow-up

Revision `4807074` passed all four jobs in
[run 44124653229](https://github.com/rishwith2306/rust-flutter/actions/runs/44124653229):
Linux native wheel/viewport interactions, macOS build/tests, ARM Android compilation
and ordinary Android touch interactions plus three same-process recreations.
Downloaded Android artifacts independently record passed 158-physical-pixel
viewport touch/clip/reset checks and recreation. Their native_wheel field is
false: Android physical wheel and macOS trackpad hardware remain unverified.

## Axis drag recognition

420 workspace tests pass, including 14 new axis-drag tests. They cover strict
component slop and net reversal, cross-only movement, delta projection/raw event
preservation, both start behaviors and unrestricted pan compatibility, selected
fling speed/distance gates and clamping with raw two-dimensional estimate retention,
negative velocity, orthogonal/diagonal competition, named weak lifetime and
reentrant disposal, live binding configuration replacement and nested orthogonal
scroll viewports. The executable axis guide passes. Workspace/local x86_64
Android Clippy with warnings denied, Rustdoc, formatting and both renderer
provenance/selection checks pass.

The Linux Xvfb native probe passes the added horizontal-drag check over a vertical
viewport: its exact initial clipped blue pixels stay unchanged. The subsequent
vertical drag, controller reset, real wheel/reset, clipping, spring/implicit
motion, marker gestures, glyph/layout/assets and restoration gates remain green.
The shared scene records three roots/four targets, 33 processed frames, six
marker taps and one marker pan. Headless orthogonal recognizer/viewport tests
establish arena competition; the native cross-axis check establishes unchanged
pixels, not a device-specific angle/threshold fidelity claim. Android/macOS CI
for this revision is recorded separately.

PanConfig.axis and primary fields extend the experimental configuration/details
structs. The default remains unrestricted pan. Axis drags operate in global
logical coordinates and retain the existing wait-for-threshold policy after an
early arena win. Arbitrary local transforms, full start/detail families, device
settings, trackpad pan/zoom, multitouch, teams and the broader gesture/member
catalog remain unfinished. See [axis semantics and API migration](axis-drags.md).
