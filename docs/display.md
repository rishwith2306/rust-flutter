# View geometry and keyboard metrics

During an application's `build`, call `rust_flutter::display::view_metrics(ui.ctx())`.
It returns the current viewport rectangle, effective pixels per point (including
zoom), native density when the host supplies it, safe area, actual keyboard
visibility, and optional edge occlusion. Values are logical egui points unless a
field explicitly says otherwise.

```rust,ignore
let metrics = rust_flutter::display::view_metrics(ui.ctx());
let available = metrics.unobscured_rect().intersect(ui.max_rect());
// Use `available` to constrain a keyboard-sensitive panel or scroll viewport.
```

The framework reports geometry; it does not automatically resize every layout or
scroll a focused editor into view. `unobscured_rect` takes the maximum of safe-area
and keyboard insets on each edge, so the navigation bar is not counted twice.
It cannot represent an interior obstruction such as a floating keyboard.

`KeyboardVisibility` is `Unknown`, `Hidden`, or `Visible`. It reports the latest
platform observation rather than whether the application requested a keyboard.
A visible floating keyboard can have zero edge occlusion. `None` occlusion means
the host could not measure it; it must not be interpreted as measured zero.

When available, `ViewOcclusion` separates:

- `view_padding`: system-bar/cutout space, independent of their visibility;
- `view_insets`: edges fully obscured by the IME;
- `padding`: `max(view_padding - view_insets, 0)` on each edge.

This separation follows the geometry relationships in Flutter's
[viewInsets](https://api.flutter.dev/flutter/widgets/MediaQueryData/viewInsets.html)
and [padding](https://api.flutter.dev/flutter/widgets/MediaQueryData/padding.html).
The API is a geometry subset, not the complete Flutter MediaQuery API.

## Host coverage

Android reads public [WindowInsets APIs](https://developer.android.com/reference/android/view/WindowInsets)
on the Java UI thread, asynchronously at approximately one-second intervals.
API 30+ can report actual IME visibility. Occlusion currently requires the
enforced edge-to-edge host on Android **and target SDK 35+**, and matching decor
and native-surface dimensions. A resize mismatch suppresses geometry until a new
matching sample arrives. Polling does not track every keyboard animation frame.
Failed/unavailable samples clear prior observations when their callback completes.

Desktop and Android below API 30 leave keyboard visibility unknown. Hosts without
supported occlusion leave it `None`; they still expose the ordinary egui viewport
and host safe area. NativeActivity content bounds remain the legacy safe-area
fallback. Other soft-keyboard implementations, older Android geometry, physical
devices and floating keyboards require further validation.

Read inside a frame: before the first frame, egui's default bounds are not a
platform measurement. Measurements belong to the active viewport, not a global
primary screen. Calling the immediate `view_metrics` function does not create an inherited
dependency. Retained components can subscribe through the ambient model below.


## Retained component subscriptions

Every `ElementTree` supplies a read-only ambient `ViewMetrics` inherited model.
It refreshes from the active viewport before `update` and `flush` (also invoked
by `layout`/`show`). Apps must continue showing/flushing their tree each frame;
they do not need to recreate the widget description when host geometry changes.
No extra visible element or layout wrapper is inserted, so user root IDs and
component state remain stable.

Inside a component's `build`, subscribe through its `BuildContext`:

```rust,ignore
use rust_flutter::display::{ViewAspect, ViewMetrics};

let view = ctx
    .depend_on_aspect::<ViewMetrics>(ViewAspect::Size)?
    .expect("an ElementTree supplies ambient view metrics");
let width = view.viewport.width();
```

`depend_on::<ViewMetrics>()` rebuilds on any metric change. `depend_on_aspect`
accumulates selected dependencies: size, orientation, effective scale, native
density, viewport rectangle, safe area, keyboard visibility, each occlusion
inset group, the whole occlusion record, or the computed unobscured rectangle.
Only matching changes trigger `did_change_dependencies` and a component build;
multiple changes coalesce into one pass. Repeated identical samples stay quiet.
Orientation uses viewport dimensions, with squares treated as portrait.

Like other inherited snapshots, the returned value is immutable. A component
watching only size can retain older keyboard data until it rebuilds; observe all
the fields used by the component. `read::<ViewMetrics>()` does not subscribe and
is permitted during `init_state`; listening during initialization remains invalid.
A failed/unavailable host sample is a change to unknown, not a measured zero.

An explicit `Widget::inherited_model(custom_metrics, child)` shadows the ambient
model for a subtree, including previews and tests. This explicit provider has the
ordinary inherited update handle. The host-owned ambient provider exposes no
mutable handle. All subscription lifetime and unmount checks are unchanged.

This implements selective geometry dependencies described by Flutter's
[MediaQuery](https://api.flutter.dev/flutter/widgets/MediaQuery-class.html).
Text scaling, accessibility preferences, display features, remove-padding widgets,
and the rest of MediaQuery's API remain pending. The gallery's component page
includes a retained view report; resize its window to see its build count change
without rebuilding it for unrelated counter/notifier updates.
