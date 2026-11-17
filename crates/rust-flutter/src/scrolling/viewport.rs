use super::{ScrollActivity, ScrollDimensions, ScrollError, ScrollPosition, WeakScrollPosition};
use crate::{
    egui,
    element::{RenderBox, RenderContext, RenderHandle, RenderObjectWidget, Widget},
    gestures::*,
    layout::*,
    notifier::{Listenable, Subscription},
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

/// Eager retained single-child viewport with primary-pointer pan input.
/// The caller owns the position/controller attachment and uses this binding
/// with ElementTree's gesture-aware show methods for pan and scroll signals.
#[derive(Clone)]
pub struct SingleChildScrollView {
    position: ScrollPosition,
    binding: GestureBinding,
    axis: Axis,
    reverse: bool,
    scale: Rc<Cell<Option<f32>>>,
    dimensions: Rc<Cell<Option<ScrollDimensions>>>,
}
impl SingleChildScrollView {
    pub fn new(position: &ScrollPosition, binding: &GestureBinding) -> Result<Self, ScrollError> {
        if position.is_disposed() || binding.is_disposed() {
            return Err(ScrollError::Disposed);
        }
        Ok(Self {
            position: position.clone(),
            binding: binding.clone(),
            axis: Axis::Vertical,
            reverse: false,
            scale: Rc::new(Cell::new(Some(1.0))),
            dimensions: Rc::new(Cell::new(None)),
        })
    }
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }
    pub fn into_widget(self, child: Widget) -> Result<Widget, ScrollError> {
        let down = self.position.downgrade();
        let down_dimensions = self.dimensions.clone();
        let start = self.position.downgrade();
        let update = self.position.downgrade();
        let end = self.position.downgrade();
        let cancel = self.position.downgrade();
        let axis = self.axis;
        let sign = if self.reverse { 1.0 } else { -1.0 };
        let update_scale = self.scale.clone();
        let end_scale = self.scale.clone();
        let scroll = self.position.downgrade();
        let scroll_dimensions = self.dimensions.clone();
        let scroll_scale = self.scale.clone();
        let reverse = self.reverse;
        let config = GestureTargetConfig {
            on_scroll: Some(Rc::new(move |event| {
                if event.modifiers.ctrl || event.modifiers.command {
                    return false;
                }
                let Some(position) = scroll.upgrade().filter(|p| !p.is_disposed()) else {
                    return false;
                };
                let Some(dimensions) = scroll_dimensions.get().or_else(|| position.dimensions())
                else {
                    return false;
                };
                let raw = if event.modifiers.shift && event.scroll_delta.x == 0.0 {
                    if axis == Axis::Horizontal {
                        event.scroll_delta.y
                    } else {
                        0.0
                    }
                } else {
                    main(axis, event.scroll_delta)
                };
                let multiplier = match event.unit {
                    ScrollSignalUnit::Points => {
                        let Some(scale) = scroll_scale.get() else {
                            return false;
                        };
                        1.0 / f64::from(scale)
                    }
                    ScrollSignalUnit::Lines => 20.0,
                    ScrollSignalUnit::Pages => f64::from(dimensions.viewport_dimension()),
                };
                let delta = f64::from(raw) * multiplier * if reverse { -1.0 } else { 1.0 };
                let current = position.pixels().clamp(
                    dimensions.min_scroll_extent(),
                    dimensions.max_scroll_extent(),
                );
                let target = (f64::from(current) + delta).clamp(
                    f64::from(dimensions.min_scroll_extent()),
                    f64::from(dimensions.max_scroll_extent()),
                ) as f32;
                if target == current {
                    return false;
                }
                if position.apply_dimensions(dimensions).is_err() {
                    return false;
                }
                position.jump_to(target).is_ok()
            })),
            pan: Some((
                PanConfig {
                    start_behavior: DragStartBehavior::Down,
                    ..Default::default()
                },
                PanCallbacks {
                    on_down: Some(Rc::new(move |_| {
                        if let Some(p) = down.upgrade() {
                            if let Some(dimensions) = down_dimensions.get() {
                                let _ = p.apply_dimensions(dimensions);
                            }
                            let _ = p.cancel_activity();
                        }
                    })),
                    on_start: Some(Rc::new(move |_| {
                        if let Some(p) = start.upgrade() {
                            let _ = p.begin_drag();
                        }
                    })),
                    on_update: Some(Rc::new(move |d| {
                        if let (Some(p), Some(scale)) = (update.upgrade(), update_scale.get()) {
                            let _ = p.drag_by(sign * main(axis, d.delta) / scale);
                        }
                    })),
                    on_end: Some(Rc::new(move |d| {
                        if let Some(p) = end.upgrade() {
                            if let Some(scale) = end_scale.get() {
                                let _ = p.end_drag(
                                    sign * main(axis, d.velocity.pixels_per_second()) / scale,
                                    0.135,
                                );
                            } else {
                                let _ = p.cancel_activity();
                            }
                        }
                    })),
                    on_cancel: Some(Rc::new(move |_| {
                        if let Some(p) = cancel.upgrade()
                            && p.activity() == ScrollActivity::Drag
                        {
                            let _ = p.cancel_activity();
                        }
                    })),
                },
            )),
            tap: None,
        };
        let detector = GestureDetector::new(&self.binding, config)
            .map_err(ScrollError::Binding)?
            .behavior(HitTestBehavior::Opaque)
            .sequence_key((
                self.position.identity(),
                self.axis == Axis::Vertical,
                self.reverse,
            ));
        Ok(detector.into_widget(Widget::render_object(self, vec![child])))
    }
}
fn main(axis: Axis, size: egui::Vec2) -> f32 {
    if axis == Axis::Vertical {
        size.y
    } else {
        size.x
    }
}
fn child_constraints(axis: Axis, c: BoxConstraints) -> Result<BoxConstraints, LayoutError> {
    if axis == Axis::Vertical {
        BoxConstraints::new(
            egui::vec2(c.min().x, 0.0),
            egui::vec2(c.max().x, f32::INFINITY),
        )
    } else {
        BoxConstraints::new(
            egui::vec2(0.0, c.min().y),
            egui::vec2(f32::INFINITY, c.max().y),
        )
    }
}
pub struct ScrollViewportRenderBox {
    position: ScrollPosition,
    axis: Axis,
    reverse: bool,
    scale: Rc<Cell<Option<f32>>>,
    dimensions: Rc<Cell<Option<ScrollDimensions>>>,
    lease: Option<Rc<()>>,
    subscription: Option<Subscription>,
    task: Option<Rc<DimensionsTask>>,
}
impl ScrollViewportRenderBox {
    fn bind(&mut self, config: &SingleChildScrollView, ctx: &RenderContext) {
        self.axis = config.axis;
        self.reverse = config.reverse;
        config.dimensions.set(self.dimensions.get());
        self.dimensions = config.dimensions.clone();
        config.scale.set(self.scale.get());
        self.scale = config.scale.clone();
        if !self.position.same_source(&config.position) {
            self.release();
            self.position = config.position.clone();
        }
        self.acquire(ctx);
    }
    fn acquire(&mut self, ctx: &RenderContext) {
        if self.lease.is_none()
            && let Ok(lease) = self.position.claim_viewport()
        {
            self.lease = Some(lease);
            let handle = ctx.handle();
            self.subscription = self
                .position
                .subscribe(Rc::new(move || {
                    let _ = handle.mark_needs_layout();
                }))
                .ok();
            self.task = Some(DimensionsTask::new(
                ctx.egui(),
                &self.position,
                ctx.handle(),
            ));
        }
    }
    fn release(&mut self) {
        if let Some(task) = self.task.take() {
            task.active.set(false);
            task.pending.set(None);
        }
        self.subscription = None;
        self.lease = None;
    }
    fn validate(&self, children: usize) -> Result<(), LayoutError> {
        if children != 1 {
            return Err(LayoutError::InvalidConfiguration(
                "SingleChildScrollView requires one child",
            ));
        }
        if self.position.is_disposed() || self.lease.is_none() {
            return Err(LayoutError::InvalidConfiguration(
                "scroll position is disposed or already has a viewport",
            ));
        }
        Ok(())
    }
}
impl Drop for ScrollViewportRenderBox {
    fn drop(&mut self) {
        self.release();
    }
}
impl RenderObjectWidget for SingleChildScrollView {
    type Object = ScrollViewportRenderBox;
    fn create_render_object(&self, ctx: &RenderContext) -> Self::Object {
        let mut object = ScrollViewportRenderBox {
            position: self.position.clone(),
            axis: self.axis,
            reverse: self.reverse,
            scale: self.scale.clone(),
            dimensions: self.dimensions.clone(),
            lease: None,
            subscription: None,
            task: None,
        };
        object.bind(self, ctx);
        object
    }
    fn update_render_object(&self, object: &mut Self::Object, ctx: &RenderContext) {
        object.bind(self, ctx);
    }
}
impl RenderBox for ScrollViewportRenderBox {
    fn layout_revision(&self) -> u64 {
        u64::from(self.position.pixels().to_bits()) | (u64::from(self.position.is_disposed()) << 32)
    }
    fn dry_layout(
        &self,
        c: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<egui::Vec2, LayoutError> {
        self.validate(children.len())?;
        c.constrain(children.dry_layout(0, child_constraints(self.axis, c)?)?)
    }
    fn dry_baseline(
        &self,
        _: BoxConstraints,
        _: TextBaseline,
        children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        self.validate(children.len())?;
        Ok(None)
    }
    fn intrinsic(
        &self,
        _: IntrinsicDimension,
        _: f32,
        _: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        Err(LayoutError::UnsupportedQuery(
            "scroll viewport does not expose intrinsic dimensions",
        ))
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        ctx: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        self.acquire(ctx);
        self.validate(children.len())?;
        let content = children.layout(0, child_constraints(self.axis, c)?)?;
        let size = c.constrain(content)?;
        let extent = (main(self.axis, content) - main(self.axis, size)).max(0.0);
        let offset = self.position.pixels().clamp(0.0, extent);
        let translation = if self.reverse {
            main(self.axis, size) - main(self.axis, content) + offset
        } else {
            -offset
        };
        children.place(
            0,
            if self.axis == Axis::Vertical {
                egui::vec2(0.0, translation)
            } else {
                egui::vec2(translation, 0.0)
            },
        )?;
        let dimensions = ScrollDimensions::new(0.0, extent, main(self.axis, size), self.axis)
            .map_err(|_| LayoutError::InvalidConfiguration("invalid viewport dimensions"))?;
        self.dimensions.set(Some(dimensions));
        if let Some(task) = &self.task {
            task.pending.set(Some(dimensions));
            if self.position.dimensions() != Some(dimensions) {
                ctx.egui().request_repaint_of(task.viewport);
            }
        }
        let mut geometry = BoxGeometry::new(size);
        geometry.clip = Clip::HardEdge;
        Ok(geometry)
    }
    fn paint(&mut self, ui: &mut egui::Ui, _: &RenderContext) {
        let scale = ui
            .ctx()
            .layer_transform_to_global(ui.layer_id())
            .map_or(1.0, |t| t.scaling);
        self.scale
            .set((scale.is_finite() && scale > 0.0).then_some(scale));
    }
    fn dispose(&mut self) {
        self.release();
    }
}

#[derive(Clone, Copy)]
struct Driver(u64);
static NEXT_DRIVER: AtomicU64 = AtomicU64::new(1);
thread_local! {static TASKS:RefCell<HashMap<u64,Vec<Weak<DimensionsTask>>>>=RefCell::new(HashMap::new());}
struct DimensionsTask {
    key: u64,
    viewport: egui::ViewportId,
    active: Cell<bool>,
    pending: Cell<Option<ScrollDimensions>>,
    position: WeakScrollPosition,
    handle: RenderHandle,
}
impl DimensionsTask {
    fn new(ctx: &egui::Context, position: &ScrollPosition, handle: RenderHandle) -> Rc<Self> {
        let (key, new) = ctx.data_mut(|data| {
            let id = egui::Id::new("rust-flutter-scroll-dimensions");
            if let Some(driver) = data.get_temp::<Driver>(id) {
                (driver.0, false)
            } else {
                let key = NEXT_DRIVER.fetch_add(1, Ordering::Relaxed);
                data.insert_temp(id, Driver(key));
                (key, true)
            }
        });
        let task = Rc::new(Self {
            key,
            viewport: ctx.viewport_id(),
            active: Cell::new(true),
            pending: Cell::new(None),
            position: position.downgrade(),
            handle,
        });
        TASKS.with(|tasks| {
            tasks
                .borrow_mut()
                .entry(key)
                .or_default()
                .push(Rc::downgrade(&task))
        });
        if new {
            ctx.on_end_pass(
                "rust-flutter scroll dimensions",
                Arc::new(move |ctx| {
                    let deliveries = TASKS.with(|tasks| {
                        let mut tasks = tasks.borrow_mut();
                        let mut deliveries = vec![];
                        if let Some(entries) = tasks.get_mut(&key) {
                            entries.retain(|weak| weak.strong_count() != 0);
                            for weak in entries.iter() {
                                if let Some(task) = weak.upgrade()
                                    && task.active.get()
                                    && task.viewport == ctx.viewport_id()
                                {
                                    let pending = task.pending.take();
                                    deliveries.push((task, pending));
                                }
                            }
                            if entries.is_empty() {
                                tasks.remove(&key);
                            }
                        }
                        deliveries
                    });
                    for (task, pending) in deliveries {
                        if task.active.get()
                            && task.handle.mounted()
                            && let (Some(position), Some(dimensions)) =
                                (task.position.upgrade(), pending)
                            && position.apply_dimensions(dimensions) == Err(ScrollError::Superseded)
                        {
                            task.pending.set(Some(dimensions));
                            ctx.request_repaint_of(task.viewport);
                        }
                    }
                }),
            );
        }
        task
    }
}
impl Drop for DimensionsTask {
    fn drop(&mut self) {
        let _ = TASKS.try_with(|tasks| {
            if let Ok(mut tasks) = tasks.try_borrow_mut()
                && let Some(entries) = tasks.get_mut(&self.key)
            {
                entries.retain(|weak| weak.strong_count() != 0);
                if entries.is_empty() {
                    tasks.remove(&self.key);
                }
            }
        });
    }
}
