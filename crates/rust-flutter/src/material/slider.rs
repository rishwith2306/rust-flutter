//! Controlled continuous and discrete Material slider.
use super::ThemeData;
use crate::{
    egui::{self, Color32, Pos2, Rect, Stroke, Vec2},
    element::{
        BuildContext, RenderBox, RenderContext, RenderObjectWidget, StatelessWidget, Widget,
    },
    gestures::{
        DragAxis, DragEndDetails, GestureBinding, GestureDetector, GestureTargetConfig,
        PanCallbacks, PanConfig, PointerEvent, TapCallbacks, TapConfig, TapDetails,
    },
    layout::{
        BoxChildren, BoxConstraints, BoxGeometry, BoxQueries, IntrinsicDimension, LayoutError,
    },
};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy)]
pub(crate) struct SliderColors {
    pub active: Color32,
    pub inactive: Color32,
    pub thumb: Color32,
    pub outline: Color32,
}

/// A controlled range slider. The owner supplies the updated value after
/// `on_changed` runs.
#[derive(Clone)]
pub struct Slider {
    binding: GestureBinding,
    value: f32,
    min: f32,
    max: f32,
    divisions: Option<u32>,
    width: f32,
    enabled: bool,
    on_changed: Rc<dyn Fn(f32)>,
    on_change_start: Option<Rc<dyn Fn(f32)>>,
    on_change_end: Option<Rc<dyn Fn(f32)>>,
    semantics_label: Option<String>,
    colors: Option<SliderColors>,
}

impl Slider {
    pub fn new(binding: &GestureBinding, value: f32, on_changed: impl Fn(f32) + 'static) -> Self {
        Self {
            binding: binding.clone(),
            value,
            min: 0.0,
            max: 1.0,
            divisions: None,
            width: 200.0,
            enabled: true,
            on_changed: Rc::new(on_changed),
            on_change_start: None,
            on_change_end: None,
            semantics_label: None,
            colors: None,
        }
    }

    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Set the number of intervals. `None` keeps the slider continuous.
    pub fn divisions(mut self, divisions: Option<u32>) -> Self {
        self.divisions = divisions;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn on_change_start(mut self, callback: impl Fn(f32) + 'static) -> Self {
        self.on_change_start = Some(Rc::new(callback));
        self
    }

    pub fn on_change_end(mut self, callback: impl Fn(f32) + 'static) -> Self {
        self.on_change_end = Some(Rc::new(callback));
        self
    }

    /// Sets the screen reader name for the numeric slider.
    pub fn semantics_label(mut self, label: impl Into<String>) -> Self {
        self.semantics_label = Some(label.into());
        self
    }

    pub(crate) fn with_colors(mut self, colors: SliderColors) -> Self {
        self.colors = Some(colors);
        self
    }

    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if !self.min.is_finite()
            || !self.max.is_finite()
            || self.min >= self.max
            || !self.value.is_finite()
            || self.value < self.min
            || self.value > self.max
            || self.divisions == Some(0)
            || !self.width.is_finite()
            || self.width <= 0.0
        {
            return Err("invalid slider value, range, divisions, or width");
        }
        Ok(())
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        self.validate()?;
        Ok(Widget::stateless(SliderView(self)))
    }
}

struct SliderView(Slider);

impl StatelessWidget for SliderView {
    fn build(&self, context: &BuildContext) -> Widget {
        let slider = &self.0;
        let colors = slider.colors.unwrap_or_else(|| {
            let scheme = ThemeData::of(context)
                .expect("Material sliders resolve theme during build")
                .color_scheme;
            if slider.enabled {
                SliderColors {
                    active: scheme.primary,
                    inactive: scheme.surface_container,
                    thumb: scheme.primary,
                    outline: scheme.primary,
                }
            } else {
                SliderColors {
                    active: scheme.on_surface.gamma_multiply(0.38),
                    inactive: scheme.on_surface.gamma_multiply(0.12),
                    thumb: scheme.on_surface.gamma_multiply(0.38),
                    outline: scheme.on_surface.gamma_multiply(0.12),
                }
            }
        });
        let position = Rc::new(Cell::new(Rect::NOTHING));
        let value_at = {
            let position = position.clone();
            let min = slider.min;
            let max = slider.max;
            let divisions = slider.divisions;
            Rc::new(move |event: PointerEvent| {
                let rect = position.get();
                if !rect.is_finite() || rect.width() <= 24.0 {
                    return None;
                }
                let t = ((event.position.x - (rect.left() + 12.0)) / (rect.width() - 24.0))
                    .clamp(0.0, 1.0);
                let t = divisions.map_or(t, |count| (t * count as f32).round() / count as f32);
                Some(min + (max - min) * t)
            })
        };
        let mut config = GestureTargetConfig::default();
        if slider.enabled {
            let tap_position = value_at.clone();
            let changed = slider.on_changed.clone();
            config.tap = Some((
                TapConfig::default(),
                TapCallbacks {
                    on_tap: Some(Rc::new(move |details: TapDetails| {
                        if let Some(value) = tap_position(details.up) {
                            changed(value);
                        }
                    })),
                    ..Default::default()
                },
            ));
            let drag_position = value_at.clone();
            let changed = slider.on_changed.clone();
            let on_start = slider.on_change_start.clone();
            let start_value = value_at.clone();
            let on_end = slider.on_change_end.clone();
            let end_value = value_at;
            config.pan = Some((
                PanConfig {
                    axis: Some(DragAxis::Horizontal),
                    slop: 6.0,
                    ..Default::default()
                },
                PanCallbacks {
                    on_start: on_start.map(|callback| {
                        Rc::new(move |event| {
                            if let Some(value) = start_value(event) {
                                callback(value);
                            }
                        }) as Rc<dyn Fn(PointerEvent)>
                    }),
                    on_update: Some(Rc::new(move |details| {
                        if let Some(value) = drag_position(details.event) {
                            changed(value);
                        }
                    })),
                    on_end: on_end.map(|callback| {
                        Rc::new(move |details: DragEndDetails| {
                            if let Some(value) = end_value(details.event) {
                                callback(value);
                            }
                        }) as Rc<dyn Fn(DragEndDetails)>
                    }),
                    ..Default::default()
                },
            ));
        }
        let track = SliderTrack {
            value: slider.value,
            min: slider.min,
            max: slider.max,
            width: slider.width,
            enabled: slider.enabled,
            active: colors.active,
            inactive: colors.inactive,
            thumb: colors.thumb,
            outline: colors.outline,
            position,
            divisions: slider.divisions,
            semantics_label: slider.semantics_label.clone(),
            on_changed: slider.on_changed.clone(),
        };
        GestureDetector::new(&slider.binding, config)
            .expect("validated Material slider gesture configuration")
            .behavior(crate::layout::HitTestBehavior::Opaque)
            .into_widget(Widget::render_object(track, vec![]))
    }
}

struct SliderTrack {
    value: f32,
    min: f32,
    max: f32,
    width: f32,
    enabled: bool,
    active: Color32,
    inactive: Color32,
    thumb: Color32,
    outline: Color32,
    position: Rc<Cell<Rect>>,
    divisions: Option<u32>,
    semantics_label: Option<String>,
    on_changed: Rc<dyn Fn(f32)>,
}

struct SliderTrackRender {
    config: SliderTrack,
    handle: crate::element::RenderHandle,
    id: egui::Id,
}

impl RenderObjectWidget for SliderTrack {
    type Object = SliderTrackRender;

    fn create_render_object(&self, context: &RenderContext) -> Self::Object {
        SliderTrackRender {
            config: SliderTrack {
                value: self.value,
                min: self.min,
                max: self.max,
                width: self.width,
                enabled: self.enabled,
                active: self.active,
                inactive: self.inactive,
                thumb: self.thumb,
                outline: self.outline,
                position: self.position.clone(),
                divisions: self.divisions,
                semantics_label: self.semantics_label.clone(),
                on_changed: self.on_changed.clone(),
            },
            handle: context.handle(),
            id: egui::Id::new(("rust_flutter_slider", context.element_id().value())),
        }
    }

    fn update_render_object(&self, object: &mut Self::Object, context: &RenderContext) {
        object.config.value = self.value;
        object.config.min = self.min;
        object.config.max = self.max;
        object.config.width = self.width;
        object.config.enabled = self.enabled;
        object.config.active = self.active;
        object.config.inactive = self.inactive;
        object.config.thumb = self.thumb;
        object.config.outline = self.outline;
        object.config.position = self.position.clone();
        object.config.divisions = self.divisions;
        object.config.semantics_label = self.semantics_label.clone();
        object.config.on_changed = self.on_changed.clone();
        object.handle = context.handle();
        let _ = object.handle.mark_needs_paint();
    }
}

impl RenderBox for SliderTrackRender {
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        if !children.is_empty() {
            return Err(LayoutError::InvalidConfiguration(
                "slider track cannot have children",
            ));
        }
        let size = constraints.constrain(Vec2::new(self.config.width, 40.0))?;
        let mut geometry = BoxGeometry::new(size);
        geometry.hit_test_self = true;
        Ok(geometry)
    }

    fn dry_layout(
        &self,
        constraints: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<Vec2, LayoutError> {
        if !children.is_empty() {
            return Err(LayoutError::InvalidConfiguration(
                "slider track cannot have children",
            ));
        }
        constraints.constrain(Vec2::new(self.config.width, 40.0))
    }

    fn intrinsic(
        &self,
        dimension: IntrinsicDimension,
        _: f32,
        _: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        Ok(match dimension {
            IntrinsicDimension::MinWidth => 24.0,
            IntrinsicDimension::MaxWidth => self.config.width,
            IntrinsicDimension::MinHeight | IntrinsicDimension::MaxHeight => 40.0,
        })
    }

    fn paint(&mut self, ui: &mut egui::Ui, _: &RenderContext) {
        let bounds = ui.max_rect();
        self.config.position.set(bounds);
        let response = ui.interact(bounds, self.id, egui::Sense::focusable_noninteractive());
        let range = self.config.max - self.config.min;
        let step = self
            .config
            .divisions
            .map_or(range / 100.0, |divisions| range / divisions as f32);
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::Slider);
            if let Some(label) = &self.config.semantics_label {
                node.set_label(label.clone());
            }
            node.set_numeric_value(self.config.value as f64);
            node.set_min_numeric_value(self.config.min as f64);
            node.set_max_numeric_value(self.config.max as f64);
            node.set_numeric_value_step(step as f64);
            if self.config.enabled {
                node.add_action(egui::accesskit::Action::Increment);
                node.add_action(egui::accesskit::Action::Decrement);
                node.add_action(egui::accesskit::Action::SetValue);
            } else {
                node.set_disabled();
            }
        });
        let mut requested_value = None;
        if self.config.enabled {
            ui.input_mut(|input| {
                input.consume_accesskit_action_requests(response.id, |request| {
                    let current = requested_value.unwrap_or(self.config.value);
                    let value = match request.action {
                        egui::accesskit::Action::Increment => Some(current + step),
                        egui::accesskit::Action::Decrement => Some(current - step),
                        egui::accesskit::Action::SetValue => match request.data.as_ref() {
                            Some(egui::accesskit::ActionData::NumericValue(value)) => {
                                Some(*value as f32)
                            }
                            Some(egui::accesskit::ActionData::Value(value)) => {
                                value.parse::<f32>().ok()
                            }
                            _ => None,
                        },
                        _ => None,
                    };
                    if let Some(value) = value.filter(|value| value.is_finite()) {
                        requested_value = Some(value);
                        true
                    } else {
                        false
                    }
                });
            });
            if response.has_focus() {
                let keyboard_value = ui.input_mut(|input| {
                    let mut value = None;
                    for (key, delta) in [
                        (egui::Key::ArrowRight, step),
                        (egui::Key::ArrowUp, step),
                        (egui::Key::ArrowLeft, -step),
                        (egui::Key::ArrowDown, -step),
                        (egui::Key::PageUp, step * 10.0),
                        (egui::Key::PageDown, -step * 10.0),
                    ] {
                        if input.consume_key(egui::Modifiers::NONE, key) {
                            value = Some(value.unwrap_or(self.config.value) + delta);
                        }
                    }
                    if input.consume_key(egui::Modifiers::NONE, egui::Key::Home) {
                        value = Some(self.config.min);
                    }
                    if input.consume_key(egui::Modifiers::NONE, egui::Key::End) {
                        value = Some(self.config.max);
                    }
                    value
                });
                if keyboard_value.is_some() {
                    requested_value = keyboard_value;
                }
            }
        }
        if let Some(mut value) = requested_value {
            value = value.clamp(self.config.min, self.config.max);
            if self.config.divisions.is_some() {
                value = self.config.min + ((value - self.config.min) / step).round() * step;
            }
            if value != self.config.value {
                (self.config.on_changed)(value);
            }
        }
        let start = Pos2::new(bounds.left() + 12.0, bounds.center().y);
        let end = Pos2::new(bounds.right() - 12.0, bounds.center().y);
        let t = ((self.config.value - self.config.min) / (self.config.max - self.config.min))
            .clamp(0.0, 1.0);
        let thumb = Pos2::new(start.x + (end.x - start.x) * t, bounds.center().y);
        let painter = ui.painter();
        painter.line_segment([start, end], Stroke::new(4.0_f32, self.config.inactive));
        painter.line_segment([start, thumb], Stroke::new(4.0_f32, self.config.active));
        painter.circle_filled(thumb, 10.0, self.config.thumb);
        painter.circle_stroke(thumb, 10.0, Stroke::new(1.0_f32, self.config.outline));
    }
}
