//! An initial Cupertino theme and button foundation.
//!
//! This is a small starting point for Cupertino styling, not a complete iOS
//! component catalog or native macOS/iOS behavior layer.
use crate::{
    egui::{self, Color32, Vec2},
    element::{BuildContext, DependencyError, StatelessWidget, Widget},
    gestures::{GestureBinding, GestureButton},
};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Brightness {
    Light,
    Dark,
}

/// Core colors used by the first retained Cupertino widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CupertinoThemeData {
    pub brightness: Brightness,
    pub primary_color: Color32,
    pub scaffold_background_color: Color32,
    pub text_color: Color32,
    pub disabled_color: Color32,
}

impl CupertinoThemeData {
    pub const fn light() -> Self {
        Self {
            brightness: Brightness::Light,
            primary_color: Color32::from_rgb(0, 122, 255),
            scaffold_background_color: Color32::WHITE,
            text_color: Color32::BLACK,
            disabled_color: Color32::from_gray(150),
        }
    }

    pub const fn dark() -> Self {
        Self {
            brightness: Brightness::Dark,
            primary_color: Color32::from_rgb(10, 132, 255),
            scaffold_background_color: Color32::from_rgb(0x00, 0x00, 0x00),
            text_color: Color32::WHITE,
            disabled_color: Color32::from_gray(110),
        }
    }

    pub fn of(context: &BuildContext) -> Result<Self, DependencyError> {
        Ok(context
            .depend_on::<Self>()?
            .map(|theme| *theme)
            .unwrap_or_default())
    }

    pub fn provide(self, child: Widget) -> Widget {
        Widget::inherited(self, child)
    }
}

impl Default for CupertinoThemeData {
    fn default() -> Self {
        Self::light()
    }
}

/// Root widget that makes Cupertino theme data available to its home subtree.
pub struct CupertinoApp {
    home: Widget,
    theme: CupertinoThemeData,
}

impl CupertinoApp {
    pub fn new(home: Widget) -> Self {
        Self {
            home,
            theme: CupertinoThemeData::default(),
        }
    }

    pub fn theme(mut self, theme: CupertinoThemeData) -> Self {
        self.theme = theme;
        self
    }

    pub fn into_widget(self) -> Widget {
        self.theme.provide(self.home)
    }
}

/// Owner-controlled Cupertino button with regular or filled styling.
#[derive(Clone)]
pub struct CupertinoButton {
    binding: GestureBinding,
    label: String,
    on_pressed: Rc<dyn Fn()>,
    enabled: bool,
    size: Vec2,
    filled: bool,
}

impl CupertinoButton {
    pub fn new(
        binding: &GestureBinding,
        label: impl Into<String>,
        on_pressed: impl Fn() + 'static,
    ) -> Self {
        Self {
            binding: binding.clone(),
            label: label.into(),
            on_pressed: Rc::new(on_pressed),
            enabled: true,
            size: egui::vec2(80.0, 44.0),
            filled: false,
        }
    }

    pub fn filled(
        binding: &GestureBinding,
        label: impl Into<String>,
        on_pressed: impl Fn() + 'static,
    ) -> Self {
        let mut button = Self::new(binding, label, on_pressed);
        button.filled = true;
        button
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    pub fn into_widget(self) -> Widget {
        Widget::stateless(CupertinoButtonWidget(self))
    }
}

#[derive(Clone)]
struct CupertinoButtonWidget(CupertinoButton);

impl StatelessWidget for CupertinoButtonWidget {
    fn build(&self, context: &BuildContext) -> Widget {
        let theme =
            CupertinoThemeData::of(context).expect("Cupertino buttons resolve theme during build");
        let (fill, foreground) = if self.0.filled {
            (
                Some(theme.primary_color),
                if self.0.enabled {
                    Color32::WHITE
                } else {
                    theme.disabled_color
                },
            )
        } else {
            (
                None,
                if self.0.enabled {
                    theme.primary_color
                } else {
                    theme.disabled_color
                },
            )
        };
        let callback = self.0.on_pressed.clone();
        let mut button = GestureButton::new(
            &self.0.binding,
            egui::RichText::new(self.0.label.clone()).color(foreground),
            move || callback(),
        )
        .size(self.0.size)
        .enabled(self.0.enabled)
        .corner_radius(egui::CornerRadius::same(22));
        if let Some(fill) = fill {
            button = button.fill(fill);
        }
        button.into_widget()
    }
}
