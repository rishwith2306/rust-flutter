use super::ThemeData;
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, Color32, Stroke},
    element::{FlexItem, FlexStyle, Semantics, Widget},
    layout::{Axis, CrossAxisAlignment, Insets},
    text::{DefaultTextStyle, Text, TextStyle},
};
use std::rc::Rc;

/// A Material feedback surface suitable for placement over a `Scaffold` body.
/// `ScaffoldMessengerHost` can manage its display duration.
pub struct SnackBar {
    message: String,
    action: Option<SnackBarAction>,
    background: Color32,
    foreground: Color32,
}

#[derive(Clone)]
/// An optional action displayed alongside a [`SnackBar`] message.
pub struct SnackBarAction {
    label: String,
    on_pressed: Rc<dyn Fn()>,
}

impl SnackBarAction {
    pub fn new(label: impl Into<String>, on_pressed: impl Fn() + 'static) -> Self {
        Self {
            label: label.into(),
            on_pressed: Rc::new(on_pressed),
        }
    }
}

impl Clone for SnackBar {
    fn clone(&self) -> Self {
        Self {
            message: self.message.clone(),
            action: self.action.clone(),
            background: self.background,
            foreground: self.foreground,
        }
    }
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

    pub fn action(mut self, action: SnackBarAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.background = theme.color_scheme.inverse_surface;
        self.foreground = theme.color_scheme.on_inverse_surface;
        self
    }

    pub fn into_widget(&self) -> Widget {
        let message = self.message.clone();
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
        if let Some(action) = self.action.clone() {
            let label = action.label;
            let color = self.foreground;
            let on_pressed = action.on_pressed;
            row.push(FlexItem::fixed(Widget::leaf(
                egui::vec2(72.0, 40.0),
                move |ui| {
                    let button = egui::Button::new(egui::RichText::new(&label).color(color))
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE);
                    if ui.add(button).clicked() {
                        on_pressed();
                    }
                },
            )));
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
