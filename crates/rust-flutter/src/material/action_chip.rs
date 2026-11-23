use super::{ChipThemeData, ThemeData};
use crate::{
    egui::{self, FontId, Stroke},
    element::{BuildContext, StatelessWidget, Tooltip, Widget},
    gestures::{GestureBinding, GestureButton, GestureSemanticRole},
};
use std::rc::Rc;

/// A controlled Material action chip with optional pointer-hover tooltip.
pub struct ActionChip {
    binding: GestureBinding,
    label: String,
    icon: Option<String>,
    enabled: bool,
    tooltip: Option<String>,
    on_pressed: Rc<dyn Fn()>,
}

impl ActionChip {
    pub fn new(
        binding: &GestureBinding,
        label: impl Into<String>,
        on_pressed: impl Fn() + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            label: label.into(),
            icon: None,
            enabled: true,
            tooltip: None,
            on_pressed: Rc::new(on_pressed),
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Adds a leading text glyph, for example an icon-font character, to the
    /// action. Flutter uses an icon to distinguish an Assist chip from a
    /// Suggestion chip; the icon is decorative and the accessible name remains
    /// the label.
    pub fn icon(mut self, glyph: impl Into<String>) -> Self {
        self.icon = Some(glyph.into());
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

impl StatelessWidget for ActionChip {
    fn build(&self, context: &BuildContext) -> Widget {
        let theme =
            ThemeData::of(context).expect("Material action chip resolves theme during build");
        let scheme = theme.color_scheme;
        let chip_theme = theme
            .extension::<ChipThemeData>()
            .cloned()
            .unwrap_or_default();
        let (fill, foreground, outline) = chip_theme.resolve(scheme, false, self.enabled);
        let label = match &self.icon {
            Some(icon) => format!("{icon}  {}", self.label),
            None => self.label.clone(),
        };
        let font = FontId::proportional(14.0);
        let width = context.egui().fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(label.clone(), font.clone(), foreground)
                .size()
                .x
        });
        let callback = self.on_pressed.clone();
        let chip = GestureButton::new(
            &self.binding,
            egui::RichText::new(label).font(font).color(foreground),
            move || callback(),
        )
        .size(egui::vec2((width + 24.0).max(64.0), 32.0))
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, outline))
        .corner_radius(egui::CornerRadius::same(8))
        .enabled(self.enabled)
        .accessibility_role(GestureSemanticRole::Button)
        .accessibility_label(self.label.clone())
        .into_widget();
        match &self.tooltip {
            Some(message) => Tooltip::new(chip, message.clone()).into_widget(),
            None => chip,
        }
    }
}
