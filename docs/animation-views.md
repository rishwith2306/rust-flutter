# Constant and reverse animation views

AlwaysStoppedAnimation<T> supplies an immutable value wherever an Animation<T>
is required. It has no clock, requests no frames and retains no listener
callbacks. Its status is Forward, following the reviewed Flutter contract;
status-derived helpers therefore report Forward even though it never ticks.
Clones share source identity and value storage. Values may be arbitrary Clone
application types, including strings.

ReverseAnimation maps finite scalar values through `1 - parent` and also maps
Forward↔Reverse and Completed↔Dismissed. A descending ValueTween changes value
alone; use this view when status and direction must reverse too. Overshooting
finite inputs are allowed, so output can fall outside 0…1.

```rust
use rust_flutter::animation::{AlwaysStoppedAnimation, AnimationDrive,
    ReverseAnimation, ValueTween};
let parent = AlwaysStoppedAnimation::new(0.25_f32);
let reverse = ReverseAnimation::new(parent)?;
assert_eq!(reverse.try_value()?, 0.75);
assert_eq!(reverse.status(), rust_flutter::animation::AnimationStatus::Reverse);
let width = reverse.clone().drive(ValueTween::new(0.0_f32, 100.0)?)?;
assert_eq!(width.try_value()?, 75.0);
width.dispose()?;
reverse.dispose()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Reverse views forward owned value/status subscriptions through the existing
DrivenAnimation machinery. Callbacks inspect the view's reversed status. Clones
share identity and disposal; disposing or dropping the last view releases its
links without stopping the parent. Weak handles support callbacks without an
ownership cycle. Double reversal restores the original value/status within f32
precision. Disposed or invalid parents propagate checked sample errors; direct
value getters panic on failed samples, like other derived sources. Use try_value
for fallible sources.

The Rust owned subscriptions, explicit reverse-view disposal and checked f32
sampling differ from Flutter's lazy listener mixins and double-valued API.
Six headless tests cover generic constants/listener retention, four-way status
mapping, status callbacks, disposal/drop cleanup, finite overshoot, invalid or
disposed parents, double reversal and real controller-frame/tween playback.
These pure source tests do not add a new device/runtime claim.

Proxy/TrainHopping/compound animation families, specialized constant completion
sources and full member/diagnostic compatibility remain unfinished. Reviewed the
[ReverseAnimation contract](https://api.flutter.dev/flutter/animation/ReverseAnimation-class.html)
and [AlwaysStoppedAnimation contract](https://api.flutter.dev/flutter/animation/AlwaysStoppedAnimation-class.html).
