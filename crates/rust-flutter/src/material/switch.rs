//! Controlled Material switch built on the retained gesture button.
use super::ThemeData;
use crate::{
    egui::{self, Color32, Stroke, Vec2},
    element::{BuildContext, StatelessWidget, Widget},
    gestures::{GestureBinding, GestureButton},
};
use std::rc::Rc;

/// A controlled Material switch for a single on/off setting.
///
/// The owner supplies `value` again after `on_changed` runs. This initial
/// implementation paints the track and thumb directly but does not animate
/// the thumb between states.
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
        let (track, outline, thumb) = if switch.enabled {
            if switch.value {
                (scheme.primary_container, scheme.primary, scheme.primary)
            } else {
                (
                    scheme.surface_container,
                    scheme.outline_variant,
                    scheme.outline,
                )
            }
        } else {
            (
                scheme.on_surface.gamma_multiply(0.12),
                scheme.on_surface.gamma_multiply(0.12),
                scheme.on_surface.gamma_multiply(0.38),
            )
        };
        let callback = switch.on_changed.clone();
        let value = switch.value;
        let paint_value = switch.value;
        let paint = move |painter: &egui::Painter, bounds: egui::Rect| {
            let track_rect = egui::Rect::from_center_size(bounds.center(), Vec2::new(52.0, 32.0));
            painter.rect_filled(track_rect, egui::CornerRadius::same(16), track);
            painter.rect_stroke(
                track_rect,
                egui::CornerRadius::same(16),
                Stroke::new(1.0_f32, outline),
                egui::StrokeKind::Inside,
            );
            let x_offset = if paint_value { 10.0 } else { -10.0 };
            painter.circle_filled(
                egui::pos2(bounds.center().x + x_offset, bounds.center().y),
                10.0,
                thumb,
            );
        };
        GestureButton::new(&switch.binding, egui::RichText::new(""), move || {
            callback(!value)
        })
        .size(Vec2::new(56.0, 40.0))
        .enabled(switch.enabled)
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
        .corner_radius(egui::CornerRadius::ZERO)
        .paint_content(paint)
        .into_widget()
    }
}
