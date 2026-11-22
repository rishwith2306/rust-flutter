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
        CupertinoActivityIndicator, CupertinoPageScaffold, CupertinoSlider,
        CupertinoSegment, CupertinoSegmentedControl, CupertinoSwitch,
        CupertinoTextField, CupertinoThemeData,
    },
    element::Widget,
    gestures::GestureBinding,
};

let binding = GestureBinding::new();
let theme = CupertinoThemeData::default();
let button = CupertinoButton::filled(&binding, "Continue", || {}).into_widget();
let field = CupertinoTextField::new("").placeholder("Email").into_widget()?;
let switch = CupertinoSwitch::new(&binding, true, |enabled| {
    let _ = enabled;
})
.into_widget();
let volume = CupertinoSlider::new(&binding, 0.5, |value| {
    let _ = value;
})
.range(0.0, 1.0)
.divisions(Some(10))
.into_widget()?;
let loading = CupertinoActivityIndicator::new().into_widget()?;
let density = CupertinoSegmentedControl::new(
    &binding,
    vec![
        CupertinoSegment::new("compact", "Compact"),
        CupertinoSegment::new("comfortable", "Comfortable"),
    ],
    "compact",
    |value| { let _ = value; },
)?
.into_widget()?;
let content = Widget::flex(
    rust_flutter::element::FlexStyle::new(rust_flutter::layout::Axis::Vertical),
    vec![
        rust_flutter::element::FlexItem::fixed(field),
        rust_flutter::element::FlexItem::fixed(switch),
        rust_flutter::element::FlexItem::fixed(volume),
        rust_flutter::element::FlexItem::fixed(loading),
        rust_flutter::element::FlexItem::fixed(density),
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
or keyboard accessory APIs. The switch is controlled and uses the iOS green on
state. The slider supports continuous or discrete ranges, drag callbacks, and
theme-based Cupertino track colors while reusing the shared gesture engine.
The activity indicator draws a rotating radial tick pattern and supports a
static partially revealed progress value. Switch thumb animation, slider value
indicators and semantics, accessibility, and the broader Cupertino component
catalog remain to be implemented.

This is not full Cupertino parity. `CupertinoApp` currently provides theme data
only; routing, localization, system appearance following, dialogs, and pickers
are separate work. Compare the official Flutter [CupertinoApp](https://api.flutter.dev/flutter/cupertino/CupertinoApp-class.html)
and [CupertinoButton](https://api.flutter.dev/flutter/cupertino/CupertinoButton-class.html)
APIs for the broader Flutter surface. The control references include
[CupertinoActivityIndicator](https://api.flutter.dev/flutter/cupertino/CupertinoActivityIndicator-class.html),
[CupertinoSlider](https://api.flutter.dev/flutter/cupertino/CupertinoSlider-class.html),
[CupertinoSegmentedControl](https://api.flutter.dev/flutter/cupertino/CupertinoSegmentedControl-class.html),
[CupertinoSwitch](https://api.flutter.dev/flutter/cupertino/CupertinoSwitch-class.html),
and [CupertinoTextField](https://api.flutter.dev/flutter/cupertino/CupertinoTextField-class.html).
