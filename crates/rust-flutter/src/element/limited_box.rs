use super::{RenderBox, RenderContext, RenderObjectWidget, Widget};
use crate::{egui, layout::*};

/// Supplies maximum dimensions only on axes left unbounded by the parent.
/// Parent minimum dimensions always take precedence over these limits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LimitedBox {
    pub max_width: f32,
    pub max_height: f32,
}
impl Default for LimitedBox {
    fn default() -> Self {
        Self {
            max_width: f32::INFINITY,
            max_height: f32::INFINITY,
        }
    }
}
impl LimitedBox {
    pub fn new(max_width: f32, max_height: f32) -> Result<Self, LayoutError> {
        let value = Self {
            max_width,
            max_height,
        };
        value.validate(0)?;
        Ok(value)
    }
    pub fn into_widget(self, child: Widget) -> Widget {
        Widget::render_object(self, vec![child])
    }
    fn validate(self, count: usize) -> Result<(), LayoutError> {
        if self.max_width.is_nan()
            || self.max_height.is_nan()
            || self.max_width < 0.0
            || self.max_height < 0.0
        {
            return Err(LayoutError::InvalidConfiguration(
                "LimitedBox limits must be nonnegative",
            ));
        }
        if count > 1 {
            return Err(LayoutError::InvalidConfiguration(
                "LimitedBox accepts at most one child",
            ));
        }
        Ok(())
    }
    fn constraints(self, c: BoxConstraints) -> Result<BoxConstraints, LayoutError> {
        let min = c.min();
        let max = c.max();
        BoxConstraints::new(
            min,
            egui::vec2(
                if max.x.is_finite() {
                    max.x
                } else {
                    self.max_width.max(min.x)
                },
                if max.y.is_finite() {
                    max.y
                } else {
                    self.max_height.max(min.y)
                },
            ),
        )
    }
}
impl Widget {
    pub fn limited_box(
        max_width: f32,
        max_height: f32,
        child: Widget,
    ) -> Result<Self, LayoutError> {
        Ok(LimitedBox::new(max_width, max_height)?.into_widget(child))
    }
}
impl RenderObjectWidget for LimitedBox {
    type Object = Self;
    fn create_render_object(&self, _: &RenderContext) -> Self {
        *self
    }
    fn update_render_object(&self, object: &mut Self, _: &RenderContext) {
        *object = *self;
    }
}
impl RenderBox for LimitedBox {
    fn dry_layout(
        &self,
        c: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<egui::Vec2, LayoutError> {
        self.validate(children.len())?;
        if children.is_empty() {
            Ok(c.min())
        } else {
            children.dry_layout(0, self.constraints(c)?)
        }
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        extent: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        self.validate(children.len())?;
        if children.is_empty() {
            Ok(0.0)
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
        self.validate(children.len())?;
        if children.is_empty() {
            Ok(None)
        } else {
            children.dry_baseline(0, self.constraints(c)?, k)
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
            return Ok(BoxGeometry::new(c.min()));
        }
        let size = children.layout(0, self.constraints(c)?)?;
        children.place(0, egui::Vec2::ZERO)?;
        let mut geometry = BoxGeometry::new(size);
        geometry.baselines = Baselines {
            alphabetic: children.baseline(0, TextBaseline::Alphabetic, true)?,
            ideographic: children.baseline(0, TextBaseline::Ideographic, true)?,
        };
        Ok(geometry)
    }
}
