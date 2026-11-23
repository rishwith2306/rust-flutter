use super::ThemeData;
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, Color32},
    element::{Semantics, Widget},
    gestures::{GestureBinding, GestureDetector, GestureTargetConfig, TapCallbacks, TapConfig},
    layout::{BoxConstraints, HitTestBehavior},
    text::{DefaultTextStyle, TextStyle},
};
use std::rc::Rc;

/// A Material 3 floating action button. Place it with [`Scaffold`](super::Scaffold)
/// for the usual bottom-end position.
pub struct FloatingActionButton {
    binding: GestureBinding,
    child: Widget,
    on_pressed: Rc<dyn Fn()>,
    background_color: Option<Color32>,
    foreground_color: Option<Color32>,
    theme: ThemeData,
    mini: bool,
    large: bool,
    extended: bool,
    semantics_label: Option<String>,
}

impl FloatingActionButton {
    pub fn new(binding: &GestureBinding, child: Widget, on_pressed: impl Fn() + 'static) -> Self {
        Self {
            binding: binding.clone(),
            child,
            on_pressed: Rc::new(on_pressed),
            background_color: None,
            foreground_color: None,
            theme: ThemeData::default(),
            mini: false,
            large: false,
            extended: false,
            semantics_label: None,
        }
    }

    pub fn mini(mut self, mini: bool) -> Self {
        self.mini = mini;
        self
    }

    pub fn large(mut self, large: bool) -> Self {
        self.large = large;
        self
    }

    /// Use the extended, label-bearing pill shape and intrinsic child width.
    pub fn extended(mut self) -> Self {
        self.extended = true;
        self
    }

    /// Sets an accessible name, especially useful for icon-only actions.
    pub fn semantics_label(mut self, label: impl Into<String>) -> Self {
        self.semantics_label = Some(label.into());
        self
    }

    pub fn background_color(mut self, color: Color32) -> Self {
        self.background_color = Some(color);
        self
    }

    pub fn foreground_color(mut self, color: Color32) -> Self {
        self.foreground_color = Some(color);
        self
    }

    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.theme = theme;
        self
    }

    pub fn into_widget(self) -> Widget {
        let colors = self.theme.color_scheme;
        let background = self.background_color.unwrap_or(colors.secondary_container);
        let foreground = self
            .foreground_color
            .unwrap_or(colors.on_secondary_container);
        let (width, height, radius) = if self.extended {
            (None, Some(56.0), 28)
        } else if self.large {
            (Some(96.0), Some(96.0), 28)
        } else if self.mini {
            (Some(40.0), Some(40.0), 20)
        } else {
            (Some(56.0), Some(56.0), 28)
        };
        let content = DefaultTextStyle {
            style: TextStyle {
                color: Some(foreground),
                ..Default::default()
            },
            ..Default::default()
        }
        .provide(self.child);
        let content = if self.extended {
            Widget::padding(
                crate::layout::Insets {
                    left: 20.0,
                    right: 20.0,
                    ..Default::default()
                },
                content,
            )
        } else {
            content
        };
        let constraints = BoxConstraints::tight_for(width, height)
            .expect("finite floating action button size is valid");
        let surface = DecoratedBox::new(BoxDecoration {
            color: background,
            corner_radius: egui::CornerRadius::same(radius),
            ..Default::default()
        })
        .into_widget(Widget::constrained(
            constraints,
            Widget::align(crate::layout::Alignment::CENTER, content),
        ));
        let callback = self.on_pressed;
        let semantics_callback = callback.clone();
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
        .expect("floating action button gesture binding must remain available")
        .behavior(HitTestBehavior::Opaque)
        .into_widget(surface);
        let mut semantics =
            Semantics::new(egui::accesskit::Role::Button).on_click(move || semantics_callback());
        if let Some(label) = self.semantics_label {
            semantics = semantics.label(label);
        }
        semantics.into_widget(visual)
    }
}
