//! Thread-safe shared state that wakes the UI after mutations.
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

type SignalListener = Arc<dyn Fn() + Send + Sync>;
struct SignalObserver {
    id: u64,
    listener: SignalListener,
}

pub struct Signal<T> {
    inner: Arc<Mutex<T>>,
    observers: Arc<Mutex<Vec<SignalObserver>>>,
    context: egui::Context,
}
static NEXT_SIGNAL_OBSERVER: AtomicU64 = AtomicU64::new(1);

/// Owned cross-thread signal listener registration. Dropping it unregisters
/// that exact callback.
pub struct SignalSubscription {
    observers: Arc<Mutex<Vec<SignalObserver>>>,
    id: u64,
}
impl Drop for SignalSubscription {
    fn drop(&mut self) {
        let removed = {
            let mut observers = self.observers.lock().unwrap();
            observers
                .iter()
                .position(|observer| observer.id == self.id)
                .map(|index| observers.remove(index))
        };
        drop(removed);
    }
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            observers: self.observers.clone(),
            context: self.context.clone(),
        }
    }
}
impl<T> Signal<T> {
    pub fn new(value: T, context: &egui::Context) -> Self {
        Self {
            inner: Arc::new(Mutex::new(value)),
            observers: Arc::new(Mutex::new(Vec::new())),
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
        let listeners = self
            .observers
            .lock()
            .expect("signal observers poisoned")
            .iter()
            .map(|observer| observer.listener.clone())
            .collect::<Vec<_>>();
        for listener in listeners {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| listener()));
        }
        result
    }
    pub fn set(&self, value: T) {
        self.update(|slot| *slot = value);
    }

    pub fn same_source(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }

    /// Listen for value mutations from any thread. Callbacks execute after the
    /// value lock has been released, so they may safely schedule UI rebuilds.
    pub fn subscribe(&self, listener: impl Fn() + Send + Sync + 'static) -> SignalSubscription {
        let mut id = NEXT_SIGNAL_OBSERVER.load(Ordering::Relaxed);
        loop {
            let next = id
                .checked_add(1)
                .expect("signal observer ID space exhausted");
            match NEXT_SIGNAL_OBSERVER.compare_exchange_weak(
                id,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => id = actual,
            }
        }
        self.observers
            .lock()
            .expect("signal observers poisoned")
            .push(SignalObserver {
                id,
                listener: Arc::new(listener),
            });
        SignalSubscription {
            observers: self.observers.clone(),
            id,
        }
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
    Cancelled,
}

/// Cooperative cancellation signal passed to background work.
#[derive(Clone, Debug)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    /// Returns true after the owning task has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// A cancellable background task and its observable result.
pub struct CancellableTask<T, E> {
    state: Signal<TaskState<T, E>>,
    cancellation: Arc<AtomicBool>,
}

impl<T, E> CancellableTask<T, E> {
    pub fn state(&self) -> &Signal<TaskState<T, E>> {
        &self.state
    }

    /// Requests cancellation. Work must check its token to stop promptly.
    pub fn cancel(&self) {
        self.cancellation.store(true, Ordering::Release);
    }
}

/// Lifecycle of a background value stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamStatus {
    Pending,
    Active,
    Done,
    Panicked,
    Cancelled,
}

/// Latest stream snapshot. Values are shared so cloning a snapshot does not
/// require `T` or `E` to implement `Clone`.
#[derive(Debug)]
pub struct StreamState<T, E> {
    pub status: StreamStatus,
    pub data: Option<Arc<T>>,
    pub error: Option<Arc<E>>,
}
impl<T, E> Clone for StreamState<T, E> {
    fn clone(&self) -> Self {
        Self {
            status: self.status,
            data: self.data.clone(),
            error: self.error.clone(),
        }
    }
}

/// Handle to a background producer and its repainting snapshot signal.
pub struct TaskStream<T, E> {
    state: Signal<StreamState<T, E>>,
    cancellation: CancellationToken,
}

impl<T, E> TaskStream<T, E> {
    pub fn state(&self) -> &Signal<StreamState<T, E>> {
        &self.state
    }

    /// Requests cooperative cancellation. The producer must check its sink.
    pub fn cancel(&self) {
        self.cancellation.0.store(true, Ordering::Release);
    }

    /// Build a retained subtree from the latest stream snapshot and rebuild it
    /// whenever the worker publishes an update.
    pub fn builder(
        &self,
        builder: impl Fn(&crate::element::BuildContext, StreamState<T, E>) -> crate::element::Widget
        + 'static,
    ) -> crate::element::Widget
    where
        T: Send + Sync + 'static,
        E: Send + Sync + 'static,
    {
        crate::element::Widget::signal_builder(self.state.clone(), builder)
    }
}

/// Producer side of a background stream. Each update wakes the UI for another
/// frame; the snapshot retains the latest data when the producer completes.
pub struct StreamSink<T, E> {
    state: Signal<StreamState<T, E>>,
    cancellation: CancellationToken,
}

impl<T, E> StreamSink<T, E> {
    /// Publishes a value and clears the previous error. Returns false if cancelled.
    pub fn emit(&self, value: T) -> bool {
        if self.cancellation.is_cancelled() {
            return false;
        }
        self.state.update(|state| {
            state.status = StreamStatus::Active;
            state.data = Some(Arc::new(value));
            state.error = None;
        });
        true
    }

    /// Publishes a recoverable error while retaining the latest data.
    pub fn report_error(&self, error: E) -> bool {
        if self.cancellation.is_cancelled() {
            return false;
        }
        self.state.update(|state| {
            state.status = StreamStatus::Active;
            state.error = Some(Arc::new(error));
        });
        true
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }
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

/// Spawn blocking work that can stop cooperatively by checking its token.
/// Cancellation is reflected in the signal and takes precedence over a result
/// returned after cancellation was requested.
pub fn spawn_cancellable_task<T, E>(
    ctx: &egui::Context,
    work: impl FnOnce(CancellationToken) -> Result<T, E> + Send + 'static,
) -> CancellableTask<T, E>
where
    T: Send + 'static,
    E: Send + 'static,
{
    let state = Signal::new(TaskState::Pending, ctx);
    let worker = state.clone();
    let cancellation = Arc::new(AtomicBool::new(false));
    let token = CancellationToken(cancellation.clone());
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(token.clone())));
        worker.set(if token.is_cancelled() {
            TaskState::Cancelled
        } else {
            match result {
                Ok(value) => TaskState::Ready(value),
                Err(_) => TaskState::Panicked,
            }
        });
    });
    CancellableTask {
        state,
        cancellation,
    }
}

/// Runs a synchronous producer on a background thread and publishes its latest
/// data/error snapshot through a repainting signal. Check `sink.is_cancelled()`
/// periodically during long-running work. Panics and completion are retained
/// in the final stream snapshot.
pub fn spawn_stream<T, E>(
    ctx: &egui::Context,
    work: impl FnOnce(StreamSink<T, E>) -> Result<(), E> + Send + 'static,
) -> TaskStream<T, E>
where
    T: Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    let state = Signal::new(
        StreamState {
            status: StreamStatus::Pending,
            data: None,
            error: None,
        },
        ctx,
    );
    let cancellation = CancellationToken(Arc::new(AtomicBool::new(false)));
    let sink = StreamSink {
        state: state.clone(),
        cancellation: cancellation.clone(),
    };
    let worker = state.clone();
    let token = cancellation.clone();
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(sink)));
        worker.update(|state| {
            state.status = if token.is_cancelled() {
                StreamStatus::Cancelled
            } else {
                match result {
                    Ok(Ok(())) => StreamStatus::Done,
                    Ok(Err(error)) => {
                        state.error = Some(Arc::new(error));
                        StreamStatus::Done
                    }
                    Err(_) => StreamStatus::Panicked,
                }
            };
        });
    });
    TaskStream {
        state,
        cancellation,
    }
}
