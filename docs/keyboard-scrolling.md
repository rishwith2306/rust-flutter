# Keyboard scrolling

Retained `SingleChildScrollView` viewports are keyboard focus targets by default.
Tab into a viewport, or tap its blank content, then use these keys:

| Key | Movement |
| --- | --- |
| Up/Down for vertical, Left/Right for horizontal | 50 logical points |
| PageUp/PageDown | 80% of the viewport length |
| Space / Shift+Space | One page forward / backward |
| Home / End | Minimum / maximum scroll extent |

Reverse viewports reverse directional and page movement. Home and End still
select the minimum and maximum offsets. Movement clamps to current dimensions
and immediately cancels ongoing programmatic or ballistic motion. These are
immediate jumps; platform smooth keyboard scrolling remains future work.

```rust
use rust_flutter::{egui, element::Widget, gestures::GestureBinding,
    scrolling::{ScrollPosition, SingleChildScrollView}};
let ctx = egui::Context::default();
let position = ScrollPosition::new(&ctx, 0.0)?;
let binding = GestureBinding::new();
let view = SingleChildScrollView::new(&position, &binding)?
    .keyboard_scrolling(true)
    .into_widget(Widget::leaf(egui::vec2(200.0, 600.0), |ui| {
        ui.painter().rect_filled(ui.max_rect(), 0.0, egui::Color32::LIGHT_BLUE);
    }))?;
# let _ = view;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Mount and paint the view with the same binding as shown in the
[viewport guide](scroll-viewports.md). Retain the position and binding between
frames. `.keyboard_scrolling(false)` removes the viewport focus target and its
focus-tap recognizer; pan and wheel input continue to work.

Focus identity follows the retained element, including parent UI scope changes.
Tab leaves the viewport normally. Focused viewports show corner markers inside
the clip. A focused descendant text field or button keeps its own keyboard input;
tapping an ordinary backend control preserves the focus it acquires. Controls
using the gesture arena compete normally with the viewport's tap recognizer.
Disabled, hidden, omitted, unmounted or unfocused viewports do not handle keys.
Ctrl, Command and Alt combinations, unsupported Shift combinations and
cross-axis arrow keys remain available to other handlers.

Key presses are processed in input order after painting and dimension delivery,
outside render-object borrows. Repeated backend passes do not replay consumed
keys. Reentrant listeners can change focus or unmount the view; later key events
then remain unconsumed. Native key repeat uses the backend's pressed events.

[`FocusNode` and `FocusScopeNode`](focus.md) can request/observe framework
control focus and explicitly traverse a scope's current targets. Scope
traversal is not wired to Tab or arrow keys. This does not implement Flutter's
focus ancestry, Shortcuts, Actions, ScrollIntent or full ScrollAction APIs.
Descendant-to-ancestor keyboard bubbling, coordinated nested scrolling,
configurable shortcut maps, platform key conventions and accessibility
scrolling remain unfinished. macOS runtime keyboard behavior and physical
Android keyboards need separate evidence.
The line/page increments follow the documented
[Flutter ScrollAction defaults](https://api.flutter.dev/flutter/widgets/ScrollAction-class.html).

Fifteen headless tests cover ordering, repeat passes, both axes/reverse, modifier
handling, focus traversal and preservation, disabled/omitted views, cancellation,
resize, unmount and reentrant listeners. Native smoke probes inject End, Home,
PageDown and ArrowDown and require rendered movement followed by exact reset.
Their `native_keyboard` result is true only after those pixel checks pass.
Both Linux X11 and API 35 / x86_64 Android emulator artifacts pass these checks
at revision `350d929`; see the [exact CI runtime record](verification.md#keyboard-scrolling-ci-runtime-result).
