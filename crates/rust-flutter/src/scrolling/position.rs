use super::ScrollError;
use crate::{
    animation::{AnimationConfig, AnimationController, AnimationOutcome, AnimationRun, Curve},
    egui,
    layout::Axis,
    notifier::{ChangeNotifier, Disposed, Listenable, Listener, Subscription, ValueListenable},
    physics::{BoundedFrictionSimulation, FrictionSimulation},
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollDimensions {
    min: f32,
    max: f32,
    viewport: f32,
    axis: Axis,
}
impl ScrollDimensions {
    pub fn new(min: f32, max: f32, viewport: f32, axis: Axis) -> Result<Self, ScrollError> {
        if !min.is_finite()
            || !max.is_finite()
            || min > max
            || !viewport.is_finite()
            || viewport < 0.0
        {
            return Err(ScrollError::InvalidDimensions);
        }
        Ok(Self {
            min,
            max,
            viewport,
            axis,
        })
    }
    pub fn min_scroll_extent(self) -> f32 {
        self.min
    }
    pub fn max_scroll_extent(self) -> f32 {
        self.max
    }
    pub fn viewport_dimension(self) -> f32 {
        self.viewport
    }
    pub fn axis(self) -> Axis {
        self.axis
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollMetrics {
    pub pixels: f32,
    pub dimensions: ScrollDimensions,
}
impl ScrollMetrics {
    pub fn extent_before(self) -> f64 {
        (f64::from(self.pixels) - f64::from(self.dimensions.min)).max(0.0)
    }
    pub fn extent_after(self) -> f64 {
        (f64::from(self.dimensions.max) - f64::from(self.pixels)).max(0.0)
    }
    pub fn extent_inside(self) -> f64 {
        f64::from(self.dimensions.viewport)
    }
    pub fn at_edge(self) -> bool {
        self.pixels <= self.dimensions.min || self.pixels >= self.dimensions.max
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollActivity {
    Idle,
    Drag,
    Driven,
    Ballistic,
}
struct State {
    command: Rc<()>,
    viewport: Weak<()>,
    pixels: f32,
    dimensions: Option<ScrollDimensions>,
    activity: ScrollActivity,
    disposed: bool,
}
struct Inner {
    state: RefCell<State>,
    drive: AnimationController,
    pixels: ChangeNotifier,
    metrics: ChangeNotifier,
    activity: ChangeNotifier,
    depth: Cell<usize>,
    subscriptions: RefCell<Vec<Subscription>>,
}
impl Drop for Inner {
    fn drop(&mut self) {
        let _ = self.drive.dispose();
    }
}
#[derive(Clone)]
pub struct ScrollPosition(Rc<Inner>);
#[derive(Clone)]
pub struct WeakScrollPosition(Weak<Inner>);
impl WeakScrollPosition {
    pub fn upgrade(&self) -> Option<ScrollPosition> {
        self.0.upgrade().map(ScrollPosition)
    }
}
impl ScrollPosition {
    pub fn new(ctx: &egui::Context, initial_offset: f32) -> Result<Self, ScrollError> {
        if !initial_offset.is_finite() {
            return Err(ScrollError::InvalidOffset);
        }
        let drive = AnimationController::new(
            ctx,
            AnimationConfig {
                lower_bound: f32::MIN,
                upper_bound: f32::MAX,
                value: Some(initial_offset),
                ..Default::default()
            },
        )
        .map_err(ScrollError::Animation)?;
        let position = Self(Rc::new(Inner {
            state: RefCell::new(State {
                command: Rc::new(()),
                viewport: Weak::new(),
                pixels: initial_offset,
                dimensions: None,
                activity: ScrollActivity::Idle,
                disposed: false,
            }),
            drive,
            pixels: ChangeNotifier::new(),
            metrics: ChangeNotifier::new(),
            activity: ChangeNotifier::new(),
            depth: Cell::new(0),
            subscriptions: RefCell::new(vec![]),
        }));
        let weak = position.downgrade();
        let pixels = position
            .0
            .drive
            .subscribe(Rc::new(move || {
                if let Some(position) = weak.upgrade() {
                    position.sync_pixels();
                }
            }))
            .map_err(|_| ScrollError::Disposed)?;
        let weak = position.downgrade();
        let status = position
            .0
            .drive
            .subscribe_status(Rc::new(move || {
                if let Some(position) = weak.upgrade()
                    && !position.0.drive.is_animating()
                    && matches!(
                        position.activity(),
                        ScrollActivity::Driven | ScrollActivity::Ballistic
                    )
                {
                    position.set_activity(ScrollActivity::Idle);
                }
            }))
            .map_err(|_| ScrollError::Disposed)?;
        *position.0.subscriptions.borrow_mut() = vec![pixels, status];
        Ok(position)
    }
    pub(super) fn identity(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }
    pub(super) fn claim_viewport(&self) -> Result<Rc<()>, ScrollError> {
        self.check()?;
        let mut state = self.0.state.borrow_mut();
        if state.viewport.upgrade().is_some() {
            return Err(ScrollError::ViewportAlreadyAttached);
        }
        let lease = Rc::new(());
        state.viewport = Rc::downgrade(&lease);
        Ok(lease)
    }
    pub fn downgrade(&self) -> WeakScrollPosition {
        WeakScrollPosition(Rc::downgrade(&self.0))
    }
    pub fn pixels(&self) -> f32 {
        self.0.state.borrow().pixels
    }
    pub fn dimensions(&self) -> Option<ScrollDimensions> {
        self.0.state.borrow().dimensions
    }
    pub fn metrics(&self) -> Option<ScrollMetrics> {
        self.dimensions().map(|dimensions| ScrollMetrics {
            pixels: self.pixels(),
            dimensions,
        })
    }
    pub fn activity(&self) -> ScrollActivity {
        self.0.state.borrow().activity
    }
    pub fn is_scrolling(&self) -> bool {
        self.activity() != ScrollActivity::Idle
    }
    pub fn is_disposed(&self) -> bool {
        self.0.state.borrow().disposed
    }
    fn check(&self) -> Result<(), ScrollError> {
        if self.is_disposed() {
            Err(ScrollError::Disposed)
        } else {
            Ok(())
        }
    }
    fn command(&self) -> Rc<()> {
        let command = Rc::new(());
        self.0.state.borrow_mut().command = command.clone();
        command
    }
    pub(super) fn command_stamp(&self) -> Rc<()> {
        self.0.state.borrow().command.clone()
    }
    pub(super) fn current(&self, command: &Rc<()>) -> bool {
        !self.is_disposed() && Rc::ptr_eq(command, &self.0.state.borrow().command)
    }
    fn emit(&self, notifier: &ChangeNotifier) {
        if self.is_disposed() {
            return;
        }
        self.0.depth.set(self.0.depth.get() + 1);
        let _guard = NotifyGuard(&self.0.depth);
        let _ = notifier.notify_listeners();
    }
    fn set_activity(&self, activity: ScrollActivity) {
        let changed = {
            let mut state = self.0.state.borrow_mut();
            if state.disposed {
                return;
            }
            let changed = state.activity != activity;
            state.activity = activity;
            changed
        };
        if changed {
            self.emit(&self.0.activity);
        }
    }
    fn sync_pixels(&self) {
        let value = self.0.drive.value();
        let changed = {
            let mut state = self.0.state.borrow_mut();
            if state.disposed {
                return;
            }
            let value = state
                .dimensions
                .map_or(value, |d| value.clamp(d.min, d.max));
            let changed = state.pixels != value;
            state.pixels = value;
            changed
        };
        if changed {
            self.emit(&self.0.pixels);
        }
    }
    /// Publish complete validated dimensions. Changes cancel existing activity
    /// and correct the current offset; identical dimensions are a no-op.
    pub fn apply_dimensions(&self, dimensions: ScrollDimensions) -> Result<bool, ScrollError> {
        self.check()?;
        if self.dimensions() == Some(dimensions) {
            return Ok(false);
        }
        let command = self.command();
        self.0.drive.stop(true).map_err(ScrollError::Animation)?;
        self.check()?;
        if !self.current(&command) {
            return Err(ScrollError::Superseded);
        }
        let (pixels, pixel_changed, activity_changed) = {
            let mut state = self.0.state.borrow_mut();
            let pixels = state.pixels.clamp(dimensions.min, dimensions.max);
            let changed = state.pixels != pixels;
            let activity_changed = state.activity != ScrollActivity::Idle;
            state.dimensions = Some(dimensions);
            state.pixels = pixels;
            state.activity = ScrollActivity::Idle;
            (pixels, changed, activity_changed)
        };
        self.0
            .drive
            .set_value(pixels)
            .map_err(ScrollError::Animation)?;
        if pixel_changed {
            self.emit(&self.0.pixels);
        }
        self.emit(&self.0.metrics);
        if activity_changed {
            self.emit(&self.0.activity);
        }
        Ok(true)
    }
    pub fn jump_to(&self, offset: f32) -> Result<(), ScrollError> {
        self.check()?;
        if !offset.is_finite() {
            return Err(ScrollError::InvalidOffset);
        }
        let target = self
            .dimensions()
            .map_or(offset, |d| offset.clamp(d.min, d.max));
        let command = self.command();
        self.0
            .drive
            .set_value(target)
            .map_err(ScrollError::Animation)?;
        if self.current(&command) {
            self.set_activity(ScrollActivity::Idle);
        }
        Ok(())
    }
    pub fn begin_drag(&self) -> Result<(), ScrollError> {
        self.check()?;
        if self.dimensions().is_none() {
            return Err(ScrollError::NoDimensions);
        }
        let command = self.command();
        self.0.drive.stop(true).map_err(ScrollError::Animation)?;
        self.check()?;
        if !self.current(&command) {
            return Err(ScrollError::Superseded);
        }
        self.set_activity(ScrollActivity::Drag);
        Ok(())
    }
    /// Positive delta moves toward the maximum content offset. Returns the
    /// unapplied part in logical points, widened to avoid finite f32 overflow.
    pub fn drag_by(&self, delta: f32) -> Result<f64, ScrollError> {
        self.check()?;
        if !delta.is_finite() {
            return Err(ScrollError::InvalidOffset);
        }
        if self.activity() != ScrollActivity::Drag {
            return Err(ScrollError::NotDragging);
        }
        let d = self.dimensions().ok_or(ScrollError::NoDimensions)?;
        let old = self.pixels();
        let new =
            (f64::from(old) + f64::from(delta)).clamp(f64::from(d.min), f64::from(d.max)) as f32;
        self.command();
        self.0
            .drive
            .set_value(new)
            .map_err(ScrollError::Animation)?;
        Ok(f64::from(delta) - (f64::from(new) - f64::from(old)))
    }
    pub fn cancel_activity(&self) -> Result<(), ScrollError> {
        self.check()?;
        let command = self.command();
        self.0.drive.stop(true).map_err(ScrollError::Animation)?;
        if !self.current(&command) {
            return Err(ScrollError::Superseded);
        }
        self.set_activity(ScrollActivity::Idle);
        Ok(())
    }
    pub fn animate_to(
        &self,
        target: f32,
        duration: Duration,
        curve: Curve,
    ) -> Result<AnimationRun, ScrollError> {
        self.check()?;
        if !target.is_finite() {
            return Err(ScrollError::InvalidOffset);
        }
        if duration.is_zero() {
            return Err(ScrollError::InvalidDuration);
        }
        let d = self.dimensions().ok_or(ScrollError::NoDimensions)?;
        let command = self.command();
        let run = self
            .0
            .drive
            .animate_to(target.clamp(d.min, d.max), Some(duration), curve)
            .map_err(ScrollError::Animation)?;
        if self.current(&command) {
            self.started(&run, ScrollActivity::Driven);
        }
        Ok(run)
    }
    /// Explicit fluid-drag policy, not an Android/macOS scroll-physics replica.
    /// Requires a drag; velocity is in content-offset points per second.
    pub fn end_drag(&self, velocity: f32, drag: f64) -> Result<AnimationRun, ScrollError> {
        self.check()?;
        if self.activity() != ScrollActivity::Drag {
            return Err(ScrollError::NotDragging);
        }
        if !velocity.is_finite() {
            return Err(ScrollError::InvalidOffset);
        }
        let d = self.dimensions().ok_or(ScrollError::NoDimensions)?;
        let motion = FrictionSimulation::new(drag, f64::from(self.pixels()), f64::from(velocity))
            .and_then(|m| BoundedFrictionSimulation::new(m, f64::from(d.min), f64::from(d.max)))
            .map_err(|e| ScrollError::Animation(crate::animation::AnimationError::Simulation(e)))?;
        let command = self.command();
        let run = self
            .0
            .drive
            .animate_with(Rc::new(motion))
            .map_err(ScrollError::Animation)?;
        if self.current(&command) {
            self.started(&run, ScrollActivity::Ballistic);
        }
        Ok(run)
    }
    fn started(&self, run: &AnimationRun, activity: ScrollActivity) {
        if run.outcome() == Some(AnimationOutcome::Completed) {
            self.set_activity(ScrollActivity::Idle);
        } else if run.outcome().is_none() {
            self.set_activity(activity);
        }
    }
    pub fn subscribe_metrics(&self, listener: Listener) -> Result<Subscription, Disposed> {
        if self.is_disposed() {
            Err(Disposed)
        } else {
            self.0.metrics.subscribe(listener)
        }
    }
    pub fn subscribe_activity(&self, listener: Listener) -> Result<Subscription, Disposed> {
        if self.is_disposed() {
            Err(Disposed)
        } else {
            self.0.activity.subscribe(listener)
        }
    }
    pub fn dispose(&self) -> Result<(), ScrollError> {
        self.check()?;
        if self.0.depth.get() != 0 {
            return Err(ScrollError::DuringNotification);
        }
        {
            let mut state = self.0.state.borrow_mut();
            state.command = Rc::new(());
            state.disposed = true;
            state.activity = ScrollActivity::Idle;
        }
        self.0.drive.dispose().map_err(ScrollError::Animation)?;
        let subscriptions = self.0.subscriptions.take();
        drop(subscriptions);
        self.0.pixels.dispose().expect("dispatch depth checked");
        self.0.metrics.dispose().expect("dispatch depth checked");
        self.0.activity.dispose().expect("dispatch depth checked");
        Ok(())
    }
}
impl Listenable for ScrollPosition {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
        if self.is_disposed() {
            Err(Disposed)
        } else {
            self.0.pixels.subscribe(listener)
        }
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl ValueListenable for ScrollPosition {
    type Value = f32;
    fn value(&self) -> f32 {
        self.pixels()
    }
}
struct NotifyGuard<'a>(&'a Cell<usize>);
impl Drop for NotifyGuard<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}
