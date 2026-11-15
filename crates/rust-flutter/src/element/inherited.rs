//! Inherited values and dependency bookkeeping. Values and contexts are UI-local;
//! update handles send commands through the existing UI-thread queue.
use super::{
    ElementError, ElementId,
    state::{BuildContext, Queue, StateHandle, Unmounted},
};
use crate::notifier::{Listenable, Listener, Subscription};
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::{HashMap, HashSet},
    rc::{Rc, Weak},
    sync::{Arc, Weak as SyncWeak},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyError {
    Unmounted,
    InvalidPhase,
}
impl std::fmt::Display for DependencyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Unmounted => "component is unmounted",
            Self::InvalidPhase => {
                "depend_on is unavailable during init_state; use read for a non-listening lookup"
            }
        })
    }
}
impl std::error::Error for DependencyError {}

pub(super) type Scope = HashMap<TypeId, Rc<Link>>;
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Phase {
    Idle,
    Init,
    Build,
    Dependencies,
}
struct Read {
    chain: Vec<ElementId>,
    links: Vec<Rc<Link>>,
}
type Reads = HashMap<TypeId, Read>;

/// A typed inherited model with selectively observed aspects. Predicates receive
/// the preceding value, and the model's new value is `self`.
pub trait InheritedModel: Sized + 'static {
    /// Identifies a part of this model; each dependent accumulates a set.
    type Aspect: Clone + Eq + std::hash::Hash + 'static;
    /// Global notification gate. A false result also skips per-dependent checks.
    fn update_should_notify(&self, old: &Self) -> bool;
    /// Whether an eligible transition affects this dependent's aspect set.
    /// Unconditional dependents bypass this predicate.
    fn update_should_notify_dependent(&self, old: &Self, aspects: &HashSet<Self::Aspect>) -> bool;
    /// False makes aspect lookup continue to the next same-type ancestor,
    /// registering a dependency on this model along the way.
    fn is_supported_aspect(&self, _aspect: &Self::Aspect) -> bool {
        true
    }
}
fn chain(link: Option<Rc<Link>>) -> Vec<Rc<Link>> {
    let mut result = Vec::new();
    let mut next = link;
    while let Some(link) = next {
        next = link.parent.borrow().as_ref().and_then(Weak::upgrade);
        result.push(link);
    }
    result
}
pub(super) struct Dependencies {
    id: ElementId,
    queue: SyncWeak<Queue>,
    scope: RefCell<Scope>,
    build_reads: RefCell<Reads>,
    hook_reads: RefCell<Reads>,
    phase: Cell<Phase>,
    pub changed: Cell<bool>,
}
impl Dependencies {
    pub fn new(id: ElementId, queue: &Arc<Queue>, scope: Scope) -> Rc<Self> {
        Rc::new(Self {
            id,
            queue: Arc::downgrade(queue),
            scope: RefCell::new(scope),
            build_reads: RefCell::new(HashMap::new()),
            hook_reads: RefCell::new(HashMap::new()),
            phase: Cell::new(Phase::Idle),
            changed: Cell::new(true),
        })
    }
    fn mark_changed(&self) {
        if let Some(queue) = self.queue.upgrade()
            && queue.mounted()
        {
            self.changed.set(true);
            let _ = queue.request_rebuild();
        }
    }
    fn clear_reads(&self, phase: Phase) {
        let (clear, other) = if phase == Phase::Build {
            (&self.build_reads, &self.hook_reads)
        } else {
            (&self.hook_reads, &self.build_reads)
        };
        let old = std::mem::take(&mut *clear.borrow_mut());
        for link in old.into_values().flat_map(|read| read.links) {
            if !other
                .borrow()
                .values()
                .flat_map(|read| &read.links)
                .any(|other| other.id == link.id)
            {
                link.subscribers.borrow_mut().remove(&self.id);
            }
        }
    }
    pub fn enter(self: &Rc<Self>, phase: Phase) -> PhaseGuard {
        let previous = self.phase.replace(phase);
        PhaseGuard {
            dependencies: self.clone(),
            previous,
        }
    }
    pub fn set_scope(&self, scope: &Scope) {
        let differs = self
            .build_reads
            .borrow()
            .iter()
            .chain(self.hook_reads.borrow().iter())
            .any(|(kind, old)| {
                old.chain
                    != chain(scope.get(kind).cloned())
                        .iter()
                        .map(|link| link.id)
                        .collect::<Vec<_>>()
            });
        if differs {
            // Retain dependencies for the mounted lifetime. If an ancestor
            // binding changes, register against its new scope on the next build.
            self.clear_reads(Phase::Build);
            self.clear_reads(Phase::Dependencies);
        }
        *self.scope.borrow_mut() = scope.clone();
        if differs {
            self.mark_changed();
        }
    }
    fn lookup(&self, kind: TypeId) -> Option<Rc<Link>> {
        self.scope
            .borrow()
            .get(&kind)
            .filter(|link| link.active.get())
            .cloned()
    }
    fn subscribe<A: Clone + Eq + std::hash::Hash + 'static>(
        self: &Rc<Self>,
        kind: TypeId,
        links: Vec<Rc<Link>>,
        aspect: Option<A>,
    ) -> Result<(), DependencyError> {
        let reads = match self.phase.get() {
            Phase::Build | Phase::Idle => &self.build_reads,
            Phase::Dependencies => &self.hook_reads,
            Phase::Init => return Err(DependencyError::InvalidPhase),
        };
        let signature = chain(self.lookup(kind))
            .iter()
            .map(|link| link.id)
            .collect();
        let mut reads = reads.borrow_mut();
        let read = reads.entry(kind).or_insert_with(|| Read {
            chain: signature,
            links: vec![],
        });
        for link in links {
            if !read.links.iter().any(|old| old.id == link.id) {
                read.links.push(link.clone());
            }
            let mut subscribers = link.subscribers.borrow_mut();
            let subscriber = subscribers
                .entry(self.id)
                .or_insert_with(|| DependentSubscription {
                    dependent: Rc::downgrade(self),
                    aspects: aspect
                        .as_ref()
                        .map(|_| Rc::new(HashSet::<A>::new()) as Rc<dyn Any>),
                });
            if let Some(aspect) = &aspect {
                if let Some(aspects) = &subscriber.aspects {
                    let mut aspects = aspects
                        .downcast_ref::<HashSet<A>>()
                        .expect("model aspect type")
                        .clone();
                    aspects.insert(aspect.clone());
                    subscriber.aspects = Some(Rc::new(aspects));
                }
            } else {
                subscriber.aspects = None;
            }
        }
        Ok(())
    }
    pub fn close(&self) {
        self.clear_reads(Phase::Build);
        self.clear_reads(Phase::Dependencies);
        self.scope.borrow_mut().clear();
        self.phase.set(Phase::Idle);
    }
}
pub(super) struct PhaseGuard {
    dependencies: Rc<Dependencies>,
    previous: Phase,
}
impl Drop for PhaseGuard {
    fn drop(&mut self) {
        self.dependencies.phase.set(self.previous);
    }
}
struct DependentSubscription {
    dependent: Weak<Dependencies>,
    // None is an unconditional dependency; a set accumulates aspects.
    aspects: Option<Rc<dyn Any>>,
}
pub(super) struct Link {
    pub id: ElementId,
    pub kind: TypeId,
    queue: SyncWeak<Queue>,
    pub parent: RefCell<Option<Weak<Link>>>,
    active: Cell<bool>,
    value: RefCell<Option<Rc<dyn Any>>>,
    subscribers: RefCell<HashMap<ElementId, DependentSubscription>>,
}
impl Link {
    fn new<T: 'static>(id: ElementId, queue: &Arc<Queue>, value: Rc<T>) -> Rc<Self> {
        Rc::new(Self {
            id,
            kind: TypeId::of::<T>(),
            queue: Arc::downgrade(queue),
            parent: RefCell::new(None),
            active: Cell::new(true),
            value: RefCell::new(Some(value)),
            subscribers: RefCell::new(HashMap::new()),
        })
    }
    fn notify(&self) {
        self.notify_where(|_| true);
    }
    fn notify_where(&self, predicate: impl Fn(&dyn Any) -> bool) {
        let subscribers: Vec<_> = self
            .subscribers
            .borrow()
            .values()
            .filter_map(|s| {
                s.dependent
                    .upgrade()
                    .map(|dependent| (dependent, s.aspects.clone()))
            })
            .collect();
        self.subscribers
            .borrow_mut()
            .retain(|_, s| s.dependent.strong_count() > 0);
        for (subscriber, aspects) in subscribers {
            if aspects
                .as_ref()
                .is_none_or(|aspects| predicate(aspects.as_ref()))
            {
                subscriber.mark_changed();
            }
        }
    }
    fn value<T: 'static>(&self) -> Option<Rc<T>> {
        self.value.borrow().as_ref()?.clone().downcast().ok()
    }
    fn close(&self) {
        self.active.set(false);
        let old = self.value.borrow_mut().take();
        self.notify();
        self.subscribers.borrow_mut().clear();
        drop(old);
    }
}
impl BuildContext {
    fn dependency_link<T: 'static>(&self) -> Result<Option<Rc<Link>>, DependencyError> {
        if !self.mounted() {
            return Err(DependencyError::Unmounted);
        }
        Ok(self.dependencies.lookup(TypeId::of::<T>()))
    }
    /// Nearest inherited value, without creating a rebuild subscription.
    pub fn read<T: 'static>(&self) -> Result<Option<Rc<T>>, DependencyError> {
        Ok(self.dependency_link::<T>()?.and_then(|link| link.value()))
    }
    /// Subscribe while mounted, except during init_state, and return a value snapshot.
    pub fn depend_on<T: 'static>(&self) -> Result<Option<Rc<T>>, DependencyError> {
        let link = self.dependency_link::<T>()?;
        self.dependencies.subscribe::<()>(
            TypeId::of::<T>(),
            link.clone().into_iter().collect(),
            None,
        )?;
        Ok(link.and_then(|link| link.value()))
    }
    /// Observe an aspect through every same-type model up to and including the
    /// first supporting model. Returns the outermost visited value if none support it.
    pub fn depend_on_aspect<T: InheritedModel>(
        &self,
        aspect: T::Aspect,
    ) -> Result<Option<Rc<T>>, DependencyError> {
        let nearest = self.dependency_link::<T>()?;
        if self.dependencies.phase.get() == Phase::Init {
            return Err(DependencyError::InvalidPhase);
        }
        let mut links = Vec::new();
        let mut value = None;
        for link in chain(nearest) {
            let Some(model) = link.value::<T>() else {
                continue;
            };
            let supported = model.is_supported_aspect(&aspect);
            value = Some(model);
            links.push(link);
            if supported {
                break;
            }
        }
        self.dependencies
            .subscribe(TypeId::of::<T>(), links, Some(aspect))?;
        Ok(value)
    }
    pub fn inherited_handle<T: 'static>(
        &self,
    ) -> Result<Option<InheritedHandle<T>>, DependencyError> {
        Ok(self
            .dependency_link::<T>()?
            .and_then(|link| link.queue.upgrade())
            .map(InheritedHandle::new))
    }
}

type Notify<T> = Rc<dyn Fn(&T, &T) -> bool>;
type AspectNotify<T> = Rc<dyn Fn(&T, &T, &dyn Any) -> bool>;
/// The optional observable source exposed by an inherited notifier widget.
/// Reading this wrapper does not subscribe to its source; depend_on does.
#[derive(Clone)]
pub struct InheritedNotifier<N: Listenable> {
    pub notifier: Option<N>,
}
type Subscribe<T> =
    Rc<dyn Fn(&T, Listener) -> Result<Option<Subscription>, crate::notifier::Disposed>>;
pub(super) struct Watch<T> {
    pub same: Notify<T>,
    pub subscribe: Subscribe<T>,
}
struct ValueState<T: 'static> {
    value: Rc<T>,
    notify: Notify<T>,
    aspect_notify: Option<AspectNotify<T>>,
    pending: Vec<(Rc<T>, Rc<T>)>,
    force_notify: bool,
    watch: Option<Rc<Watch<T>>>,
    subscription: Option<Subscription>,
    notified: Rc<Cell<bool>>,
    binding_error: bool,
    watch_changed: bool,
    link: Rc<Link>,
}
impl<T: 'static> ValueState<T> {
    fn replace(&mut self, value: T) {
        let next = Rc::new(value);
        let watch_changed = std::mem::take(&mut self.watch_changed);
        let rebind = watch_changed
            || self
                .watch
                .as_ref()
                .is_some_and(|watch| !(watch.same)(&self.value, &next));
        if (self.notify)(&self.value, &next) {
            self.pending.push((self.value.clone(), next.clone()));
        }
        self.value = next.clone();
        let previous = self.link.value.replace(Some(next));
        drop(previous);
        if rebind {
            self.bind();
        }
    }
    fn set_watch(&mut self, watch: Option<Rc<Watch<T>>>) {
        let changed = self.watch.is_some() != watch.is_some();
        self.watch = watch;
        self.watch_changed |= changed;
    }
    fn bind(&mut self) {
        self.subscription.take();
        if let Some(watch) = &self.watch {
            let queue = self.link.queue.clone();
            let notified = self.notified.clone();
            let listener: Listener = Rc::new(move || {
                if let Some(queue) = queue.upgrade()
                    && queue.mounted()
                {
                    notified.set(true);
                    let _ = queue.request_rebuild();
                }
            });
            match (watch.subscribe)(&self.value, listener) {
                Ok(subscription) => self.subscription = subscription,
                Err(_) => self.binding_error = true,
            }
        }
    }
}
/// Commands replace immutable snapshots on the UI thread. Like StateHandle, the
/// handle is sendable even when the inherited value type itself is not Send.
pub struct InheritedHandle<T: 'static> {
    inner: StateHandle<ValueState<T>>,
}
impl<T: 'static> Clone for InheritedHandle<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
impl<T: 'static> InheritedHandle<T> {
    pub(super) fn new(queue: Arc<Queue>) -> Self {
        Self {
            inner: StateHandle::new(queue),
        }
    }
    pub fn mounted(&self) -> bool {
        self.inner.mounted()
    }
    pub fn update(&self, update: impl FnOnce(&T) -> T + Send + 'static) -> Result<(), Unmounted> {
        self.inner.update(move |state| {
            let next = update(&state.value);
            state.replace(next);
        })
    }
    pub fn set(&self, value: T) -> Result<(), Unmounted>
    where
        T: Send,
    {
        self.update(move |_| value)
    }
    pub fn notify(&self) -> Result<(), Unmounted> {
        self.inner.update(|state| state.force_notify = true)
    }
}

pub(super) trait ProviderDescription {
    fn kind(&self) -> TypeId;
    fn name(&self) -> &'static str;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn mount(self: Box<Self>, id: ElementId, queue: Arc<Queue>) -> Box<dyn Provider>;
}
pub(super) trait Provider {
    fn link(&self) -> Rc<Link>;
    fn update(&mut self, description: Box<dyn Any>);
    fn apply_pending(&mut self) -> Result<(), ElementError>;
}
pub(super) struct Provided<T: 'static> {
    pub value: T,
    pub notify: Notify<T>,
    pub aspect_notify: Option<AspectNotify<T>>,
    pub watch: Option<Rc<Watch<T>>>,
}
struct MountedProvider<T: 'static> {
    state: ValueState<T>,
    queue: Arc<Queue>,
}
impl<T: 'static> ProviderDescription for Provided<T> {
    fn kind(&self) -> TypeId {
        TypeId::of::<T>()
    }
    fn name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn mount(self: Box<Self>, id: ElementId, queue: Arc<Queue>) -> Box<dyn Provider> {
        let value = Rc::new(self.value);
        let link = Link::new(id, &queue, value.clone());
        let mut state = ValueState {
            value,
            notify: self.notify,
            aspect_notify: self.aspect_notify,
            pending: vec![],
            force_notify: false,
            watch: self.watch,
            subscription: None,
            notified: Rc::new(Cell::new(false)),
            binding_error: false,
            watch_changed: false,
            link,
        };
        state.bind();
        Box::new(MountedProvider { state, queue })
    }
}
impl<T: 'static> Provider for MountedProvider<T> {
    fn link(&self) -> Rc<Link> {
        self.state.link.clone()
    }
    fn update(&mut self, description: Box<dyn Any>) {
        self.queue.drain(&mut self.state);
        let next = *description
            .downcast::<Provided<T>>()
            .expect("matching inherited type");
        self.state.notify = next.notify;
        self.state.aspect_notify = next.aspect_notify;
        self.state.set_watch(next.watch);
        self.state.replace(next.value);
        // The immediately following apply_pending/rebuild delivers notifications.
    }
    fn apply_pending(&mut self) -> Result<(), ElementError> {
        self.queue.drain(&mut self.state);
        if self.state.binding_error {
            return Err(ElementError::DisposedNotifier);
        }
        self.deliver();
        Ok(())
    }
}
impl<T: 'static> MountedProvider<T> {
    fn deliver(&mut self) {
        let pending = std::mem::take(&mut self.state.pending);
        let forced = std::mem::take(&mut self.state.force_notify);
        let notified = self.state.notified.replace(false);
        if forced || notified {
            self.state.link.notify();
        } else {
            for (old, new) in pending {
                self.state.link.notify_where(|aspects| {
                    self.state
                        .aspect_notify
                        .as_ref()
                        .is_none_or(|notify| notify(&old, &new, aspects))
                });
            }
        }
    }
}
impl<T: 'static> Drop for MountedProvider<T> {
    fn drop(&mut self) {
        self.queue.close();
        self.state.subscription.take();
        self.state.link.close();
    }
}
