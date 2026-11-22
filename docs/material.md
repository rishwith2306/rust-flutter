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
`TextButton`. They use the nearest scheme and the framework gesture binding:

```rust
use rust_flutter::{gestures::GestureBinding, material::{FilledButton, ThemeData}};

let binding = GestureBinding::new();
let save = FilledButton::new(&binding, "Save", || {}).into_widget();
let themed_save = ThemeData::default().provide(save);
# let _ = themed_save;
```

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

This is an initial theme layer, not Material 3 parity. The palette is static,
does not derive tonal roles from a seed, and does not yet include every Flutter
`ColorScheme` role, `TextTheme`, component themes, `MaterialApp`, scaffold
drawers/sheets/FAB/inset handling, or most of the Material control catalog. The
role names and app-level/subtree theming model were checked against Flutter's
[ColorScheme API](https://api.flutter.dev/flutter/material/ColorScheme-class.html).
