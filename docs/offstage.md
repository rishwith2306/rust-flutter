# Retained hidden subtrees

`element::Offstage` and `Widget::offstage` retain and measure a child while
suppressing its visible output and pointer targets. Toggle the flag on the same
widget type/key to preserve the child's element identity, state and focus.
`Offstage::default()` starts hidden.

```rust
use rust_flutter::{egui, element::{ElementTree, Widget}, layout::BoxConstraints};
fn content(hidden: bool) -> Widget {
    Widget::offstage(hidden, Widget::leaf(egui::vec2(120.0, 40.0), |ui| {
        ui.label("Retained content");
    }).keyed("content"))
}
let ctx = egui::Context::default();
let mut tree = ElementTree::new(&ctx, content(false))?;
let constraints = BoxConstraints::loose(egui::vec2(320.0, 200.0))?;
assert_eq!(tree.layout(constraints)?.size, egui::vec2(120.0, 40.0));
tree.update(content(true))?;
assert_eq!(tree.layout(constraints)?.size, egui::Vec2::ZERO);
tree.update(content(false))?;
assert_eq!(tree.layout(constraints)?.size, egui::vec2(120.0, 40.0));
# Ok::<(), Box<dyn std::error::Error>>(())
```

Wet layout always measures the child with the original parent constraints.
When hidden, the wrapper reports the constraints' minimum size, discards child
overflow and reports no baseline. Tight constraints can therefore reserve space
even while hidden. When shown, it forwards the child's size and baselines.
Hidden dry layout returns the minimum size, intrinsic queries return zero and
dry baseline queries return none without querying the child.

Hidden descendants are excluded from retained hit paths and gesture scenes.
Hiding cancels existing pointer capture, including a hidden child below a visible
root. A hidden root does not block an interactive root underneath it. Keyboard
focus is preserved: focused text edits, gesture buttons and retained scroll
viewports can still process keyboard input. Hidden viewports decline wheel
input. State updates, inherited dependencies, layout builders and element-owned
animations continue; use [TickerMode](ticker-mode.md) separately to mute tickers.

Backend UI callbacks still execute with an empty clip rectangle so focused
controls can process keyboard events. This is an explicit difference from
Flutter's skipped paint traversal: callback work and side effects still occur,
so hiding is not a performance optimization. Paint through the supplied UI and
its painter; drawing through a separate global painter can bypass that clip.
Nested retained trees inherit the hidden tag. Desktop AccessKit containers are
marked hidden, and hidden GestureButton accessibility click requests are ignored.
These metadata tests do not establish VoiceOver or TalkBack runtime parity.

Thirteen headless tests cover geometry, pure queries, keyed state and disposal,
clipped callbacks, pointer capture cancellation and root fallthrough, focused
text/button/viewport keyboard input, layout/inherited updates, AccessKit actions
and active animation lifetime. The Lazy List gallery has a hide/show button and
F8 shortcut; F8 preserves viewport focus for native input checks. Runtime
results are recorded in [verification](verification.md).

This foundation does not implement lazy-list keep-alive buckets, automatic
keep-alive clients, GlobalKey reparenting, SliverOffstage, the full Focus API,
native IME continuity or complete Flutter member compatibility. Reviewed the
[official Offstage behavior](https://api.flutter.dev/flutter/widgets/Offstage-class.html).
