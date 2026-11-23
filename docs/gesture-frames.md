# Multiple roots and arena-backed buttons

`GestureBinding::begin_egui_frame` collects the geometry of multiple ElementTrees
in one egui context/viewport. Show each tree with `show_in_gesture_frame`, then
call `finish` after all roots and backend areas have been shown. This dispatches
pointer-down using the complete front-to-back scene, rather than whichever root
was shown first. Existing captured sequences keep their routes across roots.

```rust
use rust_flutter::{egui, element::ElementTree, gestures::*, layout::BoxConstraints};
use std::{cell::Cell, rc::Rc};
let ctx = egui::Context::default();
let binding = GestureBinding::new();
let count = Rc::new(Cell::new(0));
let first = count.clone();
let second = count.clone();
let mut a = ElementTree::new(&ctx, GestureButton::new(&binding, "First",
    move || first.set(first.get() + 1)).into_widget())?;
let mut b = ElementTree::new(&ctx, GestureButton::new(&binding, "Second",
    move || second.set(second.get() + 1)).into_widget())?;
let _ = ctx.run(egui::RawInput::default(), |ctx| {
    let mut frame = binding.begin_egui_frame(ctx).unwrap();
    egui::CentralPanel::default().show(ctx, |ui| {
        let c = BoxConstraints::loose(egui::vec2(160.0, 40.0)).unwrap();
        a.show_in_gesture_frame(ui, c, &mut frame).unwrap();
        b.show_in_gesture_frame(ui, c, &mut frame).unwrap();
    });
    let report = frame.finish().unwrap();
    assert!(report.failures.is_empty());
});
assert_eq!(binding.target_count(), 2);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Scene ordering, capture and lifetime

Show roots in paint order within each egui layer. The last root in that layer is
tested first. Area/Window layer order and modal interaction gates come from
egui's layer registry (its modal gate reflects the prior frame's modal layer). A bare custom Ui layer is inserted into that paint-order
registry on first use; existing layers keep their stacking positions. Unrelated
backend areas above the scene occlude roots beneath them. Pointer capture still
continues when the pointer moves outside the initial root or over another root.

`LayoutBox::hit_test_result` exposes both the ordered path and `absorbed` flag.
An opaque hit stops searching roots behind it. Translucent entries can join the
same arena as a root behind them; an opaque descendant still blocks that root.
Disabled opaque roots block roots behind them without registering candidates.
Parent clips and root bounds use half-open right/bottom edges.

The collector supports hit testing through finite positive, invertible uniform
egui layer scale/translation transforms. Unsupported/nonfinite, zero, negative
or numerically noninvertible transforms report `InvalidTransform`. Callbacks
still receive global logical samples; arbitrary render transforms and local
drag-detail conversion remain future work.

Targets under omitted, disabled or modal-blocked roots are canceled before input
dispatch, even when their ElementTree remains mounted. A previous winner cannot
complete a hidden button's tap on later up. If the host stops drawing a binding
entirely, call `cancel_all` or `dispose`; an empty completed frame can also remove
the old roots from the visible scene.

One collector can be open per binding. It must finish within its owning egui
frame and viewport. Duplicate root IDs, overlapping collectors and expired
collectors report errors. Dropping an unfinished collector releases its lease
and cancels capture rather than guessing an incomplete hit scene. Lease IDs are
checked for exhaustion before changing ownership.

Create and finish a collector on every egui pass. Repeated passes update geometry
and visibility without replaying raw input. `show_with_gestures` remains the
single-root convenience API. Do not mix it with a collector in the same frame;
changing between the two modes across frames cancels old capture. Different
viewports/contexts need separate bindings; a global multi-viewport dispatcher
is not implemented.

## GestureButton

`GestureButton` reuses egui button drawing, rich labels and focus. Its pointer
action joins the framework's tap arena. It uses a focusable, non-clicking backend
sense, so egui does not independently trigger the pointer action. A child button
competes with a parent detector's tap/pan, instead of invoking both pointer
callbacks. This is an opt-in bridge; ordinary egui controls are unchanged.

The builder supports `.size`, `.fill`, `.enabled`, and optional AccessKit
accessibility role/toggled-state overrides for semantic controls. Default preferred size is
80×24 logical points; constraints can override it. Nonfinite/negative preferred
sizes fail layout. Dry/intrinsic queries are pure and baselines are currently
unknown. This is backend-style button rendering, not Material or Cupertino.

Callback-only rebuilds preserve an active tap and use the latest callback.
Disabling the button cancels its candidates. Pointer activation requests focus;
plain Enter/Space activates the focused enabled button once per egui frame,
including across repeated passes. Keyboard actions are independent of pointer
arenas. Press feedback requests repaint; component changes should use queued
StateHandle updates, as elsewhere in the retained tree.

The backend widget ID is tied to the retained element, so focus and desktop
accessibility actions survive moving the same tree between parent Ui scopes.
On desktop, the button exposes and consumes the egui/AccessKit Click action once
per frame and can override the node role and toggled value for controls such as
Material Switch. A headless action-request test verifies that bridge; actual macOS
VoiceOver interaction and the complete semantics pipeline remain unverified.
The Android host still lacks TalkBack/semantics integration. This button does
not make other backend controls or backend window dragging join its arena.

The gallery uses one collector for its observable-counter and typography/marker
roots. The counter and ticker toggle now use GestureButton. Native tests require
two collected roots, three registered targets, taps/pan through capture, exact
colors/glyphs, paused animation behavior and working interaction after dragging.

The integration suite covers root paint order, cross-root translucent competition,
capture, hidden/disabled/modal roots, backend occlusion, transformed layers,
collector errors/expiry, mode changes, button/parent arbitration, latest callbacks,
keyboard multipass activation, desktop Click action and pure sizing. The public
kernel and broader remaining gesture work are documented in
[binding](gesture-binding.md) and [recognizers](gesture-recognizers.md).

Implementation references: the pinned [egui 0.33.3 source](https://github.com/emilk/egui/blob/0.33.3/crates/egui/src/context.rs), `Sense`, `Response`,
`Context::layer_id_at`, `Memory::layer_ids` and AccessKit action APIs in the
installed primary source. No complete Flutter/egui member compatibility is claimed.
