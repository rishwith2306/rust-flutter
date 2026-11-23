use super::ThemeData;
use crate::{
    egui::{self, Color32, Vec2},
    element::{BuildContext, StatelessWidget, Widget},
};
use std::time::Duration;

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
