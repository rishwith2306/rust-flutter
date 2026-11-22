use super::{FloatingActionButton, ThemeData};
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, Color32, Stroke},
    element::{FlexItem, FlexStyle, StackItem, StackStyle, Widget},
    gestures::{
        GestureBinding, GestureButton, GestureDetector, GestureTargetConfig, TapCallbacks,
        TapConfig,
    },
    layout::{
        Alignment, Axis, BoxConstraints, Clip, CrossAxisAlignment, HitTestBehavior, Insets,
        Positioned, StackFit,
    },
    text::{DefaultTextStyle, Text, TextStyle},
};
use std::rc::Rc;

/// A Material side panel. Give the scaffold drawer a fixed logical width.
pub struct Drawer {
    child: Widget,
    width: f32,
    background_color: Color32,
}

impl Drawer {
    pub fn new(child: Widget) -> Self {
        Self {
            child,
            width: 304.0,
            background_color: ThemeData::default().color_scheme.surface,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn background_color(mut self, color: Color32) -> Self {
        self.background_color = color;
        self
    }

    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.background_color = theme.color_scheme.surface;
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if !self.width.is_finite() || self.width <= 0.0 {
            return Err("Material drawer width must be positive and finite");
        }
        let child = Widget::safe_area(self.child);
        let panel = DecoratedBox::new(BoxDecoration {
            color: self.background_color,
            border: Stroke::new(0.5_f32, Color32::from_black_alpha(28)),
            corner_radius: egui::CornerRadius::same(16),
        })
        .into_widget(child);
        let constraints = BoxConstraints::tight_for(Some(self.width), None)
            .expect("validated finite drawer width");
        Ok(Widget::constrained(constraints, panel))
    }
}

struct DrawerSlot {
    binding: GestureBinding,
    child: Widget,
    open: bool,
    on_open: Rc<dyn Fn()>,
    on_dismiss: Rc<dyn Fn()>,
}

/// A 56-point Material top bar with a title, optional leading slot and actions.
pub struct AppBar {
    title: String,
    leading: Option<Widget>,
    actions: Vec<Widget>,
    background_color: Color32,
    foreground_color: Color32,
}

impl AppBar {
    pub fn new(title: impl Into<String>) -> Self {
        let scheme = ThemeData::default().color_scheme;
        Self {
            title: title.into(),
            leading: None,
            actions: Vec::new(),
            background_color: scheme.surface,
            foreground_color: scheme.on_surface,
        }
    }

    pub fn leading(mut self, widget: Widget) -> Self {
        self.leading = Some(widget);
        self
    }

    pub fn action(mut self, widget: Widget) -> Self {
        self.actions.push(widget);
        self
    }

    pub fn background_color(mut self, color: Color32) -> Self {
        self.background_color = color;
        self
    }

    pub fn foreground_color(mut self, color: Color32) -> Self {
        self.foreground_color = color;
        self
    }

    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.background_color = theme.color_scheme.surface;
        self.foreground_color = theme.color_scheme.on_surface;
        self
    }

    pub fn into_widget(self) -> Widget {
        let title = DefaultTextStyle {
            style: TextStyle {
                font_size: Some(22.0),
                color: Some(self.foreground_color),
                ..Default::default()
            },
            ..Default::default()
        }
        .provide(Text::new(self.title).into_widget());
        let mut items = Vec::with_capacity(2 + self.actions.len());
        if let Some(leading) = self.leading {
            let width = BoxConstraints::tight_for(Some(56.0), None)
                .expect("finite app bar leading width is valid");
            items.push(FlexItem::fixed(Widget::constrained(width, leading)));
        }
        items.push(FlexItem::expanded(title, 1));
        items.extend(self.actions.into_iter().map(FlexItem::fixed));
        let mut style = FlexStyle::new(Axis::Horizontal);
        style.cross_alignment = CrossAxisAlignment::Center;
        let row = Widget::flex(style, items);
        let row = Widget::padding(
            Insets {
                left: 16.0,
                right: 16.0,
                ..Default::default()
            },
            row,
        );
        let height =
            BoxConstraints::tight_for(None, Some(56.0)).expect("finite app bar height is valid");
        DecoratedBox::new(BoxDecoration::solid(self.background_color))
            .into_widget(Widget::constrained(height, row))
    }
}

/// A Material page shell with optional top and bottom bars.
///
/// The body expands to the available height. A controlled `Drawer` can be
/// layered over the page with scrim dismissal; entrance animation, swipe
/// gestures, end drawers, and keyboard insets remain explicit app work.
pub struct Scaffold {
    body: Widget,
    floating_action_button: Option<Widget>,
    app_bar: Option<Widget>,
    app_bar_config: Option<AppBar>,
    bottom_navigation_bar: Option<Widget>,
    drawer: Option<DrawerSlot>,
    background_color: Color32,
    scrim_color: Color32,
}

impl Scaffold {
    pub fn new(body: Widget) -> Self {
        Self {
            body,
            floating_action_button: None,
            app_bar: None,
            app_bar_config: None,
            bottom_navigation_bar: None,
            drawer: None,
            background_color: ThemeData::default().color_scheme.surface,
            scrim_color: Color32::from_black_alpha(96),
        }
    }

    pub fn app_bar(mut self, app_bar: Widget) -> Self {
        self.app_bar = Some(app_bar);
        self.app_bar_config = None;
        self
    }

    /// Configure an app bar that can add the drawer menu action automatically.
    /// An explicit `.leading(...)` keeps its custom action.
    pub fn app_bar_config(mut self, app_bar: AppBar) -> Self {
        self.app_bar = None;
        self.app_bar_config = Some(app_bar);
        self
    }

    pub fn bottom_navigation_bar(mut self, bar: Widget) -> Self {
        self.bottom_navigation_bar = Some(bar);
        self
    }

    pub fn floating_action_button(mut self, button: FloatingActionButton) -> Self {
        self.floating_action_button = Some(button.into_widget());
        self
    }

    /// Add a controlled modal drawer. The owner updates `open` after invoking
    /// `on_dismiss`, for example from a stateful page or an app-bar action.
    pub fn drawer(
        mut self,
        binding: &GestureBinding,
        child: Widget,
        open: bool,
        on_open: impl Fn() + 'static,
        on_dismiss: impl Fn() + 'static,
    ) -> Self {
        self.drawer = Some(DrawerSlot {
            binding: binding.clone(),
            child,
            open,
            on_open: Rc::new(on_open),
            on_dismiss: Rc::new(on_dismiss),
        });
        self
    }

    pub fn background_color(mut self, color: Color32) -> Self {
        self.background_color = color;
        self
    }

    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.background_color = theme.color_scheme.surface;
        self.scrim_color = theme.color_scheme.scrim.gamma_multiply(0.38);
        self
    }

    pub fn into_widget(self) -> Widget {
        let app_bar = self.app_bar_config.map(|mut app_bar| {
            if app_bar.leading.is_none()
                && let Some(drawer) = &self.drawer
            {
                let on_open = drawer.on_open.clone();
                app_bar.leading = Some(
                    GestureButton::new(&drawer.binding, "☰", move || on_open())
                        .size(egui::vec2(48.0, 48.0))
                        .into_widget(),
                );
            }
            app_bar.into_widget()
        });
        let app_bar = app_bar.or(self.app_bar);
        let mut body = self.body;
        if let Some(button) = self.floating_action_button {
            body = Widget::stack(
                StackStyle {
                    alignment: Alignment::TOP_LEFT,
                    fit: StackFit::Expand,
                    clip: Clip::HardEdge,
                },
                vec![
                    StackItem::new(body),
                    StackItem::positioned(
                        button,
                        Positioned {
                            right: Some(16.0),
                            bottom: Some(16.0),
                            ..Default::default()
                        },
                    ),
                ],
            );
        }
        let mut children = Vec::with_capacity(3);
        if let Some(app_bar) = app_bar {
            children.push(FlexItem::fixed(app_bar));
        }
        children.push(FlexItem::expanded(body, 1));
        if let Some(bottom) = self.bottom_navigation_bar {
            children.push(FlexItem::fixed(bottom));
        }
        let mut style = FlexStyle::new(Axis::Vertical);
        style.cross_alignment = CrossAxisAlignment::Stretch;
        let page = DecoratedBox::new(BoxDecoration::solid(self.background_color))
            .into_widget(Widget::flex(style, children));
        let Some(drawer) = self.drawer else {
            return page;
        };
        let on_dismiss = drawer.on_dismiss.clone();
        let barrier = GestureDetector::new(
            &drawer.binding,
            GestureTargetConfig {
                tap: Some((
                    TapConfig::default(),
                    TapCallbacks {
                        on_tap: Some(Rc::new(move |_| on_dismiss())),
                        ..Default::default()
                    },
                )),
                ..Default::default()
            },
        )
        .expect("live Material drawer gesture binding")
        .behavior(HitTestBehavior::Opaque)
        .into_widget(
            DecoratedBox::new(BoxDecoration::solid(self.scrim_color)).into_widget(Widget::empty()),
        );
        let barrier = Widget::offstage(!drawer.open, barrier);
        let child = Widget::offstage(!drawer.open, drawer.child);
        Widget::stack(
            StackStyle {
                alignment: Alignment::TOP_LEFT,
                fit: StackFit::Expand,
                clip: Clip::HardEdge,
            },
            vec![
                StackItem::new(page),
                StackItem::positioned(barrier, Positioned::fill()),
                StackItem::positioned(
                    child,
                    Positioned {
                        left: Some(0.0),
                        top: Some(0.0),
                        bottom: Some(0.0),
                        ..Default::default()
                    },
                ),
            ],
        )
    }
}
