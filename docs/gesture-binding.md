# Hit paths, shared input and retained gesture detectors

For multiple roots and opt-in backend button participation, see
[shared gesture frames and GestureButton](gesture-frames.md). The single-root
convenience method described here remains available.

`GestureDetector` registers a retained tap/pan target with a `GestureBinding`.
Use `ElementTree::show_with_gestures` to lay out a root, dispatch its normalized
primary-pointer input, then paint. Nested targets join the same arena in hit-path
order. The gallery uses this public path; its former private pointer host has
been removed.

```rust
use rust_flutter::{egui, element::{ElementTree, Widget}, gestures::*, layout::*};
use std::{cell::Cell, rc::Rc};
let ctx = egui::Context::default();
let binding = GestureBinding::new();
let count = Rc::new(Cell::new(0));
let taps = count.clone();
let detector = GestureDetector::new(&binding, GestureTargetConfig {
    tap: Some((TapConfig::default(), TapCallbacks {
        on_tap: Some(Rc::new(move |_| taps.set(taps.get() + 1))),
        ..Default::default()
    })),
    ..Default::default()
})?;
let mut tree = ElementTree::new(&ctx, detector.into_widget(
    Widget::leaf(egui::vec2(80.0, 40.0), |ui| {
        ui.painter().rect_filled(ui.max_rect(), 4.0, egui::Color32::BLUE);
    })
))?;
let _ = ctx.run(egui::RawInput::default(), |ctx| {
    egui::CentralPanel::default().show(ctx, |ui| {
        tree.show_with_gestures(ui, BoxConstraints::loose(ui.available_size()).unwrap(),
            &binding).unwrap();
    });
});
assert_eq!(binding.target_count(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The detector passes child constraints, dry/intrinsic queries and baselines
through unchanged. It requires one child. Its default behavior defers to that
child; `.behavior(HitTestBehavior::Opaque)` receives anywhere in its rectangular
bounds, and `Translucent` can also contribute a hit without blocking a sibling.
Rebuilds that only change callbacks preserve active recognizers and use the
latest callbacks. Changing recognizer settings or enabling/disabling tap/pan
replaces the candidates and cancels the old tracking. Changing behavior affects
future down hit tests; existing input stays captured. Unmount unregisters the
target and cancels its recognizers, including winners still waiting for up.

## Geometry and dispatch

`LayoutBox::hit_test_path` returns deepest/frontmost entries before ancestors.
Each entry contains the layout ID, local position and translation from the hit
test's root. It walks reverse paint order. Bounds use half-open right/bottom
edges, and ancestors gate hits even when `Clip::None` permits visual overflow.
Nonfinite/outside positions miss. Hit testing uses current layout, without
requiring paint. `hit_test` remains a convenience for the first entry's ID.

Custom render boxes can set `BoxGeometry.hit_test_behavior`; `None` preserves
the existing `hit_test_self` flag. Deferred targets receive a hit when a child
blocks it. Opaque targets block objects behind them. Translucent targets add an
entry even without a child hit, letting the stack continue behind them. An opaque
descendant of a translucent target still blocks siblings behind that subtree.
This follows the reviewed proxy-box hit-test algorithm; transforms currently
support translations only, and shape/rounded clipping is not implemented here.

For another host, call `binding.dispatch_layout(event, &geometry, root_origin)`
or `binding.dispatch(event, ordered_target_ids)`. Event positions and the origin
share global logical coordinates. Layout is consulted only for down; later
events use the recognizers' captured subscriptions, including outside the root
or its current clip. Callbacks receive global samples as documented in
[recognizer policies](gesture-recognizers.md); local transformed detail streams
remain future work. Direct dispatch supports independent pointer IDs; each
current tap/pan recognizer still tracks only one primary pointer at a time.

`register_target` is available for hosts without an ElementTree. Retain its
`GestureTarget` token. Drop/cancel unregisters only that target, preserving other
participants in its pointer arena. Tokens and weak binding handles do not retain
the binding. Use weak captures in callbacks to avoid ownership cycles.

Down snapshots target identities in the hit path, removes duplicate IDs, registers
all surviving candidates, routes the sample and then closes/flushes the arena.
Up routes before sweep/cleanup; cancel routes before rejecting the arena. Removing
or replacing a target skips stale queued down work. Events dispatched reentrantly
wait until the entire current route/arena/cleanup packet finishes, so a new down
with a reused pointer ID cannot be swept by its predecessor's up.

`DispatchReport` counts attempted packets and lists errors. A reentrant call
returns `queued = true`; the outer report contains the deferred results. Duplicate
down, missing down, old timestamps and invalid geometry are reported. Invalid
move/up geometry also terminates existing capture; cancellation tolerates missing
geometry/time. `cancel_all` queues one cancel for each live sequence and is
idempotent during reentrant cancellation. `dispose` is immediate, terminal and
cancels all targets, including already-resolved winners, and discards queued packets. Keep ownership of the
binding's arena/router with the binding; external service disposal makes further
dispatch terminal and cleans up tracking.

## egui host scope

Use one binding for one root in one egui context/viewport. Its adapter deduplicates
multipass input per frame, respects the parent clip for down, captures movement
outside the root, and cancels on focus/pointer loss, disable or other-button
changes. Reusing it for a different context/viewport or a second root in the same
frame reports an error. `ElementTree::show` retains its existing paint-only
behavior; pass the same binding used by the detector to `show_with_gestures`.
StateHandle callbacks queue updates and request repaint; the already-laid-out
frame is painted before those component updates are applied on a later flush.

The adapter normalizes egui's primary pointer as ID 0 with `Unknown` device kind.
It does not yet bind raw multitouch, hover, pointer signals or trackpad pan/zoom.
Ordinary backend egui controls continue to use their own input system and do not join
these arenas automatically. GestureButton is an opt-in arena-backed bridge. Combining such a control and a detector can therefore
produce independent callbacks. A shared GestureFrame now handles multiple roots in one viewport. Global
multi-viewport event binding, gesture teams,
the full recognizer catalog and semantics remain unfinished.

The test suite covers nested ordering, opaque/deferred/translucent hits, offsets,
capture, target replacement/unmount, latest callbacks, pointer reuse during up,
disposal, input protocol errors, weak lifetime, multipass delivery, parent clips,
focus loss and disabled roots. Native probes verify a registered detector and
active captured pointer when taps and pan start occur, alongside the existing
exact-pixel and post-drag interaction checks.

Reviewed references: [hit-test behavior](https://api.flutter.dev/flutter/rendering/HitTestBehavior.html),
[proxy-box hit-test algorithm](https://api.flutter.dev/flutter/rendering/RenderProxyBoxWithHitTestBehavior/hitTest.html),
[binding pointer dispatch](https://api.flutter.dev/flutter/gestures/GestureBinding/handlePointerEvent.html).
