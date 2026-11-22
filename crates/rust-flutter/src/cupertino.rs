//! An initial Cupertino theme and button foundation.
//!
//! This is a small starting point for Cupertino styling, not a complete iOS
//! component catalog or native macOS/iOS behavior layer.
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, Color32, Vec2},
    element::{BuildContext, DependencyError, FlexItem, FlexStyle, StatelessWidget, Widget},
    forms::{TextEditingController, TextFormField},
    gestures::{GestureBinding, GestureButton},
    layout::{Axis, BoxConstraints, CrossAxisAlignment, Insets},
    text::{DefaultTextStyle, Text, TextStyle},
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

/// A 44-point Cupertino navigation bar with optional leading and trailing slots.
pub struct CupertinoNavigationBar {
    title: String,
    leading: Option<Widget>,
    trailing: Option<Widget>,
    background_color: Color32,
    foreground_color: Color32,
}

impl CupertinoNavigationBar {
    pub fn new(title: impl Into<String>) -> Self {
        let theme = CupertinoThemeData::default();
        Self {
            title: title.into(),
            leading: None,
            trailing: None,
            background_color: theme.scaffold_background_color,
            foreground_color: theme.text_color,
        }
    }

    pub fn leading(mut self, widget: Widget) -> Self {
        self.leading = Some(widget);
        self
    }

    pub fn trailing(mut self, widget: Widget) -> Self {
        self.trailing = Some(widget);
        self
    }

    pub fn theme(mut self, theme: CupertinoThemeData) -> Self {
        self.background_color = theme.scaffold_background_color;
        self.foreground_color = theme.text_color;
        self
    }

    pub fn into_widget(self) -> Widget {
        let title = DefaultTextStyle {
            style: TextStyle {
                font_size: Some(17.0),
                color: Some(self.foreground_color),
                ..Default::default()
            },
            ..Default::default()
        }
        .provide(Text::new(self.title).into_widget());
        let slot = |child: Option<Widget>| {
            let width = BoxConstraints::tight_for(Some(72.0), None)
                .expect("finite Cupertino navigation slot width is valid");
            Widget::constrained(width, child.unwrap_or_else(Widget::empty))
        };
        let mut row_style = FlexStyle::new(Axis::Horizontal);
        row_style.cross_alignment = CrossAxisAlignment::Center;
        let row = Widget::flex(
            row_style,
            vec![
                FlexItem::fixed(slot(self.leading)),
                FlexItem::expanded(title, 1),
                FlexItem::fixed(slot(self.trailing)),
            ],
        );
        let row = Widget::padding(Insets::all(8.0), row);
        let height = BoxConstraints::tight_for(None, Some(44.0))
            .expect("finite Cupertino navigation bar height is valid");
        DecoratedBox::new(BoxDecoration {
            color: self.background_color,
            border: egui::Stroke::new(0.5_f32, self.foreground_color.gamma_multiply(0.2)),
            ..Default::default()
        })
        .into_widget(Widget::constrained(height, row))
    }
}

/// Cupertino page shell with an optional navigation bar and expanding body.
pub struct CupertinoPageScaffold {
    child: Widget,
    navigation_bar: Option<Widget>,
    background_color: Color32,
}

impl CupertinoPageScaffold {
    pub fn new(child: Widget) -> Self {
        Self {
            child,
            navigation_bar: None,
            background_color: CupertinoThemeData::default().scaffold_background_color,
        }
    }

    pub fn navigation_bar(mut self, navigation_bar: Widget) -> Self {
        self.navigation_bar = Some(navigation_bar);
        self
    }

    pub fn theme(mut self, theme: CupertinoThemeData) -> Self {
        self.background_color = theme.scaffold_background_color;
        self
    }

    pub fn background_color(mut self, color: Color32) -> Self {
        self.background_color = color;
        self
    }

    pub fn into_widget(self) -> Widget {
        let mut children = Vec::with_capacity(2);
        if let Some(navigation_bar) = self.navigation_bar {
            children.push(FlexItem::fixed(navigation_bar));
        }
        children.push(FlexItem::expanded(self.child, 1));
        let mut style = FlexStyle::new(Axis::Vertical);
        style.cross_alignment = CrossAxisAlignment::Stretch;
        DecoratedBox::new(BoxDecoration::solid(self.background_color))
            .into_widget(Widget::flex(style, children))
    }
}

/// Cupertino styled text entry backed by the shared retained editor/controller.
pub struct CupertinoTextField {
    field: TextFormField,
}

impl CupertinoTextField {
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            field: TextFormField::new(initial),
        }
    }

    pub fn with_controller(controller: TextEditingController) -> Self {
        Self {
            field: TextFormField::with_controller(controller),
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.field.decoration.hint = placeholder.into();
        self
    }

    pub fn on_changed(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.field = self.field.on_changed(callback);
        self
    }

    pub fn on_submitted(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.field = self.field.on_submitted(callback);
        self
    }

    pub fn read_only(mut self, value: bool) -> Self {
        self.field = self.field.read_only(value);
        self
    }

    pub fn enabled(mut self, value: bool) -> Self {
        self.field = self.field.enabled(value);
        self
    }

    pub fn obscure_text(mut self, value: bool) -> Self {
        self.field = self.field.obscure_text(value);
        self
    }

    pub fn multiline(mut self, min_lines: usize, max_lines: Option<usize>) -> Self {
        self.field = self.field.multiline(min_lines, max_lines);
        self
    }

    pub fn max_chars(mut self, value: Option<usize>) -> Self {
        self.field = self.field.max_chars(value);
        self
    }

    pub fn preferred_width(mut self, width: f32) -> Self {
        self.field = self.field.preferred_width(width);
        self
    }

    pub fn into_widget(self) -> Result<Widget, crate::forms::TextEditingError> {
        self.field.clone().into_widget()?;
        Ok(Widget::stateless(CupertinoTextFieldWidget(self.field)))
    }
}

struct CupertinoTextFieldWidget(TextFormField);

impl StatelessWidget for CupertinoTextFieldWidget {
    fn build(&self, context: &BuildContext) -> Widget {
        let theme = CupertinoThemeData::of(context).unwrap_or_default();
        let mut field = self.0.clone();
        if field.decoration.text_color.is_none() {
            field.decoration.text_color = Some(theme.text_color);
        }
        if field.decoration.background_color.is_none() {
            field.decoration.background_color = Some(Color32::TRANSPARENT);
        }
        field.into_widget().unwrap_or_else(|_| Widget::empty())
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
