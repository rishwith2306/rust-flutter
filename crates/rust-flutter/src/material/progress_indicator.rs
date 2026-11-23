use super::{ProgressIndicatorThemeData, StrokeCap, ThemeData};
use crate::animation::AnimationController;
use crate::{
    egui::{self, Color32, Vec2},
    element::{BuildContext, StatelessWidget, Widget},
};
use std::time::Duration;

/// Applies progress indicator defaults and an optional shared controller to a subtree.
pub struct ProgressIndicatorTheme {
    child: Widget,
    data: ProgressIndicatorThemeData,
    controller: Option<AnimationController>,
}

impl ProgressIndicatorTheme {
    /// Creates a subtree theme around `child` with no local overrides.
    pub fn new(child: Widget) -> Self {
        Self {
            child,
            data: ProgressIndicatorThemeData::default(),
            controller: None,
        }
    }

    /// Sets properties that override the nearest app or subtree defaults.
    pub fn data(mut self, data: ProgressIndicatorThemeData) -> Self {
        self.data = data;
        self
    }

    /// Supplies one animation source to indeterminate indicators in this subtree.
    pub fn controller(mut self, controller: AnimationController) -> Self {
        self.controller = Some(controller);
        self
    }

    /// Builds the inherited theme and optional controller providers.
    pub fn into_widget(self) -> Widget {
        let controller = self.controller.or(self.data.controller.clone());
        let child = Widget::inherited(self.data, self.child);
        match controller {
            Some(controller) => Widget::inherited_notifier(Some(controller), child),
            None => child,
        }
    }
}

fn expose_progress_semantics(
    ui: &egui::Ui,
    rect: egui::Rect,
    value: Option<f32>,
    label: &Option<String>,
    semantic_value: &Option<String>,
) {
    if crate::layout::ui_is_offstage(ui) {
        return;
    }
    ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
        node.set_role(egui::accesskit::Role::ProgressIndicator);
        node.set_bounds(egui::accesskit::Rect {
            x0: f64::from(rect.left()),
            y0: f64::from(rect.top()),
            x1: f64::from(rect.right()),
            y1: f64::from(rect.bottom()),
        });
        if let Some(label) = label {
            node.set_label(label.clone());
        } else {
            node.clear_label();
        }
        match value {
            Some(value) => {
                node.set_value(
                    semantic_value
                        .clone()
                        .unwrap_or_else(|| format!("{:.0}%", value * 100.0)),
                );
                node.set_numeric_value(f64::from(value * 100.0));
                node.set_min_numeric_value(0.0);
                node.set_max_numeric_value(100.0);
            }
            None => {
                if let Some(value) = semantic_value {
                    node.set_value(value.clone());
                } else {
                    node.clear_value();
                }
                node.clear_numeric_value();
                node.clear_min_numeric_value();
                node.clear_max_numeric_value();
            }
        }
    });
}

fn cubic_bezier_progress(value: f32, x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let (mut low, mut high) = (0.0, 1.0);
    for _ in 0..16 {
        let t = (low + high) * 0.5;
        let inverse = 1.0 - t;
        let x = 3.0 * inverse * inverse * t * x1 + 3.0 * inverse * t * t * x2 + t * t * t;
        if x < value {
            low = t;
        } else {
            high = t;
        }
    }
    let t = (low + high) * 0.5;
    let inverse = 1.0 - t;
    3.0 * inverse * inverse * t * y1 + 3.0 * inverse * t * t * y2 + t * t * t
}

fn interval_curve(value: f32, start: f32, end: f32, curve: [f32; 4]) -> f32 {
    if value <= start {
        0.0
    } else if value >= end {
        1.0
    } else {
        cubic_bezier_progress(
            (value - start) / (end - start),
            curve[0],
            curve[1],
            curve[2],
            curve[3],
        )
    }
}

// Motion intervals and cubic curves follow Flutter's Material implementation:
// https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/material/progress_indicator.dart
fn indeterminate_linear_segments(phase: f32) -> [(f32, f32); 2] {
    let head1 = interval_curve(phase, 0.0, 750.0 / 1800.0, [0.2, 0.0, 0.8, 1.0]);
    let tail1 = interval_curve(phase, 333.0 / 1800.0, 1083.0 / 1800.0, [0.4, 0.0, 1.0, 1.0]);
    let head2 = interval_curve(
        phase,
        1000.0 / 1800.0,
        1567.0 / 1800.0,
        [0.0, 0.0, 0.65, 1.0],
    );
    let tail2 = interval_curve(phase, 1267.0 / 1800.0, 1.0, [0.1, 0.0, 0.45, 1.0]);
    [(tail1, head1), (tail2, head2)]
}

/// Themed Material circular progress indicator with determinate or indeterminate progress.
#[derive(Clone)]
pub struct CircularProgressIndicator {
    value: Option<f32>,
    color: Option<Color32>,
    background_color: Option<Color32>,
    stroke_width: Option<f32>,
    stroke_align: Option<f32>,
    stroke_cap: Option<StrokeCap>,
    track_gap: Option<f32>,
    size: f32,
    semantics_label: Option<String>,
    semantics_value: Option<String>,
    controller: Option<AnimationController>,
}

impl CircularProgressIndicator {
    pub fn new(value: Option<f32>) -> Self {
        Self {
            value,
            color: None,
            background_color: None,
            stroke_width: None,
            stroke_align: None,
            stroke_cap: None,
            track_gap: None,
            size: 40.0,
            semantics_label: None,
            semantics_value: None,
            controller: None,
        }
    }

    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn background_color(mut self, color: Color32) -> Self {
        self.background_color = Some(color);
        self
    }

    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = Some(width);
        self
    }

    /// Sets the stroke position from inside (-1) through centered (0) to outside (1).
    pub fn stroke_align(mut self, alignment: f32) -> Self {
        self.stroke_align = Some(alignment);
        self
    }

    /// Sets the end shape of the circular progress arc.
    pub fn stroke_cap(mut self, cap: StrokeCap) -> Self {
        self.stroke_cap = Some(cap);
        self
    }

    /// Sets the gap between the active arc and the remaining circular track.
    pub fn track_gap(mut self, gap: f32) -> Self {
        self.track_gap = Some(gap);
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Drives this indeterminate indicator from a shared controller.
    pub fn controller(mut self, controller: AnimationController) -> Self {
        self.controller = Some(controller);
        self
    }

    /// Sets the screen reader label where the host exposes AccessKit semantics.
    pub fn semantics_label(mut self, label: impl Into<String>) -> Self {
        self.semantics_label = Some(label.into());
        self
    }

    /// Overrides the announced value; determinate progress defaults to a percentage.
    pub fn semantics_value(mut self, value: impl Into<String>) -> Self {
        self.semantics_value = Some(value.into());
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if self.value.is_some() && self.controller.is_some() {
            return Err("progress indicator cannot set both a value and a controller");
        }
        if self
            .controller
            .as_ref()
            .is_some_and(AnimationController::is_disposed)
        {
            return Err("progress indicator controller has been disposed");
        }
        if self
            .stroke_width
            .is_some_and(|width| !width.is_finite() || width <= 0.0 || width > self.size)
            || self
                .stroke_align
                .is_some_and(|align| !align.is_finite() || !(-1.0..=1.0).contains(&align))
            || self
                .track_gap
                .is_some_and(|gap| !gap.is_finite() || gap < 0.0)
            || !self.size.is_finite()
            || self.size <= 0.0
            || self
                .value
                .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            return Err("progress size and stroke must be positive and value must be in 0..=1");
        }
        Ok(Widget::stateless(CircularProgressView(self)))
    }
}

struct CircularProgressView(CircularProgressIndicator);

impl StatelessWidget for CircularProgressView {
    fn build(&self, context: &BuildContext) -> Widget {
        if let Some(controller) = self.0.controller.clone() {
            let indicator = self.0.clone();
            let mut indicator = indicator;
            indicator.controller = None;
            return Widget::inherited_notifier(
                Some(controller),
                Widget::stateless(CircularProgressView(indicator)),
            );
        }
        let animation_controller = if self.0.value.is_none() {
            context
                .depend_on::<crate::element::InheritedNotifier<AnimationController>>()
                .expect("progress indicators resolve inherited animation controllers")
                .and_then(|provider| provider.notifier.clone())
        } else {
            None
        };
        let theme = ThemeData::of(context)
            .expect("Material progress indicators resolve theme during build");
        let colors = theme.color_scheme;
        let default_progress_theme = theme.progress_indicator_theme.clone();
        let progress_theme = context
            .depend_on::<ProgressIndicatorThemeData>()
            .expect("progress indicators resolve inherited theme data")
            .map_or_else(
                || default_progress_theme.clone(),
                |local| local.as_ref().clone().merge(default_progress_theme.clone()),
            );
        let indicator = &self.0;
        let value = indicator.value;
        let semantics_label = indicator.semantics_label.clone();
        let semantics_value = indicator.semantics_value.clone();
        let animation_phase = animation_controller.map(|controller| controller.normalized_value());
        let size = indicator.size;
        let stroke_width = indicator
            .stroke_width
            .or(progress_theme
                .stroke_width
                .filter(|width| width.is_finite() && *width > 0.0))
            .unwrap_or(4.0)
            .min(size);
        let stroke_align = indicator
            .stroke_align
            .or(progress_theme
                .stroke_align
                .filter(|align| align.is_finite() && (-1.0..=1.0).contains(align)))
            .unwrap_or(-1.0);
        let stroke_cap = indicator
            .stroke_cap
            .or(progress_theme.stroke_cap)
            .unwrap_or(StrokeCap::Round);
        let track_gap = indicator
            .track_gap
            .or(progress_theme.track_gap)
            .filter(|gap| gap.is_finite() && *gap >= 0.0)
            .unwrap_or(0.0);
        let color = indicator
            .color
            .or(progress_theme.color)
            .unwrap_or(colors.primary);
        let background = indicator
            .background_color
            .or(progress_theme.circular_track_color)
            .unwrap_or(colors.surface_container_highest);
        Widget::measured(
            move |constraints| constraints.constrain(Vec2::splat(size)),
            move |ui| {
                let rect = ui.max_rect();
                expose_progress_semantics(ui, rect, value, &semantics_label, &semantics_value);
                let center = rect.center();
                let radius =
                    rect.width().min(rect.height()) * 0.5 + stroke_width * stroke_align * 0.5;
                if radius <= 0.0 {
                    return;
                }
                let (start, sweep) = if let Some(value) = value {
                    (-std::f32::consts::FRAC_PI_2, std::f32::consts::TAU * value)
                } else {
                    let phase = animation_phase.unwrap_or_else(|| {
                        let duration = 1.25_f64;
                        let phase =
                            ui.ctx().input(|input| input.time).rem_euclid(duration) / duration;
                        ui.ctx().request_repaint_after(Duration::from_millis(16));
                        phase as f32
                    });
                    (
                        -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * phase,
                        std::f32::consts::TAU * 0.28,
                    )
                };
                let track_stroke = egui::Stroke::new(stroke_width, background);
                if track_gap > 0.0 && value.is_some_and(|value| value > 0.0) {
                    // Match Flutter's Material gap sizing: separate the active
                    // arc from the complementary track by the stroke radius
                    // plus the requested logical-pixel gap.
                    let start_gap = (stroke_width + track_gap) / radius;
                    let end_gap = start_gap * 2.0;
                    let track_start = start + sweep + start_gap;
                    let track_sweep = (std::f32::consts::TAU - sweep - end_gap).max(0.0);
                    if track_sweep > 0.0 {
                        let track_segments =
                            ((track_sweep * radius / 2.0).ceil() as usize).clamp(2, 192);
                        for index in 0..track_segments {
                            let from =
                                track_start + track_sweep * index as f32 / track_segments as f32;
                            let to = track_start
                                + track_sweep * (index + 1) as f32 / track_segments as f32;
                            let point =
                                |angle: f32| center + Vec2::new(angle.cos(), angle.sin()) * radius;
                            ui.painter()
                                .line_segment([point(from), point(to)], track_stroke);
                        }
                    }
                } else {
                    ui.painter().circle_stroke(center, radius, track_stroke);
                }
                if sweep <= 0.0 {
                    return;
                }
                let segments = ((sweep * radius / 2.0).ceil() as usize).clamp(2, 96);
                let point = |angle: f32| center + Vec2::new(angle.cos(), angle.sin()) * radius;
                let stroke = egui::Stroke::new(stroke_width, color);
                let tangent = |angle: f32| Vec2::new(-angle.sin(), angle.cos());
                let start_point = point(start);
                let end_point = point(start + sweep);
                if stroke_cap == StrokeCap::Square {
                    let extension = stroke_width * 0.5;
                    ui.painter().line_segment(
                        [start_point - tangent(start) * extension, start_point],
                        stroke,
                    );
                    ui.painter().line_segment(
                        [end_point, end_point + tangent(start + sweep) * extension],
                        stroke,
                    );
                }
                for index in 0..segments {
                    let from = start + sweep * index as f32 / segments as f32;
                    let to = start + sweep * (index + 1) as f32 / segments as f32;
                    ui.painter().line_segment([point(from), point(to)], stroke);
                }
                if stroke_cap == StrokeCap::Round {
                    ui.painter()
                        .circle_filled(start_point, stroke_width * 0.5, color);
                    ui.painter()
                        .circle_filled(end_point, stroke_width * 0.5, color);
                }
            },
        )
    }
}

/// Themed Material linear progress indicator with determinate or indeterminate progress.
#[derive(Clone)]
pub struct LinearProgressIndicator {
    value: Option<f32>,
    color: Option<Color32>,
    background_color: Option<Color32>,
    min_height: Option<f32>,
    border_radius: Option<f32>,
    track_gap: Option<f32>,
    stop_indicator_radius: Option<f32>,
    stop_indicator_color: Option<Color32>,
    semantics_label: Option<String>,
    semantics_value: Option<String>,
    controller: Option<AnimationController>,
}

impl LinearProgressIndicator {
    pub fn new(value: Option<f32>) -> Self {
        Self {
            value,
            color: None,
            background_color: None,
            min_height: None,
            border_radius: None,
            track_gap: None,
            stop_indicator_radius: None,
            stop_indicator_color: None,
            semantics_label: None,
            semantics_value: None,
            controller: None,
        }
    }

    pub fn color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn background_color(mut self, color: Color32) -> Self {
        self.background_color = Some(color);
        self
    }

    pub fn min_height(mut self, height: f32) -> Self {
        self.min_height = Some(height);
        self
    }

    /// Sets the corner radius for the track and active segment.
    pub fn border_radius(mut self, radius: f32) -> Self {
        self.border_radius = Some(radius);
        self
    }

    /// Sets the gap between the active segment and remaining track.
    pub fn track_gap(mut self, gap: f32) -> Self {
        self.track_gap = Some(gap);
        self
    }

    /// Adds a circular completion marker at the end of the track.
    pub fn stop_indicator_radius(mut self, radius: f32) -> Self {
        self.stop_indicator_radius = Some(radius);
        self
    }

    /// Sets the completion marker color; defaults to the indicator color.
    pub fn stop_indicator_color(mut self, color: Color32) -> Self {
        self.stop_indicator_color = Some(color);
        self
    }

    /// Drives this indeterminate indicator from a shared controller.
    pub fn controller(mut self, controller: AnimationController) -> Self {
        self.controller = Some(controller);
        self
    }

    /// Sets the screen reader label where the host exposes AccessKit semantics.
    pub fn semantics_label(mut self, label: impl Into<String>) -> Self {
        self.semantics_label = Some(label.into());
        self
    }

    /// Overrides the announced value; determinate progress defaults to a percentage.
    pub fn semantics_value(mut self, value: impl Into<String>) -> Self {
        self.semantics_value = Some(value.into());
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if self.value.is_some() && self.controller.is_some() {
            return Err("progress indicator cannot set both a value and a controller");
        }
        if self
            .controller
            .as_ref()
            .is_some_and(AnimationController::is_disposed)
        {
            return Err("progress indicator controller has been disposed");
        }
        if self
            .min_height
            .is_some_and(|height| !height.is_finite() || height <= 0.0)
            || self
                .track_gap
                .is_some_and(|gap| !gap.is_finite() || gap < 0.0)
            || self
                .border_radius
                .is_some_and(|radius| !radius.is_finite() || radius < 0.0)
            || self
                .stop_indicator_radius
                .is_some_and(|radius| !radius.is_finite() || radius < 0.0)
            || self
                .value
                .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        {
            return Err("progress height must be positive and value must be in 0..=1");
        }
        Ok(Widget::stateless(LinearProgressView(self)))
    }
}

struct LinearProgressView(LinearProgressIndicator);

impl StatelessWidget for LinearProgressView {
    fn build(&self, context: &BuildContext) -> Widget {
        if let Some(controller) = self.0.controller.clone() {
            let indicator = self.0.clone();
            let mut indicator = indicator;
            indicator.controller = None;
            return Widget::inherited_notifier(
                Some(controller),
                Widget::stateless(LinearProgressView(indicator)),
            );
        }
        let animation_controller = if self.0.value.is_none() {
            context
                .depend_on::<crate::element::InheritedNotifier<AnimationController>>()
                .expect("progress indicators resolve inherited animation controllers")
                .and_then(|provider| provider.notifier.clone())
        } else {
            None
        };
        let theme = ThemeData::of(context)
            .expect("Material progress indicators resolve theme during build");
        let colors = theme.color_scheme;
        let default_progress_theme = theme.progress_indicator_theme.clone();
        let progress_theme = context
            .depend_on::<ProgressIndicatorThemeData>()
            .expect("progress indicators resolve inherited theme data")
            .map_or_else(
                || default_progress_theme.clone(),
                |local| local.as_ref().clone().merge(default_progress_theme.clone()),
            );
        let indicator = &self.0;
        let value = indicator.value;
        let semantics_label = indicator.semantics_label.clone();
        let semantics_value = indicator.semantics_value.clone();
        let animation_phase = animation_controller.map(|controller| controller.normalized_value());
        let height = indicator
            .min_height
            .or(progress_theme
                .linear_min_height
                .filter(|height| height.is_finite() && *height > 0.0))
            .unwrap_or(4.0);
        let border_radius = indicator.border_radius.or(progress_theme
            .border_radius
            .filter(|radius| radius.is_finite() && *radius >= 0.0));
        let track_gap = indicator
            .track_gap
            .or(progress_theme
                .track_gap
                .filter(|gap| gap.is_finite() && *gap >= 0.0))
            .unwrap_or(0.0);
        let stop_radius = indicator.stop_indicator_radius.or(progress_theme
            .stop_indicator_radius
            .filter(|radius| radius.is_finite() && *radius >= 0.0));
        let stop_color = indicator
            .stop_indicator_color
            .or(progress_theme.stop_indicator_color);
        let color = indicator
            .color
            .or(progress_theme.color)
            .unwrap_or(colors.primary);
        let background = indicator
            .background_color
            .or(progress_theme.linear_track_color)
            .unwrap_or(colors.surface_container_highest);
        Widget::measured(
            move |constraints| {
                let max_width = constraints.max().x;
                let width = if max_width.is_finite() {
                    max_width
                } else {
                    0.0
                };
                constraints.constrain(Vec2::new(width, height))
            },
            move |ui| {
                let rect = ui.max_rect();
                expose_progress_semantics(ui, rect, value, &semantics_label, &semantics_value);
                let radius = egui::CornerRadius::same(
                    border_radius
                        .unwrap_or(height * 0.5)
                        .round()
                        .clamp(0.0, height * 0.5)
                        .clamp(0.0, 255.0) as u8,
                );
                let Some(value) = value else {
                    let phase = animation_phase.unwrap_or_else(|| {
                        let duration = 1.8_f64;
                        let phase =
                            ui.ctx().input(|input| input.time).rem_euclid(duration) / duration;
                        ui.ctx().request_repaint_after(Duration::from_millis(16));
                        phase as f32
                    });
                    let segments = indeterminate_linear_segments(phase);
                    let gap_fraction = if rect.width() > 0.0 {
                        (track_gap / rect.width()).min(1.0)
                    } else {
                        0.0
                    };
                    let effective_gap =
                        |fraction: f32| gap_fraction * fraction.clamp(0.0, 0.01) / 0.01;
                    let draw = |start: f32, end: f32, fill: Color32| {
                        let start = start.clamp(0.0, 1.0);
                        let end = end.clamp(0.0, 1.0);
                        if end <= start {
                            return;
                        }
                        let segment = egui::Rect::from_min_max(
                            egui::pos2(rect.left() + rect.width() * start, rect.top()),
                            egui::pos2(rect.left() + rect.width() * end, rect.bottom()),
                        );
                        ui.painter().rect_filled(segment, radius, fill);
                    };
                    let (tail1, head1) = segments[0];
                    let (tail2, head2) = segments[1];
                    if head1 < 1.0 - gap_fraction {
                        let start = if head1 > 0.0 {
                            head1 + effective_gap(head1)
                        } else {
                            0.0
                        };
                        draw(start, 1.0, background);
                    }
                    if tail1 < head1 {
                        draw(tail1, head1, color);
                    }
                    if tail1 > gap_fraction {
                        let start = if head2 > 0.0 {
                            head2 + effective_gap(head2)
                        } else {
                            0.0
                        };
                        let end = if tail1 < 1.0 {
                            tail1 - effective_gap(1.0 - tail1)
                        } else {
                            1.0
                        };
                        draw(start, end, background);
                    }
                    if tail2 < head2 {
                        draw(tail2, head2, color);
                    }
                    if tail2 > gap_fraction {
                        let end = if tail2 < 1.0 {
                            tail2 - effective_gap(1.0 - tail2)
                        } else {
                            1.0
                        };
                        draw(0.0, end, background);
                    }
                    return;
                };
                let gap = if value <= 0.0 {
                    0.0
                } else {
                    track_gap.min(rect.width())
                };
                let track_start = (rect.left() + rect.width() * value + gap).min(rect.right());
                if track_start < rect.right() {
                    ui.painter().rect_filled(
                        egui::Rect::from_min_max(egui::pos2(track_start, rect.top()), rect.max),
                        radius,
                        background,
                    );
                }
                if let Some(stop_radius) = stop_radius.filter(|radius| *radius > 0.0) {
                    ui.painter().circle_filled(
                        egui::pos2(rect.right() - height * 0.5, rect.center().y),
                        stop_radius.min(height * 0.5),
                        stop_color.unwrap_or(color),
                    );
                }
                if value > 0.0 {
                    let fill = egui::Rect::from_min_max(
                        rect.min,
                        egui::pos2(rect.left() + rect.width() * value, rect.bottom()),
                    );
                    ui.painter().rect_filled(fill, radius, color);
                }
            },
        )
    }
}
