//! Controlled Material radio selection built on the retained gesture button.
use super::ThemeData;
use crate::{
    egui::{self, Color32, Stroke, Vec2},
    element::{BuildContext, StatelessWidget, Widget},
    gestures::{GestureBinding, GestureButton, GestureSemanticRole},
};
use std::rc::Rc;

/// A controlled radio option for a group whose selected value is owned by the
/// parent widget.
pub struct Radio<T: Clone + PartialEq + 'static> {
    binding: GestureBinding,
    value: T,
    group_value: Option<T>,
    enabled: bool,
    on_changed: Rc<dyn Fn(T)>,
    semantics_label: Option<String>,
}

impl<T: Clone + PartialEq + 'static> Radio<T> {
    pub fn new(
        binding: &GestureBinding,
        value: T,
        group_value: Option<T>,
        on_changed: impl Fn(T) + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            value,
            group_value,
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
        Widget::stateless(RadioView(self))
    }
}

struct RadioView<T: Clone + PartialEq + 'static>(Radio<T>);

impl<T: Clone + PartialEq + 'static> StatelessWidget for RadioView<T> {
    fn build(&self, context: &BuildContext) -> Widget {
        let radio = &self.0;
        let scheme = ThemeData::of(context)
            .expect("Material radios resolve theme during build")
            .color_scheme;
        let selected = radio.group_value.as_ref() == Some(&radio.value);
        let (outline, dot) = if radio.enabled {
            if selected {
                (scheme.primary, scheme.primary)
            } else {
                (scheme.outline, Color32::TRANSPARENT)
            }
        } else {
            let disabled = scheme.on_surface.gamma_multiply(0.38);
            (
                disabled,
                if selected {
                    disabled
                } else {
                    Color32::TRANSPARENT
                },
            )
        };
        let paint = move |painter: &egui::Painter, bounds: egui::Rect| {
            let center = bounds.center();
            painter.circle_stroke(center, 9.0, Stroke::new(2.0_f32, outline));
            if dot != Color32::TRANSPARENT {
                painter.circle_filled(center, 5.0, dot);
            }
        };
        let callback = radio.on_changed.clone();
        let value = radio.value.clone();
        let semantics_label = radio.semantics_label.clone();
        GestureButton::new(&radio.binding, egui::RichText::new(""), move || {
            if !selected {
                callback(value.clone());
            }
        })
        .size(Vec2::splat(48.0))
        .enabled(radio.enabled)
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
        .corner_radius(egui::CornerRadius::ZERO)
        .accessibility_role(GestureSemanticRole::RadioButton)
        .accessibility_selected(selected)
        .accessibility_label(semantics_label.unwrap_or_default())
        .paint_content(paint)
        .into_widget()
    }
}
