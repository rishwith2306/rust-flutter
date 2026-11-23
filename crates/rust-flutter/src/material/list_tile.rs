use super::ThemeData;
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    element::{FlexItem, FlexStyle, Semantics, Widget},
    gestures::{GestureBinding, GestureDetector, GestureTargetConfig, TapCallbacks, TapConfig},
    layout::{Axis, BoxConstraints, CrossAxisAlignment, HitTestBehavior, Insets},
    text::{DefaultTextStyle, TextStyle},
};
use std::rc::Rc;

/// A single fixed-height Material row with optional leading, subtitle and
/// trailing widgets.
pub struct ListTile {
    binding: GestureBinding,
    leading: Option<Widget>,
    title: Widget,
    subtitle: Option<Widget>,
    trailing: Option<Widget>,
    enabled: bool,
    selected: bool,
    dense: bool,
    is_three_line: bool,
    content_padding: Insets,
    background_color: Option<crate::egui::Color32>,
    theme: ThemeData,
    on_tap: Option<Rc<dyn Fn()>>,
    semantics_label: Option<String>,
}

impl ListTile {
    pub fn new(binding: &GestureBinding, title: Widget) -> Self {
        Self {
            binding: binding.clone(),
            leading: None,
            title,
            subtitle: None,
            trailing: None,
            enabled: true,
            selected: false,
            dense: false,
            is_three_line: false,
            content_padding: Insets {
                left: 16.0,
                right: 16.0,
                ..Default::default()
            },
            background_color: None,
            theme: ThemeData::default(),
            on_tap: None,
            semantics_label: None,
        }
    }

    pub fn leading(mut self, widget: Widget) -> Self {
        self.leading = Some(widget);
        self
    }

    pub fn subtitle(mut self, widget: Widget) -> Self {
        self.subtitle = Some(widget);
        self
    }

    pub fn trailing(mut self, widget: Widget) -> Self {
        self.trailing = Some(widget);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn dense(mut self, dense: bool) -> Self {
        self.dense = dense;
        self
    }

    pub fn is_three_line(mut self, is_three_line: bool) -> Self {
        self.is_three_line = is_three_line;
        self
    }

    pub fn content_padding(mut self, padding: Insets) -> Self {
        self.content_padding = padding;
        self
    }

    pub fn background_color(mut self, color: crate::egui::Color32) -> Self {
        self.background_color = Some(color);
        self
    }

    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.theme = theme;
        self
    }

    pub fn on_tap(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_tap = Some(Rc::new(callback));
        self
    }

    /// Sets the screen reader name when child content does not provide one.
    pub fn semantics_label(mut self, label: impl Into<String>) -> Self {
        self.semantics_label = Some(label.into());
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if [
            self.content_padding.left,
            self.content_padding.top,
            self.content_padding.right,
            self.content_padding.bottom,
        ]
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err("Material list tile padding must be finite and nonnegative");
        }
        let colors = self.theme.color_scheme;
        let has_subtitle = self.subtitle.is_some();
        let text_color = if !self.enabled {
            colors.on_surface.gamma_multiply(0.38)
        } else if self.selected {
            colors.primary
        } else {
            colors.on_surface
        };
        let title = DefaultTextStyle {
            style: TextStyle {
                font_size: Some(if self.dense { 14.0 } else { 16.0 }),
                color: Some(text_color),
                ..Default::default()
            },
            ..Default::default()
        }
        .provide(self.title);
        let mut title_items = vec![FlexItem::fixed(title)];
        if let Some(subtitle) = self.subtitle {
            title_items.push(FlexItem::fixed(Widget::padding(
                Insets {
                    top: 2.0,
                    ..Default::default()
                },
                DefaultTextStyle {
                    style: TextStyle {
                        font_size: Some(if self.dense { 12.0 } else { 14.0 }),
                        color: Some(if self.enabled {
                            colors.on_surface_variant
                        } else {
                            colors.on_surface.gamma_multiply(0.38)
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                }
                .provide(subtitle),
            )));
        }
        let mut title_style = FlexStyle::new(Axis::Vertical);
        title_style.cross_alignment = CrossAxisAlignment::Stretch;
        let title_block = Widget::flex(title_style, title_items);
        let mut row_items = Vec::with_capacity(5);
        if let Some(leading) = self.leading {
            let constraints = BoxConstraints::tight_for(Some(40.0), None)
                .expect("finite list tile leading width is valid");
            row_items.push(FlexItem::fixed(Widget::constrained(
                constraints,
                DefaultTextStyle {
                    style: TextStyle {
                        color: Some(if self.enabled {
                            colors.on_surface_variant
                        } else {
                            colors.on_surface.gamma_multiply(0.38)
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                }
                .provide(leading),
            )));
            row_items.push(FlexItem::fixed(Widget::leaf(
                crate::egui::vec2(16.0, 0.0),
                |_| {},
            )));
        }
        row_items.push(FlexItem::expanded(title_block, 1));
        if let Some(trailing) = self.trailing {
            row_items.push(FlexItem::fixed(Widget::leaf(
                crate::egui::vec2(16.0, 0.0),
                |_| {},
            )));
            row_items.push(FlexItem::fixed(
                DefaultTextStyle {
                    style: TextStyle {
                        color: Some(if self.enabled {
                            colors.on_surface_variant
                        } else {
                            colors.on_surface.gamma_multiply(0.38)
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                }
                .provide(trailing),
            ));
        }
        let mut row_style = FlexStyle::new(Axis::Horizontal);
        row_style.cross_alignment = CrossAxisAlignment::Center;
        let row = Widget::padding(self.content_padding, Widget::flex(row_style, row_items));
        let height = match (self.dense, has_subtitle, self.is_three_line) {
            (false, false, _) => 56.0,
            (true, false, _) => 48.0,
            (false, true, true) => 88.0,
            (true, true, true) => 76.0,
            (false, true, false) => 72.0,
            (true, true, false) => 64.0,
        };
        let height = BoxConstraints::tight_for(None, Some(height))
            .expect("finite list tile height is valid");
        let background = self.background_color.unwrap_or(if self.selected {
            colors.secondary_container
        } else {
            crate::egui::Color32::TRANSPARENT
        });
        let tile = DecoratedBox::new(BoxDecoration::solid(background))
            .into_widget(Widget::constrained(height, row));
        let role = if self.on_tap.is_some() {
            crate::egui::accesskit::Role::Button
        } else {
            crate::egui::accesskit::Role::ListItem
        };
        let mut semantics = Semantics::new(role)
            .enabled(self.enabled)
            .selected(self.selected);
        if let Some(label) = self.semantics_label {
            semantics = semantics.label(label);
        }
        if self.enabled
            && let Some(on_tap) = self.on_tap
        {
            let semantic_callback = on_tap.clone();
            semantics = semantics.on_click(move || semantic_callback());
            let callback = on_tap;
            let visual = GestureDetector::new(
                &self.binding,
                GestureTargetConfig {
                    tap: Some((
                        TapConfig::default(),
                        TapCallbacks {
                            on_tap: Some(Rc::new(move |_| callback())),
                            ..Default::default()
                        },
                    )),
                    ..Default::default()
                },
            )
            .map_err(|_| "Material list tile gesture binding is unavailable")?
            .behavior(HitTestBehavior::Opaque)
            .into_widget(tile);
            return Ok(semantics.into_widget(visual));
        }
        Ok(semantics.into_widget(tile))
    }
}
