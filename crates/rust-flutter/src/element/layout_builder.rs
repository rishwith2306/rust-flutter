use super::{
    BuildContext, ElementId, RenderBox, RenderContext, RenderObjectWidget, Widget,
    render_object::Pipeline,
    state::{Component, Description, Queue},
    widget::Kind,
};
use crate::layout::*;
use std::{
    any::{Any, TypeId},
    cell::Cell,
    rc::Rc,
    sync::{Arc, Weak},
};

type Builder = dyn Fn(&BuildContext, BoxConstraints) -> Widget;
/// Builds a retained child with its actual parent constraints before painting.
/// Repeated identical constraints reuse the child; configuration and inherited
/// dependency updates rebuild with current or last-known layout constraints.
pub struct LayoutBuilder {
    builder: Rc<Builder>,
}
impl LayoutBuilder {
    pub fn new(builder: impl Fn(&BuildContext, BoxConstraints) -> Widget + 'static) -> Self {
        Self {
            builder: Rc::new(builder),
        }
    }
    pub fn into_widget(self) -> Widget {
        Widget {
            key: None,
            kind: Kind::Component(Box::new(LayoutDescription(self))),
            children: vec![],
        }
    }
}
impl Widget {
    pub fn layout_builder(
        builder: impl Fn(&BuildContext, BoxConstraints) -> Widget + 'static,
    ) -> Self {
        LayoutBuilder::new(builder).into_widget()
    }
}
struct LayoutDescription(LayoutBuilder);
struct Shared {
    owner: ElementId,
    queue: Weak<Queue>,
    pipeline: Weak<Pipeline>,
    built: Cell<Option<BoxConstraints>>,
    dirty: Cell<bool>,
}
impl Shared {
    fn request(&self, constraints: BoxConstraints) {
        if let Some(pipeline) = self.pipeline.upgrade() {
            pipeline.request_layout_build(
                self.owner,
                self.queue.clone(),
                Box::new(move |state| {
                    state
                        .downcast_mut::<Instance>()
                        .expect("layout builder command")
                        .pending = Some(constraints);
                }),
            );
        }
    }
    fn cancel(&self) {
        if let Some(pipeline) = self.pipeline.upgrade() {
            pipeline.cancel_layout_build(self.owner);
        }
    }
}
struct Instance {
    builder: Rc<Builder>,
    context: BuildContext,
    shared: Rc<Shared>,
    pending: Option<BoxConstraints>,
    initialized: bool,
}
impl Description for LayoutDescription {
    fn token(&self) -> TypeId {
        TypeId::of::<Self>()
    }
    fn name(&self) -> &'static str {
        "LayoutBuilder"
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        Box::new(self.0)
    }
    fn mount(self: Box<Self>, context: BuildContext) -> Box<dyn Component> {
        let shared = Rc::new(Shared {
            owner: context.element_id(),
            queue: Arc::downgrade(&context.queue),
            pipeline: Arc::downgrade(&context.queue.pipeline),
            built: Cell::new(None),
            dirty: Cell::new(true),
        });
        Box::new(Instance {
            builder: self.0.builder,
            context,
            shared,
            pending: None,
            initialized: false,
        })
    }
}
impl Component for Instance {
    fn dependencies_changed(&mut self) {}
    fn update(&mut self, description: Box<dyn Any>) {
        self.builder = description
            .downcast::<LayoutBuilder>()
            .expect("layout builder description")
            .builder;
    }
    fn apply_pending(&mut self) {
        let queue = self.context.queue.clone();
        queue.drain(self);
    }
    fn state(&self) -> Option<&dyn Any> {
        None
    }
    fn build(&mut self) -> Option<Widget> {
        if let Some(constraints) = self.pending.take() {
            self.shared.built.set(Some(constraints));
            self.shared.dirty.set(false);
            let child = (self.builder)(&self.context, constraints);
            Some(Widget::render_object(
                Probe(self.shared.clone()),
                vec![child],
            ))
        } else if !self.initialized {
            self.initialized = true;
            Some(Widget::render_object(
                Probe(self.shared.clone()),
                vec![Widget::empty()],
            ))
        } else {
            self.shared.dirty.set(true);
            // If an ancestor skips this probe, retain the last known constraints.
            // A later measurement in this transaction can replace this fallback.
            if let Some(constraints) = self.shared.built.get() {
                self.shared.request(constraints);
            }
            None
        }
    }
}
impl Drop for Instance {
    fn drop(&mut self) {
        self.context.queue.close();
        self.shared.cancel();
    }
}
#[derive(Clone)]
struct Probe(Rc<Shared>);
struct ProbeObject(Rc<Shared>);
impl RenderObjectWidget for Probe {
    type Object = ProbeObject;
    fn create_render_object(&self, _: &RenderContext) -> Self::Object {
        ProbeObject(self.0.clone())
    }
    fn update_render_object(&self, object: &mut Self::Object, _: &RenderContext) {
        object.0 = self.0.clone();
    }
}
impl RenderBox for ProbeObject {
    fn dry_layout(&self, _: BoxConstraints, _: &BoxQueries<'_>) -> Result<egui::Vec2, LayoutError> {
        Err(LayoutError::UnsupportedQuery(
            "LayoutBuilder requires wet parent constraints",
        ))
    }
    fn intrinsic(
        &self,
        _: IntrinsicDimension,
        _: f32,
        _: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        Err(LayoutError::UnsupportedQuery(
            "LayoutBuilder has no intrinsic size",
        ))
    }
    fn dry_baseline(
        &self,
        _: BoxConstraints,
        _: TextBaseline,
        _: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        Err(LayoutError::UnsupportedQuery(
            "LayoutBuilder has no dry baseline",
        ))
    }
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        if children.len() != 1 {
            return Err(LayoutError::InvalidConfiguration(
                "LayoutBuilder requires one retained child",
            ));
        }
        if self.0.dirty.get() || self.0.built.get() != Some(constraints) {
            self.0.request(constraints);
            children.defer_for_build(0, constraints.min())?;
            return Ok(BoxGeometry::new(constraints.min()));
        }
        self.0.cancel();
        let size = children.layout(0, constraints)?;
        children.place(0, egui::Vec2::ZERO)?;
        let mut geometry = BoxGeometry::new(size);
        geometry.baselines = Baselines {
            alphabetic: children.baseline(0, TextBaseline::Alphabetic, true)?,
            ideographic: children.baseline(0, TextBaseline::Ideographic, true)?,
        };
        Ok(geometry)
    }
}
