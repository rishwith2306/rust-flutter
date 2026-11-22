use super::{Brightness, CupertinoThemeData};
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, Color32, Stroke, Vec2},
    element::{FlexItem, FlexStyle, Widget},
    gestures::{GestureBinding, GestureButton},
    layout::{Axis, BoxConstraints, CrossAxisAlignment, Insets},
    text::{DefaultTextStyle, TextStyle},
};
use std::rc::Rc;

/// Cupertino alert panel. Place it in an application-owned route or overlay to
/// present it modally; this widget builds the dialog surface and content.
pub struct CupertinoAlertDialog {
    title: Widget,
    content: Option<Widget>,
    actions: Vec<Widget>,
    width: f32,
    theme: CupertinoThemeData,
}

impl CupertinoAlertDialog {
    pub fn new(title: Widget) -> Self {
        Self {
            title,
            content: None,
            actions: Vec::new(),
            width: 360.0,
            theme: CupertinoThemeData::default(),
        }
    }

    pub fn content(mut self, content: Widget) -> Self {
        self.content = Some(content);
        self
    }

    /// Actions are laid out top to bottom with thin separators.
    pub fn actions(mut self, actions: Vec<Widget>) -> Self {
        self.actions = actions;
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
            return Err("Cupertino alert dialog width must be positive and finite");
        }
        let mut children = vec![FlexItem::fixed(Widget::padding(
            Insets {
                left: 20.0,
                top: 18.0,
                right: 20.0,
                bottom: 8.0,
            },
            DefaultTextStyle {
                style: TextStyle {
                    font_size: Some(17.0),
                    color: Some(self.theme.text_color),
                    ..Default::default()
                },
                ..Default::default()
            }
            .provide(self.title),
        ))];
        if let Some(content) = self.content {
            children.push(FlexItem::fixed(Widget::padding(
                Insets {
                    left: 20.0,
                    top: 0.0,
                    right: 20.0,
                    bottom: 16.0,
                },
                DefaultTextStyle {
                    style: TextStyle {
                        font_size: Some(13.0),
                        color: Some(self.theme.text_color.gamma_multiply(0.82)),
                        ..Default::default()
                    },
                    ..Default::default()
                }
                .provide(content),
            )));
        }
        if !self.actions.is_empty() {
            let divider = |color: Color32| {
                FlexItem::fixed(Widget::leaf(Vec2::new(0.0, 0.5), move |ui| {
                    ui.painter().line_segment(
                        [ui.max_rect().left_center(), ui.max_rect().right_center()],
                        Stroke::new(0.5_f32, color),
                    );
                }))
            };
            children.push(divider(self.theme.text_color.gamma_multiply(0.18)));
            let mut actions = self.actions.into_iter();
            if let Some(first) = actions.next() {
                children.push(FlexItem::fixed(first));
            }
            for action in actions {
                children.push(divider(self.theme.text_color.gamma_multiply(0.18)));
                children.push(FlexItem::fixed(action));
            }
        }
        let mut style = FlexStyle::new(Axis::Vertical);
        style.cross_alignment = CrossAxisAlignment::Stretch;
        let background = match self.theme.brightness {
            Brightness::Light => Color32::from_rgb(250, 250, 250),
            Brightness::Dark => Color32::from_rgb(44, 44, 46),
        };
        let panel = DecoratedBox::new(BoxDecoration {
            color: background,
            border: Stroke::new(0.5_f32, self.theme.text_color.gamma_multiply(0.12)),
            corner_radius: egui::CornerRadius::same(14),
        })
        .into_widget(Widget::flex(style, children));
        let constraints = BoxConstraints::loose(Vec2::new(self.width, 1200.0))
            .expect("validated finite alert dialog width");
        Ok(Widget::constrained(constraints, panel))
    }
}

/// A selectable action for a [`CupertinoAlertDialog`].
pub struct CupertinoDialogAction {
    binding: GestureBinding,
    label: String,
    on_pressed: Rc<dyn Fn()>,
    default_action: bool,
    destructive_action: bool,
    theme: CupertinoThemeData,
    width: f32,
}

impl CupertinoDialogAction {
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
            width: 320.0,
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
            return Err("Cupertino dialog action width must be positive and finite");
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
            .size(Vec2::new(self.width, 44.0))
            .fill(Color32::TRANSPARENT)
            .corner_radius(egui::CornerRadius::ZERO)
            .into_widget())
    }
}
