//! Controlled Material switch built on the retained gesture button.
use super::ThemeData;
use crate::{
    animation::{Curve, TweenAnimationBuilder},
    egui::{self, Color32, Stroke, Vec2},
    element::{BuildContext, StatelessWidget, Widget},
    gestures::{GestureBinding, GestureButton, GestureSemanticRole},
};
use std::{rc::Rc, time::Duration};

/// A controlled Material switch for a single on/off setting.
///
/// The owner supplies `value` again after `on_changed` runs; the thumb then
/// animates to the updated controlled value.
pub struct Switch {
    binding: GestureBinding,
    value: bool,
    enabled: bool,
    on_changed: Rc<dyn Fn(bool)>,
    semantics_label: Option<String>,
}

impl Switch {
    pub fn new(binding: &GestureBinding, value: bool, on_changed: impl Fn(bool) + 'static) -> Self {
        Self {
            binding: binding.clone(),
            value,
            enabled: true,
            on_changed: Rc::new(on_changed),
            semantics_label: None,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets an accessible name for screen readers on hosts with semantics support.
    pub fn semantics_label(mut self, label: impl Into<String>) -> Self {
        self.semantics_label = Some(label.into());
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
        let binding = switch.binding.clone();
        let enabled = switch.enabled;
        let semantics_label = switch.semantics_label.clone();
        let target = if value { 1.0_f32 } else { 0.0_f32 };
        TweenAnimationBuilder::new(
            target,
            target,
            Duration::from_millis(200),
            move |_, position| {
                let callback = callback.clone();
                let semantics_label = semantics_label.clone();
                let paint = move |painter: &egui::Painter, bounds: egui::Rect| {
                    let track_rect =
                        egui::Rect::from_center_size(bounds.center(), Vec2::new(52.0, 32.0));
                    painter.rect_filled(track_rect, egui::CornerRadius::same(16), track);
                    painter.rect_stroke(
                        track_rect,
                        egui::CornerRadius::same(16),
                        Stroke::new(1.0_f32, outline),
                        egui::StrokeKind::Inside,
                    );
                    let x_offset = -10.0 + 20.0 * position.clamp(0.0, 1.0);
                    painter.circle_filled(
                        egui::pos2(bounds.center().x + x_offset, bounds.center().y),
                        10.0,
                        thumb,
                    );
                };
                GestureButton::new(&binding, egui::RichText::new(""), move || callback(!value))
                    .size(Vec2::new(56.0, 40.0))
                    .enabled(enabled)
                    .fill(Color32::TRANSPARENT)
                    .stroke(Stroke::NONE)
                    .corner_radius(egui::CornerRadius::ZERO)
                    .accessibility_role(GestureSemanticRole::Switch)
                    .accessibility_toggled(value)
                    .accessibility_label(semantics_label.unwrap_or_default())
                    .paint_content(paint)
                    .into_widget()
            },
        )
        .expect("finite Material switch animation endpoints")
        .with_curve(Curve::EaseOut)
        .into_widget()
    }
}
