use super::*;
use crate::{
    element::{RenderBox, RenderContext, RenderObjectWidget, Widget},
    layout::*,
};

/// Retained single-child pointer target. Use ElementTree::show_with_gestures
/// with this same binding to deliver normalized egui primary-pointer input.
#[derive(Clone)]
pub struct GestureDetector {
    binding: GestureBinding,
    config: GestureTargetConfig,
    behavior: HitTestBehavior,
    sequence_key: Option<crate::egui::Id>,
}
impl GestureDetector {
    pub fn new(
        binding: &GestureBinding,
        config: GestureTargetConfig,
    ) -> Result<Self, BindingError> {
        config.validate()?;
        if binding.is_disposed() {
            return Err(BindingError::Disposed);
        }
        Ok(Self {
            binding: binding.clone(),
            config,
            behavior: HitTestBehavior::DeferToChild,
            sequence_key: None,
        })
    }
    pub fn behavior(mut self, behavior: HitTestBehavior) -> Self {
        self.behavior = behavior;
        self
    }
    /// Changing this key cancels active sequences while preserving the child.
    pub fn sequence_key(mut self, key: impl std::hash::Hash) -> Self {
        self.sequence_key = Some(crate::egui::Id::new(key));
        self
    }
    pub fn into_widget(self, child: Widget) -> Widget {
        Widget::render_object(self, vec![child])
    }
}
pub struct GestureDetectorRenderBox {
    binding: GestureBinding,
    target: Option<GestureTarget>,
    behavior: HitTestBehavior,
    sequence_key: Option<crate::egui::Id>,
    error: Option<BindingError>,
}
impl GestureDetectorRenderBox {
    fn configure(&mut self, config: &GestureDetector, ctx: &RenderContext) {
        self.behavior = config.behavior;
        if !self.binding.same_source(&config.binding) || self.sequence_key != config.sequence_key {
            self.target = None;
            self.binding = config.binding.clone();
            self.sequence_key = config.sequence_key;
        }
        let result = if let Some(target) = &self.target {
            target.update(config.config.clone())
        } else {
            self.binding
                .register_target(
                    crate::egui::Id::new(ctx.element_id()),
                    config.config.clone(),
                )
                .map(|t| self.target = Some(t))
        };
        self.error = result.err();
    }
    fn validate(&self, children: usize) -> Result<(), LayoutError> {
        if children != 1 {
            return Err(LayoutError::InvalidConfiguration(
                "GestureDetector requires one child",
            ));
        }
        if self.error.is_some() || !self.target.as_ref().is_some_and(GestureTarget::is_active) {
            return Err(LayoutError::InvalidConfiguration(
                "GestureDetector target registration is unavailable",
            ));
        }
        Ok(())
    }
}
impl RenderObjectWidget for GestureDetector {
    type Object = GestureDetectorRenderBox;
    fn create_render_object(&self, ctx: &RenderContext) -> Self::Object {
        let mut object = GestureDetectorRenderBox {
            binding: self.binding.clone(),
            target: None,
            behavior: self.behavior,
            sequence_key: self.sequence_key,
            error: None,
        };
        object.configure(self, ctx);
        object
    }
    fn update_render_object(&self, object: &mut Self::Object, ctx: &RenderContext) {
        object.configure(self, ctx);
    }
}
impl RenderBox for GestureDetectorRenderBox {
    fn dry_layout(
        &self,
        constraints: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<crate::egui::Vec2, LayoutError> {
        self.validate(children.len())?;
        children.dry_layout(0, constraints)
    }
    fn intrinsic(
        &self,
        dimension: IntrinsicDimension,
        extent: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        self.validate(children.len())?;
        children.intrinsic(0, dimension, extent)
    }
    fn dry_baseline(
        &self,
        constraints: BoxConstraints,
        kind: TextBaseline,
        children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        self.validate(children.len())?;
        children.dry_baseline(0, constraints, kind)
    }
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        self.validate(children.len())?;
        let size = children.layout(0, constraints)?;
        let mut geometry = BoxGeometry::new(size);
        geometry.baselines = Baselines {
            alphabetic: children.baseline(0, TextBaseline::Alphabetic, true)?,
            ideographic: children.baseline(0, TextBaseline::Ideographic, true)?,
        };
        geometry.hit_test_behavior = Some(self.behavior);
        children.place(0, crate::egui::Vec2::ZERO)?;
        Ok(geometry)
    }
    fn dispose(&mut self) {
        self.target = None;
    }
}
