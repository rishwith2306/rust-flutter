# One-dimensional physics

The public `physics` module provides pure, checked `Simulation` queries. Positions
use logical points, velocity uses logical points per second, and sampling takes
elapsed `Duration`. A sample contains position, velocity and a `done` advisory.
Queries are stateless: sampling a later time does not change earlier results.
The trait is object safe. These models do not schedule frames, attach themselves
to a widget, or implement ScrollPosition/ScrollPhysics.
[AnimationController can now drive these models](simulation-animations.md).

```rust
use rust_flutter::physics::*;
use std::time::Duration;

// A caller can supply a pan's effective fling velocity for either axis.
let friction = FrictionSimulation::new(0.135, 0.0, 1000.0)?;
let bounded = BoundedFrictionSimulation::new(friction, 0.0, 200.0)?;
let release = bounded.sample(Duration::ZERO)?;
assert_eq!(release.velocity, 1000.0);
let stopped = bounded.sample(Duration::from_secs(2))?;
assert_eq!(stopped.position, 200.0);
assert_eq!(stopped.velocity, 0.0);
assert!(stopped.done);

let spring = SpringDescription::with_damping_ratio(1.0, 100.0, 0.5)?;
let returning = SpringSimulation::with_options(
    spring, 200.0, 100.0, 0.0, Tolerance::default(), true,
)?;
assert!(!returning.sample(Duration::ZERO)?.done);
let settled = returning.sample(Duration::from_secs(10))?;
assert_eq!(settled.position, 100.0);
assert!(settled.done);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Friction and contact

For `0 < drag < 1`, friction follows `v(t) = v0 * exp(-rate*t)` with
`rate = -ln(drag)`. Position integrates that velocity. `final_position` gives the
asymptotic destination. Both directions and zero initial velocity are supported.
`exp_m1` preserves small-time displacement that subtracting nearly equal
exponentials would lose. Completion tests speed strictly below velocity tolerance;
a done sample keeps its current analytic position and velocity.

`BoundedFrictionSimulation` accepts finite ordered bounds containing the initial
position. Contact clamps position, sets velocity to zero, and marks completion.
Outward motion at a bound stops immediately; inward motion may proceed. Equal
bounds pin position. Bounds are fixed for this simulation. This is a contact
policy, not Android's ClampingScrollSimulation or a bouncing scroll model.

## Springs

The spring solves `mass*x'' + damping*x' + stiffness*(x-end) = 0` with a specified
start and initial velocity. Mass and stiffness must be finite and positive;
damping must be finite and nonnegative. `with_damping_ratio` constructs a
description from mass, stiffness and a finite nonnegative ratio. `spring_type`
reports underdamped, critical or overdamped using the derived rates.

All regimes use closed-form solutions. The overdamped slow root avoids subtracting
nearly equal large rates, and exponential differences preserve near-critical
small-time behavior. A zero-time sample returns the exact supplied starting
position/velocity, subject to optional completion snapping. Undamped springs can
oscillate indefinitely. Completion requires both displacement and speed strictly
below their respective tolerances. `snap_to_end` makes a done sample return the
exact endpoint and zero velocity. Without it the analytic sample is preserved.
Completion is a per-query advisory, not a latched state or a proof that an
arbitrary undamped trajectory can never leave the tolerance region later.

## Validation and numerical scope

`Tolerance` defaults to 0.001 for distance, velocity and time. All fields must be
finite and strictly positive. The time field is reserved; these models do not use
it for completion. Tolerances are copied at construction and cannot mutate an
existing simulation. Negative/nonfinite time is excluded by Duration.

Constructors reject invalid data and unrepresentable derived rates, coefficients
or asymptotic friction travel. Samples return `PhysicsError::NumericalRange`
instead of nonfinite geometry. f64 arithmetic still has rounding, underflow and
trigonometric argument-reduction limits. Extremely large finite spring samples
may be rejected; no accuracy claim is made for enormous oscillator phases.
Exponential underflow can produce exact rest at very large elapsed times.

Analytic tests check both friction directions and spring regimes, differential
equation/derivative residuals, near-critical continuity, bounded contacts,
strict tolerances, snapping, query order, tiny/huge times, invalid inputs and
checked numerical failure. This is mathematical evidence, not native scrolling
or platform feel validation.

## Compatibility work remaining

Reviewed references: [Simulation](https://api.flutter.dev/flutter/physics/Simulation-class.html),
[FrictionSimulation](https://api.flutter.dev/flutter/physics/FrictionSimulation-class.html),
[SpringSimulation](https://api.flutter.dev/flutter/physics/SpringSimulation-class.html),
[SpringDescription](https://api.flutter.dev/flutter/physics/SpringDescription-class.html)
and [Tolerance](https://api.flutter.dev/flutter/physics/Tolerance-class.html).
These are partial Rust counterparts, not a complete member audit. Constant
deceleration, through/time-at-position friction constructors, duration/bounce spring
configuration, gravity/clamped composition, scroll simulations/physics,
overscroll, slivers and native scroll runtime validation
remain unfinished. Existing pan release data can feed these pure models; wiring
it to a retained scroll viewport is separate work.
