use super::{ScrollActivity, ScrollError, ScrollMetrics, ScrollPosition};
use crate::{
    egui::{self, Color32, Pos2, Rect, Vec2, vec2},
    element::{RenderBox, RenderContext, RenderObjectWidget, Widget},
    gestures::*,
    layout::*,
    notifier::{Listenable, Subscription},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

/// Always-visible scrollbar styling in logical points. The bar overlays content.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarStyle {
    pub thickness: f32,
    pub margin: f32,
    pub min_thumb_length: f32,
    pub thumb_color: Color32,
    pub track_color: Color32,
}
impl Default for ScrollbarStyle {
    fn default() -> Self {
        Self {
            thickness: 8.0,
            margin: 2.0,
            min_thumb_length: 18.0,
            thumb_color: Color32::from_rgb(185, 199, 216),
            track_color: Color32::from_rgb(78, 88, 102),
        }
    }
}
impl ScrollbarStyle {
    fn validate(self) -> Result<(), ScrollError> {
        if !self.thickness.is_finite()
            || self.thickness <= 0.0
            || !self.margin.is_finite()
            || self.margin < 0.0
            || !self.min_thumb_length.is_finite()
            || self.min_thumb_length <= 0.0
        {
            Err(ScrollError::InvalidScrollbarStyle)
        } else {
            Ok(())
        }
    }
}
#[derive(Clone, Copy)]
struct Geometry {
    axis: Axis,
    thumb: Rect,
    length: f32,
    cross: f32,
    min: f64,
    max: f64,
    viewport: f64,
    reverse: bool,
}
impl Geometry {
    fn new(
        metrics: ScrollMetrics,
        size: Vec2,
        style: ScrollbarStyle,
        reverse: bool,
        axis: Axis,
    ) -> Option<Self> {
        let d = metrics.dimensions;
        if d.axis() != axis {
            return None;
        }
        let range = f64::from(d.max_scroll_extent()) - f64::from(d.min_scroll_extent());
        let viewport = f64::from(d.viewport_dimension());
        let length = (f64::from(main(axis, size)) - 2.0 * f64::from(style.margin)).max(0.0) as f32;
        let cross = style
            .thickness
            .min((main(other(axis), size) - style.margin).max(0.0));
        if range <= 0.0 || viewport <= 0.0 || length <= 0.0 || cross <= 0.0 {
            return None;
        }
        let thumb_length = (f64::from(length) * viewport / (range + viewport))
            .max(f64::from(style.min_thumb_length))
            .min(f64::from(length)) as f32;
        let fraction = ((f64::from(metrics.pixels) - f64::from(d.min_scroll_extent())) / range)
            .clamp(0.0, 1.0);
        let fraction = if reverse { 1.0 - fraction } else { fraction };
        let offset = (fraction * f64::from(length - thumb_length)) as f32;
        Some(Self {
            axis,
            thumb: Rect::from_min_size(pos(axis, offset, 0.0), size_for(axis, thumb_length, cross)),
            length,
            cross,
            min: f64::from(d.min_scroll_extent()),
            max: f64::from(d.max_scroll_extent()),
            viewport,
            reverse,
        })
    }
    fn travel(self) -> f64 {
        f64::from(self.length) - f64::from(main(self.axis, self.thumb.size()))
    }
    fn sign(self) -> f64 {
        if self.reverse { -1.0 } else { 1.0 }
    }
}
fn main(axis: Axis, size: Vec2) -> f32 {
    if axis == Axis::Vertical {
        size.y
    } else {
        size.x
    }
}
fn other(axis: Axis) -> Axis {
    if axis == Axis::Vertical {
        Axis::Horizontal
    } else {
        Axis::Vertical
    }
}
fn size_for(axis: Axis, main: f32, cross: f32) -> Vec2 {
    if axis == Axis::Vertical {
        vec2(cross, main)
    } else {
        vec2(main, cross)
    }
}
fn pos(axis: Axis, main: f32, cross: f32) -> Pos2 {
    size_for(axis, main, cross).to_pos2()
}
#[derive(Clone, Copy)]
struct Painted {
    origin: Pos2,
    scale: f32,
    enabled: bool,
}
#[derive(Default)]
struct State {
    geometry: Cell<Option<Geometry>>,
    painted: Cell<Option<Painted>>,
    active: Cell<bool>,
    drag: RefCell<Rc<DragOwner>>,
}
impl State {
    fn input(&self) -> Option<(Geometry, Painted)> {
        if !self.active.get() {
            return None;
        }
        Some((
            self.geometry.get()?,
            self.painted.get().filter(|p| p.enabled)?,
        ))
    }
}

// Shared across configuration rebuilds; command stamps keep callback-created
// application motion separate from this pointer sequence's drag ownership.
#[derive(Default)]
struct DragOwner {
    command: RefCell<Option<Rc<()>>>,
    active: Cell<bool>,
}
impl DragOwner {
    fn finish_drag(&self, position: &ScrollPosition) {
        let command = self.command.borrow_mut().take();
        if command.is_some_and(|command| position.current(&command))
            && position.activity() == ScrollActivity::Drag
        {
            let _ = position.cancel_activity();
        }
    }
    fn adopt(&self, position: &ScrollPosition, command: Rc<()>) {
        if position.current(&command) {
            if self.active.get() {
                *self.command.borrow_mut() = Some(command);
            } else {
                let _ = position.cancel_activity();
            }
        } else {
            self.command.borrow_mut().take();
        }
    }
}

/// Explicit-position retained scrollbar. Wrap a scroll viewport using the same
/// position and binding. Axis and reverse must match the viewport. This is not a Material/Cupertino themed scrollbar.
#[derive(Clone)]
pub struct Scrollbar {
    position: ScrollPosition,
    binding: GestureBinding,
    style: ScrollbarStyle,
    reverse: bool,
    interactive: bool,
    axis: Axis,
    state: Rc<State>,
}
impl Scrollbar {
    pub fn new(position: &ScrollPosition, binding: &GestureBinding) -> Result<Self, ScrollError> {
        if position.is_disposed() || binding.is_disposed() {
            return Err(ScrollError::Disposed);
        }
        Ok(Self {
            position: position.clone(),
            binding: binding.clone(),
            style: ScrollbarStyle::default(),
            reverse: false,
            interactive: true,
            axis: position.dimensions().map_or(Axis::Vertical, |d| d.axis()),
            state: Rc::new(State::default()),
        })
    }
    pub fn style(mut self, style: ScrollbarStyle) -> Result<Self, ScrollError> {
        style.validate()?;
        self.style = style;
        Ok(self)
    }
    /// Select the viewport axis. Defaults to known dimensions, or vertical.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
    pub fn reverse(mut self, reverse: bool) -> Self {
        self.reverse = reverse;
        self
    }
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }
    pub fn into_widget(mut self, child: Widget) -> Result<Widget, ScrollError> {
        self.state = Rc::new(State::default());
        self.style.validate()?;
        if self.position.is_disposed() || self.binding.is_disposed() {
            return Err(ScrollError::Disposed);
        }
        let mut thumb = Widget::render_object(
            Part {
                state: self.state.clone(),
                style: self.style,
                thumb: true,
            },
            vec![],
        );
        if self.interactive {
            let down = self.position.downgrade();
            let down_state = self.state.clone();
            let start = self.position.downgrade();
            let start_state = self.state.clone();
            let end = self.position.downgrade();
            let end_state = self.state.clone();
            let cancel = self.position.downgrade();
            let cancel_state = self.state.clone();
            let update = self.position.downgrade();
            let state = self.state.clone();
            thumb = GestureDetector::new(
                &self.binding,
                GestureTargetConfig {
                    pan: Some((
                        PanConfig {
                            axis: Some(if self.axis == Axis::Vertical {
                                DragAxis::Vertical
                            } else {
                                DragAxis::Horizontal
                            }),
                            start_behavior: DragStartBehavior::Down,
                            ..Default::default()
                        },
                        PanCallbacks {
                            on_down: Some(Rc::new(move |_| {
                                if down_state.input().is_some()
                                    && let Some(p) = down.upgrade()
                                {
                                    down_state.drag.borrow().command.borrow_mut().take();
                                    let _ = p.cancel_activity();
                                }
                            })),
                            on_start: Some(Rc::new(move |_| {
                                let owner = start_state.drag.borrow().clone();
                                if start_state.input().is_some()
                                    && let Some(p) = start.upgrade()
                                    && let Ok(command) = p.begin_drag_with_stamp()
                                {
                                    owner.adopt(&p, command);
                                }
                            })),
                            on_end: Some(Rc::new(move |_| {
                                if let Some(p) = end.upgrade() {
                                    let owner = end_state.drag.borrow().clone();
                                    owner.finish_drag(&p);
                                }
                            })),
                            on_cancel: Some(Rc::new(move |_| {
                                if let Some(p) = cancel.upgrade() {
                                    let owner = cancel_state.drag.borrow().clone();
                                    owner.finish_drag(&p);
                                }
                            })),
                            on_update: Some(Rc::new(move |d| {
                                if let (Some(p), Some((g, painted))) =
                                    (update.upgrade(), state.input())
                                    && !p.is_disposed()
                                    && g.travel() > 0.0
                                {
                                    let owner = state.drag.borrow().clone();
                                    let owns = owner
                                        .command
                                        .borrow()
                                        .as_ref()
                                        .is_some_and(|command| p.current(command));
                                    if !owns {
                                        owner.command.borrow_mut().take();
                                        return;
                                    }
                                    let target = (f64::from(p.pixels())
                                        + f64::from(main(g.axis, d.delta))
                                            / f64::from(painted.scale)
                                            * (g.max - g.min)
                                            / g.travel()
                                            * g.sign())
                                    .clamp(g.min, g.max)
                                        as f32;
                                    if let Ok(command) = p.drag_to_with_stamp(target) {
                                        owner.adopt(&p, command);
                                    }
                                }
                            })),
                        },
                    )),
                    ..Default::default()
                },
            )
            .map_err(ScrollError::Binding)?
            .behavior(HitTestBehavior::Translucent)
            .sequence_key((
                self.position.identity(),
                self.reverse,
                self.axis == Axis::Vertical,
                self.style.thickness.to_bits(),
                self.style.min_thumb_length.to_bits(),
                self.style.margin.to_bits(),
            ))
            .into_widget(thumb);
        }
        let mut track = Widget::render_object(
            Part {
                state: self.state.clone(),
                style: self.style,
                thumb: false,
            },
            vec![thumb],
        );
        if self.interactive {
            let position = self.position.downgrade();
            let state = self.state.clone();
            track = GestureDetector::new(
                &self.binding,
                GestureTargetConfig {
                    tap: Some((
                        TapConfig::default(),
                        TapCallbacks {
                            on_tap: Some(Rc::new(move |d| {
                                if let (Some(p), Some((g, painted))) =
                                    (position.upgrade(), state.input())
                                    && !p.is_disposed()
                                {
                                    let point = (d.up.position - painted.origin) / painted.scale;
                                    let at = main(g.axis, point);
                                    let before = main(g.axis, g.thumb.min.to_vec2());
                                    let after = main(g.axis, g.thumb.max.to_vec2());
                                    let direction = if at < before {
                                        -1.0
                                    } else if at > after {
                                        1.0
                                    } else {
                                        0.0
                                    };
                                    if direction != 0.0 {
                                        let _ = p.jump_to(
                                            (f64::from(p.pixels())
                                                + direction * g.sign() * g.viewport * 0.8)
                                                .clamp(g.min, g.max)
                                                as f32,
                                        );
                                    }
                                }
                            })),
                            ..Default::default()
                        },
                    )),
                    ..Default::default()
                },
            )
            .map_err(ScrollError::Binding)?
            .behavior(HitTestBehavior::Translucent)
            .sequence_key((
                self.position.identity(),
                self.reverse,
                self.axis == Axis::Vertical,
            ))
            .into_widget(track);
        }
        Ok(Widget::render_object(self, vec![child, track]))
    }
}

pub struct ScrollbarRenderBox {
    config: Scrollbar,
    subscriptions: Vec<Subscription>,
}
impl ScrollbarRenderBox {
    fn configure(&mut self, config: &Scrollbar, ctx: &RenderContext) {
        if !Rc::ptr_eq(&self.config.state, &config.state) {
            if self.config.position.identity() == config.position.identity() {
                *config.state.drag.borrow_mut() = self.config.state.drag.borrow().clone();
            } else {
                self.config.state.drag.borrow().active.set(false);
            }
            config.state.geometry.set(self.config.state.geometry.get());
            config.state.painted.set(self.config.state.painted.get());
            self.config.state.active.set(false);
        }
        self.subscriptions.clear();
        self.config = config.clone();
        self.config.state.active.set(true);
        self.config
            .state
            .drag
            .borrow()
            .active
            .set(config.interactive);
        for metrics in [false, true] {
            let handle = ctx.handle();
            let callback: crate::notifier::Listener = Rc::new(move || {
                let _ = handle.mark_needs_layout();
            });
            let sub = if metrics {
                config.position.subscribe_metrics(callback)
            } else {
                config.position.subscribe(callback)
            };
            if let Ok(sub) = sub {
                self.subscriptions.push(sub);
            }
        }
    }
    fn validate(children: usize) -> Result<(), LayoutError> {
        if children == 2 {
            Ok(())
        } else {
            Err(LayoutError::InvalidConfiguration(
                "Scrollbar requires content and track",
            ))
        }
    }
}
impl RenderObjectWidget for Scrollbar {
    type Object = ScrollbarRenderBox;
    fn create_render_object(&self, ctx: &RenderContext) -> Self::Object {
        let mut object = ScrollbarRenderBox {
            config: self.clone(),
            subscriptions: vec![],
        };
        object.configure(self, ctx);
        object
    }
    fn update_render_object(&self, object: &mut Self::Object, ctx: &RenderContext) {
        object.configure(self, ctx);
    }
}
impl RenderBox for ScrollbarRenderBox {
    fn layout_revision(&self) -> u64 {
        u64::from(self.config.position.pixels().to_bits())
            | (u64::from(self.config.position.is_disposed()) << 32)
    }
    fn dry_layout(
        &self,
        c: BoxConstraints,
        children: &BoxQueries<'_>,
    ) -> Result<Vec2, LayoutError> {
        Self::validate(children.len())?;
        children.dry_layout(0, c)
    }
    fn intrinsic(
        &self,
        d: IntrinsicDimension,
        e: f32,
        children: &BoxQueries<'_>,
    ) -> Result<f32, LayoutError> {
        Self::validate(children.len())?;
        children.intrinsic(0, d, e)
    }
    fn dry_baseline(
        &self,
        c: BoxConstraints,
        k: TextBaseline,
        children: &BoxQueries<'_>,
    ) -> Result<Option<f32>, LayoutError> {
        Self::validate(children.len())?;
        children.dry_baseline(0, c, k)
    }
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        Self::validate(children.len())?;
        let size = children.layout(0, c)?;
        let geometry = (!self.config.position.is_disposed())
            .then(|| self.config.position.metrics())
            .flatten()
            .and_then(|m| {
                Geometry::new(
                    m,
                    size,
                    self.config.style,
                    self.config.reverse,
                    self.config.axis,
                )
            });
        self.config.state.geometry.set(geometry);
        let (track_size, offset) = if let Some(g) = geometry {
            (
                size_for(g.axis, g.length, g.cross),
                size_for(
                    g.axis,
                    self.config.style.margin,
                    (main(other(g.axis), size) - self.config.style.margin - g.cross).max(0.0),
                ),
            )
        } else {
            (Vec2::ZERO, Vec2::ZERO)
        };
        children.layout(1, BoxConstraints::tight(track_size)?)?;
        children.place(0, Vec2::ZERO)?;
        children.place(1, offset)?;
        let mut result = BoxGeometry::new(size);
        result.clip = Clip::HardEdge;
        result.baselines = Baselines {
            alphabetic: children.baseline(0, TextBaseline::Alphabetic, true)?,
            ideographic: children.baseline(0, TextBaseline::Ideographic, true)?,
        };
        Ok(result)
    }
    fn dispose(&mut self) {
        self.subscriptions.clear();
        self.config.state.active.set(false);
        self.config.state.drag.borrow().active.set(false);
        self.config.state.geometry.set(None);
        self.config.state.painted.set(None);
    }
}
#[derive(Clone)]
struct Part {
    state: Rc<State>,
    style: ScrollbarStyle,
    thumb: bool,
}
struct PartRenderBox(Part);
impl RenderObjectWidget for Part {
    type Object = PartRenderBox;
    fn create_render_object(&self, _: &RenderContext) -> Self::Object {
        PartRenderBox(self.clone())
    }
    fn update_render_object(&self, o: &mut Self::Object, _: &RenderContext) {
        o.0 = self.clone();
    }
}
impl RenderBox for PartRenderBox {
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        let size = c.min();
        if !self.0.thumb {
            let thumb = self.0.state.geometry.get().map_or(Rect::ZERO, |g| g.thumb);
            children.layout(0, BoxConstraints::tight(thumb.size())?)?;
            children.place(0, thumb.min.to_vec2())?;
        }
        Ok(BoxGeometry::new(size))
    }
    fn paint(&mut self, ui: &mut egui::Ui, _: &RenderContext) {
        if !self.0.thumb {
            let transform = ui
                .ctx()
                .layer_transform_to_global(ui.layer_id())
                .unwrap_or_default();
            let origin = transform * ui.max_rect().min;
            self.0.state.painted.set(
                (transform.scaling.is_finite() && transform.scaling > 0.0 && origin.is_finite())
                    .then_some(Painted {
                        origin,
                        scale: transform.scaling,
                        enabled: ui.is_enabled(),
                    }),
            );
        }
        if self.0.state.geometry.get().is_some() && ui.max_rect().is_positive() {
            ui.painter().rect_filled(
                ui.max_rect(),
                self.0.style.thickness * 0.5,
                if self.0.thumb {
                    self.0.style.thumb_color
                } else {
                    self.0.style.track_color
                },
            );
        }
    }
}
