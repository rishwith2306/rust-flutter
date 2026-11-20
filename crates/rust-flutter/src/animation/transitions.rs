use super::{Animation, AnimationError, TweenValue};
use crate::{
    decoration::{BoxDecoration, DecorationPosition},
    egui,
    element::{RenderBox, RenderContext, RenderHandle, RenderObjectWidget, Widget},
    layout::*,
    notifier::Subscription,
};
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

struct Shared<T: Copy + TweenValue> {
    value: RefCell<Result<T, AnimationError>>,
    command: RefCell<Rc<()>>,
    active: Cell<bool>,
    revision: Cell<u64>,
}
impl<T: Copy + TweenValue> Shared<T> {
    fn record(&self, value: Result<T, AnimationError>, handle: &RenderHandle, layout: bool) {
        let old_error = self.value.borrow().is_err();
        *self.value.borrow_mut() = value;
        if layout || old_error || value.is_err() {
            self.revision.set(self.revision.get().wrapping_add(1));
            let _ = handle.mark_needs_layout();
        } else {
            let _ = handle.mark_needs_paint();
        }
    }
    fn refresh<P: Animation<T>>(&self, source: &P, handle: &RenderHandle, layout: bool)
    where
        T: 'static,
    {
        if !self.active.get() {
            return;
        }
        let stamp = Rc::new(());
        *self.command.borrow_mut() = stamp.clone();
        let sample = catch_unwind(AssertUnwindSafe(|| {
            if source.is_disposed() {
                Err(AnimationError::Disposed)
            } else {
                source.try_value().and_then(|value| {
                    value.validate()?;
                    Ok(value)
                })
            }
        }))
        .map_err(|_| AnimationError::InterpolationPanicked)
        .and_then(|s| s);
        let current = self.command.borrow().clone();
        if self.active.get() && Rc::ptr_eq(&stamp, &current) {
            self.record(
                if source.is_disposed() {
                    Err(AnimationError::Disposed)
                } else {
                    sample
                },
                handle,
                layout,
            );
        }
    }
}
struct SourceLink<P: Animation<T> + Clone, T: Copy + TweenValue + 'static> {
    source: P,
    shared: Rc<Shared<T>>,
    subscription: Option<Subscription>,
    handle: RenderHandle,
    layout: bool,
}
impl<P: Animation<T> + Clone, T: Copy + TweenValue + 'static> SourceLink<P, T> {
    fn new(source: &P, ctx: &RenderContext, layout: bool) -> Self {
        let shared = Rc::new(Shared {
            value: RefCell::new(Err(AnimationError::Disposed)),
            command: RefCell::new(Rc::new(())),
            active: Cell::new(true),
            revision: Cell::new(0),
        });
        let handle = ctx.handle();
        let owner = shared.clone();
        let invalidation = handle.clone();
        let parent = source.clone();
        let subscription = source.subscribe(Rc::new(move || {
            owner.refresh(&parent, &invalidation, layout)
        }));
        let subscription = match subscription {
            Ok(s) => {
                shared.refresh(source, &handle, layout);
                Some(s)
            }
            Err(_) => {
                shared.record(Err(AnimationError::Disposed), &handle, layout);
                None
            }
        };
        Self {
            source: source.clone(),
            shared,
            subscription,
            handle,
            layout,
        }
    }
    fn bind(&mut self, source: &P, ctx: &RenderContext) {
        if self.source.same_source(source) && self.subscription.is_some() {
            self.source = source.clone();
            self.shared.refresh(source, &self.handle, self.layout);
        } else {
            self.stop();
            *self = Self::new(source, ctx, self.layout);
        }
    }
    fn stop(&mut self) {
        self.shared.active.set(false);
        let subscription = self.subscription.take();
        drop(subscription);
    }
    fn revision(&self) -> u64 {
        if self.source.is_disposed() && self.shared.value.borrow().is_ok() {
            self.shared
                .record(Err(AnimationError::Disposed), &self.handle, self.layout);
        }
        self.shared.revision.get()
    }
    fn value(&self) -> Result<T, LayoutError> {
        if self.source.is_disposed() {
            return Err(LayoutError::Animation(AnimationError::Disposed));
        }
        (*self.shared.value.borrow()).map_err(LayoutError::Animation)
    }
}
impl<P: Animation<T> + Clone, T: Copy + TweenValue + 'static> Drop for SourceLink<P, T> {
    fn drop(&mut self) {
        self.stop();
    }
}
fn single(n: usize) -> Result<(), LayoutError> {
    if n == 1 {
        Ok(())
    } else {
        Err(LayoutError::InvalidConfiguration(
            "transition requires exactly one child",
        ))
    }
}
#[derive(Clone, Copy)]
struct AlignConfig {
    alignment: Alignment,
    width: Option<f32>,
    height: Option<f32>,
    clip: Clip,
}
impl AlignConfig {
    fn validate(self) -> Result<(), LayoutError> {
        self.alignment.offset(egui::Vec2::ZERO)?;
        if self
            .width
            .iter()
            .chain(self.height.iter())
            .any(|v| !v.is_finite() || *v < 0.0)
        {
            Err(LayoutError::InvalidConfiguration(
                "transition size factors must be finite and nonnegative",
            ))
        } else {
            Ok(())
        }
    }
    fn geometry(
        self,
        c: BoxConstraints,
        child: egui::Vec2,
    ) -> Result<(egui::Vec2, egui::Vec2), LayoutError> {
        self.validate()?;
        let max = c.max();
        let size = c.constrain(egui::vec2(
            self.width
                .map_or(if max.x.is_finite() { max.x } else { child.x }, |f| {
                    child.x * f
                }),
            self.height
                .map_or(if max.y.is_finite() { max.y } else { child.y }, |f| {
                    child.y * f
                }),
        ))?;
        Ok((size, self.alignment.offset(size - child)?))
    }
    fn dry(self, c: BoxConstraints, children: &BoxQueries<'_>) -> Result<egui::Vec2, LayoutError> {
        single(children.len())?;
        self.validate()?;
        self.geometry(c, children.dry_layout(0, c.loosen())?)
            .map(|g| g.0)
    }
    fn baseline(
        self,
        c: BoxConstraints,
        k: TextBaseline,
        children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        single(children.len())?;
        self.validate()?;
        let baseline = children.dry_baseline(0, c.loosen(), k)?;
        if let Some(b) = baseline {
            let (_, offset) = self.geometry(c, children.dry_layout(0, c.loosen())?)?;
            let value = b + offset.y;
            if !value.is_finite() {
                return Err(LayoutError::InvalidConfiguration(
                    "translated transition baseline must be finite",
                ));
            }
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
    fn intrinsic(
        self,
        d: IntrinsicDimension,
        e: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        single(children.len())?;
        self.validate()?;
        let factor = if d.axis() == Axis::Horizontal {
            self.width
        } else {
            self.height
        }
        .unwrap_or(1.0);
        let value = children.intrinsic(0, d, e)? * factor;
        if !value.is_finite() {
            return Err(LayoutError::InvalidSize);
        }
        Ok(value)
    }
    fn layout(
        self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
    ) -> Result<BoxGeometry, LayoutError> {
        single(children.len())?;
        self.validate()?;
        let child = children.layout(0, c.loosen())?;
        let (size, offset) = self.geometry(c, child)?;
        children.place(0, offset)?;
        let mut geometry = BoxGeometry::new(size);
        geometry.clip = self.clip;
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
/// Animates the visible width/height of a retained, measured child. Negative
/// factors collapse to zero; parent constraints still determine the final size.
#[derive(Clone)]
pub struct SizeTransition<P: Animation<f32> + Clone> {
    pub size_factor: P,
    pub axis: Axis,
    pub alignment: Option<Alignment>,
    pub fixed_cross_axis_size_factor: Option<f32>,
}
impl<P: Animation<f32> + Clone> SizeTransition<P> {
    pub fn new(size_factor: P) -> Self {
        Self {
            size_factor,
            axis: Axis::Vertical,
            alignment: None,
            fixed_cross_axis_size_factor: None,
        }
    }
    pub fn into_widget(self, child: Widget) -> Widget {
        Widget::render_object(self, vec![child])
    }
}
pub struct SizeTransitionRenderBox<P: Animation<f32> + Clone> {
    source: SourceLink<P, f32>,
    axis: Axis,
    alignment: Option<Alignment>,
    cross: Option<f32>,
}
impl<P: Animation<f32> + Clone> SizeTransitionRenderBox<P> {
    fn config(&self) -> Result<AlignConfig, LayoutError> {
        let value = self.source.value()?;
        if !value.is_finite() {
            return Err(LayoutError::InvalidConfiguration(
                "size transition factor must be finite",
            ));
        }
        let factor = value.max(0.0);
        let alignment = self.alignment.unwrap_or(if self.axis == Axis::Vertical {
            Alignment { x: -1.0, y: 0.0 }
        } else {
            Alignment { x: 0.0, y: -1.0 }
        });
        Ok(AlignConfig {
            alignment,
            width: if self.axis == Axis::Horizontal {
                Some(factor)
            } else {
                self.cross
            },
            height: if self.axis == Axis::Vertical {
                Some(factor)
            } else {
                self.cross
            },
            clip: Clip::HardEdge,
        })
    }
}
impl<P: Animation<f32> + Clone> RenderObjectWidget for SizeTransition<P> {
    type Object = SizeTransitionRenderBox<P>;
    fn create_render_object(&self, ctx: &RenderContext) -> Self::Object {
        SizeTransitionRenderBox {
            source: SourceLink::new(&self.size_factor, ctx, true),
            axis: self.axis,
            alignment: self.alignment,
            cross: self.fixed_cross_axis_size_factor,
        }
    }
    fn update_render_object(&self, o: &mut Self::Object, ctx: &RenderContext) {
        o.source.bind(&self.size_factor, ctx);
        o.axis = self.axis;
        o.alignment = self.alignment;
        o.cross = self.fixed_cross_axis_size_factor;
    }
}
impl<P: Animation<f32> + Clone> RenderBox for SizeTransitionRenderBox<P> {
    fn layout_revision(&self) -> u64 {
        self.source.revision()
    }
    fn dry_layout(&self, c: BoxConstraints, q: &BoxQueries<'_>) -> Result<egui::Vec2, LayoutError> {
        self.config()?.dry(c, q)
    }
    fn dry_baseline(
        &self,
        c: BoxConstraints,
        k: TextBaseline,
        q: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        self.config()?.baseline(c, k, q)
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        e: f32,
        q: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        self.config()?.intrinsic(d, e, q)
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        q: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        self.config()?.layout(c, q)
    }
    fn dispose(&mut self) {
        self.source.stop();
    }
}
/// Aligns a retained child from an observable physical Alignment value. Optional
/// size factors follow the existing Align box policy; overflow remains unclipped.
#[derive(Clone)]
pub struct AlignTransition<P: Animation<Alignment> + Clone> {
    pub alignment: P,
    pub width_factor: Option<f32>,
    pub height_factor: Option<f32>,
}
impl<P: Animation<Alignment> + Clone> AlignTransition<P> {
    pub fn new(alignment: P) -> Self {
        Self {
            alignment,
            width_factor: None,
            height_factor: None,
        }
    }
    pub fn into_widget(self, child: Widget) -> Widget {
        Widget::render_object(self, vec![child])
    }
}
pub struct AlignTransitionRenderBox<P: Animation<Alignment> + Clone> {
    source: SourceLink<P, Alignment>,
    width: Option<f32>,
    height: Option<f32>,
}
impl<P: Animation<Alignment> + Clone> AlignTransitionRenderBox<P> {
    fn config(&self) -> Result<AlignConfig, LayoutError> {
        Ok(AlignConfig {
            alignment: self.source.value()?,
            width: self.width,
            height: self.height,
            clip: Clip::None,
        })
    }
}
impl<P: Animation<Alignment> + Clone> RenderObjectWidget for AlignTransition<P> {
    type Object = AlignTransitionRenderBox<P>;
    fn create_render_object(&self, ctx: &RenderContext) -> Self::Object {
        AlignTransitionRenderBox {
            source: SourceLink::new(&self.alignment, ctx, true),
            width: self.width_factor,
            height: self.height_factor,
        }
    }
    fn update_render_object(&self, o: &mut Self::Object, ctx: &RenderContext) {
        o.source.bind(&self.alignment, ctx);
        o.width = self.width_factor;
        o.height = self.height_factor;
    }
}
impl<P: Animation<Alignment> + Clone> RenderBox for AlignTransitionRenderBox<P> {
    fn layout_revision(&self) -> u64 {
        self.source.revision()
    }
    fn dry_layout(&self, c: BoxConstraints, q: &BoxQueries<'_>) -> Result<egui::Vec2, LayoutError> {
        self.config()?.dry(c, q)
    }
    fn dry_baseline(
        &self,
        c: BoxConstraints,
        k: TextBaseline,
        q: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        self.config()?.baseline(c, k, q)
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        e: f32,
        q: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        self.config()?.intrinsic(d, e, q)
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        q: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        self.config()?.layout(c, q)
    }
    fn dispose(&mut self) {
        self.source.stop();
    }
}
/// Paints an observable box decoration around a retained child. Successful
/// source changes repaint without rebuilding or relaying out the child.
#[derive(Clone)]
pub struct DecoratedBoxTransition<P: Animation<BoxDecoration> + Clone> {
    pub decoration: P,
    pub position: DecorationPosition,
}
impl<P: Animation<BoxDecoration> + Clone> DecoratedBoxTransition<P> {
    pub fn new(decoration: P) -> Self {
        Self {
            decoration,
            position: DecorationPosition::Background,
        }
    }
    pub fn into_widget(self, child: Widget) -> Widget {
        Widget::render_object(self, vec![child])
    }
}
pub struct DecoratedBoxTransitionRenderBox<P: Animation<BoxDecoration> + Clone> {
    source: SourceLink<P, BoxDecoration>,
    position: DecorationPosition,
}
impl<P: Animation<BoxDecoration> + Clone> RenderObjectWidget for DecoratedBoxTransition<P> {
    type Object = DecoratedBoxTransitionRenderBox<P>;
    fn create_render_object(&self, ctx: &RenderContext) -> Self::Object {
        DecoratedBoxTransitionRenderBox {
            source: SourceLink::new(&self.decoration, ctx, false),
            position: self.position,
        }
    }
    fn update_render_object(&self, o: &mut Self::Object, ctx: &RenderContext) {
        o.source.bind(&self.decoration, ctx);
        o.position = self.position;
    }
}
impl<P: Animation<BoxDecoration> + Clone> DecoratedBoxTransitionRenderBox<P> {
    fn validate(&self, n: usize) -> Result<(), LayoutError> {
        single(n)?;
        self.source.value()?;
        Ok(())
    }
}
impl<P: Animation<BoxDecoration> + Clone> RenderBox for DecoratedBoxTransitionRenderBox<P> {
    fn layout_revision(&self) -> u64 {
        self.source.revision()
    }
    fn dry_layout(&self, c: BoxConstraints, q: &BoxQueries<'_>) -> Result<egui::Vec2, LayoutError> {
        self.validate(q.len())?;
        q.dry_layout(0, c)
    }
    fn dry_baseline(
        &self,
        c: BoxConstraints,
        k: TextBaseline,
        q: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        self.validate(q.len())?;
        q.dry_baseline(0, c, k)
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        e: f32,
        q: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        self.validate(q.len())?;
        q.intrinsic(0, d, e)
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        q: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        self.validate(q.len())?;
        let size = q.layout(0, c)?;
        q.place(0, egui::Vec2::ZERO)?;
        let mut geometry = BoxGeometry::new(size);
        geometry.baselines = Baselines {
            alphabetic: q.baseline(0, TextBaseline::Alphabetic, true)?,
            ideographic: q.baseline(0, TextBaseline::Ideographic, true)?,
        };
        Ok(geometry)
    }
    fn paint(&mut self, ui: &mut egui::Ui, _: &RenderContext) {
        if self.position == DecorationPosition::Background
            && let Ok(decoration) = self.source.value()
        {
            decoration.paint(ui);
        }
    }
    fn paint_foreground(&mut self, ui: &mut egui::Ui, _: &RenderContext) {
        if self.position == DecorationPosition::Foreground
            && let Ok(decoration) = self.source.value()
        {
            decoration.paint(ui);
        }
    }
    fn dispose(&mut self) {
        self.source.stop();
    }
}
