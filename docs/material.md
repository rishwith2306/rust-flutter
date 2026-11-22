# Material theme foundation

`material::ColorScheme` exposes core semantic roles for primary, secondary,
tertiary, error, surface, outline, inverse surface, shadow and scrim colors.
`ThemeData` carries that scheme and can provide it to a retained widget subtree
or apply its surface, text, selection and hyperlink colors to egui defaults.

```rust
use rust_flutter::{egui, material::{ColorScheme, ThemeData}};

let theme = ThemeData {
    color_scheme: ColorScheme::dark(),
    ..ThemeData::default()
};
theme.apply(&egui::Context::default());
# let _ = theme;
```

For inherited lookup inside a retained component, call `ThemeData::of(ctx)`;
for a subtree, use `theme.provide(child)`. A custom brand palette can be made
with a `ColorScheme` struct update from `ColorScheme::light()` or `dark()`.

The first retained controls are `FilledButton`, `OutlinedButton`, and
`TextButton`. They use the nearest scheme and the framework gesture binding.
`Checkbox` is owner-controlled and supports optional tristate cycling:

```rust
use rust_flutter::{gestures::GestureBinding, material::{FilledButton, ThemeData}};

let binding = GestureBinding::new();
let save = FilledButton::new(&binding, "Save", || {}).into_widget();
let themed_save = ThemeData::default().provide(save);
# let _ = themed_save;
```

```rust
use rust_flutter::{gestures::GestureBinding, material::Checkbox};

let binding = GestureBinding::new();
let selected = Checkbox::new(&binding, Some(false), |next| {
    let _ = next;
})
.into_widget()?;
# let _ = selected;
```

For tristate selection, pass an `Option<bool>` and call `.tristate(true)`; the
callback receives the next controlled value. This first version uses check and
dash marks painted by the framework. It does not yet provide the complete
Material interaction-state or accessibility semantics surface.

`Switch` is also controlled: supply its value and update that value in the
owner after the callback. The framework paints its track and moving thumb, but
the thumb currently jumps between states without the animated slide Flutter
provides:

```rust
use rust_flutter::{gestures::GestureBinding, material::Switch};

let binding = GestureBinding::new();
let notifications = Switch::new(&binding, true, |next| {
    let _ = next;
})
.into_widget();
# let _ = notifications;
```

`Radio<T>` draws one option from an owner-managed selection group. Give every
option the same current `group_value` and handle callbacks in the owning state:

```rust
use rust_flutter::{gestures::GestureBinding, material::Radio};

let binding = GestureBinding::new();
let option = Radio::new(&binding, "compact", Some("comfortable"), |value| {
    let _ = value;
})
.into_widget();
# let _ = option;
```

This initial radio does not implement toggleable groups, focus/hover color
properties, or screen-reader selection semantics.

`Scaffold` provides a vertical page shell with an expanding body and optional
`AppBar` / bottom-navigation slots. Pass the same theme to those shell widgets
for their surface colors, and provide it above the page so buttons in the body
can resolve inherited colors:

```rust
use rust_flutter::{element::Widget, material::{AppBar, Scaffold, ThemeData}};

let theme = ThemeData::default();
let page = Scaffold::new(Widget::empty())
    .app_bar(AppBar::new("Home").theme(theme).into_widget())
    .theme(theme)
    .into_widget();
let themed_page = theme.provide(page);
# let _ = themed_page;
```

The scaffold does not add safe-area padding or keyboard insets automatically.
Compose those behaviors explicitly around the relevant child.

`NavigationBar` is controlled: provide destinations, the selected index and a
callback, then update the index in the owning state when the callback runs.
Destination icons are app-supplied text glyphs until a Material icon/font asset
catalog is implemented.

```rust
use rust_flutter::{
    element::Widget,
    gestures::GestureBinding,
    material::{NavigationBar, NavigationDestination, Scaffold, ThemeData},
};

let binding = GestureBinding::new();
let bar = NavigationBar::new(
    &binding,
    vec![
        NavigationDestination::new("⌂", "Home"),
        NavigationDestination::new("⚙", "Settings"),
    ],
    0,
    |selected_index| { let _ = selected_index; },
).into_widget();
let page = ThemeData::default()
    .provide(Scaffold::new(Widget::empty()).bottom_navigation_bar(bar).into_widget());
# let _ = page;
```

`TextField` provides themed single-line, multiline, read-only and obscured text
entry. It reads text and surface colors from the nearest `ThemeData` unless
the decoration supplies explicit colors, and retains editing state through
`TextEditingController`:

```rust
use rust_flutter::{forms::TextEditingController, material::TextField};

let controller = TextEditingController::new("");
let email = TextField::with_controller(controller)
    .label("Email")
    .hint("name@example.com")
    .into_widget()?;
# let _ = email;
```

This initial field does not yet implement the full Flutter `InputDecoration`
surface, including prefix/suffix widgets and floating-label animation.

This is an initial theme layer, not Material 3 parity. The palette is static,
does not derive tonal roles from a seed, and does not yet include every Flutter
`ColorScheme` role, `TextTheme`, component themes, `MaterialApp`, scaffold
drawers/sheets/FAB/inset handling, icon assets, or most of the Material control catalog. The
role names and app-level/subtree theming model were checked against Flutter's
[ColorScheme API](https://api.flutter.dev/flutter/material/ColorScheme-class.html).
