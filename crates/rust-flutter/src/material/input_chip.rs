use super::{ChipThemeData, ThemeData};
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
    on_pressed: Option<Rc<dyn Fn()>>,
    on_deleted: Option<Rc<dyn Fn()>>,
    delete_tooltip: Option<String>,
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
            on_pressed: None,
            on_deleted: None,
            delete_tooltip: None,
        }
    }

    pub fn on_selected(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.on_selected = Some(Rc::new(callback));
        self.on_pressed = None;
        self
    }

    /// Make the label area activate an action instead of changing selection.
    pub fn on_pressed(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_pressed = Some(Rc::new(callback));
        self.on_selected = None;
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

    pub fn delete_tooltip(mut self, message: impl Into<String>) -> Self {
        self.delete_tooltip = Some(message.into());
        self
    }

    pub fn into_widget(self) -> Widget {
        Widget::stateless(self)
    }
}

impl StatelessWidget for InputChip {
    fn build(&self, context: &BuildContext) -> Widget {
        let theme =
            ThemeData::of(context).expect("Material input chip resolves theme during build");
        let scheme = theme.color_scheme;
        let chip_theme = theme
            .extension::<ChipThemeData>()
            .cloned()
            .unwrap_or_default();
        let (chip_fill, foreground, outline) =
            chip_theme.resolve(scheme, self.selected, self.enabled);
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
        let on_selected = self.on_selected.clone();
        let on_pressed = self.on_pressed.clone();
        let selected = self.selected;
        let stroke = if self.selected {
            Stroke::NONE
        } else {
            Stroke::new(1.0_f32, outline)
        };
        let main = GestureButton::new(
            &self.binding,
            egui::RichText::new(label).font(font).color(foreground),
            move || {
                if let Some(callback) = &on_selected {
                    callback(!selected);
                } else if let Some(callback) = &on_pressed {
                    callback();
                }
            },
        )
        .size(egui::vec2((width + 24.0).max(64.0), 32.0))
        .fill(chip_fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(8))
        .enabled(self.enabled && (self.on_selected.is_some() || self.on_pressed.is_some()));
        let main = if self.on_selected.is_some() {
            main.accessibility_role(GestureSemanticRole::CheckBox)
                .accessibility_label(self.label.clone())
                .accessibility_toggled(self.selected)
        } else if self.on_pressed.is_some() {
            main.accessibility_role(GestureSemanticRole::Button)
                .accessibility_label(self.label.clone())
        } else {
            main.accessibility_label(self.label.clone())
        }
        .into_widget();

        let mut style = FlexStyle::new(Axis::Horizontal);
        style.cross_alignment = CrossAxisAlignment::Center;
        style.spacing = 0.0;
        let row = if let Some(on_deleted) = &self.on_deleted {
            let on_deleted = on_deleted.clone();
            let remove = GestureButton::new(&self.binding, "×", move || on_deleted())
                .size(egui::vec2(28.0, 32.0))
                .fill(chip_fill)
                .stroke(Stroke::new(1.0_f32, outline))
                .corner_radius(egui::CornerRadius::same(8))
                .enabled(self.enabled)
                .accessibility_role(GestureSemanticRole::Button)
                .accessibility_label(format!("Remove {}", self.label))
                .into_widget();
            let remove = match &self.delete_tooltip {
                Some(message) => Tooltip::new(remove, message.clone()).into_widget(),
                None => remove,
            };
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
