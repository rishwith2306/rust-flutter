# Android OS scroll-input verification

The Android runtime pipeline now includes mouse-wheel input against the ordinary
cargo-apk gallery package. A separate self-targeted instrumentation runner sends
OS MotionEvent packets; screenshots verify their effects through the public
native input and retained scroll path. No gallery model setter, Java Activity
subclass or native testing entrypoint is added.

The existing lifecycle phase still uses its separately documented instrumentable
gallery variant to test recreation. The orchestrator then reinstalls the original
pure-native APK before running ordinary interactions, including this wheel probe.
WheelInstrumentation targets the test package itself, not the gallery. It creates
no Activity and does not read or write the gallery's private state.

## Input and evidence

After touch-drag, cross-axis and controller-reset checks, the probe locates the
initial blue scroll viewport from rendered pixels. The runner injects four
SOURCE_MOUSE / ACTION_SCROLL packets at its physical screen center, each with
AXIS_VSCROLL=-1. It sends no preceding hover event. This exercises the Android
host's current-pointer-position emission on a scroll event, rather than relying
on a stale touch/hover location. Packets are separated by 100 ms; the delay is not
accepted as presentation evidence.

UiAutomation must accept every packet. The gallery PID must remain the same
before/after injection, while the test runner PID must differ. The result parser
requires a unique successful terminal code/summary, exact packet count, expected
event type/direction, finite coordinates and valid process identities. Six
GPU-free regression tests reject canceled/ambiguous results, bad counts/types,
wrong event descriptions, restarts and same-process injection.

Acceptance alone does not pass the test. The shared rendered probe requires the
initial blue stripe to disappear and later content to appear, then clicks the
controller reset and requires the exact original blue pixels. The existing clip,
touch/cross-axis drag, spring, gesture, glyph, layout, RGB888, assets, restoration
and recreation assertions remain mandatory.

Artifacts include `android-wheel-1-instrumentation.txt`, wheel/reset screenshots,
the normalized native trace, and `android-result.json` fields `wheel_injections`
and `components.scroll_viewport.native_wheel`. A result without the optional
wheel runner remains explicitly false for native_wheel.

## Run and scope

Use an isolated AVD named RustFlutterSmoke, a freshly built debug gallery APK and
the SDK tools described in [platform setup](platforms.md):

```sh
ANDROID_HOME=/path/to/Android/Sdk python3 scripts/android-runtime-smoke.py --serial emulator-5554
```

The orchestrator builds/installs the runtime test APK through android-recreation.py,
restores ordinary gallery packaging, and enables `--wheel-instrumentation` in
android-smoke.py. The wheel option rejects other AVD names and requires the test
package to be installed. The ordinary smoke test resets only the gallery's data;
never point it at a personal emulator profile. The full pipeline's recreation
phase also preserves and compares every native library byte in its test variant.

Local pre-device checks compile the Java fixture against API 35, convert it with
D8 for min API 24, package both instrumentation declarations with AAPT, and run
the wheel/IME/readiness parser tests. Native success is recorded only after the
emulator pipeline and rendered artifacts pass. An implemented or compiled runner
does not by itself establish Android wheel behavior.

This is OS-injected emulator mouse input, not a physical mouse, touchpad or
device-fidelity certification. macOS trackpad hardware, Android vendor-specific
mouse behavior, pan/zoom phases, accessibility scrolling and full Flutter parity
remain unfinished. The first runtime result for this probe is tracked in
[verification](verification.md).

Reviewed Android references:
[UiAutomation.injectInputEvent](https://developer.android.com/reference/android/app/UiAutomation#injectInputEvent(android.view.InputEvent,%20boolean))
and [MotionEvent.ACTION_SCROLL](https://developer.android.com/reference/android/view/MotionEvent#ACTION_SCROLL).
