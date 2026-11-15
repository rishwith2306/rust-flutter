//! UI-local observable state, reentrant listener dispatch, and scoped subscriptions.
use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

/// Callbacks run synchronously on the UI thread and may recursively notify.
pub type Listener = Rc<dyn Fn()>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Disposed;
impl std::fmt::Display for Disposed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("notifier is disposed")
    }
}
impl std::error::Error for Disposed {}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisposeError {
    Disposed,
    DuringNotification,
}
impl std::fmt::Display for DisposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Disposed => "notifier is disposed",
            Self::DuringNotification => "cannot dispose during notification",
        })
    }
}
impl std::error::Error for DisposeError {}

/// Owns one registration. Dropping or cancelling removes exactly that registration.
#[must_use = "dropping a subscription immediately unregisters its listener"]
pub struct Subscription {
    cancel: Option<Box<dyn FnOnce()>>,
}
impl Subscription {
    /// Custom Listenable implementations can supply their own removal operation.
    pub fn new(cancel: impl FnOnce() + 'static) -> Self {
        Self {
            cancel: Some(Box::new(cancel)),
        }
    }
    pub fn cancel(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel();
        }
    }
}
impl Drop for Subscription {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub trait Listenable: 'static {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed>;
    /// Cloneable source handles should compare their underlying source identity.
    fn same_source(&self, other: &Self) -> bool
    where
        Self: Sized;
}
impl<T: Listenable + ?Sized> Listenable for Rc<T> {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
        (**self).subscribe(listener)
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(self, other)
    }
}
/// A value-producing Listenable. Returned snapshots need not borrow the source.
pub trait ValueListenable: Listenable {
    type Value: Clone;
    fn value(&self) -> Self::Value;
}
impl<T: ValueListenable + ?Sized> ValueListenable for Rc<T> {
    type Value = T::Value;
    fn value(&self) -> Self::Value {
        (**self).value()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ListenerId(u64);
static NEXT_LISTENER: AtomicU64 = AtomicU64::new(1);
struct Entry {
    id: ListenerId,
    listener: Listener,
}
#[derive(Default)]
struct Inner {
    listeners: Vec<Option<Entry>>,
    depth: usize,
    disposed: bool,
}
#[derive(Clone, Default)]
pub struct ChangeNotifier {
    inner: Rc<RefCell<Inner>>,
}
#[derive(Clone)]
pub struct WeakChangeNotifier(std::rc::Weak<RefCell<Inner>>);
impl WeakChangeNotifier {
    pub fn upgrade(&self) -> Option<ChangeNotifier> {
        self.0.upgrade().map(|inner| ChangeNotifier { inner })
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NotificationReport {
    /// Invocations in this dispatch, excluding recursive dispatches.
    pub called: usize,
    /// Panicking listeners are isolated; the normal Rust panic hook still runs.
    pub panicked: usize,
}
impl ChangeNotifier {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn downgrade(&self) -> WeakChangeNotifier {
        WeakChangeNotifier(Rc::downgrade(&self.inner))
    }
    pub fn is_disposed(&self) -> bool {
        self.inner.borrow().disposed
    }
    pub fn has_listeners(&self) -> bool {
        self.inner.borrow().listeners.iter().any(Option::is_some)
    }
    pub fn add_listener(&self, listener: Listener) -> Result<ListenerId, Disposed> {
        let mut inner = self.inner.borrow_mut();
        if inner.disposed {
            return Err(Disposed);
        }
        let id = ListenerId(
            NEXT_LISTENER
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
                .expect("listener ID space exhausted"),
        );
        inner.listeners.push(Some(Entry { id, listener }));
        Ok(id)
    }
    pub fn remove_listener(&self, id: ListenerId) -> bool {
        let removed = {
            let mut inner = self.inner.borrow_mut();
            let Some(index) = inner
                .listeners
                .iter()
                .position(|entry| entry.as_ref().is_some_and(|entry| entry.id == id))
            else {
                return false;
            };
            let removed = inner.listeners[index].take();
            if inner.depth == 0 {
                inner.listeners.retain(Option::is_some);
            }
            removed
        };
        // Captured values may have destructors that inspect or modify this notifier.
        drop(removed);
        true
    }
    pub fn notify_listeners(&self) -> Result<NotificationReport, Disposed> {
        let end = {
            let mut inner = self.inner.borrow_mut();
            if inner.disposed {
                return Err(Disposed);
            }
            inner.depth += 1;
            inner.listeners.len()
        };
        let mut report = NotificationReport::default();
        for index in 0..end {
            let listener = self.inner.borrow().listeners[index]
                .as_ref()
                .map(|entry| entry.listener.clone());
            if let Some(listener) = listener {
                report.called += 1;
                if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| listener())).is_err() {
                    report.panicked += 1;
                }
            }
        }
        let mut inner = self.inner.borrow_mut();
        inner.depth -= 1;
        if inner.depth == 0 {
            inner.listeners.retain(Option::is_some);
        }
        Ok(report)
    }
    /// Disposal does not notify, and is rejected during a notification dispatch.
    pub fn dispose(&self) -> Result<(), DisposeError> {
        let listeners = {
            let mut inner = self.inner.borrow_mut();
            if inner.disposed {
                return Err(DisposeError::Disposed);
            }
            if inner.depth != 0 {
                return Err(DisposeError::DuringNotification);
            }
            inner.disposed = true;
            std::mem::take(&mut inner.listeners)
        };
        drop(listeners);
        Ok(())
    }
}
impl Listenable for ChangeNotifier {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
        let id = self.add_listener(listener)?;
        let weak = Rc::downgrade(&self.inner);
        Ok(Subscription::new(move || {
            if let Some(inner) = weak.upgrade() {
                ChangeNotifier { inner }.remove_listener(id);
            }
        }))
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
struct ValueInner<T> {
    value: RefCell<T>,
    changes: ChangeNotifier,
}
pub struct ValueNotifier<T> {
    inner: Rc<ValueInner<T>>,
}
pub struct WeakValueNotifier<T>(std::rc::Weak<ValueInner<T>>);
impl<T> Clone for WeakValueNotifier<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T> WeakValueNotifier<T> {
    pub fn upgrade(&self) -> Option<ValueNotifier<T>> {
        self.0.upgrade().map(|inner| ValueNotifier { inner })
    }
}
impl<T> Clone for ValueNotifier<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
impl<T: 'static> ValueNotifier<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(ValueInner {
                value: RefCell::new(value),
                changes: ChangeNotifier::new(),
            }),
        }
    }
    pub fn downgrade(&self) -> WeakValueNotifier<T> {
        WeakValueNotifier(Rc::downgrade(&self.inner))
    }
    /// Do not recursively mutate this value while a read borrow is active.
    pub fn read<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        read(&self.inner.value.borrow())
    }
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.read(Clone::clone)
    }
    pub fn is_disposed(&self) -> bool {
        self.inner.changes.is_disposed()
    }
    pub fn has_listeners(&self) -> bool {
        self.inner.changes.has_listeners()
    }
    pub fn dispose(&self) -> Result<(), DisposeError> {
        self.inner.changes.dispose()
    }
    /// Explicit invalidation supports application-owned interior-mutable values.
    pub fn notify_listeners(&self) -> Result<NotificationReport, Disposed> {
        self.inner.changes.notify_listeners()
    }
    /// Returns whether equality changed; callbacks run after releasing the value borrow.
    pub fn set(&self, value: T) -> Result<bool, Disposed>
    where
        T: PartialEq,
    {
        if self.is_disposed() {
            return Err(Disposed);
        }
        if *self.inner.value.borrow() == value {
            return Ok(false);
        }
        let old = self.inner.value.replace(value);
        drop(old);
        self.inner.changes.notify_listeners()?;
        Ok(true)
    }
    pub fn update(&self, update: impl FnOnce(&T) -> T) -> Result<bool, Disposed>
    where
        T: PartialEq,
    {
        if self.is_disposed() {
            return Err(Disposed);
        }
        let next = self.read(update);
        self.set(next)
    }
}
impl<T: 'static> Listenable for ValueNotifier<T> {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
        self.inner.changes.subscribe(listener)
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}
impl<T: Clone + 'static> ValueListenable for ValueNotifier<T> {
    type Value = T;
    fn value(&self) -> T {
        self.get()
    }
}

/// A fixed set of sources. One source occurring twice intentionally registers twice.
#[derive(Clone)]
pub struct MergedListenable {
    sources: Rc<Vec<Rc<dyn Listenable>>>,
}
impl MergedListenable {
    pub fn new(sources: Vec<Rc<dyn Listenable>>) -> Self {
        Self {
            sources: Rc::new(sources),
        }
    }
}
impl Listenable for MergedListenable {
    fn subscribe(&self, listener: Listener) -> Result<Subscription, Disposed> {
        let mut subscriptions = Vec::new();
        for source in self.sources.iter() {
            subscriptions.push(source.subscribe(listener.clone())?);
        }
        Ok(Subscription::new(move || drop(subscriptions)))
    }
    fn same_source(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.sources, &other.sources)
    }
}
