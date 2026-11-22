# Focus

`FocusNode` is an application-owned handle that can request and observe one
descendant `TextFormField` or retained `GestureButton`. A request made before
the control mounts is queued and applied when its egui focus target attaches.

```rust
use rust_flutter::{focus::{Focus, FocusNode}, forms::TextFormField};

let name_focus = FocusNode::new();
let field = TextFormField::new("").into_widget()?;
let focused_field = Focus::new(name_focus.clone(), field)
    .autofocus(true)
    .into_widget();

assert!(name_focus.has_focus() == false); // focus resolves when mounted and painted
# let _ = focused_field;
# Ok::<(), rust_flutter::forms::TextEditingError>(())
```

`FocusNode` implements `Listenable`, so retained components can observe focus
changes. `set_can_request_focus(false)` prevents future requests and releases
the current target. `unfocus()` clears its request. `has_primary_focus()` is
currently equivalent to `has_focus()` because this implementation does not yet
build an ancestor focus tree.

This is a first focus adapter for framework controls, not Flutter's full focus system.
Focus scopes, ancestor focus aggregation, traversal policies, key bubbling,
Shortcuts, Actions, traversal groups and screen-reader focus semantics remain
unimplemented. A node attaches to one control at a time; ordinary egui controls
do not yet attach to `FocusNode`.

The API shape was checked against the official [Flutter Focus](https://api.flutter.dev/flutter/widgets/Focus-class.html)
and [FocusNode](https://api.flutter.dev/flutter/widgets/FocusNode-class.html)
references. Behavior is backend-specific and must be tested with physical
keyboards on macOS and Android; current evidence covers headless egui focus.
