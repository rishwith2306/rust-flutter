# Fractional sizing

`FractionallySizedBox` gives its child a tight width or height equal to the
corresponding parent maximum multiplied by a configured factor. `None` passes
that axis's incoming constraints through unchanged. The box then constrains its
own size to the parent and aligns the child inside it.

```rust
use rust_flutter::{element::{FractionallySizedBox, Widget}, egui::vec2};
let widget = FractionallySizedBox::new(Some(0.5), Some(0.25))?
    .into_widget(Widget::leaf(vec2(20.0, 10.0), |_| {}));
# Ok::<(), rust_flutter::layout::LayoutError>(())
```

Under loose maxima of 200 by 80, both child and box become 100 by 20 logical
points. Under a tight 200 by 80 parent, the child remains 100 by 20 and is
centered at offset (50, 30). Set the public `alignment` field to change this
physical alignment. A factor of 2 can make a child larger than its parent;
this wrapper adds no clip. Pointer hits remain bounded by ancestor boxes even
where overflow is painted. Direction-sensitive alignment is not implemented.

Factors are finite and nonnegative; zero and values above 1 are accepted. A
factor on an unbounded parent axis produces a checked `UnboundedSize` error,
including zero times infinity. Provide a finite ancestor constraint, for example
`LimitedBox`, or leave the factor absent on that axis. Products that cannot fit
a finite `f32` also fail. Public field configurations are checked during layout
and pure queries, not just by `new`. `Widget::fractionally_sized_box` provides a
checked convenience constructor.

Dry layout uses the same child constraints as actual layout. Real alphabetic
and ideographic baselines include the child's aligned vertical offset, including
negative offsets for overflow. Intrinsic queries multiply the opposite-axis
argument by its factor and divide the child's result by the queried axis's
factor. Undefined or nonfinite intrinsic results, such as division by zero,
return `InvalidSize`; these are not fabricated finite measurements. Pure queries
never invoke mutable layout callbacks.

Configuration changes retain keyed children. Direct render-object construction
also accepts no child: sizing uses the inner constraints' minimum, constrained
by the parent. More than one child is an error. Defaults have no factors and
center alignment.

The gallery composes `LimitedBox(max_width: 48)`, a width factor of 0.5, and an
`AspectRatio` child in its green size-transition marker. Native probes require
24 by 12 logical child dimensions as well as the existing rendered marker gates.

The behavior review uses Flutter's [FractionallySizedBox](https://api.flutter.dev/flutter/widgets/FractionallySizedBox-class.html)
and [RenderFractionallySizedOverflowBox](https://api.flutter.dev/flutter/rendering/RenderFractionallySizedOverflowBox-class.html),
with layout, intrinsic and baseline algorithms reviewed in the official
[rendering source](https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/rendering/shifted_box.dart).
This independent Rust implementation uses checked errors rather than framework
assertions for invalid dimensions. Full member, directionality, diagnostics,
semantics and numeric precision compatibility remain unaudited. Linux rendering
checks do not establish macOS runtime or Android device compatibility.
