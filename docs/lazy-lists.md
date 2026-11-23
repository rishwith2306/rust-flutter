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
`.drag_coefficient(value)?` configures bounded touch-release friction; values
must be strictly between zero and one, and the default is `0.135`.

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
items leaving it are disposed unless [keep-alive](keep-alive.md) is requested.
Complete automatic keep-alive APIs and offscreen restoration are not implemented;
values that must survive disposal still belong in application state.

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
separately in [verification](verification.md). Linux X11 and API 35 / x86_64
Android emulator artifacts pass these gates at revision `85c7a6e`; see the
[exact CI runtime record](verification.md#lazy-list-ci-runtime-result).
Physical devices and macOS runtime remain unverified.

This implements finite fixed-extent lazy lists. Variable/prototype extents,
shrink wrapping, padding/separators, infinite delegates, slivers, full
keep-alive APIs/repaint boundaries, semantics indexes, restoration and complete
ListView member compatibility remain unfinished. Reviewed the official
[ListView builder](https://api.flutter.dev/flutter/widgets/ListView/ListView.builder.html);
this Rust API is not a reproduction of its full constructor or cache-policy API.

## Fixed-extent lazy grids

`scrolling::GridView::builder` builds a finite grid by delegating its rows or
columns to the same lazy list viewport. Each cell has a fixed extent along the
scrolling axis and equal sizing across the cross axis:

```rust
use rust_flutter::scrolling::GridView;
use rust_flutter::layout::Axis;

let grid = GridView::builder(&position, &binding, 10_000, 3, 96.0,
    |_, index| make_card(index))?
    .main_axis_spacing(12.0)?
    .cross_axis_spacing(8.0)?
    .cache_extent(192.0)?
    .into_widget()?;

let horizontal = GridView::builder(&position, &binding, 10_000, 2, 160.0,
    |_, index| make_card(index))?
    .axis(Axis::Horizontal)
    .into_widget()?;
# let _ = horizontal;
```

The builder receives the original item index, and only groups in the viewport
and cache range are built. For vertical scrolling, `cross_axis_count` is the
column count; for horizontal scrolling, it is the row count. `main_axis_spacing`
adds space after each group, including the final group; `cross_axis_spacing`
separates cells within a group. The final group is padded with empty cells.
`.reverse(true)` reverses groups along the scrolling axis. `.max_realized_items`
defaults to 4096 and bounds the total cells in the realized and cached groups;
if the viewport/cache would exceed the budget, layout returns an error. Variable
cell extents, masonry layouts, slivers, grid-specific keep-alive and complete
Flutter `GridView` member compatibility remain unfinished. The caller owns the
position and gesture binding exactly as for `ListView`.
