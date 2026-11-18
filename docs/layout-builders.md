# Layout-time widget construction

`element::LayoutBuilder` and `Widget::layout_builder` construct a retained child
using the constraints actually supplied by the parent. The callback receives the
normal `BuildContext`, including inherited values and dependency subscriptions.
The wrapper takes its child's size and forwards actual baselines.

```rust
use rust_flutter::{egui, element::{ElementTree, Widget}, layout::BoxConstraints};
let ctx = egui::Context::default();
let mut tree = ElementTree::new(&ctx, Widget::layout_builder(|_, constraints| {
    let width = if constraints.max().x < 480.0 { 120.0 } else { 240.0 };
    Widget::leaf(egui::vec2(width, 40.0), |ui| {
        ui.painter().rect_filled(ui.max_rect(), 0.0, egui::Color32::LIGHT_BLUE);
    }).keyed("content")
}))?;
let wide = tree.layout(BoxConstraints::loose(egui::vec2(800.0, 100.0))?)?;
assert_eq!(wide.size, egui::vec2(240.0, 40.0));
let narrow = tree.layout(BoxConstraints::loose(egui::vec2(320.0, 100.0))?)?;
assert_eq!(narrow.size, egui::vec2(120.0, 40.0));
# Ok::<(), Box<dyn std::error::Error>>(())
```

The callback first runs during wet layout, before that layout call returns and
before painting or gesture hit-path collection. Repeating identical constraints
reuses the child. A changed builder configuration, an explicit context rebuild or
an inherited dependency notification also requests construction. Stable keys and
widget types preserve ordinary child state across these builds. Removed or
replaced children follow the existing element lifecycle; there is no separate
nested tree or inherited-value boundary.

`ElementTree::update` and `flush` defer layout-builder callbacks until wet layout.
A pending configuration/dependency rebuild starts with the last known constraints;
a measurement in that transaction replaces them with the current constraints.
Unmeasured builders do not run before their first layout. A callback may receive
unbounded axes; handle infinity explicitly when choosing a child. Flexible
children still need bounded main-axis space under the existing flex policy.

The current implementation collects layout requests, releases render-object
borrows, builds/reconciles the requested children and retries layout within the
same call. Speculative placeholders never reach paint or input. It batches
siblings, and custom parents that measure one child several times coalesce its
requests to the last measurement. A final measurement matching an already-built
child cancels an earlier speculative request. Callback execution uses ordinary
component reconciliation order, rather than an in-place callback inside the
parent's render-object method. This timing/coalescing policy is an explicit
difference from Flutter's complete layout-callback protocol.

Keep builders free of effects that change their own parent constraints. A cycle
that fails to settle after 256 transactions returns a layout error while keeping
the tree mounted; a later layout can retry after the cause is corrected. Normal
widget validation still applies, including duplicate-key and depth errors.
Those build errors follow the existing unmount policy.

Dry layout, intrinsic size and dry baseline queries return UnsupportedQuery;
they never invent constraints to run the callback. The normal tree may flush
pending widget updates before a query, but layout-builder construction remains
deferred until wet layout. Actual layout forwards the built child's baselines.

Eleven headless tests cover first-paint construction, reuse, configuration and
constraint changes, keyed state, inherited dependencies, nested builders, 600
siblings, repeated custom measurements, ancestor borrow safety, explicit
rebuild/unmount and convergence/validation errors. The gallery builds its scroll
stripes through this API; native probes require a positive constrained width and
an unbounded content height, with widths 984→884 on Linux window resize. Runtime
results are recorded separately in [verification](verification.md).

This provides constraint-dependent composition. [Fixed-extent lazy lists](lazy-lists.md) now use this foundation. Variable-extent
child delegates, slivers and the complete Flutter LayoutBuilder/ConstrainedLayoutBuilder member
surface remain unfinished. Reviewed the official
[LayoutBuilder behavior](https://api.flutter.dev/flutter/widgets/LayoutBuilder-class.html).
