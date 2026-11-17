//! UI-local controllers driven automatically by their owning egui viewport.
use super::Curve;
use crate::notifier::{
    ChangeNotifier, Disposed, Listenable, Listener, Subscription, ValueListenable,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    future::Future,
    pin::Pin,
    rc::{Rc, Weak},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    task::{Context, Poll, Waker},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationStatus {
    Dismissed,
    Forward,
    Reverse,
    Completed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationOutcome {
    Completed,
    Canceled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationError {
    Disposed,
    DuringNotification,
    InvalidConfiguration(&'static str),
}
impl std::fmt::Display for AnimationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disposed => f.write_str("animation controller is disposed"),
            Self::DuringNotification => f.write_str("cannot dispose during animation notification"),
            Self::InvalidConfiguration(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for AnimationError {}
#[derive(Clone, Copy, Debug)]
pub struct AnimationConfig {
    pub lower_bound: f32,
    pub upper_bound: f32,
    pub value: Option<f32>,
    pub duration: Duration,
    pub reverse_duration: Option<Duration>,
}
impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            lower_bound: 0.0,
            upper_bound: 1.0,
            value: None,
            duration: Duration::from_millis(300),
            reverse_duration: None,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct RepeatOptions {
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub period: Option<Duration>,
    pub reverse: bool,
    pub count: Option<u32>,
}
#[derive(Default)]
struct Completion {
    outcome: Option<AnimationOutcome>,
    wakers: Vec<Waker>,
}
/// Cloneable UI-local completion future. Replacement, disposal and canceled stop
/// resolve Canceled; successful completion and stop(false) resolve Completed.
#[derive(Clone)]
pub struct AnimationRun(Rc<RefCell<Completion>>);
impl AnimationRun {
    fn pending() -> Self {
        Self(Rc::new(RefCell::new(Completion::default())))
    }
    pub fn outcome(&self) -> Option<AnimationOutcome> {
        self.0.borrow().outcome
    }
    fn finish(&self, outcome: AnimationOutcome) {
        let wakers = {
            let mut state = self.0.borrow_mut();
            if state.outcome.is_some() {
                return;
            }
            state.outcome = Some(outcome);
            std::mem::take(&mut state.wakers)
        };
        for waker in wakers {
            waker.wake();
        }
    }
}
impl Future for AnimationRun {
    type Output = AnimationOutcome;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.0.borrow_mut();
        if let Some(outcome) = state.outcome {
            return Poll::Ready(outcome);
        }
        if !state.wakers.iter().any(|w| w.will_wake(cx.waker())) {
            state.wakers.push(cx.waker().clone());
        }
        Poll::Pending
    }
}
#[derive(Clone, Copy)]
enum Simulation {
    To {
        from: f32,
        target: f32,
        seconds: f64,
        curve: Curve,
        reverse: bool,
    },
    Repeat {
        min: f32,
        max: f32,
        seconds: f64,
        phase: f64,
        reverse: bool,
        count: Option<u32>,
    },
}
struct Motion {
    start: f64,
    simulation: Simulation,
    run: AnimationRun,
}
struct State {
    config: AnimationConfig,
    value: f32,
    status: AnimationStatus,
    reported: AnimationStatus,
    reverse: bool,
    motion: Option<Motion>,
    muted: bool,
    disposed: bool,
    last_time: f64,
}
struct Inner {
    state: RefCell<State>,
    changes: ChangeNotifier,
    status_changes: ChangeNotifier,
    depth: Cell<usize>,
    ctx: egui::Context,
    viewport: egui::ViewportId,
    driver: Cell<Option<u64>>,
}
impl Drop for Inner {
    fn drop(&mut self) {
        if let Some(key) = self.driver.get() {
            let _ = DRIVERS.try_with(|drivers| {
                if let Ok(mut drivers) = drivers.try_borrow_mut()
                    && let Some(entries) = drivers.get_mut(&key)
                {
                    entries.retain(|entry| entry.strong_count() != 0);
                    if entries.is_empty() {
                        drivers.remove(&key);
                    }
                }
            });
        }
        if let Some(motion) = self.state.get_mut().motion.take() {
            motion.run.finish(AnimationOutcome::Canceled);
        }
    }
}
#[derive(Clone)]
pub struct AnimationController(Rc<Inner>);
#[derive(Clone)]
pub struct WeakAnimationController(Weak<Inner>);
impl WeakAnimationController {
    pub fn upgrade(&self) -> Option<AnimationController> {
        self.0.upgrade().map(AnimationController)
    }
}
thread_local! {static DRIVERS:RefCell<HashMap<u64,Vec<Weak<Inner>>>>=RefCell::new(HashMap::new());}
#[derive(Clone, Copy)]
struct DriverId(u64);
static NEXT_DRIVER: AtomicU64 = AtomicU64::new(1);
fn register(controller: &AnimationController) {
    let (key, new) = controller.0.ctx.data_mut(|d| {
        let id = egui::Id::new("rust-flutter-animation-driver");
        if let Some(key) = d.get_temp::<DriverId>(id) {
            (key.0, false)
        } else {
            let key = NEXT_DRIVER.fetch_add(1, Ordering::Relaxed);
            d.insert_temp(id, DriverId(key));
            (key, true)
        }
    });
    controller.0.driver.set(Some(key));
    DRIVERS.with(|drivers| {
        drivers
            .borrow_mut()
            .entry(key)
            .or_default()
            .push(Rc::downgrade(&controller.0))
    });
    if new {
        controller.0.ctx.on_begin_pass(
            "rust-flutter animations",
            Arc::new(move |ctx| {
                let controllers = DRIVERS.with(|drivers| {
                    let mut drivers = drivers.borrow_mut();
                    let mut active = vec![];
                    if let Some(entries) = drivers.get_mut(&key) {
                        entries.retain(|weak| {
                            if let Some(inner) = weak.upgrade()
                                && !inner.state.borrow().disposed
                            {
                                active.push(AnimationController(inner));
                                return true;
                            }
                            false
                        });
                        if entries.is_empty() {
                            drivers.remove(&key);
                        }
                    }
                    active
                });
                for controller in controllers {
                    if controller.0.viewport == ctx.viewport_id() {
                        controller.tick();
                    }
                }
            }),
        );
    }
}
struct DispatchGuard<'a>(&'a Cell<usize>);
impl Drop for DispatchGuard<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}
impl AnimationController {
    pub fn new(ctx: &egui::Context, config: AnimationConfig) -> Result<Self, AnimationError> {
        if !config.lower_bound.is_finite()
            || !config.upper_bound.is_finite()
            || config.lower_bound >= config.upper_bound
            || config.value.is_some_and(|v| !v.is_finite())
        {
            return Err(AnimationError::InvalidConfiguration(
                "animation bounds/value must be finite with lower < upper",
            ));
        }
        let value = config
            .value
            .unwrap_or(config.lower_bound)
            .clamp(config.lower_bound, config.upper_bound);
        let status = if value == config.lower_bound {
            AnimationStatus::Dismissed
        } else if value == config.upper_bound {
            AnimationStatus::Completed
        } else {
            AnimationStatus::Forward
        };
        let controller = Self(Rc::new(Inner {
            state: RefCell::new(State {
                config,
                value,
                status,
                reported: status,
                reverse: false,
                motion: None,
                muted: false,
                disposed: false,
                last_time: ctx.input(|i| i.time),
            }),
            changes: ChangeNotifier::new(),
            status_changes: ChangeNotifier::new(),
            depth: Cell::new(0),
            ctx: ctx.clone(),
            viewport: ctx.viewport_id(),
            driver: Cell::new(None),
        }));
        register(&controller);
        Ok(controller)
    }
    pub fn downgrade(&self) -> WeakAnimationController {
        WeakAnimationController(Rc::downgrade(&self.0))
    }
    pub fn value(&self) -> f32 {
        self.0.state.borrow().value
    }
    pub fn status(&self) -> AnimationStatus {
        self.0.state.borrow().status
    }
    pub fn is_animating(&self) -> bool {
        self.0.state.borrow().motion.is_some()
    }
    pub fn is_disposed(&self) -> bool {
        self.0.state.borrow().disposed
    }
    pub fn is_muted(&self) -> bool {
        self.0.state.borrow().muted
    }
    fn now(&self) -> f64 {
        self.0.ctx.input_for(self.0.viewport, |i| i.time)
    }
    fn check(&self) -> Result<(), AnimationError> {
        if self.is_disposed() {
            Err(AnimationError::Disposed)
        } else {
            Ok(())
        }
    }
    fn repaint(&self) {
        if self.is_animating() && !self.is_muted() {
            self.0.ctx.request_repaint_of(self.0.viewport);
        }
    }
    fn dispatch(&self, value: bool) {
        self.0.depth.set(self.0.depth.get() + 1);
        let _guard = DispatchGuard(&self.0.depth);
        if value {
            let _ = self.0.changes.notify_listeners();
        }
        let changed = {
            let mut state = self.0.state.borrow_mut();
            let changed = state.reported != state.status;
            state.reported = state.status;
            changed
        };
        if changed {
            let _ = self.0.status_changes.notify_listeners();
        }
    }
    pub fn subscribe_status(&self, listener: Listener) -> Result<Subscription, Disposed> {
        self.0.status_changes.subscribe(listener)
    }
    pub fn set_muted(&self, muted: bool) -> Result<(), AnimationError> {
        self.check()?;
        self.0.state.borrow_mut().muted = muted;
        self.repaint();
        Ok(())
    }
    /// Stops without changing value/status or notifying value/status listeners.
    pub fn stop(&self, canceled: bool) -> Result<(), AnimationError> {
        self.check()?;
        let motion = self.0.state.borrow_mut().motion.take();
        if let Some(motion) = motion {
            motion.run.finish(if canceled {
                AnimationOutcome::Canceled
            } else {
                AnimationOutcome::Completed
            });
        }
        Ok(())
    }
    /// Clamp a finite value to bounds, stop playback and always notify value listeners.
    pub fn set_value(&self, value: f32) -> Result<(), AnimationError> {
        self.check()?;
        if !value.is_finite() {
            return Err(AnimationError::InvalidConfiguration(
                "animation value must be finite",
            ));
        }
        let motion = {
            let mut state = self.0.state.borrow_mut();
            let motion = state.motion.take();
            state.value = value.clamp(state.config.lower_bound, state.config.upper_bound);
            state.status = if state.value == state.config.lower_bound {
                AnimationStatus::Dismissed
            } else if state.value == state.config.upper_bound {
                AnimationStatus::Completed
            } else if state.reverse {
                AnimationStatus::Reverse
            } else {
                AnimationStatus::Forward
            };
            motion
        };
        if let Some(motion) = motion {
            motion.run.finish(AnimationOutcome::Canceled);
        }
        self.dispatch(true);
        self.0.ctx.request_repaint_of(self.0.viewport);
        Ok(())
    }
    pub fn reset(&self) -> Result<(), AnimationError> {
        let lower = self.0.state.borrow().config.lower_bound;
        self.set_value(lower)
    }
    pub fn forward(&self) -> Result<AnimationRun, AnimationError> {
        let upper = self.0.state.borrow().config.upper_bound;
        self.animate(upper, None, Curve::Linear, false)
    }
    pub fn reverse(&self) -> Result<AnimationRun, AnimationError> {
        let lower = self.0.state.borrow().config.lower_bound;
        self.animate(lower, None, Curve::Linear, true)
    }
    pub fn animate_to(
        &self,
        target: f32,
        duration: Option<Duration>,
        curve: Curve,
    ) -> Result<AnimationRun, AnimationError> {
        self.animate(target, duration, curve, false)
    }
    pub fn animate_back(
        &self,
        target: f32,
        duration: Option<Duration>,
        curve: Curve,
    ) -> Result<AnimationRun, AnimationError> {
        self.animate(target, duration, curve, true)
    }
    fn animate(
        &self,
        target: f32,
        duration: Option<Duration>,
        curve: Curve,
        reverse: bool,
    ) -> Result<AnimationRun, AnimationError> {
        self.check()?;
        if !target.is_finite() {
            return Err(AnimationError::InvalidConfiguration(
                "animation target must be finite",
            ));
        }
        let now = self.now();
        let run = AnimationRun::pending();
        let (old, instant) = {
            let mut state = self.0.state.borrow_mut();
            let old = state.motion.take();
            let target = target.clamp(state.config.lower_bound, state.config.upper_bound);
            let seconds = duration.map_or_else(
                || {
                    let base = if reverse {
                        state
                            .config
                            .reverse_duration
                            .unwrap_or(state.config.duration)
                    } else {
                        state.config.duration
                    };
                    base.as_secs_f64() * (f64::from(target) - f64::from(state.value)).abs()
                        / (f64::from(state.config.upper_bound)
                            - f64::from(state.config.lower_bound))
                },
                |d| d.as_secs_f64(),
            );
            let instant = seconds == 0.0 || target == state.value;
            state.reverse = reverse;
            state.status = match (reverse, instant) {
                (false, false) => AnimationStatus::Forward,
                (true, false) => AnimationStatus::Reverse,
                (false, true) => AnimationStatus::Completed,
                (true, true) => AnimationStatus::Dismissed,
            };
            if instant {
                state.value = target;
            } else {
                state.motion = Some(Motion {
                    start: now,
                    simulation: Simulation::To {
                        from: state.value,
                        target,
                        seconds,
                        curve,
                        reverse,
                    },
                    run: run.clone(),
                });
            }
            state.last_time = now;
            (old, instant)
        };
        if let Some(old) = old {
            old.run.finish(AnimationOutcome::Canceled);
        }
        if instant {
            run.finish(AnimationOutcome::Completed);
        }
        self.dispatch(instant);
        if instant {
            self.0.ctx.request_repaint_of(self.0.viewport);
        }
        self.repaint();
        Ok(run)
    }
    pub fn repeat(&self, options: RepeatOptions) -> Result<AnimationRun, AnimationError> {
        self.check()?;
        let now = self.now();
        let run = AnimationRun::pending();
        let old = {
            let mut state = self.0.state.borrow_mut();
            let min = options.min.unwrap_or(state.config.lower_bound);
            let max = options.max.unwrap_or(state.config.upper_bound);
            let period = options.period.unwrap_or(state.config.duration);
            if !min.is_finite()
                || !max.is_finite()
                || min >= max
                || min < state.config.lower_bound
                || max > state.config.upper_bound
                || period.is_zero()
                || options.count == Some(0)
            {
                return Err(AnimationError::InvalidConfiguration(
                    "repeat needs ordered in-bound limits, nonzero period and positive count",
                ));
            }
            let phase = ((f64::from(state.value) - f64::from(min))
                / (f64::from(max) - f64::from(min)))
            .clamp(0.0, 1.0);
            let old = state.motion.take();
            state.motion = Some(Motion {
                start: now,
                simulation: Simulation::Repeat {
                    min,
                    max,
                    seconds: period.as_secs_f64(),
                    phase,
                    reverse: options.reverse,
                    count: options.count,
                },
                run: run.clone(),
            });
            state.status = AnimationStatus::Forward;
            state.reverse = false;
            state.last_time = now;
            old
        };
        if let Some(old) = old {
            old.run.finish(AnimationOutcome::Canceled);
        }
        self.dispatch(false);
        self.repaint();
        Ok(run)
    }
    fn tick(&self) {
        let now = self.now();
        let (notify, completed) = {
            let mut state = self.0.state.borrow_mut();
            if state.disposed || state.muted || !now.is_finite() || now <= state.last_time {
                drop(state);
                self.repaint();
                return;
            }
            let Some(motion) = &state.motion else {
                return;
            };
            let elapsed = (now - motion.start).max(0.0);
            let (value, status, done) = match motion.simulation {
                Simulation::To {
                    from,
                    target,
                    seconds,
                    curve,
                    reverse,
                } => {
                    let t = (elapsed / seconds).clamp(0.0, 1.0);
                    let value = lerp(from, target, curve.transform(t as f32));
                    (
                        if t >= 1.0 { target } else { value },
                        match (reverse, t >= 1.0) {
                            (false, false) => AnimationStatus::Forward,
                            (true, false) => AnimationStatus::Reverse,
                            (false, true) => AnimationStatus::Completed,
                            (true, true) => AnimationStatus::Dismissed,
                        },
                        t >= 1.0,
                    )
                }
                Simulation::Repeat {
                    min,
                    max,
                    seconds,
                    phase,
                    reverse,
                    count,
                } => {
                    let t = phase + elapsed / seconds;
                    let done = count.is_some_and(|count| t >= count as f64);
                    let backwards = reverse
                        && if done {
                            (count.unwrap() - 1) % 2 == 1
                        } else {
                            t.rem_euclid(2.0) >= 1.0
                        };
                    let part = if done { 1.0 } else { t.fract() };
                    (
                        lerp(
                            min,
                            max,
                            if backwards {
                                (1.0 - part) as f32
                            } else {
                                part as f32
                            },
                        ),
                        match (backwards, done) {
                            (false, false) => AnimationStatus::Forward,
                            (true, false) => AnimationStatus::Reverse,
                            (false, true) => AnimationStatus::Completed,
                            (true, true) => AnimationStatus::Dismissed,
                        },
                        done,
                    )
                }
            };
            let notify = state.value != value;
            state.value = value;
            state.status = status;
            state.reverse = matches!(
                status,
                AnimationStatus::Reverse | AnimationStatus::Dismissed
            );
            state.last_time = now;
            let completed = if done {
                state.motion.take().map(|m| m.run)
            } else {
                None
            };
            (notify, completed)
        };
        if let Some(run) = completed {
            run.finish(AnimationOutcome::Completed);
        }
        self.dispatch(notify);
        self.repaint();
    }
    pub fn dispose(&self) -> Result<(), AnimationError> {
        self.check()?;
        if self.0.depth.get() != 0 {
            return Err(AnimationError::DuringNotification);
        }
        let motion = {
            let mut state = self.0.state.borrow_mut();
            state.disposed = true;
            state.motion.take()
        };
        self.0.changes.dispose().expect("dispatch depth checked");
        self.0
            .status_changes
            .dispose()
            .expect("dispatch depth checked");
        if let Some(motion) = motion {
            motion.run.finish(AnimationOutcome::Canceled);
        }
        Ok(())
    }
}
fn lerp(from: f32, to: f32, t: f32) -> f32 {
    (f64::from(from) * (1.0 - f64::from(t)) + f64::from(to) * f64::from(t)) as f32
}
impl Listenable for AnimationController {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
        self.0.changes.subscribe(listener)
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl ValueListenable for AnimationController {
    type Value = f32;
    fn value(&self) -> f32 {
        self.value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dropping_last_controller_removes_its_driver_registration_without_another_frame() {
        let ctx = egui::Context::default();
        let controller = AnimationController::new(&ctx, AnimationConfig::default()).unwrap();
        let key = controller.0.driver.get().unwrap();
        let weak = controller.downgrade();
        assert!(DRIVERS.with(|drivers| drivers.borrow().contains_key(&key)));
        drop(controller);
        assert!(weak.upgrade().is_none());
        assert!(!DRIVERS.with(|drivers| drivers.borrow().contains_key(&key)));
        let replacement = AnimationController::new(&ctx, AnimationConfig::default()).unwrap();
        replacement.forward().unwrap();
        ctx.begin_pass(egui::RawInput {
            time: Some(1.0),
            ..Default::default()
        });
        let _ = ctx.end_pass();
        assert_eq!(replacement.value(), 1.0);
    }
}
