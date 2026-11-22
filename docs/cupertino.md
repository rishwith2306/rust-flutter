# Cupertino foundation

The `cupertino` module starts a Cupertino design layer with inherited theme data,
a root widget, and owner-controlled regular and filled buttons. It uses the
framework's retained tree and shared gesture binding on desktop and Android;
it does not reproduce native UIKit/AppKit controls.

```rust
use rust_flutter::{
    cupertino::{CupertinoApp, CupertinoButton, CupertinoThemeData},
    element::Widget,
    gestures::GestureBinding,
};

let binding = GestureBinding::new();
let button = CupertinoButton::filled(&binding, "Continue", || {})
    .into_widget();
let app = CupertinoApp::new(button)
    .theme(CupertinoThemeData::default())
    .into_widget();
# let _: Widget = app;
```

The default theme uses the familiar blue accent for light and dark palettes.
Applications can provide custom primary, text, disabled, and background colors
through `CupertinoThemeData`. The initial button supports enabled state and a
custom size; hover/pressed animation, focus styling, accessibility semantics,
and the broader Cupertino component catalog remain to be implemented.

This is not full Cupertino parity. `CupertinoApp` currently provides theme data
only; routing, localization, system appearance following, safe-area policy,
page scaffolds, navigation bars, dialogs, pickers, and text fields are separate
work. Compare the official Flutter [CupertinoApp](https://api.flutter.dev/flutter/cupertino/CupertinoApp-class.html)
and [CupertinoButton](https://api.flutter.dev/flutter/cupertino/CupertinoButton-class.html)
APIs for the broader Flutter surface.
