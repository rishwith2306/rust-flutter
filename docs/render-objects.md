# Persistent box render objects

`Widget::render_object(configuration, children)` mounts a typed `RenderObjectWidget`
and its persistent `RenderBox`. An empty child list makes a leaf; one or multiple
children support proxy and container boxes. The widget type and sibling key govern
identity, independently of its associated object type. Updating configuration or
reordering keyed siblings retains the object. Changing the widget type or key
replaces it.

This is the first custom render-object layer. Existing built-in widget adapters
still regenerate `BoxNode` descriptions. This does not implement Flutter's entire
RenderObject/PipelineOwner API, retained compositing, semantics, slivers or
relayout boundaries.

## Implementing a box

```rust
use rust_flutter::{egui, element::*, layout::*};

struct Background(egui::Color32);
struct BackgroundBox(egui::Color32);

impl RenderObjectWidget for Background {
    type Object = BackgroundBox;
    fn create_render_object(&self, _: &RenderContext) -> Self::Object {
        BackgroundBox(self.0)
    }
    fn update_render_object(&self, object: &mut Self::Object, _: &RenderContext) {
        object.0 = self.0;
    }
}
impl RenderBox for BackgroundBox {
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        if children.len() != 1 {
            return Err(LayoutError::InvalidConfiguration("background needs one child"));
        }
        let size = children.layout(0, constraints)?;
        children.place(0, egui::Vec2::ZERO)?;
        Ok(BoxGeometry::new(size))
    }
    fn paint(&mut self, ui: &mut egui::Ui, _: &RenderContext) {
        ui.painter().rect_filled(ui.max_rect(), 0.0, self.0);
    }
}

let widget = Widget::render_object(
    Background(egui::Color32::DARK_BLUE),
    vec![Widget::leaf(egui::vec2(100.0, 40.0), |ui| { ui.label("Child"); })],
);
```

`BoxChildren` exposes ordered child IDs, layout with constraints, and placement.
Each child must be successfully laid out and placed exactly once. Multiple
measurements are allowed before placement. Invalid indices, duplicate placement,
placement before measurement, layout after placement, missing placements and
nonfinite offsets return layout errors. The framework checks child and parent
constraints, finite bounds, tree depth and duplicate IDs. Negative offsets are
permitted for overflow.

`BoxGeometry` selects size, clipping, and participation in geometry self-hit tests.
Children paint in placement order and hit test in reverse order, followed by the
parent if `hit_test_self` is true. Ancestor bounds gate geometry hits even when
painting overflows them. Custom boxes paint **before** children; arbitrary
interleaved child painting, foreground passes, transforms, specialized hit-test
paths and semantics remain work to implement. egui controls placed in a box's UI
retain their ordinary input handling.

## Lifecycle and inherited data

Mount calls `create_render_object`, then `RenderBox::attach`. An initial
`update_render_object` configures inherited subscriptions before layout. Unlike
Flutter's exact call schedule, this Rust API deliberately configures once on mount
as well as on later property/dependency updates. Creation and attachment may read
inherited snapshots but may not establish listening dependencies. Update can use
`RenderContext::depend_on` or `depend_on_aspect`; later notifications reconfigure
the same object. Layout and paint can use the existing mounted-context lookup
rules. Untracked external reads require explicit invalidation.

`children_changed` reports old/new **render descendant** IDs when ordered slots
change. A stateful/stateless or inherited wrapper's element ID is not substituted
for the actual render child. This includes a component replacing its returned
render root without replacing the component itself. Named slot abstractions and
GlobalKey adoption remain pending.

Unmount removes children first, closes the object's handle, calls `detach`,
`did_unmount_render_object` on the latest widget configuration, then `dispose`.
Each mounted object receives these removal hooks once. Failed dynamic builds
unmount the affected tree and clean up objects, including an object whose initial
child mount failed. Ordinary layout errors keep the object mounted for recovery.
User panics are not isolated by this render layer.

## Layout and paint invalidation

`RenderContext::handle()` returns a sendable `RenderHandle`; objects and their
configuration remain UI-local and may contain `Rc` or other non-Send values.

- `mark_needs_layout()` invalidates the tree's layout revision and requests a frame.
- `mark_needs_paint()` requests a frame while preserving custom layout caches.
- Both return `Unmounted` after the object is removed.

Custom layout caches include constraints, object/child identity and a tree-local
layout revision. Configuration updates, component changes, inherited updates and
explicit layout requests conservatively invalidate custom layout across that tree.
Identical frames and paint-only requests reuse custom geometry. A request made
during layout is preserved for a later pass. This is a coarse layout owner;
per-object relayout boundaries, parentUsesSize propagation and separate compositing
and semantics dirty queues are unfinished.

Painting still uses egui each frame. There is no retained display-list paint cache
or claim that a paint-only request implements Flutter's complete flushPaint phase.
Existing standalone measured leaves still execute normally; a measured child under
a cached custom parent must participate in its parent's invalidation contract.

Standalone `BoxNode` users can share a `CustomBox` implementing `CustomBoxLayout`.
Call `CustomBox::invalidate()` when changing geometry inputs (including child
properties with unchanged IDs), or supply an external `revision()`. Local requests
made during layout are safe and are not lost. This low-level handle does not
request host frames; the retained adapter's RenderHandle does.

The gallery view report uses a persistent custom container. The Linux probe checks
one creation and one layout across unrelated interactions, then one configuration
update and one additional layout after native resize. The Android probe checks
one creation/update/layout while running the ordinary component interactions.
These scoped probes do not establish full native rendering or Flutter parity.

Reviewed primary references:
[RenderObjectWidget](https://api.flutter.dev/flutter/widgets/RenderObjectWidget-class.html),
[RenderObjectElement](https://api.flutter.dev/flutter/widgets/RenderObjectElement-class.html),
[RenderBox](https://api.flutter.dev/flutter/rendering/RenderBox-class.html), and
[PipelineOwner](https://api.flutter.dev/flutter/rendering/PipelineOwner-class.html).

Read-only dry layout, intrinsic dimensions and sizing wrappers are described in
[size queries](size-queries.md). Missing pure implementations return explicit errors;
queries do not fall back to the mutable layout method.
