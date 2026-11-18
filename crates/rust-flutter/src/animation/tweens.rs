use super::{
    Animation, AnimationError, AnimationStatus, ParametricCurve, TweenValue, listeners::Forwarded,
};
use crate::notifier::{Disposed, Listenable, Listener, Subscription, ValueListenable};
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::{Rc, Weak},
};

fn finite(t: f32) -> Result<(), AnimationError> {
    if t.is_finite() {
        Ok(())
    } else {
        Err(AnimationError::InvalidConfiguration(
            "tween progress must be finite",
        ))
    }
}
/// A checked mapping from scalar progress to an application value. Mappings can
/// be composed independently of an animation's clock, direction and lifetime.
pub trait Animatable: 'static {
    type Value: Clone + 'static;
    fn transform(&self, progress: f32) -> Result<Self::Value, AnimationError>;
    fn evaluate<P: Animation<f32>>(&self, parent: &P) -> Result<Self::Value, AnimationError>
    where
        Self: Sized,
    {
        self.transform(parent.try_value()?)
    }
    fn animate<P: Animation<f32>>(
        self,
        parent: P,
    ) -> Result<DrivenAnimation<P, Self>, AnimationError>
    where
        Self: Sized,
    {
        DrivenAnimation::new(parent, self)
    }
    /// Apply parent first, then this mapping: self(parent(progress)).
    fn chain<P: Animatable<Value = f32>>(self, parent: P) -> ChainedAnimatable<Self, P>
    where
        Self: Sized,
    {
        ChainedAnimatable {
            child: self,
            parent,
        }
    }
}
impl<A: Animatable + ?Sized> Animatable for Rc<A> {
    type Value = A::Value;
    fn transform(&self, t: f32) -> Result<Self::Value, AnimationError> {
        (**self).transform(t)
    }
}
struct CallbackState<T: Clone + 'static> {
    callback: Box<dyn Fn(f32) -> Result<T, AnimationError>>,
    sampling: Cell<bool>,
}
/// A UI-local checked mapping supplied as a closure.
pub struct CallbackAnimatable<T: Clone + 'static>(Rc<CallbackState<T>>);
impl<T: Clone + 'static> Clone for CallbackAnimatable<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T: Clone + 'static> CallbackAnimatable<T> {
    pub fn new(callback: impl Fn(f32) -> Result<T, AnimationError> + 'static) -> Self {
        Self(Rc::new(CallbackState {
            callback: Box::new(callback),
            sampling: Cell::new(false),
        }))
    }
}
impl<T: Clone + 'static> Animatable for CallbackAnimatable<T> {
    type Value = T;
    fn transform(&self, t: f32) -> Result<T, AnimationError> {
        finite(t)?;
        if self.0.sampling.get() {
            return Err(AnimationError::Superseded);
        }
        self.0.sampling.set(true);
        let _sampling = SamplingGuard(&self.0.sampling);
        catch_unwind(AssertUnwindSafe(|| (self.0.callback)(t)))
            .map_err(|_| AnimationError::InterpolationPanicked)?
    }
}
pub struct ChainedAnimatable<C: Animatable, P: Animatable<Value = f32>> {
    child: C,
    parent: P,
}
impl<C: Animatable, P: Animatable<Value = f32>> Animatable for ChainedAnimatable<C, P> {
    type Value = C::Value;
    fn transform(&self, t: f32) -> Result<Self::Value, AnimationError> {
        finite(t)?;
        self.child.transform(self.parent.transform(t)?)
    }
}
/// Scalar-animation extension for driving typed mappings.
pub trait AnimationDrive: Animation<f32> + Sized {
    fn drive<M: Animatable>(self, mapping: M) -> Result<DrivenAnimation<Self, M>, AnimationError> {
        DrivenAnimation::new(self, mapping)
    }
}
impl<P: Animation<f32>> AnimationDrive for P {}
struct Endpoints<T: TweenValue> {
    begin: Rc<T>,
    end: Rc<T>,
    command: Rc<()>,
    sampling: Rc<Cell<bool>>,
}
/// Shared, mutable endpoints. Existing driven animations sample changes without
/// replacing their identity; endpoint edits do not emit parent notifications.
pub struct ValueTween<T: TweenValue>(Rc<RefCell<Endpoints<T>>>);
pub struct WeakValueTween<T: TweenValue>(Weak<RefCell<Endpoints<T>>>);
impl<T: TweenValue> Clone for WeakValueTween<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T: TweenValue> WeakValueTween<T> {
    pub fn upgrade(&self) -> Option<ValueTween<T>> {
        self.0.upgrade().map(ValueTween)
    }
}
impl<T: TweenValue> Clone for ValueTween<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
fn checked_endpoint<T: TweenValue>(value: &T) -> Result<(), AnimationError> {
    catch_unwind(AssertUnwindSafe(|| value.validate()))
        .map_err(|_| AnimationError::InterpolationPanicked)?
}
impl<T: TweenValue> ValueTween<T> {
    pub fn new(begin: T, end: T) -> Result<Self, AnimationError> {
        checked_endpoint(&begin)?;
        checked_endpoint(&end)?;
        Ok(Self(Rc::new(RefCell::new(Endpoints {
            begin: Rc::new(begin),
            end: Rc::new(end),
            command: Rc::new(()),
            sampling: Rc::new(Cell::new(false)),
        }))))
    }
    pub fn downgrade(&self) -> WeakValueTween<T> {
        WeakValueTween(Rc::downgrade(&self.0))
    }
    pub fn begin(&self) -> T {
        let value = self.0.borrow().begin.clone();
        value.as_ref().clone()
    }
    pub fn end(&self) -> T {
        let value = self.0.borrow().end.clone();
        value.as_ref().clone()
    }
    pub fn set_begin(&self, value: T) -> Result<(), AnimationError> {
        self.set(value, true)
    }
    pub fn set_end(&self, value: T) -> Result<(), AnimationError> {
        self.set(value, false)
    }
    fn set(&self, value: T, begin: bool) -> Result<(), AnimationError> {
        let stamp = self.0.borrow().command.clone();
        checked_endpoint(&value)?;
        let old = {
            let mut endpoints = self.0.borrow_mut();
            if !Rc::ptr_eq(&stamp, &endpoints.command) {
                return Err(AnimationError::Superseded);
            }
            endpoints.command = Rc::new(());
            std::mem::replace(
                if begin {
                    &mut endpoints.begin
                } else {
                    &mut endpoints.end
                },
                Rc::new(value),
            )
        };
        drop(old);
        Ok(())
    }
    pub fn transform(&self, t: f32) -> Result<T, AnimationError> {
        <Self as Animatable>::transform(self, t)
    }
}
impl<T: TweenValue> Animatable for ValueTween<T> {
    type Value = T;
    fn transform(&self, t: f32) -> Result<T, AnimationError> {
        finite(t)?;
        let (begin, end, stamp, sampling) = {
            let e = self.0.borrow();
            (
                e.begin.clone(),
                e.end.clone(),
                e.command.clone(),
                e.sampling.clone(),
            )
        };
        if sampling.get() {
            return Err(AnimationError::Superseded);
        }
        sampling.set(true);
        let _sampling = SamplingGuard(&sampling);
        let sample = catch_unwind(AssertUnwindSafe(|| {
            let value = if t == 0.0 {
                begin.as_ref().clone()
            } else if t == 1.0 {
                end.as_ref().clone()
            } else if (0.0..=1.0).contains(&t) {
                begin.interpolate(&end, t)
            } else {
                begin.interpolate_unbounded(&end, t)?
            };
            value.validate()?;
            Ok(value)
        }));
        let current = self.0.borrow().command.clone();
        if !Rc::ptr_eq(&stamp, &current) {
            return Err(AnimationError::Superseded);
        }
        sample.map_err(|_| AnimationError::InterpolationPanicked)?
    }
}
pub struct ConstantTween<T: Clone + 'static>(pub T);
impl<T: Clone + 'static> Animatable for ConstantTween<T> {
    type Value = T;
    fn transform(&self, t: f32) -> Result<T, AnimationError> {
        finite(t)?;
        Ok(self.0.clone())
    }
}
struct CurveState {
    curve: Rc<dyn ParametricCurve>,
    command: Rc<()>,
    sampling: Rc<Cell<bool>>,
}
#[derive(Clone)]
pub struct CurveTween(Rc<RefCell<CurveState>>);
#[derive(Clone)]
pub struct WeakCurveTween(Weak<RefCell<CurveState>>);
impl WeakCurveTween {
    pub fn upgrade(&self) -> Option<CurveTween> {
        self.0.upgrade().map(CurveTween)
    }
}
impl CurveTween {
    pub fn new(curve: impl ParametricCurve) -> Self {
        Self(Rc::new(RefCell::new(CurveState {
            curve: Rc::new(curve),
            command: Rc::new(()),
            sampling: Rc::new(Cell::new(false)),
        })))
    }
    pub fn curve(&self) -> Rc<dyn ParametricCurve> {
        self.0.borrow().curve.clone()
    }
    pub fn downgrade(&self) -> WeakCurveTween {
        WeakCurveTween(Rc::downgrade(&self.0))
    }
    pub fn set_curve(&self, curve: impl ParametricCurve) {
        let old = {
            let mut state = self.0.borrow_mut();
            state.command = Rc::new(());
            std::mem::replace(&mut state.curve, Rc::new(curve))
        };
        drop(old);
    }
}
impl Animatable for CurveTween {
    type Value = f32;
    fn transform(&self, t: f32) -> Result<f32, AnimationError> {
        super::curves::progress(t)?;
        let (curve, stamp, sampling) = {
            let state = self.0.borrow();
            (
                state.curve.clone(),
                state.command.clone(),
                state.sampling.clone(),
            )
        };
        if sampling.get() {
            return Err(AnimationError::Superseded);
        }
        sampling.set(true);
        let _sampling = SamplingGuard(&sampling);
        let sample = catch_unwind(AssertUnwindSafe(|| curve.transform(t)));
        let current = self.0.borrow().command.clone();
        if !Rc::ptr_eq(&stamp, &current) {
            return Err(AnimationError::Superseded);
        }
        let value = sample.map_err(|_| AnimationError::CurvePanicked)??;
        if !value.is_finite() || ((t == 0.0 || t == 1.0) && value != t) {
            return Err(AnimationError::InvalidConfiguration(
                "curve tween returned invalid output or endpoints",
            ));
        }
        Ok(value)
    }
}
/// One positive-weight section. Use Rc<dyn Animatable<Value=T>> to mix mapping
/// types, including constants and curve/tween chains, within a sequence.
pub struct SequenceItem<T: Clone + 'static> {
    mapping: Rc<dyn Animatable<Value = T>>,
    weight: f64,
}
impl<T: Clone + 'static> SequenceItem<T> {
    pub fn new(mapping: impl Animatable<Value = T>, weight: f64) -> Result<Self, AnimationError> {
        if !weight.is_finite() || weight <= 0.0 {
            return Err(AnimationError::InvalidConfiguration(
                "sequence weight must be positive and finite",
            ));
        }
        Ok(Self {
            mapping: Rc::new(mapping),
            weight,
        })
    }
}
struct Segment<T: Clone + 'static> {
    mapping: Rc<dyn Animatable<Value = T>>,
    begin: f64,
    end: f64,
}
pub struct TweenSequence<T: Clone + 'static> {
    segments: Rc<Vec<Segment<T>>>,
}
impl<T: Clone + 'static> Clone for TweenSequence<T> {
    fn clone(&self) -> Self {
        Self {
            segments: self.segments.clone(),
        }
    }
}
impl<T: Clone + 'static> TweenSequence<T> {
    pub fn new(items: Vec<SequenceItem<T>>) -> Result<Self, AnimationError> {
        if items.is_empty() {
            return Err(AnimationError::InvalidConfiguration(
                "tween sequence requires an item",
            ));
        }
        let max = items.iter().map(|i| i.weight).fold(0.0_f64, f64::max);
        let total = items.iter().map(|i| i.weight / max).sum::<f64>();
        let len = items.len();
        let mut begin = 0.0;
        let mut sum = 0.0;
        let mut segments = Vec::with_capacity(len);
        for (index, item) in items.into_iter().enumerate() {
            sum += item.weight / max;
            let end = if index == len - 1 { 1.0 } else { sum / total };
            if !end.is_finite() || end <= begin {
                return Err(AnimationError::InvalidConfiguration(
                    "sequence weights cannot form distinct intervals",
                ));
            }
            segments.push(Segment {
                mapping: item.mapping,
                begin,
                end,
            });
            begin = end;
        }
        Ok(Self {
            segments: Rc::new(segments),
        })
    }
}
impl<T: Clone + 'static> Animatable for TweenSequence<T> {
    type Value = T;
    fn transform(&self, t: f32) -> Result<T, AnimationError> {
        super::curves::progress(t)?;
        let t = f64::from(t);
        let index = if t == 1.0 {
            self.segments.len() - 1
        } else {
            self.segments.partition_point(|s| s.end <= t)
        };
        let segment = &self.segments[index];
        segment
            .mapping
            .transform(((t - segment.begin) / (segment.end - segment.begin)).clamp(0.0, 1.0) as f32)
    }
}
struct Inner<P: Animation<f32>, M: Animatable> {
    parent: P,
    mapping: M,
    disposed: Cell<bool>,
    sampling: Cell<bool>,
    links: Forwarded,
}
impl<P: Animation<f32>, M: Animatable> Drop for Inner<P, M> {
    fn drop(&mut self) {
        self.links.cancel_all();
    }
}
/// An animation mapped to typed values. Clones share identity and registration
/// ownership; disposing/dropping it releases its links and preserves the parent.
pub struct DrivenAnimation<P: Animation<f32>, M: Animatable>(Rc<Inner<P, M>>);
pub struct WeakDrivenAnimation<P: Animation<f32>, M: Animatable>(Weak<Inner<P, M>>);
impl<P: Animation<f32>, M: Animatable> Clone for DrivenAnimation<P, M> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<P: Animation<f32>, M: Animatable> Clone for WeakDrivenAnimation<P, M> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<P: Animation<f32>, M: Animatable> WeakDrivenAnimation<P, M> {
    pub fn upgrade(&self) -> Option<DrivenAnimation<P, M>> {
        self.0.upgrade().map(DrivenAnimation)
    }
}
impl<P: Animation<f32>, M: Animatable> DrivenAnimation<P, M> {
    pub fn new(parent: P, mapping: M) -> Result<Self, AnimationError> {
        if parent.is_disposed() {
            return Err(AnimationError::Disposed);
        }
        Ok(Self(Rc::new(Inner {
            parent,
            mapping,
            disposed: Cell::new(false),
            sampling: Cell::new(false),
            links: Forwarded::default(),
        })))
    }
    pub fn parent(&self) -> &P {
        &self.0.parent
    }
    pub fn mapping(&self) -> &M {
        &self.0.mapping
    }
    pub fn downgrade(&self) -> WeakDrivenAnimation<P, M> {
        WeakDrivenAnimation(Rc::downgrade(&self.0))
    }
    pub fn status(&self) -> AnimationStatus {
        self.0.parent.status()
    }
    pub fn is_disposed(&self) -> bool {
        self.0.disposed.get() || self.0.parent.is_disposed()
    }
    pub fn try_value(&self) -> Result<M::Value, AnimationError> {
        if self.is_disposed() {
            return Err(AnimationError::Disposed);
        }
        if self.0.sampling.get() {
            return Err(AnimationError::Superseded);
        }
        self.0.sampling.set(true);
        let _sampling = SamplingGuard(&self.0.sampling);
        let status = self.status();
        let progress = self.0.parent.try_value()?;
        finite(progress)?;
        let sample = catch_unwind(AssertUnwindSafe(|| self.0.mapping.transform(progress)));
        if self.is_disposed() {
            return Err(AnimationError::Disposed);
        }
        let parent = self.0.parent.try_value()?;
        let status_now = self.status();
        if self.is_disposed() {
            return Err(AnimationError::Disposed);
        }
        if parent != progress || status_now != status {
            return Err(AnimationError::Superseded);
        }
        sample.map_err(|_| AnimationError::InterpolationPanicked)?
    }
    /// # Panics
    /// Panics for disposed sources, invalid mapping output, or a failed sample.
    /// Use try_value when application mappings can fail.
    pub fn value(&self) -> M::Value {
        self.try_value().expect("invalid driven animation sample")
    }
    fn listen(&self, listener: Listener, status: bool) -> Result<Subscription, Disposed> {
        if self.is_disposed() {
            return Err(Disposed);
        }
        let weak = Rc::downgrade(&self.0);
        let callback = Rc::new(move || {
            if let Some(inner) = weak.upgrade()
                && !inner.disposed.get()
                && !inner.parent.is_disposed()
            {
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
        Ok(self.0.links.add(subscription))
    }
    pub fn subscribe_status(&self, l: Listener) -> Result<Subscription, Disposed> {
        self.listen(l, true)
    }
    pub fn dispose(&self) -> Result<(), AnimationError> {
        if self.0.disposed.replace(true) {
            return Err(AnimationError::Disposed);
        }
        self.0.links.cancel_all();
        Ok(())
    }
}
impl<P: Animation<f32>, M: Animatable> Listenable for DrivenAnimation<P, M> {
    fn subscribe(&self, l: Listener) -> Result<Subscription, Disposed> {
        self.listen(l, false)
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl<P: Animation<f32>, M: Animatable> ValueListenable for DrivenAnimation<P, M> {
    type Value = M::Value;
    fn value(&self) -> Self::Value {
        self.value()
    }
}
impl<P: Animation<f32>, M: Animatable> Animation<M::Value> for DrivenAnimation<P, M> {
    fn status(&self) -> AnimationStatus {
        self.status()
    }
    fn try_value(&self) -> Result<M::Value, AnimationError> {
        self.try_value()
    }
    fn subscribe_status(&self, l: Listener) -> Result<Subscription, Disposed> {
        self.subscribe_status(l)
    }
    fn is_disposed(&self) -> bool {
        self.is_disposed()
    }
}
struct SamplingGuard<'a>(&'a Cell<bool>);
impl Drop for SamplingGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
