# Scroll positions and controller attachments

The public `scrolling` module separates persistent offset/activity from layout.
`ScrollPosition` is a UI-local, observable finite f32 offset driven by its owning
egui context/viewport. `ScrollController` observes and commands attached positions.
[SingleChildScrollView](scroll-viewports.md) now connects this state to retained
layout, clipping and primary-pointer pan input. [Wheel/point signal routing](scroll-signals.md) is available; slivers and
platform scroll physics remain unfinished. The existing
`widgets::scroll` helper continues to use an independent egui ScrollArea.

```rust
use rust_flutter::{egui, animation::Curve, layout::Axis, scrolling::*};
use std::time::Duration;
let ctx = egui::Context::default();
ctx.begin_pass(egui::RawInput {time: Some(0.0), ..Default::default()});
let controller = ScrollController::new(20.0)?;
let position = controller.create_position(&ctx)?;
let attachment = controller.attach(&position)?;
assert_eq!(controller.offset()?, 20.0);
// A viewport/layout owner publishes these dimensions after measuring content.
position.apply_dimensions(ScrollDimensions::new(0.0, 300.0, 100.0, Axis::Vertical)?)?;
let runs = controller.animate_to(200.0, Duration::from_secs(1), Curve::Linear)?;
let _ = ctx.end_pass();
ctx.begin_pass(egui::RawInput {time: Some(0.5), ..Default::default()});
assert_eq!(position.pixels(), 110.0);
assert!(position.is_scrolling());
let _ = ctx.end_pass();
position.begin_drag()?; // interrupts the programmatic run
assert_eq!(runs[0].outcome(), Some(rust_flutter::animation::AnimationOutcome::Canceled));
assert_eq!(position.drag_by(250.0)?, 60.0); // clamps at 300
assert_eq!(position.pixels(), 300.0);
position.cancel_activity()?;
drop(attachment);
assert!(!controller.has_clients());
position.dispose()?;
controller.dispose()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Position and dimensions

Initial offsets and pre-layout jumps can be any finite f32. Dimensions are
initially absent; drag and animation return NoDimensions until supplied.
`ScrollDimensions::new` validates finite ordered extents, a nonnegative finite
viewport dimension and an axis. Equal extents are allowed. Applying dimensions
clamps the offset. Subsequent jumps, animated targets and drag updates clamp to
the range. There is no out-of-range overscroll storage in this implementation.

Identical dimensions are a no-op. Changed dimensions cancel existing activity,
correct offset and return to Idle. This explicit policy also applies to resize
and axis changes; adapting an existing simulation to new dimensions is future
work. Layout owners must publish dimensions outside their active render-object
borrows because notification callbacks can run application code synchronously.

`metrics()` returns an optional snapshot of pixels and dimensions. Before/after
extents use f64 arithmetic so finite extreme f32 extents do not overflow.
`extent_inside` is the viewport dimension; it does not represent a stretched or
overscrolled viewport. Getters retain their final snapshots after disposal.

Pixel changes notify `Listenable` subscribers; equality is quiet. The position
also implements ValueListenable<Value=f32> for retained observable builders.
Dimension changes use `subscribe_metrics`, and activity changes use
`subscribe_activity`. Controller listeners forward only pixel changes, not
attachment/dimension/activity events. Notification callbacks read current state;
reentrant commands may change that state before later callbacks run.

## Motion

`jump_to` cancels playback and moves immediately. `animate_to` requires dimensions,
a finite target and nonzero duration, returning the shared controller AnimationRun.
Its Curve applies over the full specified duration. `begin_drag` cancels prior
motion; `drag_by` requires Drag and accepts content-offset delta, with positive
values moving toward the maximum. It returns unapplied distance as f64. Finger
coordinate direction, axis selection and reverse scrolling are adapter work.

`end_drag(velocity, drag)` starts bounded exponential friction using explicit
content-offset velocity in points/second and `0 < drag < 1`. Contact stops at an
extent. A stationary release completes immediately. Invalid release data leaves
the drag active. This is a configurable fluid-drag model, not Android's
ClampingScrollSimulation or a macOS overscroll policy. The existing
[pan release details](gesture-velocity.md) can supply velocity when an adapter
performs coordinate conversion. `cancel_activity` preserves pixels and returns
Idle. Activities are Idle, Drag, Driven and Ballistic.

Animation follows owning viewport time and can skip intermediate frames. This
does not yet integrate ScrollPosition with TickerMode or automatic element
ownership. Owners must cancel/dispose as appropriate. Position disposal and final
handle drop cancel pending runs; controller attachment can keep a position alive.

## Attachments, broadcast and lifetime

Retain `ScrollAttachment` for the lifetime of a client. Drop or explicit detach
removes exactly that registration and forwarding subscription. Duplicate attachment
to the same controller is rejected. A position may be observed by distinct
controllers, but has one offset/activity; this does not create independent views.
Attachment does not itself dispose a position. Controller disposal detaches
clients and clears its listeners while leaving independently owned positions live.

`offset`/`position` require exactly one attachment; zero/multiple clients return
NoClients/MultipleClients. `positions` returns a snapshot. Broadcast jump/animation
validates input and known preconditions for all snapshot clients before starting;
animation returns a vector of runs for the clients it actually starts.
Each client clamps to its own extents. A later client removed or independently
commanded during an earlier callback is skipped. Clients attached during dispatch
wait for the next command. A newer controller broadcast supersedes pending work
from the older one. This is reentrant dispatch, not an all-client transaction.

Position commands likewise guard trailing activity changes against newer commands
issued by listeners or cancellation wakes. A dimensions/drag/cancel command
superseded during cancellation can return Superseded. Notification borrows are
released before callbacks; disposal during that object's notification is rejected
with DuringNotification. Weak position/controller handles avoid capture cycles.
Subscriptions use the ordinary notifier's panic isolation and owned cleanup.

## Scope and references

Tests cover validation, metrics correction, separate notifications, drag bounds,
programmatic/ballistic motion, interruption/completion, attachment ownership,
multi-client commands, reentrant detach/replacement, disposal and final-drop
cancellation. These establish state and mathematical behavior, not native scroll
input, rendering, accessibility or performance.

Reviewed [ScrollController](https://api.flutter.dev/flutter/widgets/ScrollController-class.html)
and [ScrollPosition](https://api.flutter.dev/flutter/widgets/ScrollPosition-class.html).
This is a partial Rust counterpart. Full trackpad phase/device support, ScrollPhysics composition, overscroll, restoration,
notifications/bubbling, ensureVisible, activity/context adoption, tracking/primary
controllers, scrollbars, accessibility, lazy slivers and full member parity remain
unfinished. [SingleChildScrollView](https://api.flutter.dev/flutter/widgets/SingleChildScrollView-class.html)
now has a partial retained Rust counterpart; its remaining scope is recorded separately.
