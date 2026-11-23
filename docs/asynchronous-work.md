# Background tasks and streams

`state::spawn_task` runs one blocking operation on a worker thread and publishes
its result. `spawn_cancellable_task` adds cooperative cancellation. For repeated
updates, `state::spawn_stream` gives a producer a `StreamSink<T, E>` and returns a
`TaskStream<T, E>` whose `Signal<StreamState<T, E>>` keeps the latest data/error
snapshot and wakes the UI after every update.

```rust,ignore
use rust_flutter::state::{StreamStatus, TaskStream, spawn_stream};

let updates: TaskStream<u32, String> = spawn_stream(ctx, |sink| {
    for value in 0..100 {
        if sink.is_cancelled() {
            return Ok(());
        }
        sink.emit(value);
    }
    Ok(())
});

let snapshot = updates.state().get();
match snapshot.status {
    StreamStatus::Pending => { /* show a loading state */ }
    StreamStatus::Active => { /* read snapshot.data / snapshot.error */ }
    StreamStatus::Done => { /* final data remains in the snapshot */ }
    StreamStatus::Panicked => { /* producer panicked */ }
    StreamStatus::Cancelled => { /* cancellation was requested */ }
}
```

Keep the `TaskStream` handle in application state and read its signal while
building the UI. `StreamSink::emit` replaces the latest data and clears an older
error. `report_error` retains the latest data while publishing an error; the
producer may continue and emit a later value. A returned `Err` stores the error
and completes the stream. Panics and cooperative cancellation are terminal.

The worker is synchronous Rust on a framework-owned thread. Check
`StreamSink::is_cancelled` inside long loops; cancellation does not forcibly stop
blocking work. Signals request UI repaints, but they do not provide an async
runtime, event history, backpressure, or guaranteed rendering of every rapid
intermediate update. Use Tokio or another executor for async I/O and advanced
stream scheduling.
