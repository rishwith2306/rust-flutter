use super::{ChipThemeData, ThemeData};
use crate::{
    egui::{self, FontId, Stroke},
    element::{BuildContext, StatelessWidget, Widget},
    gestures::{GestureBinding, GestureButton, GestureSemanticRole},
};
use std::rc::Rc;

/// A controlled Material choice chip with radio selection semantics.
pub struct ChoiceChip {
    binding: GestureBinding,
    label: String,
    selected: bool,
    enabled: bool,
    on_selected: Rc<dyn Fn(bool)>,
}

impl ChoiceChip {
    pub fn new(
        binding: &GestureBinding,
        label: impl Into<String>,
        selected: bool,
        on_selected: impl Fn(bool) + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            label: label.into(),
            selected,
            enabled: true,
            on_selected: Rc::new(on_selected),
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn into_widget(self) -> Widget {
        Widget::stateless(self)
    }
}

impl StatelessWidget for ChoiceChip {
    fn build(&self, context: &BuildContext) -> Widget {
        let theme =
            ThemeData::of(context).expect("Material choice chip resolves theme during build");
        let scheme = theme.color_scheme;
        let chip_theme = theme
            .extension::<ChipThemeData>()
            .cloned()
            .unwrap_or_default();
        let (fill, foreground, outline) = chip_theme.resolve(scheme, self.selected, self.enabled);
        let stroke = if self.selected {
            Stroke::NONE
        } else {
            Stroke::new(1.0_f32, outline)
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
        GestureButton::new(
            &self.binding,
            egui::RichText::new(label).font(font).color(foreground),
            move || callback(!selected),
        )
        .size(egui::vec2((width + 24.0).max(64.0), 32.0))
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(8))
        .enabled(self.enabled)
        .accessibility_role(GestureSemanticRole::RadioButton)
        .accessibility_label(self.label.clone())
        .accessibility_selected(self.selected)
        .into_widget()
    }
}
