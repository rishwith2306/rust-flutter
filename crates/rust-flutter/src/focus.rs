//! Retained keyboard-focus handles for framework text controls.
//!
//! This is an adapter to egui focus, not Flutter's complete focus tree or
//! traversal manager. A node currently attaches to one descendant text field.
use crate::{
    egui,
    element::Widget,
    notifier::{ChangeNotifier, Disposed, Listenable, Listener, Subscription},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct FocusCore {
    target: RefCell<Option<(egui::Context, egui::Id)>>,
    focused: Cell<bool>,
    pending: Cell<bool>,
    can_request: Cell<bool>,
    autofocus_requested: Cell<bool>,
    notifier: ChangeNotifier,
}
/// Application-owned handle for requesting and observing a text control's focus.
#[derive(Clone)]
pub struct FocusNode(Rc<FocusCore>);
impl Default for FocusNode {
    fn default() -> Self {
        Self::new()
    }
}
impl FocusNode {
    pub fn new() -> Self {
        Self(Rc::new(FocusCore {
            target: RefCell::new(None),
            focused: Cell::new(false),
            pending: Cell::new(false),
            can_request: Cell::new(true),
            autofocus_requested: Cell::new(false),
            notifier: ChangeNotifier::new(),
        }))
    }
    pub fn has_focus(&self) -> bool {
        self.0.focused.get()
    }
    pub fn has_primary_focus(&self) -> bool {
        self.has_focus()
    }
    pub fn can_request_focus(&self) -> bool {
        self.0.can_request.get()
    }
    pub fn set_can_request_focus(&self, value: bool) {
        if self.0.can_request.replace(value) != value {
            if !value {
                self.unfocus();
            }
            let _ = self.0.notifier.notify_listeners();
        }
    }
    /// Requests focus now, or queues it until a descendant text control mounts.
    pub fn request_focus(&self) -> bool {
        if !self.can_request_focus() {
            return false;
        }
        if let Some((ctx, id)) = self.0.target.borrow().as_ref() {
            ctx.memory_mut(|m| m.request_focus(*id));
            ctx.request_repaint();
            true
        } else {
            self.0.pending.set(true);
            true
        }
    }
    fn request_autofocus(&self) {
        if !self.0.autofocus_requested.replace(true) {
            let _ = self.request_focus();
        }
    }
    pub fn unfocus(&self) {
        self.0.pending.set(false);
        if let Some((ctx, id)) = self.0.target.borrow().as_ref() {
            ctx.memory_mut(|m| m.surrender_focus(*id));
            ctx.request_repaint();
        }
        self.sync(false);
    }
    pub(super) fn attach(&self, ctx: &egui::Context, id: egui::Id) {
        self.0.target.replace(Some((ctx.clone(), id)));
        if self.0.pending.replace(false) && self.can_request_focus() {
            ctx.memory_mut(|m| m.request_focus(id));
            ctx.request_repaint();
        }
        self.sync(ctx.memory(|m| m.has_focus(id)));
    }
    pub(super) fn sync(&self, focused: bool) {
        if self.0.focused.replace(focused) != focused {
            let _ = self.0.notifier.notify_listeners();
        }
    }
    pub(super) fn detach(&self, id: egui::Id) {
        let matches = self
            .0
            .target
            .borrow()
            .as_ref()
            .is_some_and(|(_, target)| *target == id);
        if matches {
            let target = self.0.target.take();
            if let Some((ctx, target_id)) = target {
                ctx.memory_mut(|m| m.surrender_focus(target_id));
            }
            self.sync(false);
        }
    }
}
impl Listenable for FocusNode {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
        self.0.notifier.subscribe(listener)
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl PartialEq for FocusNode {
    fn eq(&self, other: &Self) -> bool {
        self.same_source(other)
    }
}

/// Provides a focus node to descendants. `TextFormField` attaches its egui
/// editor to this node so callers can request focus and observe focus changes.
pub struct Focus {
    pub node: FocusNode,
    pub autofocus: bool,
    child: Widget,
}
impl Focus {
    pub fn new(node: FocusNode, child: Widget) -> Self {
        Self {
            node,
            autofocus: false,
            child,
        }
    }
    pub fn autofocus(mut self, value: bool) -> Self {
        self.autofocus = value;
        self
    }
    pub fn into_widget(self) -> Widget {
        if self.autofocus {
            self.node.request_autofocus();
        }
        Widget::inherited_notifier(Some(self.node), self.child)
    }
}
