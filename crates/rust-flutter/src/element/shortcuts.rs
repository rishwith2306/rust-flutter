use super::Actions;
use super::{StackItem, StackStyle, Widget};
use crate::egui::{self, KeyboardShortcut};
use crate::focus::FocusScopeNode;
use std::{any::Any, rc::Rc};

struct Binding {
    shortcut: KeyboardShortcut,
    action: Rc<dyn Fn()>,
}

/// A retained subtree that handles configured keyboard shortcuts.
///
/// Child scopes paint first and therefore get first chance to consume a key.
/// Bindings bubble outwards when no child binding matches. An optional focus
/// scope gate can restrict when this subtree handles keyboard input.
pub struct Shortcuts {
    child: Widget,
    bindings: Vec<Binding>,
    focus_scope: Option<FocusScopeNode>,
}

impl Shortcuts {
    pub fn new(child: Widget) -> Self {
        Self {
            child,
            bindings: Vec::new(),
            focus_scope: None,
        }
    }

    /// Restrict this scope to periods when the given focus scope owns focus.
    pub fn focus_scope(mut self, scope: FocusScopeNode) -> Self {
        self.focus_scope = Some(scope);
        self
    }

    /// Handle a shortcut with a UI-local action callback.
    pub fn bind(mut self, shortcut: KeyboardShortcut, action: impl Fn() + 'static) -> Self {
        self.bindings.push(Binding {
            shortcut,
            action: Rc::new(action),
        });
        self
    }

    /// Bind a shortcut to a typed intent handled by the provided action registry.
    pub fn bind_intent<I: Any + Clone>(
        self,
        shortcut: KeyboardShortcut,
        actions: Actions,
        intent: I,
    ) -> Self {
        self.bind(shortcut, move || {
            actions.invoke(&intent);
        })
    }

    pub fn into_widget(self) -> Widget {
        let mut items = vec![StackItem::new(self.child)];
        for binding in self.bindings {
            let shortcut = binding.shortcut;
            let action = binding.action;
            let focus_scope = self.focus_scope.clone();
            let listener = Widget::leaf(egui::Vec2::ZERO, move |ui| {
                let scope_active = focus_scope.as_ref().is_none_or(FocusScopeNode::has_focus);
                if scope_active && ui.input_mut(|input| input.consume_shortcut(&shortcut)) {
                    action();
                }
            });
            items.push(StackItem::new(listener));
        }
        Widget::stack(StackStyle::default(), items)
    }
}
