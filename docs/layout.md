# Constraint-based box layout

`rust_flutter::layout` supplies an owned box tree with independent geometry
calculation. Parents pass `BoxConstraints` down; children return finite sizes;
parents choose child offsets. The result can be inspected, hit-tested and rendered
through `show_layout`. Existing `row`, `column` and other helpers still use egui's
layout. This module adds constraints and geometry without requiring those helpers.

Keep a `BoxNode` in your application model, mutate its configuration when needed,
and call `layout` for the available space each frame. Keys identify boxes and
preserve their egui leaf widget IDs when siblings reorder. For declarative widget reconciliation and component-owned state, use
[ElementTree](components.md), which adapts to this box tree. Dirty-subtree render
layout caching, GlobalKey and the full Flutter element/render pipeline remain
unimplemented.

```rust
use rust_flutter::{egui, layout::*};

let mut flex = Flex::new(Axis::Horizontal, vec![
    FlexChild::fixed(BoxNode::leaf("fixed", egui::vec2(60.0, 40.0))),
    FlexChild::expanded(BoxNode::leaf("one", egui::Vec2::ZERO), 1),
    FlexChild::expanded(BoxNode::leaf("two", egui::Vec2::ZERO), 2),
]);
flex.spacing = 8.0;
flex.cross_alignment = CrossAxisAlignment::Stretch;
let mut tree = BoxNode::padding(
    "root", Insets::all(8.0), BoxNode::flex("row", flex)
);
let constraints = BoxConstraints::tight(egui::vec2(400.0, 180.0)).unwrap();
let geometry = tree.layout(constraints).unwrap();
// 400 - 16 padding - 60 fixed - 16 spacing = 308 flexible points.
let one = geometry.find(egui::Id::new("one")).unwrap();
let two = geometry.find(egui::Id::new("two")).unwrap();
assert!((two.size.x - 2.0 * one.size.x).abs() < 0.001);
```

`BoxConstraints::new`, `tight`, `loose` and `tight_for` validate min/max extents.
Extents must satisfy `0 <= min <= max`; infinity represents an unbounded axis.
`loosen` removes minimums, `deflate` subtracts padding while clamping at zero, and
`enforce` clamps extra constraints to a parent's range. `constrain` clamps a
requested size, then rejects a result that is still infinite. No layout result
may contain nonfinite sizes or offsets. A custom measured leaf must return a size
inside its received constraints; violations return `LayoutError`.

`BoxNode::constrained` adds size limits. `padding` deflates the child constraints
and applies the corresponding offset. `align` expands within bounded axes and
shrink-wraps unbounded axes, then positions the child using an `Alignment`.
`BoxKind::Align` also accepts optional width/height factors. Factors multiply the
child's size before parent constraints are applied. Alignment coordinates use
-1 for left/top, 0 for center, and 1 for right/bottom; finite values outside that
range deliberately permit overflow. These alignments are physical coordinates.

A flex tree measures fixed children with an unbounded main axis, subtracts their
sizes and explicit gaps, then divides remaining space by positive flex factors.
Expanded uses tight allocations; Flexible can use a loose upper limit.
`MainAxisSize::Min` uses actual child sizes. `Max` fills a bounded main axis.
Start/end/center and the three space-distribution modes position the result.
`TextDirection` and `VerticalDirection` resolve the start and end edges. Stretch
requires a bounded cross axis. A flex factor of zero is an error; use
`FlexChild::fixed` instead. An unbounded main axis permits flexible children only
when all are loose and the flex is shrink-wrapping. Expanded in a scrolling,
unbounded axis returns `UnboundedFlex` rather than inventing an infinite size.

```rust
let mut stack = Stack::new(vec![
    StackChild::new(BoxNode::leaf("background", egui::vec2(200.0, 120.0))),
    StackChild::positioned(BoxNode::leaf("badge", egui::Vec2::ZERO), Positioned {
        right: Some(4.0), bottom: Some(4.0),
        width: Some(48.0), height: Some(32.0),
        ..Default::default()
    }),
]);
stack.alignment = Alignment::CENTER;
let mut overlay = BoxNode::stack("stack", stack);
let geometry = overlay.layout(BoxConstraints::loose(egui::vec2(300.0, 200.0)).unwrap()).unwrap();
assert_eq!(geometry.size, egui::vec2(200.0, 120.0));
```

Stack sizes come from non-positioned children. Loose fit removes their minimums,
Expand tightens them to the bounded maximum, and Passthrough preserves the parent
constraints. An all-positioned or empty stack needs finite maximums. Positioned
children accept at most two of left/right/width or top/bottom/height per axis.
Opposite edges derive a tight size, clamped at zero. Negative offsets are allowed;
negative explicit sizes and conflicting triples are errors. Positioned children
do not contribute to the stack's size. Children paint in list order, with the last
on top. A `Positioned` with all fields unset behaves as a non-positioned child.

Rendering from `Application::build`:

```rust
let geometry = tree.layout(BoxConstraints::tight(
    egui::vec2(ui.available_width(), 180.0)
).unwrap()).unwrap();
show_layout(ui, &geometry, |key, ui| {
    let rect = ui.max_rect();
    if key == egui::Id::new("fixed") {
        if ui.put(rect, egui::Button::new("Tap")).clicked() {
            // Update application state here.
        }
    } else {
        ui.painter().rect_filled(rect, 4.0, egui::Color32::DARK_GREEN);
    }
});
```

The callback receives independent leaf UIs with computed rectangles and stable
IDs. Fit widgets to that rectangle using `ui.put` or paint directly; the adapter
does not automatically measure arbitrary egui widgets. Empty containers do not
invoke leaf callbacks. Keys must be unique across one entire tree, and root keys
must be distinct when multiple trees share the same parent UI. Trees deeper than
256 levels return an error. Widget IDs remain stable under sibling reordering
within the same root/parent UI; this is separate from lifecycle reconciliation.

`LayoutBox` contains child offsets, local paint bounds and overflow on each edge.
`rect_for` includes all ancestor offsets. `hit_test` returns the topmost leaf,
using half-open box bounds; a point outside an ancestor cannot hit its descendants.
The standalone hit test does not dispatch events. Embedded controls continue to
use egui's event system. Hard-edge clipping is inherited by painting and egui
leaf UIs; Clip::None permits painting overflow. Both Flex and Stack currently
default to HardEdge. Callbacks that paint or create interaction regions outside
their assigned rectangles can differ from the standalone box hit test; full
render-object input dispatch remains unfinished.

The gallery retains one tree, resizes its root constraints and exercises fixed
width, 1:2 expansion, cross-axis stretch, padding and a positioned overlay. Tests
cover constraint validation, finite results, flex directions/spacing/overflow,
stack sizing/offsets/clip bounds/hit order, stable leaf widget IDs and an actual
pointer click on overlapping egui controls. The desktop smoke also samples the
rendered 1:2 widths after resize. The Android smoke opens the page, activates the
constrained button and returns through system Back on the isolated test emulator. These are foundation checks; baseline alignment,
intrinsic size protocols, slivers, gesture arbitration, semantics and the full
Widget/Element/RenderObject lifecycle remain separate work.

Primary references: [Flutter box constraints](https://api.flutter.dev/flutter/rendering/BoxConstraints-class.html),
[RenderFlex](https://api.flutter.dev/flutter/rendering/RenderFlex-class.html),
[Stack](https://api.flutter.dev/flutter/widgets/Stack-class.html).
