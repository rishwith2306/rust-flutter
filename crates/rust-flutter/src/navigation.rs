//! Typed route stacks. The root route cannot be popped.
use crate::{
    element::{BuildContext, Widget},
    notifier::{Disposed, ValueNotifier},
};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

type BackCallback = Rc<dyn Fn() -> bool>;

struct BackHandlerEntry {
    subscription: Weak<()>,
    callback: BackCallback,
}

/// UI-local dispatcher for Android system Back, macOS Escape and other app
/// back requests. The most recently registered active handler gets first
/// refusal; a handler returns `true` when it consumed the request.
#[derive(Clone, Default)]
pub struct BackDispatcher {
    handlers: Rc<RefCell<Vec<BackHandlerEntry>>>,
}

impl BackDispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler until the returned subscription is dropped.
    pub fn register(&self, handler: impl Fn() -> bool + 'static) -> BackSubscription {
        let subscription = Rc::new(());
        self.handlers.borrow_mut().push(BackHandlerEntry {
            subscription: Rc::downgrade(&subscription),
            callback: Rc::new(handler),
        });
        BackSubscription {
            _lease: subscription,
        }
    }

    /// Dispatch back to the newest live handler that consumes the request.
    pub fn dispatch(&self) -> bool {
        let handlers = {
            let mut entries = self.handlers.borrow_mut();
            entries.retain(|entry| entry.subscription.strong_count() > 0);
            entries
                .iter()
                .rev()
                .map(|entry| (entry.subscription.clone(), entry.callback.clone()))
                .collect::<Vec<_>>()
        };
        handlers
            .into_iter()
            .any(|(subscription, handler)| subscription.upgrade().is_some() && handler())
    }

    /// Callback adapter for `run_widget_with_back_handler`.
    pub fn handler(&self) -> impl Fn() -> bool + 'static + use<> {
        let dispatcher = self.clone();
        move || dispatcher.dispatch()
    }

    pub(crate) fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.handlers, &other.handlers)
    }
}

/// Keeps a [`BackDispatcher`] handler active. Dropping it unregisters the
/// callback without requiring mutable access to the dispatcher.
pub struct BackSubscription {
    _lease: Rc<()>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Navigator<R> {
    stack: Vec<R>,
}
impl<R> Navigator<R> {
    pub fn new(root: R) -> Self {
        Self { stack: vec![root] }
    }
    pub fn current(&self) -> &R {
        self.stack.last().expect("root route always exists")
    }
    pub fn push(&mut self, route: R) {
        self.stack.push(route);
    }
    pub fn pop(&mut self) -> Option<R> {
        (self.stack.len() > 1).then(|| self.stack.pop().unwrap())
    }
    pub fn replace(&mut self, route: R) -> R {
        std::mem::replace(self.stack.last_mut().unwrap(), route)
    }
    pub fn reset(&mut self, root: R) {
        self.stack.clear();
        self.stack.push(root);
    }
    pub fn can_pop(&self) -> bool {
        self.stack.len() > 1
    }
    pub fn routes(&self) -> &[R] {
        &self.stack
    }
}

/// A UI-local route stack that notifies its retained view when pages change.
#[derive(Clone)]
pub struct NavigatorController<R: Clone + PartialEq + 'static> {
    routes: ValueNotifier<Vec<R>>,
}

impl<R: Clone + PartialEq + 'static> NavigatorController<R> {
    pub fn new(root: R) -> Self {
        Self {
            routes: ValueNotifier::new(vec![root]),
        }
    }

    pub fn routes(&self) -> Vec<R> {
        self.routes.get()
    }

    pub fn current(&self) -> Option<R> {
        self.routes.read(|routes| routes.last().cloned())
    }

    pub fn can_pop(&self) -> bool {
        self.routes.read(|routes| routes.len() > 1)
    }

    /// Register this route stack as a lower-priority back handler. Keep the
    /// returned subscription alive while the navigator should handle Back.
    pub fn register_back_handler(&self, dispatcher: &BackDispatcher) -> BackSubscription {
        let navigator = self.clone();
        dispatcher.register(move || navigator.pop().is_ok_and(|popped| popped.is_some()))
    }

    pub fn push(&self, route: R) -> Result<bool, Disposed> {
        self.routes.update(|routes| {
            let mut next = routes.clone();
            next.push(route);
            next
        })
    }

    pub fn pop(&self) -> Result<Option<R>, Disposed> {
        let mut popped = None;
        self.routes
            .update(|routes| {
                let mut next = routes.clone();
                if next.len() > 1 {
                    popped = next.pop();
                }
                next
            })
            .map(|_| popped)
    }

    pub fn replace(&self, route: R) -> Result<Option<R>, Disposed> {
        let mut replaced = None;
        self.routes
            .update(|routes| {
                let mut next = routes.clone();
                if let Some(current) = next.last_mut() {
                    replaced = Some(std::mem::replace(current, route));
                }
                next
            })
            .map(|_| replaced)
    }

    pub fn reset(&self, root: R) -> Result<bool, Disposed> {
        self.routes.set(vec![root])
    }

    /// Rebuild the current route's widget whenever the stack changes.
    pub fn view(&self, builder: impl Fn(&BuildContext, &R) -> Widget + 'static) -> Widget {
        let builder = Rc::new(builder);
        Widget::value_listenable_builder(self.routes.clone(), move |context, routes| {
            routes
                .last()
                .map_or_else(Widget::empty, |route| builder(context, route))
        })
    }
}
