# Baseline layout

`TextBaseline` selects alphabetic or ideographic coordinates. `Baselines` stores
optional real distances from a box's top. `None` means the box has no real baseline
of that kind. The framework does not infer a font baseline from a box's height or
from arbitrary text drawn by its paint closure.

`BoxNode::baseline_leaf` and `Widget::baseline_leaf` accept declared metrics.
Custom render boxes return metrics in `BoxGeometry::baselines`. Prefer
`BoxGeometry::new(size)` and then assign metrics; the geometry struct now includes
this additional field. Distances must be finite, but may be negative or outside
the box. Missing ideographic metrics are not synthesized from alphabetic values. Changing
reported baselines is layout-affecting even when the box size is unchanged: use
`RenderHandle::mark_needs_layout` or `CustomBox::invalidate` for custom data changes.
Paint-only requests deliberately preserve cached geometry and its metrics.

```rust
use rust_flutter::{egui::vec2, layout::*};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let small = BoxNode::baseline_leaf("small", vec2(20.0, 20.0),
        Baselines { alphabetic: Some(8.0), ideographic: None });
    let large = BoxNode::baseline_leaf("large", vec2(30.0, 30.0),
        Baselines { alphabetic: Some(20.0), ideographic: None });
    let mut row = Flex::new(Axis::Horizontal,
        vec![FlexChild::fixed(small), FlexChild::fixed(large)]);
    row.main_size = MainAxisSize::Min;
    row.cross_alignment = CrossAxisAlignment::Baseline(TextBaseline::Alphabetic);
    let mut tree = BoxNode::flex("row", row);
    let constraints = BoxConstraints::loose(vec2(200.0, 100.0))?;
    assert_eq!(tree.dry_layout(constraints)?, vec2(50.0, 32.0));
    assert_eq!(tree.dry_baseline(constraints, TextBaseline::Alphabetic)?, Some(20.0));
    let actual = tree.layout(constraints)?;
    assert_eq!(actual.baseline(TextBaseline::Alphabetic), Some(20.0));
    assert_eq!(actual.children[0].offset.y, 12.0);
    Ok(())
}
```

## Rows and propagation

A baseline-aligned horizontal flex combines the maximum ascent and maximum
descent of children that have real metrics. This group can be taller than any
single child. Other children contribute their ordinary height and sit at the top;
they do not use a fabricated bottom baseline for row alignment. `VerticalDirection`
does not flip baseline positions. `TextDirection` still controls horizontal order.

`CrossAxisAlignment::Baseline(kind)` encodes the required kind in the enum, rather
than allowing an absent configuration. It is meaningful for horizontal rows.
Vertical flex uses zero cross offset for this variant; use Start/End when directional
cross-edge alignment is desired.

Padding, alignment, constraints and intrinsic wrappers translate or propagate real
metrics. Rows and stacks report the highest translated real baseline of the
requested kind. Columns report the first baseline-bearing child in child-list
order, including when visual order runs upward. Queries for either kind use the
actual placement offsets, even when the row aligns by the other kind.

After layout, `LayoutBox::baseline(kind)` returns a real value or `None`.
`distance_to_baseline(kind, only_real)` can explicitly fall back to the box's bottom
when `only_real` is false. `BoxChildren::baseline(index, kind, only_real)` provides
the same information during a custom parent's layout, after that child was laid
out. Querying an unmeasured child returns an error. Snapshot reads are not Flutter's
live RenderObject phase-checked getter API.

## Baseline and IgnoreBaseline wrappers

`Widget::baseline(distance, kind, child)` and `BoxNode::baseline` shift the child
so its selected real baseline reaches the requested distance. If the child lacks
one, the wrapper positions its bottom there instead. This fallback positions the
child but does not invent a real baseline for descendants/ancestors. Any other
real baseline kind is still translated correctly, including in dry queries.

The wrapper sizes around the shifted child, subject to parent constraints. A small
or negative target can place the child above the wrapper. Painting may overflow,
but ancestor hit bounds still gate input; the overflowed part is not automatically
interactive. Intrinsic contributions delegate to the child, as for a shifted box,
and do not add the requested target distance.

`Widget::ignore_baseline` and `BoxNode::ignore_baseline` suppress outward real
metrics while preserving layout, painting, children and hit geometry. They do not
make an unsupported child size query computable.

## Dry baselines

`BoxNode::dry_baseline`, `ElementTree::dry_baseline` and
`BoxQueries::dry_baseline` speculate without running wet layout or changing its
cache. Tree methods first flush pending widget updates, like other tree size
queries. Implement optional `RenderBox::dry_baseline` or
`CustomBoxLayout::dry_baseline` using `&self`, configured properties and immutable
child queries. Return a real distance, `Ok(None)` for known absence, or
`UnsupportedQuery` when the computation is unavailable. Dry callbacks do not
receive a mutable RenderContext.

A baseline-aligned row needs children's pure baseline implementations when
computing its dry size and cross-axis intrinsic contributions. A non-baseline
row's dry-size query does not ask for baselines unnecessarily. Legacy measured
leaves report no real metadata without executing their callback. A shifted wrapper
can still need the child's dry size to calculate a bottom fallback.

A stack's positioned top baseline can be computed without knowing that child's
size, when the policy supplies a baseline. Bottom-anchored or alignment-dependent
positions may also need dry size. Pure queries return explicit errors rather than
running an opaque callback to obtain missing data.

## Verification and limits

Tests cover ascent/descent group sizing, both kinds, other-kind queries,
missing metrics, direction changes, flex fits/tight constraints, first/highest
propagation, positioned stacks, negative overflow/hit bounds, custom child access,
cache preservation, invalid values and retained state across errors/updates.

The gallery uses actual pinned-egui galley glyph baseline positions to draw ASCII
text at two font sizes. Its probes locate the rendered colored boxes and reference
line, then compare their physical baseline positions within two pixels. The Linux
probe places both fonts and the line at pixel y=429 after resize. The local
Android API 37 probe places both font baselines at y≈1405 and the line at
y=1404.5, within tolerance. This is scoped
mixed-font backend evidence, not complete typography or Unicode shaping validation.
The example rebuilds its font metrics when effective display scale changes.

Font fallback/shaping, general text/rich-text widgets, accurate ideographic font
metrics, multi-line text baseline policy, slivers, baseline caches, intrinsic
debug audits and the full Flutter rendering API remain unfinished. Interfaces
support reported ideographic distances; the demo deliberately reports only an
alphabetic value.

Primary references reviewed:
[actual baseline and fallback](https://api.flutter.dev/flutter/rendering/RenderBox/getDistanceToBaseline.html),
[dry baseline contract](https://api.flutter.dev/flutter/rendering/RenderBox/getDryBaseline.html),
[Baseline wrapper](https://api.flutter.dev/flutter/rendering/RenderBaseline-class.html),
[stack propagation](https://api.flutter.dev/flutter/rendering/RenderStack/computeDistanceToActualBaseline.html), and
[flex implementation](https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/rendering/flex.dart).
