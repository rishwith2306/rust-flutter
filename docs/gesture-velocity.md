# Velocity estimates and pan release details

`VelocityTracker` maintains bounded, timestamped global logical-point samples.
It fits a quadratic position curve and evaluates its derivative at the newest
sample. `PanGestureRecognizer` now uses that history to report `DragEndDetails`.
`FlingConfig` classifies release motion and bounds its effective velocity. This
supplies release data for future scrolling/physics; it does not itself implement
inertial motion, a ScrollView, or a physics simulation.

```rust
use rust_flutter::{egui::pos2, gestures::*};
use std::time::Duration;
let mut tracker = VelocityTracker::new(PointerDeviceKind::Touch);
for i in 0..=8 {
    tracker.add_position(Duration::from_millis(i * 10), pos2(i as f32 * 10.0, 0.0))?;
}
let estimate = tracker.estimate_at(Duration::from_millis(90)).unwrap();
assert_eq!(estimate.status, VelocityEstimateStatus::Fitted);
assert!((estimate.velocity.pixels_per_second().x - 1000.0).abs() < 0.01);
assert_eq!(estimate.age, Duration::from_millis(10));
let fling = FlingConfig::default().classify(&estimate)?;
assert!(fling.is_fling);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Sampling and estimation

Defaults retain up to 20 samples within 100 ms, with a maximum continuous-motion
gap of 40 ms. `VelocityTrackerConfig` accepts history sizes 3–256 and positive
horizon/stop-gap durations. A gap greater than the stop threshold starts a new
segment; samples outside the horizon and excess oldest samples are removed.
Positions must be finite and times monotonic. Invalid or older samples return
errors without changing history. Equal-time samples replace the newest sample,
preventing a batch of same-time events from inventing temporal precision.

`estimate` queries at the latest sample time. `estimate_at` uses an explicit
query/release timestamp, without reading a wall clock. Empty history, a query
before the latest sample, or geometry/velocity outside finite Vec2 range returns
None. Convenience `velocity`/`velocity_at` return zero when unavailable.

An estimate includes velocity, sample count, selected duration/offset, sample age,
confidence and status:

- `Fitted`: at least three distinct, numerically usable times produced a fit.
- `Unavailable`: samples exist but are insufficient or degenerate; velocity is zero.
- `Stopped`: query time is more than the stop gap beyond the latest sample;
  velocity, duration and offset are zero, and age records the elapsed gap.

Confidence is the product of the two axes' clamped R² fit scores. A constant axis
has confidence 1. Fallback statuses also carry confidence 1; use status rather
than confidence alone to determine whether motion was fitted. A low-confidence
fit is exposed to the application, rather than silently treated as reliable.

The solver uses f64 intermediates, positions relative to the newest sample and
times normalized to the selected duration. Absolute Duration epochs never become
floating-point values. Two-pass modified Gram-Schmidt QR avoids normal equations
and rejects degenerate time bases. Results convert back to finite f32 Vec2 data;
unrepresentable results return None. Query allocations/work are bounded by the
validated history cap; movement recording does not run the solver.

## Fling policy

Defaults require speed strictly greater than 50 logical points/second and recent
offset magnitude strictly greater than 18 logical points. Qualifying effective
velocity is clamped to at most 8000 points/second, preserving direction to
floating-point precision. Both tests use the selected recent segment, not total
drag distance. These are explicit current policies, not device-specific settings.

`FlingConfig` requires finite, nonnegative, ordered velocity limits and a finite
nonnegative minimum distance. `classify` rejects invalid offset/confidence data.
Unavailable/stopped estimates do not fling. Non-flings carry zero effective
velocity while the estimate retains its raw fitted motion. A zero maximum with
zero minimum suppresses effective flings. `Velocity` validates finite components;
its magnitude clamp preserves a zero vector even for a nonzero minimum, because
zero has no direction. Length calculations widen to f64 to avoid f32 overflow.

## Pan integration and API change

`PanConfig` now includes `velocity` and `fling` settings. Use
`..PanConfig::default()` when specifying only slop/start behavior. Invalid settings
fail recognizer/target construction or target update before disturbing tracking.
Changing settings on a retained target follows the existing cancel/replace policy.

`PanCallbacks.on_end` now takes `DragEndDetails`, replacing the previous final
PointerEvent payload. Read `details.event.position` for the final position;
`details.velocity.pixels_per_second()` is the effective fling velocity,
`details.is_fling` is its classification and `details.estimate` contains raw fit
metadata. This is a breaking experimental API change.

Each sequence records down and valid moves, including movement before winning
the arena. A changed up position contributes final motion; an unchanged up
queries elapsed time without adding a stationary sample that biases the fit.
Release estimates use the up timestamp even when arena acceptance occurs later.
Cancellation/disposal still suppresses end. A new pointer sequence gets fresh
history, including when the same pointer ID or timestamp epoch is reused.

The egui adapters timestamp normalized events at frame time. Same-frame samples
coalesce, and sparse/janky frames can correctly produce unavailable or stopped
estimates. Precise raw device timestamps/history, alternate platform strategies,
full local-coordinate details, transform conversion and complete gesture/scroll physics
compatibility remain unfinished. Hosts with precise normalized samples can supply
their timestamps through the public router/binding directly.

The gallery draws cyan release feedback from its rich end callback, alongside
pink drag-start feedback. Native probes require those exact pixels, continued
tap interaction, a release payload with finite/clamped values, valid confidence
and sample count, and age/status consistency. They allow legitimate stopped or
unavailable estimates; they do not establish native fling accuracy or simulation
parity. Analytic tests verify linear/quadratic motion, strict thresholds, clamping,
pause behavior, timestamps, bounds, numerical limits and delayed acceptance.

Reviewed references: [Flutter velocity tracker](https://api.flutter.dev/flutter/gestures/VelocityTracker-class.html),
[estimate implementation](https://api.flutter.dev/flutter/gestures/VelocityTracker/getVelocityEstimate.html),
[pan fling classification](https://api.flutter.dev/flutter/gestures/PanGestureRecognizer/isFlingGesture.html),
[drag end details](https://api.flutter.dev/flutter/gestures/DragEndDetails-class.html).
No complete member-level compatibility has been established.

[Axis drags](axis-drags.md) now project classification onto the selected component
while retaining raw two-dimensional estimates in end details.
