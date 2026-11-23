use super::ThemeData;
use crate::{
    egui::{self, Color32, Vec2},
    element::{BuildContext, StatelessWidget, Widget},
};
use std::time::Duration;

/// Themed Material circular progress indicator with determinate or indeterminate progress.
pub struct CircularProgressIndicator {
    value: Option<f32>,
    color: Option<Color32>,
    background_color: Option<Color32>,
    stroke_width: f32,
    size: f32,
}

impl CircularProgressIndicator {
    pub fn new(value: Option<f32>) -> Self {
        Self {
            value,
            color: None,
            background_color: None,
            stroke_width: 4.0,
            size: 40.0,
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
        self.stroke_width = width;
        self
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if !self.stroke_width.is_finite()
            || self.stroke_width <= 0.0
            || !self.size.is_finite()
            || self.size <= 0.0
            || self.stroke_width > self.size
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
        let colors = ThemeData::of(context)
            .expect("Material progress indicators resolve theme during build")
            .color_scheme;
        let indicator = &self.0;
        let value = indicator.value;
        let size = indicator.size;
        let stroke_width = indicator.stroke_width;
        let color = indicator.color.unwrap_or(colors.primary);
        let background = indicator
            .background_color
            .unwrap_or(colors.surface_container_highest);
        Widget::measured(
            move |constraints| constraints.constrain(Vec2::splat(size)),
            move |ui| {
                let rect = ui.max_rect();
                let center = rect.center();
                let radius = (rect.width().min(rect.height()) - stroke_width) * 0.5;
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
                for index in 0..segments {
                    let from = start + sweep * index as f32 / segments as f32;
                    let to = start + sweep * (index + 1) as f32 / segments as f32;
                    ui.painter().line_segment([point(from), point(to)], stroke);
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
    min_height: f32,
}

impl LinearProgressIndicator {
    pub fn new(value: Option<f32>) -> Self {
        Self {
            value,
            color: None,
            background_color: None,
            min_height: 4.0,
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
        self.min_height = height;
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        if !self.min_height.is_finite()
            || self.min_height <= 0.0
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
        let colors = ThemeData::of(context)
            .expect("Material progress indicators resolve theme during build")
            .color_scheme;
        let indicator = &self.0;
        let value = indicator.value;
        let height = indicator.min_height;
        let color = indicator.color.unwrap_or(colors.primary);
        let background = indicator
            .background_color
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
                let radius =
                    egui::CornerRadius::same((height * 0.5).round().clamp(0.0, 255.0) as u8);
                ui.painter().rect_filled(rect, radius, background);
                let Some(value) = value else {
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
