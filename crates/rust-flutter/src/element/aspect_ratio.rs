use super::{RenderBox, RenderContext, RenderObjectWidget, Widget};
use crate::{egui, layout::*};
/// Fits a preferred width/height ratio within parent constraints, then gives
/// that exact size to its child. Conflicting parent limits take precedence.
#[derive(Clone, Copy, Debug)]
pub struct AspectRatio {
    pub aspect_ratio: f32,
}
impl AspectRatio {
    pub fn new(aspect_ratio: f32) -> Result<Self, LayoutError> {
        let value = Self { aspect_ratio };
        value.validate()?;
        Ok(value)
    }
    pub fn into_widget(self, child: Widget) -> Widget {
        Widget::render_object(self, vec![child])
    }
    fn validate(self) -> Result<(), LayoutError> {
        if self.aspect_ratio.is_finite() && self.aspect_ratio > 0.0 {
            Ok(())
        } else {
            Err(LayoutError::InvalidConfiguration(
                "aspect ratio must be positive and finite",
            ))
        }
    }
    fn size(self, c: BoxConstraints) -> Result<egui::Vec2, LayoutError> {
        self.validate()?;
        if c.is_tight() {
            let size = c.min();
            return if size.is_finite() {
                Ok(size)
            } else {
                Err(LayoutError::UnboundedSize)
            };
        }
        let min = c.min();
        let max = c.max();
        let r = f64::from(self.aspect_ratio);
        if !max.x.is_finite() && !max.y.is_finite() {
            return Err(LayoutError::UnboundedSize);
        }
        let mut w = f64::from(max.x);
        let mut h = w / r;
        if !w.is_finite() {
            h = f64::from(max.y);
            w = h * r;
        }
        if w > f64::from(max.x) {
            w = f64::from(max.x);
            h = w / r;
        }
        if h > f64::from(max.y) {
            h = f64::from(max.y);
            w = h * r;
        }
        if w < f64::from(min.x) {
            w = f64::from(min.x);
            h = w / r;
        }
        if h < f64::from(min.y) {
            h = f64::from(min.y);
            w = h * r;
        }
        let size = c.constrain(egui::vec2(w as f32, h as f32))?;
        if !size.is_finite() {
            return Err(LayoutError::UnboundedSize);
        }
        Ok(size)
    }
    fn single(self, n: usize) -> Result<(), LayoutError> {
        self.validate()?;
        if n == 1 {
            Ok(())
        } else {
            Err(LayoutError::InvalidConfiguration(
                "AspectRatio requires one child",
            ))
        }
    }
}
impl Widget {
    pub fn aspect_ratio(ratio: f32, child: Widget) -> Result<Self, LayoutError> {
        Ok(AspectRatio::new(ratio)?.into_widget(child))
    }
}
impl RenderObjectWidget for AspectRatio {
    type Object = Self;
    fn create_render_object(&self, _: &RenderContext) -> Self {
        *self
    }
    fn update_render_object(&self, object: &mut Self, _: &RenderContext) {
        *object = *self;
    }
}
impl RenderBox for AspectRatio {
    fn dry_layout(
        &self,
        c: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<egui::Vec2, LayoutError> {
        self.single(children.len())?;
        self.size(c)
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        extent: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        self.single(children.len())?;
        if extent.is_finite() {
            let value = if d.axis() == Axis::Horizontal {
                f64::from(extent) * f64::from(self.aspect_ratio)
            } else {
                f64::from(extent) / f64::from(self.aspect_ratio)
            } as f32;
            if !value.is_finite() || value < 0.0 {
                return Err(LayoutError::InvalidSize);
            }
            Ok(value)
        } else {
            children.intrinsic(0, d, extent)
        }
    }
    fn dry_baseline(
        &self,
        c: BoxConstraints,
        k: TextBaseline,
        children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        self.single(children.len())?;
        children.dry_baseline(0, BoxConstraints::tight(self.size(c)?)?, k)
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        self.single(children.len())?;
        let size = self.size(c)?;
        children.layout(0, BoxConstraints::tight(size)?)?;
        children.place(0, egui::Vec2::ZERO)?;
        let mut geometry = BoxGeometry::new(size);
        geometry.baselines = Baselines {
            alphabetic: children.baseline(0, TextBaseline::Alphabetic, true)?,
            ideographic: children.baseline(0, TextBaseline::Ideographic, true)?,
        };
        Ok(geometry)
    }
}
