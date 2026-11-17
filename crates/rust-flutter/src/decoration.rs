//! Retained box decoration. Decoration never adds padding or clips its child.
use crate::{
    element::{RenderBox, RenderContext, RenderObjectWidget, Widget},
    layout::*,
};
use egui::{Color32, CornerRadius, Stroke, StrokeKind, Vec2};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DecorationPosition {
    #[default]
    Background,
    Foreground,
}
/// Solid rounded fill and a uniform inside border. Corner radii are backend u8
/// logical-point values; rounding affects paint only, not clipping/hit geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxDecoration {
    pub color: Color32,
    pub border: Stroke,
    pub corner_radius: CornerRadius,
}
impl Default for BoxDecoration {
    fn default() -> Self {
        Self {
            color: Color32::TRANSPARENT,
            border: Stroke::NONE,
            corner_radius: CornerRadius::ZERO,
        }
    }
}
impl BoxDecoration {
    pub fn solid(color: Color32) -> Self {
        Self {
            color,
            ..Self::default()
        }
    }
    fn validate(self) -> Result<(), LayoutError> {
        if !self.border.width.is_finite() || self.border.width < 0.0 {
            return Err(LayoutError::InvalidConfiguration(
                "decoration border width must be finite and nonnegative",
            ));
        }
        Ok(())
    }
    fn paint(self, ui: &mut egui::Ui) {
        ui.painter().rect(
            ui.max_rect(),
            self.corner_radius,
            self.color,
            self.border,
            StrokeKind::Inside,
        );
    }
}
/// Single-child decoration; the child's layout, intrinsic sizes and baselines
/// pass through unchanged. Use padding separately to reserve border space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DecoratedBox {
    pub decoration: BoxDecoration,
    pub position: DecorationPosition,
}
impl DecoratedBox {
    pub fn new(decoration: BoxDecoration) -> Self {
        Self {
            decoration,
            ..Self::default()
        }
    }
    fn validate(&self, children: usize) -> Result<(), LayoutError> {
        self.decoration.validate()?;
        if children != 1 {
            return Err(LayoutError::InvalidConfiguration(
                "DecoratedBox requires exactly one child",
            ));
        }
        Ok(())
    }
    pub fn into_widget(self, child: Widget) -> Widget {
        Widget::render_object(self, vec![child])
    }
}
impl RenderObjectWidget for DecoratedBox {
    type Object = Self;
    fn create_render_object(&self, _: &RenderContext) -> Self {
        *self
    }
    fn update_render_object(&self, object: &mut Self, _: &RenderContext) {
        *object = *self;
    }
}
impl RenderBox for DecoratedBox {
    fn dry_layout(
        &self,
        c: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<Vec2, LayoutError> {
        self.validate(children.len())?;
        children.dry_layout(0, c)
    }
    fn dry_baseline(
        &self,
        c: BoxConstraints,
        kind: TextBaseline,
        children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        self.validate(children.len())?;
        children.dry_baseline(0, c, kind)
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        extent: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        self.validate(children.len())?;
        children.intrinsic(0, d, extent)
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        self.validate(children.len())?;
        let size = children.layout(0, c)?;
        children.place(0, Vec2::ZERO)?;
        let mut geometry = BoxGeometry::new(size);
        geometry.baselines = Baselines {
            alphabetic: children.baseline(0, TextBaseline::Alphabetic, true)?,
            ideographic: children.baseline(0, TextBaseline::Ideographic, true)?,
        };
        Ok(geometry)
    }
    fn paint(&mut self, ui: &mut egui::Ui, _: &RenderContext) {
        if self.position == DecorationPosition::Background {
            self.decoration.paint(ui);
        }
    }
    fn paint_foreground(&mut self, ui: &mut egui::Ui, _: &RenderContext) {
        if self.position == DecorationPosition::Foreground {
            self.decoration.paint(ui);
        }
    }
}
