# Retained navigation

`Navigator<R>` is a small owned route stack. For retained widget trees,
`NavigatorController<R>` adds notifications and a view builder. Routes must be
cloneable and comparable so stack changes can be detected.

```rust
use rust_flutter::{
    element::{BuildContext, Widget},
    navigation::NavigatorController,
};

let navigator = NavigatorController::new("home");
let controller = navigator.clone();
let pages = navigator.view(move |_: &BuildContext, route| {
    match *route {
        "home" => Widget::empty(),
        "settings" => Widget::empty(),
        _ => Widget::empty(),
    }
});
let _ = controller.push("settings");
let _ = controller.pop();
# let _ = pages;
```

The root route cannot be popped. `push`, `pop`, `replace`, and `reset` notify
the retained route view, which rebuilds the current page while preserving
matching keyed state in the tree. The controller is UI-local; it is not a
thread-safe navigation service.

Connect the stack to native back requests with `run_widget_with_back_handler`.
Return `true` after a page was popped; return `false` at the root to let the
host close the app:

```rust
use rust_flutter::{
    element::{BuildContext, Widget},
    material::MaterialApp,
    navigation::NavigatorController,
};

let navigator = NavigatorController::new("home");
let root = MaterialApp::new(navigator.view(|_: &BuildContext, route| {
    match *route {
        "home" => Widget::empty(),
        "settings" => Widget::empty(),
        _ => Widget::empty(),
    }
})).into_widget();
let back_stack = navigator.clone();
let handle_back = move || {
    back_stack.pop().ok().flatten().is_some()
};
# let _ = (root, handle_back);
```

`MaterialApp::with_navigator` combines the route view and inherited theme:

```rust
use rust_flutter::{
    element::{BuildContext, Widget},
    material::{MaterialApp, ThemeData},
    navigation::NavigatorController,
};

let navigator = NavigatorController::new("home");
let root = MaterialApp::with_navigator(&navigator, |_: &BuildContext, _route| {
    Widget::empty()
})
    .theme(ThemeData::default())
    .into_widget();
# let _ = root;
```

Pass a clone of the same controller to the back handler shown above. Page
transitions, deep links, named route tables and restoration remain future work.
