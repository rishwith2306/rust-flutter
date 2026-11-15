//! Thread-safe shared state that wakes the UI after mutations.
use std::sync::{Arc, Mutex};

pub struct Signal<T> {
    inner: Arc<Mutex<T>>,
    context: egui::Context,
}
impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            context: self.context.clone(),
        }
    }
}
impl<T> Signal<T> {
    pub fn new(value: T, context: &egui::Context) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
            context: context.clone(),
        }
    }
    /// Closures must not recursively access this signal.
    pub fn read<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.inner.lock().expect("signal lock poisoned"))
    }
    pub fn update<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let result = f(&mut self.inner.lock().expect("signal lock poisoned"));
        self.context.request_repaint();
        result
    }
    pub fn set(&self, value: T) {
        self.update(|slot| *slot = value);
    }
}
impl<T: Clone> Signal<T> {
    pub fn get(&self) -> T {
        self.read(Clone::clone)
    }
}

/// A background operation's observable state. Errors are explicit application values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskState<T, E> {
    Pending,
    Ready(Result<T, E>),
    Panicked,
}

/// Spawn blocking I/O or computation without blocking the UI. Panics become `Panicked`.
/// Dropping the returned signal does not cancel the worker.
pub fn spawn_task<T, E>(
    ctx: &egui::Context,
    work: impl FnOnce() -> Result<T, E> + Send + 'static,
) -> Signal<TaskState<T, E>>
where
    T: Send + 'static,
    E: Send + 'static,
{
    let state = Signal::new(TaskState::Pending, ctx);
    let worker = state.clone();
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work));
        worker.set(match result {
            Ok(value) => TaskState::Ready(value),
            Err(_) => TaskState::Panicked,
        });
    });
    state
}
