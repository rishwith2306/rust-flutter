# Gesture arena arbitration

`gestures::GestureArenaManager` arbitrates recognizers competing for a pointer
sequence. It is a UI-local mechanism, independent of rendering and input devices.
Hosts/recognizers provide pointer routing and movement/timing decisions. This
adds the arena kernel; it does not yet supply a general GestureDetector widget
or automatically connect every egui control to the same arena.

```rust
use rust_flutter::gestures::*;
use std::{cell::RefCell, rc::Rc};
struct Candidate { name: &'static str, log: Rc<RefCell<Vec<(&'static str, bool)>>> }
impl GestureArenaMember for Candidate {
    fn accept_gesture(&self, _: PointerId) { self.log.borrow_mut().push((self.name, true)); }
    fn reject_gesture(&self, _: PointerId) { self.log.borrow_mut().push((self.name, false)); }
}
let manager = GestureArenaManager::new();
let pointer = PointerId(7);
let log = Rc::new(RefCell::new(Vec::new()));
let tap = manager.add(pointer, Rc::new(Candidate { name: "tap", log: log.clone() }))?;
let _drag = manager.add(pointer, Rc::new(Candidate { name: "drag", log: log.clone() }))?;
manager.close(pointer)?; // finished registering the pointer-down hit path
assert!(log.borrow().is_empty());
// A recognizer decides this sequence is a tap, e.g. on up within its slop policy.
assert!(tap.resolve(GestureDisposition::Accepted));
assert_eq!(*log.borrow(), vec![("drag", false), ("tap", true)]);
manager.sweep(pointer)?; // already resolved: harmless
assert_eq!(manager.active_pointers(), 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Lifecycle and resolution

`add` joins an open arena, creating one if absent. `close` prevents further
registrations and starts resolution. An accept claim while open is remembered;
with multiple participants, closing chooses the first still-active claimant.
An explicit accept after close resolves immediately. A repeated accept claim
is a no-op. Rejecting removes the registration and calls its reject callback.
A recognizer can withdraw an eager claim before final resolution; the next
remaining claim keeps its original acceptance order.

When a closed arena has one member left, the manager schedules a deferred
default. Call `flush()` after the relevant event dispatch/microtask boundary.
This gives the last participant an opportunity to reject before default victory.
Flush processes the queue snapshot present when called. Defaults queued by
callbacks wait for another flush, and reentrant flush does not drain them.
Explicit resolutions can still occur during callbacks. If a closed arena has no
participants left, it is removed without an accept callback. An empty open arena can still receive new registrations before close.

A host normally calls `sweep` after dispatching pointer-up. Sweep chooses the
first registration when participants remain undecided. `hold` delays sweep;
a swept held arena records pending_sweep, and `release` performs it. Hold only
blocks sweep: it does not block explicit acceptance or a deferred sole-member
winner. Sweeping an open arena reports ArenaError::StillOpen. Adding after close
reports Closed. Operations on already resolved/missing pointers are harmless.

Explicit/default resolution rejects losers in registration order before
accepting the winner. Sweep accepts the first member, then rejects the others.
Registrations are removed before final callbacks run. Callback state borrows are
released, allowing reentrant resolution, cancellation, disposal and registration
of new arenas. Notifications for a committed resolution finish even if one of
those callbacks disposes the manager. A panicking callback does not prevent the
remaining notifications under ordinary Rust unwinding.

Each registration receives one final accept or reject callback. An arena win is
arbitration, not necessarily a completed user gesture: a recognizer may win
before pointer-up and must still implement its own gesture timing/callback rules.
Register each recognizer once per pointer unless separate registrations and
separate final notifications are deliberately wanted.

## Pointer reuse and lifetime

PointerId is a host-assigned u64 identity. Active contacts must have distinct
IDs. Independent pointers have separate arenas. Entry handles and deferred
defaults also carry internal generations, so stale work cannot resolve a later
arena reusing the same PointerId. ID exhaustion reports an error without leaving
a partially created registration.

Entry handles are cloneable weak tokens. They do not keep the manager alive.
Dropping a token does not withdraw a participant: reject explicitly when a
recognizer stops tracking before resolution. This permits passive participants
which only receive arena outcomes. The manager holds Rc members while their
registrations remain active. Avoid a member-to-manager strong-reference cycle;
use `downgrade` or store weak entry tokens when callbacks need the manager.

`cancel(pointer)` rejects remaining participants and removes that arena, as on
pointer cancellation. `dispose` rejects all unresolved registrations, clears
defaults and prevents new work. Dropping the final manager handle also rejects
remaining registrations. Committed winners are not later rejected by cancel.
`arena(pointer)` returns an immutable snapshot of member count, open/held state,
pending sweep and eager-claim presence. It exposes no live RefCell borrow.

## Native validation adapter and remaining scope

The [gallery adapter](../examples/gallery/src/gesture_marker.rs) supplies a
primary-pointer experiment over raw egui PointerButton/PointerMoved events.
A down inside the marker registers tap and drag candidates, then closes the
arena. Movement beyond a fixed 18 logical points rejects tap and accepts drag;
otherwise up accepts tap. It sweeps on up, flushes after event dispatch, and
cancels on pointer loss, focus loss or disposal. The adapter deduplicates egui
multipass delivery per frame. It is gallery code, not the public widget input
binding.

The native probe injects taps to drive the existing animation, then drags outside
the marker. The drag must win, render an exact pink feedback swatch and leave the
marker unexpanded. A subsequent tap/return cycle verifies pointer reuse. Native
traces require both tap and drag winners and a rejected tap. Existing animation,
ticker mode, completion, exact-color, covered-child and baseline gates remain.
Linux uses actual xdotool pointer events; Android uses input tap/swipe on an
isolated emulator.

`tests/gesture_arena.rs` covers eager and explicit claims, deferred defaults,
withdrawal, hold/release/sweep ordering, cancellation, stale generation safety,
reentrant callbacks and pointer recreation, flush boundaries, disposal/drop,
protocol errors, independent pointers and panic isolation. A private test checks
ID exhaustion rollback.

General widget-tree hit-path dispatch, arena participation by backend controls,
multitouch normalization/recognizer routing, recognizer teams, tap deadlines,
double/long/secondary taps, drag detail streams and velocity, scale/rotation,
scrolling/physics, semantics and complete Flutter gesture member compatibility
remain unfinished. This arena is a foundation for those capabilities, not a
claim that the full gesture system exists.

References: [GestureArenaManager](https://api.flutter.dev/flutter/gestures/GestureArenaManager-class.html),
[close](https://api.flutter.dev/flutter/gestures/GestureArenaManager/close.html),
[sweep](https://api.flutter.dev/flutter/gestures/GestureArenaManager/sweep.html),
[hold](https://api.flutter.dev/flutter/gestures/GestureArenaManager/hold.html),
[GestureBinding](https://api.flutter.dev/flutter/gestures/GestureBinding-mixin.html).
