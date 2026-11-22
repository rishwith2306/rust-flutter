//! Controlled Material checkbox built on the retained gesture button.
use super::ThemeData;
use crate::{
    egui::{self, Color32, Stroke, Vec2},
    element::{BuildContext, StatelessWidget, Widget},
    gestures::{GestureBinding, GestureButton},
};
use std::rc::Rc;

/// A controlled Material checkbox.
///
/// The owner supplies `value` again after `on_changed` runs. With `tristate`
/// enabled, values cycle `false -> true -> None -> false`.
pub struct Checkbox {
    binding: GestureBinding,
    value: Option<bool>,
    tristate: bool,
    enabled: bool,
    on_changed: Rc<dyn Fn(Option<bool>)>,
}

impl Checkbox {
    pub fn new(
        binding: &GestureBinding,
        value: Option<bool>,
        on_changed: impl Fn(Option<bool>) + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            value,
            tristate: false,
            enabled: true,
            on_changed: Rc::new(on_changed),
        }
    }

    pub fn tristate(mut self, tristate: bool) -> Self {
        self.tristate = tristate;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Build the checkbox widget, rejecting `None` unless tristate is enabled.
    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if self.value.is_none() && !self.tristate {
            return Err("a non-tristate checkbox requires a true or false value");
        }
        Ok(Widget::stateless(CheckboxView(self)))
    }
}

struct CheckboxView(Checkbox);

impl StatelessWidget for CheckboxView {
    fn build(&self, context: &BuildContext) -> Widget {
        let checkbox = &self.0;
        let scheme = ThemeData::of(context)
            .expect("Material checkboxes resolve theme during build")
            .color_scheme;
        let (glyph, fill, foreground, stroke) = match (checkbox.value, checkbox.enabled) {
            (Some(true), true) => (
                "✓",
                scheme.primary,
                scheme.on_primary,
                Stroke::new(1.0_f32, scheme.primary),
            ),
            (None, true) => (
                "−",
                scheme.primary,
                scheme.on_primary,
                Stroke::new(1.0_f32, scheme.primary),
            ),
            (Some(false), true) => (
                " ",
                Color32::TRANSPARENT,
                scheme.on_surface,
                Stroke::new(1.5_f32, scheme.outline),
            ),
            (Some(true), false) => (
                "✓",
                scheme.on_surface.gamma_multiply(0.12),
                scheme.on_surface.gamma_multiply(0.38),
                Stroke::new(1.0_f32, scheme.on_surface.gamma_multiply(0.12)),
            ),
            (None, false) => (
                "−",
                scheme.on_surface.gamma_multiply(0.12),
                scheme.on_surface.gamma_multiply(0.38),
                Stroke::new(1.0_f32, scheme.on_surface.gamma_multiply(0.12)),
            ),
            (Some(false), false) => (
                " ",
                Color32::TRANSPARENT,
                scheme.on_surface.gamma_multiply(0.38),
                Stroke::new(1.5_f32, scheme.on_surface.gamma_multiply(0.38)),
            ),
        };
        let changed = checkbox.on_changed.clone();
        let value = checkbox.value;
        let tristate = checkbox.tristate;
        GestureButton::new(
            &checkbox.binding,
            egui::RichText::new(glyph).color(foreground),
            move || {
                changed(checkbox_next(value, tristate));
            },
        )
        .size(Vec2::splat(48.0))
        .enabled(checkbox.enabled)
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(4))
        .into_widget()
    }
}

fn checkbox_next(value: Option<bool>, tristate: bool) -> Option<bool> {
    match (value, tristate) {
        (Some(false), _) => Some(true),
        (Some(true), true) => None,
        (Some(true), false) => Some(false),
        (None, true) => Some(false),
        (None, false) => None,
    }
}
