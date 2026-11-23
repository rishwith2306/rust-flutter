use super::ThemeData;
use crate::{
    egui::{self, FontId, Stroke},
    element::{BuildContext, FlexItem, FlexStyle, StatelessWidget, Tooltip, Widget},
    gestures::{GestureBinding, GestureButton, GestureSemanticRole},
    layout::{Axis, CrossAxisAlignment},
};
use std::rc::Rc;

/// A Material input chip with optional selection and deletion actions.
pub struct InputChip {
    binding: GestureBinding,
    label: String,
    selected: bool,
    enabled: bool,
    tooltip: Option<String>,
    on_selected: Option<Rc<dyn Fn(bool)>>,
    on_deleted: Option<Rc<dyn Fn()>>,
}

impl InputChip {
    pub fn new(binding: &GestureBinding, label: impl Into<String>, selected: bool) -> Self {
        Self {
            binding: binding.clone(),
            label: label.into(),
            selected,
            enabled: true,
            tooltip: None,
            on_selected: None,
            on_deleted: None,
        }
    }

    pub fn on_selected(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.on_selected = Some(Rc::new(callback));
        self
    }

    pub fn on_deleted(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_deleted = Some(Rc::new(callback));
        self
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

impl StatelessWidget for InputChip {
    fn build(&self, context: &BuildContext) -> Widget {
        let scheme = ThemeData::of(context)
            .expect("Material input chip resolves theme during build")
            .color_scheme;
        let (fill, foreground) = if self.selected {
            (scheme.secondary_container, scheme.on_secondary_container)
        } else {
            (scheme.surface_container_low, scheme.on_surface_variant)
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
        let selected = self.selected;
        let chip_fill = if self.enabled {
            fill
        } else {
            fill.gamma_multiply(0.5)
        };
        let main = GestureButton::new(
            &self.binding,
            egui::RichText::new(label).font(font).color(foreground),
            move || {
                if let Some(callback) = &callback {
                    callback(!selected);
                }
            },
        )
        .size(egui::vec2((width + 24.0).max(64.0), 32.0))
        .fill(chip_fill)
        .stroke(Stroke::new(1.0_f32, scheme.outline))
        .corner_radius(egui::CornerRadius::same(8))
        .enabled(self.enabled && self.on_selected.is_some())
        .accessibility_role(GestureSemanticRole::CheckBox)
        .accessibility_label(self.label.clone())
        .accessibility_toggled(self.selected)
        .into_widget();

        let mut style = FlexStyle::new(Axis::Horizontal);
        style.cross_alignment = CrossAxisAlignment::Center;
        style.spacing = 0.0;
        let row = if let Some(on_deleted) = &self.on_deleted {
            let on_deleted = on_deleted.clone();
            let remove = GestureButton::new(&self.binding, "×", move || on_deleted())
                .size(egui::vec2(28.0, 32.0))
                .fill(chip_fill)
                .stroke(Stroke::new(1.0_f32, scheme.outline))
                .corner_radius(egui::CornerRadius::same(8))
                .enabled(self.enabled)
                .accessibility_role(GestureSemanticRole::Button)
                .accessibility_label(format!("Remove {}", self.label))
                .into_widget();
            Widget::flex(style, vec![FlexItem::fixed(main), FlexItem::fixed(remove)])
        } else {
            main
        };
        match &self.tooltip {
            Some(message) => Tooltip::new(row, message.clone()).into_widget(),
            None => row,
        }
    }
}
