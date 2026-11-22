//! Typed route stacks. The root route cannot be popped.
use crate::{
    element::{BuildContext, Widget},
    notifier::{Disposed, ValueNotifier},
};
use std::rc::Rc;

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
