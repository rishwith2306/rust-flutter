use super::{Brightness, CupertinoThemeData};
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, Color32, Stroke, Vec2},
    element::{FlexItem, FlexStyle, Widget},
    gestures::{GestureBinding, GestureButton},
    layout::{Axis, BoxConstraints, CrossAxisAlignment, Insets},
};
use std::rc::Rc;

/// An iOS-style choice panel. Place it in an application-owned modal popup to
/// slide it from the bottom of the screen.
pub struct CupertinoActionSheet {
    title: Option<Widget>,
    message: Option<Widget>,
    actions: Vec<Widget>,
    cancel_button: Option<Widget>,
    width: f32,
    theme: CupertinoThemeData,
}

impl CupertinoActionSheet {
    pub fn new() -> Self {
        Self {
            title: None,
            message: None,
            actions: Vec::new(),
            cancel_button: None,
            width: 420.0,
            theme: CupertinoThemeData::default(),
        }
    }

    pub fn title(mut self, title: Widget) -> Self {
        self.title = Some(title);
        self
    }

    pub fn message(mut self, message: Widget) -> Self {
        self.message = Some(message);
        self
    }

    pub fn actions(mut self, actions: Vec<Widget>) -> Self {
        self.actions = actions;
        self
    }

    pub fn cancel_button(mut self, button: Widget) -> Self {
        self.cancel_button = Some(button);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn theme(mut self, theme: CupertinoThemeData) -> Self {
        self.theme = theme;
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if !self.width.is_finite() || self.width <= 0.0 {
            return Err("Cupertino action sheet width must be positive and finite");
        }
        let mut parts = Vec::new();
        if self.title.is_some() || self.message.is_some() {
            let mut header = Vec::new();
            if let Some(title) = self.title {
                header.push(FlexItem::fixed(Widget::padding(
                    Insets {
                        left: 20.0,
                        top: 18.0,
                        right: 20.0,
                        bottom: if self.message.is_some() { 4.0 } else { 16.0 },
                    },
                    title,
                )));
            }
            if let Some(message) = self.message {
                header.push(FlexItem::fixed(Widget::padding(
                    Insets {
                        left: 20.0,
                        top: 0.0,
                        right: 20.0,
                        bottom: 16.0,
                    },
                    message,
                )));
            }
            parts.push(sheet_group(header, self.theme, self.width, 14));
        }
        if !self.actions.is_empty() {
            if !parts.is_empty() {
                parts.push(FlexItem::fixed(Widget::leaf(Vec2::new(0.0, 8.0), |_| {})));
            }
            let mut actions = Vec::with_capacity(self.actions.len() * 2 - 1);
            for (index, action) in self.actions.into_iter().enumerate() {
                if index > 0 {
                    actions.push(sheet_divider(self.theme));
                }
                actions.push(FlexItem::fixed(action));
            }
            parts.push(sheet_group(actions, self.theme, self.width, 14));
        }
        if let Some(cancel_button) = self.cancel_button {
            if !parts.is_empty() {
                parts.push(FlexItem::fixed(Widget::leaf(Vec2::new(0.0, 8.0), |_| {})));
            }
            parts.push(sheet_group(
                vec![FlexItem::fixed(cancel_button)],
                self.theme,
                self.width,
                14,
            ));
        }
        let mut column = FlexStyle::new(Axis::Vertical);
        column.cross_alignment = CrossAxisAlignment::Stretch;
        let content = Widget::flex(column, parts);
        Ok(Widget::constrained(
            BoxConstraints::loose(Vec2::new(self.width, 1200.0))
                .expect("validated finite action sheet width"),
            content,
        ))
    }
}

impl Default for CupertinoActionSheet {
    fn default() -> Self {
        Self::new()
    }
}

fn sheet_group(
    children: Vec<FlexItem>,
    theme: CupertinoThemeData,
    width: f32,
    radius: u8,
) -> FlexItem {
    let mut style = FlexStyle::new(Axis::Vertical);
    style.cross_alignment = CrossAxisAlignment::Stretch;
    let color = match theme.brightness {
        Brightness::Light => Color32::from_rgb(250, 250, 250),
        Brightness::Dark => Color32::from_rgb(44, 44, 46),
    };
    let group = DecoratedBox::new(BoxDecoration {
        color,
        border: Stroke::new(0.5_f32, theme.text_color.gamma_multiply(0.1)),
        corner_radius: egui::CornerRadius::same(radius),
    })
    .into_widget(Widget::flex(style, children));
    FlexItem::fixed(Widget::constrained(
        BoxConstraints::loose(Vec2::new(width, 1200.0))
            .expect("validated finite action sheet width"),
        group,
    ))
}

fn sheet_divider(theme: CupertinoThemeData) -> FlexItem {
    let color = theme.text_color.gamma_multiply(0.18);
    FlexItem::fixed(Widget::leaf(Vec2::new(0.0, 0.5), move |ui| {
        ui.painter().line_segment(
            [ui.max_rect().left_center(), ui.max_rect().right_center()],
            Stroke::new(0.5_f32, color),
        );
    }))
}

/// A standard action sheet choice with optional default or destructive styling.
pub struct CupertinoActionSheetAction {
    binding: GestureBinding,
    label: String,
    on_pressed: Rc<dyn Fn()>,
    default_action: bool,
    destructive_action: bool,
    theme: CupertinoThemeData,
    width: f32,
}

impl CupertinoActionSheetAction {
    pub fn new(
        binding: &GestureBinding,
        label: impl Into<String>,
        on_pressed: impl Fn() + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            label: label.into(),
            on_pressed: Rc::new(on_pressed),
            default_action: false,
            destructive_action: false,
            theme: CupertinoThemeData::default(),
            width: 400.0,
        }
    }

    pub fn default_action(mut self, value: bool) -> Self {
        self.default_action = value;
        self
    }

    pub fn destructive_action(mut self, value: bool) -> Self {
        self.destructive_action = value;
        self
    }

    pub fn theme(mut self, theme: CupertinoThemeData) -> Self {
        self.theme = theme;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if !self.width.is_finite() || self.width <= 0.0 {
            return Err("Cupertino action sheet action width must be positive and finite");
        }
        let mut label = egui::RichText::new(self.label).color(if self.destructive_action {
            Color32::from_rgb(255, 59, 48)
        } else {
            self.theme.primary_color
        });
        if self.default_action {
            label = label.strong();
        }
        let callback = self.on_pressed;
        Ok(GestureButton::new(&self.binding, label, move || callback())
            .size(Vec2::new(self.width, 50.0))
            .fill(Color32::TRANSPARENT)
            .corner_radius(egui::CornerRadius::ZERO)
            .into_widget())
    }
}
