# Retained scrollbars

`scrolling::Scrollbar` overlays a track and proportional thumb on a retained
viewport. Use the same externally owned position and gesture binding for both.
The bar observes pixel and dimension changes and keeps the content's layout size,
elements and baseline. It does not create or attach another scroll position.

```rust
use rust_flutter::{egui, element::Widget, gestures::GestureBinding,
    layout::Axis, scrolling::{ScrollPosition, Scrollbar, SingleChildScrollView}};
let ctx = egui::Context::default();
let position = ScrollPosition::new(&ctx, 0.0)?;
let binding = GestureBinding::new();
let viewport = SingleChildScrollView::new(&position, &binding)?
    .axis(Axis::Vertical)
    .into_widget(Widget::leaf(egui::vec2(200.0, 600.0), |ui| {
        ui.painter().rect_filled(ui.max_rect(), 0.0, egui::Color32::LIGHT_BLUE);
    }))?;
let scrollable = Scrollbar::new(&position, &binding)?
    .axis(Axis::Vertical)
    .into_widget(viewport)?;
# let _ = scrollable;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Mount and paint `scrollable` with the binding as shown in the
[viewport guide](scroll-viewports.md). Set `.axis(Axis::Horizontal)` and
`.reverse(true)` on both descriptions when required. The bar's initial axis
comes from dimensions already available at construction, or defaults to vertical;
it does not automatically follow later axis changes. An axis mismatch hides the
bar until configuration and dimensions agree. Reverse puts the minimum offset
at the far end of the track and reverses input direction.

The track overlays the right edge for vertical scrolling or bottom edge for
horizontal scrolling. `ScrollbarStyle` defaults to thickness 8, margin 2 and
minimum thumb length 18 logical points, with solid track/thumb colors. `.style()`
validates finite positive thickness and minimum length, plus finite nonnegative
margin. The thumb length is the visible fraction of total extent, clamped to
its minimum and the available track length. Track travel maps the full
minimum-to-maximum scroll range, including nonzero/negative minimum extents.
Dimensions, viewport length and available track space must permit scrolling;
otherwise the bar has no painted or hit-test area. The first measured dimensions
arrive after layout, so the bar can appear on the following requested frame.

Dragging the thumb uses the selected axis's arena recognizer. It competes ahead
of the content pan, captures movement outside the viewport and converts positive
uniform layer scale back to logical track motion. Down stops ongoing motion;
acceptance enters `ScrollActivity::Drag`. Updates clamp to the content range;
release/cancel returns an owned drag to Idle without a fling. Command identities
prevent later pointer updates or release from overwriting/canceling a newer
application command, including one issued by a synchronous listener. Rebuilds
preserve ownership; disabling interaction or unmounting releases the recognizer.
A tap outside the thumb pages 80% of the viewport toward the tap, with an
immediate clamped jump. Tapping the thumb itself does not move content.

Both track and thumb targets are translucent to hit testing. Wheel signals can
fall through to the underlying viewport in the same binding; they do not require
a separate backend scroll area. `.interactive(false)` removes both scrollbar
recognizers while keeping the indicator and underlying content input. Disabled
UI does not handle scrollbar input. The current interactive region is the
painted track/thumb bounds, without touch-specific hit expansion.

Unmounting drops subscriptions and recognizers, not the caller's position or
binding. Pure dry queries delegate to the content without publishing metrics or
running wet layout. The scrollbar hard-clips its overlay and content.

This is a retained scrollbar foundation, not the complete Flutter RawScrollbar,
Material Scrollbar or CupertinoScrollbar API. Fade/hover animations and cursors,
platform styling, RTL/leading-edge placement, automatic controller/notification
selection, expanded touch targets, long-press policies, overscroll thumb
compression, platform release inertia and accessibility semantics remain
unfinished. Layout invalidation is currently coarse; this is not a retained
paint-boundary optimization or a performance claim.

Nineteen headless tests cover proportional/minimum geometry, offsets, resize,
style changes, both axes/reverse, scaled layers, thumb/content competition,
capture, track pages, wheel fallthrough, noninteractive/disabled behavior,
rebuild/unmount, command supersession and reentrant layout queries. Native probes
require a rendered thumb/track, thumb-driven final content, a page-sized move,
wheel-over-thumb movement and exact reset of both content and thumb. Linux X11 and API 35 / x86_64 Android emulator artifacts pass these gates at
revision `48a8209`; see the [exact CI runtime record](verification.md#scrollbar-ci-runtime-result).
Physical devices and macOS runtime remain unverified.

Reviewed the official [RawScrollbar API](https://api.flutter.dev/flutter/widgets/RawScrollbar-class.html)
and its [gesture/painting implementation](https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/widgets/scrollbar.dart).
Complete member compatibility remains unproven.
