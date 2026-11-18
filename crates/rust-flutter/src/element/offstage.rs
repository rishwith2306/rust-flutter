use super::{RenderBox, RenderContext, RenderObjectWidget, Widget};
use crate::{egui, layout::*};
/// Retains and measures a child while suppressing its visible output/pointer
/// targets. Backend callbacks still execute with an empty clip for keyboard
/// focus and input. Animations remain active; use TickerMode to mute them.
#[derive(Clone, Copy, Debug)]
pub struct Offstage {
    pub offstage: bool,
}
impl Default for Offstage {
    fn default() -> Self {
        Self { offstage: true }
    }
}
impl Offstage {
    pub fn new(offstage: bool) -> Self {
        Self { offstage }
    }
    pub fn into_widget(self, child: Widget) -> Widget {
        Widget::render_object(self, vec![child])
    }
}
impl Widget {
    pub fn offstage(offstage: bool, child: Widget) -> Self {
        Offstage::new(offstage).into_widget(child)
    }
}
pub struct OffstageRenderBox {
    offstage: bool,
}
impl RenderObjectWidget for Offstage {
    type Object = OffstageRenderBox;
    fn create_render_object(&self, _: &RenderContext) -> Self::Object {
        OffstageRenderBox {
            offstage: self.offstage,
        }
    }
    fn update_render_object(&self, o: &mut Self::Object, _: &RenderContext) {
        o.offstage = self.offstage;
    }
}
fn validate(n: usize) -> Result<(), LayoutError> {
    if n == 1 {
        Ok(())
    } else {
        Err(LayoutError::InvalidConfiguration(
            "Offstage requires one child",
        ))
    }
}
impl RenderBox for OffstageRenderBox {
    fn dry_layout(
        &self,
        c: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<egui::Vec2, LayoutError> {
        validate(children.len())?;
        if self.offstage {
            Ok(c.min())
        } else {
            children.dry_layout(0, c)
        }
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        e: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        validate(children.len())?;
        if self.offstage {
            Ok(0.0)
        } else {
            children.intrinsic(0, d, e)
        }
    }
    fn dry_baseline(
        &self,
        c: BoxConstraints,
        k: TextBaseline,
        children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        validate(children.len())?;
        if self.offstage {
            Ok(None)
        } else {
            children.dry_baseline(0, c, k)
        }
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        validate(children.len())?;
        let child = children.layout(0, c)?;
        children.place(0, egui::Vec2::ZERO)?;
        let mut g = BoxGeometry::new(if self.offstage { c.min() } else { child });
        g.offstage = self.offstage;
        if !self.offstage {
            g.baselines = Baselines {
                alphabetic: children.baseline(0, TextBaseline::Alphabetic, true)?,
                ideographic: children.baseline(0, TextBaseline::Ideographic, true)?,
            };
        }
        Ok(g)
    }
}
