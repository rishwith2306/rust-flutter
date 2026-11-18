# Fixed-extent lazy lists

`scrolling::ListView::builder` builds rows near the viewport from a finite item
count and a fixed positive item extent. It uses the same externally owned
ScrollPosition and GestureBinding as retained single-child scrolling, including
pan, wheel, keyboard and scrollbar input. It fills both bounded parent axes.

```rust
use rust_flutter::{egui, element::{ElementTree, Widget}, gestures::GestureBinding,
    layout::BoxConstraints, scrolling::{ListView, ScrollPosition}};
use std::{cell::Cell, rc::Rc};
let ctx = egui::Context::default();
ctx.begin_pass(egui::RawInput::default());
let position = ScrollPosition::new(&ctx, 0.0)?;
let binding = GestureBinding::new();
let calls = Rc::new(Cell::new(0));
let observed = calls.clone();
let list = ListView::builder(&position, &binding, 1_000_000, 24.0,
    move |_, index| {
        observed.set(observed.get() + 1);
        Widget::leaf(egui::vec2(320.0, 24.0), move |ui| {
            ui.label(format!("Row {index}"));
        })
    })?
    .cache_extent(48.0)?
    .into_widget()?;
let mut tree = ElementTree::new(&ctx, list)?;
tree.layout(BoxConstraints::tight(egui::vec2(320.0, 240.0))?)?;
let _ = ctx.end_pass();
assert_eq!(calls.get(), 12);
assert_eq!(position.dimensions().unwrap().max_scroll_extent(), 23_999_760.0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Paint the tree with `show_in_gesture_frame` and finish the shared frame after all
roots, as in the [viewport guide](scroll-viewports.md). Wrap the list in a
[Scrollbar](scrollbars.md) using the same position/binding. Set `.axis()` and
`.reverse()` consistently on the list and scrollbar for horizontal or reversed
content. `.keyboard_scrolling(false)` disables the viewport's keyboard shortcuts.

The item extent is enforced along the scrolling axis; rows receive tight
cross-axis constraints. The content reports the full count×extent scroll range
while laying out only the realized children. Local row coordinates are rebased
near the current range before the viewport applies its offset. Positions and
dimensions still use f32 logical points, so very large offsets lose precision.
This is not a release-performance or arbitrary-range precision guarantee.

The cache extends in both directions by logical points, defaulting to 250.
`.cache_extent(0.0)` realizes only rows intersecting the viewport. A partially
visible boundary row is included. Empty and zero-area viewports build no rows;
short lists still fill the viewport, with reverse content aligned to the far
edge. Unbounded parent axes return a layout error; constrain the list's viewport.
Invalid item/cache extents and overflowing content totals return constructor
errors. `.max_realized_items()` defaults to 4096; a denser visible/cache range
returns a layout error before invoking item builders. Increase this limit
explicitly when the intended density requires it. There is no silent truncation.

Builders receive the normal inherited context and may be called again for a
cached index during a scroll-driven rebuild. Work scales with the realized
range, not the total item count. Default item identity is its index. Supply
`.item_key(|index| Key::U64(stable_data_id))` when visible data can reorder. This
key replaces the root key returned by the item builder. Duplicate keys follow
normal element validation. Items that remain in the cache retain ordinary state;
items leaving it are disposed. Automatic keep-alive and offscreen restoration
are not implemented, so durable row values belong in application state.

The caller owns the position and binding. The list does not add a controller
attachment or dispose those objects on unmount. It preserves the viewport's
exclusive position lease. Dry/intrinsic queries do not realize rows and remain
unsupported through the layout builder.

Eighteen headless tests cover a million-item dataset, large jumps, fractional
and partial rows, both axes/reverse, empty/short/zero-area lists, cached state,
stable-key reordering, inherited values, resize/count reduction, initial offset
clamping, tiny viewports at rounded large endpoints, invalid bounds/budgets, disposal and captured pan/wheel/keyboard/thumb
input. The native gallery has 10,002 rows. Probes require endpoint rendering,
drag and wheel motion, exact reset and at most nine built rows per paint pass
with its 240-point viewport and 120-point cache. Platform evidence is recorded
separately in [verification](verification.md).

This implements finite fixed-extent lazy lists. Variable/prototype extents,
shrink wrapping, padding/separators, infinite delegates, slivers/grids, automatic
keep-alive/repaint boundaries, semantics indexes, restoration and complete
ListView member compatibility remain unfinished. Reviewed the official
[ListView builder](https://api.flutter.dev/flutter/widgets/ListView/ListView.builder.html);
this Rust API is not a reproduction of its full constructor or cache-policy API.
