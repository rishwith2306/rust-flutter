use super::ThemeData;
use crate::{
    egui::{self, FontId, Stroke},
    element::{BuildContext, StatelessWidget, Widget},
    gestures::{GestureBinding, GestureButton, GestureSemanticRole},
};
use std::rc::Rc;

/// A controlled Material choice chip with radio selection semantics.
pub struct ChoiceChip {
    binding: GestureBinding,
    label: String,
    selected: bool,
    enabled: bool,
    on_selected: Rc<dyn Fn()>,
}

impl ChoiceChip {
    pub fn new(
        binding: &GestureBinding,
        label: impl Into<String>,
        selected: bool,
        on_selected: impl Fn() + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            label: label.into(),
            selected,
            enabled: true,
            on_selected: Rc::new(on_selected),
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn into_widget(self) -> Widget {
        Widget::stateless(self)
    }
}

impl StatelessWidget for ChoiceChip {
    fn build(&self, context: &BuildContext) -> Widget {
        let scheme = ThemeData::of(context)
            .expect("Material choice chip resolves theme during build")
            .color_scheme;
        let (fill, foreground, stroke) = if self.selected {
            (
                scheme.secondary_container,
                scheme.on_secondary_container,
                Stroke::NONE,
            )
        } else {
            (
                scheme.surface_container_low,
                scheme.on_surface_variant,
                Stroke::new(1.0_f32, scheme.outline),
            )
        };
        let label = if self.selected {
            format!("✓  {}", self.label)
        } else {
            self.label.clone()
        };
        let font = FontId::proportional(14.0);
        let width = context.egui().fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(label.clone(), font.clone(), foreground)
                .size()
                .x
        });
        let callback = self.on_selected.clone();
        GestureButton::new(
            &self.binding,
            egui::RichText::new(label).font(font).color(foreground),
            move || callback(),
        )
        .size(egui::vec2((width + 24.0).max(64.0), 32.0))
        .fill(if self.enabled {
            fill
        } else {
            fill.gamma_multiply(0.5)
        })
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(8))
        .enabled(self.enabled)
        .accessibility_role(GestureSemanticRole::RadioButton)
        .accessibility_label(self.label.clone())
        .accessibility_selected(self.selected)
        .into_widget()
    }
}
