# Horizontal and vertical drags

`PanConfig.axis` selects an optional global logical axis. None (the default)
keeps unrestricted two-dimensional pan behavior. `HorizontalDragGestureRecognizer`
and `VerticalDragGestureRecognizer` are named constructors using the same tracked
sequence, arena, routing and lifetime engine. Their constructor fixes the named
axis, overriding any axis supplied in PanConfig. Callback configuration continues
to use PanCallbacks.

```rust
use rust_flutter::{egui::{pos2,vec2},gestures::*};
use std::{cell::RefCell,rc::Rc,time::Duration};
let arena=GestureArenaManager::new();
let router=PointerRouter::new();
let updates=Rc::new(RefCell::new(Vec::new()));
let log=updates.clone();
let vertical=VerticalDragGestureRecognizer::new(&arena,&router,PanConfig {
    start_behavior:DragStartBehavior::Down,..Default::default()
},PanCallbacks {
    on_update:Some(Rc::new(move |details|log.borrow_mut().push(details))),
    ..Default::default()
})?;
let down=PointerEvent {pointer:PointerId(1),phase:PointerPhase::Down,
    position:pos2(0.0,0.0),time:Duration::ZERO,device:PointerDeviceKind::Touch,
    buttons:PointerButtons::PRIMARY};
vertical.add_pointer(down)?;
arena.close(down.pointer)?;
arena.flush();
router.route(down.pointer,&PointerEvent {phase:PointerPhase::Move,
    position:pos2(100.0,25.0),time:Duration::from_millis(10),..down});
assert_eq!(updates.borrow()[0].delta,vec2(0.0,25.0));
assert_eq!(updates.borrow()[0].primary_delta,Some(25.0));
vertical.cancel();
# Ok::<(),Box<dyn std::error::Error>>(())
```

## Recognition and updates

Axis drag slop uses the absolute selected component of net displacement from
down, strictly greater than configured slop. Cross-axis distance does not
contribute. Reversals below the threshold do not accumulate path length.
Threshold crossing remains sticky once reached. A sole recognizer can win the
arena before moving, but callbacks still wait for the threshold under this
framework's existing policy. Cross-only movement followed by up cancels without
start/end. Device-specific thresholds and Flutter's configurable early-start
policy remain unfinished.

Horizontal/vertical recognizers compete in the same arena. The component that
crosses threshold can accept while the other stays pending. If both cross in
one packet, routing/registration order decides which acceptance is seen first;
there is no angle-dominance test. Unrestricted pan still uses its existing radial
net-displacement threshold. The engine tracks one primary-button pointer sequence;
raw multitouch, teams, trackpad pan/zoom and other-button strategies remain pending.

Start behavior is unchanged: Start drops accumulated threshold motion, while
Down starts at the original down event and delivers accumulated displacement.
Axis updates zero the other component. `primary_delta` contains the signed
selected component, including negative movement; for unrestricted pan it is None.
Cross-only updates after start are suppressed. The event inside each details
value retains its raw global position and timestamp.

## Fling classification and API changes

The tracker still records full two-dimensional motion. End details retain that
raw estimate, age, confidence and status. Classification uses a projected copy:
both recent displacement and fitted velocity must pass their configured thresholds
on the selected axis. Large cross-axis speed/distance cannot manufacture a fling.
Effective velocity is axis-only and clamped by that component's magnitude.
`primary_velocity` contains the signed effective component, or zero for a non-fling.
Unrestricted pan keeps a two-dimensional effective velocity and None primary
velocity. Stopped/unavailable estimates retain the existing no-fling policy.

This extends experimental structs. Explicit PanConfig literals need an axis
field or `..Default::default()`. Explicit DragUpdateDetails/DragEndDetails literals
must provide the new primary fields. Applications receiving callbacks can use
the existing fields as before. Raw numeric validation still applies to both
coordinates, even when only one axis controls the drag; invalid geometry/device/
buttons/time retain the existing cancellation/error behavior.

Changing PanConfig.axis on a binding target replaces recognizers and cancels the
old sequence. Callback-only updates preserve it. Named weak handles expose the
same upgrade/cancel/dispose/tracked-pointer operations as the other recognizers.
Reentrant disposal during start suppresses subsequent update/end callbacks.

## Scroll viewport integration

SingleChildScrollView selects its declared axis in PanConfig. A vertical child
therefore stays pending during a horizontal drag, allowing a horizontal ancestor
to win. Reverse scrolling changes content direction after recognition; it does
not change which axis is recognized. Global threshold/primary values convert to
content coordinates through the viewport's existing positive scale handling.
This is orthogonal arena competition, not same-axis edge handoff, residual drag
distribution or coordinated nested scrolling. Wheel signals keep their separate
[resolver policy](scroll-signals.md).

Tests cover strict slop/reversal, cross-only motion, update projection/raw event
preservation, both start behaviors, ordinary pan compatibility, axis-only fling
clamping and distance/speed gates, negative velocity, orthogonal/tied competition,
named weak lifetime, configuration replacement and nested orthogonal viewports.
The native probe adds horizontal motion over the vertical gallery viewport and
requires unchanged clipped pixels before verifying vertical drag and wheel/reset
behavior. These establish the stated geometry/input paths, not full native device
fidelity or Flutter member compatibility.

Reviewed [VerticalDragGestureRecognizer](https://api.flutter.dev/flutter/gestures/VerticalDragGestureRecognizer-class.html),
[HorizontalDragGestureRecognizer](https://api.flutter.dev/flutter/gestures/HorizontalDragGestureRecognizer-class.html),
and the [drag implementation](https://github.com/flutter/flutter/blob/master/packages/flutter/lib/src/gestures/monodrag.dart).
Local/arbitrary transforms, full drag detail families, multitouch policies,
platform velocity strategies and the broader recognizer catalog remain unfinished.
