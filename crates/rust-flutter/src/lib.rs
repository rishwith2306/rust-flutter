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

pub mod prelude {
    pub use crate::{
        AppContext, AppEvent, Application, EventResponse, LifecycleEvent,
        animation::*,
        focus::{Focus, FocusNode, FocusScope, FocusScopeNode},
        navigation::Navigator,
        state::Signal,
        widgets::*,
    };
    pub use egui::{self, Ui};
}
