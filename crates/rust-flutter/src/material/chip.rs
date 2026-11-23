use super::{ChipThemeData, ThemeData};
use crate::{
    egui::{self, Align2, FontId, Stroke},
    element::{BuildContext, FlexItem, FlexStyle, StatelessWidget, Tooltip, Widget},
    gestures::{GestureBinding, GestureButton, GestureSemanticRole},
    layout::{Axis, CrossAxisAlignment},
};
use std::rc::Rc;

/// A compact Material label chip with an optional independent delete action.
pub struct Chip {
    label: String,
    enabled: bool,
    tooltip: Option<String>,
    delete_tooltip: Option<String>,
    on_deleted: Option<(GestureBinding, Rc<dyn Fn()>)>,
}

impl Chip {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
            tooltip: None,
            delete_tooltip: None,
            on_deleted: None,
        }
    }

    pub fn on_deleted(mut self, binding: &GestureBinding, callback: impl Fn() + 'static) -> Self {
        self.on_deleted = Some((binding.clone(), Rc::new(callback)));
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

impl StatelessWidget for Chip {
    fn build(&self, context: &BuildContext) -> Widget {
        let theme = ThemeData::of(context).expect("Material chip resolves theme during build");
        let scheme = theme.color_scheme;
        let chip_theme = theme
            .extension::<ChipThemeData>()
            .cloned()
            .unwrap_or_default();
        let (fill, foreground, outline) = chip_theme.resolve(scheme, false, self.enabled);
        let font = FontId::proportional(14.0);
        let width = context.egui().fonts_mut(|fonts| {
            fonts
                .layout_no_wrap(self.label.clone(), font.clone(), foreground)
                .size()
                .x
        });
        let label = self.label.clone();
        let chip = Widget::leaf(egui::vec2(width + 24.0, 32.0), move |ui| {
            let rect = ui.max_rect();
            ui.painter()
                .rect_filled(rect, egui::CornerRadius::same(8), fill);
            ui.painter().rect_stroke(
                rect,
                egui::CornerRadius::same(8),
                Stroke::new(1.0_f32, outline),
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                label.clone(),
                font.clone(),
                foreground,
            );
        });
        let row = if let Some((binding, callback)) = &self.on_deleted {
            let callback = callback.clone();
            let label = self.label.clone();
            let delete = GestureButton::new(binding, "×", move || callback())
                .size(egui::vec2(28.0, 32.0))
                .fill(fill)
                .stroke(Stroke::new(1.0_f32, outline))
                .corner_radius(egui::CornerRadius::same(8))
                .enabled(self.enabled)
                .accessibility_role(GestureSemanticRole::Button)
                .accessibility_label(format!("Remove {label}"))
                .into_widget();
            let mut style = FlexStyle::new(Axis::Horizontal);
            style.cross_alignment = CrossAxisAlignment::Center;
            style.spacing = 0.0;
            let delete = match &self.delete_tooltip {
                Some(message) => Tooltip::new(delete, message.clone()).into_widget(),
                None => delete,
            };
            Widget::flex(style, vec![FlexItem::fixed(chip), FlexItem::fixed(delete)])
        } else {
            chip
        };
        match &self.tooltip {
            Some(message) => Tooltip::new(row, message.clone()).into_widget(),
            None => row,
        }
    }
}
