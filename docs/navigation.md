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
let pages = navigator.retained_view(move |_: &BuildContext, route| {
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

`NavigatorController::view` builds only the current route. Use
`NavigatorController::retained_view` when inactive routes must stay mounted;
it keeps every route offstage with its own element key and mutes inactive route
tickers. `CupertinoTabScaffold` uses this retained view for each tab's stack.

Use a shared `BackDispatcher` for route stacks and modal overlays. The highest-priority
live handler receives Back first; equal priorities use newest-first order. A
handler returns `true` when it handled the event.
`NavigatorController::register_back_handler` pops one page and returns false at
the root. Keep the subscription alive for as long as the route stack is active:

```rust
use rust_flutter::{
    element::{BuildContext, Widget},
    material::MaterialApp,
    navigation::{BackDispatcher, NavigatorController},
};

let navigator = NavigatorController::new("home");
let root = MaterialApp::new(navigator.retained_view(|_: &BuildContext, route| {
    match *route {
        "home" => Widget::empty(),
        "settings" => Widget::empty(),
        _ => Widget::empty(),
    }
})).into_widget();
let back = BackDispatcher::new();
let _route_back = navigator.register_back_handler(&back);
let handle_back = back.handler();
# let _ = (root, handle_back);
```

Pass `handle_back` to `run_widget_with_back_handler` or
`run_widget_with_options_and_back`. `MaterialDialogHost`,
`ModalBottomSheetHost` and `CupertinoModalPopup` can receive the same dispatcher
with `.back_dispatcher(&back)`. They use modal priority and therefore dismiss
before the route stack pops the current page.

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

For string-based routing, register a checked table of named page builders. The
controller still owns navigation state, so pushing a route name updates the
retained view:

```rust
use rust_flutter::{
    element::{BuildContext, Widget},
    material::{MaterialApp, NamedRoute},
    navigation::NavigatorController,
};

let navigator = NavigatorController::new("home".to_owned());
let routes = vec![
    NamedRoute::new("home", |_: &BuildContext| Widget::empty()),
    NamedRoute::new("settings", |_: &BuildContext| Widget::empty()),
];
let app = MaterialApp::with_named_routes(&navigator, routes)?;
let _ = navigator.push("settings".to_owned());
# let _ = app;
```

An empty name, duplicate route name, or unregistered initial route returns a
`RouteConfigurationError`. Later pushes to unknown names display the default
fallback; `with_named_routes_and_fallback` accepts an app-specific fallback.

Page transitions, deep links, route restoration and declarative Router
integration remain future work.
