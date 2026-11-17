//! Frame-driven controllers, implicit builders and scalar tween sampling.
mod controller;
mod implicit;
pub use controller::{
    AnimationConfig, AnimationController, AnimationError, AnimationOutcome, AnimationRun,
    AnimationStatus, RepeatOptions, WeakAnimationController,
};
pub use implicit::{TweenAnimationBuilder, TweenAnimationState, TweenValue};
use std::time::Duration;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Curve {
    #[default]
    Linear,
    EaseInOut,
    EaseOut,
}
impl Curve {
    pub fn transform(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::EaseInOut => t * t * (3.0 - 2.0 * t),
            Self::EaseOut => 1.0 - (1.0 - t).powi(3),
        }
    }
}
pub struct Tween {
    from: f32,
    to: f32,
    start: f64,
    duration: Duration,
    curve: Curve,
}
impl Tween {
    pub fn new(from: f32, to: f32, start: f64, duration: Duration, curve: Curve) -> Self {
        Self {
            from,
            to,
            start,
            duration,
            curve,
        }
    }
    pub fn sample(&self, time: f64) -> (f32, bool) {
        let t = if self.duration.is_zero() {
            1.0
        } else {
            ((time - self.start) / self.duration.as_secs_f64()).clamp(0.0, 1.0) as f32
        };
        (
            self.from + (self.to - self.from) * self.curve.transform(t),
            t >= 1.0,
        )
    }
    pub fn value(&self, ctx: &egui::Context) -> f32 {
        let (value, finished) = self.sample(ctx.input(|i| i.time));
        if !finished {
            ctx.request_repaint();
        }
        value
    }
}
