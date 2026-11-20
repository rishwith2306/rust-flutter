use super::{RenderBox, RenderContext, RenderObjectWidget, Widget};
use crate::{egui, layout::*};

/// Gives a child tight dimensions computed from the parent's maximum extents.
/// Unspecified axes pass through. The parent still constrains this box's size;
/// the child can overflow it without clipping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FractionallySizedBox {
    pub width_factor: Option<f32>,
    pub height_factor: Option<f32>,
    pub alignment: Alignment,
}
impl Default for FractionallySizedBox {
    fn default() -> Self {
        Self {
            width_factor: None,
            height_factor: None,
            alignment: Alignment::CENTER,
        }
    }
}
impl FractionallySizedBox {
    pub fn new(width_factor: Option<f32>, height_factor: Option<f32>) -> Result<Self, LayoutError> {
        let value = Self {
            width_factor,
            height_factor,
            ..Self::default()
        };
        value.validate(0)?;
        Ok(value)
    }
    pub fn into_widget(self, child: Widget) -> Widget {
        Widget::render_object(self, vec![child])
    }
    fn validate(self, count: usize) -> Result<(), LayoutError> {
        if self
            .width_factor
            .iter()
            .chain(self.height_factor.iter())
            .any(|f| !f.is_finite() || *f < 0.0)
        {
            return Err(LayoutError::InvalidConfiguration(
                "fractional size factors must be finite and nonnegative",
            ));
        }
        self.alignment.offset(egui::Vec2::ZERO)?;
        if count > 1 {
            return Err(LayoutError::InvalidConfiguration(
                "FractionallySizedBox accepts at most one child",
            ));
        }
        Ok(())
    }
    fn constraints(self, c: BoxConstraints) -> Result<BoxConstraints, LayoutError> {
        let mut min = c.min();
        let mut max = c.max();
        for (axis, factor) in [self.width_factor, self.height_factor]
            .into_iter()
            .enumerate()
        {
            if let Some(factor) = factor {
                let value = (f64::from(max[axis]) * f64::from(factor)) as f32;
                if !value.is_finite() {
                    return Err(LayoutError::UnboundedSize);
                }
                min[axis] = value;
                max[axis] = value;
            }
        }
        BoxConstraints::new(min, max)
    }
    fn geometry(
        self,
        c: BoxConstraints,
        child: egui::Vec2,
    ) -> Result<(egui::Vec2, egui::Vec2), LayoutError> {
        let size = c.constrain(child)?;
        Ok((size, self.alignment.offset(size - child)?))
    }
    fn empty_size(self, c: BoxConstraints) -> Result<egui::Vec2, LayoutError> {
        c.constrain(self.constraints(c)?.min())
    }
}
impl Widget {
    pub fn fractionally_sized_box(
        width_factor: Option<f32>,
        height_factor: Option<f32>,
        child: Widget,
    ) -> Result<Self, LayoutError> {
        Ok(FractionallySizedBox::new(width_factor, height_factor)?.into_widget(child))
    }
}
impl RenderObjectWidget for FractionallySizedBox {
    type Object = Self;
    fn create_render_object(&self, _: &RenderContext) -> Self {
        *self
    }
    fn update_render_object(&self, object: &mut Self, _: &RenderContext) {
        *object = *self;
    }
}
impl RenderBox for FractionallySizedBox {
    fn dry_layout(
        &self,
        c: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<egui::Vec2, LayoutError> {
        self.validate(children.len())?;
        if children.is_empty() {
            self.empty_size(c)
        } else {
            c.constrain(children.dry_layout(0, self.constraints(c)?)?)
        }
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        extent: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        self.validate(children.len())?;
        let (factor, opposite) = if d.axis() == Axis::Horizontal {
            (self.width_factor, self.height_factor)
        } else {
            (self.height_factor, self.width_factor)
        };
        let result = if children.is_empty() {
            0.0
        } else {
            let argument = (f64::from(extent) * f64::from(opposite.unwrap_or(1.0))) as f32;
            if argument.is_nan() || (extent.is_finite() && !argument.is_finite()) {
                return Err(LayoutError::InvalidSize);
            }
            children.intrinsic(0, d, argument)?
        } / factor.unwrap_or(1.0);
        if result.is_finite() {
            Ok(result)
        } else {
            Err(LayoutError::InvalidSize)
        }
    }
    fn dry_baseline(
        &self,
        c: BoxConstraints,
        kind: TextBaseline,
        children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        self.validate(children.len())?;
        if children.is_empty() {
            return Ok(None);
        }
        let inner = self.constraints(c)?;
        let Some(baseline) = children.dry_baseline(0, inner, kind)? else {
            return Ok(None);
        };
        let (_, offset) = self.geometry(c, children.dry_layout(0, inner)?)?;
        let value = baseline + offset.y;
        if value.is_finite() {
            Ok(Some(value))
        } else {
            Err(LayoutError::InvalidSize)
        }
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        self.validate(children.len())?;
        if children.is_empty() {
            return Ok(BoxGeometry::new(self.empty_size(c)?));
        }
        let child = children.layout(0, self.constraints(c)?)?;
        let (size, offset) = self.geometry(c, child)?;
        children.place(0, offset)?;
        let mut geometry = BoxGeometry::new(size);
        geometry.baselines = Baselines {
            alphabetic: children
                .baseline(0, TextBaseline::Alphabetic, true)?
                .map(|b| b + offset.y),
            ideographic: children
                .baseline(0, TextBaseline::Ideographic, true)?
                .map(|b| b + offset.y),
        };
        Ok(geometry)
    }
}
