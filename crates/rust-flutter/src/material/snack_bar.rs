use super::ThemeData;
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, Color32, Stroke},
    element::{FlexItem, FlexStyle, Semantics, Widget},
    layout::{Axis, CrossAxisAlignment, Insets},
    text::{DefaultTextStyle, Text, TextStyle},
};

/// A Material feedback surface suitable for placement over a `Scaffold` body.
/// Display duration and queue management remain application-owned.
pub struct SnackBar {
    message: String,
    action: Option<Widget>,
    background: Color32,
    foreground: Color32,
}

impl SnackBar {
    pub fn new(message: impl Into<String>) -> Self {
        let colors = ThemeData::default().color_scheme;
        Self {
            message: message.into(),
            action: None,
            background: colors.inverse_surface,
            foreground: colors.on_inverse_surface,
        }
    }

    pub fn action(mut self, action: Widget) -> Self {
        self.action = Some(action);
        self
    }

    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.background = theme.color_scheme.inverse_surface;
        self.foreground = theme.color_scheme.on_inverse_surface;
        self
    }

    pub fn into_widget(self) -> Widget {
        let message = self.message;
        let mut row = vec![FlexItem::expanded(
            DefaultTextStyle {
                style: TextStyle {
                    color: Some(self.foreground),
                    font_size: Some(14.0),
                    ..Default::default()
                },
                ..Default::default()
            }
            .provide(Text::new(message.clone()).into_widget()),
            1,
        )];
        if let Some(action) = self.action {
            row.push(FlexItem::fixed(action));
        }
        let mut style = FlexStyle::new(Axis::Horizontal);
        style.spacing = 16.0;
        style.cross_alignment = CrossAxisAlignment::Center;
        let content = Widget::padding(Insets::all(16.0), Widget::flex(style, row));
        let surface = DecoratedBox::new(BoxDecoration {
            color: self.background,
            border: Stroke::NONE,
            corner_radius: egui::CornerRadius::same(4),
        })
        .into_widget(content);
        Semantics::new(egui::accesskit::Role::Alert)
            .label(message)
            .into_widget(Widget::padding(
                Insets {
                    left: 16.0,
                    right: 16.0,
                    bottom: 16.0,
                    ..Default::default()
                },
                surface,
            ))
    }
}
