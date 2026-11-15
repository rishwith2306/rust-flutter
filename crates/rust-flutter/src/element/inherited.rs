//! Inherited values and dependency bookkeeping. Values and contexts are UI-local;
//! update handles send commands through the existing UI-thread queue.
use super::{
    ElementId,
    state::{BuildContext, Queue, StateHandle, Unmounted},
};
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell},
    collections::HashMap,
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
type Reads = HashMap<TypeId, Option<Rc<Link>>>;
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
        for link in old.into_values().flatten() {
            if !other
                .borrow()
                .values()
                .flatten()
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
                old.as_ref().map(|link| link.id)
                    != scope
                        .get(kind)
                        .filter(|link| link.active.get())
                        .map(|link| link.id)
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
    fn subscribe(
        self: &Rc<Self>,
        kind: TypeId,
        link: Option<Rc<Link>>,
    ) -> Result<(), DependencyError> {
        let (reads, other) = match self.phase.get() {
            Phase::Build | Phase::Idle => (&self.build_reads, &self.hook_reads),
            Phase::Dependencies => (&self.hook_reads, &self.build_reads),
            Phase::Init => return Err(DependencyError::InvalidPhase),
        };
        let old = reads.borrow_mut().insert(kind, link.clone()).flatten();
        if let Some(old) = old
            && link.as_ref().is_none_or(|link| link.id != old.id)
            && !other
                .borrow()
                .values()
                .flatten()
                .any(|link| link.id == old.id)
        {
            old.subscribers.borrow_mut().remove(&self.id);
        }
        if let Some(link) = link {
            link.subscribers
                .borrow_mut()
                .insert(self.id, Rc::downgrade(self));
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
pub(super) struct Link {
    pub id: ElementId,
    pub kind: TypeId,
    queue: SyncWeak<Queue>,
    active: Cell<bool>,
    value: RefCell<Option<Rc<dyn Any>>>,
    subscribers: RefCell<HashMap<ElementId, Weak<Dependencies>>>,
}
impl Link {
    fn new<T: 'static>(id: ElementId, queue: &Arc<Queue>, value: Rc<T>) -> Rc<Self> {
        Rc::new(Self {
            id,
            kind: TypeId::of::<T>(),
            queue: Arc::downgrade(queue),
            active: Cell::new(true),
            value: RefCell::new(Some(value)),
            subscribers: RefCell::new(HashMap::new()),
        })
    }
    fn notify(&self) {
        let subscribers: Vec<_> = self
            .subscribers
            .borrow()
            .values()
            .filter_map(Weak::upgrade)
            .collect();
        self.subscribers
            .borrow_mut()
            .retain(|_, subscriber| subscriber.strong_count() > 0);
        for subscriber in subscribers {
            subscriber.mark_changed();
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
        self.dependencies
            .subscribe(TypeId::of::<T>(), link.clone())?;
        Ok(link.and_then(|link| link.value()))
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
struct ValueState<T: 'static> {
    value: Rc<T>,
    notify: Notify<T>,
    changed: bool,
    link: Rc<Link>,
}
impl<T: 'static> ValueState<T> {
    fn replace(&mut self, value: T) {
        let next = Rc::new(value);
        self.changed |= (self.notify)(&self.value, &next);
        self.value = next.clone();
        let previous = self.link.value.replace(Some(next));
        drop(previous);
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
        self.inner.update(|state| state.changed = true)
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
    fn apply_pending(&mut self);
}
pub(super) struct Provided<T: 'static> {
    pub value: T,
    pub notify: Notify<T>,
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
        Box::new(MountedProvider {
            state: ValueState {
                value,
                notify: self.notify,
                changed: false,
                link,
            },
            queue,
        })
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
        self.state.replace(next.value);
        self.deliver();
    }
    fn apply_pending(&mut self) {
        self.queue.drain(&mut self.state);
        self.deliver();
    }
}
impl<T: 'static> MountedProvider<T> {
    fn deliver(&mut self) {
        if std::mem::take(&mut self.state.changed) {
            self.state.link.notify();
        }
    }
}
impl<T: 'static> Drop for MountedProvider<T> {
    fn drop(&mut self) {
        self.queue.close();
        self.state.link.close();
    }
}
