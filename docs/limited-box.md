# LimitedBox

`LimitedBox` supplies a maximum width or height only when that parent axis is
unbounded. Bounded axes retain the parent's constraints, even if their maximum
is larger than the configured limit. Parent minimum sizes always win.

```rust
use rust_flutter::{element::{LimitedBox, Widget}, egui::vec2};
let child = Widget::leaf(vec2(200.0, 200.0), |_| {});
let widget = LimitedBox::new(120.0, 80.0)?.into_widget(child);
# Ok::<(), rust_flutter::layout::LayoutError>(())
```

With both parent maxima infinite, this child becomes 120 by 80 logical points.
With parent maxima 300 by 300, it remains 200 by 200. Limits do not force a
smaller child to expand. Defaults are infinity; zero is valid. Negative and NaN
limits return `LayoutError`, including configurations made through public fields.
`Widget::limited_box` is a checked convenience constructor.

Layout and dry layout pass the same adjusted constraints to the child. Intrinsic
queries delegate unchanged. Actual baselines forward the child's reported values;
dry baselines use the adjusted child constraints. Configuration updates retain
keyed children. Direct render-object construction accepts zero or one child;
without a child it takes the parent's minimum size and reports no baseline.
Multiple children produce a layout error. No clipping or decoration is added.

The behavior review uses Flutter's [LimitedBox](https://api.flutter.dev/flutter/widgets/LimitedBox-class.html)
and [RenderLimitedBox](https://api.flutter.dev/flutter/rendering/RenderLimitedBox-class.html).
This is an independent Rust implementation. Flutter's inherited proxy dry-baseline
behavior forwards incoming constraints; this implementation deliberately uses the
child's adjusted constraints so pure baseline queries agree with actual layout.
Full member, diagnostics, floating-point and framework lifecycle parity remain
unaudited. The focused tests exercise retained geometry and pure queries; they
are not native macOS or Android rendering evidence.
