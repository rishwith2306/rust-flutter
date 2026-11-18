use super::{AnimationController, AnimationError, AnimationStatus, ParametricCurve};
use crate::notifier::{Disposed, Listenable, Listener, Subscription, ValueListenable};
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::{Rc, Weak},
};

/// Observable animation value and direction/status. Status listeners use the
/// same owned-registration policy as Listenable; implementations may be UI-local.
pub trait Animation<T: Clone + 'static>: ValueListenable<Value = T> {
    fn status(&self) -> AnimationStatus;
    /// Fallible sampling for derived chains. Ordinary value sources can use
    /// this default; fallible sources override it to preserve their errors.
    fn try_value(&self) -> Result<T, AnimationError> {
        Ok(self.value())
    }
    fn subscribe_status(&self, listener: Listener) -> Result<Subscription, Disposed>;
    fn is_disposed(&self) -> bool {
        false
    }
    fn is_animating(&self) -> bool {
        matches!(
            self.status(),
            AnimationStatus::Forward | AnimationStatus::Reverse
        )
    }
    fn is_completed(&self) -> bool {
        self.status() == AnimationStatus::Completed
    }
    fn is_dismissed(&self) -> bool {
        self.status() == AnimationStatus::Dismissed
    }
    fn is_forward_or_completed(&self) -> bool {
        matches!(
            self.status(),
            AnimationStatus::Forward | AnimationStatus::Completed
        )
    }
}
impl Animation<f32> for AnimationController {
    fn status(&self) -> AnimationStatus {
        self.status()
    }
    fn subscribe_status(&self, l: Listener) -> Result<Subscription, Disposed> {
        self.subscribe_status(l)
    }
    fn is_disposed(&self) -> bool {
        self.is_disposed()
    }
    fn is_animating(&self) -> bool {
        self.is_animating()
    }
}
impl<T: Clone + 'static, P: Animation<T> + ?Sized> Animation<T> for Rc<P> {
    fn try_value(&self) -> Result<T, AnimationError> {
        (**self).try_value()
    }
    fn status(&self) -> AnimationStatus {
        (**self).status()
    }
    fn subscribe_status(&self, l: Listener) -> Result<Subscription, Disposed> {
        (**self).subscribe_status(l)
    }
    fn is_disposed(&self) -> bool {
        (**self).is_disposed()
    }
    fn is_animating(&self) -> bool {
        (**self).is_animating()
    }
}
struct Registration(RefCell<Option<Subscription>>);
impl Registration {
    fn cancel(&self) {
        let subscription = self.0.borrow_mut().take();
        drop(subscription);
    }
}
struct Curves {
    forward: Rc<dyn ParametricCurve>,
    reverse: Option<Rc<dyn ParametricCurve>>,
}
struct Inner<P: Animation<f32>> {
    parent: P,
    curves: RefCell<Curves>,
    direction: Cell<Option<AnimationStatus>>,
    disposed: Cell<bool>,
    sampling: Cell<bool>,
    command: RefCell<Rc<()>>,
    status_watch: RefCell<Option<Subscription>>,
    registrations: RefCell<Vec<Weak<Registration>>>,
}
impl<P: Animation<f32>> Inner<P> {
    fn track(&self) {
        let status = self.parent.status();
        if matches!(
            status,
            AnimationStatus::Completed | AnimationStatus::Dismissed
        ) {
            self.direction.set(None);
        } else if self.direction.get().is_none() {
            self.direction.set(Some(status));
        }
    }
    fn active(&self) -> bool {
        !self.disposed.get() && !self.parent.is_disposed()
    }
}
/// Curve-transformed animation with direction latching across mid-flight
/// reversals. Clones share identity/configuration; disposal never disposes parent.
pub struct CurvedAnimation<P: Animation<f32>>(Rc<Inner<P>>);
pub struct WeakCurvedAnimation<P: Animation<f32>>(Weak<Inner<P>>);
impl<P: Animation<f32>> Clone for WeakCurvedAnimation<P> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<P: Animation<f32>> WeakCurvedAnimation<P> {
    pub fn upgrade(&self) -> Option<CurvedAnimation<P>> {
        self.0.upgrade().map(CurvedAnimation)
    }
}
impl<P: Animation<f32>> Clone for CurvedAnimation<P> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<P: Animation<f32>> CurvedAnimation<P> {
    pub fn new(parent: P, curve: impl ParametricCurve) -> Result<Self, AnimationError> {
        if parent.is_disposed() {
            return Err(AnimationError::Disposed);
        }
        let inner = Rc::new(Inner {
            parent,
            curves: RefCell::new(Curves {
                forward: Rc::new(curve),
                reverse: None,
            }),
            direction: Cell::new(None),
            disposed: Cell::new(false),
            sampling: Cell::new(false),
            command: RefCell::new(Rc::new(())),
            status_watch: RefCell::new(None),
            registrations: RefCell::new(vec![]),
        });
        inner.track();
        let weak = Rc::downgrade(&inner);
        let watch = inner
            .parent
            .subscribe_status(Rc::new(move || {
                if let Some(inner) = weak.upgrade()
                    && inner.active()
                {
                    inner.track();
                }
            }))
            .map_err(|_| AnimationError::Disposed)?;
        if !inner.active() {
            drop(watch);
            return Err(AnimationError::Disposed);
        }
        *inner.status_watch.borrow_mut() = Some(watch);
        Ok(Self(inner))
    }
    pub fn downgrade(&self) -> WeakCurvedAnimation<P> {
        WeakCurvedAnimation(Rc::downgrade(&self.0))
    }
    pub fn parent(&self) -> &P {
        &self.0.parent
    }
    pub fn status(&self) -> AnimationStatus {
        self.0.parent.status()
    }
    pub fn is_disposed(&self) -> bool {
        !self.0.active()
    }
    /// Curve configuration changes do not emit synthetic parent notifications.
    pub fn set_curve(&self, curve: impl ParametricCurve) -> Result<(), AnimationError> {
        if self.is_disposed() {
            return Err(AnimationError::Disposed);
        }
        *self.0.command.borrow_mut() = Rc::new(());
        let old = std::mem::replace(&mut self.0.curves.borrow_mut().forward, Rc::new(curve));
        drop(old);
        Ok(())
    }
    pub fn set_reverse_curve(
        &self,
        curve: Option<Rc<dyn ParametricCurve>>,
    ) -> Result<(), AnimationError> {
        if self.is_disposed() {
            return Err(AnimationError::Disposed);
        }
        *self.0.command.borrow_mut() = Rc::new(());
        let old = std::mem::replace(&mut self.0.curves.borrow_mut().reverse, curve);
        drop(old);
        Ok(())
    }
    pub fn try_value(&self) -> Result<f32, AnimationError> {
        if self.is_disposed() {
            return Err(AnimationError::Disposed);
        }
        if self.0.sampling.get() {
            return Err(AnimationError::Superseded);
        }
        self.0.sampling.set(true);
        let _sampling = SamplingGuard(&self.0.sampling);
        let stamp = self.0.command.borrow().clone();
        let status = self.status();
        let direction = self.0.direction.get();
        let t = self.0.parent.try_value()?;
        super::curves::progress(t)?;
        let selected_direction = direction.unwrap_or(status);
        let curve = {
            let curves = self.0.curves.borrow();
            if selected_direction == AnimationStatus::Reverse {
                curves
                    .reverse
                    .clone()
                    .unwrap_or_else(|| curves.forward.clone())
            } else {
                curves.forward.clone()
            }
        };
        let sample = catch_unwind(AssertUnwindSafe(|| curve.transform(t)));
        if self.is_disposed() {
            return Err(AnimationError::Disposed);
        }
        // Parent sampling may itself evaluate a user curve in a derived chain.
        // Finish those calls before borrowing our command/configuration again.
        let parent_now = self.0.parent.try_value()?;
        let status_now = self.status();
        let current_stamp = self.0.command.borrow().clone();
        if self.is_disposed() {
            return Err(AnimationError::Disposed);
        }
        if !Rc::ptr_eq(&stamp, &current_stamp)
            || parent_now != t
            || status_now != status
            || self.0.direction.get() != direction
        {
            return Err(AnimationError::Superseded);
        }
        let value = sample.map_err(|_| AnimationError::CurvePanicked)??;
        if !value.is_finite() {
            return Err(AnimationError::InvalidConfiguration(
                "curve output must be finite",
            ));
        }
        if (t == 0.0 || t == 1.0) && value != t {
            return Err(AnimationError::InvalidConfiguration(
                "curve must preserve normalized endpoints",
            ));
        }
        Ok(value)
    }
    /// Current transformed value. Use try_value for fallible/custom curves.
    /// # Panics
    /// Panics if disposed, the parent leaves 0…1, or curve evaluation fails.
    pub fn value(&self) -> f32 {
        self.try_value().expect("invalid curved animation sample")
    }
    pub fn subscribe_status(&self, listener: Listener) -> Result<Subscription, Disposed> {
        self.listen(listener, true)
    }
    fn listen(&self, listener: Listener, status: bool) -> Result<Subscription, Disposed> {
        if self.is_disposed() {
            return Err(Disposed);
        }
        let weak = Rc::downgrade(&self.0);
        let callback = Rc::new(move || {
            if let Some(inner) = weak.upgrade()
                && inner.active()
            {
                inner.track();
                listener();
            }
        });
        let subscription = if status {
            self.0.parent.subscribe_status(callback)
        } else {
            self.0.parent.subscribe(callback)
        }?;
        if self.is_disposed() {
            drop(subscription);
            return Err(Disposed);
        }
        let registration = Rc::new(Registration(RefCell::new(Some(subscription))));
        {
            let mut registrations = self.0.registrations.borrow_mut();
            registrations.retain(|r| r.strong_count() > 0);
            registrations.push(Rc::downgrade(&registration));
        }
        Ok(Subscription::new(move || registration.cancel()))
    }
    pub fn dispose(&self) -> Result<(), AnimationError> {
        if self.0.disposed.replace(true) {
            return Err(AnimationError::Disposed);
        }
        *self.0.command.borrow_mut() = Rc::new(());
        let watch = self.0.status_watch.borrow_mut().take();
        let registrations = std::mem::take(&mut *self.0.registrations.borrow_mut());
        drop(watch);
        for registration in registrations.into_iter().filter_map(|r| r.upgrade()) {
            registration.cancel();
        }
        Ok(())
    }
}
impl<P: Animation<f32>> Listenable for CurvedAnimation<P> {
    fn subscribe(&self, l: Listener) -> Result<Subscription, Disposed> {
        self.listen(l, false)
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl<P: Animation<f32>> ValueListenable for CurvedAnimation<P> {
    type Value = f32;
    fn value(&self) -> f32 {
        self.value()
    }
}
impl<P: Animation<f32>> Animation<f32> for CurvedAnimation<P> {
    fn try_value(&self) -> Result<f32, AnimationError> {
        self.try_value()
    }
    fn status(&self) -> AnimationStatus {
        self.status()
    }
    fn subscribe_status(&self, l: Listener) -> Result<Subscription, Disposed> {
        self.subscribe_status(l)
    }
    fn is_disposed(&self) -> bool {
        self.is_disposed()
    }
}

impl<P: Animation<f32>> Drop for Inner<P> {
    fn drop(&mut self) {
        let watch = self.status_watch.get_mut().take();
        let registrations = std::mem::take(self.registrations.get_mut());
        drop(watch);
        for registration in registrations.into_iter().filter_map(|r| r.upgrade()) {
            registration.cancel();
        }
    }
}

struct SamplingGuard<'a>(&'a Cell<bool>);
impl Drop for SamplingGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
