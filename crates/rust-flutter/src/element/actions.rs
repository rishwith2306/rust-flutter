use std::{
    any::{Any, TypeId},
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

type Handler = Rc<dyn Fn(&dyn Any)>;

/// UI-local typed Intent-to-handler registry for retained shortcut scopes.
/// Clones share registrations. Handlers run synchronously on the UI thread.
#[derive(Clone, Default)]
pub struct Actions {
    handlers: Rc<RefCell<HashMap<TypeId, Handler>>>,
}

impl Actions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register or replace the handler for an intent type.
    pub fn register<I: Any>(&self, handler: impl Fn(&I) + 'static) {
        self.handlers.borrow_mut().insert(
            TypeId::of::<I>(),
            Rc::new(move |intent| {
                if let Some(intent) = intent.downcast_ref::<I>() {
                    handler(intent);
                }
            }),
        );
    }

    /// Invoke the handler registered for this value's concrete type.
    /// Returns `false` when no handler is registered.
    pub fn invoke<I: Any>(&self, intent: &I) -> bool {
        let handler = self.handlers.borrow().get(&TypeId::of::<I>()).cloned();
        if let Some(handler) = handler {
            handler(intent);
            true
        } else {
            false
        }
    }

    pub fn remove<I: Any>(&self) -> bool {
        self.handlers
            .borrow_mut()
            .remove(&TypeId::of::<I>())
            .is_some()
    }
}
