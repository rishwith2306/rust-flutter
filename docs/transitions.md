# Retained size, alignment and decoration transitions

SizeTransition, AlignTransition and DecoratedBoxTransition consume explicit
Animation sources while retaining an ordinary child subtree. Size/alignment
notifications invalidate layout; successful decoration notifications request
paint only. Animation ticks do not rebuild child widget descriptions. This is
a render-object implementation rather than Flutter's AnimatedWidget build path.

```rust
use rust_flutter::{egui, animation::{AnimationConfig, AnimationController, SizeTransition},
    element::{ElementTree, Widget}, layout::BoxConstraints};
let ctx = egui::Context::default();
let factor = AnimationController::new(&ctx, AnimationConfig::default())?;
factor.set_value(0.5)?;
let mut tree = ElementTree::new(&ctx, SizeTransition::new(factor.clone()).into_widget(
    Widget::leaf(egui::vec2(80.0, 40.0), |ui| { ui.label("Retained child"); })
))?;
let constraints = BoxConstraints::loose(egui::vec2(100.0, 100.0))?;
assert_eq!(tree.layout(constraints)?.size, egui::vec2(100.0, 20.0));
factor.set_value(1.0)?;
assert_eq!(tree.layout(constraints)?.size, egui::vec2(100.0, 40.0));
tree.unmount();
factor.dispose()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

SizeTransition measures the child with loosened parent constraints, multiplies
its selected axis by the nonnegative animation factor, then constrains its own
size and aligns the child inside a hard rectangular clip. Negative factors
collapse to zero; factors above one are allowed. Tight parent constraints can
prevent the wrapper from shrinking. `axis` defaults to Vertical;
`fixed_cross_axis_size_factor` optionally shrinks the other axis. Otherwise a
bounded cross axis expands to its maximum, and an unbounded one shrink-wraps.

An explicit physical `alignment` controls both axes. The default centers along
the animated axis and starts along the other axis, matching the reviewed LTR
implementation. This differs from the official class prose's general center
wording; directional resolution and the deprecated axisAlignment API are not
implemented. Children retain their actual size; this clips rather than scales.
Clipped-away areas do not receive new retained hit paths. Existing captured
pointer routes follow the normal binding policy, rather than Offstage's explicit
capture cancellation.

AlignTransition consumes `Animation<layout::Alignment>`. Width/height factors
follow the existing Align policy; it leaves visual overflow unclipped. Alignment
now implements TweenValue, so ValueTween can drive it. Both layout transitions
support pure dry/intrinsic/baseline queries, including translated baselines.
Queries use the latest notified sample and never fall back to wet child layout.

DecoratedBoxTransition consumes `Animation<BoxDecoration>` and paints in the
selected Background or Foreground phase. It forwards the child's size, intrinsic
queries and baselines without adding padding. Rounded fills and uniform borders
retain the existing decoration policy; children are not shape-clipped. An
isolated headless test verifies that a successful decoration-only update does
not rerun the measured child layout. The overall framework still uses coarse
layout invalidation, so unrelated layout changes can remeasure the tree.

Sources are sampled after subscription, including synchronously firing sources.
Reentrant source notifications keep the newest sample. Widget configuration
updates refresh the sample and rebind changed source identities, canceling old
links. If an earlier subscription failed, rebinding retries subscription even
when source identity is unchanged, so recovery restores future notifications. Unmount releases subscriptions without disposing externally owned
animations. Mutable mapping edits still follow their notification contract:
notify the parent or update the widget configuration to refresh the sample.

Failed samples, disposed sources and invalid paint values surface as typed
`LayoutError::Animation` errors, with the underlying error available through
std::error::Error::source. A paint-only source failure also invalidates layout so
it can be reported before painting. Fixing/replacing the source can recover
without disposing child state. Initial source and static configuration errors
are checked during layout/query processing.

Twelve headless tests cover constraints, clipping/hits, factors, axes, translated
baselines and pure queries, unchanged child identity/build counts, source swaps,
alignment overflow, decoration paint ordering and layout reuse, error recovery,
synchronous/reentrant sources and actual controller/tween playback. The gallery
adds a green clipped marker beside the typed animation marker; shared native
probes require 12→24→12-point widths and unchanged geometry while muted.
See [verification](verification.md) for platform evidence.

Full AnimatedWidget/AnimatedBuilder APIs, directional AlignmentGeometry,
Fade/Slide/Scale/Rotation/Positioned and other transitions, compositor layers,
complete focus/semantics behavior and full member compatibility remain unfinished.
Clipping does not establish native IME or screen-reader parity. Reviewed the
[SizeTransition API](https://api.flutter.dev/flutter/widgets/SizeTransition-class.html),
[AlignTransition API](https://api.flutter.dev/flutter/widgets/AlignTransition-class.html)
and [transition implementation](https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/widgets/transitions.dart).
