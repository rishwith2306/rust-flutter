# Cupertino foundation

The `cupertino` module starts a Cupertino design layer with inherited theme data,
a root widget, a page scaffold with navigation bar slots, and owner-controlled
regular and filled buttons. It uses the
framework's retained tree and shared gesture binding on desktop and Android;
it does not reproduce native UIKit/AppKit controls.

```rust
use rust_flutter::{
    cupertino::{
        CupertinoApp, CupertinoButton, CupertinoNavigationBar,
        CupertinoPageScaffold, CupertinoTextField, CupertinoThemeData,
    },
    element::Widget,
    gestures::GestureBinding,
};

let binding = GestureBinding::new();
let theme = CupertinoThemeData::default();
let button = CupertinoButton::filled(&binding, "Continue", || {}).into_widget();
let field = CupertinoTextField::new("").placeholder("Email").into_widget()?;
let content = Widget::flex(
    rust_flutter::element::FlexStyle::new(rust_flutter::layout::Axis::Vertical),
    vec![
        rust_flutter::element::FlexItem::fixed(field),
        rust_flutter::element::FlexItem::fixed(button),
    ],
);
let page = CupertinoPageScaffold::new(content)
    .navigation_bar(CupertinoNavigationBar::new("Home").theme(theme).into_widget())
    .theme(theme)
    .into_widget();
let app = CupertinoApp::new(page).theme(theme).into_widget();
# let _: Widget = app;
```

The default theme uses the familiar blue accent for light and dark palettes.
Applications can provide custom primary, text, disabled, and background colors
through `CupertinoThemeData`. The page scaffold expands its body below an
optional 44-point navigation bar. Safe areas and system appearance integration
remain explicit application work. The initial text field uses the shared
retained editor for controller, selection, validation, and input behavior. Its
styling remains basic and does not match native insets, borders, clear-button,
or keyboard accessory APIs. The button supports enabled state and a custom size;
hover/pressed animation, accessibility semantics, and the broader Cupertino
component catalog remain to be implemented.

This is not full Cupertino parity. `CupertinoApp` currently provides theme data
only; routing, localization, system appearance following, dialogs, and pickers
are separate work. Compare the official Flutter [CupertinoApp](https://api.flutter.dev/flutter/cupertino/CupertinoApp-class.html)
and [CupertinoButton](https://api.flutter.dev/flutter/cupertino/CupertinoButton-class.html)
APIs for the broader Flutter surface.
