use super::{StackItem, StackStyle, Widget};
use crate::egui::{self, KeyboardShortcut};
use std::rc::Rc;

struct Binding {
    shortcut: KeyboardShortcut,
    action: Rc<dyn Fn()>,
}

/// A retained subtree that handles configured keyboard shortcuts.
///
/// Child scopes paint first and therefore get first chance to consume a key.
/// Bindings bubble outwards when no child binding matches. This initial version
/// is subtree based; focus ancestry and typed Intent/Action dispatch are not
/// implemented.
pub struct Shortcuts {
    child: Widget,
    bindings: Vec<Binding>,
}

impl Shortcuts {
    pub fn new(child: Widget) -> Self {
        Self {
            child,
            bindings: Vec::new(),
        }
    }

    /// Handle a shortcut with a UI-local action callback.
    pub fn bind(mut self, shortcut: KeyboardShortcut, action: impl Fn() + 'static) -> Self {
        self.bindings.push(Binding {
            shortcut,
            action: Rc::new(action),
        });
        self
    }

    pub fn into_widget(self) -> Widget {
        let mut items = vec![StackItem::new(self.child)];
        for binding in self.bindings {
            let shortcut = binding.shortcut;
            let action = binding.action;
            let listener = Widget::leaf(egui::Vec2::ZERO, move |ui| {
                if ui.input_mut(|input| input.consume_shortcut(&shortcut)) {
                    action();
                }
            });
            items.push(StackItem::new(listener));
        }
        Widget::stack(StackStyle::default(), items)
    }
}
