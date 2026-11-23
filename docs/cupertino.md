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
        CupertinoActionSheet, CupertinoActionSheetAction,
        CupertinoAlertDialog, CupertinoDialogAction,
        CupertinoActivityIndicator, CupertinoListSection, CupertinoListTile,
        CupertinoPageScaffold, CupertinoSlider,
        CupertinoSegment, CupertinoSegmentedControl, CupertinoSwitch,
        CupertinoSlidingSegment, CupertinoSlidingSegmentedControl,
        CupertinoTabBar, CupertinoTabDestination, CupertinoTextField,
        CupertinoTabScaffold, CupertinoThemeData,
        CupertinoModalPopup,
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
let list_tile = CupertinoListTile::new(&binding, rust_flutter::text::Text::new("Privacy").into_widget())
    .subtitle(rust_flutter::text::Text::new("Manage privacy settings").into_widget())
    .trailing(rust_flutter::text::Text::new("›").into_widget())
    .on_tap(|| {})
    .notched()
    .into_widget()?;
let list_section = CupertinoListSection::new(vec![list_tile])
    .header(rust_flutter::text::Text::new("SETTINGS").into_widget())
    .inset_grouped()
    .into_widget()?;
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
let view = CupertinoSlidingSegmentedControl::new(
    &binding,
    vec![
        CupertinoSlidingSegment::new("day", "Day"),
        CupertinoSlidingSegment::new("week", "Week"),
        CupertinoSlidingSegment::new("month", "Month"),
    ],
    "week",
    |value| { let _ = value; },
)?
.into_widget()?;
let tabs = CupertinoTabBar::new(
    &binding,
    vec![
        CupertinoTabDestination::new("⌂", "Home"),
        CupertinoTabDestination::new("⚙", "Settings"),
    ],
    0,
    |index| { let _ = index; },
)
.into_widget()?;
let content = Widget::flex(
    rust_flutter::element::FlexStyle::new(rust_flutter::layout::Axis::Vertical),
    vec![
        rust_flutter::element::FlexItem::fixed(field),
        rust_flutter::element::FlexItem::fixed(switch),
        rust_flutter::element::FlexItem::fixed(volume),
        rust_flutter::element::FlexItem::fixed(loading),
        rust_flutter::element::FlexItem::fixed(list_section),
        rust_flutter::element::FlexItem::fixed(density),
        rust_flutter::element::FlexItem::fixed(view),
        rust_flutter::element::FlexItem::fixed(button),
    ],
);
let page = CupertinoPageScaffold::new(content)
    .navigation_bar(CupertinoNavigationBar::new("Home").theme(theme).into_widget())
    .tab_bar(tabs)
    .theme(theme)
    .into_widget();
let app = CupertinoApp::new(page).theme(theme).into_widget();
let navigator = rust_flutter::navigation::NavigatorController::new("home");
let routed_app = CupertinoApp::with_navigator(&navigator, |_, route| {
    rust_flutter::text::Text::new(format!("Cupertino route: {route}")).into_widget()
})
.theme(theme)
.into_widget();
let tabs_app = CupertinoTabScaffold::new(
    &binding,
    vec![
        CupertinoTabDestination::new("⌂", "Home"),
        CupertinoTabDestination::new("⚙", "Settings"),
    ],
    0,
    |_, index| match index {
        0 => Widget::empty(),
        _ => Widget::empty(),
    },
)?
.theme(theme)
.into_widget();
let (tabs_with_routes, tab_navigation) = CupertinoTabScaffold::with_navigators(
    &binding,
    vec![
        CupertinoTabDestination::new("⌂", "Home"),
        CupertinoTabDestination::new("⚙", "Settings"),
    ],
    vec![
        rust_flutter::navigation::NavigatorController::new("home"),
        rust_flutter::navigation::NavigatorController::new("settings"),
    ],
    0,
    |_: &rust_flutter::element::BuildContext, tab, route| {
        rust_flutter::text::Text::new(format!("Tab {tab}: {route}")).into_widget()
    },
)?;
let _current_tab_navigator = tab_navigation.current_navigator();
let _ = tab_navigation.push("details");
let back = rust_flutter::navigation::BackDispatcher::new();
let _tab_back = tab_navigation.register_back_handler(&back);
let _handle_back = back.handler();
let navigable_tabs = tabs_with_routes.theme(theme).into_widget();
let alert = CupertinoAlertDialog::new(
    rust_flutter::text::Text::new("Delete draft?").into_widget(),
)
.content(rust_flutter::text::Text::new("This action cannot be undone.").into_widget())
.actions(vec![
    CupertinoDialogAction::new(&binding, "Cancel", || {})
        .theme(theme)
        .into_widget()?,
    CupertinoDialogAction::new(&binding, "Delete", || {})
        .destructive_action(true)
        .theme(theme)
        .into_widget()?,
])
.theme(theme)
.into_widget()?;
let action = |label: &str| {
    CupertinoActionSheetAction::new(&binding, label, || {})
        .width(320.0)
        .theme(theme)
        .into_widget()
        .unwrap()
};
let sheet = CupertinoActionSheet::new()
    .title(rust_flutter::text::Text::new("Choose an action").into_widget())
    .message(rust_flutter::text::Text::new("Select an option.").into_widget())
    .actions(vec![action("Save"), action("Copy link")])
    .cancel_button(action("Cancel"))
    .width(340.0)
    .theme(theme)
    .into_widget()?;
# let _: Widget = app;
# let _: Widget = routed_app;
# let _: Widget = tabs_app;
# let _: Widget = navigable_tabs;
# let _: Widget = alert;
# let _: Widget = sheet;
```

String route tables are available through `CupertinoApp::with_named_routes` and
`CupertinoNamedRoute`, with checked empty, duplicate, and missing initial-route
errors. `with_named_routes_and_fallback` accepts a custom unknown-route builder.

```rust
use rust_flutter::cupertino::{CupertinoApp, CupertinoNamedRoute};
use rust_flutter::navigation::NavigatorController;
use rust_flutter::text::Text;

let navigator = NavigatorController::new("home".to_owned());
let app = CupertinoApp::with_named_routes(&navigator, [
    CupertinoNamedRoute::new("home", |_| Text::new("Home").into_widget()),
    CupertinoNamedRoute::new("settings", |_| Text::new("Settings").into_widget()),
])?;
```

For route snapshot persistence and its limits, see [retained navigation](navigation.md).

The default theme uses the familiar blue accent for light and dark palettes.
Applications can provide custom primary, text, disabled, and background colors
through `CupertinoThemeData`. The page scaffold expands its body below an
optional navigation and tab bars, centers the title between equal
leading/trailing slots, and applies measured safe-area insets around all three
regions. The standalone tab bar is controlled: the owner updates its selected
index after the callback. `CupertinoTabScaffold` manages selection, keeps each
tab subtree and each pushed route mounted, hides inactive pages, and mutes their
tickers.
`with_navigators` gives each tab an independent `NavigatorController`; its
returned controller pushes on the active tab and can register Back handling.
For parent rebuilds, retain a `CupertinoTabNavigationController` and pass it to
`with_navigation_controller` so selection and route state stay shared. Route
restoration is not built in. Native translucency,
scroll-under blur, large-title transitions, and status-bar tap-to-scroll remain
unfinished. System appearance integration remains explicit application work.
The initial text field uses the shared
retained editor for controller, selection, validation, and input behavior. Its
styling remains basic and does not match native insets, borders, clear-button,
or keyboard accessory APIs. The switch is controlled and uses the iOS green on
state. The slider supports continuous or discrete ranges, drag callbacks, and
theme-based Cupertino track colors while reusing the shared gesture engine.
The activity indicator draws a rotating radial tick pattern and supports a
static partially revealed progress value. `CupertinoSwitch` exposes its switch
role, optional label, disabled state, and on/off state. Switch thumb animation, slider value
indicators and semantics, persistent tile activation while asynchronous tap work
runs, semantics for the remaining controls, and the broader Cupertino component
catalog remain to be implemented. Sliding segmented options expose labeled
radio roles, selected/disabled state, and screen-reader click actions; this is
still awaiting TalkBack and VoiceOver device validation. Tiles provide a pressed background and allow a custom
activated color through `background_color_activated`.

`CupertinoAlertDialog` builds a rounded panel with optional explanatory content
and vertically separated actions. `CupertinoDialogAction` supports default
emphasis and destructive coloring. `CupertinoModalPopup` can present the panel
above a retained page with a scrim and tap-to-dismiss behavior. Pass
`.back_dispatcher(&back)` and use `back.handler()` with
`run_widget_with_options_and_back`; Back dismisses the popup before the route
stack handles it. Transition animation, route results, and restoration remain
unfinished. Set
the panel width and action widths together when adapting it to a narrow window.
`CupertinoActionSheet` groups a title/message, a list of choice actions, and an
optional separated cancel action. Its actions support default emphasis and
destructive coloring. `CupertinoModalPopup` layers either panel above the
retained page with a scrim and tap-to-dismiss behavior. Its builder receives a
`CupertinoModalPopupController`; call `present(id)` from a page action and
return the matching panel while that id is selected. Choose bottom alignment
for an action sheet. Transition animation, route results, and restoration
remain unfinished.

This is not full Cupertino parity. `CupertinoApp` provides theme data and can
render a `NavigatorController` route view; a named route table, localization,
system appearance following, modal transition animation, and pickers remain
unfinished. Compare the official Flutter [CupertinoApp](https://api.flutter.dev/flutter/cupertino/CupertinoApp-class.html)
and [CupertinoButton](https://api.flutter.dev/flutter/cupertino/CupertinoButton-class.html)
APIs for the broader Flutter surface. Flutter's [modal popup API](https://api.flutter.dev/flutter/cupertino/showCupertinoModalPopup.html)
also supports route results, root navigator selection, configurable barrier
dismissal, focus requests, and state-restorable route integration; this retained
host does not implement those features. The control references include
[CupertinoActivityIndicator](https://api.flutter.dev/flutter/cupertino/CupertinoActivityIndicator-class.html),
[CupertinoSlider](https://api.flutter.dev/flutter/cupertino/CupertinoSlider-class.html),
[CupertinoSegmentedControl](https://api.flutter.dev/flutter/cupertino/CupertinoSegmentedControl-class.html),
[CupertinoSlidingSegmentedControl](https://api.flutter.dev/flutter/cupertino/CupertinoSlidingSegmentedControl-class.html),
[CupertinoListTile](https://api.flutter.dev/flutter/cupertino/CupertinoListTile-class.html),
[CupertinoListSection](https://api.flutter.dev/flutter/cupertino/CupertinoListSection-class.html),
[CupertinoTabBar](https://api.flutter.dev/flutter/cupertino/CupertinoTabBar-class.html),
[CupertinoTabScaffold](https://api.flutter.dev/flutter/cupertino/CupertinoTabScaffold-class.html),
[CupertinoSwitch](https://api.flutter.dev/flutter/cupertino/CupertinoSwitch-class.html),
and [CupertinoTextField](https://api.flutter.dev/flutter/cupertino/CupertinoTextField-class.html).
The action-sheet reference is [CupertinoActionSheet](https://api.flutter.dev/flutter/cupertino/CupertinoActionSheet-class.html).
