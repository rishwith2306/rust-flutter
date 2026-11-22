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

This is an initial theme layer, not Material 3 parity. The palette is static,
does not derive tonal roles from a seed, and does not yet include every Flutter
`ColorScheme` role, `TextTheme`, component themes, `MaterialApp`, `Scaffold`, or
the Material control catalog. The role names and app-level/subtree theming model
were checked against Flutter's [ColorScheme API](https://api.flutter.dev/flutter/material/ColorScheme-class.html).
