use super::{
    Animatable, Animation, AnimationError, AnimationStatus, DrivenAnimation, WeakDrivenAnimation,
};
use crate::notifier::{Disposed, Listenable, Listener, Subscription, ValueListenable};
use std::rc::Rc;
/// Immutable animation value with Forward status and no clock/notifications.
/// Status helpers reflect that status, even though this source never ticks.
#[derive(Clone)]
pub struct AlwaysStoppedAnimation<T: Clone + 'static>(Rc<T>);
impl<T: Clone + 'static> AlwaysStoppedAnimation<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(value))
    }
    pub fn value(&self) -> T {
        self.0.as_ref().clone()
    }
}
impl<T: Clone + 'static> Listenable for AlwaysStoppedAnimation<T> {
    fn subscribe(&self, _: Listener) -> Result<Subscription, Disposed> {
        Ok(Subscription::new(|| {}))
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl<T: Clone + 'static> ValueListenable for AlwaysStoppedAnimation<T> {
    type Value = T;
    fn value(&self) -> T {
        self.value()
    }
}
impl<T: Clone + 'static> Animation<T> for AlwaysStoppedAnimation<T> {
    fn status(&self) -> AnimationStatus {
        AnimationStatus::Forward
    }
    fn subscribe_status(&self, _: Listener) -> Result<Subscription, Disposed> {
        Ok(Subscription::new(|| {}))
    }
}
struct ReverseProgress;
impl Animatable for ReverseProgress {
    type Value = f32;
    fn transform(&self, t: f32) -> Result<f32, AnimationError> {
        if !t.is_finite() {
            return Err(AnimationError::InvalidConfiguration(
                "reverse animation input must be finite",
            ));
        }
        Ok((1.0 - f64::from(t)) as f32)
    }
}
fn reversed(status: AnimationStatus) -> AnimationStatus {
    match status {
        AnimationStatus::Forward => AnimationStatus::Reverse,
        AnimationStatus::Reverse => AnimationStatus::Forward,
        AnimationStatus::Completed => AnimationStatus::Dismissed,
        AnimationStatus::Dismissed => AnimationStatus::Completed,
    }
}
/// A checked 1-parent scalar view whose status reverses the parent's direction
/// and endpoints. Disposal releases this view's links without stopping parent.
pub struct ReverseAnimation<P: Animation<f32>>(DrivenAnimation<P, ReverseProgress>);
pub struct WeakReverseAnimation<P: Animation<f32>>(WeakDrivenAnimation<P, ReverseProgress>);
impl<P: Animation<f32>> Clone for ReverseAnimation<P> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<P: Animation<f32>> Clone for WeakReverseAnimation<P> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<P: Animation<f32>> WeakReverseAnimation<P> {
    pub fn upgrade(&self) -> Option<ReverseAnimation<P>> {
        self.0.upgrade().map(ReverseAnimation)
    }
}
impl<P: Animation<f32>> ReverseAnimation<P> {
    pub fn new(parent: P) -> Result<Self, AnimationError> {
        Ok(Self(DrivenAnimation::new(parent, ReverseProgress)?))
    }
    pub fn parent(&self) -> &P {
        self.0.parent()
    }
    pub fn value(&self) -> f32 {
        self.0.value()
    }
    pub fn try_value(&self) -> Result<f32, AnimationError> {
        self.0.try_value()
    }
    pub fn status(&self) -> AnimationStatus {
        reversed(self.0.status())
    }
    pub fn is_disposed(&self) -> bool {
        self.0.is_disposed()
    }
    pub fn downgrade(&self) -> WeakReverseAnimation<P> {
        WeakReverseAnimation(self.0.downgrade())
    }
    pub fn dispose(&self) -> Result<(), AnimationError> {
        self.0.dispose()
    }
    pub fn subscribe_status(&self, l: Listener) -> Result<Subscription, Disposed> {
        self.0.subscribe_status(l)
    }
}
impl<P: Animation<f32>> Listenable for ReverseAnimation<P> {
    fn subscribe(&self, l: Listener) -> Result<Subscription, Disposed> {
        self.0.subscribe(l)
    }
    fn same_source(&self, other: &Self) -> bool {
        self.0.same_source(&other.0)
    }
}
impl<P: Animation<f32>> ValueListenable for ReverseAnimation<P> {
    type Value = f32;
    fn value(&self) -> f32 {
        self.value()
    }
}
impl<P: Animation<f32>> Animation<f32> for ReverseAnimation<P> {
    fn status(&self) -> AnimationStatus {
        self.status()
    }
    fn try_value(&self) -> Result<f32, AnimationError> {
        self.try_value()
    }
    fn is_disposed(&self) -> bool {
        self.is_disposed()
    }
    fn subscribe_status(&self, l: Listener) -> Result<Subscription, Disposed> {
        self.subscribe_status(l)
    }
}
