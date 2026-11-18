# Keeping lazy item state alive

Fixed-extent `ListView` items normally dispose when they leave the viewport/cache
range. `Widget::keep_alive(true, child)` or `scrolling::KeepAlive::new(true)`
requests retention of the nearest lazy item instead. The same mounted element,
local state, state handles, inherited dependencies and keyboard focus survive
scrolling away and back. This is distinct from saving row values in an external
application model.

```rust
use rust_flutter::{egui, element::{ElementTree, Widget}, gestures::GestureBinding,
    layout::BoxConstraints, scrolling::{ListView, ScrollPosition}};
let ctx = egui::Context::default();
let position = ScrollPosition::new(&ctx, 0.0)?;
let binding = GestureBinding::new();
let list = ListView::builder(&position, &binding, 1_000, 40.0, |_, index| {
    Widget::keep_alive(index == 0, Widget::leaf(egui::vec2(100.0, 40.0), move |ui| {
        ui.label(format!("Row {index}"));
    }))
})?.cache_extent(0.0)?.into_widget()?;
let mut tree = ElementTree::new(&ctx, list)?;
let constraints = BoxConstraints::tight(egui::vec2(240.0, 120.0))?;
tree.layout(constraints)?;
position.jump_to(4_000.0)?;
tree.layout(constraints)?; // row 0 remains mounted, but hidden
position.jump_to(0.0)?;
tree.layout(constraints)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Several descendants can request independently. `KeepAliveHandle::request(context)`
returns an optional UI-local owned request for imperative clients. Acquire it in
build or a dependency hook, retain it in component state, and drop it or call
`release()` when no longer needed. Release is idempotent. The final release
schedules removal if the item is outside the cache; releasing one of several
requests does not remove it. Outside an enabled item scope the result is none;
initialization cannot subscribe to inherited values, so requesting from
`init_state` returns the normal dependency-phase error.

Imperative clients must refresh their request if an inherited item scope changes
or `is_active()` becomes false. Removed items and an unmounted list invalidate
outstanding requests; a handle does not keep an unmounted tree or its state alive.
`add_automatic_keep_alives(false)` ignores descendant requests and places an
explicit scope boundary around each row, preventing nested lists from requesting
retention of an outer row accidentally. It is enabled by default, but rows are
retained only when something requests it. The backend TextEdit does not yet
request keep-alive automatically; wrap editing rows explicitly.

Use stable `item_key` values for data identity. When data can reorder, supply
`find_child_index_callback(|key| ...)`, returning the key's current index or none
if deleted. Cached items follow valid mappings, and deleted keys dispose even
with outstanding requests. Returned indices are checked against item count and
`item_key`. Without a mapping, an off-cache item is retained only if its previous
index still has its key; arbitrary reordering may discard it. Work visits the
current range and previously realized entries, not every item in the dataset.

Parked items remain in the ordinary element tree under [Offstage](offstage.md).
They are measured with row constraints, excluded from hit paths, clipped from
raster output and desktop accessibility, and retain focused keyboard activity.
Inherited changes, explicit builder rebuilds and descendant state updates remain
active. Each item builder has its own build context. Pure scrolling reuses the
parked description; changing list configuration rebuilds parked descriptions to
refresh their data. Backend callbacks and animations still execute, so retention
has CPU and memory costs and is not a skipped-layout/paint optimization.

`max_realized_items` includes visible, cached and kept-alive rows. Exceeding it
reports a checked layout error instead of silently evicting requested state.
As with other list construction error descriptions, this replaces and disposes
the item subtree and invalidates its requests; a later valid range mounts fresh
state. Choose a budget suitable for your retention policy. Retaining many rows
can still be expensive below that budget.

Thirteen headless tests cover retained identity/state/handles, ordinary disposal,
independent requests/final release, declarative withdrawal while parked,
reorder/deletion, opt-out, nested scope boundaries, inherited/explicit rebuilds,
focused hidden text input, both axes/reverse, checked budgets/recovery and unmount.
The gallery's first row owns a tap counter; native probes compare its glyphs
before and after an End/Home excursion. See [verification](verification.md) for
measured platform results.

This is a fixed-extent list retention foundation. Flutter's ParentData KeepAlive,
AutomaticKeepAlive notification/handle protocol and client mixin API, automatic
editable focus policy, sliver bucket machinery, deactivation/reactivation,
restoration, variable extents and complete member compatibility remain unfinished.
Rust handles use ownership and release rather than Dart Listenable notifications.
Reviewed the official [automatic client behavior](https://api.flutter.dev/flutter/widgets/AutomaticKeepAliveClientMixin-mixin.html),
[handle API](https://api.flutter.dev/flutter/widgets/KeepAliveHandle-class.html) and
[key-index mapping contract](https://api.flutter.dev/flutter/widgets/SliverChildBuilderDelegate/findChildIndexCallback.html).
