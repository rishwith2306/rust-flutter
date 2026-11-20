# Wrapping layout

`Wrap` measures its children eagerly and places them into runs. A horizontal
wrap starts another row when the next child's width plus spacing exceeds the
parent's maximum width. A vertical wrap uses the maximum height and starts
another column. Children receive a loose main-axis maximum and an unbounded
cross axis. This is a layout primitive, not a lazy list or grid.

```rust
use rust_flutter::{element::{Wrap, WrapAlignment, Widget}, egui::vec2};
let widget = Wrap {
    spacing: 8.0,
    run_spacing: 4.0,
    alignment: WrapAlignment::Center,
    ..Wrap::default()
}.into_widget(vec![
    Widget::leaf(vec2(140.0, 24.0), |_| {}),
    Widget::leaf(vec2(140.0, 24.0), |_| {}),
    Widget::leaf(vec2(140.0, 24.0), |_| {}),
]);
```

At 436 points wide these three children fit in one row. At 300 points they form
two rows, with the final child on the second row. The wrap tries to use its
content size, constrained by the parent; use tight parent dimensions to provide
extra alignment space. An unbounded main axis keeps all finite children in one
run. An oversized child is first constrained to the main-axis maximum.

`alignment` distributes children within each run; `run_alignment` distributes
runs across the cross axis. Both support Start, End, Center, SpaceBetween,
SpaceAround and SpaceEvenly. Fixed spacing remains in addition to distributed
free space. A singleton SpaceBetween uses Start. `cross_axis_alignment` supports
Start, End and Center within a run. It does not stretch or baseline-align children.

`text_direction` defaults to Ltr and reverses horizontal placement when Rtl;
`vertical_direction` defaults to Down and reverses vertical placement when Up.
These affect child order, run order, Start/End and cross alignment on their
respective axes. Child storage, paint and hit-test order remain the declared
order. Keyed child identity survives reflow and configuration updates.

`clip` defaults to None. HardEdge clips when content overflows the wrapper's
size; it adds no clip when the wrap itself fits. Painting overflow does not
extend ancestor pointer-hit bounds. Antialiased and retained compositor clips
are not implemented. Empty content takes the parent's minimum size.

Dry layout and dry baseline queries use the same packing and placement as actual
layout. The reported baseline is the highest real translated child baseline,
not an invented bottom-edge fallback. Pure queries require pure child protocols
and return UnsupportedQuery rather than invoking mutable measurement callbacks.
Main-axis minimum intrinsic size is the largest child minimum; maximum intrinsic
size is the sum of child maxima, excluding spacing as in the reviewed Flutter
implementation. Cross-axis intrinsic size uses dry runs at the supplied main-axis
extent.

Spacing must be finite. Negative spacing is accepted for overlapping layouts;
negative total content dimensions and geometry beyond finite f32 range return
checked layout errors. Run fitting uses a strict comparison and f64 accumulators
before conversion to f32. This does not claim Flutter's floating-point tolerance
or precision compatibility.

The gallery's three colored bars use Wrap directly. Shared native probes require
three columns in wide layouts and two columns/two rows in the compact layout,
including exact solid RGB samples and measured inter-child spacing.

This independent implementation was reviewed against the official
[Wrap widget](https://api.flutter.dev/flutter/widgets/Wrap-class.html),
[RenderWrap](https://api.flutter.dev/flutter/rendering/RenderWrap-class.html) and
[rendering source](https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/rendering/wrap.dart).
Full SDK members, diagnostics, semantics, ambient directionality and platform
rendering compatibility remain unaudited. See [verification](verification.md)
for the actual tests and device evidence.
