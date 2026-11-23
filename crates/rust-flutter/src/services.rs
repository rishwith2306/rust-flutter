//! Typed service registry for app-defined platform implementations.
//! Register real implementations per target; this module does not emulate OS APIs.
use serde_json::Value;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

#[derive(Default)]
pub struct Services {
    values: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

/// Completion callback for an asynchronous platform method call.
pub type MethodReply = Box<dyn FnOnce(Result<Value, String>) + Send + 'static>;

/// A named, asynchronous method-call bridge implemented by an application or
/// plugin crate for each target platform.
pub trait MethodChannel: Send + Sync {
    /// Invoke a method on `channel` with a JSON-compatible argument value.
    /// Implementations may complete `reply` later and from another thread.
    fn invoke_method(&self, channel: &str, method: &str, arguments: Value, reply: MethodReply);
}

/// Type-erased shared method-channel bridge stored in [`Services`].
pub type MethodChannelService = Arc<dyn MethodChannel>;

impl Services {
    /// Read the app's immutable service registry from a retained widget build
    /// context. The host installs this value at the root of `run_widget` trees.
    pub fn inherited(
        context: &crate::element::BuildContext,
    ) -> Result<Option<Arc<Self>>, crate::element::DependencyError> {
        context
            .read::<Arc<Self>>()
            .map(|services| services.map(|services| Arc::clone(&services)))
    }

    /// Register a concrete type or `Arc<dyn MyPlatformService>`.
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) -> Option<T> {
        self.values
            .insert(TypeId::of::<T>(), Box::new(value))
            .map(|v| *v.downcast::<T>().unwrap())
    }
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.values.get(&TypeId::of::<T>())?.downcast_ref()
    }
}
/// Example common platform contract. Implementations must use the OS permission flow.
pub trait Clipboard: Send + Sync {
    fn write_text(&self, text: &str) -> Result<(), String>;
}
pub type ClipboardService = Arc<dyn Clipboard>;

/// Open a URL using the host platform's default handler.
pub trait UrlLauncher: Send + Sync {
    /// Queue a URL for the host. The result reports queueing, not whether the
    /// operating system later finds a handler or successfully opens it.
    fn open(&self, url: &str) -> Result<(), String>;
}
pub type UrlLauncherService = Arc<dyn UrlLauncher>;

struct EguiClipboard(egui::Context);
impl Clipboard for EguiClipboard {
    fn write_text(&self, text: &str) -> Result<(), String> {
        self.0.copy_text(text.to_owned());
        Ok(())
    }
}

pub(crate) fn host_clipboard(context: &egui::Context) -> ClipboardService {
    Arc::new(EguiClipboard(context.clone()))
}

struct EguiUrlLauncher(egui::Context);
impl UrlLauncher for EguiUrlLauncher {
    fn open(&self, url: &str) -> Result<(), String> {
        self.0.open_url(egui::OpenUrl::same_tab(url));
        Ok(())
    }
}

pub(crate) fn host_url_launcher(context: &egui::Context) -> UrlLauncherService {
    Arc::new(EguiUrlLauncher(context.clone()))
}
