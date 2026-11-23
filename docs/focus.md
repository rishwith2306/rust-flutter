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

`FocusScopeNode` groups the framework controls in its `FocusScope` subtree. Its
`request_focus()` focuses the first enabled mounted target, and `next_focus()` /
`previous_focus()` cycle through targets in their current paint order. Tab and
Shift+Tab use the same cyclic traversal while a scoped target owns focus.
Requests can be made before a target mounts. Disabled, removed and unmounted
controls are excluded. Arrow keys do not use the scope's traversal order.

```rust
use rust_flutter::{focus::{FocusScope, FocusScopeNode}, forms::TextFormField};

let scope = FocusScopeNode::new();
let field = TextFormField::new("").into_widget()?;
let subtree = FocusScope::new(scope.clone(), field).into_widget();
scope.request_focus(); // queued until a descendant control mounts
# let _ = subtree;
# Ok::<(), rust_flutter::forms::TextEditingError>(())
```

This remains a partial match for Flutter's focus system. Nested-scope ancestry,
remembered focused children, traversal policies and groups, arrow-key focus
traversal, focus-ancestor shortcut resolution, and screen-reader focus semantics
remain unimplemented. A `FocusNode` attaches to one control at a time;
ordinary egui controls do not yet attach to framework focus scopes.

`element::Shortcuts` wraps a retained subtree and consumes configured egui
`KeyboardShortcut`s. A nested scope gets first chance to handle a shortcut; an
unmatched shortcut bubbles to the outer scope. Bind callbacks directly, or bind
a typed intent through a shared `element::Actions` registry:

```rust
use std::{cell::Cell, rc::Rc};
use rust_flutter::{egui, element::{Shortcuts, Widget}};

let count = Rc::new(Cell::new(0));
let action_count = count.clone();
let shortcut = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::K);
let page = Shortcuts::new(Widget::empty())
    .bind(shortcut, move || action_count.set(action_count.get() + 1))
    .into_widget();
# let _ = (count, page);
```

```rust
use std::{cell::Cell, rc::Rc};
use rust_flutter::{egui, element::{Actions, Shortcuts, Widget}};

struct SaveIntent { document_id: u64 }
let actions = Actions::new();
let saved = Rc::new(Cell::new(0));
let saved_count = saved.clone();
actions.register::<SaveIntent>(move |intent| {
    let _ = intent.document_id;
    saved_count.set(saved_count.get() + 1);
});
let save = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::S);
let page = Shortcuts::new(Widget::empty())
    .bind_intent(save, actions.clone(), SaveIntent { document_id: 7 })
    .into_widget();
# let _ = (actions, saved, page);
```

This subtree scope is active while mounted and does not yet check which child
owns focus. `Actions` matches intents by concrete Rust type and invokes the
registered handler synchronously. Focus-based action lookup, inherited action
scopes, and platform-specific shortcut policy remain future work.

The API shape was checked against the official [Flutter Focus](https://api.flutter.dev/flutter/widgets/Focus-class.html)
and [FocusNode](https://api.flutter.dev/flutter/widgets/FocusNode-class.html)
references. Behavior is backend-specific and must be tested with physical
keyboards on macOS and Android; current evidence covers headless egui focus.

`crates/rust-flutter/tests/forms.rs` covers queued focus requests, notifier
changes, text/button targets, scope order, explicit and Tab / Shift+Tab cycling,
disabled targets and target removal. These are headless framework checks, not
device keyboard or accessibility evidence.
