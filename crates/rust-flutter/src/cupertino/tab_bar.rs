use super::CupertinoThemeData;
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, Align2, Color32, FontId, Stroke, Vec2},
    element::{BuildContext, FlexItem, FlexStyle, StatelessWidget, Widget},
    gestures::{GestureBinding, GestureButton, GestureSemanticRole},
    layout::{Axis, BoxConstraints, CrossAxisAlignment},
};
use std::rc::Rc;

/// Text-glyph destination for [`CupertinoTabBar`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CupertinoTabDestination {
    pub icon: String,
    pub selected_icon: Option<String>,
    pub label: String,
}

impl CupertinoTabDestination {
    pub fn new(icon: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            selected_icon: None,
            label: label.into(),
        }
    }

    pub fn selected_icon(mut self, icon: impl Into<String>) -> Self {
        self.selected_icon = Some(icon.into());
        self
    }
}

/// Controlled iOS-style bottom navigation bar. The owner supplies the current
/// index again after `on_tap` runs.
pub struct CupertinoTabBar {
    binding: GestureBinding,
    items: Vec<CupertinoTabDestination>,
    current_index: usize,
    height: f32,
    icon_size: f32,
    background_color: Option<Color32>,
    active_color: Option<Color32>,
    inactive_color: Option<Color32>,
    on_tap: Rc<dyn Fn(usize)>,
}

impl CupertinoTabBar {
    pub fn new(
        binding: &GestureBinding,
        items: Vec<CupertinoTabDestination>,
        current_index: usize,
        on_tap: impl Fn(usize) + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            items,
            current_index,
            height: 50.0,
            icon_size: 19.0,
            background_color: None,
            active_color: None,
            inactive_color: None,
            on_tap: Rc::new(on_tap),
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = size;
        self
    }

    pub fn background_color(mut self, color: Color32) -> Self {
        self.background_color = Some(color);
        self
    }

    pub fn active_color(mut self, color: Color32) -> Self {
        self.active_color = Some(color);
        self
    }

    pub fn inactive_color(mut self, color: Color32) -> Self {
        self.inactive_color = Some(color);
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if !(2..=5).contains(&self.items.len())
            || self.current_index >= self.items.len()
            || !self.height.is_finite()
            || self.height < self.icon_size + 14.0
            || !self.icon_size.is_finite()
            || self.icon_size <= 0.0
        {
            return Err("invalid Cupertino tab count, current index, or dimensions");
        }
        Ok(Widget::stateless(CupertinoTabBarView(self)))
    }
}

struct CupertinoTabBarView(CupertinoTabBar);

impl StatelessWidget for CupertinoTabBarView {
    fn build(&self, context: &BuildContext) -> Widget {
        let bar = &self.0;
        let theme = CupertinoThemeData::of(context).unwrap_or_default();
        let background = bar
            .background_color
            .unwrap_or(theme.scaffold_background_color);
        let active = bar.active_color.unwrap_or(theme.primary_color);
        let inactive = bar.inactive_color.unwrap_or(theme.disabled_color);
        let bar_height = bar.height;
        let icon_size = bar.icon_size;
        let mut items = Vec::with_capacity(bar.items.len());
        for (index, item) in bar.items.iter().enumerate() {
            let selected = index == bar.current_index;
            let icon = if selected {
                item.selected_icon
                    .as_deref()
                    .unwrap_or(&item.icon)
                    .to_owned()
            } else {
                item.icon.clone()
            };
            let label = item.label.clone();
            let color = if selected { active } else { inactive };
            let callback = bar.on_tap.clone();
            let button = GestureButton::new(&bar.binding, egui::RichText::new(""), move || {
                callback(index)
            })
            .size(Vec2::new(72.0, bar_height - 1.0))
            .accessibility_role(GestureSemanticRole::Tab)
            .accessibility_label(item.label.clone())
            .accessibility_selected(selected)
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .corner_radius(egui::CornerRadius::ZERO)
            .paint_content(move |painter, bounds| {
                painter.text(
                    egui::pos2(bounds.center().x, bounds.top() + 3.0),
                    Align2::CENTER_TOP,
                    &icon,
                    FontId::proportional(icon_size),
                    color,
                );
                painter.text(
                    egui::pos2(bounds.center().x, bounds.bottom() - 4.0),
                    Align2::CENTER_BOTTOM,
                    &label,
                    FontId::proportional(10.0),
                    color,
                );
            });
            items.push(FlexItem::expanded(button.into_widget(), 1));
        }
        let mut row_style = FlexStyle::new(Axis::Horizontal);
        row_style.cross_alignment = CrossAxisAlignment::Stretch;
        let row = Widget::flex(row_style, items);
        let separator = Widget::leaf(Vec2::new(0.0, 1.0), move |ui| {
            let rect = ui.max_rect();
            ui.painter().line_segment(
                [rect.left_top(), rect.right_top()],
                Stroke::new(0.5_f32, active.gamma_multiply(0.18)),
            );
        });
        let height = BoxConstraints::tight_for(None, Some(bar_height - 1.0))
            .expect("finite Cupertino tab bar height is validated");
        let mut column_style = FlexStyle::new(Axis::Vertical);
        column_style.cross_alignment = CrossAxisAlignment::Stretch;
        DecoratedBox::new(BoxDecoration::solid(background)).into_widget(Widget::flex(
            column_style,
            vec![
                FlexItem::fixed(separator),
                FlexItem::expanded(Widget::constrained(height, row), 1),
            ],
        ))
    }
}
