use super::ThemeData;
use crate::{
    egui::{self, FontId, Stroke},
    element::{BuildContext, StatelessWidget, Tooltip, Widget},
    gestures::{GestureBinding, GestureButton, GestureSemanticRole},
};
use std::rc::Rc;

/// A controlled Material action chip with optional pointer-hover tooltip.
pub struct ActionChip {
    binding: GestureBinding,
    label: String,
    enabled: bool,
    tooltip: Option<String>,
    on_pressed: Rc<dyn Fn()>,
}

impl ActionChip {
    pub fn new(
        binding: &GestureBinding,
        label: impl Into<String>,
        on_pressed: impl Fn() + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            label: label.into(),
            enabled: true,
            tooltip: None,
            on_pressed: Rc::new(on_pressed),
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn tooltip(mut self, message: impl Into<String>) -> Self {
        self.tooltip = Some(message.into());
        self
    }

    pub fn into_widget(self) -> Widget {
        Widget::stateless(self)
    }
}

impl StatelessWidget for ActionChip {
    fn build(&self, context: &BuildContext) -> Widget {
        let scheme = ThemeData::of(context)
            .expect("Material action chip resolves theme during build")
            .color_scheme;
        let foreground = scheme.on_surface;
        let label = self.label.clone();
        let font = FontId::proportional(14.0);
        let width = context.egui().fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(label.clone(), font.clone(), foreground)
                .size()
                .x
        });
        let callback = self.on_pressed.clone();
        let chip = GestureButton::new(
            &self.binding,
            egui::RichText::new(label).font(font).color(foreground),
            move || callback(),
        )
        .size(egui::vec2((width + 24.0).max(64.0), 32.0))
        .fill(if self.enabled {
            scheme.surface_container_low
        } else {
            scheme.surface_container_low.gamma_multiply(0.5)
        })
        .stroke(Stroke::new(1.0_f32, scheme.outline))
        .corner_radius(egui::CornerRadius::same(8))
        .enabled(self.enabled)
        .accessibility_role(GestureSemanticRole::Button)
        .accessibility_label(self.label.clone())
        .into_widget();
        match &self.tooltip {
            Some(message) => Tooltip::new(chip, message.clone()).into_widget(),
            None => chip,
        }
    }
}
