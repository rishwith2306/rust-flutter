# Frame-driven animation controllers

`animation::AnimationController` is a UI-local, bounded scalar animation source.
It advances automatically at the start of its owning egui viewport's passes and
requests more frames while active and unmuted. Applications do not need to call a
sampling/tick function each frame. It implements Listenable and
ValueListenable<Value = f32>, so existing observable widget builders can animate
layout or paint configuration while preserving retained element identity. For
target-driven animation without an app-managed controller, see the
[implicit value builder](implicit-animation.md). For derived/custom cubic easing
and distinct forward/reverse mappings, see [CurvedAnimation](curved-animations.md).

```rust
use rust_flutter::{egui, animation::{AnimationConfig, AnimationController, AnimationOutcome},
    element::{ElementTree, Widget}, layout::BoxConstraints};
use std::time::Duration;
let ctx = egui::Context::default();
ctx.begin_pass(egui::RawInput { time: Some(0.0), ..Default::default() });
let controller = AnimationController::new(&ctx, AnimationConfig {
    duration: Duration::from_secs(1), ..Default::default()
})?;
let mut tree = ElementTree::new(&ctx, Widget::value_listenable_builder(
    controller.clone(), |_, value| Widget::leaf(egui::vec2(24.0 + 24.0 * value, 24.0), |ui| {
        ui.painter().rect_filled(ui.max_rect(), 0.0, egui::Color32::LIGHT_BLUE);
    }),
))?;
let run = controller.forward()?;
let _ = ctx.end_pass();
// Normal hosts supply elapsed monotonic input time on each pass.
ctx.begin_pass(egui::RawInput { time: Some(0.5), ..Default::default() });
assert_eq!(controller.value(), 0.5);
egui::CentralPanel::default().show(&ctx, |ui| {
    tree.show(ui, BoxConstraints::loose(egui::vec2(200.0, 80.0)).unwrap()).unwrap();
});
let _ = ctx.end_pass();
ctx.begin_pass(egui::RawInput { time: Some(1.0), ..Default::default() });
assert_eq!(run.outcome(), Some(AnimationOutcome::Completed));
let _ = ctx.end_pass();
controller.dispose()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Playback

Bounds default to 0…1 and must be finite with lower < upper. Initial values,
`set_value` and animation targets are clamped to those bounds; NaN and infinity
are rejected. Interpolation uses a wider intermediate so finite extreme bounds
do not overflow through subtraction. Configuration is currently fixed at
construction. Duration zero is allowed for instantaneous point-to-point runs.
`normalized_value()` maps the current value from the configured bounds to 0…1,
which is useful when a consumer needs a phase independent of those bounds.

`forward()` moves toward the upper bound and `reverse()` toward the lower bound.
Their default duration is proportional to the remaining fraction of the full
range. Reverse uses reverse_duration when configured. `animate_to` and
`animate_back` accept an optional explicit duration and one of the existing
Curve variants. An explicit duration covers the whole requested trip. Status
is Forward/Reverse according to the selected method, independently of whether
the target is numerically above or below the current value. Successful
point-to-point playback ends Completed/Dismissed according to that direction,
even for interior targets. The last direction survives completion and reset
for subsequent interior `set_value` status calculations.

`repeat(RepeatOptions)` accepts an in-bound min/max span, a nonzero period,
optional positive u32 count and optional reverse alternation. The period covers
one leg. It starts in the forward direction from the current normalized position
within the repeat range. An initial partial leg counts toward count. Without a
count it repeats indefinitely. Reversing runs alternate max/min endpoints;
finite runs finish at the final leg's endpoint with Completed/Dismissed status.
Repeat bounds require a strictly positive span.

`stop(true)` cancels the run. `stop(false)` marks it successfully completed.
Both retain the current value/status and emit no value/status notifications.
`set_value` stops/cancels playback and always notifies value listeners, including
when the clamped value is unchanged. `reset` sets the lower bound. Starting a
new run cancels the preceding one. Command validation occurs before replacing
an existing run, so invalid targets/repeat options do not interrupt playback.

`set_muted(true)` suppresses automatic value updates and further repaint
requests while retaining active playback. Time still elapses. Unmuting requests
a frame and the next owning pass catches up, possibly completing immediately.
Explicit commands and set_value still work while muted. This is manual muting;
[managed controllers and inherited TickerMode](ticker-mode.md) now provide scoped
permission and element-owned teardown.

## Listeners, completion and lifetime

`subscribe` listens to value changes through the ordinary Listenable protocol.
Automatic ticks notify only when the sampled value changes. Equal-time duplicate
passes do not produce additional value updates. `subscribe_status` uses the same
scoped Subscription type and callbacks can read `status()`. Status listeners
fire when the reported status changes. Notifications happen after releasing
controller state borrows and reuse the existing reentrant ChangeNotifier dispatch.
Listeners may start/stop/replace playback; subsequent dispatch reads current
state. Listener panics are isolated by the notifier. Do not capture a strong
controller in its own listener: `downgrade` supplies a weak handle for that use.

Every start method returns an AnimationRun. Its `outcome()` can be inspected,
or its clone can be awaited by a UI-local executor. All clones share completion.
Completion and cancellation wake registered tasks after releasing state borrows.
Unlike Flutter's TickerFuture/orCancel split, this original Rust future always
resolves to AnimationOutcome::Completed, Canceled, or Failed for a custom-model error. It does not supply an async
executor and is not Send; worker threads cannot own UI controllers/futures.

`dispose` stops/cancels playback and clears both listener lists. Further commands
and subscriptions fail, while getters retain their last snapshot. Disposal is
rejected during value/status notification to avoid partially disposing a source;
listeners can stop/reset instead. Dropping the final controller handle also
cancels its pending run and removes its weak driver registration without needing
another frame. A builder holds its own source clone, so remove/dispose the
owning component and subscriptions as appropriate. In a State implementation,
create the controller from init_state's egui context and dispose it in dispose.
Alternatively, use `ctx.animation_controller` for automatic element ownership and
scoped mode; see [managed lifetime](ticker-mode.md).

## Host integration and remaining scope

One callback per egui Context drives weak controller registrations held on the
UI thread. Callbacks release registry borrows before notifying, allowing listener
reentrancy. Only the viewport that created a controller advances it; repaint
requests target that viewport. Separate contexts have separate drivers. Keep the
Context on the same UI thread for its passes. Controllers follow supplied egui
monotonic time, not a guaranteed fixed frame rate; suspended or delayed frames
can skip intermediate samples. Nonfinite/backward tick times are ignored.

This adds bounded point-to-point/repeat control, explicit muting, futures and
observable integration. [Checked simulations](simulation-animations.md) now drive bounded controllers.
Unbounded controllers, fling/velocity convenience APIs,
mutable durations/bounds, nonlinear/custom curves, animation accessibility
policies, complete TickerProvider/force-frame/resync semantics, derived animation adapters,
the full implicit/transition widget families and the full Flutter animation member catalog
remain unfinished. Existing scalar Tween sampling stays available.

## Verification

`tests/animation_controller.rs` uses deterministic frame times to cover forward,
reverse duration/distance, directional statuses and retention, curves, repeat
phase/count/direction, indefinite repeat, muting/catch-up, clamping, zero-duration
runs, invalid-command rollback, extreme bounds, futures/wakes/cancellation/drop,
reentrant listeners/disposal guards, context/viewport isolation and retained
builder geometry updates. A private driver test verifies registration cleanup
and reuse without retaining a controller.

The native gallery's purple foreground sample is clickable. The Linux/Android
probe injects a click, waits for its rendered width to reach 48 logical points,
clicks again and requires 24 points. It also requires intermediate controller
values from actual widget builds and preserves exact foreground color, covered
child and font-baseline assertions. These checks establish frame-driven native
progress and endpoints, not a frame-rate or jank performance budget.

References: [AnimationController](https://api.flutter.dev/flutter/animation/AnimationController-class.html),
[animateTo](https://api.flutter.dev/flutter/animation/AnimationController/animateTo.html),
[repeat](https://api.flutter.dev/flutter/animation/AnimationController/repeat.html),
[stop](https://api.flutter.dev/flutter/animation/AnimationController/stop.html).
