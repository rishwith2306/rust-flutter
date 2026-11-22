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

`MaterialApp` is a root convenience widget that provides `ThemeData` to its
home subtree. It can be passed to `run_widget`; this initial implementation does
not yet add Flutter's named route table, system theme selection, or
localization configuration. `MaterialApp::with_navigator` connects a retained
`NavigatorController` to the root page builder; see [retained navigation](navigation.md).

`FilledButton`, `OutlinedButton`, and `TextButton` use the nearest scheme and
the framework gesture binding. `Checkbox` is owner-controlled and supports
optional tristate cycling:

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

`Slider` supports controlled continuous values and discrete intervals. Taps on
the track and horizontal drags update the owner with clamped values:

```rust
use rust_flutter::{gestures::GestureBinding, material::Slider};

let binding = GestureBinding::new();
let volume = Slider::new(&binding, 0.5, |value| {
    let _ = value;
})
.range(0.0, 1.0)
.divisions(Some(10))
.into_widget()?;
# let _ = volume;
```

Optional `.on_change_start` and `.on_change_end` callbacks report the value at
the beginning and end of a drag. Discrete sliders take the number of intervals
with `.divisions(Some(count))`.

This first version does not provide value indicators, secondary tracks,
keyboard adjustments, or slider accessibility semantics.

`Scaffold` provides a vertical page shell with an expanding body, optional
`AppBar` / bottom-navigation slots, and a controlled modal `Drawer`. The owner
supplies whether the drawer is open and updates that state from `on_open` and
`on_dismiss`. `.app_bar_config(AppBar)` adds a menu action when a drawer is
configured and the app bar has no explicit leading widget; `.app_bar(Widget)`
keeps its custom leading behavior. The drawer remains mounted while closed.
Pass the same theme to those shell widgets for their surface colors, and
provide it above the page so buttons in the body can resolve inherited colors:

```rust
use rust_flutter::{
    element::Widget,
    gestures::GestureBinding,
    material::{AppBar, Drawer, NavigationDrawer, NavigationDrawerDestination, Scaffold, ThemeData},
    text::Text,
};

let theme = ThemeData::default();
let binding = GestureBinding::new();
let destinations = NavigationDrawer::new(
    &binding,
    vec![
        NavigationDrawerDestination::new("⌂", "Home"),
        NavigationDrawerDestination::new("▣", "Projects"),
        NavigationDrawerDestination::new("⚙", "Settings"),
    ],
    Some(0),
    |index| { let _ = index; },
)
.header(Text::new("WORKSPACE").into_widget())
.theme(theme)
.into_widget()?;
let drawer = Drawer::new(destinations).theme(theme).into_widget()?;
let page = Scaffold::new(Widget::empty())
    .app_bar_config(AppBar::new("Home").theme(theme))
    .drawer(&binding, drawer, false, || {}, || {})
    .theme(theme)
    .into_widget();
let themed_page = theme.provide(page);
# let _ = themed_page;
```

The runnable [`Material` showcase](../examples/material) shows an app-bar action
opening the drawer, selected destination updates, and scrim dismissal. The
app bar gets its menu action from `app_bar_config`. The drawer has no entrance
animation, edge-swipe gesture, end-drawer counterpart, or native-back
integration yet. The scaffold does not add keyboard insets automatically;
compose that behavior explicitly around the relevant child. Flutter's [Drawer](https://api.flutter.dev/flutter/material/Drawer-class.html)
and [Scaffold](https://api.flutter.dev/flutter/material/Scaffold-class.html)
also provide automatic app-bar integration, swipe handling, and route-based
dismissal that this implementation does not yet match.

`NavigationDrawer` provides a controlled selected destination, optional header
and footer, selected icon glyph, disabled items, and Material 3 selected
colors. Destination icons are text glyphs, and the destination list does not
scroll or virtualize yet. Flutter also supports arbitrary children and theme,
shape, elevation, and semantics customization through [NavigationDrawer](https://api.flutter.dev/flutter/material/NavigationDrawer-class.html)
and [NavigationDrawerDestination](https://api.flutter.dev/flutter/material/NavigationDrawerDestination-class.html).

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
sheets/inset handling, icon assets, or most of the Material control catalog. The
role names and app-level/subtree theming model were checked against Flutter's
[ColorScheme API](https://api.flutter.dev/flutter/material/ColorScheme-class.html).

`ListTile` composes leading, title, optional subtitle, and trailing widgets in a
fixed-height row. It supports dense and three-line sizing, selection colors,
disabled state, custom padding, and a whole-row tap callback:

```rust
use rust_flutter::{
    gestures::GestureBinding,
    material::{ListTile, ThemeData},
    text::Text,
};

let binding = GestureBinding::new();
let tile = ListTile::new(&binding, Text::new("Account").into_widget())
    .leading(Text::new("◉").into_widget())
    .subtitle(Text::new("Profile and sign-in").into_widget())
    .trailing(Text::new("›").into_widget())
    .on_tap(|| {})
    .theme(ThemeData::default())
    .into_widget()?;
# let _ = tile;
```

The tile currently has no ink splash, pressed/focus/hover state or semantic
button role. Flutter exposes those states and additional alignment, color and
accessibility options through its [ListTile API](https://api.flutter.dev/flutter/material/ListTile-class.html).

`FloatingActionButton` builds a themed circular action button, with mini, large
and extended variants. `Scaffold::floating_action_button` places it over the body
near the bottom-right corner, above the optional bottom navigation bar:

```rust
use rust_flutter::{
    gestures::GestureBinding,
    material::{FloatingActionButton, Scaffold},
    text::Text,
};

let binding = GestureBinding::new();
let page = Scaffold::new(body)
    .floating_action_button(FloatingActionButton::new(
        &binding,
        Text::new("+").into_widget(),
        || {},
    ));
# let _ = page;
```

This is an initial implementation: it has no elevation/shadow, tooltip,
hero transition, disabled state, focus/hover/pressed feedback, or semantic
button role. Flutter's API includes those options and separate small, regular,
large, and extended forms ([FloatingActionButton API](https://api.flutter.dev/flutter/material/FloatingActionButton-class.html)).

`AlertDialog` builds a Material panel. `MaterialDialogHost` retains the page,
shows a selected dialog above it, and dismisses that dialog when the user taps
the scrim. Its builder receives a controller on every build, so a page action
can present a dialog and dialog actions can dismiss it:

```rust
use rust_flutter::{
    element::Widget,
    gestures::GestureBinding,
    material::{AlertDialog, MaterialDialogHost},
    text::Text,
};

let binding = GestureBinding::new();
let host = MaterialDialogHost::new(&binding, |controller, selected| {
    let page = build_page(controller.clone());
    let dialog = (selected == Some(1)).then(|| {
        AlertDialog::new(Text::new("Continue?").into_widget())
            .content(Text::new("The operation can be cancelled.").into_widget())
            .into_widget()
            .expect("valid dialog")
    });
    (page, dialog)
});
# fn build_page(_: rust_flutter::material::MaterialDialogController) -> Widget { Widget::empty() }
# let _ = host;
```

An action may use `controller.dismiss_with_result(value)`; the page can then
read that value with `controller.take_result::<T>()`. The host can disable
scrim dismissal with `.barrier_dismissible(false)` and set its scrim color.
It still has no route transition, alert icon, or automatic scrolling, and
Android back handling must be wired by the app. Flutter's
[AlertDialog API](https://api.flutter.dev/flutter/material/AlertDialog-class.html)
is displayed with `showDialog`, which also owns route and result delivery.
