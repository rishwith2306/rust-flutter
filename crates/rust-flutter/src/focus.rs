//! Retained keyboard-focus handles for framework text controls.
//!
//! This is an adapter to egui focus, not Flutter's complete focus tree or
//! traversal manager. A node currently attaches to one descendant text field or
//! retained gesture button.
use crate::{
    egui,
    element::Widget,
    notifier::{ChangeNotifier, Disposed, Listenable, Listener, Subscription},
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
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

/// Provides a focus node to descendants. `TextFormField` and `GestureButton`
/// attach their egui control to this node so callers can request focus and
/// observe focus changes.
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

struct ScopeTarget {
    context: egui::Context,
    order: usize,
    can_focus: bool,
}
struct FocusScopeCore {
    targets: RefCell<HashMap<egui::Id, ScopeTarget>>,
    pass: Cell<Option<u64>>,
    next_order: Cell<usize>,
    focused: Cell<Option<egui::Id>>,
    pending: Cell<bool>,
    can_request: Cell<bool>,
    handled_frame: Cell<Option<u64>>,
    notifier: ChangeNotifier,
}
/// A retained group of framework focus targets with explicit cyclic traversal.
#[derive(Clone)]
pub struct FocusScopeNode(Rc<FocusScopeCore>);
impl Default for FocusScopeNode {
    fn default() -> Self {
        Self::new()
    }
}
impl FocusScopeNode {
    pub fn new() -> Self {
        Self(Rc::new(FocusScopeCore {
            targets: RefCell::new(HashMap::new()),
            pass: Cell::new(None),
            next_order: Cell::new(0),
            focused: Cell::new(None),
            pending: Cell::new(false),
            can_request: Cell::new(true),
            handled_frame: Cell::new(None),
            notifier: ChangeNotifier::new(),
        }))
    }
    pub fn has_focus(&self) -> bool {
        self.0.focused.get().is_some()
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
    /// Focuses the first enabled target, or queues the request until one mounts.
    pub fn request_focus(&self) -> bool {
        if !self.can_request_focus() {
            return false;
        }
        let first = self
            .ordered_targets()
            .into_iter()
            .find(|(_, t)| t.can_focus);
        if let Some((id, target)) = first {
            target.context.memory_mut(|m| m.request_focus(id));
            target.context.request_repaint();
            self.0.pending.set(false);
            true
        } else {
            self.0.pending.set(true);
            true
        }
    }
    pub fn next_focus(&self) -> bool {
        self.traverse(true)
    }
    pub fn previous_focus(&self) -> bool {
        self.traverse(false)
    }
    pub fn unfocus(&self) {
        self.0.pending.set(false);
        let current = self.0.focused.get();
        if let Some(id) = current
            && let Some(target) = self.0.targets.borrow().get(&id)
        {
            target.context.memory_mut(|m| m.surrender_focus(id));
            target.context.request_repaint();
        }
        self.sync(None);
    }
    fn ordered_targets(&self) -> Vec<(egui::Id, ScopeTarget)> {
        let mut values = self
            .0
            .targets
            .borrow()
            .iter()
            .map(|(id, t)| {
                (
                    *id,
                    ScopeTarget {
                        context: t.context.clone(),
                        order: t.order,
                        can_focus: t.can_focus,
                    },
                )
            })
            .collect::<Vec<_>>();
        values.sort_by_key(|(_, t)| t.order);
        values
    }
    fn traverse(&self, forward: bool) -> bool {
        if !self.can_request_focus() {
            return false;
        }
        let values = self
            .ordered_targets()
            .into_iter()
            .filter(|(_, t)| t.can_focus)
            .collect::<Vec<_>>();
        if values.is_empty() {
            return false;
        }
        let current = self.0.focused.get();
        let index = values.iter().position(|(id, _)| Some(*id) == current);
        let next = match (index, forward) {
            (Some(i), true) => (i + 1) % values.len(),
            (Some(i), false) => (i + values.len() - 1) % values.len(),
            (None, true) => 0,
            (None, false) => values.len() - 1,
        };
        let (id, target) = &values[next];
        target.context.memory_mut(|m| m.request_focus(*id));
        target.context.request_repaint();
        self.0.pending.set(false);
        true
    }
    fn sync(&self, focused: Option<egui::Id>) {
        if self.0.focused.replace(focused) != focused {
            let _ = self.0.notifier.notify_listeners();
        }
    }
    pub(super) fn refresh_focus(&self) {
        let focused = self.0.targets.borrow().iter().find_map(|(id, target)| {
            (target.can_focus && target.context.memory(|m| m.has_focus(*id))).then_some(*id)
        });
        self.sync(focused);
        let Some(context) = focused.and_then(|id| {
            self.0
                .targets
                .borrow()
                .get(&id)
                .map(|target| target.context.clone())
        }) else {
            return;
        };
        let frame = context.cumulative_frame_nr();
        if self.0.handled_frame.get() == Some(frame) || !context.input(|i| i.focused) {
            return;
        }
        let tab = context.input(|i| {
            i.events.iter().find_map(|event| match event {
                egui::Event::Key {
                    key: egui::Key::Tab,
                    pressed: true,
                    modifiers,
                    ..
                } if !modifiers.ctrl && !modifiers.alt && !modifiers.command => {
                    Some((event.clone(), modifiers.shift))
                }
                _ => None,
            })
        });
        let Some((event, backwards)) = tab else {
            return;
        };
        let consumed = context.input_mut(|i| {
            if let Some(index) = i.events.iter().position(|candidate| *candidate == event) {
                i.events.remove(index);
                true
            } else {
                false
            }
        });
        if consumed {
            self.0.handled_frame.set(Some(frame));
            let _ = self.traverse(!backwards);
        }
    }
    pub(super) fn prepare_keyboard_navigation(&self, context: &egui::Context) {
        let Some(id) = context.memory(|memory| memory.focused()) else {
            return;
        };
        if !self.0.targets.borrow().contains_key(&id) {
            return;
        }
        context.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            )
        });
    }
    pub(super) fn register_target(
        &self,
        context: &egui::Context,
        id: egui::Id,
        can_focus: bool,
    ) -> FocusTarget {
        let pass = context.cumulative_pass_nr();
        if self.0.pass.replace(Some(pass)) != Some(pass) {
            self.0.next_order.set(0);
        }
        let order = self.0.next_order.get();
        self.0.next_order.set(order.saturating_add(1));
        self.0.targets.borrow_mut().insert(
            id,
            ScopeTarget {
                context: context.clone(),
                order,
                can_focus,
            },
        );
        let handle = FocusTarget {
            scope: Rc::downgrade(&self.0),
            id,
        };
        if self.0.pending.get() && can_focus && self.can_request_focus() {
            let _ = self.request_focus();
        }
        handle
    }
    pub(super) fn update_target(&self, context: &egui::Context, id: egui::Id, can_focus: bool) {
        let pass = context.cumulative_pass_nr();
        if self.0.pass.replace(Some(pass)) != Some(pass) {
            self.0.next_order.set(0);
        }
        let order = self.0.next_order.get();
        self.0.next_order.set(order.saturating_add(1));
        if let Some(target) = self.0.targets.borrow_mut().get_mut(&id) {
            target.context = context.clone();
            target.order = order;
            target.can_focus = can_focus;
        }
        if !can_focus && self.0.focused.get() == Some(id) {
            context.memory_mut(|m| m.surrender_focus(id));
            self.sync(None);
        }
        if can_focus && self.0.pending.get() && self.can_request_focus() {
            let _ = self.request_focus();
        }
    }
}
impl Listenable for FocusScopeNode {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
        self.0.notifier.subscribe(listener)
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl PartialEq for FocusScopeNode {
    fn eq(&self, other: &Self) -> bool {
        self.same_source(other)
    }
}
pub(super) struct FocusTarget {
    scope: Weak<FocusScopeCore>,
    id: egui::Id,
}
impl FocusTarget {
    pub(super) fn is_for(&self, scope: &FocusScopeNode, id: egui::Id) -> bool {
        self.id == id
            && self
                .scope
                .upgrade()
                .is_some_and(|core| Rc::ptr_eq(&core, &scope.0))
    }
}
impl Drop for FocusTarget {
    fn drop(&mut self) {
        if let Some(scope) = self.scope.upgrade() {
            let removed = scope.targets.borrow_mut().remove(&self.id);
            if scope.focused.get() == Some(self.id) {
                drop(removed);
                scope.focused.set(None);
                let _ = scope.notifier.notify_listeners();
            }
        }
    }
}
/// Provides a focus scope to descendant `TextFormField` and `GestureButton` widgets.
pub struct FocusScope {
    pub node: FocusScopeNode,
    child: Widget,
}
impl FocusScope {
    pub fn new(node: FocusScopeNode, child: Widget) -> Self {
        Self { node, child }
    }
    pub fn into_widget(self) -> Widget {
        Widget::inherited_notifier(Some(self.node), self.child)
    }
}
