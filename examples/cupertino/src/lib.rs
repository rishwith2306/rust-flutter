use rust_flutter::{
    cupertino::{
        CupertinoActivityIndicator, CupertinoApp, CupertinoButton, CupertinoListSection,
        CupertinoListTile, CupertinoNavigationBar, CupertinoPageScaffold, CupertinoSegment,
        CupertinoSegmentedControl, CupertinoSlider, CupertinoSwitch, CupertinoTextField,
        CupertinoThemeData,
    },
    element::{FlexItem, FlexStyle, Widget},
    gestures::GestureBinding,
    layout::Axis,
    text::Text,
};

/// A small runnable catalog showing the retained Cupertino controls on the
/// framework's shared macOS and Android native hosts.
pub fn showcase() -> Widget {
    let binding = GestureBinding::new();
    let theme = CupertinoThemeData::default();

    let field = CupertinoTextField::new("")
        .placeholder("Search settings")
        .into_widget()
        .expect("valid example text field");
    let toggle = CupertinoSwitch::new(&binding, true, |_| {}).into_widget();
    let slider = CupertinoSlider::new(&binding, 0.65, |_| {})
        .range(0.0, 1.0)
        .divisions(Some(10))
        .into_widget()
        .expect("valid example slider");
    let segments = CupertinoSegmentedControl::new(
        &binding,
        vec![
            CupertinoSegment::new("day", "Day"),
            CupertinoSegment::new("week", "Week"),
            CupertinoSegment::new("month", "Month"),
        ],
        "week",
        |_| {},
    )
    .expect("valid example segments")
    .into_widget()
    .expect("valid example segmented control");
    let privacy = CupertinoListTile::new(&binding, Text::new("Privacy").into_widget())
        .subtitle(Text::new("Permissions and tracking").into_widget())
        .trailing(Text::new("›").into_widget())
        .on_tap(|| {})
        .notched()
        .into_widget()
        .expect("valid example list tile");
    let account = CupertinoListTile::new(&binding, Text::new("Account").into_widget())
        .subtitle(Text::new("Profile and sign-in").into_widget())
        .trailing(Text::new("›").into_widget())
        .on_tap(|| {})
        .notched()
        .into_widget()
        .expect("valid example list tile");
    let section = CupertinoListSection::new(vec![privacy, account])
        .header(Text::new("PREFERENCES").into_widget())
        .inset_grouped()
        .into_widget()
        .expect("valid example list section");
    let button = CupertinoButton::filled(&binding, "Save changes", || {}).into_widget();
    let loading = CupertinoActivityIndicator::new()
        .into_widget()
        .expect("valid activity indicator");

    let mut style = FlexStyle::new(Axis::Vertical);
    style.spacing = 14.0;
    let content = Widget::padding(
        rust_flutter::layout::Insets::all(16.0),
        Widget::flex(
            style,
            vec![
                FlexItem::fixed(Text::new("Cupertino controls").into_widget()),
                FlexItem::fixed(field),
                FlexItem::fixed(section),
                FlexItem::fixed(toggle),
                FlexItem::fixed(slider),
                FlexItem::fixed(segments),
                FlexItem::fixed(button),
                FlexItem::fixed(loading),
            ],
        ),
    );
    let page = CupertinoPageScaffold::new(content)
        .navigation_bar(
            CupertinoNavigationBar::new("Rust Flutter")
                .theme(theme)
                .into_widget(),
        )
        .theme(theme)
        .into_widget();
    CupertinoApp::new(page).theme(theme).into_widget()
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: rust_flutter::AndroidApp) {
    rust_flutter::run_widget_with_options(
        "Rust Flutter Cupertino Showcase",
        rust_flutter::NativeOptions {
            android_app: Some(app),
            ..rust_flutter::native_options(rust_flutter::GraphicsBackend::Gles)
        },
        showcase(),
    )
    .expect("Android Cupertino showcase failed");
}
