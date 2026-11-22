//! Typed route stacks. The root route cannot be popped.
use crate::{
    element::{BuildContext, StackItem, StackStyle, Widget},
    layout::{Clip, StackFit},
    notifier::{Disposed, ValueNotifier},
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

type BackCallback = Rc<dyn Fn() -> bool>;

struct BackHandlerEntry {
    priority: i32,
    subscription: Weak<()>,
    callback: BackCallback,
}

/// UI-local dispatcher for Android system Back, macOS Escape and other app
/// back requests. The highest-priority live handler gets first refusal; ties
/// are resolved in favor of the most recently registered handler.
#[derive(Clone, Default)]
pub struct BackDispatcher {
    handlers: Rc<RefCell<Vec<BackHandlerEntry>>>,
}

impl BackDispatcher {
    /// Default priority used by route stacks.
    pub const ROUTE_PRIORITY: i32 = 0;
    /// Priority used by modal overlays so they dismiss before routes pop.
    pub const MODAL_PRIORITY: i32 = 100;

    pub fn new() -> Self {
        Self::default()
    }

    /// Register a handler until the returned subscription is dropped.
    pub fn register(&self, handler: impl Fn() -> bool + 'static) -> BackSubscription {
        self.register_with_priority(Self::ROUTE_PRIORITY, handler)
    }

    /// Register a handler at an explicit priority. Higher values receive Back
    /// first; handlers at the same priority are tried newest first.
    pub fn register_with_priority(
        &self,
        priority: i32,
        handler: impl Fn() -> bool + 'static,
    ) -> BackSubscription {
        let subscription = Rc::new(());
        self.handlers.borrow_mut().push(BackHandlerEntry {
            priority,
            subscription: Rc::downgrade(&subscription),
            callback: Rc::new(handler),
        });
        BackSubscription {
            _lease: subscription,
        }
    }

    /// Dispatch back to the highest-priority live handler that consumes it.
    pub fn dispatch(&self) -> bool {
        let mut handlers = {
            let mut entries = self.handlers.borrow_mut();
            entries.retain(|entry| entry.subscription.strong_count() > 0);
            entries
                .iter()
                .enumerate()
                .map(|(order, entry)| {
                    (
                        entry.priority,
                        order,
                        entry.subscription.clone(),
                        entry.callback.clone(),
                    )
                })
                .collect::<Vec<_>>()
        };
        handlers.sort_by_key(|(priority, order, _, _)| (*priority, *order));
        handlers
            .into_iter()
            .rev()
            .any(|(_, _, subscription, handler)| subscription.upgrade().is_some() && handler())
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
    routes: ValueNotifier<Vec<RouteEntry<R>>>,
    next_route_id: Rc<Cell<u64>>,
}

#[derive(Clone, PartialEq)]
struct RouteEntry<R> {
    id: u64,
    route: R,
}

impl<R: Clone + PartialEq + 'static> NavigatorController<R> {
    pub fn new(root: R) -> Self {
        Self {
            routes: ValueNotifier::new(vec![RouteEntry { id: 0, route: root }]),
            next_route_id: Rc::new(Cell::new(1)),
        }
    }

    pub fn routes(&self) -> Vec<R> {
        self.routes
            .read(|routes| routes.iter().map(|entry| entry.route.clone()).collect())
    }

    pub fn current(&self) -> Option<R> {
        self.routes
            .read(|routes| routes.last().map(|entry| entry.route.clone()))
    }

    pub fn can_pop(&self) -> bool {
        self.routes.read(|routes| routes.len() > 1)
    }

    /// Register this route stack at the standard route priority. Keep the
    /// returned subscription alive while the navigator should handle Back.
    pub fn register_back_handler(&self, dispatcher: &BackDispatcher) -> BackSubscription {
        let navigator = self.clone();
        dispatcher.register(move || navigator.pop().is_ok_and(|popped| popped.is_some()))
    }

    pub fn push(&self, route: R) -> Result<bool, Disposed> {
        let entry = RouteEntry {
            id: self.allocate_route_id(),
            route,
        };
        self.routes.update(|routes| {
            let mut next = routes.clone();
            next.push(entry);
            next
        })
    }

    pub fn pop(&self) -> Result<Option<R>, Disposed> {
        let mut popped = None;
        self.routes
            .update(|routes| {
                let mut next = routes.clone();
                if next.len() > 1 {
                    popped = next.pop().map(|entry| entry.route);
                }
                next
            })
            .map(|_| popped)
    }

    pub fn replace(&self, route: R) -> Result<Option<R>, Disposed> {
        let mut replaced = None;
        let entry = RouteEntry {
            id: self.allocate_route_id(),
            route,
        };
        self.routes
            .update(|routes| {
                let mut next = routes.clone();
                if let Some(current) = next.last_mut() {
                    replaced = Some(std::mem::replace(current, entry).route);
                }
                next
            })
            .map(|_| replaced)
    }

    pub fn reset(&self, root: R) -> Result<bool, Disposed> {
        self.routes.set(vec![RouteEntry {
            id: self.allocate_route_id(),
            route: root,
        }])
    }

    /// Rebuild the current route's widget whenever the stack changes.
    pub fn view(&self, builder: impl Fn(&BuildContext, &R) -> Widget + 'static) -> Widget {
        let builder = Rc::new(builder);
        Widget::value_listenable_builder(self.routes.clone(), move |context, routes| {
            routes
                .last()
                .map_or_else(Widget::empty, |route| builder(context, &route.route))
        })
    }

    /// Build every route in an expanding stack and keep inactive routes mounted
    /// offstage. This preserves route-local state while later routes are pushed.
    pub fn retained_view(&self, builder: impl Fn(&BuildContext, &R) -> Widget + 'static) -> Widget {
        let builder = Rc::new(builder);
        Widget::value_listenable_builder(self.routes.clone(), move |context, routes| {
            let current = routes.len().saturating_sub(1);
            let pages = routes
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    let active = index == current;
                    let page = builder(context, &entry.route);
                    let page = Widget::ticker_mode(active, page);
                    StackItem::new(Widget::offstage(!active, page).keyed(entry.id))
                })
                .collect();
            Widget::stack(
                StackStyle {
                    fit: StackFit::Expand,
                    clip: Clip::HardEdge,
                    ..Default::default()
                },
                pages,
            )
        })
    }

    fn allocate_route_id(&self) -> u64 {
        let id = self.next_route_id.get();
        self.next_route_id
            .set(id.checked_add(1).expect("navigator route id exhausted"));
        id
    }
}
