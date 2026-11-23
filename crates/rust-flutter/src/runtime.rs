//! Application hooks shared by native runners and headless UI tests.
use crate::Application;
use crate::services::Services;
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct AppContext {
    pub egui: egui::Context,
    /// Platform-private app storage, when available. Do not assume a desktop home
    /// directory exists on Android; the runner uses the activity's internal data path.
    pub data_dir: Option<PathBuf>,
    /// Immutable platform implementations installed before the app starts.
    pub services: Arc<Services>,
}
impl AppContext {
    /// Return a custom clipboard service or the host clipboard implementation.
    /// Host writes are queued through egui's platform output.
    pub fn clipboard(&self) -> crate::services::ClipboardService {
        self.services
            .get::<crate::services::ClipboardService>()
            .cloned()
            .unwrap_or_else(|| crate::services::host_clipboard(&self.egui))
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppEvent {
    FocusChanged(bool),
    BackRequested,
    CloseRequested,
    Lifecycle(LifecycleEvent),
}
/// Native host notifications. Surface lifetime is distinct from activity lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEvent {
    Started,
    Resumed,
    Paused,
    Stopped,
    Destroyed,
    SurfaceCreated,
    SurfaceDestroyed,
    LowMemory,
    SaveStateRequested,
    ConfigurationChanged,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum EventResponse {
    #[default]
    Continue,
    /// Consume back navigation or cancel a close request. Ignored for focus changes.
    Consumed,
}

/// Drives the same application code in eframe and renderer-independent tests.
/// Focus and activity lifecycle are distinct; use `AppEvent::Lifecycle` for Android pause/resume.
pub struct Runtime<A: Application> {
    app: A,
    data_dir: Option<PathBuf>,
    services: Arc<Services>,
    initialized: bool,
    stopped: bool,
    focused: Option<bool>,
}
impl<A: Application> Runtime<A> {
    pub fn new(app: A, data_dir: Option<PathBuf>) -> Self {
        Self::with_services(app, data_dir, Services::default())
    }

    /// Create a runtime with per-platform implementations registered before startup.
    pub fn with_services(app: A, data_dir: Option<PathBuf>, services: Services) -> Self {
        Self {
            app,
            data_dir,
            services: Arc::new(services),
            initialized: false,
            stopped: false,
            focused: None,
        }
    }
    /// Dispatch an OS event even when the rendering surface is absent.
    /// Shutdown prevents subsequent events from reinitializing an application.
    pub fn platform_event(&mut self, ctx: &egui::Context, event: AppEvent) -> EventResponse {
        if self.stopped {
            return EventResponse::Continue;
        }
        self.initialize(ctx);
        let response = self.app.on_event(ctx, event);
        if event == AppEvent::Lifecycle(LifecycleEvent::Destroyed) {
            self.shutdown();
        }
        response
    }
    pub(crate) fn focus_changed(&mut self, ctx: &egui::Context, focused: bool) {
        if self.focused != Some(focused) {
            self.focused = Some(focused);
            self.platform_event(ctx, AppEvent::FocusChanged(focused));
        }
    }
    fn initialize(&mut self, ctx: &egui::Context) {
        if !self.initialized {
            self.app.initialize(&AppContext {
                egui: ctx.clone(),
                data_dir: self.data_dir.clone(),
                services: self.services.clone(),
            });
            self.initialized = true;
        }
    }
    pub fn app(&self) -> &A {
        &self.app
    }
    pub fn app_mut(&mut self) -> &mut A {
        &mut self.app
    }
    /// Deliver Android saved-instance bytes before the first restored frame.
    pub fn restore_state(&mut self, ctx: &egui::Context, state: &[u8]) {
        if self.stopped {
            return;
        }
        self.initialize(ctx);
        self.app.restore_state(
            &AppContext {
                egui: ctx.clone(),
                data_dir: self.data_dir.clone(),
                services: self.services.clone(),
            },
            state,
        );
    }

    /// Ask the application for a best-effort saved-instance snapshot.
    pub fn save_state(&mut self, ctx: &egui::Context) -> Option<Vec<u8>> {
        if self.stopped {
            return None;
        }
        self.initialize(ctx);
        self.app.save_state(&AppContext {
            egui: ctx.clone(),
            data_dir: self.data_dir.clone(),
            services: self.services.clone(),
        })
    }
    /// Run inside `egui::Context::run`. Once shut down, no further frames are built.
    pub fn frame(&mut self, ctx: &egui::Context) {
        if self.stopped {
            return;
        }
        self.initialize(ctx);
        let focused = ctx.input(|i| i.raw.focused);
        self.focus_changed(ctx, focused);
        let back_requested = ctx.input_mut(|input| {
            let browser_back = input.consume_key(egui::Modifiers::NONE, egui::Key::BrowserBack);
            #[cfg(target_os = "macos")]
            let escape = input.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
            #[cfg(not(target_os = "macos"))]
            let escape = false;
            browser_back || escape
        });
        if back_requested {
            let response = self.app.on_event(ctx, AppEvent::BackRequested);
            #[cfg(target_os = "android")]
            if response == EventResponse::Continue {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            #[cfg(not(target_os = "android"))]
            let _ = response;
        }
        if ctx.input(|i| i.viewport().close_requested())
            && self.app.on_event(ctx, AppEvent::CloseRequested) == EventResponse::Consumed
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        self.app.configure(ctx);
        egui::CentralPanel::default().show(ctx, |ui| self.app.build(ui));
    }
    /// Graceful shutdown notification, called at most once. Process kills may skip it.
    /// Persist essential state when it changes rather than relying on this hook alone.
    pub fn shutdown(&mut self) {
        if !self.stopped {
            self.stopped = true;
            if self.initialized {
                self.app.shutdown();
            }
        }
    }
}
impl<A: Application> Drop for Runtime<A> {
    fn drop(&mut self) {
        self.shutdown();
    }
}
impl<A: Application> eframe::App for Runtime<A> {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame(ctx);
    }
    fn on_exit(&mut self) {
        self.shutdown();
    }
}
