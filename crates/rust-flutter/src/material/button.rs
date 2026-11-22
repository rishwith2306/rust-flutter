use super::ThemeData;
use crate::{
    egui::{self, Color32, Vec2},
    element::{BuildContext, StatelessWidget, Widget},
    gestures::{GestureBinding, GestureButton},
};
use std::rc::Rc;

#[derive(Clone, Copy)]
enum Variant {
    Filled,
    Outlined,
    Text,
}

#[derive(Clone)]
struct ButtonWidget {
    binding: GestureBinding,
    label: String,
    on_pressed: Rc<dyn Fn()>,
    enabled: bool,
    size: Vec2,
    variant: Variant,
}

impl StatelessWidget for ButtonWidget {
    fn build(&self, context: &BuildContext) -> Widget {
        let scheme = ThemeData::of(context)
            .expect("Material buttons resolve theme during build")
            .color_scheme;
        let (fill, foreground, stroke) = match (self.variant, self.enabled) {
            (Variant::Filled, true) => (scheme.primary, scheme.on_primary, egui::Stroke::NONE),
            (Variant::Filled, false) => (
                scheme.on_surface.gamma_multiply(0.12),
                scheme.on_surface.gamma_multiply(0.38),
                egui::Stroke::NONE,
            ),
            (Variant::Outlined, true) => (
                scheme.surface,
                scheme.primary,
                egui::Stroke::new(1.0_f32, scheme.outline),
            ),
            (Variant::Outlined, false) => (
                scheme.surface,
                scheme.on_surface.gamma_multiply(0.38),
                egui::Stroke::new(1.0_f32, scheme.on_surface.gamma_multiply(0.12)),
            ),
            (Variant::Text, true) => (Color32::TRANSPARENT, scheme.primary, egui::Stroke::NONE),
            (Variant::Text, false) => (
                Color32::TRANSPARENT,
                scheme.on_surface.gamma_multiply(0.38),
                egui::Stroke::NONE,
            ),
        };
        let callback = self.on_pressed.clone();
        let button = GestureButton::new(
            &self.binding,
            egui::RichText::new(self.label.clone()).color(foreground),
            move || callback(),
        )
        .size(self.size)
        .enabled(self.enabled)
        .stroke(stroke);
        let button = if matches!(self.variant, Variant::Text) {
            button
        } else {
            button.fill(fill)
        };
        button.into_widget()
    }
}

macro_rules! material_button {
    ($name:ident, $variant:expr) => {
        /// A retained Material button styled from the nearest [`ThemeData`].
        pub struct $name(ButtonWidget);

        impl $name {
            pub fn new(
                binding: &GestureBinding,
                label: impl Into<String>,
                on_pressed: impl Fn() + 'static,
            ) -> Self {
                Self(ButtonWidget {
                    binding: binding.clone(),
                    label: label.into(),
                    on_pressed: Rc::new(on_pressed),
                    enabled: true,
                    size: egui::vec2(120.0, 40.0),
                    variant: $variant,
                })
            }

            pub fn enabled(mut self, enabled: bool) -> Self {
                self.0.enabled = enabled;
                self
            }

            pub fn size(mut self, size: Vec2) -> Self {
                self.0.size = size;
                self
            }

            pub fn into_widget(self) -> Widget {
                Widget::stateless(self.0)
            }
        }
    };
}

material_button!(FilledButton, Variant::Filled);
material_button!(OutlinedButton, Variant::Outlined);
material_button!(TextButton, Variant::Text);
