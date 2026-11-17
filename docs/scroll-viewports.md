# Retained single-child scrolling

`scrolling::SingleChildScrollView` connects an externally owned ScrollPosition to
a retained render box and the gesture arena. It eagerly lays out one child,
translates it by the current offset, and clips to its viewport. The default axis
is vertical; `.axis(Axis::Horizontal)` and `.reverse(true)` change orientation.

```rust
use rust_flutter::{egui, element::{ElementTree,Widget}, gestures::GestureBinding,
    layout::{Axis,BoxConstraints}, scrolling::*};
let ctx = egui::Context::default();
ctx.begin_pass(egui::RawInput {time:Some(0.0), ..Default::default()});
let controller = ScrollController::default();
let position = controller.create_position(&ctx)?;
let attachment = controller.attach(&position)?;
let binding = GestureBinding::new();
let view = SingleChildScrollView::new(&position,&binding)?
    .axis(Axis::Vertical)
    .into_widget(Widget::leaf(egui::vec2(200.0,600.0),|ui| {
        ui.painter().rect_filled(ui.max_rect(),0.0,egui::Color32::LIGHT_BLUE);
    }))?;
let mut tree = ElementTree::new(&ctx,view)?;
egui::CentralPanel::default().show(&ctx,|ui| {
    let mut frame=binding.begin_egui_frame(&ctx).unwrap();
    tree.show_in_gesture_frame(ui,BoxConstraints::tight(egui::vec2(200.0,100.0)).unwrap(),&mut frame).unwrap();
    assert!(frame.finish().unwrap().failures.is_empty());
});
let _=ctx.end_pass();
assert_eq!(position.dimensions().unwrap().max_scroll_extent(),500.0);
controller.jump_to(50.0)?;
let geometry=tree.layout(BoxConstraints::tight(egui::vec2(200.0,100.0))?)?;
assert_eq!(geometry.children[0].layout.children[0].offset.y,-50.0);
drop(tree);
drop(attachment);
position.dispose()?;
controller.dispose()?;
# Ok::<(),Box<dyn std::error::Error>>(())
```

## Layout and dimensions

The child receives the incoming cross-axis constraints and an unbounded scrolling
axis with zero minimum. The viewport constrains the resulting child size to its
own incoming constraints, including finite shrink wrapping when loose/unbounded.
Flexible children that require bounded main-axis space still cannot expand in an
unbounded scroll child. This is eager layout, not virtualization or a sliver list.

Content extent is `max(childLength - viewportLength, 0)`. The normal translation
is negative offset. Reverse aligns the trailing content edge at offset zero and
moves toward the leading edge as the offset increases; short content aligns to
the trailing edge. The render box always hard-clips. Its clip also gates child
hit testing. Cross-axis origin remains zero; directionality/RTL adaptation is
not automatic.

Dry sizing uses only child dry queries and does not publish dimensions, move
offsets or invoke wet layout. Intrinsic size queries return UnsupportedQuery, and
dry/actual viewport baselines are absent under the current policy. Offset changes
invalidate layout caches while preserving child elements. Parameter changes to
position, axis or reverse cancel active gesture sequences without replacing the
child, through GestureDetector's new `sequence_key` option.

Wet layout stages validated dimensions. Normally they publish at the end of the
owning UI pass, after render-object borrows have been released. The latest layout
wins, and unmounted deliveries are skipped even if another callback unmounts a
later delivery. A pointer down publishes the last measured dimensions before
starting input, also outside render-object borrows, so down/move in the first
pass can scroll. Dimension changes use the existing position policy: cancel
activity and clamp offset. Render geometry already uses the corrected range for
that frame. A superseded dimension delivery retries on a requested later frame.

One live viewport may claim a position. Mounting a second returns a layout error
rather than let two sizes overwrite each other's metrics. It can retry wet layout
after the first unmounts. Multiple controller clients should use distinct
positions and retain their own attachment tokens.

## Input and ownership

Use the same binding in the view and tree's gesture-aware show method. Shared
`begin_egui_frame` / `show_in_gesture_frame` / `finish` is recommended when several
roots or transformed layers are present. An opaque ancestor target competes with
child recognizers. Down cancels ongoing position activity; a winning pan begins
dragging, includes the threshold-crossing displacement, and captures subsequent
movement outside the viewport. Positive finger movement decreases normal offset;
reverse flips the sign. End supplies the axis component of effective fling
velocity to bounded fluid friction with drag 0.135. Cancel stops an active drag.

Uniform positive layer scale is sampled during paint and divides global deltas/
velocity before applying content motion. Shared frames paint before dispatch;
the older single-root adapter uses previously painted scale. Arbitrary transforms
and changing scale during capture are not a complete local-coordinate protocol.
Pan thresholds/classification remain the current radial/global-coordinate policy,
so cross-axis motion can claim a pan. Axis recognizers, edge handoff, nested-scroll
coordination and platform-specific motion policies remain pending.

The caller owns the position, attachment and binding. Unmount releases viewport
ownership, subscriptions and pending metrics; it does not dispose an externally
owned position. Gesture removal cancels captured input. An independently retained
position's programmatic/ballistic motion can continue; owners should cancel or
dispose it when hiding or retiring their view. The gallery cancels its scroll
activity when leaving Components. Automatic element/TickerMode ownership and
restoration are future work.

## Verification and remaining scope

Tests cover vertical/horizontal/reverse geometry, clipping/hit bounds, shrink
wrapping, pure queries, cached offset updates, resize correction, deferred
callback reentrancy, pending-delivery cleanup, exclusive ownership/retry, captured
pan, child-tap competition, parameter changes, first-pass input and scale-two
scene conversion across rebuilds. The native gallery adds a 60-point clipped
viewport with three 60-point stripes. Probes drag upward across the viewport's
edge, require different content within the same clip, and click a controller
reset to restore exact initial pixels. Existing spring, gesture, layout, glyph,
baseline, asset and restoration gates remain required.

This does not implement wheel/trackpad signal routing, scrollbars, keyboard/
accessibility scrolling, keep-alive/restoration, ensureVisible, nested scrolling,
lazy lists/grids/slivers, overscroll or platform feel. Reviewed
[SingleChildScrollView](https://api.flutter.dev/flutter/widgets/SingleChildScrollView-class.html)
and its [viewport source](https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/widgets/single_child_scroll_view.dart);
full member compatibility remains unproven. See also
[position ownership and motion](scroll-positions.md).
