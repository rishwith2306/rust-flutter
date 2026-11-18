# Typed tween driving and weighted sequences

`Animatable` maps scalar progress to a typed value. `ValueTween<T>` interpolates
between shared mutable endpoints; `CurveTween` maps normalized progress through
a curve. Compose them with `chain`, then call `animate(parent)` or import
`AnimationDrive` and call `parent.drive(mapping)`. The result is an observable
`DrivenAnimation` with the parent's status and a typed ValueListenable value.
The older time-based scalar `animation::Tween` helper retains its existing API.

```rust
use rust_flutter::{egui, animation::{Animatable, AnimationConfig, AnimationController,
    AnimationDrive, Cubic, CurveTween, ValueTween}};
let ctx = egui::Context::default();
let parent = AnimationController::new(&ctx, AnimationConfig::default())?;
parent.set_value(0.5)?;
let endpoints = ValueTween::new(egui::vec2(12.0, 8.0), egui::vec2(24.0, 16.0))?;
let size = parent.clone().drive(endpoints.clone())?;
assert_eq!(size.try_value()?, egui::vec2(18.0, 12.0));
endpoints.set_end(egui::vec2(36.0, 24.0))?;
assert_eq!(size.try_value()?, egui::vec2(24.0, 16.0));
let eased = ValueTween::new(0.0_f32, 100.0)?.chain(CurveTween::new(
    Cubic::new(0.42, 0.0, 0.58, 1.0)?
)).animate(parent.clone())?;
assert!((eased.try_value()? - 50.0).abs() < 0.00001);
size.dispose()?;
eased.dispose()?;
parent.dispose()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Chaining applies the given parent mapping first and then the child mapping:
`child.chain(parent).transform(t)` is `child(parent(t))`. It composes functions
without allocating intermediate animation sources. `evaluate(&animation)` samples
an existing animation without attaching listeners. `CallbackAnimatable::new`
accepts an application closure returning Result, while the open Animatable trait
supports custom implementations and `Rc<dyn Animatable<Value=T>>` sources.
Custom mappings own output validation; return an error for values outside your
application's domain. ConstantTween returns the supplied value unchanged.

ValueTween and CurveTween clones share configuration. `set_begin`, `set_end` and
`set_curve` affect existing driven animations immediately when sampled. These
edits do not emit parent notifications or schedule playback/repaint themselves;
use the surrounding application update or active parent clock to refresh UI.
Owned subscriptions forward parent value/status events, including events where
the mapped value happens to remain equal. Dropping a token removes exactly its
registration. Disposal or final source drop cancels remaining links before the
owned parent is released; the parent itself remains independently controlled.
Weak driven/tween handles support callbacks without ownership cycles.

ValueTween preserves exact endpoint values for progress 0 and 1, and validates
endpoints and sampled output through TweenValue. Built-in f32/f64, Vec2, Pos2,
Rect, Color32, Insets, Stroke and BoxDecoration support finite extrapolation.
Invalid geometric results, such as inverted rectangles or negative insets/border
widths, return an error. Overflow also returns an error. Color channels and
u8 corner radii retain the backend's existing saturated interpolation policy;
colors interpolate premultiplied 8-bit channels, without a Flutter color-space
compatibility claim. Existing custom TweenValue implementations keep their
normalized-only interpolation contract by default. Override
`interpolate_unbounded` explicitly to support extrapolation for your type.
The implicit animation builder still supplies only normalized progress.

`CurveTween` accepts progress only in 0…1 and preserves endpoints. It carries no
playback direction history; use [CurvedAnimation](curved-animations.md) for distinct
forward/reverse curves and reversal latching. Mapping or interpolation panics
become checked AnimationError outcomes. Recursive sampling is rejected;
ValueTween/CurveTween configuration edits during their own sample return
Superseded. Driven sources also reject a stale sample after parent value/status
changes or disposal. Checked errors propagate through animation chains.
`value()`/ValueListenable getters panic on failed samples; use `try_value()` when
application mappings can fail. Keep custom interpolation and validation pure.

Weighted sequences can mix different mapping types with a common output type:

```rust
use rust_flutter::animation::{Animatable, ConstantTween, SequenceItem, TweenSequence, ValueTween};
let sequence = TweenSequence::new(vec![
    SequenceItem::new(ValueTween::new(0.0_f32, 10.0)?, 2.0)?,
    SequenceItem::new(ConstantTween(10.0_f32), 1.0)?,
    SequenceItem::new(ValueTween::new(10.0_f32, 20.0)?, 1.0)?,
])?;
assert_eq!(sequence.transform(0.25)?, 5.0);
assert_eq!(sequence.transform(0.625)?, 10.0);
assert_eq!(sequence.transform(0.875)?, 15.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Weights are positive finite f64 values and are scaled before summing, so a sum
larger than f64::MAX does not overflow merely because its individual weights
are large. A sequence requires at least one item and distinct representable
intervals. Progress is normalized; an exact shared boundary belongs to the next
section, and progress 1 evaluates the last section at its endpoint. Sampling
uses binary search over section boundaries. Mappings within each section use
its local normalized progress; sequences can be nested and curve/tween chains
can be placed inside them.

Headless tests cover scalar/vector/color/geometry interpolation and validation,
mutable endpoints, old custom interpolation contracts, mapping composition,
weighted sections and numerical extremes, callbacks, status/subscription/drop
ownership, reentrancy, overshoot and real controller-driven retained layout.
The gallery's third marker now receives a driven Vec2 directly. Native checks
require typed size diagnostics matching rendered geometry, endpoints, intermediate
values and frozen muting behavior; see [verification](verification.md).

This provides typed tween driving and weighted sequences. Nullable endpoint
policies, integer/step/reverse/flipped sequence variants, the wider geometry/theme
specializations, transition widget catalog and complete Flutter member/precision
compatibility remain unfinished. Reviewed the official
[Animatable composition contract](https://api.flutter.dev/flutter/animation/Animatable-class.html),
[mutable Tween behavior](https://api.flutter.dev/flutter/animation/Tween-class.html),
[CurveTween](https://api.flutter.dev/flutter/animation/CurveTween-class.html) and
[weighted TweenSequence](https://api.flutter.dev/flutter/animation/TweenSequence-class.html).
