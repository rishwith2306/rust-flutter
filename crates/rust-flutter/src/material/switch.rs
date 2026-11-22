//! Controlled Material switch built on the retained gesture button.
use super::ThemeData;
use crate::{
    egui::{self, Stroke, Vec2},
    element::{BuildContext, StatelessWidget, Widget},
    gestures::{GestureBinding, GestureButton},
};
use std::rc::Rc;

/// A controlled Material switch for a single on/off setting.
///
/// The owner supplies `value` again after `on_changed` runs. This initial
/// implementation uses a text glyph for the thumb and does not animate it.
pub struct Switch {
    binding: GestureBinding,
    value: bool,
    enabled: bool,
    on_changed: Rc<dyn Fn(bool)>,
}

impl Switch {
    pub fn new(binding: &GestureBinding, value: bool, on_changed: impl Fn(bool) + 'static) -> Self {
        Self {
            binding: binding.clone(),
            value,
            enabled: true,
            on_changed: Rc::new(on_changed),
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn into_widget(self) -> Widget {
        Widget::stateless(SwitchView(self))
    }
}

struct SwitchView(Switch);

impl StatelessWidget for SwitchView {
    fn build(&self, context: &BuildContext) -> Widget {
        let switch = &self.0;
        let scheme = ThemeData::of(context)
            .expect("Material switches resolve theme during build")
            .color_scheme;
        let (label, fill, foreground, stroke) = if switch.enabled {
            if switch.value {
                (
                    "   ●",
                    scheme.primary_container,
                    scheme.on_primary_container,
                    Stroke::new(1.0_f32, scheme.primary),
                )
            } else {
                (
                    "●   ",
                    scheme.surface_container,
                    scheme.on_surface_variant,
                    Stroke::new(1.0_f32, scheme.outline),
                )
            }
        } else {
            (
                if switch.value { "   ●" } else { "●   " },
                scheme.on_surface.gamma_multiply(0.12),
                scheme.on_surface.gamma_multiply(0.38),
                Stroke::new(1.0_f32, scheme.on_surface.gamma_multiply(0.12)),
            )
        };
        let callback = switch.on_changed.clone();
        let value = switch.value;
        GestureButton::new(
            &switch.binding,
            egui::RichText::new(label).color(foreground),
            move || callback(!value),
        )
        .size(Vec2::new(56.0, 36.0))
        .enabled(switch.enabled)
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(18))
        .into_widget()
    }
}
