//! Typed service registry for app-defined platform implementations.
//! Register real implementations per target; this module does not emulate OS APIs.
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};
#[derive(Default)]
pub struct Services {
    values: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}
impl Services {
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
