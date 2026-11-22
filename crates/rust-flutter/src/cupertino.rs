//! An initial Cupertino theme and button foundation.
//!
//! This is a small starting point for Cupertino styling, not a complete iOS
//! component catalog or native macOS/iOS behavior layer.
use crate::{
    decoration::{BoxDecoration, DecoratedBox},
    egui::{self, Color32, Stroke, Vec2},
    element::{BuildContext, DependencyError, FlexItem, FlexStyle, StatelessWidget, Widget},
    forms::{TextEditingController, TextFormField},
    gestures::{
        GestureBinding, GestureButton, GestureDetector, GestureTargetConfig, TapCallbacks,
        TapConfig,
    },
    layout::{Axis, BoxConstraints, CrossAxisAlignment, HitTestBehavior, Insets},
    text::{DefaultTextStyle, Text, TextStyle},
};
use std::rc::Rc;

mod sliding_segmented;
pub use sliding_segmented::{CupertinoSlidingSegment, CupertinoSlidingSegmentedControl};

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

/// A Cupertino settings-style row with optional supporting content and action.
pub struct CupertinoListTile {
    binding: GestureBinding,
    title: Widget,
    subtitle: Option<Widget>,
    additional_info: Option<Widget>,
    leading: Option<Widget>,
    trailing: Option<Widget>,
    on_tap: Option<Rc<dyn Fn()>>,
    enabled: bool,
    background_color: Color32,
    leading_size: Vec2,
    leading_to_title: f32,
    notched: bool,
}

impl CupertinoListTile {
    pub fn new(binding: &GestureBinding, title: Widget) -> Self {
        Self {
            binding: binding.clone(),
            title,
            subtitle: None,
            additional_info: None,
            leading: None,
            trailing: None,
            on_tap: None,
            enabled: true,
            background_color: CupertinoThemeData::default().scaffold_background_color,
            leading_size: Vec2::splat(28.0),
            leading_to_title: 12.0,
            notched: false,
        }
    }

    pub fn subtitle(mut self, subtitle: Widget) -> Self {
        self.subtitle = Some(subtitle);
        self
    }

    pub fn additional_info(mut self, info: Widget) -> Self {
        self.additional_info = Some(info);
        self
    }

    pub fn leading(mut self, leading: Widget) -> Self {
        self.leading = Some(leading);
        self
    }

    pub fn leading_size(mut self, size: Vec2) -> Self {
        self.leading_size = size;
        self
    }

    pub fn leading_to_title(mut self, spacing: f32) -> Self {
        self.leading_to_title = spacing;
        self
    }

    pub fn trailing(mut self, trailing: Widget) -> Self {
        self.trailing = Some(trailing);
        self
    }

    pub fn on_tap(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_tap = Some(Rc::new(callback));
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn background_color(mut self, color: Color32) -> Self {
        self.background_color = color;
        self
    }

    /// Use an inset, rounded tile style similar to grouped iOS lists.
    pub fn notched(mut self) -> Self {
        self.notched = true;
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if !self.leading_size.is_finite()
            || self.leading_size.x < 0.0
            || self.leading_size.y < 0.0
            || !self.leading_to_title.is_finite()
            || self.leading_to_title < 0.0
        {
            return Err("invalid Cupertino list tile leading size or spacing");
        }
        let mut title_column = vec![FlexItem::fixed(self.title)];
        if let Some(subtitle) = self.subtitle {
            title_column.push(FlexItem::fixed(subtitle));
        }
        let mut title_style = FlexStyle::new(Axis::Vertical);
        title_style.cross_alignment = CrossAxisAlignment::Stretch;
        let mut row_items = Vec::with_capacity(4);
        if let Some(leading) = self.leading {
            let limits =
                BoxConstraints::tight_for(Some(self.leading_size.x), Some(self.leading_size.y))
                    .map_err(|_| "invalid Cupertino list tile leading constraints")?;
            row_items.push(FlexItem::fixed(Widget::padding(
                Insets {
                    right: self.leading_to_title,
                    ..Default::default()
                },
                Widget::constrained(limits, leading),
            )));
        }
        row_items.push(FlexItem::expanded(
            Widget::flex(title_style, title_column),
            1,
        ));
        if let Some(info) = self.additional_info {
            row_items.push(FlexItem::fixed(info));
        }
        if let Some(trailing) = self.trailing {
            row_items.push(FlexItem::fixed(trailing));
        }
        let mut row_style = FlexStyle::new(Axis::Horizontal);
        row_style.cross_alignment = CrossAxisAlignment::Center;
        let content = Widget::padding(
            Insets {
                left: 16.0,
                right: 16.0,
                top: 10.0,
                bottom: 10.0,
            },
            Widget::flex(row_style, row_items),
        );
        let min_height = BoxConstraints::new(
            egui::vec2(0.0, 44.0),
            egui::vec2(f32::INFINITY, f32::INFINITY),
        )
        .map_err(|_| "invalid Cupertino list tile minimum height")?;
        let decoration = BoxDecoration {
            color: self.background_color,
            corner_radius: if self.notched {
                egui::CornerRadius::same(9)
            } else {
                egui::CornerRadius::ZERO
            },
            ..Default::default()
        };
        let mut tile =
            DecoratedBox::new(decoration).into_widget(Widget::constrained(min_height, content));
        if self.notched {
            tile = Widget::padding(
                Insets {
                    left: 16.0,
                    right: 16.0,
                    ..Default::default()
                },
                tile,
            );
        }
        if self.enabled
            && let Some(callback) = self.on_tap
        {
            let config = GestureTargetConfig {
                tap: Some((
                    TapConfig::default(),
                    TapCallbacks {
                        on_tap: Some(Rc::new(move |_| callback())),
                        ..Default::default()
                    },
                )),
                ..Default::default()
            };
            tile = GestureDetector::new(&self.binding, config)
                .expect("valid Cupertino list tile tap gesture")
                .behavior(HitTestBehavior::Opaque)
                .into_widget(tile);
        }
        Ok(tile)
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

/// A controlled Cupertino range slider, using the shared slider gesture engine.
pub struct CupertinoSlider {
    slider: crate::material::Slider,
    enabled: bool,
}

impl CupertinoSlider {
    pub fn new(binding: &GestureBinding, value: f32, on_changed: impl Fn(f32) + 'static) -> Self {
        Self {
            slider: crate::material::Slider::new(binding, value, on_changed),
            enabled: true,
        }
    }

    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.slider = self.slider.range(min, max);
        self
    }

    pub fn divisions(mut self, divisions: Option<u32>) -> Self {
        self.slider = self.slider.divisions(divisions);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.slider = self.slider.width(width);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.slider = self.slider.enabled(enabled);
        self.enabled = enabled;
        self
    }

    pub fn on_change_start(mut self, callback: impl Fn(f32) + 'static) -> Self {
        self.slider = self.slider.on_change_start(callback);
        self
    }

    pub fn on_change_end(mut self, callback: impl Fn(f32) + 'static) -> Self {
        self.slider = self.slider.on_change_end(callback);
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        self.slider.validate()?;
        Ok(Widget::stateless(CupertinoSliderView {
            slider: self.slider,
            enabled: self.enabled,
        }))
    }
}

struct CupertinoSliderView {
    slider: crate::material::Slider,
    enabled: bool,
}

impl StatelessWidget for CupertinoSliderView {
    fn build(&self, context: &BuildContext) -> Widget {
        let theme = CupertinoThemeData::of(context).unwrap_or_default();
        let enabled = self.enabled;
        let palette = if enabled {
            crate::material::SliderColors {
                active: theme.primary_color,
                inactive: Color32::from_gray(145),
                thumb: Color32::WHITE,
                outline: theme.primary_color,
            }
        } else {
            crate::material::SliderColors {
                active: theme.disabled_color,
                inactive: theme.disabled_color.gamma_multiply(0.5),
                thumb: Color32::from_gray(210),
                outline: theme.disabled_color,
            }
        };
        self.slider
            .clone()
            .with_colors(palette)
            .into_widget()
            .unwrap_or_else(|_| Widget::empty())
    }
}

/// Owner-controlled Cupertino switch. Update the supplied value in its owner.
pub struct CupertinoSwitch {
    binding: GestureBinding,
    value: bool,
    enabled: bool,
    on_changed: Rc<dyn Fn(bool)>,
}

impl CupertinoSwitch {
    pub fn new(binding: &GestureBinding, value: bool, on_changed: impl Fn(bool) + 'static) -> Self {
        Self {
            binding: binding.clone(),
            value,
            enabled: true,
            on_changed: Rc::new(on_changed),
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn into_widget(self) -> Widget {
        Widget::stateless(CupertinoSwitchView(self))
    }
}

struct CupertinoSwitchView(CupertinoSwitch);

impl StatelessWidget for CupertinoSwitchView {
    fn build(&self, _: &BuildContext) -> Widget {
        let switch = &self.0;
        let active = Color32::from_rgb(52, 199, 89);
        let track = if !switch.enabled {
            Color32::from_gray(90)
        } else if switch.value {
            active
        } else {
            Color32::from_gray(125)
        };
        let thumb = if switch.enabled {
            Color32::WHITE
        } else {
            Color32::from_gray(205)
        };
        let paint_value = switch.value;
        let paint_enabled = switch.enabled;
        let paint = move |painter: &egui::Painter, bounds: egui::Rect| {
            let rect = egui::Rect::from_center_size(bounds.center(), Vec2::new(51.0, 31.0));
            painter.rect_filled(rect, egui::CornerRadius::same(16), track);
            if !paint_enabled {
                painter.rect_stroke(
                    rect,
                    egui::CornerRadius::same(16),
                    Stroke::new(1.0_f32, Color32::from_gray(145)),
                    egui::StrokeKind::Inside,
                );
            }
            let center_x = if paint_value {
                rect.right() - 15.5
            } else {
                rect.left() + 15.5
            };
            painter.circle_filled(egui::pos2(center_x, rect.center().y), 13.5, thumb);
        };
        let callback = switch.on_changed.clone();
        let value = switch.value;
        GestureButton::new(&switch.binding, egui::RichText::new(""), move || {
            callback(!value)
        })
        .size(Vec2::new(60.0, 44.0))
        .enabled(switch.enabled)
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
        .corner_radius(egui::CornerRadius::ZERO)
        .paint_content(paint)
        .into_widget()
    }
}

/// A Cupertino-style radial activity indicator with optional static progress.
pub struct CupertinoActivityIndicator {
    animating: bool,
    radius: f32,
    color: Option<Color32>,
    progress: Option<f32>,
}

impl CupertinoActivityIndicator {
    pub fn new() -> Self {
        Self {
            animating: true,
            radius: 10.0,
            color: None,
            progress: None,
        }
    }

    pub fn partially_revealed(progress: f32) -> Self {
        Self {
            animating: false,
            radius: 10.0,
            color: None,
            progress: Some(progress),
        }
    }

    pub fn animating(mut self, animating: bool) -> Self {
        self.animating = animating;
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if !self.radius.is_finite()
            || self.radius <= 0.0
            || self
                .progress
                .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            return Err("invalid Cupertino activity indicator radius or progress");
        }
        Ok(Widget::stateless(CupertinoActivityIndicatorView(self)))
    }
}

impl Default for CupertinoActivityIndicator {
    fn default() -> Self {
        Self::new()
    }
}

struct CupertinoActivityIndicatorView(CupertinoActivityIndicator);

impl StatelessWidget for CupertinoActivityIndicatorView {
    fn build(&self, context: &BuildContext) -> Widget {
        let indicator = &self.0;
        let color = indicator.color.unwrap_or_else(|| {
            CupertinoThemeData::of(context)
                .unwrap_or_default()
                .text_color
        });
        let radius = indicator.radius;
        let animating = indicator.animating;
        let progress = indicator.progress;
        Widget::leaf(Vec2::splat(radius * 2.0), move |ui| {
            let bounds = ui.max_rect();
            let center = bounds.center();
            let current_tick = if animating {
                ui.ctx().request_repaint();
                (ui.input(|input| input.time) * 12.0).floor() as usize % 12
            } else {
                0
            };
            let visible_ticks = progress.map_or(12, |value| (value * 12.0).ceil() as usize);
            for tick in 0..visible_ticks {
                let angle =
                    std::f32::consts::TAU * tick as f32 / 12.0 - std::f32::consts::FRAC_PI_2;
                let direction = egui::vec2(angle.cos(), angle.sin());
                let start = center + direction * (radius * 0.43);
                let end = center + direction * (radius * 0.88);
                let age = (tick + 12 - current_tick) % 12;
                let alpha = if animating {
                    0.2 + 0.8 * (11 - age) as f32 / 11.0
                } else {
                    0.25 + 0.75 * tick as f32 / 11.0
                };
                ui.painter().line_segment(
                    [start, end],
                    egui::Stroke::new((radius * 0.13).max(1.0), color.gamma_multiply(alpha)),
                );
            }
        })
    }
}

/// A labeled option for [`CupertinoSegmentedControl`].
pub struct CupertinoSegment<T> {
    pub value: T,
    pub label: String,
    pub enabled: bool,
}

impl<T> CupertinoSegment<T> {
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            enabled: true,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// A controlled, mutually exclusive Cupertino segmented control.
pub struct CupertinoSegmentedControl<T: Clone + PartialEq + 'static> {
    binding: GestureBinding,
    segments: Vec<CupertinoSegment<T>>,
    selected: T,
    width: f32,
    height: f32,
    on_changed: Rc<dyn Fn(T)>,
}

impl<T: Clone + PartialEq + 'static> CupertinoSegmentedControl<T> {
    pub fn new(
        binding: &GestureBinding,
        segments: Vec<CupertinoSegment<T>>,
        selected: T,
        on_changed: impl Fn(T) + 'static,
    ) -> Result<Self, &'static str> {
        if segments.is_empty()
            || !segments.iter().any(|segment| segment.value == selected)
            || segments.iter().enumerate().any(|(index, segment)| {
                segments[..index]
                    .iter()
                    .any(|previous| previous.value == segment.value)
            })
        {
            return Err("segmented control requires unique options and a selected value");
        }
        Ok(Self {
            binding: binding.clone(),
            segments,
            selected,
            width: 96.0,
            height: 32.0,
            on_changed: Rc::new(on_changed),
        })
    }

    pub fn segment_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if !self.width.is_finite()
            || self.width <= 0.0
            || !self.height.is_finite()
            || self.height <= 0.0
        {
            return Err("segmented control dimensions must be positive and finite");
        }
        Ok(Widget::stateless(CupertinoSegmentedControlView(self)))
    }
}

struct CupertinoSegmentedControlView<T: Clone + PartialEq + 'static>(CupertinoSegmentedControl<T>);

impl<T: Clone + PartialEq + 'static> StatelessWidget for CupertinoSegmentedControlView<T> {
    fn build(&self, context: &BuildContext) -> Widget {
        let control = &self.0;
        let theme = CupertinoThemeData::of(context).unwrap_or_default();
        let selected = control.selected.clone();
        let on_changed = control.on_changed.clone();
        let mut children = Vec::with_capacity(control.segments.len());
        for (index, segment) in control.segments.iter().enumerate() {
            let is_selected = segment.value == selected;
            let value = segment.value.clone();
            let selected_value = selected.clone();
            let callback = on_changed.clone();
            let button = GestureButton::new(
                &control.binding,
                egui::RichText::new(segment.label.clone()).color(if segment.enabled {
                    if is_selected {
                        Color32::WHITE
                    } else {
                        theme.primary_color
                    }
                } else {
                    theme.disabled_color
                }),
                move || {
                    if value != selected_value {
                        callback(value.clone());
                    }
                },
            )
            .size(Vec2::new(control.width, control.height))
            .enabled(segment.enabled)
            .fill(if is_selected {
                theme.primary_color
            } else {
                theme.scaffold_background_color
            })
            .corner_radius(egui::CornerRadius::same(6))
            .paint_content(move |painter, bounds| {
                if index > 0 {
                    painter.line_segment(
                        [bounds.left_top(), bounds.left_bottom()],
                        Stroke::new(0.5_f32, theme.primary_color.gamma_multiply(0.45)),
                    );
                }
            });
            children.push(FlexItem::fixed(button.into_widget()));
        }
        let mut row = FlexStyle::new(Axis::Horizontal);
        row.cross_alignment = CrossAxisAlignment::Center;
        let content = Widget::flex(row, children);
        DecoratedBox::new(BoxDecoration {
            color: theme.scaffold_background_color,
            border: Stroke::new(1.0_f32, theme.primary_color.gamma_multiply(0.45)),
            corner_radius: egui::CornerRadius::same(6),
        })
        .into_widget(content)
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
