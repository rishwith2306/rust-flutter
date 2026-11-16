# Dry layout and intrinsic dimensions

Box layout now has separate read-only sizing protocols. `BoxNode::dry_layout`
returns the size for supplied constraints without running wet layout, placing
children, changing the actual layout cache or painting. Four intrinsic queries
express minimum/maximum content contributions along either axis. These are
content measurements, not an alias for running layout with loose constraints.

`BoxNode` exposes `min_intrinsic_width`, `max_intrinsic_width`,
`min_intrinsic_height`, `max_intrinsic_height`, and a generic `intrinsic` method.
`ElementTree` exposes `dry_layout` and `intrinsic(IntrinsicDimension, extent)`.
Tree methods first flush pending widget/configuration updates; that build work
can change component state. The subsequent measurement is read-only and does not
perform wet layout. Query failures preserve mounted state like other layout
errors.

## Custom render boxes

Implement optional `RenderBox::dry_layout` and `RenderBox::intrinsic` methods.
They receive `&self` and immutable `BoxQueries`, without a mutable RenderContext.
Compute from already configured properties and query children through the pure
protocol. Subscribe to inherited inputs during `update_render_object`, not during
measurement. Interior mutability cannot enforce purity: implementations must not
change persistent layout state, invalidate geometry, place children or call wet
layout as a fallback.

```rust
use rust_flutter::{egui, element::*, layout::*};

struct Tile(egui::Vec2);
impl RenderObjectWidget for Tile {
    type Object = Tile;
    fn create_render_object(&self, _: &RenderContext) -> Tile { Tile(self.0) }
    fn update_render_object(&self, object: &mut Tile, _: &RenderContext) {
        object.0 = self.0;
    }
}
impl RenderBox for Tile {
    fn dry_layout(&self, c: BoxConstraints, _: &BoxQueries<'_>)
        -> Result<egui::Vec2, LayoutError> {
        c.constrain(self.0)
    }
    fn intrinsic(&self, d: IntrinsicDimension, _: f32, _: &BoxQueries<'_>)
        -> Result<f32, LayoutError> {
        Ok(match d {
            IntrinsicDimension::MinWidth | IntrinsicDimension::MaxWidth => self.0.x,
            IntrinsicDimension::MinHeight | IntrinsicDimension::MaxHeight => self.0.y,
        })
    }
    fn layout(&mut self, c: BoxConstraints, children: &mut BoxChildren<'_>, _: &RenderContext)
        -> Result<BoxGeometry, LayoutError> {
        if !children.is_empty() {
            return Err(LayoutError::InvalidConfiguration("Tile is a leaf"));
        }
        Ok(BoxGeometry::new(self.dry_layout(c, &children.queries())?))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = egui::Context::default();
    let mut tree = ElementTree::new(&ctx, Widget::intrinsic_width(
        Some(10.0), None,
        Widget::render_object(Tile(egui::vec2(33.0, 17.0)), vec![]),
    ))?;
    let constraints = BoxConstraints::loose(egui::vec2(200.0, 100.0))?;
    assert_eq!(tree.dry_layout(constraints)?, egui::vec2(40.0, 17.0));
    assert_eq!(tree.intrinsic(IntrinsicDimension::MaxWidth, f32::INFINITY)?, 40.0);
    assert_eq!(tree.layout(constraints)?.size, egui::vec2(40.0, 17.0));
    Ok(())
}
```

`BoxChildren::queries()` allows speculation during normal custom layout without
marking children laid out or placed. Normal layout must still call `layout` and
`place` for every child. The low-level `CustomBoxLayout` trait has the same optional
pure methods for standalone box trees.

Intrinsic extents must be nonnegative and not NaN; infinity is permitted as an
argument. Results must be finite and nonnegative. Dry sizes must satisfy incoming
constraints. The framework validates these rules, child indices, duplicate IDs
and depth. It cannot prove a custom algorithm's semantic correctness; in particular,
its dry answer must agree with its actual size for valid inputs, and minimum/maximum
contributions must obey their content contracts.

Missing implementations return `LayoutError::UnsupportedQuery`. Legacy
`Widget::measured` / `BoxNode::measured` callbacks intentionally have no implicit
pure measurement: querying them never executes their mutable callback. Provide a
custom render box/policy when pure queries are needed. Existing implementations
that only perform wet layout remain source-compatible.

## Built-in boxes and intrinsic wrappers

Fixed leaves report their declared preferred dimensions; this does not infer the
size of arbitrary text or controls drawn in a paint closure. Padding deflates the
opposite extent and adds its own space. Constrained and aligned boxes account for
their limits and factors. Flex main-axis contributions account for flex ratios and
spacing; cross-axis queries allocate main-axis space from maximum-content values.
Stack contributions exclude positioned children. Stack dry sizing similarly skips
positioned child measurements because those children do not determine stack size.

Wet and dry flex paths share the same allocation math. Dry sizing does not mutate
actual child positions or cache entries. Other dry wrappers use the same constraint
transformations as their normal layouts.

`Widget::intrinsic_width(step_width, step_height, child)` and
`Widget::intrinsic_height(child)` provide retained wrappers; equivalent `BoxNode`
constructors are available. They calculate the necessary content dimension, clamp
the child's constraints to the parent's limits, and preserve child identity on
configuration changes. Width steps round upward; optional height steps also round
upward. Steps must be finite and positive. Intermediate step arithmetic uses f64,
with logical sizes represented as f32.

Tight axes skip unneeded intrinsic queries. A tight width does not magically add
a missing dry-layout implementation to its child. The width wrapper's optional
height step queries against the original parent's maximum width, matching the
reviewed Flutter behavior. Wrappers can bound the cross axis of a stretched row or
column where ordinary unbounded stretch would fail.

## Verification and remaining work

Tests compare dry and actual flex results across axes, directions, fits and bounded
or unbounded constraints; cover wrappers, padding, flex contribution formulas,
positioned stacks, custom protocols, invalid arguments/results, unsupported
callbacks, retained inherited updates, cache preservation and state recovery.
The native gallery compares its view container's dry answer with its allocated
rectangle (within 0.01 logical point). Platform probes keep the existing one/two
actual-layout assertions, so speculative queries cannot hide extra wet layouts.

There is no intrinsic-query cache or per-child dependency graph yet. Repeated
speculation can be expensive, particularly for nested intrinsic wrappers; do not
assume a fixed frame-time cost. Text shaping/intrinsics, baselines and dry baselines,
slivers, intrinsic debug cross-validation and full Flutter rendering compatibility
remain unfinished.

Primary references reviewed:
[computeDryLayout](https://api.flutter.dev/flutter/rendering/RenderBox/computeDryLayout.html),
[minimum intrinsic width](https://api.flutter.dev/flutter/rendering/RenderBox/getMinIntrinsicWidth.html),
[maximum intrinsic width](https://api.flutter.dev/flutter/rendering/RenderBox/getMaxIntrinsicWidth.html),
[RenderFlex](https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/rendering/flex.dart), and
[intrinsic proxy boxes](https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/rendering/proxy_box.dart).
