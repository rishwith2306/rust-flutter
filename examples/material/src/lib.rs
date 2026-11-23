use rust_flutter::{
    element::{
        BuildContext, FlexItem, FlexStyle, State, StateContext, StatefulWidget, StatelessWidget,
        Widget,
    },
    gestures::GestureBinding,
    layout::{Axis, CrossAxisAlignment, Insets},
    material::{
        ActionChip, AlertDialog, AppBar, Brightness, Card, CardVariant, Chip, ChoiceChip,
        ColorScheme, Divider, Drawer, FilterChip, FloatingActionButton, InputChip, ListTile,
        MaterialApp, MaterialDialogController, MaterialDialogHost, ModalBottomSheetController,
        ModalBottomSheetHost, NavigationDrawer, NavigationDrawerDestination, Scaffold,
        ScaffoldMessengerController, ScaffoldMessengerHost, SnackBar, SnackBarAction, TextButton,
        ThemeData, ThemeMode,
    },
    text::{DefaultTextStyle, Text, TextStyle},
};
use std::{cell::Cell, cell::RefCell, rc::Rc};

/// A small Material application demonstrating drawer, FAB and modal dialog APIs.
pub fn showcase() -> Widget {
    showcase_with_back_handler().0
}

pub fn showcase_with_back_handler() -> (Widget, impl Fn() -> bool + 'static) {
    let binding = GestureBinding::new();
    let theme = ThemeData::default();
    let back_dispatcher = rust_flutter::navigation::BackDispatcher::new();
    let sheet_back_controller = Rc::new(RefCell::new(None::<ModalBottomSheetController>));
    let current_sheet_controller = sheet_back_controller.clone();
    let result = Rc::new(Cell::new(None::<bool>));
    let dialog_host = {
        let binding = binding.clone();
        let back_dispatcher = back_dispatcher.clone();
        let result = result.clone();
        let sheet_controller = current_sheet_controller.clone();
        let dialog_theme = theme.clone();
        Rc::new(move || {
            let page_binding = binding.clone();
            let build_result = result.clone();
            let current_sheet_controller = sheet_controller.clone();
            let page_theme = dialog_theme.clone();
            MaterialDialogHost::new(&binding, move |controller, selected| {
                if let Ok(Some(value)) = controller.take_result::<bool>() {
                    build_result.set(Some(value));
                }
                let page = Widget::stateful(DrawerExample {
                    binding: page_binding.clone(),
                    theme: page_theme.clone(),
                    dialog_controller: controller.clone(),
                    sheet_controller: current_sheet_controller
                        .borrow()
                        .clone()
                        .expect("modal sheet host builds before its page"),
                    result: build_result.clone(),
                });
                let dialog = (selected == Some(0)).then(|| {
                    let cancel = controller.clone();
                    let create = controller.clone();
                    AlertDialog::new(Text::new("Create an item?").into_widget())
                        .content(
                            Text::new("This action can be connected to your app data.")
                                .into_widget(),
                        )
                        .actions(vec![
                            TextButton::new(&page_binding, "Cancel", move || {
                                let _ = cancel.dismiss_with_result(false);
                            })
                            .into_widget(),
                            TextButton::new(&page_binding, "Create", move || {
                                let _ = create.dismiss_with_result(true);
                            })
                            .into_widget(),
                        ])
                        .theme(page_theme.clone())
                        .into_widget()
                        .expect("valid example alert dialog")
                });
                (page, dialog)
            })
            .back_dispatcher(&back_dispatcher)
            .into_widget()
        })
    };
    let sheet_binding = binding.clone();
    let sheet_build_controller = sheet_back_controller.clone();
    let dialog_host_builder = dialog_host.clone();
    let host = ModalBottomSheetHost::new(&binding, move |controller, is_presented| {
        *sheet_build_controller.borrow_mut() = Some(controller.clone());
        let page = dialog_host_builder();
        let sheet = is_presented.then(|| {
            let dismiss = controller.clone();
            let mut style = FlexStyle::new(Axis::Vertical);
            style.spacing = 12.0;
            Widget::padding(
                Insets::all(24.0),
                Widget::flex(
                    style,
                    vec![
                        FlexItem::fixed(Text::new("Choose an option").into_widget()),
                        FlexItem::fixed(
                            TextButton::new(&sheet_binding, "Dismiss", move || {
                                let _ = dismiss.dismiss_with_result("dismissed");
                            })
                            .into_widget(),
                        ),
                    ],
                ),
            )
        });
        (page, sheet)
    })
    .back_dispatcher(&back_dispatcher)
    .theme(theme.clone())
    .into_widget();
    let root = MaterialApp::new(host).theme(theme.clone()).into_widget();
    let on_back = back_dispatcher.handler();
    (root, on_back)
}

/// A focused runnable example of timed, queued snackbars.
pub fn queued_messenger_showcase() -> Widget {
    let binding = GestureBinding::new();
    let theme = ThemeData::default();
    let host_theme = theme.clone();
    ScaffoldMessengerHost::new(move |messenger, current| {
        let mut style = FlexStyle::new(Axis::Vertical);
        style.spacing = 12.0;
        let body = Widget::padding(
            Insets::all(24.0),
            Widget::flex(
                style,
                vec![
                    FlexItem::fixed(
                        Text::new("Press both buttons; the messages will be queued.").into_widget(),
                    ),
                    FlexItem::fixed(Widget::stateless(MessengerExample {
                        binding: binding.clone(),
                        messenger,
                        theme: host_theme.clone(),
                    })),
                ],
            ),
        );
        let mut scaffold = Scaffold::new(body)
            .app_bar_config(AppBar::new("Snackbar queue").theme(host_theme.clone()));
        if let Some(snack_bar) = current {
            scaffold = scaffold.snack_bar(snack_bar);
        }
        MaterialApp::new(scaffold.into_widget())
            .theme(host_theme.clone())
            .into_widget()
    })
    .into_widget()
}

/// A focused runnable example that follows the host's current appearance.
pub fn platform_theme_showcase() -> Widget {
    let seed = rust_flutter::egui::Color32::from_rgb(0x22, 0x66, 0xAA);
    let light_theme = ThemeData {
        color_scheme: ColorScheme::from_seed(seed, Brightness::Light),
        ..ThemeData::default()
    };
    let dark_theme = ThemeData {
        color_scheme: ColorScheme::from_seed(seed, Brightness::Dark),
        ..ThemeData::default()
    };
    MaterialApp::new(Widget::stateless(PlatformThemeExample))
        .theme(light_theme)
        .dark_theme(dark_theme)
        .theme_mode(ThemeMode::System)
        .into_widget()
}

struct PlatformThemeExample;

impl StatelessWidget for PlatformThemeExample {
    fn build(&self, context: &BuildContext) -> Widget {
        let theme = ThemeData::of(context).unwrap_or_default();
        let card = Card::new(
            DefaultTextStyle {
                style: TextStyle {
                    color: Some(theme.color_scheme.on_surface),
                    font_size: Some(20.0),
                    ..Default::default()
                },
                ..Default::default()
            }
            .provide(Text::new("This page follows the system appearance.").into_widget()),
        )
        .variant(CardVariant::Filled)
        .theme(theme.clone())
        .into_widget()
        .expect("valid example adaptive Material card");
        Scaffold::new(Widget::padding(Insets::all(24.0), card))
            .app_bar_config(AppBar::new("System theme").theme(theme))
            .into_widget()
    }
}

struct MessengerExample {
    binding: GestureBinding,
    messenger: ScaffoldMessengerController,
    theme: ThemeData,
}

impl StatelessWidget for MessengerExample {
    fn build(&self, _context: &BuildContext) -> Widget {
        let first = self.messenger.clone();
        let first_action = first.clone();
        let first_theme = self.theme.clone();
        let second = self.messenger.clone();
        let second_action = second.clone();
        let second_theme = self.theme.clone();
        let mut style = FlexStyle::new(Axis::Vertical);
        style.spacing = 8.0;
        Widget::flex(
            style,
            vec![
                FlexItem::fixed(
                    TextButton::new(&self.binding, "Queue first message", move || {
                        let action_controller = first_action.clone();
                        let snack_bar = SnackBar::new("First message")
                            .action(SnackBarAction::new("Dismiss", move || {
                                let _ = action_controller.hide_current_snack_bar();
                            }))
                            .theme(first_theme.clone());
                        let _ = first.show_snack_bar(snack_bar);
                    })
                    .into_widget(),
                ),
                FlexItem::fixed(
                    TextButton::new(&self.binding, "Queue second message", move || {
                        let action_controller = second_action.clone();
                        let snack_bar = SnackBar::new("Second message")
                            .action(SnackBarAction::new("Dismiss", move || {
                                let _ = action_controller.hide_current_snack_bar();
                            }))
                            .theme(second_theme.clone());
                        let _ = second.show_snack_bar(snack_bar);
                    })
                    .into_widget(),
                ),
            ],
        )
    }
}

struct DrawerExample {
    binding: GestureBinding,
    theme: ThemeData,
    dialog_controller: MaterialDialogController,
    result: Rc<Cell<Option<bool>>>,
    sheet_controller: ModalBottomSheetController,
}

struct DrawerExampleState {
    selected: usize,
    drawer_open: bool,
    snack_bar_visible: bool,
    vegetarian: bool,
    density_choice: usize,
    member_selected: bool,
    member_present: bool,
}

impl StatefulWidget for DrawerExample {
    type State = DrawerExampleState;

    fn create_state(&self) -> DrawerExampleState {
        DrawerExampleState {
            selected: 0,
            drawer_open: false,
            snack_bar_visible: false,
            vegetarian: false,
            density_choice: 0,
            member_selected: false,
            member_present: true,
        }
    }
}

impl State<DrawerExample> for DrawerExampleState {
    fn build(&mut self, widget: &DrawerExample, context: &StateContext<Self>) -> Widget {
        let handle = context.handle();
        let title = ["Home", "Projects", "Settings"][self.selected];
        let mut card_style = FlexStyle::new(Axis::Vertical);
        card_style.cross_alignment = CrossAxisAlignment::Stretch;
        card_style.spacing = 8.0;
        let card_content = Widget::flex(
            card_style,
            vec![
                FlexItem::fixed(Text::new(format!("Current destination: {title}")).into_widget()),
                FlexItem::fixed(
                    Divider::new()
                        .theme(widget.theme.clone())
                        .into_widget()
                        .expect("valid example Material divider"),
                ),
                FlexItem::fixed(
                    Text::new(match widget.result.get() {
                        Some(true) => "Dialog result: create",
                        Some(false) => "Dialog result: cancel",
                        None => "Use + for a dialog or the button below for a sheet.",
                    })
                    .into_widget(),
                ),
            ],
        );
        let card_content = DefaultTextStyle {
            style: TextStyle {
                font_size: Some(22.0),
                color: Some(widget.theme.color_scheme.on_surface),
                ..Default::default()
            },
            ..Default::default()
        }
        .provide(card_content);
        let open_sheet = widget.sheet_controller.clone();
        let show_snack_bar = handle.clone();
        let filter_handle = handle.clone();
        let filter_chip = FilterChip::new(
            &widget.binding,
            "Vegetarian",
            self.vegetarian,
            move |selected| {
                let _ = filter_handle.update(move |state| state.vegetarian = selected);
            },
        )
        .into_widget();
        let compact_handle = handle.clone();
        let compact_chip = ChoiceChip::new(
            &widget.binding,
            "Compact",
            self.density_choice == 0,
            move || {
                let _ = compact_handle.update(|state| state.density_choice = 0);
            },
        )
        .into_widget();
        let comfortable_handle = handle.clone();
        let comfortable_chip = ChoiceChip::new(
            &widget.binding,
            "Comfortable",
            self.density_choice == 1,
            move || {
                let _ = comfortable_handle.update(|state| state.density_choice = 1);
            },
        )
        .into_widget();
        let member_chip = if self.member_present {
            let select_member = handle.clone();
            let delete_member = handle.clone();
            InputChip::new(&widget.binding, "Ada", self.member_selected)
                .on_selected(move |selected| {
                    let _ = select_member.update(move |state| state.member_selected = selected);
                })
                .on_deleted(move || {
                    let _ = delete_member.update(|state| state.member_present = false);
                })
                .into_widget()
        } else {
            let add_member = handle.clone();
            ActionChip::new(&widget.binding, "Add Ada", move || {
                let _ = add_member.update(|state| state.member_present = true);
            })
            .into_widget()
        };
        let synced_chip = Chip::new("Synced")
            .tooltip("All changes are saved")
            .into_widget();
        let mut body_style = FlexStyle::new(Axis::Vertical);
        body_style.spacing = 12.0;
        let mut chip_row_style = FlexStyle::new(Axis::Horizontal);
        chip_row_style.spacing = 8.0;
        chip_row_style.cross_alignment = CrossAxisAlignment::Center;
        let card = Card::new(Widget::padding(Insets::all(20.0), card_content))
            .variant(CardVariant::Filled)
            .theme(widget.theme.clone())
            .into_widget()
            .expect("valid example Material card");
        let body = Widget::flex(
            body_style,
            vec![
                FlexItem::fixed(card),
                FlexItem::fixed(Text::new("Filter meals").into_widget()),
                FlexItem::fixed(Widget::flex(
                    chip_row_style,
                    vec![FlexItem::fixed(filter_chip)],
                )),
                FlexItem::fixed(Text::new("List density").into_widget()),
                FlexItem::fixed(Widget::flex(
                    chip_row_style,
                    vec![
                        FlexItem::fixed(compact_chip),
                        FlexItem::fixed(comfortable_chip),
                    ],
                )),
                FlexItem::fixed(Text::new("Participants").into_widget()),
                FlexItem::fixed(Widget::flex(
                    chip_row_style,
                    vec![FlexItem::fixed(member_chip)],
                )),
                FlexItem::fixed(Text::new("Status").into_widget()),
                FlexItem::fixed(synced_chip),
                FlexItem::fixed(
                    ActionChip::new(&widget.binding, "Open options", move || {
                        let _ = open_sheet.present();
                    })
                    .tooltip("Choose an option")
                    .into_widget(),
                ),
                FlexItem::fixed(
                    TextButton::new(&widget.binding, "Show snackbar", move || {
                        let _ = show_snack_bar.update(|state| state.snack_bar_visible = true);
                    })
                    .into_widget(),
                ),
            ],
        );

        let select = handle.clone();
        let drawer_contents = NavigationDrawer::new(
            &widget.binding,
            vec![
                NavigationDrawerDestination::new("⌂", "Home").selected_icon("⌂"),
                NavigationDrawerDestination::new("▣", "Projects"),
                NavigationDrawerDestination::new("⚙", "Settings"),
            ],
            Some(self.selected),
            move |selected| {
                let _ = select.update(move |state| {
                    state.selected = selected;
                    state.drawer_open = false;
                });
            },
        )
        .header(Text::new("WORKSPACE").into_widget())
        .footer(
            ListTile::new(
                &widget.binding,
                Text::new("About this example").into_widget(),
            )
            .leading(Text::new("ⓘ").into_widget())
            .trailing(Text::new("0.1").into_widget())
            .theme(widget.theme.clone())
            .into_widget()
            .expect("valid example list tile"),
        )
        .width(256.0)
        .theme(widget.theme.clone())
        .into_widget()
        .expect("valid example navigation drawer");
        let dismiss = handle.clone();
        let drawer = Drawer::new(drawer_contents)
            .width(288.0)
            .theme(widget.theme.clone())
            .into_widget()
            .expect("valid example drawer");

        let show_dialog = widget.dialog_controller.clone();
        let mut scaffold = Scaffold::new(Widget::padding(Insets::all(24.0), body))
            .app_bar_config(AppBar::new(title).theme(widget.theme.clone()))
            .floating_action_button(
                FloatingActionButton::new(
                    &widget.binding,
                    Text::new("+").into_widget(),
                    move || {
                        let _ = show_dialog.present(0);
                    },
                )
                .theme(widget.theme.clone()),
            );
        if self.snack_bar_visible {
            let dismiss_snack_bar = handle.clone();
            scaffold = scaffold.snack_bar(
                SnackBar::new("Saved successfully")
                    .action(SnackBarAction::new("Dismiss", move || {
                        let _ = dismiss_snack_bar.update(|state| state.snack_bar_visible = false);
                    }))
                    .theme(widget.theme.clone()),
            );
        }
        scaffold
            .drawer(
                &widget.binding,
                drawer,
                self.drawer_open,
                move || {
                    let _ = handle.update(|state| state.drawer_open = true);
                },
                move || {
                    let _ = dismiss.update(|state| state.drawer_open = false);
                },
            )
            .theme(widget.theme.clone())
            .into_widget()
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: rust_flutter::AndroidApp) {
    #[cfg(feature = "system-theme-showcase")]
    let (root, on_back) = (platform_theme_showcase(), || false);
    #[cfg(not(feature = "system-theme-showcase"))]
    let (root, on_back) = showcase_with_back_handler();
    rust_flutter::run_widget_with_options_and_back(
        if cfg!(feature = "system-theme-showcase") {
            "Rust Flutter System Theme"
        } else {
            "Rust Flutter Material Showcase"
        },
        rust_flutter::NativeOptions {
            android_app: Some(app),
            ..rust_flutter::native_options(rust_flutter::GraphicsBackend::Gles)
        },
        root,
        on_back,
    )
    .expect("Android Material showcase failed");
}
