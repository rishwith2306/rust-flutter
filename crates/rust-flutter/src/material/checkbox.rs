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
        let (fill, outline, mark) = match (checkbox.value, checkbox.enabled) {
            (Some(true), true) | (None, true) => {
                (scheme.primary, scheme.primary, scheme.on_primary)
            }
            (Some(false), true) => (Color32::TRANSPARENT, scheme.outline, scheme.on_primary),
            (Some(true), false) | (None, false) => (
                scheme.on_surface.gamma_multiply(0.12),
                scheme.on_surface.gamma_multiply(0.12),
                scheme.on_surface.gamma_multiply(0.38),
            ),
            (Some(false), false) => (
                Color32::TRANSPARENT,
                scheme.on_surface.gamma_multiply(0.38),
                scheme.on_surface.gamma_multiply(0.38),
            ),
        };
        let changed = checkbox.on_changed.clone();
        let value = checkbox.value;
        let tristate = checkbox.tristate;
        let paint_value = checkbox.value;
        let paint = move |painter: &egui::Painter, bounds: egui::Rect| {
            let box_rect = egui::Rect::from_center_size(bounds.center(), Vec2::splat(18.0));
            let radius = egui::CornerRadius::same(4);
            painter.rect_filled(box_rect, radius, fill);
            painter.rect_stroke(
                box_rect,
                radius,
                Stroke::new(1.5_f32, outline),
                egui::StrokeKind::Inside,
            );
            match paint_value {
                Some(true) => {
                    painter.line_segment(
                        [
                            egui::pos2(box_rect.left() + 3.0, box_rect.center().y),
                            egui::pos2(box_rect.left() + 7.0, box_rect.bottom() - 4.0),
                        ],
                        Stroke::new(2.0_f32, mark),
                    );
                    painter.line_segment(
                        [
                            egui::pos2(box_rect.left() + 7.0, box_rect.bottom() - 4.0),
                            egui::pos2(box_rect.right() - 3.0, box_rect.top() + 4.0),
                        ],
                        Stroke::new(2.0_f32, mark),
                    );
                }
                None => {
                    painter.line_segment(
                        [
                            egui::pos2(box_rect.left() + 4.0, box_rect.center().y),
                            egui::pos2(box_rect.right() - 4.0, box_rect.center().y),
                        ],
                        Stroke::new(2.0_f32, mark),
                    );
                }
                Some(false) => {}
            }
        };
        GestureButton::new(&checkbox.binding, egui::RichText::new(""), move || {
            changed(checkbox_next(value, tristate));
        })
        .size(Vec2::splat(48.0))
        .enabled(checkbox.enabled)
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
        .corner_radius(egui::CornerRadius::ZERO)
        .paint_content(paint)
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
