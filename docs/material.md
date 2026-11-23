# Material theme foundation

`material::ColorScheme` exposes Flutter's current primary, secondary, tertiary,
error, fixed-tone, surface-container, outline, inverse, shadow and scrim roles,
plus the deprecated `background`, `on_background` and `surface_variant` aliases.
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

The light and dark palettes are static purple-seeded presets; dynamic
`ColorScheme.fromSeed`, high-contrast schemes, and OS theme following are not
implemented. The [Flutter ColorScheme API](https://api.flutter.dev/flutter/material/ColorScheme-class.html)
documents the roles and their intended use.

For inherited lookup inside a retained component, call `ThemeData::of(ctx)`;
for a subtree, use `theme.provide(child)`. A custom brand palette can be made
with a `ColorScheme` struct update from `ColorScheme::light()` or `dark()`.

`MaterialApp` is a root convenience widget that provides `ThemeData` to its
home subtree. It can be passed to `run_widget`. `MaterialApp::with_navigator`
connects a retained `NavigatorController` to the root page builder, and
`with_named_routes` adds a checked string route table. Localization catalogs can be attached with `.localizations(catalog)` and read
through `Localizations::of(context)`; platform locale discovery and automatic
RTL directionality remain unfinished. See [localization](localization.md) and
[retained navigation](navigation.md).

Progress indicator defaults can be configured once in the inherited theme.
Indicator-specific setters take precedence over these shared values:

```rust
use rust_flutter::{egui::Color32, material::{ProgressIndicatorThemeData, ThemeData}};

let theme = ThemeData {
    progress_indicator_theme: ProgressIndicatorThemeData {
        color: Some(Color32::from_rgb(0x22, 0x66, 0xAA)),
        linear_min_height: Some(6.0),
        stroke_width: Some(5.0),
        stroke_align: Some(-1.0),
        stroke_cap: Some(rust_flutter::material::StrokeCap::Round),
        ..ProgressIndicatorThemeData::default()
    },
    ..ThemeData::default()
};
# let _ = theme;
```

The shared theme currently supports indicator and track colors, linear height,
circular stroke width, stroke alignment and cap, corner radius, track gap,
linear stop marker styling, and a shared animation controller. The controller
can be set in `ThemeData.progress_indicator_theme` for the whole app, or in a
`ProgressIndicatorTheme` wrapper for one subtree. Descendant indeterminate
indicators use that controller unless an indicator supplies its own.
Flutter's [ProgressIndicatorThemeData API](https://api.flutter.dev/flutter/material/ProgressIndicatorThemeData-class.html)
also provides constraints, circular track padding, refresh colors, animated
value colors and theme interpolation; those properties remain unavailable.
For an app-wide controller, set the theme field alongside the visual defaults:

```rust
use rust_flutter::{animation::AnimationController, material::{ProgressIndicatorThemeData, ThemeData}};

let controller = AnimationController::new();
let theme = ThemeData {
    progress_indicator_theme: ProgressIndicatorThemeData {
        controller: Some(controller.clone()),
        ..ProgressIndicatorThemeData::default()
    },
    ..ThemeData::default()
};
# let _ = (theme, controller);
```

Use the `ProgressIndicatorTheme` wrapper to scope the same configuration to a
subtree:

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
Material interaction-state surface. Desktop AccessKit nodes expose checkbox
role and checked/mixed state when `.semantics_label(...)` is supplied. The Android
AccessKit bridge is implemented; TalkBack device validation remains pending.

`Switch` is also controlled: supply its value and update that value in the
owner after the callback. The framework animates the thumb to the new value
over 200 ms when the owner rebuilds, while the track and thumb colors update
immediately. On desktop the control exposes the AccessKit switch role, an
optional accessible name, and its toggled state. The Android AccessKit bridge
is implemented; TalkBack device validation remains pending. Flutter's controlled `Switch` also waits for the owner to rebuild
with its new value after invoking `onChanged` ([Flutter Switch API](https://api.flutter.dev/flutter/material/Switch-class.html)):

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
option the same current `group_value` and handle callbacks in the owning state.
On desktop it exposes the radio role and selection state, with an optional
accessible label. The Android AccessKit bridge is implemented; TalkBack device
validation remains pending:

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

This first version does not provide value indicators or secondary tracks. Sliders
can take keyboard focus and respond to arrow, Page Up/Down, Home and End keys. They
expose an AccessKit slider role, numeric value and range, increment/decrement, and
set-value actions. Use `.semantics_label(...)` when surrounding content does not
name the slider.

`Scaffold` provides a vertical page shell with an expanding body, optional
`AppBar`, persistent `bottom_sheet`, bottom-navigation slots, and a controlled
modal `Drawer`. A persistent sheet stays visible while the user interacts with
the body. It can be any widget; `Scaffold.bottom_sheet` places it between the
body and bottom-navigation bar. Modal sheets, which block interaction and
dismiss through a route, are a separate feature. Flutter describes the same
distinction on [`Scaffold.bottomSheet`](https://api.flutter.dev/flutter/material/Scaffold/bottomSheet.html).
The owner
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
    .bottom_sheet(Text::new("Persistent panel").into_widget())
    .drawer(&binding, drawer, false, || {}, || {})
    .theme(theme)
    .into_widget();
let themed_page = theme.provide(page);
# let _ = themed_page;
```

The runnable [`Material` showcase](../examples/material) shows an app-bar action
opening the drawer, selected destination updates, and scrim dismissal. The
app bar gets its menu action from `app_bar_config`, which builds an `IconButton`.
The drawer has no entrance
animation, edge-swipe gesture, end-drawer counterpart, or native-back
integration yet. The scaffold does not add keyboard insets automatically;
compose that behavior explicitly around the relevant child. Flutter's [Drawer](https://api.flutter.dev/flutter/material/Drawer-class.html)
and [Scaffold](https://api.flutter.dev/flutter/material/Scaffold-class.html)
also provide automatic app-bar integration, swipe handling, and route-based
dismissal that this implementation does not yet match.

`IconButton` accepts an icon widget and callback and supports standard, filled,
filled-tonal and outlined variants. Selection is controlled by the owner, which
may supply a separate selected icon:

```rust
use rust_flutter::{
    gestures::GestureBinding,
    material::{IconButton, IconButtonVariant, ThemeData},
    text::Text,
};

let binding = GestureBinding::new();
let favorite = IconButton::new(&binding, Text::new("♡").into_widget(), || {})
    .selected_icon(Text::new("♥").into_widget())
    .selected(true)
    .variant(IconButtonVariant::FilledTonal)
    .theme(ThemeData::default())
    .into_widget()?;
# let _ = favorite;
```

The widget currently has no tooltip, ink splash, focus/hover feedback, or
semantic button role. Material Icons are not bundled; applications supply an
icon widget or asset. Flutter's [IconButton API](https://api.flutter.dev/flutter/material/IconButton-class.html)
also offers these accessibility and interaction features.

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
hero transition, disabled state, or focus/hover/pressed feedback. It exposes a
button role and click action; use `.semantics_label("Create new item")` for an
icon-only child. Flutter's API includes those options and separate small,
regular, large, and extended forms ([FloatingActionButton API](https://api.flutter.dev/flutter/material/FloatingActionButton-class.html)).

`Card` groups related content in an elevated, filled, or outlined rounded
surface. The card adds its default four-point outside margin and can be made a
single tap target:

```rust
use rust_flutter::{
    element::Widget,
    material::{Card, CardVariant, ThemeData},
};

let card = Card::new(Widget::empty())
    .variant(CardVariant::Outlined)
    .theme(ThemeData::default())
    .into_widget()?;
# let _ = card;
```

The variants currently use simple fills and borders. Elevation shadows,
surface tint, child clipping, ink feedback, semantic grouping, and card theme
data remain incomplete. Flutter's [Card API](https://api.flutter.dev/flutter/material/Card-class.html)
provides those shape and elevation options.

`Divider` and `VerticalDivider` draw thin themed lines with configurable extent,
thickness, and leading/trailing insets:

```rust
use rust_flutter::material::{Divider, VerticalDivider};

let separator = Divider::new().indent(16.0).end_indent(16.0).into_widget()?;
let column_separator = VerticalDivider::new().width(32.0).into_widget()?;
# let _ = (separator, column_separator);
```

Rounded line ends and automatic resolution from a nearest inherited theme are
not implemented yet. See Flutter's [Divider](https://api.flutter.dev/flutter/material/Divider-class.html)
and [VerticalDivider](https://api.flutter.dev/flutter/material/VerticalDivider-class.html)
APIs for their full sizing and styling options.

`LinearProgressIndicator` fills the finite horizontal space allowed by its
parent. Supply `Some(value)` for progress from zero to one, or `None` for the
indeterminate sweep. The value and minimum height are validated; the theme
provides the default indicator and track colors.

```rust
use rust_flutter::material::{LinearProgressIndicator, MaterialApp, ThemeData};

let progress = LinearProgressIndicator::new(Some(0.65))
    .min_height(6.0)
    .border_radius(3.0)
    .track_gap(4.0)
    .stop_indicator_radius(3.0)
    .semantics_label("Download progress")
    .into_widget()?;
let loading = LinearProgressIndicator::new(None).into_widget()?;
let app = MaterialApp::new(progress).theme(ThemeData::default()).into_widget();
# let _ = (loading, app);
# Ok::<(), Box<dyn std::error::Error>>(())
```

The linear indicator supports configurable corner radius, track gap, and an
optional completion marker. Its indeterminate mode animates two eased segments
across the track. Flutter's [LinearProgressIndicator API](https://api.flutter.dev/flutter/material/LinearProgressIndicator-class.html)
also includes animation controllers, coordinated Material progress
indicator themes and versioned appearances. A controller can be supplied per
indicator; see the synchronization example below.

`CircularProgressIndicator` provides the matching circular form with a
determinate arc or rotating indeterminate arc. Set `size` and `stroke_width` to
fit the surrounding layout:

```rust
use rust_flutter::material::{CircularProgressIndicator, MaterialApp, ThemeData};

let progress = CircularProgressIndicator::new(Some(0.65))
    .size(48.0)
    .stroke_width(5.0)
    .stroke_align(-1.0)
    .track_gap(4.0)
    .stroke_cap(rust_flutter::material::StrokeCap::Round)
    .semantics_label("Loading")
    .into_widget()?;
let loading = CircularProgressIndicator::new(None).into_widget()?;
let app = MaterialApp::new(progress).theme(ThemeData::default()).into_widget();
# let _ = (loading, app);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Both modes expose a progress-bar role, optional label/value, and a percentage
value for determinate progress through the desktop host and Android AccessKit
bridge. TalkBack device validation remains pending. Both modes share the checked
value range and theme colors. Stroke alignment
supports the range from inside (-1) to outside (1), and caps can be butt, round,
or square. The track gap can be set per indicator or in the shared theme.
Controllers can synchronize indicators directly, through root `ThemeData`, or
through the subtree `ProgressIndicatorTheme`. Circular track padding and
versioned Material appearance remain unfinished. See Flutter's
[CircularProgressIndicator API](https://api.flutter.dev/flutter/material/CircularProgressIndicator-class.html).

Pass clones of the same `AnimationController` to indeterminate indicators to
keep them in sync. Its normalized value drives both the linear sweep and the
circular rotation:

```rust
use rust_flutter::{
    animation::{AnimationConfig, AnimationController, RepeatOptions},
    egui,
    material::{
        CircularProgressIndicator, LinearProgressIndicator,
        ProgressIndicatorTheme, ProgressIndicatorThemeData,
    },
};
use std::time::Duration;

let ctx = egui::Context::default();
let controller = AnimationController::new(&ctx, AnimationConfig {
    duration: Duration::from_secs(2),
    ..AnimationConfig::default()
})?;
let _run = controller.repeat(RepeatOptions::default())?;
let linear = LinearProgressIndicator::new(None).into_widget()?;
let circular = CircularProgressIndicator::new(None)
    .controller(controller.clone())
    .into_widget()?;
let subtree = ProgressIndicatorTheme::new(linear)
    .data(ProgressIndicatorThemeData {
        color: Some(egui::Color32::from_rgb(0x22, 0x66, 0xAA)),
        ..ProgressIndicatorThemeData::default()
    })
    .controller(controller.clone())
    .into_widget();
# let _ = (subtree, circular);
# Ok::<(), Box<dyn std::error::Error>>(())
```

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
system Back can be connected through the shared `BackDispatcher`. Flutter's
[AlertDialog API](https://api.flutter.dev/flutter/material/AlertDialog-class.html)
is displayed with `showDialog`, which also owns route and result delivery.

`ModalBottomSheetHost` provides a retained modal sheet with the same page and
typed-result pattern. It positions a themed sheet at the bottom of the viewport,
adds a scrim, and supports a configurable height limit and barrier dismissal:

```rust
use rust_flutter::{
    element::Widget,
    gestures::GestureBinding,
    material::ModalBottomSheetHost,
    text::Text,
};

let binding = GestureBinding::new();
let host = ModalBottomSheetHost::new(&binding, |controller, is_presented| {
    let page = build_page(controller.clone());
    let sheet = is_presented.then(|| Text::new("Choose an option").into_widget());
    (page, sheet)
})
.max_height(560.0)?;
# fn build_page(_: rust_flutter::material::ModalBottomSheetController) -> Widget { Widget::empty() }
# let _ = host;
```

Pass `.back_dispatcher(&back)` to connect this sheet to a shared
`BackDispatcher`; Back dismisses this overlay before a registered route handler
gets the request. The host shows a drag handle by default. Dragging down at least 72 logical points
or releasing it downward at fling speed dismisses the sheet; `.enable_drag(false)`
disables this, and `.show_drag_handle(false)` hides the handle. The handle owns
the drag recognizer, so vertical scrolling inside the sheet remains available.

Sheet actions can call `dismiss_with_result(value)`; the page reads it with
`take_result::<T>()`. Dragging does not animate or follow the pointer yet;
route/back integration, transitions, keyboard insets, and Flutter's full shape
and elevation options remain to be implemented. See Flutter's
[showModalBottomSheet API](https://api.flutter.dev/flutter/material/showModalBottomSheet.html).
