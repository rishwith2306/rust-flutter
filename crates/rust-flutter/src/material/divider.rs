use super::ThemeData;
use crate::{
    egui::{self, Color32, Stroke},
    element::Widget,
};

/// A thin horizontal line with configurable height, thickness and insets.
pub struct Divider {
    height: f32,
    thickness: f32,
    indent: f32,
    end_indent: f32,
    color: Color32,
}

impl Divider {
    pub fn new() -> Self {
        Self {
            height: 16.0,
            thickness: 1.0,
            indent: 0.0,
            end_indent: 0.0,
            color: ThemeData::default().color_scheme.outline_variant,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }
    pub fn indent(mut self, indent: f32) -> Self {
        self.indent = indent;
        self
    }
    pub fn end_indent(mut self, end_indent: f32) -> Self {
        self.end_indent = end_indent;
        self
    }
    pub fn color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.color = theme.color_scheme.outline_variant;
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        validate(self.height, self.thickness, self.indent, self.end_indent)?;
        Ok(Widget::leaf(egui::vec2(0.0, self.height), move |ui| {
            let rect = ui.max_rect();
            let start = rect.left() + self.indent.min(rect.width());
            let end = rect.right() - self.end_indent.min(rect.width());
            if end >= start {
                let y = rect.center().y;
                ui.painter().line_segment(
                    [egui::pos2(start, y), egui::pos2(end, y)],
                    Stroke::new(self.thickness, self.color),
                );
            }
        }))
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::new()
    }
}

/// A thin vertical line with configurable width, thickness and insets.
pub struct VerticalDivider {
    width: f32,
    thickness: f32,
    indent: f32,
    end_indent: f32,
    color: Color32,
}

impl VerticalDivider {
    pub fn new() -> Self {
        Self {
            width: 16.0,
            thickness: 1.0,
            indent: 0.0,
            end_indent: 0.0,
            color: ThemeData::default().color_scheme.outline_variant,
        }
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }
    pub fn indent(mut self, indent: f32) -> Self {
        self.indent = indent;
        self
    }
    pub fn end_indent(mut self, end_indent: f32) -> Self {
        self.end_indent = end_indent;
        self
    }
    pub fn color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    pub fn theme(mut self, theme: ThemeData) -> Self {
        self.color = theme.color_scheme.outline_variant;
        self
    }

    pub fn into_widget(self) -> Result<Widget, &'static str> {
        validate(self.width, self.thickness, self.indent, self.end_indent)?;
        Ok(Widget::leaf(egui::vec2(self.width, 0.0), move |ui| {
            let rect = ui.max_rect();
            let start = rect.top() + self.indent.min(rect.height());
            let end = rect.bottom() - self.end_indent.min(rect.height());
            if end >= start {
                let x = rect.center().x;
                ui.painter().line_segment(
                    [egui::pos2(x, start), egui::pos2(x, end)],
                    Stroke::new(self.thickness, self.color),
                );
            }
        }))
    }
}

impl Default for VerticalDivider {
    fn default() -> Self {
        Self::new()
    }
}

fn validate(extent: f32, thickness: f32, indent: f32, end_indent: f32) -> Result<(), &'static str> {
    if !extent.is_finite()
        || extent < 0.0
        || !thickness.is_finite()
        || thickness < 0.0
        || !indent.is_finite()
        || indent < 0.0
        || !end_indent.is_finite()
        || end_indent < 0.0
    {
        return Err("Material divider dimensions must be finite and nonnegative");
    }
    Ok(())
}
