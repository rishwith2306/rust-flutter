use super::{StackItem, StackStyle, Widget};
use crate::{egui, layout::Positioned};

/// Shows a native egui tooltip when the pointer hovers over a retained child.
/// The hover layer does not claim clicks or gestures from the child.
pub struct Tooltip {
    child: Widget,
    message: String,
    enabled: bool,
}

impl Tooltip {
    pub fn new(child: Widget, message: impl Into<String>) -> Self {
        Self {
            child,
            message: message.into(),
            enabled: true,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn into_widget(self) -> Widget {
        if !self.enabled || self.message.is_empty() {
            return self.child;
        }
        let message = self.message;
        let hover = Widget::leaf(egui::Vec2::ZERO, move |ui| {
            ui.interact(
                ui.max_rect(),
                ui.id().with("rust_flutter_tooltip"),
                egui::Sense::hover(),
            )
            .on_hover_text(message.clone());
        });
        Widget::stack(
            StackStyle::default(),
            vec![
                StackItem::new(self.child),
                StackItem::positioned(hover, Positioned::fill()),
            ],
        )
    }
}
