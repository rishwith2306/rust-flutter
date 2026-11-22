//! A Rust application framework with composable UI, typed navigation and shared state.
//!
//! Desktop and Android use the same [`Application`] implementation. See the gallery
//! for entrypoints. This crate is experimental; Flutter feature parity is tracked
//! in `docs/flutter-parity.md` in the repository.
pub use eframe::{Error, NativeOptions};
pub use egui;

/// Select one graphics API when a platform surface cannot be shared by multiple
/// backend probes. `Auto` is useful on desktop; Android defaults to `Gles`.
#[derive(Debug, Clone, Copy)]
pub enum GraphicsBackend {
    Auto,
    Vulkan,
    Gles,
    Metal,
}

pub fn native_options(backend: GraphicsBackend) -> NativeOptions {
    let mut options = NativeOptions::default();
    if !matches!(backend, GraphicsBackend::Auto) {
        let mut setup = eframe::egui_wgpu::WgpuSetupCreateNew::default();
        setup.instance_descriptor.backends = match backend {
            GraphicsBackend::Vulkan => wgpu::Backends::VULKAN,
            GraphicsBackend::Gles => wgpu::Backends::GL,
            GraphicsBackend::Metal => wgpu::Backends::METAL,
            GraphicsBackend::Auto => unreachable!(),
        };
        options.wgpu_options.wgpu_setup = eframe::egui_wgpu::WgpuSetup::CreateNew(setup);
    }
    options
}

#[cfg(target_os = "android")]
mod android_host;
#[cfg(target_os = "android")]
mod android_insets;
pub mod animation;
pub mod assets;
pub mod cupertino;
pub mod decoration;
pub mod display;
pub mod element;
pub mod focus;
pub mod forms;
#[cfg(any(target_os = "android", test))]
mod frame_schedule;
pub mod gestures;
pub mod layout;
pub mod localization;
pub mod material;
pub mod navigation;
pub mod notifier;
pub mod physics;
pub mod runtime;
pub mod scrolling;
pub mod storage;
pub mod text;
pub use runtime::{AppContext, AppEvent, EventResponse, LifecycleEvent, Runtime};
pub mod services;
pub mod state;
pub mod widgets;

/// An application's persistent model. `build` runs on demand for each UI frame.
/// Keep side effects outside `build`, or guard them behind interaction responses.
pub trait Application: 'static {
    fn build(&mut self, ui: &mut egui::Ui);
    /// Called once before the first frame to load state or install resources.
    fn initialize(&mut self, _context: &AppContext) {}
    fn on_event(&mut self, _ctx: &egui::Context, _event: AppEvent) -> EventResponse {
        EventResponse::Continue
    }
    /// Called once on graceful shutdown. Mobile process termination may skip this.
    fn shutdown(&mut self) {}
    /// Called before building the frame; useful for configuring fonts and themes.
    fn configure(&mut self, _ctx: &egui::Context) {}
}

/// Run on macOS, Linux or Windows. The OS event loop runs on the calling thread.
#[cfg(not(target_os = "android"))]
pub fn run<A: Application>(title: &str, app: A) -> Result<(), Error> {
    run_with_options(title, NativeOptions::default(), app)
}

/// Run with custom window, rendering and platform options.
pub fn run_with_options<A: Application>(
    title: &str,
    options: NativeOptions,
    app: A,
) -> Result<(), Error> {
    #[cfg(target_os = "android")]
    {
        let _ = title;
        android_host::run(options, app)
    }
    #[cfg(not(target_os = "android"))]
    {
        let data_dir = storage::app_data_dir(options.viewport.app_id.as_deref().unwrap_or(title));
        eframe::run_native(
            title,
            options,
            Box::new(move |_| {
                let runtime = Runtime::new(app, data_dir);
                Ok(Box::new(runtime))
            }),
        )
    }
}

/// Run a retained framework widget tree with the native host.
///
/// The tree is created once and kept mounted across frames. Use
/// [`material::MaterialApp`] to provide app-level Material theme data.
pub fn run_widget_with_options(
    title: &str,
    options: NativeOptions,
    root: element::Widget,
) -> Result<(), Error> {
    run_with_options(title, options, WidgetApplication::new(root, None))
}

/// Run a retained tree and let the application decide whether native back
/// requests are consumed. Return `true` to keep the app open.
pub fn run_widget_with_options_and_back(
    title: &str,
    options: NativeOptions,
    root: element::Widget,
    on_back: impl Fn() -> bool + 'static,
) -> Result<(), Error> {
    run_with_options(
        title,
        options,
        WidgetApplication::new(root, Some(std::rc::Rc::new(on_back))),
    )
}

/// Run a retained framework widget tree on macOS, Linux, or Windows.
#[cfg(not(target_os = "android"))]
pub fn run_widget(title: &str, root: element::Widget) -> Result<(), Error> {
    run_widget_with_options(title, NativeOptions::default(), root)
}

/// Run a retained widget tree with a native back-request handler.
#[cfg(not(target_os = "android"))]
pub fn run_widget_with_back_handler(
    title: &str,
    root: element::Widget,
    on_back: impl Fn() -> bool + 'static,
) -> Result<(), Error> {
    run_widget_with_options_and_back(title, NativeOptions::default(), root, on_back)
}

#[cfg(target_os = "android")]
pub use android_activity::AndroidApp;

/// Attach to the NativeActivity supplied to `android_main`.
#[cfg(target_os = "android")]
pub fn run_android<A: Application>(activity: AndroidApp, title: &str, app: A) -> Result<(), Error> {
    run_with_options(
        title,
        NativeOptions {
            android_app: Some(activity),
            ..native_options(GraphicsBackend::Gles)
        },
        app,
    )
}

/// Run a retained widget tree on the Android NativeActivity host.
#[cfg(target_os = "android")]
pub fn run_widget_android(
    activity: AndroidApp,
    title: &str,
    root: element::Widget,
) -> Result<(), Error> {
    run_android(activity, title, WidgetApplication::new(root, None))
}

/// Run a retained Android widget tree with a back-request handler.
#[cfg(target_os = "android")]
pub fn run_widget_android_with_back_handler(
    activity: AndroidApp,
    title: &str,
    root: element::Widget,
    on_back: impl Fn() -> bool + 'static,
) -> Result<(), Error> {
    run_android(
        activity,
        title,
        WidgetApplication::new(root, Some(std::rc::Rc::new(on_back))),
    )
}

struct WidgetApplication {
    root: Option<element::Widget>,
    tree: Option<element::ElementTree>,
    gestures: gestures::GestureBinding,
    startup_error: Option<String>,
    on_back: Option<std::rc::Rc<dyn Fn() -> bool>>,
}

impl WidgetApplication {
    fn new(root: element::Widget, on_back: Option<std::rc::Rc<dyn Fn() -> bool>>) -> Self {
        Self {
            root: Some(root),
            tree: None,
            gestures: gestures::GestureBinding::new(),
            startup_error: None,
            on_back,
        }
    }
}

impl Application for WidgetApplication {
    fn initialize(&mut self, context: &AppContext) {
        if let Some(root) = self.root.take() {
            match element::ElementTree::new(&context.egui, root) {
                Ok(tree) => self.tree = Some(tree),
                Err(error) => self.startup_error = Some(format!("{error:?}")),
            }
        }
    }

    fn build(&mut self, ui: &mut egui::Ui) {
        if let Some(error) = &self.startup_error {
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                format!("Widget tree error: {error}"),
            );
            return;
        }
        let constraints = match layout::BoxConstraints::loose(ui.available_size()) {
            Ok(constraints) => constraints,
            Err(error) => {
                ui.colored_label(
                    egui::Color32::LIGHT_RED,
                    format!("Widget layout constraints error: {error}"),
                );
                return;
            }
        };
        if let Some(tree) = &mut self.tree
            && let Err(error) = tree.show_with_gestures(ui, constraints, &self.gestures)
        {
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                format!("Widget tree frame error: {error}"),
            );
        }
    }

    fn on_event(&mut self, _: &egui::Context, event: AppEvent) -> EventResponse {
        if event == AppEvent::BackRequested
            && self.on_back.as_ref().is_some_and(|handler| handler())
        {
            EventResponse::Consumed
        } else {
            EventResponse::Continue
        }
    }

    fn shutdown(&mut self) {
        self.tree = None;
    }
}

pub mod prelude {
    pub use crate::{
        AppContext, AppEvent, Application, EventResponse, LifecycleEvent,
        animation::*,
        focus::{Focus, FocusNode, FocusScope, FocusScopeNode},
        material::{
            AppBar, Brightness, Checkbox, ColorScheme, FilledButton, MaterialApp, NavigationBar,
            NavigationDestination, OutlinedButton, Radio, Scaffold, Slider, Switch, TextButton,
            TextField, ThemeData,
        },
        navigation::{Navigator, NavigatorController},
        state::Signal,
        widgets::*,
    };
    pub use egui::{self, Ui};
}
