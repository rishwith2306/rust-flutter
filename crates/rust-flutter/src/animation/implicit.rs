//! Implicit value animations with element-owned playback and post-pass completion.
use super::{AnimationConfig, AnimationController, AnimationError, AnimationStatus, Curve};
use crate::{
    decoration::BoxDecoration,
    element::{BuildContext, State, StateContext, StatefulWidget, Widget},
    layout::Insets,
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
    time::Duration,
};

/// Value interpolation supplied by an application or a built-in implementation.
/// The builder invokes interpolation only with finite progress strictly in 0…1;
/// endpoints are returned exactly. Implementations must preserve valid values.
pub trait TweenValue: Clone + PartialEq + 'static {
    fn interpolate(&self, target: &Self, progress: f32) -> Self;
    fn validate(&self) -> Result<(), AnimationError> {
        Ok(())
    }
    /// Optional extrapolation for explicit value tweens. Existing custom
    /// interpolators retain their normalized-only contract by default.
    fn interpolate_unbounded(&self, target: &Self, progress: f32) -> Result<Self, AnimationError> {
        if !progress.is_finite() || !(0.0..=1.0).contains(&progress) {
            return Err(AnimationError::InvalidConfiguration(
                "custom tween does not support this extrapolation",
            ));
        }
        let value = self.interpolate(target, progress);
        value.validate()?;
        Ok(value)
    }
}
macro_rules! unbounded_interpolation {
    () => {
        fn interpolate_unbounded(
            &self,
            target: &Self,
            progress: f32,
        ) -> Result<Self, AnimationError> {
            if !progress.is_finite() {
                return Err(AnimationError::InvalidConfiguration(
                    "tween progress must be finite",
                ));
            }
            let value = self.interpolate(target, progress);
            value.validate()?;
            Ok(value)
        }
    };
}
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    (f64::from(a) * (1.0 - f64::from(t)) + f64::from(b) * f64::from(t)) as f32
}
fn finite(valid: bool) -> Result<(), AnimationError> {
    if valid {
        Ok(())
    } else {
        Err(AnimationError::InvalidConfiguration(
            "implicit animation endpoints contain invalid geometry",
        ))
    }
}
impl TweenValue for f64 {
    unbounded_interpolation!();
    fn interpolate(&self, b: &Self, t: f32) -> Self {
        self * (1.0 - f64::from(t)) + b * f64::from(t)
    }
    fn validate(&self) -> Result<(), AnimationError> {
        finite(self.is_finite())
    }
}
impl TweenValue for f32 {
    unbounded_interpolation!();
    fn interpolate(&self, b: &Self, t: f32) -> Self {
        lerp(*self, *b, t)
    }
    fn validate(&self) -> Result<(), AnimationError> {
        finite(self.is_finite())
    }
}
impl TweenValue for egui::Vec2 {
    unbounded_interpolation!();
    fn interpolate(&self, b: &Self, t: f32) -> Self {
        Self::new(lerp(self.x, b.x, t), lerp(self.y, b.y, t))
    }
    fn validate(&self) -> Result<(), AnimationError> {
        finite(self.is_finite())
    }
}
impl TweenValue for egui::Pos2 {
    unbounded_interpolation!();
    fn interpolate(&self, b: &Self, t: f32) -> Self {
        Self::new(lerp(self.x, b.x, t), lerp(self.y, b.y, t))
    }
    fn validate(&self) -> Result<(), AnimationError> {
        finite(self.is_finite())
    }
}
impl TweenValue for egui::Rect {
    unbounded_interpolation!();
    fn interpolate(&self, b: &Self, t: f32) -> Self {
        Self::from_min_max(
            self.min.interpolate(&b.min, t),
            self.max.interpolate(&b.max, t),
        )
    }
    fn validate(&self) -> Result<(), AnimationError> {
        finite(self.is_finite() && self.min.x <= self.max.x && self.min.y <= self.max.y)
    }
}
impl TweenValue for egui::Color32 {
    unbounded_interpolation!();
    fn interpolate(&self, b: &Self, t: f32) -> Self {
        let a = self.to_array();
        let b = b.to_array();
        let channel = |i| lerp(f32::from(a[i]), f32::from(b[i]), t).round() as u8;
        Self::from_rgba_premultiplied(channel(0), channel(1), channel(2), channel(3))
    }
}
impl TweenValue for Insets {
    unbounded_interpolation!();
    fn interpolate(&self, b: &Self, t: f32) -> Self {
        Self {
            left: lerp(self.left, b.left, t),
            top: lerp(self.top, b.top, t),
            right: lerp(self.right, b.right, t),
            bottom: lerp(self.bottom, b.bottom, t),
        }
    }
    fn validate(&self) -> Result<(), AnimationError> {
        finite(
            [self.left, self.top, self.right, self.bottom]
                .iter()
                .all(|v| v.is_finite() && *v >= 0.0),
        )
    }
}
impl TweenValue for egui::Stroke {
    unbounded_interpolation!();
    fn interpolate(&self, b: &Self, t: f32) -> Self {
        Self::new(
            lerp(self.width, b.width, t),
            self.color.interpolate(&b.color, t),
        )
    }
    fn validate(&self) -> Result<(), AnimationError> {
        finite(self.width.is_finite() && self.width >= 0.0)
    }
}
impl TweenValue for BoxDecoration {
    unbounded_interpolation!();
    fn interpolate(&self, b: &Self, t: f32) -> Self {
        let radius = |a: u8, b: u8| lerp(f32::from(a), f32::from(b), t).round() as u8;
        Self {
            color: self.color.interpolate(&b.color, t),
            border: self.border.interpolate(&b.border, t),
            corner_radius: egui::CornerRadius {
                nw: radius(self.corner_radius.nw, b.corner_radius.nw),
                ne: radius(self.corner_radius.ne, b.corner_radius.ne),
                sw: radius(self.corner_radius.sw, b.corner_radius.sw),
                se: radius(self.corner_radius.se, b.corner_radius.se),
            },
        }
    }
    fn validate(&self) -> Result<(), AnimationError> {
        self.border.validate()
    }
}
type Builder<T> = Rc<dyn Fn(&BuildContext, T) -> Widget>;
type EndCallback = Rc<dyn Fn()>;
/// Immutable implicit-animation configuration. Begin is used only on first
/// mount. A changed target retargets continuously from the currently sampled value.
pub struct TweenAnimationBuilder<T: TweenValue> {
    begin: T,
    target: T,
    duration: Duration,
    curve: Curve,
    builder: Builder<T>,
    on_end: Option<EndCallback>,
}
impl<T: TweenValue> TweenAnimationBuilder<T> {
    pub fn new(
        begin: T,
        target: T,
        duration: Duration,
        builder: impl Fn(&BuildContext, T) -> Widget + 'static,
    ) -> Result<Self, AnimationError> {
        begin.validate()?;
        target.validate()?;
        Ok(Self {
            begin,
            target,
            duration,
            curve: Curve::Linear,
            builder: Rc::new(builder),
            on_end: None,
        })
    }
    pub fn with_curve(mut self, curve: Curve) -> Self {
        self.curve = curve;
        self
    }
    pub fn on_end(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_end = Some(Rc::new(callback));
        self
    }
    pub fn into_widget(self) -> Widget {
        Widget::stateful(self)
    }
}
/// Public only as the associated State type. Configuration and controller remain
/// private so implicit ownership cannot be bypassed through a state reference.
pub struct TweenAnimationState<T: TweenValue> {
    from: T,
    target: T,
    curve: Curve,
    duration: Duration,
    controller: Option<AnimationController>,
    ticks: Option<Subscription>,
    status: Option<Subscription>,
    completion: Option<Rc<CompletionEvents>>,
}
impl<T: TweenValue> TweenAnimationState<T> {
    fn value(&self) -> T {
        let progress = self
            .curve
            .transform(self.controller.as_ref().map_or(0.0, |c| c.value()));
        if progress <= 0.0 {
            self.from.clone()
        } else if progress >= 1.0 {
            self.target.clone()
        } else {
            self.from.interpolate(&self.target, progress)
        }
    }
    fn start(&self) {
        let controller = self
            .controller
            .as_ref()
            .expect("initialized implicit controller");
        controller
            .set_value(0.0)
            .expect("mounted implicit controller");
        if self.from != self.target {
            controller
                .animate_to(1.0, Some(self.duration), Curve::Linear)
                .expect("valid implicit animation");
        }
    }
}
impl<T: TweenValue> StatefulWidget for TweenAnimationBuilder<T> {
    type State = TweenAnimationState<T>;
    fn create_state(&self) -> Self::State {
        TweenAnimationState {
            from: self.begin.clone(),
            target: self.target.clone(),
            curve: self.curve,
            duration: self.duration,
            controller: None,
            ticks: None,
            status: None,
            completion: None,
        }
    }
}
impl<T: TweenValue> State<TweenAnimationBuilder<T>> for TweenAnimationState<T> {
    fn init_state(&mut self, w: &TweenAnimationBuilder<T>, ctx: &StateContext<Self>) {
        let controller = ctx
            .animation_controller(AnimationConfig::default())
            .expect("mounted implicit owner");
        let handle = ctx.handle();
        self.ticks = Some(
            controller
                .subscribe(Rc::new(move || {
                    let _ = handle.request_rebuild();
                }))
                .expect("live implicit source"),
        );
        let completion = CompletionEvents::new(ctx.egui(), w.on_end.clone());
        let weak = Rc::downgrade(&completion);
        let weak_controller = controller.downgrade();
        let egui = ctx.egui().clone();
        let viewport = egui.viewport_id();
        self.status = Some(
            controller
                .subscribe_status(Rc::new(move || {
                    if weak_controller
                        .upgrade()
                        .is_some_and(|c| c.status() == AnimationStatus::Completed)
                        && let Some(completion) = weak.upgrade()
                    {
                        completion.queue();
                        egui.request_repaint_of(viewport);
                    }
                }))
                .expect("live implicit status"),
        );
        self.completion = Some(completion);
        self.controller = Some(controller);
        self.start();
    }
    fn did_update_widget(
        &mut self,
        _: &TweenAnimationBuilder<T>,
        w: &TweenAnimationBuilder<T>,
        _: &StateContext<Self>,
    ) {
        *self.completion.as_ref().unwrap().callback.borrow_mut() = w.on_end.clone();
        let controller = self.controller.as_ref().unwrap();
        if self.target != w.target
            || (controller.is_animating() && (self.duration != w.duration || self.curve != w.curve))
        {
            let current = self.value();
            self.from = current;
            self.target = w.target.clone();
            self.curve = w.curve;
            self.duration = w.duration;
            self.start();
        } else if !controller.is_animating() {
            self.curve = w.curve;
            self.duration = w.duration;
        }
    }
    fn build(&mut self, w: &TweenAnimationBuilder<T>, ctx: &StateContext<Self>) -> Widget {
        (w.builder)(ctx, self.value())
    }
    fn dispose(&mut self) {
        if let Some(completion) = &self.completion {
            completion.active.set(false);
            completion.pending.borrow_mut().clear();
        }
    }
}
#[derive(Clone, Copy)]
struct CompletionDriver(u64);
static NEXT_COMPLETION_DRIVER: AtomicU64 = AtomicU64::new(1);
thread_local! {static COMPLETIONS:RefCell<HashMap<u64,Vec<Weak<CompletionEvents>>>>=RefCell::new(HashMap::new());}
struct CompletionEvents {
    key: u64,
    viewport: egui::ViewportId,
    active: Cell<bool>,
    callback: RefCell<Option<EndCallback>>,
    pending: RefCell<Vec<EndCallback>>,
}
impl CompletionEvents {
    fn new(ctx: &egui::Context, callback: Option<EndCallback>) -> Rc<Self> {
        let (key, new) = ctx.data_mut(|data| {
            let id = egui::Id::new("rust-flutter-implicit-completions");
            if let Some(driver) = data.get_temp::<CompletionDriver>(id) {
                (driver.0, false)
            } else {
                let key = NEXT_COMPLETION_DRIVER.fetch_add(1, Ordering::Relaxed);
                data.insert_temp(id, CompletionDriver(key));
                (key, true)
            }
        });
        let events = Rc::new(Self {
            key,
            viewport: ctx.viewport_id(),
            active: Cell::new(true),
            callback: RefCell::new(callback),
            pending: RefCell::new(vec![]),
        });
        COMPLETIONS.with(|all| {
            all.borrow_mut()
                .entry(key)
                .or_default()
                .push(Rc::downgrade(&events))
        });
        if new {
            ctx.on_end_pass(
                "rust-flutter implicit completion",
                Arc::new(move |ctx| {
                    let events = COMPLETIONS.with(|all| {
                        let mut all = all.borrow_mut();
                        let mut events = vec![];
                        if let Some(entries) = all.get_mut(&key) {
                            entries.retain(|weak| {
                                if let Some(event) = weak.upgrade()
                                    && event.active.get()
                                {
                                    events.push(event);
                                    true
                                } else {
                                    false
                                }
                            });
                            if entries.is_empty() {
                                all.remove(&key);
                            }
                        }
                        events
                    });
                    let deliveries = events
                        .into_iter()
                        .filter(|event| event.viewport == ctx.viewport_id() && event.active.get())
                        .map(|event| {
                            let pending = std::mem::take(&mut *event.pending.borrow_mut());
                            (event, pending)
                        })
                        .collect::<Vec<_>>();
                    for (event, pending) in deliveries {
                        for callback in pending {
                            if !event.active.get() {
                                break;
                            }
                            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                callback()
                            }));
                        }
                    }
                }),
            );
        }
        events
    }
    fn queue(&self) {
        if self.active.get()
            && let Some(callback) = self.callback.borrow().clone()
        {
            self.pending.borrow_mut().push(callback);
        }
    }
}
impl Drop for CompletionEvents {
    fn drop(&mut self) {
        let _ = COMPLETIONS.try_with(|all| {
            if let Ok(mut all) = all.try_borrow_mut()
                && let Some(entries) = all.get_mut(&self.key)
            {
                entries.retain(|weak| weak.strong_count() != 0);
                if entries.is_empty() {
                    all.remove(&self.key);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn completion_registration_releases_callbacks_without_another_frame_and_can_be_reused() {
        let ctx = egui::Context::default();
        let calls = Rc::new(Cell::new(0));
        let log = calls.clone();
        let events = CompletionEvents::new(&ctx, Some(Rc::new(move || log.set(log.get() + 1))));
        let key = events.key;
        events.queue();
        drop(events);
        assert_eq!(Rc::strong_count(&calls), 1);
        assert!(!COMPLETIONS.with(|all| all.borrow().contains_key(&key)));
        let log = calls.clone();
        let replacement =
            CompletionEvents::new(&ctx, Some(Rc::new(move || log.set(log.get() + 1))));
        replacement.queue();
        ctx.begin_pass(egui::RawInput::default());
        let _ = ctx.end_pass();
        assert_eq!(calls.get(), 1);
    }
    #[test]
    fn cross_widget_completions_queued_during_delivery_wait_until_the_next_pass() {
        let ctx = egui::Context::default();
        let recipient = Rc::new(RefCell::new(None::<Weak<CompletionEvents>>));
        let route = recipient.clone();
        let first = CompletionEvents::new(
            &ctx,
            Some(Rc::new(move || {
                route.borrow().as_ref().unwrap().upgrade().unwrap().queue()
            })),
        );
        let calls = Rc::new(Cell::new(0));
        let log = calls.clone();
        let second = CompletionEvents::new(&ctx, Some(Rc::new(move || log.set(log.get() + 1))));
        *recipient.borrow_mut() = Some(Rc::downgrade(&second));
        first.queue();
        ctx.begin_pass(egui::RawInput::default());
        let _ = ctx.end_pass();
        assert_eq!(calls.get(), 0);
        ctx.begin_pass(egui::RawInput::default());
        let _ = ctx.end_pass();
        assert_eq!(calls.get(), 1);
    }
}
