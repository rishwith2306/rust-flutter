use rust_flutter::{
    cupertino::{
        CupertinoActivityIndicator, CupertinoApp, CupertinoButton, CupertinoListSection,
        CupertinoListTile, CupertinoNavigationBar, CupertinoPageScaffold, CupertinoSegment,
        CupertinoSegmentedControl, CupertinoSlider, CupertinoSwitch, CupertinoTabDestination,
        CupertinoTabScaffold, CupertinoTextField, CupertinoThemeData,
    },
    element::{FlexItem, FlexStyle, State, StateContext, StatefulWidget, Widget},
    gestures::GestureBinding,
    layout::{Axis, Insets},
    scrolling::{ScrollPosition, SingleChildScrollView},
    text::Text,
};

/// A focused runnable catalog for the retained Cupertino controls.
pub fn showcase() -> Widget {
    let binding = GestureBinding::new();
    let theme = CupertinoThemeData::default();
    let page_binding = binding.clone();
    let tabs = CupertinoTabScaffold::new(
        &binding,
        vec![
            CupertinoTabDestination::new("◉", "Controls"),
            CupertinoTabDestination::new("ⓘ", "About"),
        ],
        0,
        move |_, index| {
            if index == 0 {
                Widget::stateful(ControlsPage {
                    binding: page_binding.clone(),
                    theme,
                })
            } else {
                page(
                    "About",
                    Widget::padding(
                        Insets::all(20.0),
                        Text::new("A shared retained widget tree running on macOS and Android.")
                            .into_widget(),
                    ),
                    theme,
                )
            }
        },
    )
    .expect("two valid example tabs")
    .theme(theme)
    .into_widget();
    CupertinoApp::new(tabs).theme(theme).into_widget()
}

fn page(title: &str, content: Widget, theme: CupertinoThemeData) -> Widget {
    CupertinoPageScaffold::new(content)
        .navigation_bar(
            CupertinoNavigationBar::new(title)
                .theme(theme)
                .into_widget(),
        )
        .theme(theme)
        .into_widget()
}

struct ControlsPage {
    binding: GestureBinding,
    theme: CupertinoThemeData,
}

struct ControlsState {
    enabled: bool,
    slider: f32,
    segment: &'static str,
    has_query: bool,
    saved: bool,
    position: Option<ScrollPosition>,
}

impl StatefulWidget for ControlsPage {
    type State = ControlsState;

    fn create_state(&self) -> ControlsState {
        ControlsState {
            enabled: true,
            slider: 0.65,
            segment: "week",
            has_query: false,
            saved: false,
            position: None,
        }
    }
}

impl State<ControlsPage> for ControlsState {
    fn init_state(&mut self, _: &ControlsPage, context: &StateContext<Self>) {
        self.position =
            Some(ScrollPosition::new(context.egui(), 0.0).expect("valid initial scroll offset"));
    }

    fn build(&mut self, widget: &ControlsPage, context: &StateContext<Self>) -> Widget {
        let handle = context.handle();
        let field = CupertinoTextField::new("")
            .placeholder("Search settings")
            .on_changed({
                let handle = handle.clone();
                move |value| {
                    let has_query = !value.is_empty();
                    let _ = handle.update(move |state| state.has_query = has_query);
                }
            })
            .into_widget()
            .expect("valid example text field");
        let toggle = CupertinoSwitch::new(&widget.binding, self.enabled, {
            let handle = handle.clone();
            move |enabled| {
                let _ = handle.update(move |state| state.enabled = enabled);
            }
        })
        .into_widget();
        let slider = CupertinoSlider::new(&widget.binding, self.slider, {
            let handle = handle.clone();
            move |value| {
                let _ = handle.update(move |state| state.slider = value);
            }
        })
        .range(0.0, 1.0)
        .divisions(Some(10))
        .into_widget()
        .expect("valid example slider");
        let segments = CupertinoSegmentedControl::new(
            &widget.binding,
            vec![
                CupertinoSegment::new("day", "Day"),
                CupertinoSegment::new("week", "Week"),
                CupertinoSegment::new("month", "Month"),
            ],
            self.segment,
            {
                let handle = handle.clone();
                move |value| {
                    let segment = match value {
                        "day" => "day",
                        "month" => "month",
                        _ => "week",
                    };
                    let _ = handle.update(move |state| state.segment = segment);
                }
            },
        )
        .expect("valid example segments")
        .into_widget()
        .expect("valid example segmented control");
        let privacy = CupertinoListTile::new(&widget.binding, Text::new("Privacy").into_widget())
            .subtitle(Text::new("Permissions and tracking").into_widget())
            .trailing(Text::new("›").into_widget())
            .on_tap(|| {})
            .notched()
            .into_widget()
            .expect("valid example list tile");
        let account = CupertinoListTile::new(&widget.binding, Text::new("Account").into_widget())
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
        let button = CupertinoButton::filled(
            &widget.binding,
            if self.saved { "Saved" } else { "Save changes" },
            {
                let handle = handle.clone();
                move || {
                    let _ = handle.update(|state| state.saved = true);
                }
            },
        )
        .into_widget();
        let loading = CupertinoActivityIndicator::new()
            .into_widget()
            .expect("valid activity indicator");

        let mut style = FlexStyle::new(Axis::Vertical);
        style.spacing = 14.0;
        let content = Widget::padding(
            Insets::all(16.0),
            Widget::flex(
                style,
                vec![
                    FlexItem::fixed(Text::new("Cupertino controls").into_widget()),
                    FlexItem::fixed(field),
                    FlexItem::fixed(
                        Text::new(if self.has_query {
                            "Search query entered"
                        } else {
                            "Enter a search query"
                        })
                        .into_widget(),
                    ),
                    FlexItem::fixed(section),
                    FlexItem::fixed(toggle),
                    FlexItem::fixed(Text::new(format!("Enabled: {}", self.enabled)).into_widget()),
                    FlexItem::fixed(slider),
                    FlexItem::fixed(Text::new(format!("Slider: {:.1}", self.slider)).into_widget()),
                    FlexItem::fixed(segments),
                    FlexItem::fixed(Text::new(format!("Range: {}", self.segment)).into_widget()),
                    FlexItem::fixed(button),
                    FlexItem::fixed(loading),
                ],
            ),
        );
        let view = SingleChildScrollView::new(
            self.position.as_ref().expect("initialized scroll position"),
            &widget.binding,
        )
        .expect("valid scroll viewport")
        .into_widget(content)
        .expect("valid scroll content");
        page("Controls", view, widget.theme)
    }
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
