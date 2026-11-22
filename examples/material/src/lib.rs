use rust_flutter::{
    element::{State, StateContext, StatefulWidget, Widget},
    gestures::GestureBinding,
    layout::Insets,
    material::{
        AlertDialog, AppBar, Drawer, FloatingActionButton, ListTile, MaterialApp,
        MaterialDialogController, MaterialDialogHost, NavigationDrawer,
        NavigationDrawerDestination, Scaffold, TextButton, ThemeData,
    },
    text::{DefaultTextStyle, Text, TextStyle},
};

/// A small Material application demonstrating drawer, FAB and modal dialog APIs.
pub fn showcase() -> Widget {
    let binding = GestureBinding::new();
    let theme = ThemeData::default();
    let page_binding = binding.clone();
    let host = MaterialDialogHost::new(&binding, move |controller, selected| {
        let page = Widget::stateful(DrawerExample {
            binding: page_binding.clone(),
            theme,
            dialog_controller: controller.clone(),
        });
        let dialog = (selected == Some(0)).then(|| {
            let cancel = controller.clone();
            let create = controller.clone();
            AlertDialog::new(Text::new("Create an item?").into_widget())
                .content(Text::new("This action can be connected to your app data.").into_widget())
                .actions(vec![
                    TextButton::new(&page_binding, "Cancel", move || {
                        let _ = cancel.dismiss();
                    })
                    .into_widget(),
                    TextButton::new(&page_binding, "Create", move || {
                        let _ = create.dismiss();
                    })
                    .into_widget(),
                ])
                .theme(theme)
                .into_widget()
                .expect("valid example alert dialog")
        });
        (page, dialog)
    })
    .into_widget();
    MaterialApp::new(host).theme(theme).into_widget()
}

struct DrawerExample {
    binding: GestureBinding,
    theme: ThemeData,
    dialog_controller: MaterialDialogController,
}

struct DrawerExampleState {
    selected: usize,
    drawer_open: bool,
}

impl StatefulWidget for DrawerExample {
    type State = DrawerExampleState;

    fn create_state(&self) -> DrawerExampleState {
        DrawerExampleState {
            selected: 0,
            drawer_open: false,
        }
    }
}

impl State<DrawerExample> for DrawerExampleState {
    fn build(&mut self, widget: &DrawerExample, context: &StateContext<Self>) -> Widget {
        let handle = context.handle();
        let title = ["Home", "Projects", "Settings"][self.selected];
        let body = DefaultTextStyle {
            style: TextStyle {
                font_size: Some(22.0),
                color: Some(widget.theme.color_scheme.on_surface),
                ..Default::default()
            },
            ..Default::default()
        }
        .provide(
            Text::new(format!(
                "Current destination: {title}\nTap + to open a dialog."
            ))
            .into_widget(),
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
            .theme(widget.theme)
            .into_widget()
            .expect("valid example list tile"),
        )
        .width(256.0)
        .theme(widget.theme)
        .into_widget()
        .expect("valid example navigation drawer");
        let dismiss = handle.clone();
        let drawer = Drawer::new(drawer_contents)
            .width(288.0)
            .theme(widget.theme)
            .into_widget()
            .expect("valid example drawer");

        let show_dialog = widget.dialog_controller.clone();
        Scaffold::new(Widget::padding(Insets::all(24.0), body))
            .app_bar_config(AppBar::new(title).theme(widget.theme))
            .floating_action_button(
                FloatingActionButton::new(
                    &widget.binding,
                    Text::new("+").into_widget(),
                    move || {
                        let _ = show_dialog.present(0);
                    },
                )
                .theme(widget.theme),
            )
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
            .theme(widget.theme)
            .into_widget()
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: rust_flutter::AndroidApp) {
    rust_flutter::run_widget_with_options(
        "Rust Flutter Material Showcase",
        rust_flutter::NativeOptions {
            android_app: Some(app),
            ..rust_flutter::native_options(rust_flutter::GraphicsBackend::Gles)
        },
        showcase(),
    )
    .expect("Android Material showcase failed");
}
