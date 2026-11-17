# Explicit pointer routing

`gestures::PointerRouter<E>` dispatches a host-defined event payload to
pointer-specific subscriptions and global observers. The public router is
UI-local and independent of egui and rendering. It complements the
[gesture arena](gesture-arena.md): routing delivers input, while the arena
decides which competing participant wins.

```rust
use rust_flutter::gestures::{PointerId, PointerRouter};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy)]
enum Event { Move { x: f32, y: f32 }, Up, Cancel }
let router = PointerRouter::<Event>::new();
let positions = Rc::new(RefCell::new(Vec::new()));
let received = positions.clone();
let tracking = router.add_route(PointerId(8), move |_, event| {
    if let Event::Move { x, y } = *event {
        received.borrow_mut().push((x, y));
    }
})?;
router.route(PointerId(8), &Event::Move { x: 12.0, y: 4.0 });
router.route(PointerId(8), &Event::Up);
drop(tracking); // unsubscribe when this sequence finishes
assert_eq!(*positions.borrow(), vec![(12.0, 4.0)]);
assert_eq!(router.route(PointerId(8), &Event::Cancel), 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Retain the returned `PointerRoute<E>` token while tracking input. Dropping it
or calling `cancel()` removes that registration. Unlike arena entries, route
tokens actively unsubscribe on drop. Tokens and `WeakPointerRouter<E>` do not
retain the router. Capture `router.downgrade()` in callbacks that need to route
recursively or register listeners; capturing a strong router creates a reference
cycle until the registration is canceled or the router is disposed.

Each dispatch snapshots all matching registrations before callbacks begin.
Pointer-specific callbacks run before global callbacks, with registration order
within each group. Additions during dispatch wait for another dispatch. An
explicit nested `route` call gets its own snapshot and can see those additions.
Removing a route before its turn prevents that invocation, even when it was
included in the snapshot. Callback captures may remain alive until their
dispatch snapshot is released. Routing borrows no manager state during callbacks
or capture destruction. Panicking callbacks are isolated under ordinary Rust
unwinding; the standard panic hook still runs. `route` returns the number of
callbacks attempted, including panicking ones.

`remove_pointer` unregisters that pointer's routes while preserving global
routes and other pointers. IDs are never reused, so old tokens cannot remove a
new registration for the same pointer. `dispose` is terminal and idempotent:
all tokens become inactive, subsequent events are ignored, and registration
returns `PointerRouterError::Disposed`.

The router deliberately does not interpret payloads, synthesize cancellation,
close/sweep arenas, remove subscriptions on up, normalize device IDs, transform
coordinates or perform hit testing. Hosts must dispatch their cancel payloads
before cleanup, and explicitly end tracking on up/cancel/unmount. The gallery's
primary-pointer adapter now uses this router to track movement outside its
initial hit region and releases its subscription on up or cancellation.
Native tap/drag probes exercise this integration.
[Tap and pan recognizers](gesture-recognizers.md) now use this routing service.
General widget hit-path binding, full recognizer catalog, teams, velocity,
multitouch policy and full Flutter gesture compatibility remain unfinished.

`tests/pointer_router.rs` covers pointer/global order, independent pointers,
reentrant dispatch and registration, cancellation during dispatch, pointer
reuse, terminal disposal, callback panics, nested token destruction and weak
lifetime. A private unit test checks ID exhaustion without partial mutation.
