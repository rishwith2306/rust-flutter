//! Persistent, typed box render objects and visual invalidation.
use super::{
    BuildContext,
    inherited::Phase,
    state::{Queue, Unmounted},
    widget::Paint,
};
use crate::layout::{
    BoxChildren, BoxConstraints, BoxGeometry, BoxQueries, CustomBox, CustomBoxLayout,
    IntrinsicDimension, LayoutError, TextBaseline,
};
use std::{
    any::{Any, TypeId},
    cell::RefCell,
    ops::Deref,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

pub(super) struct Pipeline {
    revision: AtomicU64,
}
impl Pipeline {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            revision: AtomicU64::new(0),
        })
    }
    pub fn invalidate(&self) {
        self.revision.fetch_add(1, Ordering::AcqRel);
    }
    fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
}
/// Sendable invalidation handle; the render object itself stays on the UI thread.
#[derive(Clone)]
pub struct RenderHandle {
    queue: Arc<Queue>,
}
impl RenderHandle {
    pub fn mounted(&self) -> bool {
        self.queue.mounted()
    }
    pub fn mark_needs_layout(&self) -> Result<(), Unmounted> {
        self.queue.request_visual_update(true)
    }
    /// Repaint through egui without invalidating custom layout caches.
    pub fn mark_needs_paint(&self) -> Result<(), Unmounted> {
        self.queue.request_visual_update(false)
    }
}
#[derive(Clone)]
pub struct RenderContext {
    build: BuildContext,
}
impl Deref for RenderContext {
    type Target = BuildContext;
    fn deref(&self) -> &BuildContext {
        &self.build
    }
}
impl RenderContext {
    pub fn handle(&self) -> RenderHandle {
        RenderHandle {
            queue: self.build.queue.clone(),
        }
    }
}
/// Typed immutable widget configuration for a persistent Cartesian render box.
/// Initial creation may read inherited values. Subscribe in update/layout/paint;
/// listening during create/attach is restricted like component initialization.
pub trait RenderObjectWidget: Sized + 'static {
    type Object: RenderBox;
    fn create_render_object(&self, ctx: &RenderContext) -> Self::Object;
    fn update_render_object(&self, object: &mut Self::Object, ctx: &RenderContext);
    fn did_unmount_render_object(&self, _object: &mut Self::Object) {}
}
/// Custom leaf or container layout, paint and mounted lifetime hooks. Child
/// geometry is validated by BoxChildren; paint runs before placed children.
pub trait RenderBox: 'static {
    fn dry_baseline(
        &self,
        _constraints: BoxConstraints,
        _kind: TextBaseline,
        _children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        Err(LayoutError::UnsupportedQuery(
            "render box has no dry-baseline implementation",
        ))
    }
    /// Read-only sizing. Never call wet layout as a fallback for these protocols.
    fn dry_layout(
        &self,
        _constraints: BoxConstraints,
        _children: &BoxQueries<'_>,
    ) -> Result<egui::Vec2, LayoutError> {
        Err(LayoutError::UnsupportedQuery(
            "render box has no dry-layout implementation",
        ))
    }
    fn intrinsic(
        &self,
        _dimension: IntrinsicDimension,
        _extent: f32,
        _children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        Err(LayoutError::UnsupportedQuery(
            "render box has no intrinsic-size implementation",
        ))
    }
    fn attach(&mut self, _ctx: &RenderContext) {}
    fn children_changed(&mut self, _old: &[egui::Id], _new: &[egui::Id], _ctx: &RenderContext) {}
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        children: &mut BoxChildren<'_>,
        ctx: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError>;
    fn paint(&mut self, _ui: &mut egui::Ui, _ctx: &RenderContext) {}
    fn detach(&mut self) {}
    fn dispose(&mut self) {}
}
pub(super) trait RenderDescription {
    fn token(&self) -> TypeId;
    fn name(&self) -> &'static str;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn mount(self: Box<Self>, ctx: BuildContext) -> Box<dyn MountedRender>;
}
pub(super) trait MountedRender {
    fn update(&mut self, widget: Box<dyn Any>);
    fn mark_needs_update(&mut self);
    fn synchronize(&mut self, children: Vec<egui::Id>);
    fn layout(&self) -> CustomBox;
    fn paint(&self) -> Paint;
}
pub(super) struct RenderWidget<W: RenderObjectWidget>(pub W);
struct Instance<W: RenderObjectWidget> {
    widget: W,
    object: W::Object,
    context: RenderContext,
    children: Vec<egui::Id>,
    needs_update: bool,
}
struct Mounted<W: RenderObjectWidget> {
    instance: Rc<RefCell<Instance<W>>>,
    layout: CustomBox,
    paint: Paint,
}
struct Adapter<W: RenderObjectWidget>(Rc<RefCell<Instance<W>>>);
impl<W: RenderObjectWidget> CustomBoxLayout for Adapter<W> {
    fn dry_baseline(
        &self,
        c: BoxConstraints,
        kind: TextBaseline,
        children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        let instance = self
            .0
            .try_borrow()
            .map_err(|_| LayoutError::InvalidConfiguration("render object is already active"))?;
        instance.object.dry_baseline(c, kind, children)
    }
    fn dry_layout(
        &self,
        c: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<egui::Vec2, LayoutError> {
        let instance = self
            .0
            .try_borrow()
            .map_err(|_| LayoutError::InvalidConfiguration("render object is already active"))?;
        instance.object.dry_layout(c, children)
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        extent: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        let instance = self
            .0
            .try_borrow()
            .map_err(|_| LayoutError::InvalidConfiguration("render object is already active"))?;
        instance.object.intrinsic(d, extent, children)
    }
    fn revision(&self) -> u64 {
        self.0.borrow().context.build.queue.pipeline.revision()
    }
    fn layout(
        &mut self,
        constraints: BoxConstraints,
        children: &mut BoxChildren<'_>,
    ) -> Result<BoxGeometry, LayoutError> {
        let mut instance = self.0.borrow_mut();
        let context = instance.context.clone();
        instance.object.layout(constraints, children, &context)
    }
}
impl<W: RenderObjectWidget> RenderDescription for RenderWidget<W> {
    fn token(&self) -> TypeId {
        TypeId::of::<W>()
    }
    fn name(&self) -> &'static str {
        std::any::type_name::<W>()
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        Box::new(self.0)
    }
    fn mount(self: Box<Self>, ctx: BuildContext) -> Box<dyn MountedRender> {
        let context = RenderContext { build: ctx };
        let object = {
            let _phase = context.build.dependencies.enter(Phase::Init);
            let mut object = self.0.create_render_object(&context);
            object.attach(&context);
            object
        };
        let instance = Rc::new(RefCell::new(Instance {
            widget: self.0,
            object,
            context,
            children: vec![],
            needs_update: true,
        }));
        let painting = instance.clone();
        let paint: Paint = Rc::new(RefCell::new(move |ui: &mut egui::Ui| {
            let mut instance = painting.borrow_mut();
            let context = instance.context.clone();
            instance.object.paint(ui, &context);
        }));
        Box::new(Mounted {
            layout: CustomBox::new(Adapter(instance.clone())),
            instance,
            paint,
        })
    }
}
impl<W: RenderObjectWidget> MountedRender for Mounted<W> {
    fn update(&mut self, widget: Box<dyn Any>) {
        let mut instance = self.instance.borrow_mut();
        instance.widget = *widget.downcast::<W>().expect("matching render widget type");
        instance.needs_update = true;
    }
    fn mark_needs_update(&mut self) {
        self.instance.borrow_mut().needs_update = true;
    }
    fn synchronize(&mut self, children: Vec<egui::Id>) {
        let mut instance = self.instance.borrow_mut();
        let context = instance.context.clone();
        let dependencies_changed = context.build.dependencies.changed.replace(false);
        if instance.needs_update || dependencies_changed {
            instance.needs_update = false;
            let _phase = context.build.dependencies.enter(Phase::Build);
            let Instance { widget, object, .. } = &mut *instance;
            widget.update_render_object(object, &context);
        }
        if instance.children != children {
            let old = std::mem::replace(&mut instance.children, children);
            let new = instance.children.clone();
            instance.object.children_changed(&old, &new, &context);
        }
    }
    fn layout(&self) -> CustomBox {
        self.layout.clone()
    }
    fn paint(&self) -> Paint {
        self.paint.clone()
    }
}
impl<W: RenderObjectWidget> Drop for Mounted<W> {
    fn drop(&mut self) {
        let mut instance = self.instance.borrow_mut();
        instance.context.build.queue.close();
        instance.object.detach();
        let Instance { widget, object, .. } = &mut *instance;
        widget.did_unmount_render_object(object);
        object.dispose();
    }
}
