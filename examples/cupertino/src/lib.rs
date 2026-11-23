use rust_flutter::{
    cupertino::{
        CupertinoActionSheet, CupertinoActionSheetAction, CupertinoActivityIndicator,
        CupertinoAlertDialog, CupertinoApp, CupertinoButton, CupertinoDialogAction,
        CupertinoListSection, CupertinoListTile, CupertinoModalPopup,
        CupertinoModalPopupController, CupertinoNavigationBar, CupertinoPageScaffold,
        CupertinoSegment, CupertinoSegmentedControl, CupertinoSlider, CupertinoSwitch,
        CupertinoTabDestination, CupertinoTabNavigationController, CupertinoTabScaffold,
        CupertinoTextField, CupertinoThemeData,
    },
    element::{FlexItem, FlexStyle, State, StateContext, StatefulWidget, Widget},
    gestures::GestureBinding,
    layout::{Alignment, Axis, Insets},
    scrolling::{ScrollPosition, SingleChildScrollView},
    text::Text,
};

/// A focused runnable catalog for the retained Cupertino controls.
pub fn showcase() -> Widget {
    showcase_with_back_handler().0
}

/// Create the showcase and a native-back callback that dismisses its open popup.
pub fn showcase_with_back_handler() -> (Widget, impl Fn() -> bool + 'static) {
    let binding = GestureBinding::new();
    let theme = CupertinoThemeData::default();
    let page_binding = binding.clone();
    let back_dispatcher = rust_flutter::navigation::BackDispatcher::new();
    let tab_navigation = CupertinoTabNavigationController::new(
        vec![
            rust_flutter::navigation::NavigatorController::new("controls".to_owned()),
            rust_flutter::navigation::NavigatorController::new("dialog".to_owned()),
            rust_flutter::navigation::NavigatorController::new("actions".to_owned()),
            rust_flutter::navigation::NavigatorController::new("about".to_owned()),
        ],
        0,
    )
    .expect("valid example tab navigators");
    let route_back = tab_navigation.register_back_handler(&back_dispatcher);
    let page_navigation = tab_navigation.clone();
    let popup = CupertinoModalPopup::new(&binding, move |controller, selected| {
        let tabs = showcase_tabs(
            page_binding.clone(),
            theme,
            controller.clone(),
            page_navigation.clone(),
        );
        let panel = match selected {
            Some(0) => Some((alert_panel(&page_binding, theme, controller), None)),
            Some(1) => Some((
                action_sheet_panel(&page_binding, theme, controller),
                Some(Alignment { x: 0.0, y: 1.0 }),
            )),
            _ => None,
        };
        (tabs, panel)
    })
    .back_dispatcher(&back_dispatcher)
    .into_widget();
    let root = CupertinoApp::new(popup).theme(theme).into_widget();
    let on_back = move || {
        let _keep_route_handler = &route_back;
        back_dispatcher.dispatch()
    };
    (root, on_back)
}

fn showcase_tabs(
    binding: GestureBinding,
    theme: CupertinoThemeData,
    popup_controller: CupertinoModalPopupController,
    navigation: CupertinoTabNavigationController<String>,
) -> Widget {
    let page_binding = binding.clone();
    CupertinoTabScaffold::with_navigation_controller(
        &binding,
        vec![
            CupertinoTabDestination::new("◉", "Controls"),
            CupertinoTabDestination::new("▣", "Dialog"),
            CupertinoTabDestination::new("☰", "Actions"),
            CupertinoTabDestination::new("ⓘ", "About"),
        ],
        navigation.clone(),
        move |_, index, route| {
            if index == 0 && route == "controls" {
                Widget::stateful(ControlsPage {
                    binding: page_binding.clone(),
                    theme,
                })
            } else if index == 1 && route == "dialog" {
                let controller = popup_controller.clone();
                let show_alert =
                    CupertinoButton::filled(&page_binding, "Show alert dialog", move || {
                        let _ = controller.present(0);
                    })
                    .size(rust_flutter::egui::vec2(240.0, 48.0))
                    .into_widget();
                let detail_navigation = navigation.navigator(index).unwrap();
                let open_details =
                    CupertinoButton::filled(&page_binding, "Open tab detail route", move || {
                        let _ = detail_navigation.push("dialog_details".to_owned());
                    })
                    .size(rust_flutter::egui::vec2(240.0, 48.0))
                    .into_widget();
                let mut style = FlexStyle::new(Axis::Vertical);
                style.spacing = 10.0;
                page(
                    "Dialog",
                    Widget::flex(
                        style,
                        vec![
                            rust_flutter::element::FlexItem::fixed(show_alert),
                            rust_flutter::element::FlexItem::fixed(open_details),
                        ],
                    ),
                    theme,
                )
            } else if index == 1 && route == "dialog_details" {
                let back = navigation.navigator(index).unwrap();
                let button = CupertinoButton::new(&page_binding, "Pop this tab route", move || {
                    let _ = back.pop();
                })
                .size(rust_flutter::egui::vec2(240.0, 48.0))
                .into_widget();
                page("Dialog detail", button, theme)
            } else if index == 2 && route == "actions" {
                let controller = popup_controller.clone();
                let button =
                    CupertinoButton::filled(&page_binding, "Show action sheet", move || {
                        let _ = controller.present(1);
                    })
                    .size(rust_flutter::egui::vec2(240.0, 48.0))
                    .into_widget();
                page("Action sheet", button, theme)
            } else if index == 3 && route == "about" {
                page(
                    "About",
                    Widget::padding(
                        Insets::all(20.0),
                        Text::new("A shared retained widget tree running on macOS and Android.")
                            .into_widget(),
                    ),
                    theme,
                )
            } else {
                Widget::empty()
            }
        },
    )
    .expect("valid example tab navigators")
    .theme(theme)
    .into_widget()
}

fn alert_panel(
    binding: &GestureBinding,
    theme: CupertinoThemeData,
    controller: CupertinoModalPopupController,
) -> Widget {
    let close = controller.clone();
    let cancel = CupertinoDialogAction::new(binding, "Cancel", move || {
        let _ = close.dismiss();
    })
    .width(320.0)
    .theme(theme)
    .into_widget()
    .expect("valid example dialog action");
    let delete = CupertinoDialogAction::new(binding, "Delete", move || {
        let _ = controller.dismiss();
    })
    .width(320.0)
    .destructive_action(true)
    .theme(theme)
    .into_widget()
    .expect("valid example dialog action");
    CupertinoAlertDialog::new(Text::new("Delete draft?").into_widget())
        .content(Text::new("This action cannot be undone.").into_widget())
        .actions(vec![cancel, delete])
        .width(360.0)
        .theme(theme)
        .into_widget()
        .expect("valid example alert dialog")
}

fn action_sheet_panel(
    binding: &GestureBinding,
    theme: CupertinoThemeData,
    controller: CupertinoModalPopupController,
) -> Widget {
    let action = |label: &'static str, destructive: bool| {
        let close = controller.clone();
        CupertinoActionSheetAction::new(binding, label, move || {
            println!("Cupertino action selected: {label}");
            let _ = close.dismiss();
        })
        .width(320.0)
        .destructive_action(destructive)
        .theme(theme)
        .into_widget()
        .expect("valid example action sheet action")
    };
    CupertinoActionSheet::new()
        .title(Text::new("Choose an action").into_widget())
        .message(Text::new("Select how to share this item.").into_widget())
        .actions(vec![
            action("Save to Files", false),
            action("Copy Link", false),
            action("Delete", true),
        ])
        .cancel_button(action("Cancel", false))
        .width(340.0)
        .theme(theme)
        .into_widget()
        .expect("valid example action sheet")
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
            .semantics_label("Loading")
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
    let (root, on_back) = showcase_with_back_handler();
    rust_flutter::run_widget_with_options_and_back(
        "Rust Flutter Cupertino Showcase",
        rust_flutter::NativeOptions {
            android_app: Some(app),
            ..rust_flutter::native_options(rust_flutter::GraphicsBackend::Gles)
        },
        root,
        on_back,
    )
    .expect("Android Cupertino showcase failed");
}
