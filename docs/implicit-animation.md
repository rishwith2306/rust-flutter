# Implicit value animations

`animation::TweenAnimationBuilder<T>` owns its controller and animates a value
when its target changes. Applications supply begin, target, duration and a widget
builder. They do not need to sample or dispose a controller. The underlying
controller inherits TickerMode and is retired with the element.

```rust
use rust_flutter::{egui, animation::TweenAnimationBuilder, element::{ElementTree, Widget},
    layout::BoxConstraints};
use std::{cell::Cell, rc::Rc, time::Duration};
let ctx = egui::Context::default();
ctx.begin_pass(egui::RawInput { time: Some(0.0), ..Default::default() });
let completions = Rc::new(Cell::new(0));
let log = completions.clone();
let animated = TweenAnimationBuilder::new(24.0_f32, 48.0, Duration::from_secs(1), |_, width| {
    Widget::leaf(egui::vec2(width, 24.0), |ui| {
        ui.painter().rect_filled(ui.max_rect(), 0.0, egui::Color32::LIGHT_BLUE);
    })
})?.on_end(move || log.set(log.get() + 1));
let mut tree = ElementTree::new(&ctx, animated.into_widget())?;
let _ = ctx.end_pass();
ctx.begin_pass(egui::RawInput { time: Some(0.5), ..Default::default() });
egui::CentralPanel::default().show(&ctx, |ui| {
    let response = tree.show(ui, BoxConstraints::loose(egui::vec2(200.0, 80.0)).unwrap()).unwrap();
    assert_eq!(response.rect.width(), 36.0);
});
let _ = ctx.end_pass();
ctx.begin_pass(egui::RawInput { time: Some(1.0), ..Default::default() });
assert_eq!(completions.get(), 0); // completion is delivered after the pass
let _ = ctx.end_pass();
assert_eq!(completions.get(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Retargeting and identity

Rebuild the same typed/keyed widget with a new target to start another animation.
Begin is used only on initial mount; a replacement begin is ignored. The new trip
starts from the currently sampled value, so changing targets during playback
preserves continuity. Matching T type/key retains the animation state and matching
child widgets keep their ordinary retained state. Changing the value type or key
creates a new animation owner.

Changing only the builder or on_end callback does not restart playback. If curve
or duration changes while active, this implementation retargets continuously from
the current sample to the same target using the new settings. At rest those
changes configure the next trip. This policy is explicit and is not an audited
copy of Flutter's ImplicitlyAnimatedWidget property-update scheduling.

`with_curve` selects one of the existing Curve variants. Duration covers the whole
new trip, rather than scaling with the target distance. Zero duration displays
the target immediately. Equal begin/target values and retargeting exactly to the
current value do not start a run and do not produce a completion event.

The builder receives the BuildContext and sampled T. It can compose ordinary
padding, constraints, decorations and custom widgets. It still needs to produce
valid geometry for its chosen value. A builder invocation is not a side-effect
hook: keep commands in interaction handlers or completion callbacks.

## Supported values

TweenValue is implemented for f32, f64, egui Vec2/Pos2/Rect/Color32/Stroke,
layout Insets, and BoxDecoration. Constructors validate both endpoints, even
when a later begin would be ignored. Numeric coordinates must be finite,
rectangles ordered, and inset/stroke widths nonnegative. Numeric interpolation
uses weighted wider intermediates for f32 to avoid overflow across extreme
finite endpoints. Exact endpoints, including curve values rounded to zero or one, are returned
without calling interpolation.

Colors interpolate the stored premultiplied RGBA bytes, rounding to u8. This is
an encoded-byte policy, not linear-light/perceptual/HDR color interpolation.
Decoration interpolation combines fill, border width/color and per-corner u8
radii; it retains the existing decoration's paint-only rounding policy.

Applications can implement TweenValue for their own Clone + PartialEq value
types. Implement interpolate for finite progress strictly between zero and one;
it must preserve valid values. Override validate when the type has invalid
representations. Store immutable endpoint snapshots; hidden mutation can defeat
target equality/change detection.

## Completion and lifetime

`on_end` runs after the owning viewport's UI pass. It is queued when a run
completes, including for zero-duration runs. It never runs synchronously inside
initialization or property reconciliation. Each queued completion captures the
callback configured when that completion occurred; subsequent property updates
do not replace already queued callbacks. Callbacks queued during delivery wait
for a later pass, preventing an immediate zero-duration chain from draining
forever inside one callback loop.

Canceling/retargeting an unfinished run does not call on_end. Unmount discards
undelivered callbacks, including if a completion callback itself removes its
owner. Callbacks run after registry/state borrows are released and may queue
parent state updates, retarget the tree or unmount it. Callback panics are
isolated; the ordinary Rust panic hook still runs. There is one completion driver
per Context with weak registrations, and removal releases queued callback
captures without requiring another frame.

TickerMode and explicit managed ownership use the [scoped ticker policy](ticker-mode.md).
Muted time still elapses and the first enabled frame catches up. Instantaneous
commands can still update the value while muted. The Context must stay on the
same UI thread and supplied frame times must be monotonic, as for controllers.

The Rust configuration does not expose or mutate a shared external Tween object.
It also does not yet supply Flutter's prebuilt child optimization or the complete
AnimatedContainer/AnimatedSize/AnimatedOpacity/transition widget catalog. Derived
animation objects, custom nonlinear curves, curve/duration policy parity,
accessibility animation policy and full member-level compatibility remain work.

## Verification

`tests/implicit_animation.rs` covers initial progression, exact endpoints,
continuous retargeting, ignored replacement begin, unchanged targets, builder
replacement, active curve/duration changes, inherited modes, retained child state,
zero-duration timing, callback snapshots, completion-driven unmount/retarget,
suppression after unmount, custom types, color/geometry/decoration interpolation,
invalid endpoints and callback panic isolation. A private test verifies weak
completion registration cleanup and reuse without another frame.

The gallery marker now uses this builder instead of an app-managed controller.
Linux/Android probes inject target-changing clicks, require settled expansion and
contraction, inspect a frozen intermediate size under TickerMode, re-enable and
check catch-up, and require four completion callbacks from actual native runs.
Existing exact-color, covered-child and baseline gates remain. These probes do
not establish a performance/jank budget or complete Flutter animation parity.

References: [TweenAnimationBuilder](https://api.flutter.dev/flutter/widgets/TweenAnimationBuilder-class.html),
[target and begin rules](https://api.flutter.dev/flutter/widgets/TweenAnimationBuilder/tween.html),
[implicit widget family](https://api.flutter.dev/flutter/widgets/ImplicitlyAnimatedWidget-class.html).
