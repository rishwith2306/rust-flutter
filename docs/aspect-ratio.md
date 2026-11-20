# Constrained aspect-ratio layout

`element::AspectRatio` and `Widget::aspect_ratio` attempt a width/height ratio
while giving the child one tight final size. The parent constraints take
precedence when that ratio cannot be satisfied.

```rust
use rust_flutter::{egui, element::{AspectRatio, ElementTree, Widget}, layout::BoxConstraints};
let mut tree = ElementTree::new(&egui::Context::default(),
    AspectRatio::new(2.0)?.into_widget(Widget::leaf(egui::vec2(1.0,1.0), |_| {})))?;
let size = tree.layout(BoxConstraints::loose(egui::vec2(100.0,100.0))?)?.size;
assert_eq!(size, egui::vec2(100.0,50.0));
# Ok::<(), Box<dyn std::error::Error>>(())
```

The solver starts with maximum width. When width is unbounded, it starts from
maximum height instead. It adjusts the dimensions against maximum and minimum
limits, then constrains the final result. Tight constraints are accepted directly;
conflicting limits can therefore produce a final size with a different ratio.
Both unbounded axes report UnboundedSize. Ratios must be positive and finite;
unrepresentable final geometry also reports a checked error. f64 intermediates
avoid avoidable overflow while the framework's geometry remains f32.

Dry layout solves the same size without measuring the child. Finite intrinsic
queries derive width from height or height from width; unbounded intrinsic
queries delegate to the child's pure protocol. Actual and dry baselines forward
from the child under its computed tight constraints. The wrapper does not add
painting or clipping. Stable child keys/types retain identity through ratio
updates. Use Widget::empty for an explicitly empty child.

Six headless tests cover bounded maxima, one/both unbounded axes, tight and
conflicting limits, pure queries without wet measurement, baseline/identity
updates, invalid ratios, zero constraints and extreme values. The gallery's green
transition child now obtains its 24×12 size from a 2:1 ratio with fixed height and
unbounded width. Native probes require that measured child geometry alongside
the existing clipped endpoint/muting checks; see [verification](verification.md).

Complete Flutter member/diagnostic and precision compatibility remains unfinished.
This is a constraint solver, not image fitting, cropping or scaling. Reviewed the
[AspectRatio behavior](https://api.flutter.dev/flutter/widgets/AspectRatio-class.html)
and [RenderAspectRatio API](https://api.flutter.dev/flutter/rendering/RenderAspectRatio-class.html).
