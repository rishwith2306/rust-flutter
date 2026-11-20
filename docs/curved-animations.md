# Derived animation values and cubic easing

`animation::Animation<T>` extends `ValueListenable` with status subscriptions and
direction helpers. The existing controller implements it. `CurvedAnimation`
applies a checked `ParametricCurve` to any scalar Animation parent, so derived
sources can be chained and supplied to ordinary observable widget builders.
Its parent must produce normalized progress in 0…1; curve output can overshoot.

```rust
use rust_flutter::{egui, animation::{AnimationConfig, AnimationController,
    Cubic, Curve, CurvedAnimation}, notifier::ValueListenable};
use std::{rc::Rc, time::Duration};
let ctx = egui::Context::default();
ctx.begin_pass(egui::RawInput { time: Some(0.0), ..Default::default() });
let parent = AnimationController::new(&ctx, AnimationConfig {
    duration: Duration::from_secs(1), ..Default::default()
})?;
let eased = CurvedAnimation::new(parent.clone(), Cubic::new(0.42, 0.0, 0.58, 1.0)?)?;
eased.set_reverse_curve(Some(Rc::new(Curve::EaseOut)))?;
parent.forward()?;
let _ = ctx.end_pass();
ctx.begin_pass(egui::RawInput { time: Some(0.25), ..Default::default() });
assert!((eased.try_value()? - 0.1291619).abs() < 0.00001);
assert_eq!(ValueListenable::value(&eased), eased.value());
let _ = ctx.end_pass();
eased.dispose()?;
parent.dispose()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`Cubic::new(x1, y1, x2, y2)` creates a cubic Bézier from (0,0) to (1,1).
It solves the x coordinate for the supplied progress before evaluating y; merely
evaluating the y polynomial at progress would produce the wrong easing.
Finite x controls must lie in 0…1. Finite y controls may lie outside that range
and cause overshoot. A bounded bisection with f64 intermediates handles flat
slopes and subnormal f32 progress without an unbounded iteration loop.
`FlippedCurve(curve)` maps progress through `1 - curve(1 - progress)`.
The existing Curve enum implements the checked trait too; its original inherent
transform/clamping behavior remains unchanged.

An optional reverse curve is selected by the direction in which the current
trip began. Changing direction mid-flight keeps that curve, avoiding a sudden
jump between forward/reverse mappings. Completed or Dismissed resets the latch;
the next trip can choose the other curve. Starting from an already reversing
parent initializes the latch accordingly. Status remains the parent's status.
Status-derived helpers follow that status; a controller's own `is_animating`
override instead reports its actual active motion, including stop/muting policy.

`set_curve` and `set_reverse_curve` update shared configuration across clones;
they do not manufacture parent value/status notifications. Clones compare as the
same Listenable source. Each returned Subscription owns one parent registration.
Disposal cancels registrations created through the derived source and its
internal status watch without disposing the parent. It is safe during a derived
listener callback. Dropping the last derived source also cancels its remaining
registrations, even if caller tokens outlive it. Use `downgrade()` for callbacks
that need to refer back to the derived source without creating an ownership cycle.

Custom ParametricCurve implementations can return an AnimationError. The
`try_value()` getter propagates checked samples through parent chains, checks progress/output finiteness and exact endpoint
preservation, catches a panicking curve as CurvePanicked, and propagates errors.
It reports Superseded if curve evaluation replaces its configuration or changes
parent value/status, rejects recursive sampling of the same source as Superseded,
and reports Disposed if evaluation disposes it. Callbacks execute
without holding curve-configuration borrows. `value()` and the ValueListenable
getter panic on these invalid samples; use the checked getter when accepting
fallible application curves. Parent playback remains controlled by its owner;
evaluation errors leave its current commands in effect.

Exact endpoint validation is stricter than Flutter's debug near-endpoint check.
The checked Rust trait/ownership/disposal APIs are explicit differences. A parent
outside normalized progress is rejected when sampled; output overshoot is never
silently clamped. Supplying overshooting output to another normalized curve can
therefore fail. The legacy controller/implicit-builder Curve parameters still
accept their existing enum, rather than arbitrary ParametricCurve instances;
use a derived source with a value builder for cubic/custom curves.

Fifteen headless tests cover cubic reference samples and subnormal/flat cases, overshoot,
validation, reverse latching, parent/status events, configuration, subscriptions,
chaining, cleanup, reentrant sampling and existing controller-driven widgets.
The gallery's third animation marker uses distinct forward/reverse cubics and
the same element-owned parent/muting lifecycle. Native checks require its
rendered endpoints and frozen geometry, plus nonlinear diagnostic samples in
both directions. Results are recorded in [verification](verification.md).

[Typed tween driving and weighted sequences](value-tweens.md) now use this
foundation. [Constant and reverse views](animation-views.md) are also available. Full Animation members, Proxy/TrainHopping families, the
curve catalog, further tween specializations, transition widgets and complete
member compatibility remain unfinished. Reviewed the official
[Animation contract](https://api.flutter.dev/flutter/animation/Animation-class.html),
[CurvedAnimation direction/lifetime behavior](https://api.flutter.dev/flutter/animation/CurvedAnimation-class.html)
and [Cubic API](https://api.flutter.dev/flutter/animation/Cubic-class.html).
