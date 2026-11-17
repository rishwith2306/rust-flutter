# Controller-driven simulations

`AnimationController::animate_with` and `animate_back_with` drive an
`Rc<dyn physics::Simulation>` automatically on the controller's owning viewport.
They reuse frame scheduling, listeners/builders, cancellation futures, scoped
TickerMode and managed element ownership. Applications no longer need to sample
a spring or friction model in their build method.

```rust
use rust_flutter::{egui, animation::*, physics::*};
use std::rc::Rc;
let ctx = egui::Context::default();
ctx.begin_pass(egui::RawInput {time: Some(0.0), ..Default::default()});
let controller = AnimationController::new(&ctx, AnimationConfig::default())?;
let spring = SpringSimulation::with_options(
    SpringDescription {mass: 1.0, stiffness: 100.0, damping: 20.0},
    0.0, 0.8, 0.0, Tolerance::default(), true,
)?;
let run = controller.animate_with(Rc::new(spring))?;
let _ = ctx.end_pass();
ctx.begin_pass(egui::RawInput {time: Some(0.1), ..Default::default()});
assert!(controller.value() > 0.0 && controller.value() < 0.8);
assert_eq!(controller.status(), AnimationStatus::Forward);
let _ = ctx.end_pass();
ctx.begin_pass(egui::RawInput {time: Some(2.0), ..Default::default()});
assert_eq!(controller.value(), 0.8);
assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
let _ = ctx.end_pass();
controller.dispose()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Sampling, bounds and direction

Starting a model samples it at zero, immediately sets the clamped initial value,
notifies value listeners, cancels previous playback and schedules a frame.
An initially done sample completes immediately. Subsequent samples use elapsed
Duration from the command's frame time. Equal/backward/nonfinite host tick times
do not advance playback. A model should be pure and inexpensive, as described
in [physics](physics.md); elapsed times can skip frames during suspension.

Finite f64 positions clamp to the controller's finite f32 bounds before conversion.
Touching a bound does not complete the run: the model's `done` flag controls
completion. Forward/reverse status comes from the chosen method, independently
of velocity sign or numerical destination. Successful forward playback ends
Completed, and successful backward playback ends Dismissed, including for interior
endpoints. While active, statuses stay Forward/Reverse. No simulation velocity
getter or unbounded controller is provided yet.

Manual/scoped muting suppresses ticks but retains the model and run. Elapsed time
continues, so the next unmuted owning pass catches up. Explicit commands still
work while muted. Existing stop, set_value, reset, replacement, disposal and final
handle drop cancel/finish custom playback using the same controller policies.

## Failures and reentrancy

A failed, panicking or nonfinite initial sample returns AnimationError without
replacing existing playback. Custom sampling happens outside controller state
borrows. A model may itself issue commands; those side effects are not rolled
back. If a valid initial sample issues a playback command, the outer start
returns Superseded rather than overwrite that command. Disposal returns Disposed.
Playback-changing commands include stop even when no run was active.

A later model error, panic, nonfinite position/velocity, or elapsed Duration
overflow stops that run and resolves its future to **AnimationOutcome::Failed**
with an AnimationError cause. The last finite value and directional status remain.
This is an experimental API extension: exhaustive matches on AnimationOutcome
must now handle Failed. A failure is neither successful completion nor cancellation.
Recovery commands can start new playback normally. Panic isolation requires
unwinding; an aborting panic terminates the process. Destructors are ordinary Rust
destructors, not a second sandbox for panicking application code.

Ticks apply a result only while the same run remains active and the controller
is live and unmuted. A model that replaces/stops/disposes its owner cannot write
an obsolete value, completion or failure into a replacement. Completion wakes
and final model drops happen outside state borrows, so callbacks/destructors can
use controller commands. Nested driver ticks during sampling are suppressed.
Use weak controller captures in models/listeners to avoid ownership cycles.
The model and controller remain UI-local; this does not add an async executor.

## Evidence and remaining work

Deterministic tests cover analytic spring progress, directional completion,
clamping without early completion, initially done models, failed/panicking
preflight rollback, later failures/future wake, muting/duplicate times, reentrant
replacement/stop/disposal, nested custom starts, destructor reentrancy,
nonfinite input and listener-triggered replacement.

The gallery's orange marker uses a managed critical spring controller, with
12/24-point endpoints. The existing purple implicit-animation target drives both
markers; they share the TickerMode scope. Native probes require exact orange
pixels, both endpoints, stable muted geometry, and intermediate/settled debug
samples alongside all existing interaction checks. The gallery restarts a spring
from its current value with zero initial velocity; it does not preserve retarget
velocity. These checks establish rendered progress, not native scrolling,
platform motion feel or a performance budget.

Reviewed API references:
[animateWith](https://api.flutter.dev/flutter/animation/AnimationController/animateWith.html)
and [animateBackWith](https://api.flutter.dev/flutter/animation/AnimationController/animateBackWith.html).
Unbounded/velocity controllers, fling convenience APIs, runtime configuration,
platform scroll simulations, wheel/nested scroll integration, slivers, overscroll,
animation accessibility policy and full member compatibility remain unfinished.
