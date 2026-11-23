use super::{StrokeCap, ThemeData};
use crate::{
    egui::{self, Color32, Vec2},
    element::{BuildContext, StatelessWidget, Widget},
};
use std::time::Duration;

#[cfg(not(target_os = "android"))]
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

#[cfg(target_os = "android")]
fn expose_progress_semantics(
    _ui: &egui::Ui,
    _rect: egui::Rect,
    _value: Option<f32>,
    _label: &Option<String>,
    _semantic_value: &Option<String>,
) {
}

/// Themed Material circular progress indicator with determinate or indeterminate progress.
pub struct CircularProgressIndicator {
    value: Option<f32>,
    color: Option<Color32>,
    background_color: Option<Color32>,
    stroke_width: Option<f32>,
    stroke_align: Option<f32>,
    stroke_cap: Option<StrokeCap>,
    size: f32,
    semantics_label: Option<String>,
    semantics_value: Option<String>,
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
            size: 40.0,
            semantics_label: None,
            semantics_value: None,
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

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
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
        if self
            .stroke_width
            .is_some_and(|width| !width.is_finite() || width <= 0.0 || width > self.size)
            || self
                .stroke_align
                .is_some_and(|align| !align.is_finite() || !(-1.0..=1.0).contains(&align))
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
        let theme = ThemeData::of(context)
            .expect("Material progress indicators resolve theme during build");
        let colors = theme.color_scheme;
        let progress_theme = theme.progress_indicator_theme;
        let indicator = &self.0;
        let value = indicator.value;
        let semantics_label = indicator.semantics_label.clone();
        let semantics_value = indicator.semantics_value.clone();
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
                ui.painter().circle_stroke(
                    center,
                    radius,
                    egui::Stroke::new(stroke_width, background),
                );
                let (start, sweep) = if let Some(value) = value {
                    (-std::f32::consts::FRAC_PI_2, std::f32::consts::TAU * value)
                } else {
                    let duration = 1.25_f64;
                    let phase = ui.ctx().input(|input| input.time).rem_euclid(duration) / duration;
                    ui.ctx().request_repaint_after(Duration::from_millis(16));
                    (
                        -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * phase as f32,
                        std::f32::consts::TAU * 0.28,
                    )
                };
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
        let theme = ThemeData::of(context)
            .expect("Material progress indicators resolve theme during build");
        let colors = theme.color_scheme;
        let progress_theme = theme.progress_indicator_theme;
        let indicator = &self.0;
        let value = indicator.value;
        let semantics_label = indicator.semantics_label.clone();
        let semantics_value = indicator.semantics_value.clone();
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
                    ui.painter().rect_filled(rect, radius, background);
                    let duration = 1.8_f64;
                    let phase = ui.ctx().input(|input| input.time).rem_euclid(duration) / duration;
                    let segment_width = rect.width() * 0.3;
                    let left =
                        rect.left() + (rect.width() + segment_width) * phase as f32 - segment_width;
                    let segment = egui::Rect::from_min_size(
                        egui::pos2(left, rect.top()),
                        Vec2::new(segment_width, rect.height()),
                    );
                    ui.painter()
                        .with_clip_rect(rect)
                        .rect_filled(segment, radius, color);
                    ui.ctx().request_repaint_after(Duration::from_millis(16));
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
