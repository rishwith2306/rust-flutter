# Tap and pan recognizers

`gestures::TapGestureRecognizer` and `PanGestureRecognizer` join the public
[arena](gesture-arena.md) and subscribe to the [pointer router](pointer-routing.md).
Hosts still perform initial hit testing, normalize input and register the down
with each target recognizer. A recognizer tracks one primary-pointer sequence
at a time; extra pointers are ignored until that sequence ends. Instances without
any callbacks do not register. These are initial recognizers, not the complete
Flutter gesture system. A [retained detector and shared root binding](gesture-binding.md)
now connect them to layout hit paths.

```rust
use rust_flutter::{egui::Pos2, gestures::*};
use std::{cell::Cell, rc::Rc, time::Duration};

let arena = GestureArenaManager::new();
let router = PointerRouter::new();
let taps = Rc::new(Cell::new(0));
let count = taps.clone();
let tap = TapGestureRecognizer::new(&arena, &router, TapConfig::default(),
    TapCallbacks { on_tap: Some(Rc::new(move |_| count.set(count.get() + 1))),
        ..Default::default() })?;
let pan = PanGestureRecognizer::new(&arena, &router, PanConfig::default(),
    PanCallbacks { on_start: Some(Rc::new(|start| println!("pan at {:?}", start.position))),
        ..Default::default() })?;
let down = PointerEvent { pointer: PointerId(8), phase: PointerPhase::Down,
    position: Pos2::ZERO, time: Duration::ZERO, device: PointerDeviceKind::Touch,
    buttons: PointerButtons::PRIMARY };
// Register the entire pointer-down hit path before closing/flushing this arena.
tap.add_pointer(down)?;
pan.add_pointer(down)?;
arena.close(down.pointer)?;
arena.flush();
let up = PointerEvent { phase: PointerPhase::Up, time: Duration::from_millis(80),
    buttons: PointerButtons::NONE, ..down };
router.route(up.pointer, &up);
arena.sweep(up.pointer)?;
arena.flush();
assert_eq!(taps.get(), 1);
assert_eq!(router.route_count(), 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Keep the arena and router alive for the recognizers' lifetime: recognizers hold
weak service references. Registering returns `true` if the down was joined,
even if an immediate callback subsequently canceled it. `false` means an active
sequence, no callbacks or buttons other than exactly primary. Invalid down
geometry/phase, disposed services/recognizers, exhausted IDs and a closed arena
report errors. Registration failure rolls back routing without leaving a new
empty open arena.

`PointerEvent` carries a phase, pointer ID, logical global position, monotonic
`Duration` timestamp, device label and button bitmask. Hosts map physical devices
to non-colliding IDs and convert coordinates before delivery. Move samples must
keep the initial primary button; up must have no buttons. Device or button
changes cancel the sequence. Older timestamps are ignored, while equal times
are accepted for input batches. Nonfinite geometry or unrepresentable movement
deltas cancel. Cancel always terminates, even with unusable geometry/time. A
router key that differs from the payload pointer is ignored. Device labels do
not yet select device-specific thresholds or filters.

## Tap policy

`on_down` is immediate candidate feedback. `on_tap` receives the down and final up
samples only after both pointer-up and an arena win. If the arena resolves while
the pointer is still down, completion waits for up. If up arrives while the arena
is open, the recognizer removes its route, records an acceptance claim and waits
for the host's later close/resolution. There is no tap deadline timer yet.

Movement strictly greater than the configured radial slop cancels. TapConfig
has separate pre-accept and post-accept limits, each defaulting to 18 logical
points. An early win therefore does not guarantee a later completed tap.
Rejection, cancellation, disposal or final-handle drop invokes `on_cancel` once
for an active candidate, releases tracking and withdraws any unresolved claim.

## Pan policy

`on_down` is immediate candidate feedback. Movement strictly beyond PanConfig's
radial displacement threshold (default 18 logical points from down) claims the
arena. `on_start` requires both that threshold and an arena win. Even a sole
participant that wins early waits for the threshold. Up below threshold cancels.
A final up sample can cross the threshold, producing start then end.

With `DragStartBehavior::Start`, start uses the latest recognition position and
pre-start movement is omitted. With `Down`, start uses the original down sample
and the first update contains accumulated displacement. Subsequent nonzero
updates report global logical deltas. Up delivers any final movement before
`on_end`; end currently contains the final sample without velocity or fling
estimation. Cancel can follow start and replaces end when a running pan stops.

## Ownership and reentrancy

Cloning a recognizer retains it; dropping its final strong handle cancels active
tracking. `cancel` is reusable, while `dispose` is terminal and idempotent. Both
remove the current route and reject an unresolved arena entry. Cancel each
recognizer before disposing its router: removing a route alone does not
synthesize an input cancellation or reject its arena claim. Arena disposal does
notify and cancel its remaining participants. A winner still tracking input
needs explicit recognizer cancellation when its host is removed.

Callbacks run without recognizer state borrows; callback panics are isolated
under Rust unwinding. The standard panic hook still runs. A per-recognizer work
queue defers reentrant input and arena acceptance until the current notification
packet finishes. Direct cancellation/disposal remains immediate and suppresses
pending updates/end. Generation checks protect a new sequence from stale work.
`downgrade()` supplies weak handles for callback captures, avoiding ownership
cycles. This queue does not replace a host-wide event binding; hosts permitting
nested pointer reuse must also keep their up/sweep cleanup tied to the original
sequence, rather than sweeping a newly created arena with the same pointer ID.

The gallery now uses the public retained GestureDetector and shared root binding
instead of a private primary-pointer host. Its normalized egui events use
`Unknown` device labels, since that stream alone does not distinguish mouse from
emulated touch. Native probes inject a drag outside the initial hit region,
verify tap rejection/drag feedback, then tap again to verify reuse.

The integration suite covers competition, boundaries, early wins, delayed
resolution, accumulated/start-position behavior, cancellation and teardown,
protocol errors/rollback, callback panics and reentrant input/reuse. A private
test verifies sequence-ID exhaustion rollback.

Global multi-viewport binding, general backend control arbitration, secondary/tertiary taps, deadlines, double/long
taps, axis drags, device settings, velocity/flings, drag teams, multitouch
strategies, local transforms, scale/rotation, pointer signals and pan/zoom events,
scrolling physics and semantics remain unfinished. The immediate tap-down timing
and radial pan threshold are explicit current policies. Full member-level
compatibility has not been established.

Reviewed references: [TapGestureRecognizer](https://api.flutter.dev/flutter/gestures/TapGestureRecognizer-class.html),
[PanGestureRecognizer](https://api.flutter.dev/flutter/gestures/PanGestureRecognizer-class.html),
[OneSequenceGestureRecognizer](https://api.flutter.dev/flutter/gestures/OneSequenceGestureRecognizer-class.html).
